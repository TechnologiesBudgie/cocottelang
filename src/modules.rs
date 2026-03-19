// modules.rs — Module and library loader for Cocotte
// Handles `module add "name"` and `library add "path/lib.cotlib"`

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use crate::value::{Value, NativeFunction};
use crate::error::{CocotteError, Result};

/// Load a built-in or file-based module by name
/// Returns a Value::Module containing the module's namespace
pub fn load_module(name: &str, project_root: &Path) -> Result<Value> {
    match name {
        "charlotte" => Ok(make_charlotte_module()),
        "math" => Ok(make_math_module()),
        "network" => Ok(make_network_stub_module()),
        "json" => Ok(make_json_module()),
        "os" => Ok(make_os_module()),
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
    let mut ns: HashMap<String, Value> = HashMap::new();

    macro_rules! charlotte_fn {
        ($name:expr, $arity:expr, $body:expr) => {
            ns.insert($name.to_string(), Value::NativeFunction(NativeFunction {
                name: format!("charlotte.{}", $name),
                arity: $arity,
                func: Arc::new($body),
            }));
        };
    }

    charlotte_fn!("create_window", Some(3), |args| {
        let title = args.get(0).map(|v| v.to_display()).unwrap_or_else(|| "Window".to_string());
        let width  = args.get(1).map(|v| v.to_display()).unwrap_or_else(|| "800".to_string());
        let height = args.get(2).map(|v| v.to_display()).unwrap_or_else(|| "600".to_string());
        println!("[Charlotte] Creating window '{}' ({}x{})", title, width, height);
        // Return a handle map representing the window
        let mut handle: HashMap<String, Value> = HashMap::new();
        handle.insert("title".to_string(), Value::Str(title));
        handle.insert("width".to_string(),  Value::Str(width));
        handle.insert("height".to_string(), Value::Str(height));
        handle.insert("_type".to_string(),  Value::Str("window".to_string()));
        Ok(Value::Map(Arc::new(Mutex::new(handle))))
    });

    charlotte_fn!("add_button", Some(3), |args| {
        let _window = args.get(0);
        let label = args.get(1).map(|v| v.to_display()).unwrap_or_default();
        println!("[Charlotte] Adding button '{}'", label);
        let mut handle: HashMap<String, Value> = HashMap::new();
        handle.insert("label".to_string(), Value::Str(label));
        handle.insert("_type".to_string(), Value::Str("button".to_string()));
        // Store the callback
        if let Some(cb) = args.get(2) {
            handle.insert("_callback".to_string(), cb.clone());
        }
        Ok(Value::Map(Arc::new(Mutex::new(handle))))
    });

    charlotte_fn!("add_label", Some(2), |args| {
        let _window = args.get(0);
        let text = args.get(1).map(|v| v.to_display()).unwrap_or_default();
        println!("[Charlotte] Adding label '{}'", text);
        let mut handle: HashMap<String, Value> = HashMap::new();
        handle.insert("text".to_string(), Value::Str(text));
        handle.insert("_type".to_string(), Value::Str("label".to_string()));
        Ok(Value::Map(Arc::new(Mutex::new(handle))))
    });

    charlotte_fn!("add_input", Some(2), |args| {
        let _window = args.get(0);
        let placeholder = args.get(1).map(|v| v.to_display()).unwrap_or_default();
        println!("[Charlotte] Adding input field '{}'", placeholder);
        let mut handle: HashMap<String, Value> = HashMap::new();
        handle.insert("placeholder".to_string(), Value::Str(placeholder));
        handle.insert("_type".to_string(), Value::Str("input".to_string()));
        Ok(Value::Map(Arc::new(Mutex::new(handle))))
    });

    charlotte_fn!("run", Some(1), |args| {
        let title = args.get(0)
            .and_then(|v| match v {
                Value::Map(m) => m.lock().unwrap().get("title").cloned(),
                _ => None,
            })
            .map(|v| v.to_display())
            .unwrap_or_else(|| "App".to_string());
        println!("[Charlotte] Running app '{}' — GUI event loop started (stub)", title);
        println!("[Charlotte] In a full build, this launches the native GUI window.");
        Ok(Value::Nil)
    });

    charlotte_fn!("set_title", Some(2), |args| {
        if let Some(Value::Map(m)) = args.get(0) {
            if let Some(Value::Str(title)) = args.get(1) {
                m.lock().unwrap().insert("title".to_string(), Value::Str(title.clone()));
                println!("[Charlotte] Window title set to '{}'", title);
            }
        }
        Ok(Value::Nil)
    });

    Value::Module(Arc::new(Mutex::new(ns)))
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
