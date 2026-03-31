//! native_codegen.rs — True AOT native compilation for `cocotte build --native`
//!
//! Strategy:
//!   1. Walk the Cocotte AST and emit a self-contained Rust source file that
//!      imports a thin "value" shim (no parser, no interpreter).
//!   2. Invoke `cargo build [--release] [--target <triple>]` on that workspace.
//!   3. Copy the resulting binary to dist/.
//!
//! The emitted code is 100% memory-safe Rust — no `unsafe` in user code.
//! The `Val` type is a small enum (Nil/Bool/Num/Str/List/Map) with Clone.
//!
//! Limitations (v0.2 — will improve):
//!   - No class inheritance (same as interpreter)
//!   - No bytecode VM in native mode (tree-walk AOT only)
//!   - Module calls are resolved at build time via inline Rust stubs
//!   - Closures capture by clone (not by reference)

use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::Path;
use colored::Colorize;
use crate::ast::*;
use crate::codegen::{BuildOptions, TargetOS, TargetArch};
use crate::error::{CocotteError, Result};

// ── Public entry point ────────────────────────────────────────────────────────

pub fn build_native(opts: &BuildOptions, _source: &str, stmts: &Program) -> Result<()> {
    step("Transpiling", &format!("{} → Rust (native AOT)", opts.project_name.bold()));

    let rust_src = emit_rust(stmts, &opts.project_name)
        .map_err(|e| CocotteError::build_err(&format!("transpiler error: {}", e)))?;

    fs::create_dir_all(&opts.output_dir)?;

    let n_targets = opts.targets.len();
    let plural    = if n_targets == 1 { "target" } else { "targets" };
    step("Compiling", &format!("{} native {} ({} {})",
        opts.project_name.bold(), "binary".cyan(), n_targets, plural));

    for (os, arch) in &opts.targets {
        build_native_target(opts, &rust_src, os, arch)?;
    }

    step("Finished", &format!("native — output: {}", opts.output_dir.display().to_string().cyan()));
    Ok(())
}

// ── Per-target native build ───────────────────────────────────────────────────

