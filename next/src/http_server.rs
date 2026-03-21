// http_server.rs — HTTP server for Cocotte
//
// Provides `http.serve(port, handler_func)` — a synchronous, single-threaded
// HTTP server using only the standard library.  The handler callback is a
// Cocotte function that receives a Request map and must return a Response map.
//
// Request map keys:
//   "method"   -> string  ("GET", "POST", "PUT", "DELETE", ...)
//   "path"     -> string  ("/api/users")
//   "query"    -> string  ("q=hello&page=2") — raw query string, may be ""
//   "headers"  -> map     (header-name -> header-value, both lowercase)
//   "body"     -> string  (raw request body, may be "")
//
// Response map keys (all optional with defaults):
//   "status"   -> number  (default: 200)
//   "body"     -> string  (default: "")
//   "headers"  -> map     (additional response headers)
//
// Usage:
//
//   module add "http"
//
//   http.serve(9192, func(req)
//       var path = req.get("path")
//       if path == "/hello"
//           return {"status": 200, "body": "Hello, World!"}
//       end
//       return {"status": 404, "body": "Not found"}
//   end)

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::value::{Value, CocotteFunction};
use crate::error::{CocotteError, Result};

/// Called by `make_http_server_serve()`.  Blocks forever serving requests.
pub fn run_server(port: u16, handler: CocotteFunction) -> Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr)
        .map_err(|e| CocotteError::runtime(&format!("http.serve: bind failed on {}: {}", addr, e)))?;
    println!("http.serve: listening on {}", addr);

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("http.serve: accept error: {}", e);
                continue;
            }
        };

        // Parse the request
        let req_val = match parse_request(&mut stream) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("http.serve: parse error: {}", e);
                let _ = write_response(&mut stream, 400, &[], "Bad Request");
                continue;
            }
        };

        // Call the Cocotte handler
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

        // Convert response map -> HTTP response
        let (status, extra_headers, body) = extract_response(response);
        let _ = write_response(&mut stream, status, &extra_headers, &body);
    }
    Ok(())
}

// ── Request parsing ───────────────────────────────────────────────────────────

fn parse_request<S: Read>(stream: &mut S) -> Result<Value> {
    let mut reader = BufReader::new(stream);

    // Read request line
    let mut request_line = String::new();
    reader.read_line(&mut request_line)
        .map_err(|e| CocotteError::runtime(&format!("read request line: {}", e)))?;
    let request_line = request_line.trim_end();

    let mut parts = request_line.splitn(3, ' ');
    let method = parts.next().unwrap_or("GET").to_string();
    let raw_target = parts.next().unwrap_or("/").to_string();

    // Split path and query
    let (path, query) = if let Some(pos) = raw_target.find('?') {
        (raw_target[..pos].to_string(), raw_target[pos + 1..].to_string())
    } else {
        (raw_target, String::new())
    };

    // Read headers
    let mut headers: HashMap<String, Value> = HashMap::new();
    let mut content_length: usize = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)
            .map_err(|e| CocotteError::runtime(&format!("read header: {}", e)))?;
        let line = line.trim_end();
        if line.is_empty() { break; }
        if let Some(colon) = line.find(':') {
            let name  = line[..colon].trim().to_lowercase();
            let value = line[colon + 1..].trim().to_string();
            if name == "content-length" {
                content_length = value.parse().unwrap_or(0);
            }
            headers.insert(name, Value::Str(value));
        }
    }

    // Read body
    let body = if content_length > 0 {
        let mut buf = vec![0u8; content_length];
        reader.read_exact(&mut buf)
            .map_err(|e| CocotteError::runtime(&format!("read body: {}", e)))?;
        String::from_utf8_lossy(&buf).to_string()
    } else {
        String::new()
    };

    // Build Cocotte map
    let mut map: HashMap<String, Value> = HashMap::new();
    map.insert("method".into(),  Value::Str(method));
    map.insert("path".into(),    Value::Str(path));
    map.insert("query".into(),   Value::Str(query));
    map.insert("headers".into(), Value::Map(Arc::new(Mutex::new(headers))));
    map.insert("body".into(),    Value::Str(body));

    Ok(Value::Map(Arc::new(Mutex::new(map))))
}

