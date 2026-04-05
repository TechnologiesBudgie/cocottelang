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
        "math"      => Ok(make_math_module()),
        "network"   => Ok(make_network_stub_module()),
        "json"      => Ok(make_json_module()),
        "os"        => Ok(make_os_module()),
        "http"      => Ok(make_http_module()),
        "sqlite"    => Ok(make_sqlite_module()),
        "path"      => Ok(make_path_module()),
        "env"       => Ok(make_env_module()),
        "args"      => Ok(make_args_module()),
        "uuid"      => Ok(make_uuid_module()),
        "log"       => Ok(make_log_module()),
        "process"   => Ok(make_process_module()),
        "csv"       => Ok(make_csv_module()),
        "crypto"    => Ok(make_crypto_module()),
        "base64"    => Ok(make_base64_module()),
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
    let full_path = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        project_root.join("libraries").join(path)
    };

    if !full_path.exists() {
        return Err(CocotteError::module_err(&format!(
            "Library file '{}' not found at '{}'",
            path,
            full_path.display()
        )));
    }

    load_cotmod_file(&full_path)
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
        Value::Number(n) => {
            // Emit whole numbers as JSON integers, not 95.0
            if n.fract() == 0.0 && n.abs() < 9007199254740992.0 {
                serde_json::Value::Number(serde_json::Number::from(*n as i64))
            } else {
                serde_json::json!(n)
            }
        }
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

    // sqlite.exec_params(db, sql, params) — safe parameterised exec, no SQL injection risk
    // Usage: sqlite.exec_params(db, "INSERT INTO t(name) VALUES(?)", [user_input])
    ns.insert("exec_params".into(), Value::NativeFunction(NativeFunction {
        name: "sqlite.exec_params".into(),
        arity: Some(3),
        func: Arc::new(|args| {
            let path = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("sqlite.exec_params(db, sql, params) — db must be a string")),
            };
            let sql = match args.get(1) {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("sqlite.exec_params(db, sql, params) — sql must be a string")),
            };
            let params: Vec<rusqlite::types::Value> = match args.get(2) {
                Some(Value::List(l)) => l.lock().unwrap().iter().map(sql_param).collect(),
                _ => return Err(CocotteError::type_err("sqlite.exec_params(db, sql, params) — params must be a list")),
            };
            let conn = Connection::open(&path)
                .map_err(|e| CocotteError::runtime(&format!("sqlite.exec_params failed: {}", e)))?;
            conn.execute(&sql, rusqlite::params_from_iter(params.iter()))
                .map_err(|e| CocotteError::runtime(&format!("sqlite.exec_params failed: {}", e)))?;
            Ok(Value::Nil)
        }),
    }));

    // sqlite.query_params(db, sql, params) — safe parameterised SELECT
    // Usage: sqlite.query_params(db, "SELECT * FROM t WHERE name=?", [name])
    ns.insert("query_params".into(), Value::NativeFunction(NativeFunction {
        name: "sqlite.query_params".into(),
        arity: Some(3),
        func: Arc::new(|args| {
            let path = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("sqlite.query_params(db, sql, params) — db must be a string")),
            };
            let sql = match args.get(1) {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("sqlite.query_params(db, sql, params) — sql must be a string")),
            };
            let params: Vec<rusqlite::types::Value> = match args.get(2) {
                Some(Value::List(l)) => l.lock().unwrap().iter().map(sql_param).collect(),
                _ => return Err(CocotteError::type_err("sqlite.query_params(db, sql, params) — params must be a list")),
            };
            let conn = Connection::open(&path)
                .map_err(|e| CocotteError::runtime(&format!("sqlite.query_params failed: {}", e)))?;
            let mut stmt = conn.prepare(&sql)
                .map_err(|e| CocotteError::runtime(&format!("sqlite.query_params prepare failed: {}", e)))?;
            let col_names: Vec<String> = stmt.column_names().into_iter().map(|s| s.to_string()).collect();
            let rows_iter = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
                let mut map: HashMap<String, Value> = HashMap::new();
                for (i, col) in col_names.iter().enumerate() {
                    let val: rusqlite::types::Value = row.get(i).unwrap_or(rusqlite::types::Value::Null);
                    let cv = match val {
                        rusqlite::types::Value::Null       => Value::Nil,
                        rusqlite::types::Value::Integer(n) => Value::Number(n as f64),
                        rusqlite::types::Value::Real(f)    => Value::Number(f),
                        rusqlite::types::Value::Text(s)    => Value::Str(s),
                        rusqlite::types::Value::Blob(b)    => Value::Str(
                            b.iter().map(|byte| format!("{:02x}", byte)).collect()
                        ),
                    };
                    map.insert(col.clone(), cv);
                }
                Ok(map)
            }).map_err(|e| CocotteError::runtime(&format!("sqlite.query_params execute failed: {}", e)))?;
            let mut results: Vec<Value> = Vec::new();
            for row in rows_iter {
                let map = row.map_err(|e| CocotteError::runtime(&format!("sqlite.query_params row error: {}", e)))?;
                results.push(Value::Map(Arc::new(Mutex::new(map))));
            }
            Ok(Value::List(Arc::new(Mutex::new(results))))
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