fn build_native_target(
    opts:     &BuildOptions,
    rust_src: &str,
    os:       &TargetOS,
    arch:     &TargetArch,
) -> Result<()> {
    let label = match (os, arch) {
        (TargetOS::Current, TargetArch::Current) => "native".to_string(),
        _ => format!("{}-{}", os.name(), arch.name()),
    };
    step("Targeting", &label);

    let triple_opt = os.rust_target(arch);
    if triple_opt.is_none() {
        step_warn("Warning", &format!(
            "unsupported native target {}/{} — emitting source bundle",
            os.name(), arch.name()
        ));
        return emit_native_bundle(opts, rust_src, &label);
    }
    let triple = triple_opt.unwrap();

    let tmp_dir = std::env::temp_dir()
        .join(format!("cocotte_native_{}_{}", opts.project_name, label));
    fs::create_dir_all(&tmp_dir)?;
    let src_dir = tmp_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    // Write the generated Rust source
    fs::write(src_dir.join("main.rs"), rust_src)?;

    // Write a Cargo.toml for the native crate (no cocotte_rt dependency!)
    let cargo_toml = format!(r#"[package]
name    = "{name}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{name}"
path = "src/main.rs"

[profile.release]
opt-level = 3
lto       = true
strip     = true
"#, name = opts.project_name);
    fs::write(tmp_dir.join("Cargo.toml"), cargo_toml)?;

    let binary_name = os.binary_name(&opts.project_name, arch);
    let out_path    = opts.output_dir.join(&binary_name);

    let mut cargo_args: Vec<String> = vec!["build".into()];
    if opts.release       { cargo_args.push("--release".into()); }
    if !triple.is_empty() { cargo_args.push("--target".into()); cargo_args.push(triple.clone()); }

    if opts.verbose {
        substep(&format!("cargo {}", cargo_args.join(" ")));
        substep(&format!("workspace: {}", tmp_dir.display()));
    }

    step("Linking", &format!("{} (native cargo{}{})",
        opts.project_name,
        if opts.release { " --release" } else { "" },
        if !triple.is_empty() { format!(" --target {}", triple) } else { String::new() },
    ));

    match run_native_cargo(&cargo_args, &tmp_dir, opts.verbose) {
        Ok(status) if status.success() => {
            let profile = if opts.release { "release" } else { "debug" };
            let bin_name = if matches!(os, TargetOS::Windows) {
                format!("{}.exe", opts.project_name)
            } else {
                opts.project_name.clone()
            };
            let native_path = tmp_dir.join("target").join(profile).join(&bin_name);
            let cross_path  = tmp_dir.join("target").join(&triple).join(profile).join(&bin_name);
            match [native_path, cross_path].into_iter().find(|p| p.exists()) {
                Some(p) => {
                    fs::copy(&p, &out_path)?;
                    step("Compiled", &out_path.display().to_string().green().bold().to_string());
                }
                None => {
                    step_warn("Warning", "cargo succeeded but binary not found");
                    emit_native_bundle(opts, rust_src, &label)?;
                }
            }
        }
        Ok(_) => {
            step_warn("Warning", "cargo build failed — emitting Rust source bundle");
            if !opts.verbose {
                substep("re-run with --verbose to see the full cargo output");
            }
            emit_native_bundle(opts, rust_src, &label)?;
        }
        Err(_) => {
            step_warn("Warning", "cargo not found — emitting Rust source bundle");
            substep("install Rust from https://rustup.rs then retry");
            emit_native_bundle(opts, rust_src, &label)?;
        }
    }
    Ok(())
}

fn emit_native_bundle(opts: &BuildOptions, rust_src: &str, target: &str) -> Result<()> {
    let bundle_dir = opts.output_dir
        .join(format!("{}_{}_native_src", opts.project_name, target));
    let src_dir = bundle_dir.join("src");
    fs::create_dir_all(&src_dir)?;
    fs::write(src_dir.join("main.rs"), rust_src)?;
    fs::write(bundle_dir.join("Cargo.toml"), format!(r#"[package]
name    = "{name}"
version = "0.1.0"
edition = "2021"
[[bin]]
name = "{name}"
path = "src/main.rs"
[profile.release]
opt-level = 3
lto       = true
strip     = true
"#, name = opts.project_name))?;
    fs::write(bundle_dir.join("README.md"), format!(
        "# {} — Native Rust Source Bundle\n\nCompile with:\n\n```sh\ncargo build --release\n```\n\nTarget: {}\n",
        opts.project_name, target
    ))?;
    step("Bundling", &format!("Rust source → {}", bundle_dir.display()));
    Ok(())
}

fn run_native_cargo(
    args:     &[String],
    work_dir: &Path,
    verbose:  bool,
) -> std::io::Result<std::process::ExitStatus> {
    if verbose {
        return std::process::Command::new("cargo")
            .args(args)
            .current_dir(work_dir)
            .status();
    }
    std::process::Command::new("cargo")
        .args(args)
        .current_dir(work_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .status()
}

// ── Step printers (mirror codegen.rs) ────────────────────────────────────────

fn step(verb: &str, detail: &str) {
    println!("{:>12} {}", verb.green().bold(), detail);
}
fn step_warn(verb: &str, detail: &str) {
    println!("{:>12} {}", verb.yellow().bold(), detail);
}
fn substep(detail: &str) {
    println!("             {}", detail.dimmed());
}

// ── AST → Rust transpiler ─────────────────────────────────────────────────────

fn emit_rust(program: &Program, project_name: &str) -> std::result::Result<String, String> {
    let mut out = String::new();

    // Runtime header — embedded Val type
    writeln!(out, "// Auto-generated by cocotte build --native").ok();
    writeln!(out, "// Project: {}", project_name).ok();
    writeln!(out, "// DO NOT EDIT — regenerate with: cocotte build --native").ok();
    writeln!(out, "").ok();
    out.push_str(NATIVE_RUNTIME_HEADER);
    writeln!(out, "").ok();
    writeln!(out, "fn main() {{").ok();
    writeln!(out, "    let _env: Env = env_new();").ok();
    writeln!(out, "    _register_builtins(&_env);").ok();

    let mut transpiler = Transpiler::new();
    for stmt in &program.statements {
        let code = transpiler.emit_stmt(stmt, 1)?;
        out.push_str(&code);
    }

    writeln!(out, "}}").ok();
    Ok(out)
}

struct Transpiler {
    tmp_counter: usize,
}

impl Transpiler {
    fn new() -> Self { Transpiler { tmp_counter: 0 } }

    fn fresh(&mut self) -> String {
        self.tmp_counter += 1;
        format!("_t{}", self.tmp_counter)
    }

    fn indent(level: usize) -> String {
        "    ".repeat(level)
    }

    fn emit_stmt(&mut self, stmt: &Stmt, depth: usize) -> std::result::Result<String, String> {
        let ind = Self::indent(depth);
        match stmt {
            Stmt::VarDecl { name, value, .. } => {
                let val = self.emit_expr(value, depth)?;
                Ok(format!("{ind}env_set(&_env, {name:?}, {val});\n",
                    ind = ind, name = name, val = val))
            }

            Stmt::Assign { target, value, .. } => {
                let val = self.emit_expr(value, depth)?;
                match target {
                    AssignTarget::Ident(n) => {
                        Ok(format!("{ind}env_set(&_env, {key:?}, {val});\n",
                            ind = ind, val = val, key = n.as_str()))
                    }
                    AssignTarget::Field(obj, field) => {
                        let obj_e = self.emit_expr(obj, depth)?;
                        Ok(format!("{ind}val_set_field(&{obj}, \"{field}\", {val});\n",
                            ind = ind, obj = obj_e, field = field, val = val))
                    }
                    AssignTarget::Index(obj, idx) => {
                        let obj_e = self.emit_expr(obj, depth)?;
                        let idx_e = self.emit_expr(idx, depth)?;
                        Ok(format!("{ind}val_set_index(&{obj}, {idx}, {val});\n",
                            ind = ind, obj = obj_e, idx = idx_e, val = val))
                    }
                }
            }

            Stmt::ExprStmt { expr, .. } => {
                let e = self.emit_expr(expr, depth)?;
                Ok(format!("{ind}let _ = {e};\n", ind = ind, e = e))
            }

            Stmt::Print { value, .. } => {
                let e = self.emit_expr(value, depth)?;
                Ok(format!("{ind}println!(\"{{}}\", val_display(&{e}));\n", ind = ind, e = e))
            }

            Stmt::Return { value, .. } => {
                let e = match value {
                    Some(v) => self.emit_expr(v, depth)?,
                    None    => "Val::Nil".into(),
                };
                Ok(format!("{ind}return {e};\n", ind = ind, e = e))
            }

            Stmt::If { condition, then_branch, elif_branches, else_branch, .. } => {
                let cond = self.emit_expr(condition, depth)?;
                let mut s = format!("{ind}if val_truthy(&{cond}) {{\n", ind = ind, cond = cond);
                for stmt in then_branch {
                    s.push_str(&self.emit_stmt(stmt, depth + 1)?);
                }
                for (elif_cond, elif_body) in elif_branches {
                    let ec = self.emit_expr(elif_cond, depth)?;
                    s.push_str(&format!("{ind}}} else if val_truthy(&{ec}) {{\n", ind = ind, ec = ec));
                    for stmt in elif_body {
                        s.push_str(&self.emit_stmt(stmt, depth + 1)?);
                    }
                }
                if let Some(else_stmts) = else_branch {
                    s.push_str(&format!("{ind}}} else {{\n", ind = ind));
                    for stmt in else_stmts {
                        s.push_str(&self.emit_stmt(stmt, depth + 1)?);
                    }
                }
                s.push_str(&format!("{ind}}}\n", ind = ind));
                Ok(s)
            }

            Stmt::While { condition, body, .. } => {
                let cond = self.emit_expr(condition, depth)?;
                let cond_name = self.fresh();
                let mut s = format!("{ind}let mut {cn} = {cond};\n", ind = ind, cn = cond_name, cond = cond);
                s.push_str(&format!("{ind}while val_truthy(&{cn}) {{\n", ind = ind, cn = cond_name));
                for stmt in body {
                    s.push_str(&self.emit_stmt(stmt, depth + 1)?);
                }
                let cond2 = self.emit_expr(condition, depth + 1)?;
                s.push_str(&format!("    {ind}{cn} = {cond2};\n", ind = ind, cn = cond_name, cond2 = cond2));
                s.push_str(&format!("{ind}}}\n", ind = ind));
                Ok(s)
            }

            Stmt::For { var: variable, iterable, body, .. } => {
                let iter_e = self.emit_expr(iterable, depth)?;
                let iter_name = self.fresh();
                let loop_var  = self.fresh();
                let mut s = format!("{ind}let {it} = val_to_iter({ie});\n",
                    ind = ind, it = iter_name, ie = iter_e);
                s.push_str(&format!("{ind}for {lv} in {it} {{\n",
                    ind = ind, lv = loop_var, it = iter_name));
                s.push_str(&format!("{ind}    env_set(&_env, {vk:?}, {lv}.clone());\n",
                    ind = ind, vk = variable.as_str(), lv = loop_var));
                for stmt in body {
                    s.push_str(&self.emit_stmt(stmt, depth + 1)?);
                }
                s.push_str(&format!("{ind}}}\n", ind = ind));
                Ok(s)
            }

            Stmt::FuncDecl { name, params, body, .. } => {
                // In native mode functions become closures assigned to Val::Func
                let param_list = params.iter().map(|p| escape_name(p)).collect::<Vec<_>>().join(", ");
                let _ = param_list; // will be used below
                // Emit as a closure stored in env
                let mut s = String::new();
                // Simple function stub — parameters bound by position
                s.push_str(&format!("{ind}// func {name}\n", ind = ind, name = name));
                // Capture outer env before closure so it's available inside
                let cap_name = self.fresh();
                s.push_str(&format!("{ind}let {cap} = _env.clone();\n", ind = ind, cap = cap_name));
                s.push_str(&format!("{ind}let _fn_{name} = Val::Func(std::rc::Rc::new(move |_args: Vec<Val>| -> Val {{\n",
                    ind = ind, name = escape_name(name)));
                // Create child scope inheriting outer env, then bind params
                s.push_str(&format!("    let _env = env_child(&{cap});\n", cap = cap_name));
                for (i, p) in params.iter().enumerate() {
                    s.push_str(&format!("    env_set(&_env, {pk:?}, _args.get({i}).cloned().unwrap_or(Val::Nil));\n",
                        pk = p.as_str(), i = i));
                }
                let mut inner = Transpiler::new();
                for stmt in body {
                    s.push_str(&inner.emit_stmt(stmt, depth + 2)?);
                }
                s.push_str(&format!("{ind}    Val::Nil\n", ind = ind));
                s.push_str(&format!("{ind}}}));\n", ind = ind));
                s.push_str(&format!("{ind}env_set(&_env, {name:?}, _fn_{name});\n",
                    ind = ind, name = name));
                Ok(s)
            }

            Stmt::Break { .. }    => Ok(format!("{ind}break;\n",    ind = ind)),
            Stmt::Continue { .. } => Ok(format!("{ind}continue;\n", ind = ind)),

            Stmt::Try { body, catch_var, catch_body, .. } => {
                // Native try-catch: wrap body in a closure, catch std::panic via catch_unwind.
                let mut s = String::new();
                let _catch_name = escape_name(catch_var.as_deref().unwrap_or("_err")); // kept for potential future use
                s.push_str(&format!("{ind}// try-catch\n", ind = ind));
                // Suppress panic stderr output during try block by installing a silent hook
                s.push_str(&format!("{ind}std::panic::set_hook(Box::new(|_| {{}}));\n", ind = ind));
                s.push_str(&format!("{ind}let _try_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Val {{\n", ind = ind));
                let mut inner = Transpiler::new();
                for stmt in body {
                    s.push_str(&inner.emit_stmt(stmt, depth + 1)?);
                }
                s.push_str(&format!("{ind}    Val::Nil\n", ind = ind));
                s.push_str(&format!("{ind}}}));\n", ind = ind));
                // Restore default panic hook after try block
                s.push_str(&format!("{ind}let _ = std::panic::take_hook();\n", ind = ind));
                s.push_str(&format!("{ind}if let Err(_panic_val) = _try_result {{\n", ind = ind));
                // Extract the panic message if it's a &str or String
                let ck = catch_var.as_deref().unwrap_or("_err");
                s.push_str(&format!("{ind}    let _err_msg = _panic_val.downcast_ref::<String>().map(|s| s.as_str()).or_else(|| _panic_val.downcast_ref::<&str>().copied()).unwrap_or(\"runtime error\");\n", ind = ind));
                s.push_str(&format!("{ind}    env_set(&_env, {ck:?}, Val::Str(_err_msg.to_string()));\n", ind = ind, ck = ck));
                let mut cerr = Transpiler::new();
                for stmt in catch_body {
                    s.push_str(&cerr.emit_stmt(stmt, depth + 1)?);
                }
                s.push_str(&format!("{ind}}}\n", ind = ind));
                Ok(s)
            }

            Stmt::ModuleAdd { name, .. } => {
                // In native mode modules are registered as Val::Map in _env at startup.
                // Emit a no-op (built-ins are pre-registered; stdlib cotlib not available in native mode).
                Ok(format!("{ind}// module add \"{name}\" (native: built-ins pre-registered)\n",
                    ind = ind, name = name))
            }
            Stmt::LibraryAdd { path, .. } => {
                // Libraries are Cocotte source; not available in AOT native mode.
                Ok(format!("{ind}// library add \"{path}\" (native: cotlib not available in AOT mode)\n",
                    ind = ind, path = path))
            }

            Stmt::ClassDecl { name, methods, .. } => {
                // Native classes: factory closure returning a Val::Map with method closures.
                let mut s = String::new();
                s.push_str(&format!("{ind}// class {name}\n", ind = ind, name = name));
                let en = escape_name(name);
                // Capture outer env before factory closure (Rc clone = cheap)
                let class_env_cap = self.fresh();
                s.push_str(&format!("{ind}let {cec} = _env.clone();\n", ind = ind, cec = class_env_cap));
                s.push_str(&format!("{ind}let _class_{en} = Val::Func(std::rc::Rc::new({{let _cenv = {cec}; move |_ctor_args: Vec<Val>| -> Val {{\n", ind = ind, en = en, cec = class_env_cap));
                // Inside factory, expose _env so method captures work
                s.push_str(&format!("{ind}    let _env = _cenv.clone();\n", ind = ind));
                s.push_str(&format!("{ind}    use std::collections::HashMap;\n", ind = ind));
                s.push_str(&format!("{ind}    let _inst = std::rc::Rc::new(std::cell::RefCell::new(HashMap::<String, Val>::new()));\n", ind = ind));
                s.push_str(&format!("{ind}    let _self_val = Val::Map(_inst.clone());\n", ind = ind));
                // Emit each method (methods are FuncDecl Stmts)
                for method_stmt in methods {
                    if let Stmt::FuncDecl { name: mname, params, body, .. } = method_stmt {
                        let mn = escape_name(mname);
                        // _self passed as arg — no need to pre-capture _self_c
                        // Method closure: env for outer scope access; _self = first arg (the receiver instance)
                        let menv_cap = self.fresh();
                        s.push_str(&format!("{ind}    let {mc} = _env.clone();\n", ind = ind, mc = menv_cap));
                        s.push_str(&format!("{ind}    let _m_{mn} = Val::Func(std::rc::Rc::new({{let _menv_cap = {mc}.clone(); move |_margs: Vec<Val>| -> Val {{\n", ind = ind, mn = mn, mc = menv_cap));
                        s.push_str(&format!("{ind}        let _env = env_child(&_menv_cap);\n", ind = ind));
                        // _self is the receiver (self), passed as _margs[0] by val_method_call
                        s.push_str(&format!("{ind}        let _self = _margs.get(0).cloned().unwrap_or(Val::Nil);\n", ind = ind));
                        // User params start at _margs[1] (after self)
                        for (i, p) in params.iter().enumerate() {
                            let arg_idx = i + 1;
                            s.push_str(&format!("{ind}        env_set(&_env, {pk:?}, _margs.get({ai}).cloned().unwrap_or(Val::Nil));\n",
                                ind = ind, pk = p.as_str(), ai = arg_idx));
                        }
                        let mut inner = Transpiler::new();
                        for stmt in body {
                            s.push_str(&inner.emit_stmt(stmt, 3)?);
                        }
                        s.push_str(&format!("{ind}        Val::Nil\n", ind = ind));
                        s.push_str(&format!("{ind}    }}}}));\n", ind = ind));
                        s.push_str(&format!("{ind}    _inst.borrow_mut().insert(\"{mn}\".to_string(), _m_{mn});\n", ind = ind, mn = mn));
                    }
                }
                // Call init constructor with ctor args
                // Extract init_fn in a separate statement so the Ref<HashMap> borrow
                // from _inst.borrow() is dropped BEFORE val_call runs.
                s.push_str(&format!("{ind}    let _init_opt = _inst.borrow().get(\"init\").cloned();\n", ind = ind));
                s.push_str(&format!("{ind}    if let Some(init_fn) = _init_opt {{\n", ind = ind));
                // Pass self as first arg (receiver) followed by constructor args
                s.push_str(&format!("{ind}        let mut _init_args = vec![_self_val.clone()];\n", ind = ind));
                s.push_str(&format!("{ind}        _init_args.extend(_ctor_args.iter().cloned());\n", ind = ind));
                s.push_str(&format!("{ind}        val_call(init_fn, _init_args);\n", ind = ind));
                s.push_str(&format!("{ind}    }}\n", ind = ind));
                s.push_str(&format!("{ind}    _self_val\n", ind = ind));
                s.push_str(&format!("{ind}}}}}));\n", ind = ind));
                s.push_str(&format!("{ind}env_set(&_env, {name:?}, _class_{en});\n", ind = ind, name = name, en = en));
                Ok(s)
            }
        }
    }

    fn emit_expr(&mut self, expr: &Expr, depth: usize) -> std::result::Result<String, String> {
        match expr {
            Expr::Number(n, ..) => Ok(format!("Val::Num({:?})", n)),
            Expr::StringLit(s, ..)    => Ok(format!("Val::Str({:?}.to_string())", s)),
            Expr::Bool(b, ..)   => Ok(format!("Val::Bool({})", b)),
            Expr::Nil       => Ok("Val::Nil".into()),
            Expr::Ident(name, ..) => Ok(format!("env_get(&_env, {:?})", name)),

            Expr::List(items, ..) => {
                let parts: std::result::Result<Vec<_>, _> = items.iter()
                    .map(|e| self.emit_expr(e, depth))
                    .collect();
                Ok(format!("Val::List(std::rc::Rc::new(std::cell::RefCell::new(vec![{}])))",
                    parts?.join(", ")))
            }

            Expr::Map(pairs, ..) => {
                let mut entries = Vec::new();
                for (k, v) in pairs {
                    let k_s = self.emit_expr(k, depth)?;
                    let v_s = self.emit_expr(v, depth)?;
                    entries.push(format!("(val_display(&{k}), {v})", k = k_s, v = v_s));
                }
                Ok(format!("Val::Map(std::rc::Rc::new(std::cell::RefCell::new(vec![{}].into_iter().collect())))",
                    entries.join(", ")))
            }

            Expr::BinOp { left, op, right, .. } => {
                let l = self.emit_expr(left, depth)?;
                let r = self.emit_expr(right, depth)?;
                let op_fn = match op {
                    BinOp::Add => "val_add",
                    BinOp::Sub => "val_sub",
                    BinOp::Mul => "val_mul",
                    BinOp::Div => "val_div",
                    BinOp::Mod => "val_mod",
                    BinOp::Eq  => "val_eq",
                    BinOp::NotEq  => "val_ne",
                    BinOp::Lt  => "val_lt",
                    BinOp::Gt  => "val_gt",
                    BinOp::LtEq  => "val_le",
                    BinOp::GtEq  => "val_ge",
                    BinOp::And => "val_and",
                    BinOp::Or  => "val_or",
                };
                Ok(format!("{op}({l}, {r})", op = op_fn, l = l, r = r))
            }

            Expr::UnaryOp { op, operand, .. } => {
                let e = self.emit_expr(operand, depth)?;
                match op {
                    UnaryOp::Neg => Ok(format!("val_neg({e})", e = e)),
                    UnaryOp::Not => Ok(format!("val_not({e})", e = e)),
                }
            }

            Expr::Call { callee, args, .. } => {
                let callee_s = self.emit_expr(callee, depth)?;
                let args_s: std::result::Result<Vec<_>, _> = args.iter()
                    .map(|a| self.emit_expr(a, depth))
                    .collect();
                Ok(format!("val_call({callee}, vec![{args}])",
                    callee = callee_s,
                    args   = args_s?.join(", ")))
            }

            Expr::MethodCall { object, method, args, .. } => {
                let obj_s  = self.emit_expr(object, depth)?;
                let args_s: std::result::Result<Vec<_>, _> = args.iter()
                    .map(|a| self.emit_expr(a, depth))
                    .collect();
                Ok(format!("val_method_call(&{obj}, \"{method}\", vec![{args}])",
                    obj    = obj_s,
                    method = method,
                    args   = args_s?.join(", ")))
            }

            Expr::FieldAccess { object, field, .. } => {
                let obj_s = self.emit_expr(object, depth)?;
                Ok(format!("val_get_field(&{obj}, \"{field}\")",
                    obj = obj_s, field = field))
            }

            Expr::Index { object, index, .. } => {
                let obj_s = self.emit_expr(object, depth)?;
                let idx_s = self.emit_expr(index, depth)?;
                Ok(format!("val_get_index(&{obj}, &{idx})",
                    obj = obj_s, idx = idx_s))
            }

            Expr::Lambda { params, body, .. } => {
                // Lambdas capture outer _env by clone for read access,
                // then create a child scope for their own params.
                let mut s = String::new();
                s.push_str("Val::Func(std::rc::Rc::new({let _lenv = _env.clone(); move |_args: Vec<Val>| -> Val {\n");
                s.push_str("    let _env = env_child(&_lenv);\n");
                for (i, p) in params.iter().enumerate() {
                    s.push_str(&format!("    env_set(&_env, {pk:?}, _args.get({i}).cloned().unwrap_or(Val::Nil));\n",
                        pk = p.as_str(), i = i));
                }
                let mut inner = Transpiler::new();
                for stmt in body {
                    s.push_str(&inner.emit_stmt(stmt, 1)?);
                }
                s.push_str("    Val::Nil\n}}))");
                Ok(s)
            }

            Expr::SelfRef(..) => Ok("_self.clone()".into()),
        }
    }
}

fn escape_name(name: &str) -> String {
    // Avoid Rust keywords
    match name {
        "type" | "loop" | "move" | "ref" | "use" | "where" | "crate" | "super"
        | "self" | "match" | "let" | "mut" | "fn" | "mod" | "pub" | "impl"
        | "trait" | "enum" | "struct" | "if" | "else" | "while" | "for"
        | "in" | "return" | "break" | "continue" | "true" | "false" | "as" => {
            format!("r#{}", name)
        }
        other => other.replace('-', "_"),
    }
}

// ── Embedded runtime header ───────────────────────────────────────────────────
// This is compiled into every native binary. It provides Val + builtins.
// Pure safe Rust, no external deps.

const NATIVE_RUNTIME_HEADER: &str = r#"
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;

// ── Value type ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub enum Val {
    Nil,
    Bool(bool),
    Num(f64),
    Str(String),
    List(Rc<RefCell<Vec<Val>>>),
    Map(Rc<RefCell<HashMap<String, Val>>>),
    Func(Rc<dyn Fn(Vec<Val>) -> Val>),
}

impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", val_display(self))
    }
}

impl fmt::Debug for Val {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Val::Nil      => write!(f, "nil"),
            Val::Bool(b)  => write!(f, "{}", b),
            Val::Num(n)   => write!(f, "{:?}", n),
            Val::Str(s)   => write!(f, "{:?}", s),
            Val::List(_)  => write!(f, "[...]"),
            Val::Map(_)   => write!(f, "{{...}}"),
            Val::Func(_)  => write!(f, "<func>"),
        }
    }
}

