// builtins.rs — Built-in standard library functions for Cocotte
// All native Rust functions available to every Cocotte program

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::value::{Value, NativeFunction};
use crate::error::{CocotteError, Result};

/// Register all built-in functions into a namespace map
pub fn register_builtins(env: &mut HashMap<String, Value>) {
    macro_rules! builtin {
        ($name:expr, $arity:expr, $func:expr) => {
            env.insert(
                $name.to_string(),
                Value::NativeFunction(NativeFunction {
                    name: $name.to_string(),
                    arity: $arity,
                    func: Arc::new($func),
                }),
            );
        };
    }

    // ── Output ─────────────────────────────────────────────────────────────

    builtin!("print", None, |args| {
        let out: Vec<String> = args.iter().map(|v| v.to_display()).collect();
        println!("{}", out.join(" "));
        Ok(Value::Nil)
    });

    builtin!("input", Some(1), |args| {
        let prompt = args.get(0).map(|v| v.to_display()).unwrap_or_default();
        use std::io::{self, Write};
        print!("{}", prompt);
        io::stdout().flush().ok();
        let mut line = String::new();
        io::stdin().read_line(&mut line).ok();
        Ok(Value::Str(line.trim_end_matches('\n').to_string()))
    });

    // ── Type conversion ──────────────────────────────────────────────────

    builtin!("to_number", Some(1), |args| {
        match &args[0] {
            Value::Number(n) => Ok(Value::Number(*n)),
            Value::Str(s) => s.parse::<f64>()
                .map(Value::Number)
                .map_err(|_| CocotteError::runtime(&format!("Cannot convert '{}' to number", s))),
            Value::Bool(b) => Ok(Value::Number(if *b { 1.0 } else { 0.0 })),
            other => Err(CocotteError::type_err(&format!(
                "Cannot convert {} to number", other.type_name()
            ))),
        }
    });

    builtin!("to_string", Some(1), |args| {
        Ok(Value::Str(args[0].to_display()))
    });

    builtin!("to_bool", Some(1), |args| {
        Ok(Value::Bool(args[0].is_truthy()))
    });

    builtin!("type_of", Some(1), |args| {
        Ok(Value::Str(args[0].type_name().to_string()))
    });

    // ── Math ─────────────────────────────────────────────────────────────

    builtin!("abs", Some(1), |args| {
        match &args[0] {
            Value::Number(n) => Ok(Value::Number(n.abs())),
            _ => Err(CocotteError::type_err("abs() requires a number")),
        }
    });

    builtin!("floor", Some(1), |args| {
        match &args[0] {
            Value::Number(n) => Ok(Value::Number(n.floor())),
            _ => Err(CocotteError::type_err("floor() requires a number")),
        }
    });

    builtin!("ceil", Some(1), |args| {
        match &args[0] {
            Value::Number(n) => Ok(Value::Number(n.ceil())),
            _ => Err(CocotteError::type_err("ceil() requires a number")),
        }
    });

    builtin!("round", Some(1), |args| {
        match &args[0] {
            Value::Number(n) => Ok(Value::Number(n.round())),
            _ => Err(CocotteError::type_err("round() requires a number")),
        }
    });

    builtin!("sqrt", Some(1), |args| {
        match &args[0] {
            Value::Number(n) => Ok(Value::Number(n.sqrt())),
            _ => Err(CocotteError::type_err("sqrt() requires a number")),
        }
    });

    builtin!("pow", Some(2), |args| {
        match (&args[0], &args[1]) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a.powf(*b))),
            _ => Err(CocotteError::type_err("pow() requires two numbers")),
        }
    });

    builtin!("max", None, |args| {
        let nums: Result<Vec<f64>> = args.iter().map(|v| match v {
            Value::Number(n) => Ok(*n),
            _ => Err(CocotteError::type_err("max() requires numbers")),
        }).collect();
        let nums = nums?;
        nums.into_iter().reduce(f64::max)
            .map(Value::Number)
            .ok_or_else(|| CocotteError::runtime("max() called with no arguments"))
    });

    builtin!("min", None, |args| {
        let nums: Result<Vec<f64>> = args.iter().map(|v| match v {
            Value::Number(n) => Ok(*n),
            _ => Err(CocotteError::type_err("min() requires numbers")),
        }).collect();
        let nums = nums?;
        nums.into_iter().reduce(f64::min)
            .map(Value::Number)
            .ok_or_else(|| CocotteError::runtime("min() called with no arguments"))
    });

    // ── String ───────────────────────────────────────────────────────────

    builtin!("len", Some(1), |args| {
        match &args[0] {
            Value::Str(s) => Ok(Value::Number(s.len() as f64)),
            Value::List(l) => Ok(Value::Number(l.lock().unwrap().len() as f64)),
            Value::Map(m) => Ok(Value::Number(m.lock().unwrap().len() as f64)),
            _ => Err(CocotteError::type_err("len() requires a string, list, or map")),
        }
    });

    builtin!("upper", Some(1), |args| {
        match &args[0] {
            Value::Str(s) => Ok(Value::Str(s.to_uppercase())),
            _ => Err(CocotteError::type_err("upper() requires a string")),
        }
    });

    builtin!("lower", Some(1), |args| {
        match &args[0] {
            Value::Str(s) => Ok(Value::Str(s.to_lowercase())),
            _ => Err(CocotteError::type_err("lower() requires a string")),
        }
    });

    builtin!("trim", Some(1), |args| {
        match &args[0] {
            Value::Str(s) => Ok(Value::Str(s.trim().to_string())),
            _ => Err(CocotteError::type_err("trim() requires a string")),
        }
    });

    builtin!("split", Some(2), |args| {
        match (&args[0], &args[1]) {
            (Value::Str(s), Value::Str(sep)) => {
                let parts: Vec<Value> = s.split(sep.as_str())
                    .map(|p| Value::Str(p.to_string()))
                    .collect();
                Ok(Value::List(Arc::new(Mutex::new(parts))))
            }
            _ => Err(CocotteError::type_err("split() requires two strings")),
        }
    });

    builtin!("contains", Some(2), |args| {
        match (&args[0], &args[1]) {
            (Value::Str(s), Value::Str(sub)) => Ok(Value::Bool(s.contains(sub.as_str()))),
            (Value::List(l), val) => {
                let l = l.lock().unwrap();
                Ok(Value::Bool(l.iter().any(|v| v == val)))
            }
            _ => Err(CocotteError::type_err("contains() requires a string and a substring, or a list and a value")),
        }
    });

    builtin!("starts_with", Some(2), |args| {
        match (&args[0], &args[1]) {
            (Value::Str(s), Value::Str(pre)) => Ok(Value::Bool(s.starts_with(pre.as_str()))),
            _ => Err(CocotteError::type_err("starts_with() requires two strings")),
        }
    });

    builtin!("ends_with", Some(2), |args| {
        match (&args[0], &args[1]) {
            (Value::Str(s), Value::Str(suf)) => Ok(Value::Bool(s.ends_with(suf.as_str()))),
            _ => Err(CocotteError::type_err("ends_with() requires two strings")),
        }
    });

    builtin!("replace", Some(3), |args| {
        match (&args[0], &args[1], &args[2]) {
            (Value::Str(s), Value::Str(from), Value::Str(to)) => {
                Ok(Value::Str(s.replace(from.as_str(), to.as_str())))
            }
            _ => Err(CocotteError::type_err("replace() requires three strings")),
        }
    });

    // ── List ─────────────────────────────────────────────────────────────

    builtin!("push", Some(2), |args| {
        match &args[0] {
            Value::List(l) => {
                l.lock().unwrap().push(args[1].clone());
                Ok(Value::Nil)
            }
            _ => Err(CocotteError::type_err("push() requires a list")),
        }
    });

    builtin!("pop", Some(1), |args| {
        match &args[0] {
            Value::List(l) => {
                l.lock().unwrap().pop().ok_or_else(|| CocotteError::runtime("pop() on empty list"))
            }
            _ => Err(CocotteError::type_err("pop() requires a list")),
        }
    });

    builtin!("reverse", Some(1), |args| {
        match &args[0] {
            Value::List(l) => {
                let mut l = l.lock().unwrap();
                l.reverse();
                Ok(Value::Nil)
            }
            _ => Err(CocotteError::type_err("reverse() requires a list")),
        }
    });

    builtin!("sort", Some(1), |args| {
        match &args[0] {
            Value::List(l) => {
                let mut l = l.lock().unwrap();
                l.sort_by(|a, b| {
                    match (a, b) {
                        (Value::Number(x), Value::Number(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                        (Value::Str(x), Value::Str(y)) => x.cmp(y),
                        _ => std::cmp::Ordering::Equal,
                    }
                });
                Ok(Value::Nil)
            }
            _ => Err(CocotteError::type_err("sort() requires a list")),
        }
    });

    builtin!("range", None, |args| {
        let (start, end, step) = match args.len() {
            1 => match &args[0] {
                Value::Number(n) => (0.0, *n, 1.0),
                _ => return Err(CocotteError::type_err("range() requires numbers")),
            },
            2 => match (&args[0], &args[1]) {
                (Value::Number(s), Value::Number(e)) => (*s, *e, 1.0),
                _ => return Err(CocotteError::type_err("range() requires numbers")),
            },
            3 => match (&args[0], &args[1], &args[2]) {
                (Value::Number(s), Value::Number(e), Value::Number(st)) => (*s, *e, *st),
                _ => return Err(CocotteError::type_err("range() requires numbers")),
            },
            _ => return Err(CocotteError::runtime("range() takes 1-3 arguments")),
        };
        let mut vals = Vec::new();
        let mut i = start;
        while if step > 0.0 { i < end } else { i > end } {
            vals.push(Value::Number(i));
            i += step;
        }
        Ok(Value::List(Arc::new(Mutex::new(vals))))
    });

    // ── IO ───────────────────────────────────────────────────────────────

    builtin!("read_file", Some(1), |args| {
        match &args[0] {
            Value::Str(path) => {
                std::fs::read_to_string(path)
                    .map(Value::Str)
                    .map_err(|e| CocotteError::io_err(&format!("Cannot read '{}': {}", path, e)))
            }
            _ => Err(CocotteError::type_err("read_file() requires a string path")),
        }
    });

    builtin!("write_file", Some(2), |args| {
        match (&args[0], &args[1]) {
            (Value::Str(path), Value::Str(content)) => {
                std::fs::write(path, content)
                    .map(|_| Value::Nil)
                    .map_err(|e| CocotteError::io_err(&format!("Cannot write '{}': {}", path, e)))
            }
            _ => Err(CocotteError::type_err("write_file() requires two strings")),
        }
    });

    builtin!("file_exists", Some(1), |args| {
        match &args[0] {
            Value::Str(path) => Ok(Value::Bool(std::path::Path::new(path).exists())),
            _ => Err(CocotteError::type_err("file_exists() requires a string path")),
        }
    });

    builtin!("append_file", Some(2), |args| {
        match (&args[0], &args[1]) {
            (Value::Str(path), Value::Str(content)) => {
                use std::io::Write;
                let mut f = std::fs::OpenOptions::new().append(true).create(true).open(path)
                    .map_err(|e| CocotteError::io_err(&format!("Cannot open '{}': {}", path, e)))?;
                f.write_all(content.as_bytes())
                    .map(|_| Value::Nil)
                    .map_err(|e| CocotteError::io_err(&format!("Cannot write '{}': {}", path, e)))
            }
            _ => Err(CocotteError::type_err("append_file() requires two strings")),
        }
    });

    builtin!("delete_file", Some(1), |args| {
        match &args[0] {
            Value::Str(path) => {
                let p = std::path::Path::new(path);
                if p.is_dir() {
                    std::fs::remove_dir_all(p)
                } else {
                    std::fs::remove_file(p)
                }
                .map(|_| Value::Nil)
                .map_err(|e| CocotteError::io_err(&format!("Cannot delete '{}': {}", path, e)))
            }
            _ => Err(CocotteError::type_err("delete_file() requires a string path")),
        }
    });

    builtin!("make_dir", Some(1), |args| {
        match &args[0] {
            Value::Str(path) => {
                std::fs::create_dir_all(path)
                    .map(|_| Value::Nil)
                    .map_err(|e| CocotteError::io_err(&format!("Cannot create dir '{}': {}", path, e)))
            }
            _ => Err(CocotteError::type_err("make_dir() requires a string path")),
        }
    });

    builtin!("list_dir", Some(1), |args| {
        match &args[0] {
            Value::Str(path) => {
                let entries = std::fs::read_dir(path)
                    .map_err(|e| CocotteError::io_err(&format!("Cannot read dir '{}': {}", path, e)))?;
                let mut items = Vec::new();
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        items.push(Value::Str(name.to_string()));
                    }
                }
                items.sort_by(|a, b| a.to_display().cmp(&b.to_display()));
                Ok(Value::List(Arc::new(Mutex::new(items))))
            }
            _ => Err(CocotteError::type_err("list_dir() requires a string path")),
        }
    });

    builtin!("rename_file", Some(2), |args| {
        match (&args[0], &args[1]) {
            (Value::Str(from), Value::Str(to)) => {
                std::fs::rename(from, to)
                    .map(|_| Value::Nil)
                    .map_err(|e| CocotteError::io_err(&format!("Cannot rename '{}' to '{}': {}", from, to, e)))
            }
            _ => Err(CocotteError::type_err("rename_file() requires two strings")),
        }
    });

    builtin!("copy_file", Some(2), |args| {
        match (&args[0], &args[1]) {
            (Value::Str(from), Value::Str(to)) => {
                std::fs::copy(from, to)
                    .map(|_| Value::Nil)
                    .map_err(|e| CocotteError::io_err(&format!("Cannot copy '{}' to '{}': {}", from, to, e)))
            }
            _ => Err(CocotteError::type_err("copy_file() requires two strings")),
        }
    });

    builtin!("is_dir", Some(1), |args| {
        match &args[0] {
            Value::Str(path) => Ok(Value::Bool(std::path::Path::new(path).is_dir())),
            _ => Err(CocotteError::type_err("is_dir() requires a string path")),
        }
    });

    builtin!("is_file", Some(1), |args| {
        match &args[0] {
            Value::Str(path) => Ok(Value::Bool(std::path::Path::new(path).is_file())),
            _ => Err(CocotteError::type_err("is_file() requires a string path")),
        }
    });

    builtin!("file_size", Some(1), |args| {
        match &args[0] {
            Value::Str(path) => {
                std::fs::metadata(path)
                    .map(|m| Value::Number(m.len() as f64))
                    .map_err(|e| CocotteError::io_err(&format!("Cannot stat '{}': {}", path, e)))
            }
            _ => Err(CocotteError::type_err("file_size() requires a string path")),
        }
    });

    // ── System ───────────────────────────────────────────────────────────

    builtin!("exit", Some(1), |args| {
        let code = match &args[0] {
            Value::Number(n) => *n as i32,
            _ => 0,
        };
        std::process::exit(code);
    });

    builtin!("env_get", Some(1), |args| {
        match &args[0] {
            Value::Str(key) => Ok(std::env::var(key).map(Value::Str).unwrap_or(Value::Nil)),
            _ => Err(CocotteError::type_err("env_get() requires a string key")),
        }
    });

    builtin!("sleep", Some(1), |args| {
        match &args[0] {
            Value::Number(secs) => {
                std::thread::sleep(std::time::Duration::from_secs_f64(*secs));
                Ok(Value::Nil)
            }
            _ => Err(CocotteError::type_err("sleep() requires a number")),
        }
    });

    builtin!("random", Some(0), |_args| {
        // Simple LCG random since we don't depend on rand crate
        use std::time::SystemTime;
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(12345) as f64;
        let r = (seed * 1664525.0 + 1013904223.0) % 4294967296.0 / 4294967296.0;
        Ok(Value::Number(r))
    });

    // ── Map helpers ──────────────────────────────────────────────────────

    builtin!("keys", Some(1), |args| {
        match &args[0] {
            Value::Map(m) => {
                let m = m.lock().unwrap();
                let keys: Vec<Value> = m.keys().map(|k| Value::Str(k.clone())).collect();
                Ok(Value::List(Arc::new(Mutex::new(keys))))
            }
            _ => Err(CocotteError::type_err("keys() requires a map")),
        }
    });

    builtin!("values", Some(1), |args| {
        match &args[0] {
            Value::Map(m) => {
                let m = m.lock().unwrap();
                let vals: Vec<Value> = m.values().cloned().collect();
                Ok(Value::List(Arc::new(Mutex::new(vals))))
            }
            _ => Err(CocotteError::type_err("values() requires a map")),
        }
    });

    builtin!("has_key", Some(2), |args| {
        match &args[0] {
            Value::Map(m) => {
                let key = args[1].to_display();
                Ok(Value::Bool(m.lock().unwrap().contains_key(&key)))
            }
            _ => Err(CocotteError::type_err("has_key() requires a map")),
        }
    });
}
