// modules.rs — Module and library loader for Cocotte
// Handles `module add "name"` and `library add "path/lib.cotlib"`

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use crate::value::{Value, NativeFunction};
use crate::error::{CocotteError, Result};
use rusqlite::Connection;

/// Load a built-in or file-based module by name
/// Returns a Value::Module containing the module's namespace
pub fn load_module(name: &str, project_root: &Path) -> Result<Value> {
    match name {
        "charlotte" => Ok(make_charlotte_module()),
        "math" => Ok(make_math_module()),
        "network" => Ok(make_network_stub_module()),
        "json" => Ok(make_json_module()),
        "os"     => Ok(make_os_module()),
        "http"   => Ok(make_http_module()),
        "sqlite" => Ok(make_sqlite_module()),
        _ => {
            // Try to load from modules/ directory
            let mod_path = project_root.join("modules").join(format!("{}.cotmod", name));
            if mod_path.exists() {
                load_cotmod_file(&mod_path)
            } else {
                Err(CocotteError::module_err(&format!(
                    "Module '{}' not found. Try: cocotte add {}",
                    name, name
                )))
            }
        }
    }
}

/// Load a local .cotlib library from disk and execute it, returning its exported namespace
pub fn load_library(path: &str, project_root: &Path) -> Result<Value> {
    if Path::new(path).is_absolute() {
        let full_path = PathBuf::from(path);
        if !full_path.exists() {
            return Err(CocotteError::module_err(&format!(
                "Library file '{}' not found", path
            )));
        }
        return load_cotmod_file(&full_path);
    }

    // 1. Try directly relative to project root (e.g. "src/stdlib/foo.cotlib")
    let direct_path = project_root.join(path);
    if direct_path.exists() {
        return load_cotmod_file(&direct_path);
    }

    // 2. Try inside the libraries/ folder (Cocotte convention)
    let lib_path = project_root.join("libraries").join(path);
    if lib_path.exists() {
        return load_cotmod_file(&lib_path);
    }

    Err(CocotteError::module_err(&format!(
        "Library file '{}' not found (tried '{}' and '{}')",
        path,
        direct_path.display(),
        lib_path.display()
    )))
}

/// Parse and execute a .cotmod or .cotlib file, extracting its top-level namespace
fn load_cotmod_file(path: &Path) -> Result<Value> {
    let source = std::fs::read_to_string(path)?;
    // Run through lexer + parser + interpreter
    let mut lexer = crate::lexer::Lexer::new(&source);
    let tokens = lexer.tokenize()?;
    let mut parser = crate::parser::Parser::new(tokens);
    let ast = parser.parse()?;

    let mut interp = crate::interpreter::Interpreter::new();
    interp.run(&ast)?;

    // Collect the top-level environment as the module's exports
    let exports = interp.export_namespace();
    Ok(Value::Module(Arc::new(Mutex::new(exports))))
}

// ── Built-in Charlotte GUI module ─────────────────────────────────────────────

fn make_charlotte_module() -> Value {
    #[cfg(feature = "gui")]
    { return crate::charlotte::make_charlotte_module(); }
    #[cfg(not(feature = "gui"))]
    {
        let mut m: HashMap<String, Value> = HashMap::new();
        m.insert("window".to_string(), Value::NativeFunction(NativeFunction {
            name: "charlotte.window".to_string(),
            arity: None,
            func: Arc::new(|_| {
                eprintln!("charlotte: GUI not available. Enable with: cargo build --features gui");
                Ok(Value::Nil)
            }),
        }));
        m.insert("version".to_string(), Value::NativeFunction(NativeFunction {
            name: "charlotte.version".to_string(),
            arity: Some(0),
            func: Arc::new(|_| Ok(Value::Str("charlotte/stub (gui feature disabled)".into()))),
        }));
        Value::Module(Arc::new(Mutex::new(m)))
    }
}

// ── Math module ──────────────────────────────────────────────────────────────