// ── Environment ───────────────────────────────────────────────────────────────
// Rc<RefCell<...>> so closures can capture and mutate it without ownership issues.

pub type Env = Rc<RefCell<HashMap<String, Val>>>;

pub fn env_new() -> Env { Rc::new(RefCell::new(HashMap::new())) }
pub fn env_get(env: &Env, key: &str) -> Val {
    env.borrow().get(key).cloned().unwrap_or(Val::Nil)
}
pub fn env_set(env: &Env, key: &str, val: Val) {
    env.borrow_mut().insert(key.to_string(), val);
}
/// Create a child scope pre-populated with parent's bindings (for function calls).
pub fn env_child(parent: &Env) -> Env {
    Rc::new(RefCell::new(parent.borrow().clone()))
}

// ── Value helpers ─────────────────────────────────────────────────────────────

pub fn val_display(v: &Val) -> String {
    match v {
        Val::Nil     => "nil".into(),
        Val::Bool(b) => b.to_string(),
        Val::Num(n)  => {
            if n.fract() == 0.0 && n.abs() < 1e15 { format!("{}", *n as i64) }
            else { format!("{}", n) }
        }
        Val::Str(s)  => s.clone(),
        Val::List(l) => {
            let items: Vec<String> = l.borrow().iter().map(|v| format!("{:?}", v)).collect();
            format!("[{}]", items.join(", "))
        }
        Val::Map(m)  => {
            let items: Vec<String> = m.borrow().iter()
                .map(|(k, v)| format!("{:?}: {:?}", k, v)).collect();
            format!("{{{}}}", items.join(", "))
        }
        Val::Func(_) => "<func>".into(),
    }
}