// ── Response extraction ───────────────────────────────────────────────────────

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
                let hm = hm.lock().unwrap();
                for (k, v) in hm.iter() {
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

// ── Response writing ──────────────────────────────────────────────────────────

fn write_response<W: Write>(
    stream: &mut W,
    status: u16,
    extra_headers: &[(String, String)],
    body: &str,
) -> std::io::Result<()> {
    let reason = status_reason(status);
    let body_bytes = body.as_bytes();

    // Detect content type
    let content_type = if body.trim_start().starts_with('{') || body.trim_start().starts_with('[') {
        "application/json; charset=utf-8"
    } else if body.trim_start().starts_with('<') {
        "text/html; charset=utf-8"
    } else {
        "text/plain; charset=utf-8"
    };

    write!(stream, "HTTP/1.1 {} {}\r\n", status, reason)?;
    write!(stream, "Content-Type: {}\r\n", content_type)?;
    write!(stream, "Content-Length: {}\r\n", body_bytes.len())?;
    write!(stream, "Connection: close\r\n")?;
    for (k, v) in extra_headers {
        write!(stream, "{}: {}\r\n", k, v)?;
    }
    write!(stream, "\r\n")?;
    stream.write_all(body_bytes)?;
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

// ── Static file server ────────────────────────────────────────────────────────

/// Serves files from `dir` on `port`.  Blocks forever.
pub fn run_static_server(port: u16, dir: &str) -> Result<()> {
    let dir = dir.to_string();
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr)
        .map_err(|e| CocotteError::runtime(&format!("http.serve_static: bind failed on {}: {}", addr, e)))?;
    println!("http.serve_static: listening on {} serving {}", addr, dir);

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(e) => { eprintln!("accept error: {}", e); continue; }
        };

        let req = match parse_request(&mut stream) {
            Ok(v) => v,
            Err(_) => { let _ = write_response(&mut stream, 400, &[], "Bad Request"); continue; }
        };

        let path = match &req {
            Value::Map(m) => match m.lock().unwrap().get("path") {
                Some(Value::Str(s)) => s.clone(),
                _ => "/".to_string(),
            },
            _ => "/".to_string(),
        };

        // Sanitise path — prevent directory traversal
        let safe_path = path.trim_start_matches('/').replace("../", "");
        let file_path = if safe_path.is_empty() {
            format!("{}/index.html", dir)
        } else {
            format!("{}/{}", dir, safe_path)
        };

        match std::fs::read(&file_path) {
            Ok(bytes) => {
                let ct = mime_type(&file_path);
                let _ = write_raw_response(&mut stream, 200, ct, &bytes);
            }
            Err(_) => {
                let _ = write_response(&mut stream, 404, &[], "Not Found");
            }
        }
    }
    Ok(())
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

fn mime_type(path: &str) -> &'static str {
    if path.ends_with(".html") || path.ends_with(".htm") { "text/html; charset=utf-8" }
    else if path.ends_with(".css")  { "text/css; charset=utf-8" }
    else if path.ends_with(".js")   { "application/javascript; charset=utf-8" }
    else if path.ends_with(".json") { "application/json; charset=utf-8" }
    else if path.ends_with(".png")  { "image/png" }
    else if path.ends_with(".jpg") || path.ends_with(".jpeg") { "image/jpeg" }
    else if path.ends_with(".svg")  { "image/svg+xml" }
    else if path.ends_with(".ico")  { "image/x-icon" }
    else if path.ends_with(".woff2"){ "font/woff2" }
    else                            { "application/octet-stream" }
}
