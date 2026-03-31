// modules.rs — Module and library loader for Cocotte
// Handles `module add "name"` and `library add "path/lib.cotlib"`
//
// Built-in modules (always available, no install needed):
//   charlotte, math, json, os, http, sqlite, threading, parallel, ai
//
// Stdlib modules (68 .cotlib files, embedded at compile time):
//   strings, time, regex, path, fs, dates, hash, crypto, collections,
//   sort, statistics, validation, url, uuid, csv, json_schema, etc.
//
// Third-party modules live in <project>/modules/<name>.cotmod
// Local libraries live in <project>/libraries/<name>.cotlib

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, atomic::{AtomicUsize, Ordering}};
use crate::value::{Value, NativeFunction};
use crate::error::{CocotteError, Result};
use rusqlite::Connection;

// ── Embedded stdlib (compile-time) ────────────────────────────────────────────

macro_rules! embed_stdlib {
    ($( $name:literal => $path:literal ),* $(,)?) => {
        fn stdlib_source(name: &str) -> Option<&'static str> {
            match name {
                $( $name => Some(include_str!($path)), )*
                _ => None,
            }
        }
    };
}

embed_stdlib! {
    "strings"     => "stdlib/strings.cotlib",
    "time"        => "stdlib/time.cotlib",
    "regex"       => "stdlib/regex.cotlib",
    "path"        => "stdlib/path.cotlib",
    "fs"          => "stdlib/fs.cotlib",
    "dates"       => "stdlib/dates.cotlib",
    "hash"        => "stdlib/hash.cotlib",
    "crypto"      => "stdlib/crypto.cotlib",
    "collections" => "stdlib/collections.cotlib",
    "sort"        => "stdlib/sort.cotlib",
    "statistics"  => "stdlib/statistics.cotlib",
    "validation"  => "stdlib/validation.cotlib",
    "url"         => "stdlib/url.cotlib",
    "uuid"        => "stdlib/uuid.cotlib",
    "csv"         => "stdlib/csv.cotlib",
    "json_schema" => "stdlib/json_schema.cotlib",
    "cli"         => "stdlib/cli.cotlib",
    "color_utils" => "stdlib/color_utils.cotlib",
    "colors"      => "stdlib/colors.cotlib",
    "complex"     => "stdlib/complex.cotlib",
    "config"      => "stdlib/config.cotlib",
    "cache"       => "stdlib/cache.cotlib",
    "iter"        => "stdlib/iter.cotlib",
    "clipboard"   => "stdlib/clipboard.cotlib",
    "git"         => "stdlib/git.cotlib",
    "notify"      => "stdlib/notify.cotlib",
    "template"    => "stdlib/template.cotlib",
    "geometry"    => "stdlib/geometry.cotlib",
    "stack"       => "stdlib/stack.cotlib",
    "graph"       => "stdlib/graph.cotlib",
    "process"     => "stdlib/process.cotlib",
    "text"        => "stdlib/text.cotlib",
    "set"         => "stdlib/set.cotlib",
    "test"        => "stdlib/test.cotlib",
    "assert"      => "stdlib/assert.cotlib",
    "env"         => "stdlib/env.cotlib",
    "router"      => "stdlib/router.cotlib",
    "rate_limit"  => "stdlib/rate_limit.cotlib",
    "markdown"    => "stdlib/markdown.cotlib",
    "events"      => "stdlib/events.cotlib",
    "ini"         => "stdlib/ini.cotlib",
    "systeminfo"  => "stdlib/systeminfo.cotlib",
    "scheduler"   => "stdlib/scheduler.cotlib",
    "logging"     => "stdlib/logging.cotlib",
    "fmt"         => "stdlib/fmt.cotlib",
    "functional"  => "stdlib/functional.cotlib",
    "middleware"  => "stdlib/middleware.cotlib",
    "html"        => "stdlib/html.cotlib",
    "matrices"    => "stdlib/matrices.cotlib",
    "args"        => "stdlib/args.cotlib",
    "base64"      => "stdlib/base64.cotlib",
    "db"          => "stdlib/db.cotlib",
    "deque"       => "stdlib/deque.cotlib",
    "docker"      => "stdlib/docker.cotlib",
    "dotenv"      => "stdlib/dotenv.cotlib",
    "heap"        => "stdlib/heap.cotlib",
    "i18n"        => "stdlib/i18n.cotlib",
    "image"       => "stdlib/image.cotlib",
    "net"         => "stdlib/net.cotlib",
    "passwords"   => "stdlib/passwords.cotlib",
    "pdf"         => "stdlib/pdf.cotlib",
    "pipeline"    => "stdlib/pipeline.cotlib",
    "queue"       => "stdlib/queue.cotlib",
    "random"      => "stdlib/random.cotlib",
    "search"      => "stdlib/search.cotlib",
    "state"       => "stdlib/state.cotlib",
    "terminal"    => "stdlib/terminal.cotlib",
    "units"       => "stdlib/units.cotlib",
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn load_module(name: &str, project_root: &Path) -> Result<Value> {
    match name {
        // Core native modules (implemented in Rust)
        "charlotte"  => Ok(make_charlotte_module()),
        "math"       => Ok(make_math_module()),
        "json"       => Ok(make_json_module()),
        "os"         => Ok(make_os_module()),
        "http"       => Ok(make_http_module()),
        "sqlite"     => Ok(make_sqlite_module()),
        "threading"  => Ok(make_threading_module()),
        "parallel"   => Ok(make_parallel_module()),
        "ai"         => Ok(make_ai_module()),
        "network"    => Ok(make_network_stub_module()),

        // Convenience aliases → native modules
        "http_client" | "net_client" => Ok(make_http_module()),
        "ai_helpers" | "ai_utils" | "ai_lib" => Ok(make_ai_module()),

        // Stdlib .cotlib modules — embedded at compile time
        name if stdlib_source(name).is_some() => {
            let src = stdlib_source(name).unwrap();
            load_cotlib_source(src, name)
        }

        // Project-local .cotmod files
        _ => {
            let mod_path = project_root.join("modules").join(format!("{}.cotmod", name));
            if mod_path.exists() {
                return load_cotmod_file(&mod_path);
            }
            // Legacy: compiler/stdlib/<name>/module.cotlib (source-tree layout)
            let legacy = Path::new("compiler/stdlib").join(name).join("module.cotlib");
            if legacy.exists() {
                return load_cotmod_file(&legacy);
            }
            Err(CocotteError::module_err(&format!(
                "Module '{}' not found.\n  Built-in: charlotte math json os http sqlite threading parallel ai\n  Stdlib: strings time regex path fs dates hash crypto sort statistics\n          validation url uuid csv json_schema cli collections and 50+ more\n  Install local module: cocotte add my_module.cotmod",
                name
            )))
        }
    }
}

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
    let direct_path = project_root.join(path);
    if direct_path.exists() {
        return load_cotmod_file(&direct_path);
    }
    let lib_path = project_root.join("libraries").join(path);
    if lib_path.exists() {
        return load_cotmod_file(&lib_path);
    }
    // Try just the filename
    let fname = Path::new(path).file_name().unwrap_or_default();
    let by_name = project_root.join("libraries").join(fname);
    if by_name.exists() {
        return load_cotmod_file(&by_name);
    }
    Err(CocotteError::module_err(&format!(
        "Library file '{}' not found (tried '{}' and '{}')",
        path, direct_path.display(), lib_path.display()
    )))
}