pub fn val_truthy(v: &Val) -> bool {
    match v {
        Val::Nil     => false,
        Val::Bool(b) => *b,
        Val::Num(n)  => *n != 0.0,
        Val::Str(s)  => !s.is_empty(),
        _            => true,
    }
}

pub fn val_to_iter(v: Val) -> Vec<Val> {
    match v {
        Val::List(l) => l.borrow().clone(),
        Val::Str(s)  => s.chars().map(|c| Val::Str(c.to_string())).collect(),
        _            => vec![],
    }
}

pub fn val_call(callee: Val, args: Vec<Val>) -> Val {
    match callee {
        Val::Func(f) => f(args),
        other => panic!("Cannot call {:?}", other),
    }
}

pub fn val_method_call(obj: &Val, method: &str, args: Vec<Val>) -> Val {
    match obj {
        Val::Str(s) => str_method(s, method, args),
        Val::List(l) => list_method(l.clone(), method, args),
        Val::Map(m)  => {
            // Separate borrow into its own let so the Ref<HashMap> is dropped
            // before val_call — prevents BorrowError in method bodies that
            // call val_get_field / val_set_field on the same map.
            let maybe_func = m.borrow().get(method).cloned();
            if let Some(Val::Func(ref _f)) = maybe_func {
                let func = maybe_func.clone().unwrap();
                let mut full_args = vec![obj.clone()];
                full_args.extend(args);
                return val_call(func, full_args);
            }
            // Not a user method — fall back to built-in map methods
            map_method(m.clone(), method, args)
        }
        _ => panic!("No method '{}' on {:?}", method, obj),
    }
}

