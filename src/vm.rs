// vm.rs — Stack-based bytecode virtual machine for Cocotte
// Executes compiled bytecode chunks from the compiler.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::bytecode::{Chunk, Instruction};
use crate::error::{CocotteError, Result, Signal};
use crate::value::{Value, CocotteFunction, CocotteClass, ClassInstance, NativeFunction};
use crate::builtins::register_builtins;
use crate::modules::{load_module, load_library};

/// A single call frame on the call stack
struct Frame {
    instructions: Vec<Instruction>,
    ip: usize,           // instruction pointer
    locals: HashMap<String, Value>,
}

impl Frame {
    fn new(instructions: Vec<Instruction>) -> Self {
        Frame { instructions, ip: 0, locals: HashMap::new() }
    }
}

/// Iterator wrapper for `for` loops
#[derive(Clone, Debug)]
struct CocotteIter {
    items: Vec<Value>,
    pos: usize,
}

impl CocotteIter {
    fn next_item(&mut self) -> Option<Value> {
        if self.pos < self.items.len() {
            let item = self.items[self.pos].clone();
            self.pos += 1;
            Some(item)
        } else {
            None
        }
    }
}

pub struct VM {
    stack: Vec<Value>,
    globals: HashMap<String, Value>,
    call_stack: Vec<Frame>,
    pub project_root: std::path::PathBuf,
    pub debug: bool,
}

impl VM {
    pub fn new() -> Self {
        let mut globals = HashMap::new();
        register_builtins(&mut globals);
        VM {
            stack: Vec::new(),
            globals,
            call_stack: Vec::new(),
            project_root: std::env::current_dir().unwrap_or_default(),
            debug: false,
        }
    }

