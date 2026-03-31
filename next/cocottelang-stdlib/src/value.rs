// value.rs — Runtime value representation for the Cocotte interpreter
// All Cocotte values live as this enum during execution

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use crate::ast::Stmt;

/// A runtime value in Cocotte
#[derive(Clone, Debug)]
pub enum Value {
    Number(f64),
    Str(String),
    Bool(bool),
    Nil,
    List(Arc<Mutex<Vec<Value>>>),
    Map(Arc<Mutex<HashMap<String, Value>>>),
    Function(CocotteFunction),
    NativeFunction(NativeFunction),
    Instance(Arc<Mutex<ClassInstance>>),
    Class(CocotteClass),
    Module(Arc<Mutex<HashMap<String, Value>>>),
}

/// A user-defined Cocotte function
#[derive(Clone, Debug)]
pub struct CocotteFunction {
    pub name: Option<String>,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    /// Closure environment snapshot (names → values)
    pub closure: HashMap<String, Value>,
    /// Optional bytecode for VM execution (None = use tree-walk body)
    pub bytecode: Option<Vec<crate::bytecode::Instruction>>,
}

/// A built-in native function implemented in Rust
#[derive(Clone)]
pub struct NativeFunction {
    pub name: String,
    pub arity: Option<usize>, // None = variadic
    pub func: Arc<dyn Fn(Vec<Value>) -> crate::error::Result<Value> + Send + Sync>,
}

impl fmt::Debug for NativeFunction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<native fn {}>", self.name)
    }
}

/// An instance of a user-defined class
#[derive(Debug, Clone)]
pub struct ClassInstance {
    pub class_name: String,
    pub fields: HashMap<String, Value>,
    pub methods: HashMap<String, CocotteFunction>,
}

/// A class definition (callable to create instances)
#[derive(Clone, Debug)]
pub struct CocotteClass {
    pub name: String,
    pub methods: HashMap<String, CocotteFunction>,
}

impl Value {
    /// Return true if the value is truthy
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Nil => false,
            Value::Number(n) => *n != 0.0,
            Value::Str(s) => !s.is_empty(),
            _ => true,
        }
    }

    /// Human-readable type name
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_) => "number",
            Value::Str(_) => "string",
            Value::Bool(_) => "bool",
            Value::Nil => "nil",
            Value::List(_) => "list",
            Value::Map(_) => "map",
            Value::Function(_) => "function",
            Value::NativeFunction(_) => "function",
            Value::Instance(_) => "instance",
            Value::Class(_) => "class",
            Value::Module(_) => "module",
        }
    }

    /// Convert value to a display string
    pub fn to_display(&self) -> String {
        match self {
            Value::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Value::Str(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Nil => "nil".to_string(),
            Value::List(l) => {
                let l = l.lock().unwrap();
                let items: Vec<String> = l.iter().map(|v| v.to_repr()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Map(m) => {
                let m = m.lock().unwrap();
                let items: Vec<String> = m.iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, v.to_repr()))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
            Value::Function(f) => {
                format!("<func {}>", f.name.as_deref().unwrap_or("anonymous"))
            }
            Value::NativeFunction(f) => format!("<native func {}>", f.name),
            Value::Instance(inst) => {
                let inst = inst.lock().unwrap();
                format!("<{} instance>", inst.class_name)
            }
            Value::Class(c) => format!("<class {}>", c.name),
            Value::Module(m) => {
                let m = m.lock().unwrap();
                format!("<module with {} items>", m.len())
            }
        }
    }

    /// Like to_display but wraps strings in quotes for repr
    pub fn to_repr(&self) -> String {
        match self {
            Value::Str(s) => format!("\"{}\"", s),
            other => other.to_display(),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_display())
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            _ => false,
        }
    }
}

// ── Thread safety for parallel module ────────────────────────────────────────
// Value contains Arc<Mutex<...>> for List/Map/Module (safe to share across
// threads) and CocotteFunction (body is Vec<Stmt> which is Send + Sync).
// NativeFunction wraps Arc<dyn Fn(...) + Send + Sync> so it is already Send.
// ClassInstance fields are HashMap<String,Value> cloned before crossing thread
// boundaries — each thread gets its own copy. These impls are sound because:
//   - All interior mutability uses Mutex (not RefCell)
//   - CocotteFunction.closure is cloned (not shared) into each thread
unsafe impl Send for Value {}
unsafe impl Sync for Value {}