pub fn val_get_field(obj: &Val, field: &str) -> Val {
    if let Val::Map(m) = obj { m.borrow().get(field).cloned().unwrap_or(Val::Nil) }
    else { Val::Nil }
}

pub fn val_set_field(obj: &Val, field: &str, val: Val) {
    if let Val::Map(m) = obj { m.borrow_mut().insert(field.to_string(), val); }
}

pub fn val_get_index(obj: &Val, idx: &Val) -> Val {
    match (obj, idx) {
        (Val::List(l), Val::Num(n)) => {
            l.borrow().get(*n as usize).cloned().unwrap_or(Val::Nil)
        }
        (Val::Map(m), key) => {
            m.borrow().get(&val_display(key)).cloned().unwrap_or(Val::Nil)
        }
        _ => Val::Nil,
    }
}

pub fn val_set_index(obj: &Val, idx: Val, val: Val) {
    match (obj, idx) {
        (Val::List(l), Val::Num(n)) => {
            let i = n as usize;
            let mut l = l.borrow_mut();
            if i < l.len() { l[i] = val; }
        }
        (Val::Map(m), key) => {
            m.borrow_mut().insert(val_display(&key), val);
        }
        _ => {}
    }
}

// ── Binary operations ─────────────────────────────────────────────────────────