    pub fn run(&mut self, chunk: Chunk) -> Result<Value> {
        self.call_stack.push(Frame::new(chunk.instructions));

        loop {
            // Are we done with all frames?
            if self.call_stack.is_empty() {
                return Ok(self.stack.pop().unwrap_or(Value::Nil));
            }

            let instr = {
                let frame = self.call_stack.last_mut().unwrap();
                if frame.ip >= frame.instructions.len() {
                    // Frame finished without explicit Return — pop it
                    self.call_stack.pop();
                    continue;
                }
                let i = frame.instructions[frame.ip].clone();
                frame.ip += 1;
                i
            };

            if self.debug {
                eprintln!("[vm] depth={} {:?}  stack={}", self.call_stack.len(), instr, self.stack.len());
            }

            match instr {
                Instruction::LoadConst(val) => {
                    self.stack.push(val);
                }

                Instruction::LoadName(name) => {
                    // Check frame locals first, then globals
                    let val = {
                        let frame = self.call_stack.last().unwrap();
                        frame.locals.get(&name).cloned()
                    }.or_else(|| self.globals.get(&name).cloned())
                    .ok_or_else(|| CocotteError::runtime(&format!(
                        "Undefined variable '{}'. Did you declare it with 'var'?", name
                    )))?;
                    self.stack.push(val);
                }

                Instruction::StoreName(name) => {
                    let val = self.pop()?;
                    // Try to update existing global first; otherwise store in current frame
                    if self.globals.contains_key(&name) {
                        self.globals.insert(name, val);
                    } else {
                        let frame = self.call_stack.last_mut().unwrap();
                        frame.locals.insert(name, val);
                    }
                }

                Instruction::Pop => { self.stack.pop(); }

                Instruction::Dup => {
                    let top = self.stack.last().cloned()
                        .ok_or_else(|| CocotteError::runtime("Stack underflow on Dup"))?;
                    self.stack.push(top);
                }

                Instruction::Add => {
                    let (l, r) = self.pop2()?;
                    let result = match (&l, &r) {
                        (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
                        (Value::Str(a), Value::Str(b)) => Value::Str(format!("{}{}", a, b)),
                        (Value::Str(a), _) => Value::Str(format!("{}{}", a, r.to_display())),
                        (_, Value::Str(b)) => Value::Str(format!("{}{}", l.to_display(), b)),
                        _ => return Err(CocotteError::type_err(&format!(
                            "Cannot add {} and {}", l.type_name(), r.type_name()
                        ))),
                    };
                    self.stack.push(result);
                }

                Instruction::Sub => { let (l, r) = self.pop2()?; self.stack.push(num_binop!(l, r, -, "subtract")?); }
                Instruction::Mul => { let (l, r) = self.pop2()?; self.stack.push(num_binop!(l, r, *, "multiply")?); }
                Instruction::Div => {
                    let (l, r) = self.pop2()?;
                    match (&l, &r) {
                        (Value::Number(a), Value::Number(b)) => {
                            if *b == 0.0 { return Err(CocotteError::runtime("Division by zero").with_hint("Check that your divisor is not zero")); }
                            self.stack.push(Value::Number(a / b));
                        }
                        _ => return Err(CocotteError::type_err("Cannot divide non-numbers")),
                    }
                }
                Instruction::Mod => { let (l, r) = self.pop2()?; self.stack.push(num_binop!(l, r, %, "modulo")?); }
                Instruction::Neg => {
                    let v = self.pop()?;
                    match v {
                        Value::Number(n) => self.stack.push(Value::Number(-n)),
                        _ => return Err(CocotteError::type_err("Cannot negate non-number")),
                    }
                }
                Instruction::Not => {
                    let v = self.pop()?;
                    self.stack.push(Value::Bool(!v.is_truthy()));
                }
                Instruction::Eq    => { let (l, r) = self.pop2()?; self.stack.push(Value::Bool(l == r)); }
                Instruction::NotEq => { let (l, r) = self.pop2()?; self.stack.push(Value::Bool(l != r)); }
                Instruction::Lt    => { let (l, r) = self.pop2()?; self.stack.push(cmp_val!(l, r, <)?); }
                Instruction::LtEq  => { let (l, r) = self.pop2()?; self.stack.push(cmp_val!(l, r, <=)?); }
                Instruction::Gt    => { let (l, r) = self.pop2()?; self.stack.push(cmp_val!(l, r, >)?); }
                Instruction::GtEq  => { let (l, r) = self.pop2()?; self.stack.push(cmp_val!(l, r, >=)?); }

                Instruction::Jump(target) => {
                    self.call_stack.last_mut().unwrap().ip = target;
                }
                Instruction::JumpIfFalse(target) => {
                    let cond = self.pop()?;
                    if !cond.is_truthy() {
                        self.call_stack.last_mut().unwrap().ip = target;
                    }
                }
                Instruction::JumpIfTrue(target) => {
                    let cond = self.pop()?;
                    if cond.is_truthy() {
                        self.call_stack.last_mut().unwrap().ip = target;
                    }
                }

                Instruction::MakeFunc { name, params, code } => {
                    let closure = self.call_stack.last().unwrap().locals.clone();
                    let func = Value::Function(CocotteFunction {
                        name,
                        params,
                        body: Vec::new(),
                        closure,
                        bytecode: Some(code),
                    });
                    self.stack.push(func);
                }

                Instruction::Return => {
                    let ret_val = self.stack.pop().unwrap_or(Value::Nil);
                    self.call_stack.pop(); // exit current frame
                    self.stack.push(ret_val);
                }

                Instruction::Call(arity) => {
                    let mut args: Vec<Value> = (0..arity)
                        .map(|_| self.pop().unwrap_or(Value::Nil))
                        .collect();
                    args.reverse();
                    let callee = self.pop()?;
                    let result = self.call_value(callee, args)?;
                    self.stack.push(result);
                }

                Instruction::CallMethod { method, arity } => {
                    let mut args: Vec<Value> = (0..arity)
                        .map(|_| self.pop().unwrap_or(Value::Nil))
                        .collect();
                    args.reverse();
                    let obj = self.pop()?;
                    let result = self.call_method_on(obj, &method, args)?;
                    self.stack.push(result);
                }

                Instruction::MakeList(n) => {
                    let mut items: Vec<Value> = (0..n)
                        .map(|_| self.pop().unwrap_or(Value::Nil))
                        .collect();
                    items.reverse();
                    self.stack.push(Value::List(Arc::new(Mutex::new(items))));
                }

                Instruction::MakeMap(n) => {
                    let mut map: HashMap<String, Value> = HashMap::new();
                    for _ in 0..n {
                        let val = self.pop()?;
                        let key = self.pop()?.to_display();
                        map.insert(key, val);
                    }
                    self.stack.push(Value::Map(Arc::new(Mutex::new(map))));
                }

                Instruction::GetField(field) => {
                    let obj = self.pop()?;
                    let val = self.get_field_vm(obj, &field)?;
                    self.stack.push(val);
                }
                Instruction::SetField(field) => {
                    let obj = self.pop()?;
                    let val = self.pop()?;
                    self.set_field_vm(obj, &field, val)?;
                }
                Instruction::GetIndex => {
                    let idx = self.pop()?;
                    let obj = self.pop()?;
                    let val = self.get_index_vm(obj, idx)?;
                    self.stack.push(val);
                }
                Instruction::SetIndex => {
                    let val = self.pop()?;
                    let idx = self.pop()?;
                    let obj = self.pop()?;
                    self.set_index_vm(obj, idx, val)?;
                }

                Instruction::LoadModule(name) => {
                    let module = load_module(&name, &self.project_root)?;
                    self.stack.push(module);
                }
                Instruction::LoadLibrary(path) => {
                    let lib = load_library(&path, &self.project_root)?;
                    self.stack.push(lib);
                }

                Instruction::GetIter => {
                    let val = self.pop()?;
                    let items = self.value_to_iter(val)?;
                    self.stack.push(Value::List(Arc::new(Mutex::new(items))));
                    self.stack.push(Value::Number(0.0));
                }
                Instruction::ForIter(exit_target) => {
                    let pos_val = self.pop()?;
                    let list_val = self.stack.last().cloned()
                        .ok_or_else(|| CocotteError::runtime("ForIter: missing list"))?;
                    let pos = match pos_val { Value::Number(n) => n as usize, _ => 0 };
                    match &list_val {
                        Value::List(l) => {
                            let item = l.lock().unwrap().get(pos).cloned();
                            match item {
                                Some(v) => {
                                    self.stack.push(Value::Number((pos + 1) as f64));
                                    self.stack.push(v);
                                }
                                None => {
                                    self.stack.pop(); // pop list
                                    self.call_stack.last_mut().unwrap().ip = exit_target;
                                }
                            }
                        }
                        _ => return Err(CocotteError::runtime("ForIter on non-list")),
                    }
                }
                Instruction::StoreIter(name) => {
                    let val = self.pop()?;
                    let frame = self.call_stack.last_mut().unwrap();
                    frame.locals.insert(name, val);
                }

                Instruction::MakeClass { name, methods } => {
                    let closure = self.call_stack.last().unwrap().locals.clone();
                    let method_map: HashMap<String, CocotteFunction> = methods.into_iter()
                        .map(|(mname, mparams, code)| {
                            let f = CocotteFunction {
                                name: Some(mname.clone()),
                                params: mparams,
                                body: Vec::new(),
                                closure: closure.clone(),
                                bytecode: Some(code),
                            };
                            (mname, f)
                        })
                        .collect();
                    self.stack.push(Value::Class(CocotteClass { name, methods: method_map }));
                }


                Instruction::TryCatch { body_code, catch_var, catch_code } => {
                    self.call_stack.push(Frame::new(body_code));
                    let try_depth = self.call_stack.len();
                    let mut try_result: Result<()> = Ok(());
                    while self.call_stack.len() >= try_depth {
                        try_result = self.step();
                        if try_result.is_err() {
                            // Drain remaining try frames
                            while self.call_stack.len() >= try_depth {
                                self.call_stack.pop();
                            }
                            break;
                        }
                    }
                    match try_result {
                        Ok(_) => {}
                        Err(e) if !e.is_signal() => {
                            let mut catch_frame = Frame::new(catch_code);
                            if let Some(var) = catch_var {
                                catch_frame.locals.insert(var, Value::Str(e.message.clone()));
                            }
                            self.call_stack.push(catch_frame);
                            let catch_depth = self.call_stack.len();
                            while self.call_stack.len() >= catch_depth {
                                self.step()?;
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }

                Instruction::Print => {
                    let val = self.pop()?;
                    println!("{}", val.to_display());
                }
            }
        }
    }

    // ── Stack helpers ─────────────────────────────────────────────────────────

    fn pop(&mut self) -> Result<Value> {
        self.stack.pop().ok_or_else(|| CocotteError::runtime("Stack underflow"))
    }

    fn pop2(&mut self) -> Result<(Value, Value)> {
        let r = self.pop()?;
        let l = self.pop()?;
        Ok((l, r))
    }

    // ── Dispatch helpers ──────────────────────────────────────────────────────

    fn call_value(&mut self, callee: Value, args: Vec<Value>) -> Result<Value> {
        match callee {
            Value::Function(f) => {
                match f.bytecode {
                    Some(code) => {
                        // Push a new frame for the bytecode function
                        let mut new_frame = Frame::new(code);
                        // Inject closure
                        for (k, v) in &f.closure {
                            new_frame.locals.insert(k.clone(), v.clone());
                        }
                        // Bind parameters
                        for (i, param) in f.params.iter().enumerate() {
                            new_frame.locals.insert(param.clone(), args.get(i).cloned().unwrap_or(Value::Nil));
                        }
                        self.call_stack.push(new_frame);
                        // Run continues in the main loop; we need to run until this frame returns
                        // We implement this by running the inner frames eagerly here
                        self.run_until_frame_returns()
                    }
                    None => {
                        // Tree-walk fallback: run via interpreter
                        Err(CocotteError::runtime(&format!(
                            "Function '{}' has no bytecode — use tree-walk mode",
                            f.name.as_deref().unwrap_or("<lambda>")
                        )))
                    }
                }
            }
            Value::NativeFunction(nf) => {
                if let Some(arity) = nf.arity {
                    if args.len() != arity {
                        return Err(CocotteError::runtime(&format!(
                            "'{}' expects {} arguments, got {}",
                            nf.name, arity, args.len()
                        )));
                    }
                }
                (nf.func)(args)
            }
            Value::Class(class) => {
                let inst_arc = Arc::new(Mutex::new(ClassInstance {
                    class_name: class.name.clone(),
                    fields: HashMap::new(),
                    methods: class.methods.clone(),
                }));
                // Call init if present
                let init_fn = class.methods.get("init").cloned();
                if let Some(init) = init_fn {
                    let self_val = Value::Instance(inst_arc.clone());
                    let mut new_frame = Frame::new(init.bytecode.unwrap_or_default());
                    for (k, v) in &init.closure {
                        new_frame.locals.insert(k.clone(), v.clone());
                    }
                    new_frame.locals.insert("self".to_string(), self_val);
                    for (i, param) in init.params.iter().enumerate() {
                        new_frame.locals.insert(param.clone(), args.get(i).cloned().unwrap_or(Value::Nil));
                    }
                    self.call_stack.push(new_frame);
                    self.run_until_frame_returns()?;
                }
                Ok(Value::Instance(inst_arc))
            }
            other => Err(CocotteError::type_err(&format!(
                "'{}' is not callable", other.type_name()
            ))),
        }
    }

    /// Run until the most-recently-pushed frame completes, then return its result
    fn run_until_frame_returns(&mut self) -> Result<Value> {
        let depth = self.call_stack.len();
        loop {
            if self.call_stack.len() < depth {
                // Our frame was popped by a Return instruction
                return Ok(self.stack.pop().unwrap_or(Value::Nil));
            }
            if self.call_stack.len() == depth {
                let frame = self.call_stack.last().unwrap();
                if frame.ip >= frame.instructions.len() {
                    self.call_stack.pop();
                    return Ok(self.stack.pop().unwrap_or(Value::Nil));
                }
            }

            let instr = {
                let frame = self.call_stack.last_mut().unwrap();
                let i = frame.instructions[frame.ip].clone();
                frame.ip += 1;
                i
            };

            // Re-use the main dispatch by pushing onto the run loop
            // Simple approach: inline key instructions, for others push back
            match &instr {
                Instruction::Return => {
                    let ret_val = self.stack.pop().unwrap_or(Value::Nil);
                    self.call_stack.pop();
                    self.stack.push(ret_val);
                    if self.call_stack.len() < depth {
                        return Ok(self.stack.pop().unwrap_or(Value::Nil));
                    }
                }
                _ => {
                    // Push instruction back and let the main loop handle it
                    // Actually — we can't do that easily. Instead we duplicate dispatch here.
                    // We handle via a helper that processes one instruction.
                    self.call_stack.last_mut().unwrap().ip -= 1; // undo increment
                    self.step()?;
                }
            }
        }
    }

    /// Execute exactly one instruction from the top of the call stack
    fn step(&mut self) -> Result<()> {
        let instr = {
            let frame = self.call_stack.last_mut()
                .ok_or_else(|| CocotteError::runtime("No frame on call stack"))?;
            if frame.ip >= frame.instructions.len() {
                self.call_stack.pop();
                return Ok(());
            }
            let i = frame.instructions[frame.ip].clone();
            frame.ip += 1;
            i
        };

        match instr {
            Instruction::LoadConst(val) => { self.stack.push(val); }

            Instruction::LoadName(name) => {
                let val = {
                    let frame = self.call_stack.last().unwrap();
                    frame.locals.get(&name).cloned()
                }.or_else(|| self.globals.get(&name).cloned())
                .ok_or_else(|| CocotteError::runtime(&format!(
                    "Undefined variable '{}'. Did you declare it with 'var'?", name
                )))?;
                self.stack.push(val);
            }

            Instruction::StoreName(name) => {
                let val = self.pop()?;
                if self.globals.contains_key(&name) {
                    self.globals.insert(name, val);
                } else {
                    let frame = self.call_stack.last_mut().unwrap();
                    frame.locals.insert(name, val);
                }
            }

            Instruction::Pop => { self.stack.pop(); }
            Instruction::Dup => {
                let top = self.stack.last().cloned()
                    .ok_or_else(|| CocotteError::runtime("Stack underflow on Dup"))?;
                self.stack.push(top);
            }

            Instruction::Add => {
                let (l, r) = self.pop2()?;
                let result = match (&l, &r) {
                    (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
                    (Value::Str(a), Value::Str(b)) => Value::Str(format!("{}{}", a, b)),
                    (Value::Str(a), _) => Value::Str(format!("{}{}", a, r.to_display())),
                    (_, Value::Str(b)) => Value::Str(format!("{}{}", l.to_display(), b)),
                    _ => return Err(CocotteError::type_err("Cannot add these types")),
                };
                self.stack.push(result);
            }
            Instruction::Sub  => { let (l,r) = self.pop2()?; self.stack.push(num_binop!(l,r,-,"subtract")?); }
            Instruction::Mul  => { let (l,r) = self.pop2()?; self.stack.push(num_binop!(l,r,*,"multiply")?); }
            Instruction::Mod  => { let (l,r) = self.pop2()?; self.stack.push(num_binop!(l,r,%,"modulo")?); }
            Instruction::Div  => {
                let (l, r) = self.pop2()?;
                match (&l, &r) {
                    (Value::Number(a), Value::Number(b)) => {
                        if *b == 0.0 { return Err(CocotteError::runtime("Division by zero")); }
                        self.stack.push(Value::Number(a / b));
                    }
                    _ => return Err(CocotteError::type_err("Cannot divide non-numbers")),
                }
            }
            Instruction::Neg => {
                let v = self.pop()?;
                match v { Value::Number(n) => self.stack.push(Value::Number(-n)), _ => return Err(CocotteError::type_err("Cannot negate non-number")), }
            }
            Instruction::Not => { let v = self.pop()?; self.stack.push(Value::Bool(!v.is_truthy())); }

            Instruction::Eq    => { let (l,r) = self.pop2()?; self.stack.push(Value::Bool(l==r)); }
            Instruction::NotEq => { let (l,r) = self.pop2()?; self.stack.push(Value::Bool(l!=r)); }
            Instruction::Lt    => { let (l,r) = self.pop2()?; self.stack.push(cmp_val!(l,r,<)?); }
            Instruction::LtEq  => { let (l,r) = self.pop2()?; self.stack.push(cmp_val!(l,r,<=)?); }
            Instruction::Gt    => { let (l,r) = self.pop2()?; self.stack.push(cmp_val!(l,r,>)?); }
            Instruction::GtEq  => { let (l,r) = self.pop2()?; self.stack.push(cmp_val!(l,r,>=)?); }

            Instruction::Jump(t)        => { self.call_stack.last_mut().unwrap().ip = t; }
            Instruction::JumpIfFalse(t) => { let c = self.pop()?; if !c.is_truthy() { self.call_stack.last_mut().unwrap().ip = t; } }
            Instruction::JumpIfTrue(t)  => { let c = self.pop()?; if  c.is_truthy() { self.call_stack.last_mut().unwrap().ip = t; } }

            Instruction::MakeFunc { name, params, code } => {
                let closure = self.call_stack.last().unwrap().locals.clone();
                self.stack.push(Value::Function(CocotteFunction { name, params, body: Vec::new(), closure, bytecode: Some(code) }));
            }

            Instruction::Return => {
                let ret_val = self.stack.pop().unwrap_or(Value::Nil);
                self.call_stack.pop();
                self.stack.push(ret_val);
            }

            Instruction::Call(arity) => {
                let mut args: Vec<Value> = (0..arity).map(|_| self.pop().unwrap_or(Value::Nil)).collect();
                args.reverse();
                let callee = self.pop()?;
                let result = self.call_value(callee, args)?;
                self.stack.push(result);
            }

            Instruction::CallMethod { method, arity } => {
                let mut args: Vec<Value> = (0..arity).map(|_| self.pop().unwrap_or(Value::Nil)).collect();
                args.reverse();
                let obj = self.pop()?;
                let result = self.call_method_on(obj, &method, args)?;
                self.stack.push(result);
            }

            Instruction::MakeList(n) => {
                let mut items: Vec<Value> = (0..n).map(|_| self.pop().unwrap_or(Value::Nil)).collect();
                items.reverse();
                self.stack.push(Value::List(Arc::new(Mutex::new(items))));
            }

            Instruction::MakeMap(n) => {
                let mut map: HashMap<String, Value> = HashMap::new();
                for _ in 0..n { let v = self.pop()?; let k = self.pop()?.to_display(); map.insert(k, v); }
                self.stack.push(Value::Map(Arc::new(Mutex::new(map))));
            }

            Instruction::GetField(f)  => { let o = self.pop()?; self.stack.push(self.get_field_vm(o, &f)?); }
            Instruction::SetField(f)  => { let o = self.pop()?; let v = self.pop()?; self.set_field_vm(o, &f, v)?; }
            Instruction::GetIndex     => { let i = self.pop()?; let o = self.pop()?; self.stack.push(self.get_index_vm(o, i)?); }
            Instruction::SetIndex     => { let v = self.pop()?; let i = self.pop()?; let o = self.pop()?; self.set_index_vm(o, i, v)?; }

            Instruction::LoadModule(name) => {
                let m = load_module(&name, &self.project_root)?;
                self.stack.push(m);
            }
            Instruction::LoadLibrary(path) => {
                let l = load_library(&path, &self.project_root)?;
                self.stack.push(l);
            }

            Instruction::GetIter => {
                let val = self.pop()?;
                let items = self.value_to_iter(val)?;
                self.stack.push(Value::List(Arc::new(Mutex::new(items))));
                self.stack.push(Value::Number(0.0));
            }
            Instruction::ForIter(exit) => {
                let pos_val = self.pop()?;
                let list_val = self.stack.last().cloned().ok_or_else(|| CocotteError::runtime("ForIter: missing list"))?;
                let pos = match pos_val { Value::Number(n) => n as usize, _ => 0 };
                match &list_val {
                    Value::List(l) => {
                        let item = l.lock().unwrap().get(pos).cloned();
                        match item {
                            Some(v) => { self.stack.push(Value::Number((pos+1) as f64)); self.stack.push(v); }
                            None    => { self.stack.pop(); self.call_stack.last_mut().unwrap().ip = exit; }
                        }
                    }
                    _ => return Err(CocotteError::runtime("ForIter on non-list")),
                }
            }
            Instruction::StoreIter(name) => {
                let v = self.pop()?;
                self.call_stack.last_mut().unwrap().locals.insert(name, v);
            }

            Instruction::MakeClass { name, methods } => {
                let closure = self.call_stack.last().unwrap().locals.clone();
                let method_map = methods.into_iter().map(|(mname, mparams, code)| {
                    let f = CocotteFunction {
                        name: Some(mname.clone()),
                        params: mparams,
                        body: Vec::new(),
                        closure: closure.clone(),
                        bytecode: Some(code),
                    };
                    (mname, f)
                }).collect();
                self.stack.push(Value::Class(CocotteClass { name, methods: method_map }));
            }


            Instruction::TryCatch { body_code, catch_var, catch_code } => {
                self.call_stack.push(Frame::new(body_code));
                let try_depth = self.call_stack.len();
                let mut try_result: Result<()> = Ok(());
                while self.call_stack.len() >= try_depth {
                    try_result = self.step();
                    if try_result.is_err() {
                        while self.call_stack.len() >= try_depth {
                            self.call_stack.pop();
                        }
                        break;
                    }
                }
                match try_result {
                    Ok(_) => {}
                    Err(e) if !e.is_signal() => {
                        let mut catch_frame = Frame::new(catch_code);
                        if let Some(var) = catch_var {
                            catch_frame.locals.insert(var, Value::Str(e.message.clone()));
                        }
                        self.call_stack.push(catch_frame);
                        let catch_depth = self.call_stack.len();
                        while self.call_stack.len() >= catch_depth {
                            self.step()?;
                        }
                    }
                    Err(e) => return Err(e),
                }
            }

            Instruction::Print => { let v = self.pop()?; println!("{}", v.to_display()); }
        }
        Ok(())
    }

    fn call_method_on(&mut self, obj: Value, method: &str, args: Vec<Value>) -> Result<Value> {
        match &obj {
            Value::Module(ns) => {
                let func = ns.lock().unwrap().get(method).cloned()
                    .ok_or_else(|| CocotteError::runtime(
                        &format!("Module has no member '{}'", method)
                    ))?;
                self.call_value(func, args)
            }
            Value::Instance(inst) => {
                // Try instance methods (bytecode)
                let meth = inst.lock().unwrap().methods.get(method).cloned();
                if let Some(m) = meth {
                    let self_val = Value::Instance(inst.clone());
                    let code = m.bytecode.unwrap_or_default();
                    let mut new_frame = Frame::new(code);
                    for (k, v) in &m.closure { new_frame.locals.insert(k.clone(), v.clone()); }
                    new_frame.locals.insert("self".to_string(), self_val);
                    for (i, p) in m.params.iter().enumerate() {
                        new_frame.locals.insert(p.clone(), args.get(i).cloned().unwrap_or(Value::Nil));
                    }
                    self.call_stack.push(new_frame);
                    return self.run_until_frame_returns();
                }
                // Try field callables
                let field = inst.lock().unwrap().fields.get(method).cloned();
                if let Some(f) = field { return self.call_value(f, args); }
                Err(CocotteError::runtime(&format!("Instance has no method '{}'", method)))
            }
            Value::List(l) => {
                match method {
                    "push" => {
                        let val = args.into_iter().next().unwrap_or(Value::Nil);
                        l.lock().unwrap().push(val);
                        Ok(Value::Nil)
                    }
                    "pop" => {
                        Ok(l.lock().unwrap().pop().unwrap_or(Value::Nil))
                    }
                    "len" => Ok(Value::Number(l.lock().unwrap().len() as f64)),
                    "get" => {
                        let idx = match args.first() { Some(Value::Number(n)) => *n as usize, _ => return Err(CocotteError::type_err("get() requires a number index")) };
                        Ok(l.lock().unwrap().get(idx).cloned().unwrap_or(Value::Nil))
                    }
                    "contains" => {
                        let needle = args.first().cloned().unwrap_or(Value::Nil);
                        Ok(Value::Bool(l.lock().unwrap().contains(&needle)))
                    }
                    "reverse" => { l.lock().unwrap().reverse(); Ok(Value::Nil) }
                    "sort" => {
                        let mut v = l.lock().unwrap();
                        v.sort_by(|a, b| a.to_display().cmp(&b.to_display()));
                        Ok(Value::Nil)
                    }
                    "join" => {
                        let sep = match args.first() { Some(Value::Str(s)) => s.clone(), _ => String::new() };
                        let joined = l.lock().unwrap().iter().map(|v| v.to_display()).collect::<Vec<_>>().join(&sep);
                        Ok(Value::Str(joined))
                    }
                    _ => Err(CocotteError::runtime(&format!("List has no method '{}'", method)))
                }
            }
            Value::Map(m) => {
                match method {
                    "get" => {
                        let key = args.first().map(|v| v.to_display()).unwrap_or_default();
                        Ok(m.lock().unwrap().get(&key).cloned().unwrap_or(Value::Nil))
                    }
                    "set" => {
                        let key = args.first().map(|v| v.to_display()).unwrap_or_default();
                        let val = args.into_iter().nth(1).unwrap_or(Value::Nil);
                        m.lock().unwrap().insert(key, val);
                        Ok(Value::Nil)
                    }
                    "has_key" => {
                        let key = args.first().map(|v| v.to_display()).unwrap_or_default();
                        Ok(Value::Bool(m.lock().unwrap().contains_key(&key)))
                    }
                    "keys"   => Ok(Value::List(Arc::new(Mutex::new(m.lock().unwrap().keys().map(|k| Value::Str(k.clone())).collect())))),
                    "values" => Ok(Value::List(Arc::new(Mutex::new(m.lock().unwrap().values().cloned().collect())))),
                    "len"    => Ok(Value::Number(m.lock().unwrap().len() as f64)),
                    _ => Err(CocotteError::runtime(&format!("Map has no method '{}'", method)))
                }
            }
            Value::Str(s) => {
                let s = s.clone();
                match method {
                    "len"         => Ok(Value::Number(s.chars().count() as f64)),
                    "upper"       => Ok(Value::Str(s.to_uppercase())),
                    "lower"       => Ok(Value::Str(s.to_lowercase())),
                    "trim"        => Ok(Value::Str(s.trim().to_string())),
                    "to_number"   => s.trim().parse::<f64>().map(Value::Number).map_err(|_| CocotteError::runtime("Cannot convert string to number")),
                    "contains"    => { let pat = args.first().map(|v| v.to_display()).unwrap_or_default(); Ok(Value::Bool(s.contains(&pat as &str))) }
                    "starts_with" => { let pat = args.first().map(|v| v.to_display()).unwrap_or_default(); Ok(Value::Bool(s.starts_with(&pat as &str))) }
                    "ends_with"   => { let pat = args.first().map(|v| v.to_display()).unwrap_or_default(); Ok(Value::Bool(s.ends_with(&pat as &str))) }
                    "replace"     => {
                        let from = args.first().map(|v| v.to_display()).unwrap_or_default();
                        let to   = args.get(1).map(|v| v.to_display()).unwrap_or_default();
                        Ok(Value::Str(s.replace(&from as &str, &to as &str)))
                    }
                    "split" => {
                        let sep = args.first().map(|v| v.to_display()).unwrap_or_default();
                        let parts = s.split(&sep as &str).map(|p| Value::Str(p.to_string())).collect();
                        Ok(Value::List(Arc::new(Mutex::new(parts))))
                    }
                    _ => Err(CocotteError::runtime(&format!("String has no method '{}'", method)))
                }
            }
            _ => Err(CocotteError::runtime(&format!(
                "Cannot call method '{}' on {}", method, obj.type_name()
            ))),
        }
    }

    fn get_field_vm(&self, obj: Value, field: &str) -> Result<Value> {
        match obj {
            Value::Module(ns) => ns.lock().unwrap().get(field).cloned()
                .ok_or_else(|| CocotteError::runtime(&format!("Module has no member '{}'", field))),
            Value::Instance(inst) => inst.lock().unwrap().fields.get(field).cloned()
                .ok_or_else(|| CocotteError::runtime(&format!("Instance has no field '{}'", field))),
            Value::Map(m) => m.lock().unwrap().get(field).cloned()
                .ok_or_else(|| CocotteError::runtime(&format!("Map has no key '{}'", field))),
            _ => Err(CocotteError::type_err(&format!(
                "Cannot access field '{}' on {}", field, obj.type_name()
            ))),
        }
    }

    fn set_field_vm(&self, obj: Value, field: &str, val: Value) -> Result<()> {
        match obj {
            Value::Instance(inst) => {
                inst.lock().unwrap().fields.insert(field.to_string(), val);
                Ok(())
            }
            Value::Map(m) => {
                m.lock().unwrap().insert(field.to_string(), val);
                Ok(())
            }
            _ => Err(CocotteError::type_err(&format!(
                "Cannot set field '{}' on {}", field, obj.type_name()
            ))),
        }
    }

    fn get_index_vm(&self, obj: Value, idx: Value) -> Result<Value> {
        match obj {
            Value::List(l) => match idx {
                Value::Number(n) => l.lock().unwrap().get(n as usize).cloned()
                    .ok_or_else(|| CocotteError::runtime(&format!("Index {} out of range", n))),
                _ => Err(CocotteError::type_err("List index must be a number")),
            },
            Value::Map(m) => {
                let key = idx.to_display();
                m.lock().unwrap().get(&key).cloned()
                    .ok_or_else(|| CocotteError::runtime(&format!("Map has no key '{}'", key)))
            }
            _ => Err(CocotteError::type_err(&format!(
                "Cannot index into {}", obj.type_name()
            ))),
        }
    }

    fn set_index_vm(&self, obj: Value, idx: Value, val: Value) -> Result<()> {
        match obj {
            Value::List(l) => match idx {
                Value::Number(n) => {
                    let mut l = l.lock().unwrap();
                    let i = n as usize;
                    if i < l.len() {
                        l[i] = val;
                        Ok(())
                    } else {
                        Err(CocotteError::runtime(&format!("Index {} out of range", n)))
                    }
                }
                _ => Err(CocotteError::type_err("List index must be a number")),
            },
            Value::Map(m) => {
                m.lock().unwrap().insert(idx.to_display(), val);
                Ok(())
            }
            _ => Err(CocotteError::type_err("Cannot index-assign into this type")),
        }
    }

    fn value_to_iter(&self, val: Value) -> Result<Vec<Value>> {
        match val {
            Value::List(l) => Ok(l.lock().unwrap().clone()),
            Value::Str(s) => Ok(s.chars().map(|c| Value::Str(c.to_string())).collect()),
            _ => Err(CocotteError::type_err(&format!(
                "Cannot iterate over {}", val.type_name()
            ))),
        }
    }
}

// ── Helper macros ─────────────────────────────────────────────────────────────

macro_rules! num_binop {
    ($l:expr, $r:expr, $op:tt, $name:expr) => {
        match (&$l, &$r) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a $op b)),
            _ => Err(CocotteError::type_err(&format!(
                "Cannot {} {} and {}", $name, $l.type_name(), $r.type_name()
            ))),
        }
    };
}

macro_rules! cmp_val {
    ($l:expr, $r:expr, $op:tt) => {
        match (&$l, &$r) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a $op b)),
            (Value::Str(a), Value::Str(b)) => Ok(Value::Bool(a.as_str() $op b.as_str())),
            _ => Err(CocotteError::type_err(&format!(
                "Cannot compare {} and {}", $l.type_name(), $r.type_name()
            ))),
        }
    };
}

use num_binop;
use cmp_val;
