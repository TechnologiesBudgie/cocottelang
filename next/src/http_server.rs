// http_server.rs — HTTP server module for Cocotte
//
// Provides two public functions called by the `http` module:
//
//   run_server(port, handler)       — dynamic handler (Cocotte callback)
//   run_static_server(port, dir)    — static file server
//
// Both block the calling thread forever (serve_forever semantics).
//
// ── Handler protocol ──────────────────────────────────────────────────────────
//
// `http.serve(port, func(req) ... end)`
//
// The handler is a Cocotte function called for every request.
// It receives a single argument — a map with these keys:
//
//   "method"   → string   "GET" | "POST" | "PUT" | "DELETE" | ...
//   "path"     → string   "/api/users"
//   "query"    → string   raw query string, e.g. "q=hello&page=2" (may be "")
//   "headers"  → map      header-name (lowercase) → header-value
//   "body"     → string   raw request body (may be "")
//
// The handler must return a map (or a plain string for a 200 text response):
//
//   "status"   → number   HTTP status code (default: 200)
//   "body"     → string   response body   (default: "")
//   "headers"  → map      extra response headers (optional)
//
// ── Static file server ────────────────────────────────────────────────────────
//
// `http.serve_static(port, dir)`
//
// Serves files from `dir`.  Infers Content-Type from file extension.
// Returns 404 for missing files.  Directory traversal is blocked.
// Requests for "/" serve `dir/index.html`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::value::{Value, CocotteFunction};
use crate::error::{CocotteError, Result};

// ── Dynamic server ────────────────────────────────────────────────────────────

/// Start a synchronous HTTP server on `port`, calling `handler` for each request.
/// Blocks forever.
pub fn run_server(port: u16, handler: CocotteFunction) -> Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr)
        .map_err(|e| CocotteError::runtime(&format!("http.serve: bind failed on {}: {}", addr, e)))?;
    println!("http.serve: listening on {}", addr);

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(e) => { eprintln!("http.serve: accept error: {}", e); continue; }
        };

        let req_val = match parse_request(&mut stream) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("http.serve: parse error: {}", e);
                let _ = write_response(&mut stream, 400, &[], "Bad Request");
                continue;
            }
        };

        // Call the Cocotte handler via the shared interpreter pointer
        let ptr = crate::runtime_ctx::get_active_interpreter();
        if ptr == 0 {
            let _ = write_response(&mut stream, 500, &[], "No active interpreter");
            continue;
        }
        let interp = unsafe { &mut *(ptr as *mut crate::interpreter::Interpreter) };
        let response = match interp.call_function_pub(&handler, vec![req_val], None) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("http.serve: handler error: {}", e);
                let _ = write_response(&mut stream, 500, &[], &format!("Handler error: {}", e));
                continue;
            }
        };

        let (status, extra_headers, body) = extract_response(response);
        let _ = write_response(&mut stream, status, &extra_headers, &body);
    }
    Ok(())
}

// ── Static file server ────────────────────────────────────────────────────────

/// Start a static file server on `port` serving files from `dir`.
/// Blocks forever.
pub fn run_static_server(port: u16, dir: &str) -> Result<()> {
    let dir = dir.to_string();
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr)
        .map_err(|e| CocotteError::runtime(&format!("http.serve_static: bind on {}: {}", addr, e)))?;
    println!("http.serve_static: listening on {} (serving {})", addr, dir);

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(e) => { eprintln!("accept error: {}", e); continue; }
        };

        let req = match parse_request(&mut stream) {
            Ok(v) => v,
            Err(_) => { let _ = write_response(&mut stream, 400, &[], "Bad Request"); continue; }
        };

        let path_str = match &req {
            Value::Map(m) => match m.lock().unwrap().get("path") {
                Some(Value::Str(s)) => s.clone(),
                _ => "/".into(),
            },
            _ => "/".into(),
        };

        // Sanitise — block directory traversal
        let safe = path_str.trim_start_matches('/').replace("../", "");
        let file_path = if safe.is_empty() {
            format!("{}/index.html", dir)
        } else {
            format!("{}/{}", dir, safe)
        };

        match std::fs::read(&file_path) {
            Ok(bytes) => {
                let ct = mime_type(&file_path);
                let _ = write_raw_response(&mut stream, 200, ct, &bytes);
            }
            Err(_) => { let _ = write_response(&mut stream, 404, &[], "Not Found"); }
        }
    }
    Ok(())
}

// ── Request parsing ───────────────────────────────────────────────────────────