pub fn val_add(a: Val, b: Val) -> Val {
    match (a, b) {
        (Val::Num(x), Val::Num(y)) => Val::Num(x + y),
        (Val::Str(x), Val::Str(y)) => Val::Str(x + &y),
        (Val::Str(x), y)           => Val::Str(x + &val_display(&y)),
        (x, Val::Str(y))           => Val::Str(val_display(&x) + &y),
        _ => Val::Nil,
    }
}
pub fn val_sub(a: Val, b: Val) -> Val {
    if let (Val::Num(x), Val::Num(y)) = (a, b) { Val::Num(x - y) } else { Val::Nil }
}
pub fn val_mul(a: Val, b: Val) -> Val {
    if let (Val::Num(x), Val::Num(y)) = (a, b) { Val::Num(x * y) } else { Val::Nil }
}
pub fn val_div(a: Val, b: Val) -> Val {
    if let (Val::Num(x), Val::Num(y)) = (a, b) { Val::Num(x / y) } else { Val::Nil }
}
pub fn val_mod(a: Val, b: Val) -> Val {
    if let (Val::Num(x), Val::Num(y)) = (a, b) { Val::Num(x % y) } else { Val::Nil }
}
pub fn val_eq(a: Val, b: Val) -> Val {
    Val::Bool(match (a, b) {
        (Val::Nil, Val::Nil)       => true,
        (Val::Bool(x), Val::Bool(y)) => x == y,
        (Val::Num(x), Val::Num(y))   => x == y,
        (Val::Str(x), Val::Str(y))   => x == y,
        _ => false,
    })
}
pub fn val_ne(a: Val, b: Val) -> Val {
    if let Val::Bool(b) = val_eq(a, b) { Val::Bool(!b) } else { Val::Bool(true) }
}
pub fn val_lt(a: Val, b: Val) -> Val {
    Val::Bool(match (a, b) {
        (Val::Num(x), Val::Num(y)) => x < y,
        (Val::Str(x), Val::Str(y)) => x < y,
        _ => false,
    })
}
pub fn val_gt(a: Val, b: Val) -> Val {
    Val::Bool(match (a, b) {
        (Val::Num(x), Val::Num(y)) => x > y,
        (Val::Str(x), Val::Str(y)) => x > y,
        _ => false,
    })
}
pub fn val_le(a: Val, b: Val) -> Val {
    if let Val::Bool(b) = val_gt(a, b) { Val::Bool(!b) } else { Val::Bool(false) }
}
pub fn val_ge(a: Val, b: Val) -> Val {
    if let Val::Bool(b) = val_lt(a, b) { Val::Bool(!b) } else { Val::Bool(false) }
}
pub fn val_and(a: Val, b: Val) -> Val {
    if val_truthy(&a) { b } else { a }
}
pub fn val_or(a: Val, b: Val) -> Val {
    if val_truthy(&a) { a } else { b }
}
pub fn val_neg(a: Val) -> Val {
    if let Val::Num(n) = a { Val::Num(-n) } else { Val::Nil }
}
pub fn val_not(a: Val) -> Val {
    Val::Bool(!val_truthy(&a))
}

// ── String methods ────────────────────────────────────────────────────────────

fn str_method(s: &str, method: &str, args: Vec<Val>) -> Val {
    match method {
        "len"        => Val::Num(s.chars().count() as f64),
        "upper"      => Val::Str(s.to_uppercase()),
        "lower"      => Val::Str(s.to_lowercase()),
        "trim"       => Val::Str(s.trim().to_string()),
        "trim_left"  => Val::Str(s.trim_start().to_string()),
        "trim_right" => Val::Str(s.trim_end().to_string()),
        "is_empty"   => Val::Bool(s.is_empty()),
        "reverse"    => Val::Str(s.chars().rev().collect()),
        "contains" => {
            let sub = args.first().map(|v| val_display(v)).unwrap_or_default();
            Val::Bool(s.contains(&sub[..]))
        }
        "starts_with" => {
            let pre = args.first().map(|v| val_display(v)).unwrap_or_default();
            Val::Bool(s.starts_with(&pre[..]))
        }
        "ends_with" => {
            let suf = args.first().map(|v| val_display(v)).unwrap_or_default();
            Val::Bool(s.ends_with(&suf[..]))
        }
        "replace" => {
            let from = args.first().map(|v| val_display(v)).unwrap_or_default();
            let to   = args.get(1).map(|v| val_display(v)).unwrap_or_default();
            Val::Str(s.replace(&from[..], &to[..]))
        }
        "split" => {
            let sep = args.first().map(|v| val_display(v)).unwrap_or_default();
            let parts: Vec<Val> = s.split(&sep[..]).map(|p| Val::Str(p.to_string())).collect();
            Val::List(Rc::new(RefCell::new(parts)))
        }
        "slice" => {
            let from = args.first().and_then(|v| if let Val::Num(n) = v { Some(*n as usize) } else { None }).unwrap_or(0);
            let to   = args.get(1).and_then(|v| if let Val::Num(n) = v { Some(*n as usize) } else { None }).unwrap_or(s.len());
            Val::Str(s.chars().skip(from).take(to.saturating_sub(from)).collect())
        }
        "get" => {
            let idx = args.first().and_then(|v| if let Val::Num(n) = v { Some(*n as usize) } else { None }).unwrap_or(0);
            Val::Str(s.chars().nth(idx).map(|c| c.to_string()).unwrap_or_default())
        }
        "to_number" => {
            Val::Num(s.trim().parse::<f64>().unwrap_or(f64::NAN))
        }
        "to_list" => {
            let chars: Vec<Val> = s.chars().map(|c| Val::Str(c.to_string())).collect();
            Val::List(Rc::new(RefCell::new(chars)))
        }
        "index_of" => {
            let sub = args.first().map(|v| val_display(v)).unwrap_or_default();
            Val::Num(s.find(&sub[..]).map(|i| i as f64).unwrap_or(-1.0))
        }
        "repeat" => {
            let n = args.first().and_then(|v| if let Val::Num(n) = v { Some(*n as usize) } else { None }).unwrap_or(0);
            Val::Str(s.repeat(n))
        }
        _ => Val::Nil,
    }
}

// ── List methods ──────────────────────────────────────────────────────────────