/// Load a .cotlib / .cotmod from source code string.
fn load_cotlib_source(source: &str, name: &str) -> Result<Value> {
    let mut lexer = crate::lexer::Lexer::new(source);
    let tokens = lexer.tokenize().map_err(|e| {
        CocotteError::module_err(&format!("Syntax error in module '{}': {}", name, e))
    })?;
    let mut parser = crate::parser::Parser::new(tokens);
    let ast = parser.parse().map_err(|e| {
        CocotteError::module_err(&format!("Parse error in module '{}': {}", name, e))
    })?;
    let mut interp = crate::interpreter::Interpreter::new();
    // Run in a fresh env; ignore Return signals (some libs use `return {...}` at top level)
    match interp.run(&ast) {
        Ok(_) => {}
        Err(ref e) if e.is_signal() => {}
        Err(e) => return Err(CocotteError::module_err(&format!(
            "Runtime error in module '{}': {}", name, e
        ))),
    }
    let exports = interp.export_namespace();
    Ok(Value::Module(Arc::new(Mutex::new(exports))))
}

fn load_cotmod_file(path: &Path) -> Result<Value> {
    let source = std::fs::read_to_string(path)?;
    load_cotlib_source(&source, &path.display().to_string())
}

// ── Charlotte GUI module ──────────────────────────────────────────────────────

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
                eprintln!("charlotte: GUI not available (built without --features gui)");
                Ok(Value::Nil)
            }),
        }));
        m.insert("set_renderer".to_string(), Value::NativeFunction(NativeFunction {
            name: "charlotte.set_renderer".to_string(),
            arity: Some(1),
            func: Arc::new(|_| {
                eprintln!("charlotte: set_renderer() has no effect (gui feature disabled)");
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

// ── Math module ───────────────────────────────────────────────────────────────

fn make_math_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    ns.insert("PI".to_string(),  Value::Number(std::f64::consts::PI));
    ns.insert("E".to_string(),   Value::Number(std::f64::consts::E));
    ns.insert("TAU".to_string(), Value::Number(std::f64::consts::TAU));
    ns.insert("INF".to_string(), Value::Number(f64::INFINITY));
    ns.insert("NAN".to_string(), Value::Number(f64::NAN));

    macro_rules! math_fn1 {
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

    math_fn1!("sin",   f64::sin);
    math_fn1!("cos",   f64::cos);
    math_fn1!("tan",   f64::tan);
    math_fn1!("asin",  f64::asin);
    math_fn1!("acos",  f64::acos);
    math_fn1!("atan",  f64::atan);
    math_fn1!("log",   f64::ln);
    math_fn1!("log2",  f64::log2);
    math_fn1!("log10", f64::log10);
    math_fn1!("exp",   f64::exp);
    math_fn1!("sqrt",  f64::sqrt);
    math_fn1!("cbrt",  f64::cbrt);
    math_fn1!("floor", f64::floor);
    math_fn1!("ceil",  f64::ceil);
    math_fn1!("round", f64::round);
    math_fn1!("abs",   f64::abs);
    math_fn1!("sign",  f64::signum);
    math_fn1!("trunc", f64::trunc);
    math_fn1!("fract", f64::fract);
    math_fn1!("degrees", f64::to_degrees);
    math_fn1!("radians", f64::to_radians);

    macro_rules! math_fn2 {
        ($name:expr, |$a:ident, $b:ident| $body:expr) => {
            ns.insert($name.to_string(), Value::NativeFunction(NativeFunction {
                name: format!("math.{}", $name),
                arity: Some(2),
                func: Arc::new(|args| match (&args[0], &args[1]) {
                    (Value::Number($a), Value::Number($b)) => Ok(Value::Number($body)),
                    _ => Err(CocotteError::type_err(&format!("math.{}() requires two numbers", $name))),
                }),
            }));
        };
    }

    math_fn2!("pow",   |a, b| a.powf(*b));
    math_fn2!("max",   |a, b| a.max(*b));
    math_fn2!("min",   |a, b| a.min(*b));
    math_fn2!("hypot", |a, b| a.hypot(*b));
    math_fn2!("atan2", |a, b| a.atan2(*b));
    math_fn2!("log_base", |a, b| a.log(*b));

    ns.insert("clamp".to_string(), Value::NativeFunction(NativeFunction {
        name: "math.clamp".to_string(),
        arity: Some(3),
        func: Arc::new(|args| match (&args[0], &args[1], &args[2]) {
            (Value::Number(v), Value::Number(lo), Value::Number(hi)) =>
                Ok(Value::Number(v.clamp(*lo, *hi))),
            _ => Err(CocotteError::type_err("math.clamp(v, lo, hi) requires three numbers")),
        }),
    }));

    ns.insert("is_nan".to_string(), Value::NativeFunction(NativeFunction {
        name: "math.is_nan".to_string(),
        arity: Some(1),
        func: Arc::new(|args| match &args[0] {
            Value::Number(n) => Ok(Value::Bool(n.is_nan())),
            _ => Ok(Value::Bool(true)),
        }),
    }));

    ns.insert("is_finite".to_string(), Value::NativeFunction(NativeFunction {
        name: "math.is_finite".to_string(),
        arity: Some(1),
        func: Arc::new(|args| match &args[0] {
            Value::Number(n) => Ok(Value::Bool(n.is_finite())),
            _ => Ok(Value::Bool(false)),
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

// ── JSON module ───────────────────────────────────────────────────────────────

fn make_json_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    ns.insert("parse".to_string(), Value::NativeFunction(NativeFunction {
        name: "json.parse".to_string(),
        arity: Some(1),
        func: Arc::new(|args| match &args[0] {
            Value::Str(s) => {
                let v: serde_json::Value = serde_json::from_str(s)
                    .map_err(|e| CocotteError::runtime(&format!("json.parse error: {}", e)))?;
                Ok(json_to_cocotte(v))
            }
            _ => Err(CocotteError::type_err("json.parse() requires a string")),
        }),
    }));

    ns.insert("stringify".to_string(), Value::NativeFunction(NativeFunction {
        name: "json.stringify".to_string(),
        arity: Some(1),
        func: Arc::new(|args| {
            let j = cocotte_to_json(&args[0]);
            serde_json::to_string(&j)
                .map(Value::Str)
                .map_err(|e| CocotteError::runtime(&format!("json.stringify error: {}", e)))
        }),
    }));

    ns.insert("stringify_pretty".to_string(), Value::NativeFunction(NativeFunction {
        name: "json.stringify_pretty".to_string(),
        arity: Some(1),
        func: Arc::new(|args| {
            let j = cocotte_to_json(&args[0]);
            serde_json::to_string_pretty(&j)
                .map(Value::Str)
                .map_err(|e| CocotteError::runtime(&format!("json.stringify_pretty error: {}", e)))
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

pub fn json_to_cocotte(v: serde_json::Value) -> Value {
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

pub fn cocotte_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Nil       => serde_json::Value::Null,
        Value::Bool(b)   => serde_json::Value::Bool(*b),
        Value::Number(n) => {
            // Emit integer-valued floats as JSON integers (1 not 1.0)
            if n.fract() == 0.0 && n.abs() < 1e15 {
                serde_json::Value::Number(serde_json::Number::from(*n as i64))
            } else {
                serde_json::json!(n)
            }
        }
        Value::Str(s)    => serde_json::Value::String(s.clone()),
        Value::List(l)   => {
            let items: Vec<serde_json::Value> = l.lock().unwrap().iter()
                .map(cocotte_to_json).collect();
            serde_json::Value::Array(items)
        }
        Value::Map(m) => {
            let obj: serde_json::Map<String, serde_json::Value> = m.lock().unwrap()
                .iter().map(|(k, v)| (k.clone(), cocotte_to_json(v))).collect();
            serde_json::Value::Object(obj)
        }
        other => serde_json::Value::String(other.to_display()),
    }
}

// ── OS module ─────────────────────────────────────────────────────────────────

fn make_os_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    ns.insert("platform".to_string(), Value::NativeFunction(NativeFunction {
        name: "os.platform".to_string(),
        arity: Some(0),
        func: Arc::new(|_| {
            let p = if cfg!(target_os = "windows") { "windows" }
                else if cfg!(target_os = "macos")  { "macos" }
                else if cfg!(target_os = "linux")   { "linux" }
                else { "unknown" };
            Ok(Value::Str(p.to_string()))
        }),
    }));

    ns.insert("arch".to_string(), Value::NativeFunction(NativeFunction {
        name: "os.arch".to_string(),
        arity: Some(0),
        func: Arc::new(|_| {
            let a = if cfg!(target_arch = "x86_64")  { "x86_64" }
                else if cfg!(target_arch = "aarch64") { "aarch64" }
                else if cfg!(target_arch = "arm")     { "armv7" }
                else if cfg!(target_arch = "x86")     { "i686" }
                else if cfg!(target_arch = "riscv64") { "riscv64" }
                else { "unknown" };
            Ok(Value::Str(a.to_string()))
        }),
    }));

    ns.insert("exec".to_string(), Value::NativeFunction(NativeFunction {
        name: "os.exec".to_string(),
        arity: Some(1),
        func: Arc::new(|args| match &args[0] {
            Value::Str(cmd) => {
                let out = if cfg!(target_os = "windows") {
                    std::process::Command::new("cmd").args(["/C", cmd]).output()
                } else {
                    std::process::Command::new("sh").args(["-c", cmd]).output()
                };
                match out {
                    Ok(o) => Ok(Value::Str(String::from_utf8_lossy(&o.stdout).to_string())),
                    Err(e) => Err(CocotteError::io_err(&format!("os.exec failed: {}", e))),
                }
            }
            _ => Err(CocotteError::type_err("os.exec() requires a string")),
        }),
    }));

    ns.insert("exec_status".to_string(), Value::NativeFunction(NativeFunction {
        name: "os.exec_status".to_string(),
        arity: Some(1),
        func: Arc::new(|args| match &args[0] {
            Value::Str(cmd) => {
                let st = if cfg!(target_os = "windows") {
                    std::process::Command::new("cmd").args(["/C", cmd]).status()
                } else {
                    std::process::Command::new("sh").args(["-c", cmd]).status()
                };
                match st {
                    Ok(s) => Ok(Value::Number(s.code().unwrap_or(-1) as f64)),
                    Err(e) => Err(CocotteError::io_err(&format!("os.exec_status failed: {}", e))),
                }
            }
            _ => Err(CocotteError::type_err("os.exec_status() requires a string")),
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

    ns.insert("home".to_string(), Value::NativeFunction(NativeFunction {
        name: "os.home".to_string(),
        arity: Some(0),
        func: Arc::new(|_| {
            Ok(dirs::home_dir()
                .map(|p| Value::Str(p.display().to_string()))
                .unwrap_or(Value::Nil))
        }),
    }));

    ns.insert("env_get".to_string(), Value::NativeFunction(NativeFunction {
        name: "os.env_get".to_string(),
        arity: Some(1),
        func: Arc::new(|args| match &args[0] {
            Value::Str(k) => Ok(std::env::var(k).map(Value::Str).unwrap_or(Value::Nil)),
            _ => Err(CocotteError::type_err("os.env_get() requires a string")),
        }),
    }));

    ns.insert("env_set".to_string(), Value::NativeFunction(NativeFunction {
        name: "os.env_set".to_string(),
        arity: Some(2),
        func: Arc::new(|args| match (&args[0], &args[1]) {
            (Value::Str(k), v) => {
                std::env::set_var(k, v.to_display());
                Ok(Value::Nil)
            }
            _ => Err(CocotteError::type_err("os.env_set(key, value) requires strings")),
        }),
    }));

    ns.insert("args".to_string(), Value::NativeFunction(NativeFunction {
        name: "os.args".to_string(),
        arity: Some(0),
        func: Arc::new(|_| {
            let args: Vec<Value> = std::env::args().map(Value::Str).collect();
            Ok(Value::List(Arc::new(Mutex::new(args))))
        }),
    }));

    ns.insert("exit".to_string(), Value::NativeFunction(NativeFunction {
        name: "os.exit".to_string(),
        arity: None,
        func: Arc::new(|args| {
            let code = match args.first() {
                Some(Value::Number(n)) => *n as i32,
                _ => 0,
            };
            std::process::exit(code);
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

// ── HTTP module ───────────────────────────────────────────────────────────────

fn make_http_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    macro_rules! http_method {
        ($fn_name:expr, $method:expr, has_body = false) => {
            ns.insert($fn_name.into(), Value::NativeFunction(NativeFunction {
                name: format!("http.{}", $fn_name),
                arity: None,
                func: Arc::new(|args| {
                    let url = match args.first() {
                        Some(Value::Str(s)) => s.clone(),
                        _ => return Err(CocotteError::type_err(&format!("http.{}(url) requires a string URL", $fn_name))),
                    };
                    let mut req = ureq::request($method, &url);
                    if let Some(Value::Map(m)) = args.get(1) {
                        for (k, v) in m.lock().unwrap().iter() {
                            req = req.set(k, &v.to_display());
                        }
                    }
                    req.call()
                        .map_err(|e| CocotteError::runtime(&format!("http.{} failed: {}", $fn_name, e)))?
                        .into_string()
                        .map(Value::Str)
                        .map_err(|e| CocotteError::runtime(&format!("http.{} read failed: {}", $fn_name, e)))
                }),
            }));
        };
        ($fn_name:expr, $method:expr, has_body = true) => {
            ns.insert($fn_name.into(), Value::NativeFunction(NativeFunction {
                name: format!("http.{}", $fn_name),
                arity: None,
                func: Arc::new(|args| {
                    let url = match args.first() {
                        Some(Value::Str(s)) => s.clone(),
                        _ => return Err(CocotteError::type_err(&format!("http.{}(url, body) requires a string URL", $fn_name))),
                    };
                    let body = args.get(1).map(|v| v.to_display()).unwrap_or_default();
                    let mut req = ureq::request($method, &url);
                    if let Some(Value::Map(m)) = args.get(2) {
                        for (k, v) in m.lock().unwrap().iter() {
                            req = req.set(k, &v.to_display());
                        }
                    }
                    req.send_string(&body)
                        .map_err(|e| CocotteError::runtime(&format!("http.{} failed: {}", $fn_name, e)))?
                        .into_string()
                        .map(Value::Str)
                        .map_err(|e| CocotteError::runtime(&format!("http.{} read failed: {}", $fn_name, e)))
                }),
            }));
        };
    }

    http_method!("get",    "GET",    has_body = false);
    http_method!("delete", "DELETE", has_body = false);
    http_method!("post",   "POST",   has_body = true);
    http_method!("put",    "PUT",    has_body = true);
    http_method!("patch",  "PATCH",  has_body = true);

    // get_json
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
                for (k, v) in m.lock().unwrap().iter() {
                    req = req.set(k, &v.to_display());
                }
            }
            let text = req.call()
                .map_err(|e| CocotteError::runtime(&format!("http.get_json failed: {}", e)))?
                .into_string()
                .map_err(|e| CocotteError::runtime(&format!("http.get_json read failed: {}", e)))?;
            let v: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| CocotteError::runtime(&format!("JSON parse error: {}", e)))?;
            Ok(json_to_cocotte(v))
        }),
    }));

    // post_json
    ns.insert("post_json".into(), Value::NativeFunction(NativeFunction {
        name: "http.post_json".into(),
        arity: None,
        func: Arc::new(|args| {
            let url = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("http.post_json(url, value) requires a string URL")),
            };
            let body_val = args.get(1).cloned().unwrap_or(Value::Nil);
            let body_str = serde_json::to_string(&cocotte_to_json(&body_val))
                .map_err(|e| CocotteError::runtime(&format!("JSON serialise error: {}", e)))?;
            let mut req = ureq::post(&url).set("Content-Type", "application/json");
            if let Some(Value::Map(m)) = args.get(2) {
                for (k, v) in m.lock().unwrap().iter() {
                    req = req.set(k, &v.to_display());
                }
            }
            req.send_string(&body_str)
                .map_err(|e| CocotteError::runtime(&format!("http.post_json failed: {}", e)))?
                .into_string()
                .map(Value::Str)
                .map_err(|e| CocotteError::runtime(&format!("http.post_json read failed: {}", e)))
        }),
    }));

    // serve
    ns.insert("serve".into(), Value::NativeFunction(NativeFunction {
        name: "http.serve".into(),
        arity: Some(2),
        func: Arc::new(|args| {
            let port = match args.first() {
                Some(Value::Number(n)) => *n as u16,
                _ => return Err(CocotteError::type_err("http.serve(port, handler) — port must be a number")),
            };
            let handler = match args.get(1) {
                Some(Value::Function(f)) => f.clone(),
                _ => return Err(CocotteError::type_err("http.serve(port, handler) — handler must be a function")),
            };
            crate::http_server::run_server(port, handler).map(|_| Value::Nil).map(|_| Value::Nil).map(|_| Value::Nil)
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

// ── SQLite module ─────────────────────────────────────────────────────────────

fn make_sqlite_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    ns.insert("open".into(), Value::NativeFunction(NativeFunction {
        name: "sqlite.open".into(),
        arity: Some(1),
        func: Arc::new(|args| {
            let path = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("sqlite.open(path) requires a string")),
            };
            let conn = Connection::open(&path)
                .map_err(|e| CocotteError::runtime(&format!("sqlite.open failed: {}", e)))?;
            // Pragmas for reliability
            conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
                .map_err(|e| CocotteError::runtime(&format!("sqlite pragma error: {}", e)))?;
            let db_handle: HashMap<String, Value> = {
                let conn_arc = Arc::new(Mutex::new(conn));
                let mut h = HashMap::new();
                h.insert("__path".into(), Value::Str(path));
                h.insert("__conn".into(), Value::Str(format!("{:p}", Arc::as_ptr(&conn_arc))));

                // Store connection in global registry
                let id = SQLITE_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
                SQLITE_CONNECTIONS.get_or_init(|| Mutex::new(HashMap::new()))
                    .lock().unwrap()
                    .insert(id, conn_arc);
                h.insert("__id".into(), Value::Number(id as f64));
                h
            };
            Ok(Value::Map(Arc::new(Mutex::new(db_handle))))
        }),
    }));

    macro_rules! sqlite_op {
        ($fn_name:expr, $query_mode:expr) => {
            ns.insert($fn_name.into(), Value::NativeFunction(NativeFunction {
                name: format!("sqlite.{}", $fn_name),
                arity: Some(2),
                func: Arc::new(|args| {
                    let db_id = match args.first() {
                        Some(Value::Map(m)) => {
                            match m.lock().unwrap().get("__id") {
                                Some(Value::Number(n)) => *n as usize,
                                _ => return Err(CocotteError::runtime("sqlite: invalid db handle")),
                            }
                        }
                        _ => return Err(CocotteError::type_err(&format!("sqlite.{}(db, sql) requires a db handle", $fn_name))),
                    };
                    let sql = match args.get(1) {
                        Some(Value::Str(s)) => s.clone(),
                        _ => return Err(CocotteError::type_err(&format!("sqlite.{}(db, sql) requires a string SQL", $fn_name))),
                    };
                    let registry = SQLITE_CONNECTIONS.get_or_init(|| Mutex::new(HashMap::new()));
                    let registry = registry.lock().unwrap();
                    let conn_arc = registry.get(&db_id)
                        .ok_or_else(|| CocotteError::runtime("sqlite: db handle not found (already closed?)"))?
                        .clone();
                    drop(registry);
                    let conn = conn_arc.lock().unwrap();
                    sqlite_execute(&conn, &sql, $query_mode)
                }),
            }));
        };
    }

    sqlite_op!("exec",      0u8);  // 0 = exec (no results)
    sqlite_op!("query",     1u8);  // 1 = query (all rows)
    sqlite_op!("query_one", 2u8);  // 2 = query one row

    ns.insert("tables".into(), Value::NativeFunction(NativeFunction {
        name: "sqlite.tables".into(),
        arity: Some(1),
        func: Arc::new(|args| {
            let db_id = match args.first() {
                Some(Value::Map(m)) => match m.lock().unwrap().get("__id") {
                    Some(Value::Number(n)) => *n as usize,
                    _ => return Err(CocotteError::runtime("sqlite: invalid db handle")),
                },
                _ => return Err(CocotteError::type_err("sqlite.tables(db) requires a db handle")),
            };
            let registry = SQLITE_CONNECTIONS.get_or_init(|| Mutex::new(HashMap::new()));
            let registry = registry.lock().unwrap();
            let conn_arc = registry.get(&db_id)
                .ok_or_else(|| CocotteError::runtime("sqlite: db handle not found"))?
                .clone();
            drop(registry);
            let conn = conn_arc.lock().unwrap();
            sqlite_execute(&conn, "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name", 1u8)
                .map(|v| {
                    if let Value::List(rows) = v {
                        let names: Vec<Value> = rows.lock().unwrap().iter().filter_map(|row| {
                            if let Value::Map(m) = row {
                                m.lock().unwrap().get("name").cloned()
                            } else { None }
                        }).collect();
                        Value::List(Arc::new(Mutex::new(names)))
                    } else { Value::List(Arc::new(Mutex::new(Vec::new()))) }
                })
        }),
    }));

    ns.insert("close".into(), Value::NativeFunction(NativeFunction {
        name: "sqlite.close".into(),
        arity: Some(1),
        func: Arc::new(|args| {
            if let Some(Value::Map(m)) = args.first() {
                if let Some(Value::Number(n)) = m.lock().unwrap().get("__id") {
                    let id = *n as usize;
                    if let Some(reg) = SQLITE_CONNECTIONS.get() {
                        reg.lock().unwrap().remove(&id);
                    }
                }
            }
            Ok(Value::Nil)
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

static SQLITE_DB_COUNTER: AtomicUsize = AtomicUsize::new(0);
static SQLITE_CONNECTIONS: OnceLock<Mutex<HashMap<usize, Arc<Mutex<Connection>>>>> = OnceLock::new();

fn sqlite_execute(conn: &Connection, sql: &str, mode: u8) -> Result<Value> {
    match mode {
        0 => {
            conn.execute_batch(sql)
                .map(|_| Value::Nil)
                .map_err(|e| CocotteError::runtime(&format!("sqlite.exec error: {}", e)))
        }
        1 => {
            let mut stmt = conn.prepare(sql)
                .map_err(|e| CocotteError::runtime(&format!("sqlite.query prepare error: {}", e)))?;
            let cols: Vec<String> = stmt.column_names().into_iter().map(|s| s.to_string()).collect();
            let rows_iter = stmt.query_map([], |row| {
                let mut map = HashMap::new();
                for (i, col) in cols.iter().enumerate() {
                    let val: rusqlite::types::Value = row.get(i)?;
                    let cv = match val {
                        rusqlite::types::Value::Null    => Value::Nil,
                        rusqlite::types::Value::Integer(n) => Value::Number(n as f64),
                        rusqlite::types::Value::Real(n)    => Value::Number(n),
                        rusqlite::types::Value::Text(s)    => Value::Str(s),
                        rusqlite::types::Value::Blob(b)    => Value::Str(String::from_utf8_lossy(&b).into_owned()),
                    };
                    map.insert(col.clone(), cv);
                }
                Ok(map)
            }).map_err(|e| CocotteError::runtime(&format!("sqlite.query error: {}", e)))?;

            let mut rows: Vec<Value> = Vec::new();
            for row in rows_iter {
                let m = row.map_err(|e| CocotteError::runtime(&format!("sqlite row error: {}", e)))?;
                rows.push(Value::Map(Arc::new(Mutex::new(m))));
            }
            Ok(Value::List(Arc::new(Mutex::new(rows))))
        }
        2 => {
            let result = sqlite_execute(conn, sql, 1)?;
            if let Value::List(rows) = result {
                Ok(rows.lock().unwrap().first().cloned().unwrap_or(Value::Nil))
            } else {
                Ok(Value::Nil)
            }
        }
        _ => unreachable!()
    }
}

// ── Threading module ──────────────────────────────────────────────────────────
// Real join handles, channels, mutexes — all exposed to Cocotte code.

static THREAD_ID_COUNTER: AtomicUsize = AtomicUsize::new(1);

type ThreadRegistry = Mutex<HashMap<usize, std::thread::JoinHandle<Option<Value>>>>;
static THREAD_HANDLES: OnceLock<ThreadRegistry> = OnceLock::new();

fn thread_registry() -> &'static ThreadRegistry {
    THREAD_HANDLES.get_or_init(|| Mutex::new(HashMap::new()))
}

// Channel storage: sender and receiver stored by channel ID
type SenderStore  = Mutex<HashMap<usize, std::sync::mpsc::Sender<Value>>>;
type ReceiverStore = Mutex<HashMap<usize, std::sync::mpsc::Receiver<Value>>>;
static CHANNEL_SENDERS:   OnceLock<SenderStore>   = OnceLock::new();
static CHANNEL_RECEIVERS: OnceLock<ReceiverStore>  = OnceLock::new();
static CHANNEL_COUNTER:   AtomicUsize             = AtomicUsize::new(1);

fn make_threading_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    // threading.spawn(func) → number (thread id)
    // threading.spawn(func, arg) → number (thread id, func receives arg)
    ns.insert("spawn".into(), Value::NativeFunction(NativeFunction {
        name: "threading.spawn".into(),
        arity: None,
        func: Arc::new(|args| {
            let func = match args.first() {
                Some(Value::Function(f)) => f.clone(),
                _ => return Err(CocotteError::type_err("threading.spawn(func [, arg]) requires a function")),
            };
            let arg = args.get(1).cloned();
            let id = THREAD_ID_COUNTER.fetch_add(1, Ordering::SeqCst);

            let handle = std::thread::spawn(move || {
                let mut interp = crate::interpreter::Interpreter::new();
                let call_args = arg.map(|a| vec![a]).unwrap_or_default();
                match interp.call_function_pub(&func, call_args, None) {
                    Ok(v)  => Some(v),
                    Err(_) => None,
                }
            });

            thread_registry().lock().unwrap().insert(id, handle);
            Ok(Value::Number(id as f64))
        }),
    }));

    // threading.join(id) → value returned by the thread function (or nil)
    ns.insert("join".into(), Value::NativeFunction(NativeFunction {
        name: "threading.join".into(),
        arity: Some(1),
        func: Arc::new(|args| {
            let id = match args.first() {
                Some(Value::Number(n)) => *n as usize,
                _ => return Err(CocotteError::type_err("threading.join(id) requires a thread id (number)")),
            };
            let handle = thread_registry().lock().unwrap().remove(&id)
                .ok_or_else(|| CocotteError::runtime(&format!("threading.join: no thread with id {}", id)))?;
            match handle.join() {
                Ok(Some(v)) => Ok(v),
                Ok(None)    => Ok(Value::Nil),
                Err(_)      => Err(CocotteError::runtime("threading.join: thread panicked")),
            }
        }),
    }));

    // threading.sleep(seconds)
    ns.insert("sleep".into(), Value::NativeFunction(NativeFunction {
        name: "threading.sleep".into(),
        arity: Some(1),
        func: Arc::new(|args| match args.first() {
            Some(Value::Number(s)) => {
                std::thread::sleep(std::time::Duration::from_secs_f64(*s));
                Ok(Value::Nil)
            }
            _ => Err(CocotteError::type_err("threading.sleep(seconds) requires a number")),
        }),
    }));

    // threading.channel() → {"id": n}
    ns.insert("channel".into(), Value::NativeFunction(NativeFunction {
        name: "threading.channel".into(),
        arity: Some(0),
        func: Arc::new(|_| {
            let (tx, rx) = std::sync::mpsc::channel::<Value>();
            let id = CHANNEL_COUNTER.fetch_add(1, Ordering::SeqCst);
            CHANNEL_SENDERS.get_or_init(|| Mutex::new(HashMap::new()))
                .lock().unwrap().insert(id, tx);
            CHANNEL_RECEIVERS.get_or_init(|| Mutex::new(HashMap::new()))
                .lock().unwrap().insert(id, rx);
            let mut m = HashMap::new();
            m.insert("id".into(), Value::Number(id as f64));
            Ok(Value::Map(Arc::new(Mutex::new(m))))
        }),
    }));

    // threading.send(channel, value)
    ns.insert("send".into(), Value::NativeFunction(NativeFunction {
        name: "threading.send".into(),
        arity: Some(2),
        func: Arc::new(|args| {
            let id = match args.first() {
                Some(Value::Map(m)) => match m.lock().unwrap().get("id") {
                    Some(Value::Number(n)) => *n as usize,
                    _ => return Err(CocotteError::runtime("threading.send: invalid channel")),
                },
                _ => return Err(CocotteError::type_err("threading.send(channel, value)")),
            };
            let val = args.get(1).cloned().unwrap_or(Value::Nil);
            let senders = CHANNEL_SENDERS.get_or_init(|| Mutex::new(HashMap::new()));
            let lock = senders.lock().unwrap();
            let tx = lock.get(&id)
                .ok_or_else(|| CocotteError::runtime("threading.send: channel not found"))?;
            tx.send(val).map_err(|_| CocotteError::runtime("threading.send: receiver dropped"))?;
            Ok(Value::Nil)
        }),
    }));

    // threading.recv(channel) → value  (blocks)
    ns.insert("recv".into(), Value::NativeFunction(NativeFunction {
        name: "threading.recv".into(),
        arity: Some(1),
        func: Arc::new(|args| {
            let id = match args.first() {
                Some(Value::Map(m)) => match m.lock().unwrap().get("id") {
                    Some(Value::Number(n)) => *n as usize,
                    _ => return Err(CocotteError::runtime("threading.recv: invalid channel")),
                },
                _ => return Err(CocotteError::type_err("threading.recv(channel)")),
            };
            let receivers = CHANNEL_RECEIVERS.get_or_init(|| Mutex::new(HashMap::new()));
            // We can't hold the lock while blocking — peek the ptr instead
            let rx_ptr = {
                let lock = receivers.lock().unwrap();
                lock.get(&id)
                    .map(|r| r as *const std::sync::mpsc::Receiver<Value> as usize)
                    .ok_or_else(|| CocotteError::runtime("threading.recv: channel not found"))?
            };
            // Safety: the Receiver lives for the lifetime of CHANNEL_RECEIVERS
            // (static), and we never drop it while a recv is in progress because
            // we hold no mutable reference here.
            let rx = unsafe { &*(rx_ptr as *const std::sync::mpsc::Receiver<Value>) };
            rx.recv().map_err(|_| CocotteError::runtime("threading.recv: all senders dropped"))
        }),
    }));

    // threading.recv_timeout(channel, secs) → value or nil
    ns.insert("recv_timeout".into(), Value::NativeFunction(NativeFunction {
        name: "threading.recv_timeout".into(),
        arity: Some(2),
        func: Arc::new(|args| {
            let id = match args.first() {
                Some(Value::Map(m)) => match m.lock().unwrap().get("id") {
                    Some(Value::Number(n)) => *n as usize,
                    _ => return Err(CocotteError::runtime("threading.recv_timeout: invalid channel")),
                },
                _ => return Err(CocotteError::type_err("threading.recv_timeout(channel, secs)")),
            };
            let secs = match args.get(1) {
                Some(Value::Number(s)) => *s,
                _ => return Err(CocotteError::type_err("threading.recv_timeout: second arg must be seconds (number)")),
            };
            let receivers = CHANNEL_RECEIVERS.get_or_init(|| Mutex::new(HashMap::new()));
            let rx_ptr = {
                let lock = receivers.lock().unwrap();
                lock.get(&id)
                    .map(|r| r as *const std::sync::mpsc::Receiver<Value> as usize)
                    .ok_or_else(|| CocotteError::runtime("threading.recv_timeout: channel not found"))?
            };
            let rx = unsafe { &*(rx_ptr as *const std::sync::mpsc::Receiver<Value>) };
            match rx.recv_timeout(std::time::Duration::from_secs_f64(secs)) {
                Ok(v)  => Ok(v),
                Err(_) => Ok(Value::Nil),
            }
        }),
    }));

    // threading.mutex() → {"id": n}  — a simple binary mutex (lock/unlock)
    static MUTEX_COUNTER: AtomicUsize = AtomicUsize::new(1);
    type MutexStore = Mutex<HashMap<usize, Arc<Mutex<()>>>>;
    static MUTEXES: OnceLock<MutexStore> = OnceLock::new();

    ns.insert("mutex".into(), Value::NativeFunction(NativeFunction {
        name: "threading.mutex".into(),
        arity: Some(0),
        func: Arc::new(|_| {
            let id = MUTEX_COUNTER.fetch_add(1, Ordering::SeqCst);
            MUTEXES.get_or_init(|| Mutex::new(HashMap::new()))
                .lock().unwrap()
                .insert(id, Arc::new(Mutex::new(())));
            let mut m = HashMap::new();
            m.insert("id".into(), Value::Number(id as f64));
            Ok(Value::Map(Arc::new(Mutex::new(m))))
        }),
    }));

    // threading.with_lock(mutex, func)
    ns.insert("with_lock".into(), Value::NativeFunction(NativeFunction {
        name: "threading.with_lock".into(),
        arity: Some(2),
        func: Arc::new(|args| {
            let id = match args.first() {
                Some(Value::Map(m)) => match m.lock().unwrap().get("id") {
                    Some(Value::Number(n)) => *n as usize,
                    _ => return Err(CocotteError::runtime("threading.with_lock: invalid mutex")),
                },
                _ => return Err(CocotteError::type_err("threading.with_lock(mutex, func)")),
            };
            let func = match args.get(1) {
                Some(Value::Function(f)) => f.clone(),
                _ => return Err(CocotteError::type_err("threading.with_lock: second arg must be a function")),
            };
            let mutex_arc = {
                let store = MUTEXES.get_or_init(|| Mutex::new(HashMap::new()));
                store.lock().unwrap().get(&id).cloned()
                    .ok_or_else(|| CocotteError::runtime("threading.with_lock: mutex not found"))?
            };
            let _guard = mutex_arc.lock().unwrap();
            let mut interp = crate::interpreter::Interpreter::new();
            interp.call_function_pub(&func, vec![], None)
        }),
    }));

    // threading.num_cpus() → number
    ns.insert("num_cpus".into(), Value::NativeFunction(NativeFunction {
        name: "threading.num_cpus".into(),
        arity: Some(0),
        func: Arc::new(|_| {
            Ok(Value::Number(std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1) as f64))
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

// ── Parallel module (data-parallel via Rayon) ─────────────────────────────────
// Provides parallel map/filter/each for lists.

fn make_parallel_module() -> Value {
    use rayon::prelude::*;
    let mut ns: HashMap<String, Value> = HashMap::new();

    // parallel.map(list, func) → new list — evaluates func on each item concurrently
    ns.insert("map".into(), Value::NativeFunction(NativeFunction {
        name: "parallel.map".into(),
        arity: Some(2),
        func: Arc::new(|args| {
            let items = match args.first() {
                Some(Value::List(l)) => l.lock().unwrap().clone(),
                _ => return Err(CocotteError::type_err("parallel.map(list, func) requires a list")),
            };
            let func = match args.get(1) {
                Some(Value::Function(f)) => f.clone(),
                _ => return Err(CocotteError::type_err("parallel.map: second arg must be a function")),
            };
            // Each item gets its own interpreter (they are independent)
            let results: std::result::Result<Vec<Value>, String> = items
                .into_par_iter()
                .map(|item| {
                    let mut interp = crate::interpreter::Interpreter::new();
                    interp.call_function_pub(&func, vec![item], None)
                        .map_err(|e| e.message.clone())
                })
                .collect();
            results
                .map(|v| Value::List(Arc::new(Mutex::new(v))))
                .map_err(|e| CocotteError::runtime(&format!("parallel.map error: {}", e)))
        }),
    }));

    // parallel.filter(list, pred) → new list
    ns.insert("filter".into(), Value::NativeFunction(NativeFunction {
        name: "parallel.filter".into(),
        arity: Some(2),
        func: Arc::new(|args| {
            let items = match args.first() {
                Some(Value::List(l)) => l.lock().unwrap().clone(),
                _ => return Err(CocotteError::type_err("parallel.filter(list, func) requires a list")),
            };
            let func = match args.get(1) {
                Some(Value::Function(f)) => f.clone(),
                _ => return Err(CocotteError::type_err("parallel.filter: second arg must be a function")),
            };
            let results: std::result::Result<Vec<(Value, bool)>, String> = items
                .into_par_iter()
                .map(|item| {
                    let mut interp = crate::interpreter::Interpreter::new();
                    interp.call_function_pub(&func, vec![item.clone()], None)
                        .map(|r| (item, r.is_truthy()))
                        .map_err(|e| e.message.clone())
                })
                .collect();
            results
                .map(|pairs| {
                    let filtered: Vec<Value> = pairs.into_iter()
                        .filter(|(_, keep)| *keep)
                        .map(|(v, _)| v)
                        .collect();
                    Value::List(Arc::new(Mutex::new(filtered)))
                })
                .map_err(|e| CocotteError::runtime(&format!("parallel.filter error: {}", e)))
        }),
    }));

    // parallel.each(list, func) — fire-and-forget parallel iteration
    ns.insert("each".into(), Value::NativeFunction(NativeFunction {
        name: "parallel.each".into(),
        arity: Some(2),
        func: Arc::new(|args| {
            let items = match args.first() {
                Some(Value::List(l)) => l.lock().unwrap().clone(),
                _ => return Err(CocotteError::type_err("parallel.each(list, func) requires a list")),
            };
            let func = match args.get(1) {
                Some(Value::Function(f)) => f.clone(),
                _ => return Err(CocotteError::type_err("parallel.each: second arg must be a function")),
            };
            items.into_par_iter().for_each(|item| {
                let mut interp = crate::interpreter::Interpreter::new();
                let _ = interp.call_function_pub(&func, vec![item], None);
            });
            Ok(Value::Nil)
        }),
    }));

    // parallel.num_threads() → number
    ns.insert("num_threads".into(), Value::NativeFunction(NativeFunction {
        name: "parallel.num_threads".into(),
        arity: Some(0),
        func: Arc::new(|_| {
            Ok(Value::Number(rayon::current_num_threads() as f64))
        }),
    }));

    // parallel.set_threads(n) — configure rayon thread pool size
    ns.insert("set_threads".into(), Value::NativeFunction(NativeFunction {
        name: "parallel.set_threads".into(),
        arity: Some(1),
        func: Arc::new(|args| match args.first() {
            Some(Value::Number(n)) => {
                let _ = rayon::ThreadPoolBuilder::new()
                    .num_threads(*n as usize)
                    .build_global();
                Ok(Value::Nil)
            }
            _ => Err(CocotteError::type_err("parallel.set_threads(n) requires a number")),
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

// ── AI module ─────────────────────────────────────────────────────────────────
// Cocotte-native ML/AI: embedding, classification, generation via HTTP backends.
// Pure Cocotte inference helpers that call local Ollama / OpenAI-compatible APIs.

fn make_ai_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();

    // ai.generate(model, prompt [, options_map]) → string
    // Calls an Ollama-compatible /api/generate endpoint.
    ns.insert("generate".into(), Value::NativeFunction(NativeFunction {
        name: "ai.generate".into(),
        arity: None,
        func: Arc::new(|args| {
            let model = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("ai.generate(model, prompt [, opts]) — model must be a string")),
            };
            let prompt = match args.get(1) {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("ai.generate: prompt must be a string")),
            };
            let base_url = match args.get(2) {
                Some(Value::Map(m)) => {
                    m.lock().unwrap().get("base_url")
                        .and_then(|v| if let Value::Str(s) = v { Some(s.clone()) } else { None })
                        .unwrap_or_else(|| "http://localhost:11434".into())
                }
                _ => "http://localhost:11434".into(),
            };
            let temperature = match args.get(2) {
                Some(Value::Map(m)) => {
                    m.lock().unwrap().get("temperature")
                        .and_then(|v| if let Value::Number(n) = v { Some(*n) } else { None })
                        .unwrap_or(0.7)
                }
                _ => 0.7,
            };
            let payload = serde_json::json!({
                "model": model,
                "prompt": prompt,
                "stream": false,
                "options": { "temperature": temperature }
            });
            let body = serde_json::to_string(&payload)
                .map_err(|e| CocotteError::runtime(&format!("ai.generate: JSON encode error: {}", e)))?;
            let url = format!("{}/api/generate", base_url.trim_end_matches('/'));
            let resp = ureq::post(&url)
                .set("Content-Type", "application/json")
                .send_string(&body)
                .map_err(|e| CocotteError::runtime(&format!("ai.generate: HTTP error: {}", e)))?
                .into_string()
                .map_err(|e| CocotteError::runtime(&format!("ai.generate: read error: {}", e)))?;
            let json: serde_json::Value = serde_json::from_str(&resp)
                .map_err(|e| CocotteError::runtime(&format!("ai.generate: JSON parse error: {}", e)))?;
            let text = json["response"].as_str().unwrap_or("").to_string();
            Ok(Value::Str(text))
        }),
    }));

    // ai.chat(model, messages_list [, opts]) → string
    // messages_list is a list of maps: [{"role": "user", "content": "hello"}, ...]
    ns.insert("chat".into(), Value::NativeFunction(NativeFunction {
        name: "ai.chat".into(),
        arity: None,
        func: Arc::new(|args| {
            let model = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("ai.chat(model, messages [, opts])")),
            };
            let messages = match args.get(1) {
                Some(Value::List(l)) => cocotte_to_json(&Value::List(l.clone())),
                _ => return Err(CocotteError::type_err("ai.chat: messages must be a list of maps")),
            };
            let base_url = match args.get(2) {
                Some(Value::Map(m)) => m.lock().unwrap().get("base_url")
                    .and_then(|v| if let Value::Str(s) = v { Some(s.clone()) } else { None })
                    .unwrap_or_else(|| "http://localhost:11434".into()),
                _ => "http://localhost:11434".into(),
            };
            let payload = serde_json::json!({
                "model": model,
                "messages": messages,
                "stream": false
            });
            let body = serde_json::to_string(&payload)
                .map_err(|e| CocotteError::runtime(&format!("ai.chat: JSON error: {}", e)))?;
            let url = format!("{}/api/chat", base_url.trim_end_matches('/'));
            let resp = ureq::post(&url)
                .set("Content-Type", "application/json")
                .send_string(&body)
                .map_err(|e| CocotteError::runtime(&format!("ai.chat: HTTP error: {}", e)))?
                .into_string()
                .map_err(|e| CocotteError::runtime(&format!("ai.chat: read error: {}", e)))?;
            let json: serde_json::Value = serde_json::from_str(&resp)
                .map_err(|e| CocotteError::runtime(&format!("ai.chat: parse error: {}", e)))?;
            let text = json["message"]["content"].as_str()
                .or_else(|| json["response"].as_str())
                .unwrap_or("").to_string();
            Ok(Value::Str(text))
        }),
    }));

    // ai.embed(model, text [, opts]) → list of numbers (embedding vector)
    ns.insert("embed".into(), Value::NativeFunction(NativeFunction {
        name: "ai.embed".into(),
        arity: None,
        func: Arc::new(|args| {
            let model = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("ai.embed(model, text [, opts])")),
            };
            let text = match args.get(1) {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("ai.embed: text must be a string")),
            };
            let base_url = match args.get(2) {
                Some(Value::Map(m)) => m.lock().unwrap().get("base_url")
                    .and_then(|v| if let Value::Str(s) = v { Some(s.clone()) } else { None })
                    .unwrap_or_else(|| "http://localhost:11434".into()),
                _ => "http://localhost:11434".into(),
            };
            let payload = serde_json::json!({ "model": model, "prompt": text });
            let body = serde_json::to_string(&payload)
                .map_err(|e| CocotteError::runtime(&format!("ai.embed: JSON error: {}", e)))?;
            let url = format!("{}/api/embeddings", base_url.trim_end_matches('/'));
            let resp = ureq::post(&url)
                .set("Content-Type", "application/json")
                .send_string(&body)
                .map_err(|e| CocotteError::runtime(&format!("ai.embed: HTTP error: {}", e)))?
                .into_string()
                .map_err(|e| CocotteError::runtime(&format!("ai.embed: read error: {}", e)))?;
            let json: serde_json::Value = serde_json::from_str(&resp)
                .map_err(|e| CocotteError::runtime(&format!("ai.embed: parse error: {}", e)))?;
            let embedding = json["embedding"].as_array()
                .ok_or_else(|| CocotteError::runtime("ai.embed: response missing 'embedding' field"))?
                .iter()
                .map(|n| Value::Number(n.as_f64().unwrap_or(0.0)))
                .collect();
            Ok(Value::List(Arc::new(Mutex::new(embedding))))
        }),
    }));

    // ai.cosine_similarity(a, b) → number — compare two embedding vectors
    ns.insert("cosine_similarity".into(), Value::NativeFunction(NativeFunction {
        name: "ai.cosine_similarity".into(),
        arity: Some(2),
        func: Arc::new(|args| {
            fn extract_vec(v: &Value) -> Option<Vec<f64>> {
                if let Value::List(l) = v {
                    Some(l.lock().unwrap().iter().filter_map(|n| {
                        if let Value::Number(x) = n { Some(*x) } else { None }
                    }).collect())
                } else { None }
            }
            let a = extract_vec(&args[0])
                .ok_or_else(|| CocotteError::type_err("ai.cosine_similarity: both args must be lists of numbers"))?;
            let b = extract_vec(&args[1])
                .ok_or_else(|| CocotteError::type_err("ai.cosine_similarity: both args must be lists of numbers"))?;
            if a.len() != b.len() {
                return Err(CocotteError::runtime("ai.cosine_similarity: vectors must have the same length"));
            }
            let dot: f64  = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
            let mag_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
            let mag_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
            if mag_a == 0.0 || mag_b == 0.0 {
                return Ok(Value::Number(0.0));
            }
            Ok(Value::Number(dot / (mag_a * mag_b)))
        }),
    }));

    // ai.models([opts]) → list of model names from Ollama
    ns.insert("models".into(), Value::NativeFunction(NativeFunction {
        name: "ai.models".into(),
        arity: None,
        func: Arc::new(|args| {
            let base_url = match args.first() {
                Some(Value::Map(m)) => m.lock().unwrap().get("base_url")
                    .and_then(|v| if let Value::Str(s) = v { Some(s.clone()) } else { None })
                    .unwrap_or_else(|| "http://localhost:11434".into()),
                _ => "http://localhost:11434".into(),
            };
            let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
            let resp = ureq::get(&url).call()
                .map_err(|e| CocotteError::runtime(&format!("ai.models: HTTP error: {}", e)))?
                .into_string()
                .map_err(|e| CocotteError::runtime(&format!("ai.models: read error: {}", e)))?;
            let json: serde_json::Value = serde_json::from_str(&resp)
                .map_err(|e| CocotteError::runtime(&format!("ai.models: parse error: {}", e)))?;
            let models: Vec<Value> = json["models"].as_array()
                .map(|arr| arr.iter().filter_map(|m| {
                    m["name"].as_str().map(|s| Value::Str(s.to_string()))
                }).collect())
                .unwrap_or_default();
            Ok(Value::List(Arc::new(Mutex::new(models))))
        }),
    }));

    // ai.openai_chat(api_key, model, messages [, opts]) → string
    // OpenAI-compatible chat completions endpoint
    ns.insert("openai_chat".into(), Value::NativeFunction(NativeFunction {
        name: "ai.openai_chat".into(),
        arity: None,
        func: Arc::new(|args| {
            let api_key = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("ai.openai_chat(api_key, model, messages [, opts])")),
            };
            let model = match args.get(1) {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(CocotteError::type_err("ai.openai_chat: model must be a string")),
            };
            let messages = match args.get(2) {
                Some(Value::List(l)) => cocotte_to_json(&Value::List(l.clone())),
                _ => return Err(CocotteError::type_err("ai.openai_chat: messages must be a list")),
            };
            let (base_url, temperature) = match args.get(3) {
                Some(Value::Map(m)) => {
                    let lock = m.lock().unwrap();
                    let bu = lock.get("base_url")
                        .and_then(|v| if let Value::Str(s) = v { Some(s.clone()) } else { None })
                        .unwrap_or_else(|| "https://api.openai.com".into());
                    let t = lock.get("temperature")
                        .and_then(|v| if let Value::Number(n) = v { Some(*n) } else { None })
                        .unwrap_or(0.7);
                    (bu, t)
                }
                _ => ("https://api.openai.com".into(), 0.7),
            };
            let payload = serde_json::json!({
                "model": model,
                "messages": messages,
                "temperature": temperature
            });
            let body = serde_json::to_string(&payload)
                .map_err(|e| CocotteError::runtime(&format!("ai.openai_chat: JSON error: {}", e)))?;
            let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
            let resp = ureq::post(&url)
                .set("Content-Type", "application/json")
                .set("Authorization", &format!("Bearer {}", api_key))
                .send_string(&body)
                .map_err(|e| CocotteError::runtime(&format!("ai.openai_chat: HTTP error: {}", e)))?
                .into_string()
                .map_err(|e| CocotteError::runtime(&format!("ai.openai_chat: read error: {}", e)))?;
            let json: serde_json::Value = serde_json::from_str(&resp)
                .map_err(|e| CocotteError::runtime(&format!("ai.openai_chat: parse error: {}", e)))?;
            let text = json["choices"][0]["message"]["content"].as_str()
                .unwrap_or("").to_string();
            Ok(Value::Str(text))
        }),
    }));

    Value::Module(Arc::new(Mutex::new(ns)))
}

// ── Network stub (legacy) ─────────────────────────────────────────────────────

fn make_network_stub_module() -> Value {
    let mut ns: HashMap<String, Value> = HashMap::new();
    macro_rules! net_stub {
        ($name:expr) => {
            ns.insert($name.to_string(), Value::NativeFunction(NativeFunction {
                name: format!("network.{}", $name),
                arity: None,
                func: Arc::new(|args| {
                    let url = args.first().map(|v| v.to_display()).unwrap_or_default();
                    eprintln!("[network] {} {} — use module add \"http\" for real HTTP", $name, url);
                    Ok(Value::Nil)
                }),
            }));
        };
    }
    net_stub!("get");
    net_stub!("post");
    Value::Module(Arc::new(Mutex::new(ns)))
}