/// Convert a Cocotte Value to a rusqlite owned value for parameterised queries.
/// This is the safe alternative to string interpolation.
fn sql_param(v: &Value) -> rusqlite::types::Value {
    match v {
        Value::Nil       => rusqlite::types::Value::Null,
        Value::Number(n) => {
            if n.fract() == 0.0 && n.abs() < 9007199254740992.0 {
                rusqlite::types::Value::Integer(*n as i64)
            } else {
                rusqlite::types::Value::Real(*n)
            }
        }
        Value::Str(s)    => rusqlite::types::Value::Text(s.clone()),
        Value::Bool(b)   => rusqlite::types::Value::Integer(if *b { 1 } else { 0 }),
        other            => rusqlite::types::Value::Text(other.to_display()),
    }
}

// ── path module ───────────────────────────────────────────────────────────────
// Cross-platform path manipulation. No extra deps — pure std.

fn make_path_module() -> Value {
    use std::path::Path as StdPath;
    let mut ns: HashMap<String, Value> = HashMap::new();

    ns.insert("join".into(), Value::NativeFunction(NativeFunction {
        name: "path.join".into(), arity: None,
        func: Arc::new(|args| {
            if args.is_empty() { return Ok(Value::Str(String::new())); }
            let mut p = std::path::PathBuf::new();
            for a in &args { p.push(a.to_display()); }
            Ok(Value::Str(p.to_string_lossy().to_string()))
        }),
    }));

    ns.insert("basename".into(), Value::NativeFunction(NativeFunction {
        name: "path.basename".into(), arity: Some(1),
        func: Arc::new(|args| {
            let s = args.first().map(|v| v.to_display()).unwrap_or_default();
            let base = StdPath::new(&s).file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok(Value::Str(base))
        }),
    }));

    ns.insert("dirname".into(), Value::NativeFunction(NativeFunction {
        name: "path.dirname".into(), arity: Some(1),
        func: Arc::new(|args| {
            let s = args.first().map(|v| v.to_display()).unwrap_or_default();
            let dir = StdPath::new(&s).parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".into());
            Ok(Value::Str(dir))
        }),
    }));

    ns.insert("ext".into(), Value::NativeFunction(NativeFunction {
        name: "path.ext".into(), arity: Some(1),
        func: Arc::new(|args| {
            let s = args.first().map(|v| v.to_display()).unwrap_or_default();
            let ext = StdPath::new(&s).extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_default();
            Ok(Value::Str(ext))
        }),
    }));

    ns.insert("stem".into(), Value::NativeFunction(NativeFunction {
        name: "path.stem".into(), arity: Some(1),
        func: Arc::new(|args| {
            let s = args.first().map(|v| v.to_display()).unwrap_or_default();
            let stem = StdPath::new(&s).file_stem()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok(Value::Str(stem))
        }),
    }));

    ns.insert("abs".into(), Value::NativeFunction(NativeFunction {
        name: "path.abs".into(), arity: Some(1),
        func: Arc::new(|args| {
            let s = args.first().map(|v| v.to_display()).unwrap_or_default();
            let abs = std::fs::canonicalize(&s)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or(s);
            Ok(Value::Str(abs))
        }),
    }));

    ns.insert("exists".into(), Value::NativeFunction(NativeFunction {
        name: "path.exists".into(), arity: Some(1),
        func: Arc::new(|args| {
            let s = args.first().map(|v| v.to_display()).unwrap_or_default();
            Ok(Value::Bool(StdPath::new(&s).exists()))
        }),
    }));

    ns.insert("is_abs".into(), Value::NativeFunction(NativeFunction {
        name: "path.is_abs".into(), arity: Some(1),
        func: Arc::new(|args| {
            let s = args.first().map(|v| v.to_display()).unwrap_or_default();
            Ok(Value::Bool(StdPath::new(&s).is_absolute()))
        }),
    }));

    ns.insert("parts".into(), Value::NativeFunction(NativeFunction {
        name: "path.parts".into(), arity: Some(1),
        func: Arc::new(|args| {
            let s = args.first().map(|v| v.to_display()).unwrap_or_default();
            let parts: Vec<Value> = StdPath::new(&s).components()
                .map(|c| Value::Str(c.as_os_str().to_string_lossy().to_string()))
                .collect();
            Ok(Value::List(Arc::new(Mutex::new(parts))))
        }),
    }));

    ns.insert("home".into(), Value::NativeFunction(NativeFunction {
        name: "path.home".into(), arity: Some(0),
        func: Arc::new(|_| {
            let h = dirs::home_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".into());
            Ok(Value::Str(h))
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

// ── env module ────────────────────────────────────────────────────────────────

fn make_env_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    ns.insert("get".into(), Value::NativeFunction(NativeFunction {
        name: "env.get".into(), arity: Some(1),
        func: Arc::new(|args| {
            let key = args.first().map(|v| v.to_display()).unwrap_or_default();
            Ok(std::env::var(&key).map(Value::Str).unwrap_or(Value::Nil))
        }),
    }));

    ns.insert("get_or".into(), Value::NativeFunction(NativeFunction {
        name: "env.get_or".into(), arity: Some(2),
        func: Arc::new(|args| {
            let key     = args.first().map(|v| v.to_display()).unwrap_or_default();
            let default = args.get(1).cloned().unwrap_or(Value::Nil);
            Ok(std::env::var(&key).map(Value::Str).unwrap_or(default))
        }),
    }));

    ns.insert("set".into(), Value::NativeFunction(NativeFunction {
        name: "env.set".into(), arity: Some(2),
        func: Arc::new(|args| {
            let key = args.first().map(|v| v.to_display()).unwrap_or_default();
            let val = args.get(1).map(|v| v.to_display()).unwrap_or_default();
            std::env::set_var(&key, &val);
            Ok(Value::Nil)
        }),
    }));

    ns.insert("remove".into(), Value::NativeFunction(NativeFunction {
        name: "env.remove".into(), arity: Some(1),
        func: Arc::new(|args| {
            let key = args.first().map(|v| v.to_display()).unwrap_or_default();
            std::env::remove_var(&key);
            Ok(Value::Nil)
        }),
    }));

    ns.insert("all".into(), Value::NativeFunction(NativeFunction {
        name: "env.all".into(), arity: Some(0),
        func: Arc::new(|_| {
            let map: HashMap<String, Value> = std::env::vars()
                .map(|(k, v)| (k, Value::Str(v)))
                .collect();
            Ok(Value::Map(Arc::new(Mutex::new(map))))
        }),
    }));

    ns.insert("require".into(), Value::NativeFunction(NativeFunction {
        name: "env.require".into(), arity: Some(1),
        func: Arc::new(|args| {
            let key = args.first().map(|v| v.to_display()).unwrap_or_default();
            std::env::var(&key)
                .map(Value::Str)
                .map_err(|_| CocotteError::runtime(&format!(
                    "Required environment variable '{}' is not set", key
                )))
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

// ── args module ───────────────────────────────────────────────────────────────

fn make_args_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    ns.insert("all".into(), Value::NativeFunction(NativeFunction {
        name: "args.all".into(), arity: Some(0),
        func: Arc::new(|_| {
            let args: Vec<Value> = std::env::args().skip(1).map(Value::Str).collect();
            Ok(Value::List(Arc::new(Mutex::new(args))))
        }),
    }));

    ns.insert("get".into(), Value::NativeFunction(NativeFunction {
        name: "args.get".into(), arity: Some(1),
        func: Arc::new(|args| {
            let idx = match args.first() {
                Some(Value::Number(n)) => *n as usize,
                _ => return Err(CocotteError::type_err("args.get(i) requires a number")),
            };
            let val = std::env::args().skip(1).nth(idx)
                .map(Value::Str)
                .unwrap_or(Value::Nil);
            Ok(val)
        }),
    }));

    ns.insert("len".into(), Value::NativeFunction(NativeFunction {
        name: "args.len".into(), arity: Some(0),
        func: Arc::new(|_| {
            Ok(Value::Number((std::env::args().count().saturating_sub(1)) as f64))
        }),
    }));

    // args.flag("--verbose") → true if --verbose is present
    ns.insert("flag".into(), Value::NativeFunction(NativeFunction {
        name: "args.flag".into(), arity: Some(1),
        func: Arc::new(|args| {
            let name = args.first().map(|v| v.to_display()).unwrap_or_default();
            let found = std::env::args().any(|a| a == name);
            Ok(Value::Bool(found))
        }),
    }));

    // args.option("--port") → value after --port, or nil
    ns.insert("option".into(), Value::NativeFunction(NativeFunction {
        name: "args.option".into(), arity: Some(1),
        func: Arc::new(|args| {
            let name = args.first().map(|v| v.to_display()).unwrap_or_default();
            let argv: Vec<String> = std::env::args().collect();
            let val = argv.windows(2)
                .find(|w| w[0] == name)
                .map(|w| Value::Str(w[1].clone()))
                .unwrap_or(Value::Nil);
            Ok(val)
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

// ── uuid module ───────────────────────────────────────────────────────────────
// UUID v4 generation using random bytes from the OS.

fn make_uuid_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    ns.insert("v4".into(), Value::NativeFunction(NativeFunction {
        name: "uuid.v4".into(), arity: Some(0),
        func: Arc::new(|_| {
            // Generate 16 random bytes using the OS RNG
            let mut bytes = [0u8; 16];
            getrandom_bytes(&mut bytes)?;
            // Set version (4) and variant bits per RFC 4122
            bytes[6] = (bytes[6] & 0x0f) | 0x40;
            bytes[8] = (bytes[8] & 0x3f) | 0x80;
            let s = format!(
                "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
                u32::from_be_bytes([bytes[0],  bytes[1],  bytes[2],  bytes[3]]),
                u16::from_be_bytes([bytes[4],  bytes[5]]),
                u16::from_be_bytes([bytes[6],  bytes[7]]),
                u16::from_be_bytes([bytes[8],  bytes[9]]),
                {
                    let mut n = 0u64;
                    for i in 10..16 { n = (n << 8) | bytes[i] as u64; }
                    n
                }
            );
            Ok(Value::Str(s))
        }),
    }));

    ns.insert("is_valid".into(), Value::NativeFunction(NativeFunction {
        name: "uuid.is_valid".into(), arity: Some(1),
        func: Arc::new(|args| {
            let s = args.first().map(|v| v.to_display()).unwrap_or_default();
            let valid = is_valid_uuid(&s);
            Ok(Value::Bool(valid))
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

fn getrandom_bytes(buf: &mut [u8]) -> Result<()> {
    use std::io::Read;
    // Try /dev/urandom first (Linux/macOS)
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        return f.read_exact(buf)
            .map_err(|e| CocotteError::runtime(&format!("uuid: read /dev/urandom failed: {}", e)));
    }
    // Windows / other: xorshift64 seeded from system time + PID + stack address.
    // Not cryptographically strong but fine for UUID v4 uniqueness.
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let pid  = std::process::id() as u64;
    let addr = buf.as_ptr() as u64;   // stack address adds more entropy
    let mut seed = t ^ pid.wrapping_mul(0x9e3779b97f4a7c15) ^ addr.wrapping_mul(0x6c62272e07bb0142);
    if seed == 0 { seed = 0xdeadbeefcafebabe; }
    for b in buf.iter_mut() {
        // xorshift64
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        *b = (seed & 0xff) as u8;
    }
    Ok(())
}

fn is_valid_uuid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5 { return false; }
    let lens = [8, 4, 4, 4, 12];
    parts.iter().zip(lens.iter()).all(|(p, &l)| {
        p.len() == l && p.chars().all(|c| c.is_ascii_hexdigit())
    })
}

// ── log module ────────────────────────────────────────────────────────────────

fn make_log_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    // Shared log level: 0=debug 1=info 2=warn 3=error
    let level = Arc::new(Mutex::new(1usize));

    let mk_logger = |label: &'static str, min_level: usize, level_arc: Arc<Mutex<usize>>| {
        Value::NativeFunction(NativeFunction {
            name: format!("log.{}", label),
            arity: None,
            func: Arc::new(move |args| {
                let cur = *level_arc.lock().unwrap();
                if cur <= min_level {
                    let msg = args.iter().map(|v| v.to_display()).collect::<Vec<_>>().join(" ");
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let prefix = match label {
                        "debug" => "\x1b[36m[DEBUG]\x1b[0m",
                        "info"  => "\x1b[32m[INFO]\x1b[0m ",
                        "warn"  => "\x1b[33m[WARN]\x1b[0m ",
                        "error" => "\x1b[31m[ERROR]\x1b[0m",
                        _       => "[LOG]  ",
                    };
                    if label == "error" {
                        eprintln!("{} {} {}", prefix, now, msg);
                    } else {
                        println!("{} {} {}", prefix, now, msg);
                    }
                }
                Ok(Value::Nil)
            }),
        })
    };

    ns.insert("debug".into(), mk_logger("debug", 0, Arc::clone(&level)));
    ns.insert("info".into(),  mk_logger("info",  1, Arc::clone(&level)));
    ns.insert("warn".into(),  mk_logger("warn",  2, Arc::clone(&level)));
    ns.insert("error".into(), mk_logger("error", 3, Arc::clone(&level)));

    let level2 = Arc::clone(&level);
    ns.insert("set_level".into(), Value::NativeFunction(NativeFunction {
        name: "log.set_level".into(), arity: Some(1),
        func: Arc::new(move |args| {
            let lvl = match args.first() {
                Some(Value::Str(s)) => match s.as_str() {
                    "debug" => 0, "info" => 1, "warn" => 2, "error" => 3,
                    _ => return Err(CocotteError::runtime("log.set_level: use \"debug\", \"info\", \"warn\", or \"error\"")),
                },
                Some(Value::Number(n)) => *n as usize,
                _ => return Err(CocotteError::type_err("log.set_level requires a string or number")),
            };
            *level2.lock().unwrap() = lvl;
            Ok(Value::Nil)
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

// ── process module ────────────────────────────────────────────────────────────

fn make_process_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    // process.run(cmd) → map with "stdout", "stderr", "code"
    ns.insert("run".into(), Value::NativeFunction(NativeFunction {
        name: "process.run".into(), arity: Some(1),
        func: Arc::new(|args| {
            let cmd = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("process.run() requires a string command")),
            };
            let output = std::process::Command::new("sh")
                .arg("-c").arg(&cmd)
                .output()
                .map_err(|e| CocotteError::runtime(&format!("process.run failed: {}", e)))?;
            let mut map: HashMap<String, Value> = HashMap::new();
            map.insert("stdout".into(), Value::Str(String::from_utf8_lossy(&output.stdout).to_string()));
            map.insert("stderr".into(), Value::Str(String::from_utf8_lossy(&output.stderr).to_string()));
            map.insert("code".into(),   Value::Number(output.status.code().unwrap_or(-1) as f64));
            map.insert("ok".into(),     Value::Bool(output.status.success()));
            Ok(Value::Map(Arc::new(Mutex::new(map))))
        }),
    }));

    // process.run_args(program, [arg1, arg2, ...]) → map
    ns.insert("run_args".into(), Value::NativeFunction(NativeFunction {
        name: "process.run_args".into(), arity: Some(2),
        func: Arc::new(|args| {
            let program = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("process.run_args(program, args_list)")),
            };
            let argv: Vec<String> = match args.get(1) {
                Some(Value::List(l)) => l.lock().unwrap().iter().map(|v| v.to_display()).collect(),
                _ => return Err(CocotteError::type_err("process.run_args: second arg must be a list")),
            };
            let output = std::process::Command::new(&program)
                .args(&argv)
                .output()
                .map_err(|e| CocotteError::runtime(&format!("process.run_args failed: {}", e)))?;
            let mut map: HashMap<String, Value> = HashMap::new();
            map.insert("stdout".into(), Value::Str(String::from_utf8_lossy(&output.stdout).to_string()));
            map.insert("stderr".into(), Value::Str(String::from_utf8_lossy(&output.stderr).to_string()));
            map.insert("code".into(),   Value::Number(output.status.code().unwrap_or(-1) as f64));
            map.insert("ok".into(),     Value::Bool(output.status.success()));
            Ok(Value::Map(Arc::new(Mutex::new(map))))
        }),
    }));

    // process.exit(code)
    ns.insert("exit".into(), Value::NativeFunction(NativeFunction {
        name: "process.exit".into(), arity: Some(1),
        func: Arc::new(|args| {
            let code = match args.first() {
                Some(Value::Number(n)) => *n as i32,
                _ => 0,
            };
            std::process::exit(code);
        }),
    }));

    // process.pid() → current process id
    ns.insert("pid".into(), Value::NativeFunction(NativeFunction {
        name: "process.pid".into(), arity: Some(0),
        func: Arc::new(|_| Ok(Value::Number(std::process::id() as f64))),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

// ── csv module ────────────────────────────────────────────────────────────────
// Pure-Rust CSV parser — no extra dependency.

fn make_csv_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    // csv.parse(text) → list of lists
    ns.insert("parse".into(), Value::NativeFunction(NativeFunction {
        name: "csv.parse".into(), arity: Some(1),
        func: Arc::new(|args| {
            let text = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("csv.parse() requires a string")),
            };
            let rows = parse_csv(&text);
            Ok(Value::List(Arc::new(Mutex::new(rows))))
        }),
    }));

    // csv.parse_with_headers(text) → list of maps (first row = headers)
    ns.insert("parse_with_headers".into(), Value::NativeFunction(NativeFunction {
        name: "csv.parse_with_headers".into(), arity: Some(1),
        func: Arc::new(|args| {
            let text = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("csv.parse_with_headers() requires a string")),
            };
            let mut rows = parse_csv(&text);
            if rows.is_empty() { return Ok(Value::List(Arc::new(Mutex::new(vec![])))); }
            let headers: Vec<String> = match rows.remove(0) {
                Value::List(h) => h.lock().unwrap().iter().map(|v| v.to_display()).collect(),
                _ => return Err(CocotteError::runtime("csv: unexpected header row type")),
            };
            let result: Vec<Value> = rows.into_iter().map(|row| {
                let fields: Vec<Value> = match row {
                    Value::List(l) => l.lock().unwrap().clone(),
                    v => vec![v],
                };
                let mut map: HashMap<String, Value> = HashMap::new();
                for (i, header) in headers.iter().enumerate() {
                    map.insert(header.clone(), fields.get(i).cloned().unwrap_or(Value::Nil));
                }
                Value::Map(Arc::new(Mutex::new(map)))
            }).collect();
            Ok(Value::List(Arc::new(Mutex::new(result))))
        }),
    }));

    // csv.stringify(list_of_lists) → CSV string
    ns.insert("stringify".into(), Value::NativeFunction(NativeFunction {
        name: "csv.stringify".into(), arity: Some(1),
        func: Arc::new(|args| {
            let rows = match args.first() {
                Some(Value::List(l)) => l.lock().unwrap().clone(),
                _ => return Err(CocotteError::type_err("csv.stringify() requires a list of lists")),
            };
            let mut out = String::new();
            for row in rows {
                let fields: Vec<String> = match row {
                    Value::List(l) => l.lock().unwrap().iter().map(|v| csv_escape(&v.to_display())).collect(),
                    v => vec![csv_escape(&v.to_display())],
                };
                out.push_str(&fields.join(","));
                out.push('\n');
            }
            Ok(Value::Str(out))
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn parse_csv(text: &str) -> Vec<Value> {
    text.lines().map(|line| {
        let fields = parse_csv_line(line);
        Value::List(Arc::new(Mutex::new(fields.into_iter().map(Value::Str).collect())))
    }).filter(|_| true).collect()
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '"' if !in_quotes => { in_quotes = true; }
            '"' if in_quotes => {
                if chars.get(i + 1) == Some(&'"') { cur.push('"'); i += 1; }
                else { in_quotes = false; }
            }
            ',' if !in_quotes => { fields.push(cur.clone()); cur.clear(); }
            c => cur.push(c),
        }
        i += 1;
    }
    fields.push(cur);
    fields
}

// ── crypto module ─────────────────────────────────────────────────────────────
// Pure-Rust implementations — no extra deps.

fn make_crypto_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    ns.insert("sha256".into(), Value::NativeFunction(NativeFunction {
        name: "crypto.sha256".into(), arity: Some(1),
        func: Arc::new(|args| {
            let s = args.first().map(|v| v.to_display()).unwrap_or_default();
            let hash = sha256(s.as_bytes());
            Ok(Value::Str(hex(&hash)))
        }),
    }));

    ns.insert("md5".into(), Value::NativeFunction(NativeFunction {
        name: "crypto.md5".into(), arity: Some(1),
        func: Arc::new(|args| {
            let s = args.first().map(|v| v.to_display()).unwrap_or_default();
            let hash = md5(s.as_bytes());
            Ok(Value::Str(hex(&hash)))
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

// ── base64 module ─────────────────────────────────────────────────────────────

fn make_base64_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    ns.insert("encode".into(), Value::NativeFunction(NativeFunction {
        name: "base64.encode".into(), arity: Some(1),
        func: Arc::new(|args| {
            let s = args.first().map(|v| v.to_display()).unwrap_or_default();
            Ok(Value::Str(base64_encode(s.as_bytes())))
        }),
    }));

    ns.insert("decode".into(), Value::NativeFunction(NativeFunction {
        name: "base64.decode".into(), arity: Some(1),
        func: Arc::new(|args| {
            let s = args.first().map(|v| v.to_display()).unwrap_or_default();
            let bytes = base64_decode(&s)
                .map_err(|e| CocotteError::runtime(&format!("base64.decode: {}", e)))?;
            Ok(Value::Str(String::from_utf8_lossy(&bytes).to_string()))
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

// ── Pure-Rust crypto / encoding helpers ───────────────────────────────────────

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// SHA-256 — pure Rust, no external crate
fn sha256(data: &[u8]) -> [u8; 32] {
    // Round constants
    const K: [u32; 64] = [
        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,
        0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19,
    ];
    // Pre-processing: padding
    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 { msg.push(0); }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    // Process each 512-bit chunk
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[i*4], chunk[i*4+1], chunk[i*4+2], chunk[i*4+3]]);
        }
        for i in 16..64 {
            let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3);
            let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10);
            w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }
        let [mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut hh] =
            [h[0],h[1],h[2],h[3],h[4],h[5],h[6],h[7]];
        for i in 0..64 {
            let s1  = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch  = (e & f) ^ ((!e) & g);
            let tmp1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0  = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let tmp2 = s0.wrapping_add(maj);
            hh=g; g=f; f=e; e=d.wrapping_add(tmp1);
            d=c; c=b; b=a; a=tmp1.wrapping_add(tmp2);
        }
        h[0]=h[0].wrapping_add(a); h[1]=h[1].wrapping_add(b);
        h[2]=h[2].wrapping_add(c); h[3]=h[3].wrapping_add(d);
        h[4]=h[4].wrapping_add(e); h[5]=h[5].wrapping_add(f);
        h[6]=h[6].wrapping_add(g); h[7]=h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for (i, &v) in h.iter().enumerate() {
        out[i*4..i*4+4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

/// MD5 — pure Rust
fn md5(data: &[u8]) -> [u8; 16] {
    const S: [u32; 64] = [
        7,12,17,22,7,12,17,22,7,12,17,22,7,12,17,22,
        5, 9,14,20,5, 9,14,20,5, 9,14,20,5, 9,14,20,
        4,11,16,23,4,11,16,23,4,11,16,23,4,11,16,23,
        6,10,15,21,6,10,15,21,6,10,15,21,6,10,15,21,
    ];
    const K: [u32; 64] = [
        0xd76aa478,0xe8c7b756,0x242070db,0xc1bdceee,0xf57c0faf,0x4787c62a,0xa8304613,0xfd469501,
        0x698098d8,0x8b44f7af,0xffff5bb1,0x895cd7be,0x6b901122,0xfd987193,0xa679438e,0x49b40821,
        0xf61e2562,0xc040b340,0x265e5a51,0xe9b6c7aa,0xd62f105d,0x02441453,0xd8a1e681,0xe7d3fbc8,
        0x21e1cde6,0xc33707d6,0xf4d50d87,0x455a14ed,0xa9e3e905,0xfcefa3f8,0x676f02d9,0x8d2a4c8a,
        0xfffa3942,0x8771f681,0x6d9d6122,0xfde5380c,0xa4beea44,0x4bdecfa9,0xf6bb4b60,0xbebfbc70,
        0x289b7ec6,0xeaa127fa,0xd4ef3085,0x04881d05,0xd9d4d039,0xe6db99e5,0x1fa27cf8,0xc4ac5665,
        0xf4292244,0x432aff97,0xab9423a7,0xfc93a039,0x655b59c3,0x8f0ccc92,0xffeff47d,0x85845dd1,
        0x6fa87e4f,0xfe2ce6e0,0xa3014314,0x4e0811a1,0xf7537e82,0xbd3af235,0x2ad7d2bb,0xeb86d391,
    ];
    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 { msg.push(0); }
    msg.extend_from_slice(&bit_len.to_le_bytes());
    let (mut a0,mut b0,mut c0,mut d0): (u32,u32,u32,u32) =
        (0x67452301,0xefcdab89,0x98badcfe,0x10325476);
    for chunk in msg.chunks(64) {
        let mut m = [0u32; 16];
        for i in 0..16 {
            m[i] = u32::from_le_bytes([chunk[i*4],chunk[i*4+1],chunk[i*4+2],chunk[i*4+3]]);
        }
        let (mut a,mut b,mut c,mut d) = (a0,b0,c0,d0);
        for i in 0u32..64 {
            let (f, g) = match i {
                0..=15  => ((b & c) | ((!b) & d),          i),
                16..=31 => ((d & b) | ((!d) & c),     (5*i+1) % 16),
                32..=47 => (b ^ c ^ d,                (3*i+5) % 16),
                _       => (c ^ (b | (!d)),              (7*i) % 16),
            };
            let tmp = d; d = c; c = b;
            b = b.wrapping_add(
                (a.wrapping_add(f).wrapping_add(K[i as usize]).wrapping_add(m[g as usize]))
                    .rotate_left(S[i as usize])
            );
            a = tmp;
        }
        a0=a0.wrapping_add(a); b0=b0.wrapping_add(b);
        c0=c0.wrapping_add(c); d0=d0.wrapping_add(d);
    }
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&a0.to_le_bytes());
    out[4..8].copy_from_slice(&b0.to_le_bytes());
    out[8..12].copy_from_slice(&c0.to_le_bytes());
    out[12..16].copy_from_slice(&d0.to_le_bytes());
    out
}

/// Base64 encode — no external crate
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n  = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[((n >> 18) & 63) as usize] as char);
        out.push(CHARS[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 { out.push(CHARS[((n >> 6) & 63) as usize] as char); } else { out.push('='); }
        if chunk.len() > 2 { out.push(CHARS[(n & 63)        as usize] as char); } else { out.push('='); }
    }
    out
}

fn base64_decode(s: &str) -> std::result::Result<Vec<u8>, String> {
    const DEC: [i8; 128] = {
        let mut t = [-1i8; 128];
        let enc = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0usize;
        while i < 64 { t[enc[i] as usize] = i as i8; i += 1; }
        t
    };
    let s = s.trim_end_matches('=');
    let mut out = Vec::new();
    let bytes: Vec<u8> = s.bytes().collect();
    for chunk in bytes.chunks(4) {
        let mut vals = [0u32; 4];
        let mut cnt = 0;
        for (i, &b) in chunk.iter().enumerate() {
            if b as usize >= 128 || DEC[b as usize] < 0 {
                return Err(format!("invalid base64 character: {}", b as char));
            }
            vals[i] = DEC[b as usize] as u32;
            cnt = i + 1;
        }
        let n = (vals[0] << 18) | (vals[1] << 12) | (vals[2] << 6) | vals[3];
        out.push((n >> 16) as u8);
        if cnt > 2 { out.push((n >> 8) as u8); }
        if cnt > 3 { out.push(n as u8); }
    }
    Ok(out)
}