fn list_method(l: Rc<RefCell<Vec<Val>>>, method: &str, args: Vec<Val>) -> Val {
    match method {
        "len"      => Val::Num(l.borrow().len() as f64),
        "is_empty" => Val::Bool(l.borrow().is_empty()),
        "first"    => l.borrow().first().cloned().unwrap_or(Val::Nil),
        "last"     => l.borrow().last().cloned().unwrap_or(Val::Nil),
        "push"     => { l.borrow_mut().push(args.into_iter().next().unwrap_or(Val::Nil)); Val::Nil }
        "pop"      => l.borrow_mut().pop().unwrap_or(Val::Nil),
        "reverse"  => { l.borrow_mut().reverse(); Val::Nil }
        "clear"    => { l.borrow_mut().clear(); Val::Nil }
        "copy"     => Val::List(Rc::new(RefCell::new(l.borrow().clone()))),
        "get" => {
            let idx = args.first().and_then(|v| if let Val::Num(n) = v { Some(*n as usize) } else { None }).unwrap_or(0);
            l.borrow().get(idx).cloned().unwrap_or(Val::Nil)
        }
        "contains" => {
            let needle = args.first().cloned().unwrap_or(Val::Nil);
            Val::Bool(l.borrow().iter().any(|v| {
                matches!((v, &needle), (Val::Num(a), Val::Num(b)) if a == b)
                || matches!((v, &needle), (Val::Str(a), Val::Str(b)) if a == b)
                || matches!((v, &needle), (Val::Bool(a), Val::Bool(b)) if a == b)
                || matches!((v, &needle), (Val::Nil, Val::Nil))
            }))
        }
        "join" => {
            let sep = args.first().map(|v| val_display(v)).unwrap_or_default();
            Val::Str(l.borrow().iter().map(val_display).collect::<Vec<_>>().join(&sep))
        }
        "slice" => {
            let from = args.first().and_then(|v| if let Val::Num(n) = v { Some(*n as usize) } else { None }).unwrap_or(0);
            let to   = args.get(1).and_then(|v| if let Val::Num(n) = v { Some(*n as usize) } else { None }).unwrap_or(l.borrow().len());
            Val::List(Rc::new(RefCell::new(l.borrow()[from..to.min(l.borrow().len())].to_vec())))
        }
        "extend" => {
            if let Some(Val::List(other)) = args.first() {
                l.borrow_mut().extend(other.borrow().clone());
            }
            Val::Nil
        }
        "map" => {
            let f = args.into_iter().next().unwrap_or(Val::Nil);
            let items = l.borrow().clone();
            let out: Vec<Val> = items.into_iter().map(|item| val_call(f.clone(), vec![item])).collect();
            Val::List(Rc::new(RefCell::new(out)))
        }
        "filter" => {
            let f = args.into_iter().next().unwrap_or(Val::Nil);
            let items = l.borrow().clone();
            let out: Vec<Val> = items.into_iter().filter(|item| val_truthy(&val_call(f.clone(), vec![item.clone()]))).collect();
            Val::List(Rc::new(RefCell::new(out)))
        }
        "reduce" => {
            let f = args.first().cloned().unwrap_or(Val::Nil);
            let init = args.get(1).cloned().unwrap_or(Val::Num(0.0));
            let items = l.borrow().clone();
            items.into_iter().fold(init, |acc, item| val_call(f.clone(), vec![acc, item]))
        }
        "each" => {
            let f = args.into_iter().next().unwrap_or(Val::Nil);
            for item in l.borrow().clone() { val_call(f.clone(), vec![item]); }
            Val::Nil
        }
        "sort" => {
            l.borrow_mut().sort_by(|a, b| {
                match (a, b) {
                    (Val::Num(x), Val::Num(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                    (Val::Str(x), Val::Str(y)) => x.cmp(y),
                    _ => std::cmp::Ordering::Equal,
                }
            });
            Val::Nil
        }
        "index_of" => {
            let needle = args.first().cloned().unwrap_or(Val::Nil);
            Val::Num(l.borrow().iter().position(|v| {
                matches!((v, &needle), (Val::Num(a), Val::Num(b)) if a == b)
                || matches!((v, &needle), (Val::Str(a), Val::Str(b)) if a == b)
            }).map(|i| i as f64).unwrap_or(-1.0))
        }
        "find" => {
            let f = args.into_iter().next().unwrap_or(Val::Nil);
            let items = l.borrow().clone();
            items.into_iter().find(|item| val_truthy(&val_call(f.clone(), vec![item.clone()]))).unwrap_or(Val::Nil)
        }
        "count" => {
            let f = args.into_iter().next().unwrap_or(Val::Nil);
            let items = l.borrow().clone();
            Val::Num(items.into_iter().filter(|item| val_truthy(&val_call(f.clone(), vec![item.clone()]))).count() as f64)
        }
        _ => Val::Nil,
    }
}

// ── Map methods ───────────────────────────────────────────────────────────────

fn map_method(m: Rc<RefCell<HashMap<String, Val>>>, method: &str, args: Vec<Val>) -> Val {
    match method {
        "get"     => {
            let key = args.first().map(val_display).unwrap_or_default();
            m.borrow().get(&key).cloned().unwrap_or(Val::Nil)
        }
        "set"     => {
            let key = args.first().map(val_display).unwrap_or_default();
            let val = args.get(1).cloned().unwrap_or(Val::Nil);
            m.borrow_mut().insert(key, val); Val::Nil
        }
        "has_key" => {
            let key = args.first().map(val_display).unwrap_or_default();
            Val::Bool(m.borrow().contains_key(&key))
        }
        "keys"    => Val::List(Rc::new(RefCell::new(m.borrow().keys().cloned().map(Val::Str).collect()))),
        "values"  => Val::List(Rc::new(RefCell::new(m.borrow().values().cloned().collect()))),
        "len"     => Val::Num(m.borrow().len() as f64),
        _ => Val::Nil,
    }
}

// ── Built-in functions ────────────────────────────────────────────────────────

fn _register_builtins(env: &Env) {
    // Register all built-in functions into the environment so they can be called
    // by name from user code (e.g. abs(x), sqrt(x), range(0, 10), etc.)
    macro_rules! bi {
        ($name:literal, $f:expr) => {
            env_set(env, $name, Val::Func(std::rc::Rc::new($f)));
        };
    }
    bi!("print",         |args| { if let Some(v) = args.first() { println!("{}", val_display(v)); } Val::Nil });
    bi!("input",         |args| { _builtin_input(args.into_iter().next().unwrap_or(Val::Nil)) });
    bi!("to_number",     |args| { _builtin_to_number(args.into_iter().next().unwrap_or(Val::Nil)) });
    bi!("to_string",     |args| { _builtin_to_string(args.into_iter().next().unwrap_or(Val::Nil)) });
    bi!("type_of",       |args| { _builtin_type_of(args.into_iter().next().unwrap_or(Val::Nil)) });
    bi!("len",           |args| { _builtin_len(args.into_iter().next().unwrap_or(Val::Nil)) });
    bi!("exit",          |args| { let c = if let Some(Val::Num(n)) = args.first() { *n as i32 } else { 0 }; _builtin_exit(c) });
    bi!("sleep",         |args| { if let Some(Val::Num(n)) = args.first() { _builtin_sleep(*n); } Val::Nil });
    bi!("random",        |_| Val::Num(_builtin_random()));
    bi!("time_now",      |_| Val::Num(_builtin_time_now()));
    bi!("file_exists",   |args| { Val::Bool(if let Some(Val::Str(p)) = args.first() { _builtin_file_exists(p) } else { false }) });
    bi!("read_file",     |args| { if let Some(Val::Str(p)) = args.first() { _builtin_read_file(p) } else { Val::Nil } });
    bi!("write_file",    |args| { if let (Some(Val::Str(p)), Some(c)) = (args.first(), args.get(1)) { _builtin_write_file(p, &val_display(c)); } Val::Nil });
    bi!("abs",           |args| { if let Some(Val::Num(n)) = args.first() { Val::Num(_builtin_abs(*n)) } else { Val::Nil } });
    bi!("sqrt",          |args| { if let Some(Val::Num(n)) = args.first() { Val::Num(_builtin_sqrt(*n)) } else { Val::Nil } });
    bi!("floor",         |args| { if let Some(Val::Num(n)) = args.first() { Val::Num(_builtin_floor(*n)) } else { Val::Nil } });
    bi!("ceil",          |args| { if let Some(Val::Num(n)) = args.first() { Val::Num(_builtin_ceil(*n)) } else { Val::Nil } });
    bi!("round",         |args| { if let Some(Val::Num(n)) = args.first() { Val::Num(_builtin_round(*n)) } else { Val::Nil } });
    bi!("pow",           |args| { if let (Some(Val::Num(b)), Some(Val::Num(e))) = (args.first(), args.get(1)) { Val::Num(_builtin_pow(*b, *e)) } else { Val::Nil } });
    bi!("min",           |args| { if let (Some(Val::Num(a)), Some(Val::Num(b))) = (args.first(), args.get(1)) { Val::Num(_builtin_min(*a, *b)) } else { Val::Nil } });
    bi!("max",           |args| { if let (Some(Val::Num(a)), Some(Val::Num(b))) = (args.first(), args.get(1)) { Val::Num(_builtin_max(*a, *b)) } else { Val::Nil } });
    bi!("range",         |args| { 
        let s = if let Some(Val::Num(n)) = args.first() { *n } else { 0.0 };
        let e = if let Some(Val::Num(n)) = args.get(1) { *n } else { 0.0 };
        _builtin_range(s, e, 1.0) 
    });
    bi!("env_get",       |args| { 
        if let Some(Val::Str(k)) = args.first() { 
            std::env::var(k).map(Val::Str).unwrap_or(Val::Nil)
        } else { Val::Nil }
    });
    bi!("assert",        |args| {
        let ok = args.first().map(|v| val_truthy(v)).unwrap_or(false);
        if !ok {
            let msg = args.get(1).map(val_display).unwrap_or_else(|| "assertion failed".into());
            panic!("{}", msg);
        }
        Val::Nil
    });
    bi!("assert_eq",     |args| {
        let a = args.first().cloned().unwrap_or(Val::Nil);
        let b = args.get(1).cloned().unwrap_or(Val::Nil);
        if !val_truthy(&val_eq(a.clone(), b.clone())) {
            panic!("assert_eq failed: {:?} != {:?}", a, b);
        }
        Val::Nil
    });
    bi!("is_number",     |args| { Val::Bool(matches!(args.first(), Some(Val::Num(_)))) });
    bi!("is_string",     |args| { Val::Bool(matches!(args.first(), Some(Val::Str(_)))) });
    bi!("is_list",       |args| { Val::Bool(matches!(args.first(), Some(Val::List(_)))) });
    bi!("is_map",        |args| { Val::Bool(matches!(args.first(), Some(Val::Map(_)))) });
    bi!("format_number", |args| {
        if let (Some(Val::Num(n)), Some(Val::Num(d))) = (args.first(), args.get(1)) {
            Val::Str(format!("{:.prec$}", n, prec = *d as usize))
        } else { Val::Nil }
    });
}

pub fn _builtin_print(v: Val) { println!("{}", val_display(&v)); }
pub fn _builtin_input(prompt: Val) -> Val {
    print!("{}", val_display(&prompt));
    use std::io::Write;
    std::io::stdout().flush().ok();
    let mut s = String::new();
    std::io::stdin().read_line(&mut s).ok();
    Val::Str(s.trim_end_matches('\n').to_string())
}
pub fn _builtin_to_number(v: Val) -> Val {
    match v {
        Val::Num(n) => Val::Num(n),
        Val::Str(s) => Val::Num(s.trim().parse::<f64>().unwrap_or(f64::NAN)),
        Val::Bool(b) => Val::Num(if b { 1.0 } else { 0.0 }),
        _ => Val::Num(0.0),
    }
}
pub fn _builtin_to_string(v: Val) -> Val { Val::Str(val_display(&v)) }
pub fn _builtin_len(v: Val) -> Val {
    match v {
        Val::List(l) => Val::Num(l.borrow().len() as f64),
        Val::Str(s)  => Val::Num(s.chars().count() as f64),
        Val::Map(m)  => Val::Num(m.borrow().len() as f64),
        _ => Val::Num(0.0),
    }
}
pub fn _builtin_range(start: f64, end: f64, step: f64) -> Val {
    let mut vals = Vec::new();
    let mut i = start;
    while if step > 0.0 { i < end } else { i > end } {
        vals.push(Val::Num(i));
        i += step;
    }
    Val::List(Rc::new(RefCell::new(vals)))
}
pub fn _builtin_type_of(v: Val) -> Val {
    Val::Str(match v {
        Val::Nil    => "nil",
        Val::Bool(_)=> "bool",
        Val::Num(_) => "number",
        Val::Str(_) => "string",
        Val::List(_)=> "list",
        Val::Map(_) => "map",
        Val::Func(_)=> "function",
    }.to_string())
}
pub fn _builtin_abs(n: f64) -> f64 { n.abs() }
pub fn _builtin_sqrt(n: f64) -> f64 { n.sqrt() }
pub fn _builtin_floor(n: f64) -> f64 { n.floor() }
pub fn _builtin_ceil(n: f64) -> f64 { n.ceil() }
pub fn _builtin_round(n: f64) -> f64 { n.round() }
pub fn _builtin_min(a: f64, b: f64) -> f64 { a.min(b) }
pub fn _builtin_max(a: f64, b: f64) -> f64 { a.max(b) }
pub fn _builtin_pow(b: f64, e: f64) -> f64 { b.powf(e) }
pub fn _builtin_random() -> f64 {
    // LCG — no external rand dep needed
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let v = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (v >> 33) as f64 / u32::MAX as f64
}
pub fn _builtin_time_now() -> f64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
pub fn _builtin_sleep(secs: f64) {
    std::thread::sleep(std::time::Duration::from_secs_f64(secs));
}
pub fn _builtin_read_file(path: &str) -> Val {
    match std::fs::read_to_string(path) {
        Ok(s)  => Val::Str(s),
        Err(e) => panic!("read_file(\"{}\") failed: {}", path, e),
    }
}
pub fn _builtin_write_file(path: &str, content: &str) {
    std::fs::write(path, content).expect("write_file failed");
}
pub fn _builtin_file_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}
pub fn _builtin_exit(code: i32) -> ! {
    std::process::exit(code)
}
"#;
