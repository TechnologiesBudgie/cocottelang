// environment.rs — Lexical scoping environment for Cocotte
// Environments are chained: inner scopes can see outer scopes

use std::collections::HashMap;
use crate::value::Value;
use crate::error::{CocotteError, Result};

/// A single scope frame in the environment chain
#[derive(Debug, Clone)]
pub struct Environment {
    vars: HashMap<String, Value>,
    parent: Option<Box<Environment>>,
}

impl Environment {
    /// Create a fresh global environment
    pub fn new() -> Self {
        Environment {
            vars: HashMap::new(),
            parent: None,
        }
    }

    /// Create a child scope inheriting from parent
    pub fn with_parent(parent: Environment) -> Self {
        Environment {
            vars: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    /// Define a new variable in the current scope
    pub fn define(&mut self, name: &str, value: Value) {
        self.vars.insert(name.to_string(), value);
    }

    /// Look up a variable, searching parent scopes
    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(val) = self.vars.get(name) {
            Some(val.clone())
        } else if let Some(ref parent) = self.parent {
            parent.get(name)
        } else {
            None
        }
    }

    /// Assign an existing variable (search parent scopes)
    pub fn assign(&mut self, name: &str, value: Value) -> Result<()> {
        if self.vars.contains_key(name) {
            self.vars.insert(name.to_string(), value);
            Ok(())
        } else if let Some(ref mut parent) = self.parent {
            parent.assign(name, value)
        } else {
            Err(CocotteError::runtime(&format!(
                "Undefined variable '{}'. Did you mean to use 'var {} = ...' to declare it?",
                name, name
            )))
        }
    }

    /// Force-define in this exact scope (for function params etc.)
    pub fn define_local(&mut self, name: &str, value: Value) {
        self.vars.insert(name.to_string(), value);
    }

    /// Snapshot all bindings visible from this scope (for closures)
    pub fn snapshot(&self) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        if let Some(ref parent) = self.parent {
            map.extend(parent.snapshot());
        }
        map.extend(self.vars.clone());
        map
    }

    /// Restore from a closure snapshot into the current scope
    pub fn restore_from_snapshot(&mut self, snapshot: &HashMap<String, Value>) {
        for (k, v) in snapshot {
            if !self.vars.contains_key(k) {
                self.vars.insert(k.clone(), v.clone());
            }
        }
    }

    /// Consume this environment and return the parent, if any
    pub fn into_parent(self) -> Option<Environment> {
        self.parent.map(|b| *b)
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}
