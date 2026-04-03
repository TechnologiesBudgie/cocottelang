// interpreter.rs — Tree-walk interpreter for Cocotte
// Walks the AST and executes each node directly.
// This is the `cocotte run` mode — instant execution, no compilation.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::ast::*;
use crate::environment::Environment;
use crate::error::{CocotteError, Result, Signal};
use crate::value::{Value, CocotteFunction, CocotteClass, ClassInstance};
use crate::builtins::register_builtins;
use crate::modules::{load_module, load_library};

pub struct Interpreter {
    env: Environment,
    /// Project root dir for module/library resolution
    pub project_root: std::path::PathBuf,
    /// Whether to print verbose debug info
    pub debug: bool,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut env = Environment::new();
        // Inject all built-in functions
        let mut builtins: HashMap<String, Value> = HashMap::new();
        register_builtins(&mut builtins);
        for (k, v) in builtins {
            env.define(&k, v);
        }
        Interpreter {
            env,
            project_root: std::env::current_dir().unwrap_or_default(),
            debug: false,
        }
    }

    /// Export all top-level names (for use as a library) — includes functions
    pub fn export_namespace(&self) -> HashMap<String, Value> {
        self.env.full_snapshot()
    }

/// Public wrapper for call_function (used by charlotte and http server modules)
    pub fn call_function_pub(
        &mut self,
        func: &CocotteFunction,
        args: Vec<Value>,
        self_val: Option<Value>,
    ) -> Result<Value> {
        self.call_function(func, args, self_val)
    }

    /// Copy global variable bindings from another interpreter (for charlotte and http server)
    pub fn copy_globals_from(&mut self, other: &Interpreter) {
        let snapshot = other.env.snapshot();
        for (k, v) in snapshot {
            self.env.define(&k, v);
        }
        self.project_root = other.project_root.clone();
    }

    /// Run an entire program (list of statements)
    pub fn run(&mut self, program: &Program) -> Result<Value> {
        // Register this interpreter so native modules (charlotte, http server) can call back into it
        crate::runtime_ctx::set_active_interpreter(self as *mut Interpreter as usize);
        let mut last = Value::Nil;
        for stmt in &program.statements {
            last = self.exec_stmt(stmt)?;
        }
        Ok(last)
    }

    // ── Statements ────────────────────────────────────────────────────────────

    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Value> {
        match stmt {
            Stmt::VarDecl { name, value, .. } => {
                let val = self.eval_expr(value)?;
                self.env.define(name, val);
                Ok(Value::Nil)
            }

            Stmt::Assign { target, value, span } => {
                let val = self.eval_expr(value)?;
                self.assign_target(target, val, span)?;
                Ok(Value::Nil)
            }

            Stmt::FuncDecl { name, params, body, .. } => {
                // Top-level functions get an empty closure — they're global and
                // always found via the env chain. This prevents O(n²) snapshot
                // blowup when many functions are defined (e.g. in a library).
                // Nested functions (inside other functions) get a real snapshot
                // for proper lexical scoping.
                let closure = if self.env.has_parent() {
                    self.env.snapshot()
                } else {
                    std::collections::HashMap::new()
                };
                let func = Value::Function(CocotteFunction {
                    name: Some(name.clone()),
                    params: params.clone(),
                    body: body.clone(),
                    closure,
                    bytecode: None,
                });
                self.env.define(name, func);
                Ok(Value::Nil)
            }

            Stmt::ClassDecl { name, methods, .. } => {
                let mut method_map: HashMap<String, CocotteFunction> = HashMap::new();
                for method_stmt in methods {
                    if let Stmt::FuncDecl { name: mname, params, body, .. } = method_stmt {
                        method_map.insert(mname.clone(), CocotteFunction {
                            name: Some(mname.clone()),
                            params: params.clone(),
                            body: body.clone(),
                            closure: HashMap::new(),
                        
                    bytecode: None,
                });
                    }
                }
                let class = Value::Class(CocotteClass {
                    name: name.clone(),
                    methods: method_map,
                });
                self.env.define(name, class);
                Ok(Value::Nil)
            }

            Stmt::Return { value, .. } => {
                let val = match value {
                    Some(expr) => self.eval_expr(expr)?,
                    None => Value::Nil,
                };
                Err(CocotteError::return_signal(val))
            }

            Stmt::If { condition, then_branch, elif_branches, else_branch, .. } => {
                let cond = self.eval_expr(condition)?;
                if cond.is_truthy() {
                    self.exec_block(then_branch)?;
                } else {
                    let mut ran = false;
                    for (elif_cond, elif_body) in elif_branches {
                        let c = self.eval_expr(elif_cond)?;
                        if c.is_truthy() {
                            self.exec_block(elif_body)?;
                            ran = true;
                            break;
                        }
                    }
                    if !ran {
                        if let Some(else_stmts) = else_branch {
                            self.exec_block(else_stmts)?;
                        }
                    }
                }
                Ok(Value::Nil)
            }

            Stmt::While { condition, body, .. } => {
                loop {
                    let cond = self.eval_expr(condition)?;
                    if !cond.is_truthy() {
                        break;
                    }
                    match self.exec_block(body) {
                        Ok(_) => {}
                        Err(e) if matches!(e.signal, Some(Signal::Break)) => break,
                        Err(e) if matches!(e.signal, Some(Signal::Continue)) => continue,
                        Err(e) => return Err(e),
                    }
                }
                Ok(Value::Nil)
            }

            Stmt::For { var, iterable, body, .. } => {
                let iter_val = self.eval_expr(iterable)?;
                let items = self.value_to_iter(iter_val)?;
                for item in items {
                    // Define loop variable in the current scope (no new scope needed)
                    self.env.define(var, item);
                    match self.exec_block(body) {
                        Ok(_) => {}
                        Err(e) if matches!(e.signal, Some(Signal::Break)) => break,
                        Err(e) if matches!(e.signal, Some(Signal::Continue)) => continue,
                        Err(e) => return Err(e),
                    }
                }
                Ok(Value::Nil)
            }

            Stmt::Try { body, catch_var, catch_body, .. } => {
                match self.exec_block(body) {
                    Ok(_) => {}
                    Err(e) if !e.is_signal() => {
                        // Catch the error — create a child environment
                        let parent_env = std::mem::replace(&mut self.env, Environment::new());
                        self.env = Environment::with_parent(parent_env);
                        if let Some(var) = catch_var {
                            self.env.define(var, Value::Str(e.message.clone()));
                        }
                        let _ = self.exec_block(catch_body);
                        // Pop back to parent
                        let child = std::mem::replace(&mut self.env, Environment::new());
                        self.env = child.into_parent().unwrap_or_else(Environment::new);
                    }
                    Err(e) => return Err(e),
                }
                Ok(Value::Nil)
            }

            Stmt::Print { value, .. } => {
                let val = self.eval_expr(value)?;
                println!("{}", val.to_display());
                Ok(Value::Nil)
            }

            Stmt::ModuleAdd { name, span: _ } => {
                let module = load_module(name, &self.project_root)
                    .map_err(|e| CocotteError::module_err(&e.message)
                        .with_hint(&format!(
                            "Run `cocotte add {}` to install this module", name
                        )))?;
                self.env.define(name, module);
                if self.debug {
                    println!("[debug] Module '{}' loaded", name);
                }
                Ok(Value::Nil)
            }

            Stmt::LibraryAdd { path, .. } => {
                let lib = load_library(path, &self.project_root)?;
                // Register the library under a sanitized name
                let lib_name = std::path::Path::new(path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(path);
                self.env.define(lib_name, lib);
                if self.debug {
                    println!("[debug] Library '{}' loaded as '{}'", path, lib_name);
                }
                Ok(Value::Nil)
            }

            Stmt::Break { .. } => Err(CocotteError::break_signal()),
            Stmt::Continue { .. } => Err(CocotteError::continue_signal()),

            Stmt::ExprStmt { expr, .. } => {
                self.eval_expr(expr)?;
                Ok(Value::Nil)
            }
        }
    }

    /// Execute a block of statements in the current scope
    fn exec_block(&mut self, stmts: &[Stmt]) -> Result<Value> {
        let mut last = Value::Nil;
        for stmt in stmts {
            last = self.exec_stmt(stmt)?;
        }
        Ok(last)
    }

    // ── Expressions ───────────────────────────────────────────────────────────

    pub fn eval_expr(&mut self, expr: &Expr) -> Result<Value> {
        match expr {
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::StringLit(s) => Ok(Value::Str(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Nil => Ok(Value::Nil),
            Expr::SelfRef(_) => {
                self.env.get("self")
                    .ok_or_else(|| CocotteError::runtime(
                        "'self' used outside of a class method"
                    ))
            }

            Expr::Ident(name) => {
                self.env.get(name)
                    .ok_or_else(|| CocotteError::runtime(&format!(
                        "Undefined variable '{}'. Did you declare it with 'var'?", name
                    )))
            }

            Expr::List(elems) => {
                let mut items = Vec::new();
                for e in elems {
                    items.push(self.eval_expr(e)?);
                }
                Ok(Value::List(Arc::new(Mutex::new(items))))
            }

            Expr::Map(pairs) => {
                let mut map: HashMap<String, Value> = HashMap::new();
                for (k, v) in pairs {
                    let key = self.eval_expr(k)?.to_display();
                    let val = self.eval_expr(v)?;
                    map.insert(key, val);
                }
                Ok(Value::Map(Arc::new(Mutex::new(map))))
            }

            Expr::BinOp { op, left, right, span } => {
                self.eval_binop(op, left, right, span)
            }

            Expr::UnaryOp { op, operand, .. } => {
                let val = self.eval_expr(operand)?;
                match op {
                    UnaryOp::Not => Ok(Value::Bool(!val.is_truthy())),
                    UnaryOp::Neg => match val {
                        Value::Number(n) => Ok(Value::Number(-n)),
                        _ => Err(CocotteError::type_err("Unary '-' requires a number")),
                    },
                }
            }

            Expr::Lambda { params, body, .. } => {
                let closure = self.env.snapshot();
                Ok(Value::Function(CocotteFunction {
                    name: None,
                    params: params.clone(),
                    body: body.clone(),
                    closure,
                    bytecode: None,
                }))
            }

            Expr::Call { callee, args, span } => {
                let func = self.eval_expr(callee)?;
                let arg_vals: Result<Vec<Value>> = args.iter()
                    .map(|a| self.eval_expr(a)).collect();
                self.call_value(func, arg_vals?, span)
            }

            Expr::MethodCall { object, method, args, span } => {
                let obj = self.eval_expr(object)?;
                let arg_vals: Result<Vec<Value>> = args.iter()
                    .map(|a| self.eval_expr(a)).collect();
                self.call_method(obj, method, arg_vals?, span)
            }

            Expr::FieldAccess { object, field, span } => {
                let obj = self.eval_expr(object)?;
                self.get_field(obj, field, span)
            }

            Expr::Index { object, index, span } => {
                let obj = self.eval_expr(object)?;
                let idx = self.eval_expr(index)?;
                self.get_index(obj, idx, span)
            }

            Expr::FString { segments, .. } => {
                // segments: [lit0, expr_src1, lit2, expr_src3, ..., litN]
                // Even indices = literal text, odd indices = expression source to eval
                let mut result = String::new();
                for (i, seg) in segments.iter().enumerate() {
                    if i % 2 == 0 {
                        result.push_str(seg);
                    } else {
                        // Parse and evaluate the expression source
                        let tokens = crate::lexer::Lexer::new(seg).tokenize()
                            .map_err(|e| crate::error::CocotteError::runtime(
                                &format!("f-string expression error: {}", e)
                            ))?;
                        let mut parser = crate::parser::Parser::new(tokens);
                        let expr = parser.parse_fstring_expr()
                            .map_err(|e| crate::error::CocotteError::runtime(
                                &format!("f-string expression error: {}", e)
                            ))?;
                        let val = self.eval_expr(&expr)?;
                        result.push_str(&val.to_display());
                    }
                }
                Ok(Value::Str(result))
            }
        }
    }

    fn eval_binop(&mut self, op: &BinOp, left: &Expr, right: &Expr, span: &Span) -> Result<Value> {
        // Short-circuit for boolean operators
        match op {
            BinOp::And => {
                let l = self.eval_expr(left)?;
                if !l.is_truthy() { return Ok(Value::Bool(false)); }
                let r = self.eval_expr(right)?;
                return Ok(Value::Bool(r.is_truthy()));
            }
            BinOp::Or => {
                let l = self.eval_expr(left)?;
                if l.is_truthy() { return Ok(Value::Bool(true)); }
                let r = self.eval_expr(right)?;
                return Ok(Value::Bool(r.is_truthy()));
            }
            _ => {}
        }

        let lv = self.eval_expr(left)?;
        let rv = self.eval_expr(right)?;

        match op {
            BinOp::Add => match (&lv, &rv) {
                (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
                (Value::Str(a), Value::Str(b)) => Ok(Value::Str(format!("{}{}", a, b))),
                (Value::Str(a), _) => Ok(Value::Str(format!("{}{}", a, rv.to_display()))),
                (_, Value::Str(b)) => Ok(Value::Str(format!("{}{}", lv.to_display(), b))),
                _ => Err(CocotteError::type_err(&format!(
                    "Cannot add {} and {}", lv.type_name(), rv.type_name()
                ))),
            },
            BinOp::Sub => num_op!(lv, rv, -, "subtract"),
            BinOp::Mul => num_op!(lv, rv, *, "multiply"),
            BinOp::Div => match (&lv, &rv) {
                (Value::Number(a), Value::Number(b)) => {
                    if *b == 0.0 {
                        return Err(CocotteError::runtime_at(
                            span.line, span.col, "Division by zero"
                        ).with_hint("Check that your divisor is not zero before dividing"));
                    }
                    Ok(Value::Number(a / b))
                }
                _ => Err(CocotteError::type_err("Cannot divide non-numbers")),
            },
            BinOp::Mod => num_op!(lv, rv, %, "modulo"),
            BinOp::Eq  => Ok(Value::Bool(lv == rv)),
            BinOp::NotEq => Ok(Value::Bool(lv != rv)),
            BinOp::Lt  => cmp_op!(lv, rv, <),
            BinOp::LtEq => cmp_op!(lv, rv, <=),
            BinOp::Gt  => cmp_op!(lv, rv, >),
            BinOp::GtEq => cmp_op!(lv, rv, >=),
            BinOp::And | BinOp::Or => unreachable!(),
        }
    }

    // ── Calls ─────────────────────────────────────────────────────────────────

    fn call_value(&mut self, func: Value, args: Vec<Value>, span: &Span) -> Result<Value> {
        match func {
            Value::Function(f) => self.call_function(&f, args, None),
            Value::NativeFunction(nf) => {
                if let Some(arity) = nf.arity {
                    if args.len() != arity {
                        return Err(CocotteError::runtime_at(
                            span.line, span.col,
                            &format!(
                                "Function '{}' expects {} argument(s) but got {}",
                                nf.name, arity, args.len()
                            ),
                        ));
                    }
                }
                (nf.func)(args)
            }
            Value::Class(class) => self.instantiate_class(class, args),
            other => Err(CocotteError::type_err(&format!(
                "'{}' is not callable", other.type_name()
            ))),
        }
    }

    fn call_function(
        &mut self,
        func: &CocotteFunction,
        args: Vec<Value>,
        self_val: Option<Value>,
    ) -> Result<Value> {
        // Save the current environment, build a child scope on top of it
        let saved_env = std::mem::replace(&mut self.env, Environment::new());
        let mut new_env = Environment::with_parent(saved_env);
        new_env.restore_from_snapshot(&func.closure);

        // Bind parameters
        for (i, param) in func.params.iter().enumerate() {
            let val = args.get(i).cloned().unwrap_or(Value::Nil);
            new_env.define_local(param, val);
        }
        // Inject `self` if present
        if let Some(sv) = self_val {
            new_env.define_local("self", sv);
        }

        // Inject global user-defined functions so library funcs can call each other
        // without needing them in the closure snapshot (avoids exponential blowup)
        let global_fns = self.env.top_scope_functions();
        for (k, v) in global_fns {
            if !new_env.has_local(&k) {
                new_env.define_local(&k, v);
            }
        }
        self.env = new_env;
        let result = self.exec_block(&func.body);
        // Restore the parent (saved) environment
        let child = std::mem::replace(&mut self.env, Environment::new());
        self.env = child.into_parent().unwrap_or_else(Environment::new);

        match result {
            Ok(v) => Ok(v),
            Err(e) => match e.signal {
                Some(Signal::Return(v)) => Ok(v),
                _ => Err(e),
            },
        }
    }

    fn call_method(
        &mut self,
        obj: Value,
        method: &str,
        args: Vec<Value>,
        span: &Span,
    ) -> Result<Value> {
        match obj.clone() {
            Value::Instance(inst_arc) => {
                // Check fields first for callable values
                let (method_func, methods) = {
                    let inst = inst_arc.lock().unwrap();
                    let field_val = inst.fields.get(method).cloned();
                    let method_clone = inst.methods.get(method).cloned();
                    (field_val, method_clone)
                };

                if let Some(callable) = method_func {
                    return self.call_value(callable, args, span);
                }
                if let Some(func) = methods {
                    let self_val = Value::Instance(inst_arc.clone());
                    let result = self.call_function(&func, args, Some(self_val))?;
                    // inst_arc is already mutated in-place via Arc<Mutex> — no copy needed
                    return Ok(result);
                }
                Err(CocotteError::runtime_at(
                    span.line, span.col,
                    &format!("No method '{}' on instance", method),
                ))
            }
            Value::Module(ns) => {
                let func = ns.lock().unwrap().get(method).cloned()
                    .ok_or_else(|| CocotteError::runtime_at(
                        span.line, span.col,
                        &format!("Module has no member '{}'", method),
                    ))?;
                self.call_value(func, args, span)
            }
            Value::Str(s) => self.string_method(&s, method, args, span),
            Value::List(l) => self.list_method(l, method, args, span),
            Value::Map(m) => self.map_method(m, method, args, span),
            _ => Err(CocotteError::runtime_at(
                span.line, span.col,
                &format!("Cannot call method '{}' on {}", method, obj.type_name()),
            )),
        }
    }

    // ── Field / Index access ──────────────────────────────────────────────────

    fn get_field(&self, obj: Value, field: &str, span: &Span) -> Result<Value> {
        match obj {
            Value::Instance(inst) => {
                let inst = inst.lock().unwrap();
                inst.fields.get(field)
                    .or_else(|| inst.methods.get(field).map(|_| &Value::Nil))
                    .cloned()
                    .or_else(|| {
                        // Return a bound method wrapper (simplified: just the func)
                        inst.methods.get(field)
                            .map(|f| Value::Function(f.clone()))
                    })
                    .ok_or_else(|| CocotteError::runtime_at(
                        span.line, span.col,
                        &format!("Instance has no field '{}'", field),
                    ))
            }
            Value::Module(ns) => {
                ns.lock().unwrap().get(field).cloned()
                    .ok_or_else(|| CocotteError::runtime_at(
                        span.line, span.col,
                        &format!("Module has no member '{}'", field),
                    ))
            }
            Value::Map(m) => {
                m.lock().unwrap().get(field).cloned()
                    .ok_or_else(|| CocotteError::runtime_at(
                        span.line, span.col,
                        &format!("Map has no key '{}'", field),
                    ))
            }
            _ => Err(CocotteError::runtime_at(
                span.line, span.col,
                &format!("Cannot access field '{}' on {}", field, obj.type_name()),
            )),
        }
    }

    fn get_index(&self, obj: Value, idx: Value, span: &Span) -> Result<Value> {
        match obj {
            Value::List(l) => {
                let l = l.lock().unwrap();
                match idx {
                    Value::Number(n) => {
                        let i = n as isize;
                        let len = l.len() as isize;
                        let i = if i < 0 { len + i } else { i };
                        l.get(i as usize).cloned().ok_or_else(|| CocotteError::runtime_at(
                            span.line, span.col,
                            &format!("List index {} out of range (length {})", n, l.len()),
                        ))
                    }
                    _ => Err(CocotteError::type_err("List index must be a number")),
                }
            }
            Value::Map(m) => {
                let key = idx.to_display();
                m.lock().unwrap().get(&key).cloned()
                    .ok_or_else(|| CocotteError::runtime_at(
                        span.line, span.col,
                        &format!("Map has no key '{}'", key),
                    ))
            }
            Value::Str(s) => {
                match idx {
                    Value::Number(n) => {
                        let i = n as usize;
                        s.chars().nth(i)
                            .map(|c| Value::Str(c.to_string()))
                            .ok_or_else(|| CocotteError::runtime_at(
                                span.line, span.col,
                                &format!("String index {} out of range", n),
                            ))
                    }
                    _ => Err(CocotteError::type_err("String index must be a number")),
                }
            }
            _ => Err(CocotteError::runtime_at(
                span.line, span.col,
                &format!("Cannot index into {}", obj.type_name()),
            )),
        }
    }

    // ── Assignment helpers ────────────────────────────────────────────────────

    fn assign_target(&mut self, target: &AssignTarget, val: Value, span: &Span) -> Result<()> {
        match target {
            AssignTarget::Ident(name) => {
                // Try assign first (existing var); fall back to define
                if self.env.assign(name, val.clone()).is_err() {
                    self.env.define(name, val);
                }
                Ok(())
            }
            AssignTarget::Field(obj_expr, field) => {
                let obj = self.eval_expr(obj_expr)?;
                match obj {
                    Value::Instance(inst) => {
                        inst.lock().unwrap().fields.insert(field.clone(), val);
                        Ok(())
                    }
                    Value::Map(m) => {
                        m.lock().unwrap().insert(field.clone(), val);
                        Ok(())
                    }
                    _ => Err(CocotteError::runtime_at(
                        span.line, span.col,
                        &format!("Cannot set field '{}' on {}", field, obj.type_name()),
                    )),
                }
            }
            AssignTarget::Index(obj_expr, idx_expr) => {
                let obj = self.eval_expr(obj_expr)?;
                let idx = self.eval_expr(idx_expr)?;
                match obj {
                    Value::List(l) => {
                        match idx {
                            Value::Number(n) => {
                                let mut l = l.lock().unwrap();
                                let i = n as isize;
                                let len = l.len() as isize;
                                let i = if i < 0 { (len + i) as usize } else { i as usize };
                                if i < l.len() {
                                    l[i] = val;
                                    Ok(())
                                } else {
                                    Err(CocotteError::runtime_at(
                                        span.line, span.col,
                                        &format!("List index {} out of range", n),
                                    ))
                                }
                            }
                            _ => Err(CocotteError::type_err("List index must be a number")),
                        }
                    }
                    Value::Map(m) => {
                        let key = idx.to_display();
                        m.lock().unwrap().insert(key, val);
                        Ok(())
                    }
                    _ => Err(CocotteError::runtime_at(
                        span.line, span.col,
                        &format!("Cannot index-assign into {}", obj.type_name()),
                    )),
                }
            }
        }
    }

    // ── Class instantiation ───────────────────────────────────────────────────

    fn instantiate_class(&mut self, class: CocotteClass, args: Vec<Value>) -> Result<Value> {
        let inst = ClassInstance {
            class_name: class.name.clone(),
            fields: HashMap::new(),
            methods: class.methods.clone(),
        };
        let inst_arc = Arc::new(Mutex::new(inst));
        let instance = Value::Instance(inst_arc.clone());

        // Call `init` if defined
        if let Some(init_fn) = class.methods.get("init") {
            let init_clone = init_fn.clone();
            self.call_function(&init_clone, args, Some(instance.clone()))?;
            // inst_arc is already updated in-place by `self.field = x` assignments
            // inside init (which operate on the same Arc<Mutex>). No copy needed.
        }

        Ok(Value::Instance(inst_arc))
    }

    // ── Value iteration ───────────────────────────────────────────────────────

    fn value_to_iter(&self, val: Value) -> Result<Vec<Value>> {
        match val {
            Value::List(l) => Ok(l.lock().unwrap().clone()),
            Value::Str(s) => Ok(s.chars().map(|c| Value::Str(c.to_string())).collect()),
            Value::Map(m) => Ok(m.lock().unwrap().keys().cloned().map(Value::Str).collect()),
            _ => Err(CocotteError::type_err(&format!(
                "Cannot iterate over {}", val.type_name()
            ))),
        }
    }

    // ── Built-in methods on primitive types ───────────────────────────────────

    fn string_method(&self, s: &str, method: &str, args: Vec<Value>, span: &Span) -> Result<Value> {
        let chars: Vec<char> = s.chars().collect();
        match method {
            "upper"       => Ok(Value::Str(s.to_uppercase())),
            "lower"       => Ok(Value::Str(s.to_lowercase())),
            "trim"        => Ok(Value::Str(s.trim().to_string())),
            "trim_left"   => Ok(Value::Str(s.trim_start().to_string())),
            "trim_right"  => Ok(Value::Str(s.trim_end().to_string())),
            "len"         => Ok(Value::Number(chars.len() as f64)),
            "is_empty"    => Ok(Value::Bool(s.is_empty())),
            "to_number"   => s.trim().parse::<f64>()
                .map(Value::Number)
                .map_err(|_| CocotteError::runtime(&format!("Cannot convert '{}' to number", s))),
            "to_list"     => {
                let items = chars.iter().map(|c| Value::Str(c.to_string())).collect();
                Ok(Value::List(Arc::new(Mutex::new(items))))
            }
            "get" => {
                let idx = match args.first() {
                    Some(Value::Number(n)) => *n as usize,
                    _ => return Err(CocotteError::type_err("string.get() requires a number index")),
                };
                chars.get(idx)
                    .map(|c| Value::Str(c.to_string()))
                    .ok_or_else(|| CocotteError::runtime(&format!("String index {} out of range", idx)))
            }
            "slice" => {
                let from = match args.first() { Some(Value::Number(n)) => *n as usize, _ => 0 };
                let to   = match args.get(1)  { Some(Value::Number(n)) => *n as usize, _ => chars.len() };
                let to   = to.min(chars.len());
                if from > to { return Ok(Value::Str(String::new())); }
                Ok(Value::Str(chars[from..to].iter().collect()))
            }
            "index_of" => {
                let pat = args.first().map(|v| v.to_display()).unwrap_or_default();
                Ok(match s.find(pat.as_str()) {
                    Some(byte_idx) => Value::Number(s[..byte_idx].chars().count() as f64),
                    None => Value::Number(-1.0),
                })
            }
            "repeat" => {
                let n = match args.first() { Some(Value::Number(n)) => *n as usize, _ => 1 };
                Ok(Value::Str(s.repeat(n)))
            }
            "pad_left" => {
                let width = match args.first() { Some(Value::Number(n)) => *n as usize, _ => 0 };
                let pad   = args.get(1).map(|v| v.to_display()).unwrap_or_else(|| " ".into());
                let pad_c = pad.chars().next().unwrap_or(' ');
                if chars.len() >= width { return Ok(Value::Str(s.to_string())); }
                let fill = pad_c.to_string().repeat(width - chars.len());
                Ok(Value::Str(fill + s))
            }
            "pad_right" => {
                let width = match args.first() { Some(Value::Number(n)) => *n as usize, _ => 0 };
                let pad   = args.get(1).map(|v| v.to_display()).unwrap_or_else(|| " ".into());
                let pad_c = pad.chars().next().unwrap_or(' ');
                if chars.len() >= width { return Ok(Value::Str(s.to_string())); }
                let fill = pad_c.to_string().repeat(width - chars.len());
                Ok(Value::Str(s.to_string() + &fill))
            }
            "split" => {
                let sep = args.first().map(|v| v.to_display()).unwrap_or_else(|| " ".to_string());
                let parts: Vec<Value> = if sep.is_empty() {
                    // split("") means split into individual characters
                    s.chars().map(|c| Value::Str(c.to_string())).collect()
                } else {
                    s.split(sep.as_str()).map(|p| Value::Str(p.to_string())).collect()
                };
                Ok(Value::List(Arc::new(Mutex::new(parts))))
            }
            "split_lines" => {
                let parts: Vec<Value> = s.lines().map(|l| Value::Str(l.to_string())).collect();
                Ok(Value::List(Arc::new(Mutex::new(parts))))
            }
            "contains"    => {
                let sub = args.first().map(|v| v.to_display()).unwrap_or_default();
                Ok(Value::Bool(s.contains(sub.as_str())))
            }
            "replace"     => {
                let from = args.first().map(|v| v.to_display()).unwrap_or_default();
                let to   = args.get(1).map(|v| v.to_display()).unwrap_or_default();
                Ok(Value::Str(s.replace(from.as_str(), to.as_str())))
            }
            "replace_first" => {
                let from = args.first().map(|v| v.to_display()).unwrap_or_default();
                let to   = args.get(1).map(|v| v.to_display()).unwrap_or_default();
                Ok(Value::Str(s.replacen(from.as_str(), to.as_str(), 1)))
            }
            "starts_with" => {
                let pre = args.first().map(|v| v.to_display()).unwrap_or_default();
                Ok(Value::Bool(s.starts_with(pre.as_str())))
            }
            "ends_with"   => {
                let suf = args.first().map(|v| v.to_display()).unwrap_or_default();
                Ok(Value::Bool(s.ends_with(suf.as_str())))
            }
            other => Err(CocotteError::runtime_at(
                span.line, span.col,
                &format!("String has no method '{}'. Available: upper lower trim trim_left trim_right len is_empty get slice index_of repeat pad_left pad_right split split_lines contains replace replace_first starts_with ends_with to_number to_list", other),
            )),
        }
    }

    fn list_method(
        &mut self,
        l: Arc<Mutex<Vec<Value>>>,
        method: &str,
        args: Vec<Value>,
        span: &Span,
    ) -> Result<Value> {
        match method {
            "push" => {
                let val = args.into_iter().next().ok_or_else(|| {
                    CocotteError::runtime("push() requires one argument")
                })?;
                l.lock().unwrap().push(val);
                Ok(Value::Nil)
            }
            "pop" => {
                // pop()  → remove last; pop(i) → remove at index i
                match args.first() {
                    None => l.lock().unwrap().pop()
                        .ok_or_else(|| CocotteError::runtime("pop() on empty list")),
                    Some(Value::Number(n)) => {
                        let mut l = l.lock().unwrap();
                        let len = l.len();
                        let i = {
                            let idx = *n as isize;
                            if idx < 0 { (len as isize + idx) as usize } else { idx as usize }
                        };
                        if i < len { Ok(l.remove(i)) }
                        else { Err(CocotteError::runtime(&format!("pop({}) out of range (length {})", n, len))) }
                    }
                    _ => Err(CocotteError::type_err("list.pop() takes no argument or a number index")),
                }
            }
            "insert" => {
                // insert(i, val) — insert val before index i
                let idx = match args.get(0) {
                    Some(Value::Number(n)) => *n as isize,
                    _ => return Err(CocotteError::type_err("list.insert(i, val) — i must be a number")),
                };
                let val = args.get(1).cloned().unwrap_or(Value::Nil);
                let mut l = l.lock().unwrap();
                let len = l.len() as isize;
                let i = (if idx < 0 { len + idx } else { idx }).max(0).min(len) as usize;
                l.insert(i, val);
                Ok(Value::Nil)
            }
            "len" => Ok(Value::Number(l.lock().unwrap().len() as f64)),
            "reverse" => {
                l.lock().unwrap().reverse();
                Ok(Value::Nil)
            }
            "contains" => {
                let val = args.into_iter().next().unwrap_or(Value::Nil);
                Ok(Value::Bool(l.lock().unwrap().iter().any(|v| v == &val)))
            }
            "join" => {
                let sep = args.get(0).map(|v| v.to_display()).unwrap_or_default();
                let items: Vec<String> = l.lock().unwrap().iter().map(|v| v.to_display()).collect();
                Ok(Value::Str(items.join(&sep)))
            }
            "sort" => {
                l.lock().unwrap().sort_by(|a, b| match (a, b) {
                    (Value::Number(x), Value::Number(y)) =>
                        x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                    (Value::Str(x), Value::Str(y)) => x.cmp(y),
                    _ => std::cmp::Ordering::Equal,
                });
                Ok(Value::Nil)
            }
            "sort_by" => {
                // sort_by(func(a, b) ... end) — func returns negative/zero/positive number
                let f = match args.into_iter().next() {
                    Some(Value::Function(f)) => f,
                    _ => return Err(CocotteError::type_err(
                        "list.sort_by() requires a comparator function(a, b) returning a number"
                    )),
                };
                let items = l.lock().unwrap().clone();
                // Use a simple insertion sort so we can call the Cocotte function
                // between iterations without fighting the borrow checker.
                // For typical list sizes this is perfectly fine.
                let mut sorted: Vec<Value> = Vec::with_capacity(items.len());
                for item in items {
                    let mut inserted = false;
                    for i in 0..sorted.len() {
                        let cmp = self.call_function(&f, vec![item.clone(), sorted[i].clone()], None)?;
                        let lt = match cmp {
                            Value::Number(n) => n < 0.0,
                            _ => return Err(CocotteError::type_err("sort_by comparator must return a number")),
                        };
                        if lt {
                            sorted.insert(i, item.clone());
                            inserted = true;
                            break;
                        }
                    }
                    if !inserted { sorted.push(item); }
                }
                *l.lock().unwrap() = sorted;
                Ok(Value::Nil)
            }
            "get" => {
                let idx = match args.get(0) {
                    Some(Value::Number(n)) => *n as usize,
                    _ => return Err(CocotteError::type_err("list.get() requires a number index")),
                };
                let l = l.lock().unwrap();
                l.get(idx).cloned().ok_or_else(|| CocotteError::runtime(
                    &format!("Index {} out of range", idx)
                ))
            }
            "slice" => {
                let from = match args.first() { Some(Value::Number(n)) => *n as usize, _ => 0 };
                let to   = match args.get(1)  { Some(Value::Number(n)) => *n as usize, _ => l.lock().unwrap().len() };
                let items = l.lock().unwrap();
                let to = to.min(items.len());
                if from > to { return Ok(Value::List(Arc::new(Mutex::new(Vec::new())))); }
                Ok(Value::List(Arc::new(Mutex::new(items[from..to].to_vec()))))
            }
            "index_of" => {
                let needle = args.first().cloned().unwrap_or(Value::Nil);
                let items = l.lock().unwrap();
                Ok(match items.iter().position(|v| v == &needle) {
                    Some(i) => Value::Number(i as f64),
                    None    => Value::Number(-1.0),
                })
            }
            "find" => {
                let f = match args.into_iter().next() {
                    Some(Value::Function(f)) => f,
                    _ => return Err(CocotteError::type_err("list.find() requires a function")),
                };
                let items = l.lock().unwrap().clone();
                for v in items {
                    let result = self.call_function(&f, vec![v.clone()], None)?;
                    if result.is_truthy() { return Ok(v); }
                }
                Ok(Value::Nil)
            }
            "filter" => {
                let f = match args.into_iter().next() {
                    Some(Value::Function(f)) => f,
                    _ => return Err(CocotteError::type_err("list.filter() requires a function")),
                };
                let items = l.lock().unwrap().clone();
                let mut out = Vec::new();
                for v in items {
                    let result = self.call_function(&f, vec![v.clone()], None)?;
                    if result.is_truthy() { out.push(v); }
                }
                Ok(Value::List(Arc::new(Mutex::new(out))))
            }
            "map" => {
                let f = match args.into_iter().next() {
                    Some(Value::Function(f)) => f,
                    _ => return Err(CocotteError::type_err("list.map() requires a function")),
                };
                let items = l.lock().unwrap().clone();
                let mut out = Vec::new();
                for v in items {
                    out.push(self.call_function(&f, vec![v], None)?);
                }
                Ok(Value::List(Arc::new(Mutex::new(out))))
            }
            "reduce" => {
                let f = match args.first().cloned() {
                    Some(Value::Function(f)) => f,
                    _ => return Err(CocotteError::type_err("list.reduce() requires (function, initial_value)")),
                };
                let init = args.get(1).cloned().unwrap_or(Value::Number(0.0));
                let items = l.lock().unwrap().clone();
                let mut acc = init;
                for v in items {
                    acc = self.call_function(&f, vec![acc, v], None)?;
                }
                Ok(acc)
            }
            "each" => {
                let f = match args.into_iter().next() {
                    Some(Value::Function(f)) => f,
                    _ => return Err(CocotteError::type_err("list.each() requires a function")),
                };
                let items = l.lock().unwrap().clone();
                for v in items {
                    self.call_function(&f, vec![v], None)?;
                }
                Ok(Value::Nil)
            }
            "first"   => l.lock().unwrap().first().cloned().ok_or_else(|| CocotteError::runtime("list.first() on empty list")),
            "last"    => l.lock().unwrap().last().cloned().ok_or_else(|| CocotteError::runtime("list.last() on empty list")),
            "is_empty" => Ok(Value::Bool(l.lock().unwrap().is_empty())),
            "count" => {
                let f = match args.into_iter().next() {
                    Some(Value::Function(f)) => f,
                    _ => return Ok(Value::Number(l.lock().unwrap().len() as f64)),
                };
                let items = l.lock().unwrap().clone();
                let mut n = 0usize;
                for v in items {
                    let r = self.call_function(&f, vec![v], None)?;
                    if r.is_truthy() { n += 1; }
                }
                Ok(Value::Number(n as f64))
            }
            "extend" => {
                let other = match args.into_iter().next() {
                    Some(Value::List(other)) => other.lock().unwrap().clone(),
                    _ => return Err(CocotteError::type_err("list.extend() requires a list")),
                };
                l.lock().unwrap().extend(other);
                Ok(Value::Nil)
            }
            "copy" => {
                Ok(Value::List(Arc::new(Mutex::new(l.lock().unwrap().clone()))))
            }
            "clear" => {
                l.lock().unwrap().clear();
                Ok(Value::Nil)
            }
            other => Err(CocotteError::runtime_at(
                span.line, span.col,
                &format!("List has no method '{}'. Available: push pop insert get len is_empty first last contains index_of find slice map filter reduce each count extend copy clear reverse sort sort_by join", other),
            )),
        }
    }

    fn map_method(
        &self,
        m: Arc<Mutex<HashMap<String, Value>>>,
        method: &str,
        args: Vec<Value>,
        span: &Span,
    ) -> Result<Value> {
        match method {
            "get" => {
                let key = args.get(0).map(|v| v.to_display()).unwrap_or_default();
                Ok(m.lock().unwrap().get(&key).cloned().unwrap_or(Value::Nil))
            }
            "set" => {
                let key = args.get(0).map(|v| v.to_display()).unwrap_or_default();
                let val = args.get(1).cloned().unwrap_or(Value::Nil);
                m.lock().unwrap().insert(key, val);
                Ok(Value::Nil)
            }
            "has_key" => {
                let key = args.get(0).map(|v| v.to_display()).unwrap_or_default();
                Ok(Value::Bool(m.lock().unwrap().contains_key(&key)))
            }
            "keys" => {
                let keys: Vec<Value> = m.lock().unwrap().keys()
                    .cloned().map(Value::Str).collect();
                Ok(Value::List(Arc::new(Mutex::new(keys))))
            }
            "values" => {
                let vals: Vec<Value> = m.lock().unwrap().values().cloned().collect();
                Ok(Value::List(Arc::new(Mutex::new(vals))))
            }
            "len" => Ok(Value::Number(m.lock().unwrap().len() as f64)),
            "remove" => {
                let key = args.get(0).map(|v| v.to_display()).unwrap_or_default();
                let removed = m.lock().unwrap().remove(&key);
                Ok(removed.unwrap_or(Value::Nil))
            }
            "merge" => {
                let other = match args.into_iter().next() {
                    Some(Value::Map(other)) => other,
                    _ => return Err(CocotteError::type_err("map.merge() requires a map")),
                };
                let other_clone = other.lock().unwrap().clone();
                m.lock().unwrap().extend(other_clone);
                Ok(Value::Nil)
            }
            "entries" => {
                let entries: Vec<Value> = m.lock().unwrap().iter()
                    .map(|(k, v)| Value::List(Arc::new(Mutex::new(vec![
                        Value::Str(k.clone()), v.clone()
                    ]))))
                    .collect();
                Ok(Value::List(Arc::new(Mutex::new(entries))))
            }
            other => Err(CocotteError::runtime_at(
                span.line, span.col,
                &format!("Map has no method '{}'. Available: get set has_key keys values len remove merge entries", other),
            )),
        }
    }
}

// ── Helper macros ─────────────────────────────────────────────────────────────

macro_rules! num_op {
    ($lv:expr, $rv:expr, $op:tt, $name:expr) => {
        match (&$lv, &$rv) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a $op b)),
            _ => Err(CocotteError::type_err(&format!(
                "Cannot {} {} and {}",
                $name, $lv.type_name(), $rv.type_name()
            ))),
        }
    };
}

macro_rules! cmp_op {
    ($lv:expr, $rv:expr, $op:tt) => {
        match (&$lv, &$rv) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a $op b)),
            (Value::Str(a), Value::Str(b)) => Ok(Value::Bool(a.as_str() $op b.as_str())),
            _ => Err(CocotteError::type_err(&format!(
                "Cannot compare {} and {}", $lv.type_name(), $rv.type_name()
            ))),
        }
    };
}

use num_op;
use cmp_op;