fn parse_request<S: Read>(stream: &mut S) -> Result<Value> {
    let mut reader = BufReader::new(stream);

    // Request line
    let mut line = String::new();
    reader.read_line(&mut line)
        .map_err(|e| CocotteError::runtime(&format!("read request line: {}", e)))?;
    let line = line.trim_end();

    let mut parts = line.splitn(3, ' ');
    let method     = parts.next().unwrap_or("GET").to_string();
    let raw_target = parts.next().unwrap_or("/").to_string();

    let (path, query) = if let Some(pos) = raw_target.find('?') {
        (raw_target[..pos].to_string(), raw_target[pos + 1..].to_string())
    } else {
        (raw_target, String::new())
    };

    // Headers
    let mut headers: HashMap<String, Value> = HashMap::new();
    let mut content_length: usize = 0;
    loop {
        let mut hline = String::new();
        reader.read_line(&mut hline)
            .map_err(|e| CocotteError::runtime(&format!("read header: {}", e)))?;
        let hline = hline.trim_end();
        if hline.is_empty() { break; }
        if let Some(colon) = hline.find(':') {
            let name  = hline[..colon].trim().to_lowercase();
            let value = hline[colon + 1..].trim().to_string();
            if name == "content-length" {
                content_length = value.parse().unwrap_or(0);
            }
            headers.insert(name, Value::Str(value));
        }
    }

    // Body
    let body = if content_length > 0 {
        let mut buf = vec![0u8; content_length];
        reader.read_exact(&mut buf)
            .map_err(|e| CocotteError::runtime(&format!("read body: {}", e)))?;
        String::from_utf8_lossy(&buf).to_string()
    } else {
        String::new()
    };

    let mut map: HashMap<String, Value> = HashMap::new();
    map.insert("method".into(),  Value::Str(method));
    map.insert("path".into(),    Value::Str(path));
    map.insert("query".into(),   Value::Str(query));
    map.insert("headers".into(), Value::Map(Arc::new(Mutex::new(headers))));
    map.insert("body".into(),    Value::Str(body));
    Ok(Value::Map(Arc::new(Mutex::new(map))))
}

// ── Response helpers ──────────────────────────────────────────────────────────

fn extract_response(val: Value) -> (u16, Vec<(String, String)>, String) {
    match val {
        Value::Map(m) => {
            let m = m.lock().unwrap();
            let status = match m.get("status") {
                Some(Value::Number(n)) => *n as u16,
                _ => 200,
            };
            let body = match m.get("body") {
                Some(Value::Str(s)) => s.clone(),
                Some(other) => other.to_display(),
                None => String::new(),
            };
            let mut extra: Vec<(String, String)> = Vec::new();
            if let Some(Value::Map(hm)) = m.get("headers") {
                for (k, v) in hm.lock().unwrap().iter() {
                    extra.push((k.clone(), v.to_display()));
                }
            }
            (status, extra, body)
        }
        Value::Str(s) => (200, vec![], s),
        Value::Nil    => (200, vec![], String::new()),
        other         => (200, vec![], other.to_display()),
    }
}

fn write_response<W: Write>(
    stream: &mut W,
    status: u16,
    extra_headers: &[(String, String)],
    body: &str,
) -> std::io::Result<()> {
    // Auto-detect content type from body content when no header overrides it
    let has_ct = extra_headers.iter().any(|(k, _)| k.to_lowercase() == "content-type");
    let ct = if has_ct {
        None
    } else if body.trim_start().starts_with('{') || body.trim_start().starts_with('[') {
        Some("application/json; charset=utf-8")
    } else if body.trim_start().starts_with('<') {
        Some("text/html; charset=utf-8")
    } else {
        Some("text/plain; charset=utf-8")
    };

    let body_bytes = body.as_bytes();
    write!(stream, "HTTP/1.1 {} {}\r\n", status, status_reason(status))?;
    if let Some(c) = ct {
        write!(stream, "Content-Type: {}\r\n", c)?;
    }
    write!(stream, "Content-Length: {}\r\n", body_bytes.len())?;
    write!(stream, "Connection: close\r\n")?;
    for (k, v) in extra_headers {
        write!(stream, "{}: {}\r\n", k, v)?;
    }
    write!(stream, "\r\n")?;
    stream.write_all(body_bytes)?;
    stream.flush()
}

fn write_raw_response<W: Write>(
    stream: &mut W,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    write!(stream, "HTTP/1.1 {} {}\r\n", status, status_reason(status))?;
    write!(stream, "Content-Type: {}\r\n", content_type)?;
    write!(stream, "Content-Length: {}\r\n", body.len())?;
    write!(stream, "Connection: close\r\n")?;
    write!(stream, "\r\n")?;
    stream.write_all(body)?;
    stream.flush()
}

fn status_reason(code: u16) -> &'static str {
    match code {
        200 => "OK", 201 => "Created", 204 => "No Content",
        301 => "Moved Permanently", 302 => "Found", 304 => "Not Modified",
        400 => "Bad Request", 401 => "Unauthorized", 403 => "Forbidden",
        404 => "Not Found", 405 => "Method Not Allowed",
        409 => "Conflict", 422 => "Unprocessable Entity",
        500 => "Internal Server Error", 501 => "Not Implemented",
        503 => "Service Unavailable",
        _   => "Unknown",
    }
}

fn mime_type(path: &str) -> &'static str {
    if      path.ends_with(".html") || path.ends_with(".htm") { "text/html; charset=utf-8" }
    else if path.ends_with(".css")   { "text/css; charset=utf-8" }
    else if path.ends_with(".js")    { "application/javascript; charset=utf-8" }
    else if path.ends_with(".json")  { "application/json; charset=utf-8" }
    else if path.ends_with(".png")   { "image/png" }
    else if path.ends_with(".jpg") || path.ends_with(".jpeg") { "image/jpeg" }
    else if path.ends_with(".svg")   { "image/svg+xml" }
    else if path.ends_with(".ico")   { "image/x-icon" }
    else if path.ends_with(".woff2") { "font/woff2" }
    else if path.ends_with(".woff")  { "font/woff" }
    else if path.ends_with(".ttf")   { "font/ttf" }
    else if path.ends_with(".webp")  { "image/webp" }
    else if path.ends_with(".gif")   { "image/gif" }
    else                             { "application/octet-stream" }
}