fn make_math_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    ns.insert("PI".to_string(), Value::Number(std::f64::consts::PI));
    ns.insert("E".to_string(),  Value::Number(std::f64::consts::E));
    ns.insert("TAU".to_string(), Value::Number(std::f64::consts::TAU));

    macro_rules! math_fn {
        ($name:expr, $f:expr) => {
            ns.insert($name.to_string(), Value::NativeFunction(NativeFunction {
                name: format!("math.{}", $name),
                arity: Some(1),
                func: Arc::new(|args| match &args[0] {
                    Value::Number(n) => Ok(Value::Number($f(*n))),
                    _ => Err(CocotteError::type_err(&format!("math.{}() requires a number", $name))),
                }),
            }));
        };
    }

    math_fn!("sin", f64::sin);
    math_fn!("cos", f64::cos);
    math_fn!("tan", f64::tan);
    math_fn!("asin", f64::asin);
    math_fn!("acos", f64::acos);
    math_fn!("atan", f64::atan);
    math_fn!("log", f64::ln);
    math_fn!("log2", f64::log2);
    math_fn!("log10", f64::log10);
    math_fn!("exp", f64::exp);
    math_fn!("sqrt", f64::sqrt);
    math_fn!("floor", f64::floor);
    math_fn!("ceil", f64::ceil);
    math_fn!("round", f64::round);
    math_fn!("abs", f64::abs);

    // Two-argument functions
    ns.insert("pow".to_string(), Value::NativeFunction(NativeFunction {
        name: "math.pow".to_string(),
        arity: Some(2),
        func: Arc::new(|args| match (&args[0], &args[1]) {
            (Value::Number(base), Value::Number(exp)) => Ok(Value::Number(base.powf(*exp))),
            _ => Err(CocotteError::type_err("math.pow() requires two numbers")),
        }),
    }));

    ns.insert("max".to_string(), Value::NativeFunction(NativeFunction {
        name: "math.max".to_string(),
        arity: Some(2),
        func: Arc::new(|args| match (&args[0], &args[1]) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a.max(*b))),
            _ => Err(CocotteError::type_err("math.max() requires two numbers")),
        }),
    }));

    ns.insert("min".to_string(), Value::NativeFunction(NativeFunction {
        name: "math.min".to_string(),
        arity: Some(2),
        func: Arc::new(|args| match (&args[0], &args[1]) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a.min(*b))),
            _ => Err(CocotteError::type_err("math.min() requires two numbers")),
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

// ── Network module (stub — real impl would use reqwest) ──────────────────────

fn make_network_stub_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    macro_rules! net_fn {
        ($name:expr, $body:expr) => {
            ns.insert($name.to_string(), Value::NativeFunction(NativeFunction {
                name: format!("network.{}", $name),
                arity: None,
                func: Arc::new($body),
            }));
        };
    }

    net_fn!("get", |args| {
        let url = args.get(0).map(|v| v.to_display()).unwrap_or_default();
        println!("[Network] GET {} (stub — add reqwest for real HTTP)", url);
        Ok(Value::Str(format!("{{\"url\":\"{}\",\"status\":200}}", url)))
    });

    net_fn!("post", |args| {
        let url = args.get(0).map(|v| v.to_display()).unwrap_or_default();
        let body = args.get(1).map(|v| v.to_display()).unwrap_or_default();
        println!("[Network] POST {} body={} (stub)", url, body);
        Ok(Value::Str("{\"status\":200}".to_string()))
    });

    Value::Module(Arc::new(Mutex::new(ns)))
}

// ── JSON module ──────────────────────────────────────────────────────────────

fn make_json_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    ns.insert("parse".to_string(), Value::NativeFunction(NativeFunction {
        name: "json.parse".to_string(),
        arity: Some(1),
        func: Arc::new(|args| {
            match &args[0] {
                Value::Str(s) => {
                    let v: serde_json::Value = serde_json::from_str(s)
                        .map_err(|e| CocotteError::runtime(&format!("JSON parse error: {}", e)))?;
                    Ok(json_to_cocotte(v))
                }
                _ => Err(CocotteError::type_err("json.parse() requires a string")),
            }
        }),
    }));

    ns.insert("stringify".to_string(), Value::NativeFunction(NativeFunction {
        name: "json.stringify".to_string(),
        arity: Some(1),
        func: Arc::new(|args| {
            let j = cocotte_to_json(&args[0]);
            let s = serde_json::to_string(&j)
                .map_err(|e| CocotteError::runtime(&format!("JSON stringify error: {}", e)))?;
            Ok(Value::Str(s))
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

fn json_to_cocotte(v: serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Nil,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => Value::Number(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => Value::Str(s),
        serde_json::Value::Array(arr) => {
            let items: Vec<Value> = arr.into_iter().map(json_to_cocotte).collect();
            Value::List(Arc::new(Mutex::new(items)))
        }
        serde_json::Value::Object(obj) => {
            let map: HashMap<String, Value> = obj.into_iter()
                .map(|(k, v)| (k, json_to_cocotte(v)))
                .collect();
            Value::Map(Arc::new(Mutex::new(map)))
        }
    }
}

fn cocotte_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Nil => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Number(n) => serde_json::json!(n),
        Value::Str(s) => serde_json::Value::String(s.clone()),
        Value::List(l) => {
            let items: Vec<serde_json::Value> = l.lock().unwrap().iter()
                .map(cocotte_to_json).collect();
            serde_json::Value::Array(items)
        }
        Value::Map(m) => {
            let obj: serde_json::Map<String, serde_json::Value> = m.lock().unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), cocotte_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
        other => serde_json::Value::String(other.to_display()),
    }
}

// ── OS module ────────────────────────────────────────────────────────────────

fn make_os_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    ns.insert("platform".to_string(), Value::NativeFunction(NativeFunction {
        name: "os.platform".to_string(),
        arity: Some(0),
        func: Arc::new(|_| {
            let platform = if cfg!(target_os = "windows") { "windows" }
                else if cfg!(target_os = "macos") { "macos" }
                else if cfg!(target_os = "linux") { "linux" }
                else { "unknown" };
            Ok(Value::Str(platform.to_string()))
        }),
    }));

    ns.insert("exec".to_string(), Value::NativeFunction(NativeFunction {
        name: "os.exec".to_string(),
        arity: Some(1),
        func: Arc::new(|args| {
            match &args[0] {
                Value::Str(cmd) => {
                    let output = if cfg!(target_os = "windows") {
                        std::process::Command::new("cmd").args(["/C", cmd]).output()
                    } else {
                        std::process::Command::new("sh").args(["-c", cmd]).output()
                    };
                    match output {
                        Ok(out) => {
                            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                            Ok(Value::Str(stdout))
                        }
                        Err(e) => Err(CocotteError::io_err(&format!("exec failed: {}", e))),
                    }
                }
                _ => Err(CocotteError::type_err("os.exec() requires a string command")),
            }
        }),
    }));

    ns.insert("cwd".to_string(), Value::NativeFunction(NativeFunction {
        name: "os.cwd".to_string(),
        arity: Some(0),
        func: Arc::new(|_| {
            std::env::current_dir()
                .map(|p| Value::Str(p.display().to_string()))
                .map_err(|e| CocotteError::io_err(&e.to_string()))
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

// ── HTTP module ──────────────────────────────────────────────────────────────
// Uses ureq (pure-Rust, no libcurl dependency, bundled TLS).

fn make_http_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    // http.get(url) -> string
    // http.get(url, headers_map) -> string
    ns.insert("get".into(), Value::NativeFunction(NativeFunction {
        name: "http.get".into(),
        arity: None,
        func: Arc::new(|args| {
            let url = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("http.get(url) requires a string URL")),
            };
            let mut req = ureq::get(&url);
            if let Some(Value::Map(m)) = args.get(1) {
                let m = m.lock().unwrap();
                for (k, v) in m.iter() {
                    req = req.set(k, &v.to_display());
                }
            }
            req.call()
                .map_err(|e| CocotteError::runtime(&format!("http.get failed: {}", e)))?
                .into_string()
                .map(Value::Str)
                .map_err(|e| CocotteError::runtime(&format!("http.get read failed: {}", e)))
        }),
    }));

    // http.post(url, body_string) -> string
    // http.post(url, body_string, headers_map) -> string
    ns.insert("post".into(), Value::NativeFunction(NativeFunction {
        name: "http.post".into(),
        arity: None,
        func: Arc::new(|args| {
            let url = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("http.post(url, body) requires a string URL")),
            };
            let body = args.get(1).map(|v| v.to_display()).unwrap_or_default();
            let mut req = ureq::post(&url);
            if let Some(Value::Map(m)) = args.get(2) {
                let m = m.lock().unwrap();
                for (k, v) in m.iter() {
                    req = req.set(k, &v.to_display());
                }
            }
            req.send_string(&body)
                .map_err(|e| CocotteError::runtime(&format!("http.post failed: {}", e)))?
                .into_string()
                .map(Value::Str)
                .map_err(|e| CocotteError::runtime(&format!("http.post read failed: {}", e)))
        }),
    }));

    // http.post_json(url, value) -> string
    ns.insert("post_json".into(), Value::NativeFunction(NativeFunction {
        name: "http.post_json".into(),
        arity: None,
        func: Arc::new(|args| {
            let url = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("http.post_json(url, value) requires a string URL")),
            };
            let body_val = args.get(1).cloned().unwrap_or(Value::Nil);
            let json_body = cocotte_to_json(&body_val);
            let body_str = serde_json::to_string(&json_body)
                .map_err(|e| CocotteError::runtime(&format!("JSON serialise error: {}", e)))?;
            ureq::post(&url)
                .set("Content-Type", "application/json")
                .send_string(&body_str)
                .map_err(|e| CocotteError::runtime(&format!("http.post_json failed: {}", e)))?
                .into_string()
                .map(Value::Str)
                .map_err(|e| CocotteError::runtime(&format!("http.post_json read failed: {}", e)))
        }),
    }));

    // http.put(url, body) -> string
    ns.insert("put".into(), Value::NativeFunction(NativeFunction {
        name: "http.put".into(),
        arity: None,
        func: Arc::new(|args| {
            let url = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("http.put(url, body) requires a string URL")),
            };
            let body = args.get(1).map(|v| v.to_display()).unwrap_or_default();
            ureq::put(&url)
                .send_string(&body)
                .map_err(|e| CocotteError::runtime(&format!("http.put failed: {}", e)))?
                .into_string()
                .map(Value::Str)
                .map_err(|e| CocotteError::runtime(&format!("http.put read failed: {}", e)))
        }),
    }));

    // http.delete(url) -> string
    ns.insert("delete".into(), Value::NativeFunction(NativeFunction {
        name: "http.delete".into(),
        arity: None,
        func: Arc::new(|args| {
            let url = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("http.delete(url) requires a string URL")),
            };
            ureq::delete(&url)
                .call()
                .map_err(|e| CocotteError::runtime(&format!("http.delete failed: {}", e)))?
                .into_string()
                .map(Value::Str)
                .map_err(|e| CocotteError::runtime(&format!("http.delete read failed: {}", e)))
        }),
    }));

    // http.get_json(url) -> parsed Cocotte value
    ns.insert("get_json".into(), Value::NativeFunction(NativeFunction {
        name: "http.get_json".into(),
        arity: None,
        func: Arc::new(|args| {
            let url = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("http.get_json(url) requires a string URL")),
            };
            let mut req = ureq::get(&url);
            if let Some(Value::Map(m)) = args.get(1) {
                let m = m.lock().unwrap();
                for (k, v) in m.iter() {
                    req = req.set(k, &v.to_display());
                }
            }
            let text = req
                .call()
                .map_err(|e| CocotteError::runtime(&format!("http.get_json failed: {}", e)))?
                .into_string()
                .map_err(|e| CocotteError::runtime(&format!("http.get_json read failed: {}", e)))?;
            let v: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| CocotteError::runtime(&format!("JSON parse error: {}", e)))?;
            Ok(json_to_cocotte(v))
        }),
    }));

    // http.serve(port, func(req) ... end)
    // Starts a synchronous HTTP server on `port`. Blocks forever.
    //
    // The handler is called for every request and receives a map:
    //   "method"  → string  "GET" | "POST" | "PUT" | "DELETE" | ...
    //   "path"    → string  "/api/users"
    //   "query"   → string  raw query string, e.g. "q=hello" (may be "")
    //   "headers" → map     header-name (lowercase) → header-value
    //   "body"    → string  raw request body (may be "")
    //
    // The handler must return a map:
    //   "status"  → number  HTTP status code (default: 200)
    //   "body"    → string  response body   (default: "")
    //   "headers" → map     extra response headers (optional)
    //
    // Or it may return a plain string for a simple 200 text response.
    ns.insert("serve".into(), Value::NativeFunction(NativeFunction {
        name: "http.serve".into(),
        arity: Some(2),
        func: Arc::new(|args| {
            let port = match args.first() {
                Some(Value::Number(n)) => *n as u16,
                _ => return Err(CocotteError::type_err(
                    "http.serve(port, handler) — port must be a number"
                )),
            };
            let handler = match args.get(1) {
                Some(Value::Function(f)) => f.clone(),
                _ => return Err(CocotteError::type_err(
                    "http.serve(port, handler) — handler must be a function"
                )),
            };
            crate::http_server::run_server(port, handler).map(|_| Value::Nil)
        }),
    }));

    // http.serve_static(port, dir)
    // Serves files from `dir` over HTTP on `port`. Blocks forever.
    // Returns 404 for missing files. Infers Content-Type from file extension.
    // Requests for "/" serve `dir/index.html`.
    ns.insert("serve_static".into(), Value::NativeFunction(NativeFunction {
        name: "http.serve_static".into(),
        arity: Some(2),
        func: Arc::new(|args| {
            let port = match args.first() {
                Some(Value::Number(n)) => *n as u16,
                _ => return Err(CocotteError::type_err(
                    "http.serve_static(port, dir) — port must be a number"
                )),
            };
            let dir = match args.get(1) {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err(
                    "http.serve_static(port, dir) — dir must be a string"
                )),
            };
            crate::http_server::run_static_server(port, &dir).map(|_| Value::Nil)
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

// ── SQLite module ─────────────────────────────────────────────────────────────
// Uses rusqlite with the "bundled" feature — no system SQLite needed.
//
// Connections are stored as a string path. Each operation opens the DB,
// runs the command, and closes it. This is simple and safe for scripts.
// For heavy use, cache the connection in a var between calls.

fn make_sqlite_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    // sqlite.open(path) -> db handle (stored as a string path)
    // The "handle" is just the path string — the module uses it to reopen.
    ns.insert("open".into(), Value::NativeFunction(NativeFunction {
        name: "sqlite.open".into(),
        arity: Some(1),
        func: Arc::new(|args| {
            let path = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("sqlite.open(path) requires a string")),
            };
            // Validate that we can open it
            Connection::open(&path)
                .map_err(|e| CocotteError::runtime(&format!("sqlite.open failed: {}", e)))?;
            Ok(Value::Str(path))
        }),
    }));

    // sqlite.exec(db, sql) -> nil
    // For CREATE TABLE, INSERT, UPDATE, DELETE — no return value.
    ns.insert("exec".into(), Value::NativeFunction(NativeFunction {
        name: "sqlite.exec".into(),
        arity: Some(2),
        func: Arc::new(|args| {
            let path = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("sqlite.exec(db, sql) — db must be a string path")),
            };
            let sql = match args.get(1) {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("sqlite.exec(db, sql) — sql must be a string")),
            };
            let conn = Connection::open(&path)
                .map_err(|e| CocotteError::runtime(&format!("sqlite.exec open failed: {}", e)))?;
            conn.execute_batch(&sql)
                .map_err(|e| CocotteError::runtime(&format!("sqlite.exec failed: {}", e)))?;
            Ok(Value::Nil)
        }),
    }));

    // sqlite.query(db, sql) -> list of maps
    // Each row is a map of { column_name: value }.
    ns.insert("query".into(), Value::NativeFunction(NativeFunction {
        name: "sqlite.query".into(),
        arity: Some(2),
        func: Arc::new(|args| {
            let path = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("sqlite.query(db, sql) — db must be a string path")),
            };
            let sql = match args.get(1) {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("sqlite.query(db, sql) — sql must be a string")),
            };
            let conn = Connection::open(&path)
                .map_err(|e| CocotteError::runtime(&format!("sqlite.query open failed: {}", e)))?;
            let mut stmt = conn.prepare(&sql)
                .map_err(|e| CocotteError::runtime(&format!("sqlite.query prepare failed: {}", e)))?;
            let col_names: Vec<String> = stmt.column_names()
                .into_iter().map(|s| s.to_string()).collect();
            let rows_iter = stmt.query_map([], |row| {
                let mut map: HashMap<String, Value> = HashMap::new();
                for (i, col) in col_names.iter().enumerate() {
                    let val: rusqlite::types::Value = row.get(i).unwrap_or(rusqlite::types::Value::Null);
                    let cv = match val {
                        rusqlite::types::Value::Null        => Value::Nil,
                        rusqlite::types::Value::Integer(n)  => Value::Number(n as f64),
                        rusqlite::types::Value::Real(f)     => Value::Number(f),
                        rusqlite::types::Value::Text(s)     => Value::Str(s),
                        rusqlite::types::Value::Blob(b)     => Value::Str(
                            b.iter().map(|byte| format!("{:02x}", byte)).collect()
                        ),
                    };
                    map.insert(col.clone(), cv);
                }
                Ok(map)
            }).map_err(|e| CocotteError::runtime(&format!("sqlite.query execute failed: {}", e)))?;

            let mut results: Vec<Value> = Vec::new();
            for row in rows_iter {
                let map = row.map_err(|e| CocotteError::runtime(&format!("sqlite.query row error: {}", e)))?;
                results.push(Value::Map(Arc::new(Mutex::new(map))));
            }
            Ok(Value::List(Arc::new(Mutex::new(results))))
        }),
    }));

    // sqlite.query_one(db, sql) -> map or nil
    ns.insert("query_one".into(), Value::NativeFunction(NativeFunction {
        name: "sqlite.query_one".into(),
        arity: Some(2),
        func: Arc::new(|args| {
            let path = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("sqlite.query_one(db, sql) — db must be a string path")),
            };
            let sql = match args.get(1) {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("sqlite.query_one(db, sql) — sql must be a string")),
            };
            let conn = Connection::open(&path)
                .map_err(|e| CocotteError::runtime(&format!("sqlite.query_one open failed: {}", e)))?;
            let mut stmt = conn.prepare(&sql)
                .map_err(|e| CocotteError::runtime(&format!("sqlite.query_one prepare failed: {}", e)))?;
            let col_names: Vec<String> = stmt.column_names()
                .into_iter().map(|s| s.to_string()).collect();
            let mut rows = stmt.query([])
                .map_err(|e| CocotteError::runtime(&format!("sqlite.query_one execute failed: {}", e)))?;
            match rows.next().map_err(|e| CocotteError::runtime(&format!("sqlite.query_one row error: {}", e)))? {
                None => Ok(Value::Nil),
                Some(row) => {
                    let mut map: HashMap<String, Value> = HashMap::new();
                    for (i, col) in col_names.iter().enumerate() {
                        let val: rusqlite::types::Value = row.get(i).unwrap_or(rusqlite::types::Value::Null);
                        let cv = match val {
                            rusqlite::types::Value::Null        => Value::Nil,
                            rusqlite::types::Value::Integer(n)  => Value::Number(n as f64),
                            rusqlite::types::Value::Real(f)     => Value::Number(f),
                            rusqlite::types::Value::Text(s)     => Value::Str(s),
                            rusqlite::types::Value::Blob(b)     => Value::Str(
                                b.iter().map(|byte| format!("{:02x}", byte)).collect()
                            ),
                        };
                        map.insert(col.clone(), cv);
                    }
                    Ok(Value::Map(Arc::new(Mutex::new(map))))
                }
            }
        }),
    }));

    // sqlite.tables(db) -> list of table name strings
    ns.insert("tables".into(), Value::NativeFunction(NativeFunction {
        name: "sqlite.tables".into(),
        arity: Some(1),
        func: Arc::new(|args| {
            let path = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("sqlite.tables(db) requires a db path")),
            };
            let conn = Connection::open(&path)
                .map_err(|e| CocotteError::runtime(&format!("sqlite.tables open failed: {}", e)))?;
            let mut stmt = conn.prepare(
                "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
            ).map_err(|e| CocotteError::runtime(&format!("sqlite.tables prepare failed: {}", e)))?;
            let names: std::result::Result<Vec<Value>, _> = stmt.query_map([], |row| {
                row.get::<_, String>(0)
            }).map_err(|e| CocotteError::runtime(&format!("sqlite.tables execute failed: {}", e)))?
            .map(|r| r.map(Value::Str).map_err(|e| CocotteError::runtime(&format!("sqlite.tables row error: {}", e))))
            .collect();
            Ok(Value::List(Arc::new(Mutex::new(names?))))
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}
