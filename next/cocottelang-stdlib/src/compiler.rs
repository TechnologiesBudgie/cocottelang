// compiler.rs — AST → bytecode compiler for Cocotte
// Used by `cocotte build` to produce an optimized bytecode representation
// before native code generation (or standalone for the VM).

use crate::ast::*;
use crate::bytecode::{Chunk, Instruction};
use crate::error::Result;

pub struct Compiler {
    chunk: Chunk,
    /// Stack of loop break-patch positions (to patch after loop)
    break_patches: Vec<Vec<usize>>,
    /// Stack of loop continue-jump targets
    continue_targets: Vec<usize>,
}

impl Compiler {
    pub fn new(name: &str) -> Self {
        Compiler {
            chunk: Chunk::new(name),
            break_patches: Vec::new(),
            continue_targets: Vec::new(),
        }
    }

    pub fn compile_program(mut self, program: &Program) -> Result<Chunk> {
        for stmt in &program.statements {
            self.compile_stmt(stmt)?;
        }
        Ok(self.chunk)
    }

    // ── Statements ────────────────────────────────────────────────────────────

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::VarDecl { name, value, .. } => {
                self.compile_expr(value)?;
                self.emit(Instruction::StoreName(name.clone()));
            }

            Stmt::Assign { target, value, .. } => {
                self.compile_expr(value)?;
                self.compile_assign_target(target)?;
            }

            Stmt::FuncDecl { name, params, body, .. } => {
                let mut func_compiler = Compiler::new(name);
                for stmt in body {
                    func_compiler.compile_stmt(stmt)?;
                }
                // Ensure function returns nil if no explicit return
                func_compiler.emit(Instruction::LoadConst(crate::value::Value::Nil));
                func_compiler.emit(Instruction::Return);
                let code = func_compiler.chunk.instructions;
                self.emit(Instruction::MakeFunc {
                    name: Some(name.clone()),
                    params: params.clone(),
                    code,
                });
                self.emit(Instruction::StoreName(name.clone()));
            }

            Stmt::ClassDecl { name, methods, .. } => {
                let mut compiled_methods = Vec::new();
                for method_stmt in methods {
                    if let Stmt::FuncDecl { name: mname, params, body, .. } = method_stmt {
                        let mut mc = Compiler::new(mname);
                        for s in body {
                            mc.compile_stmt(s)?;
                        }
                        mc.emit(Instruction::LoadConst(crate::value::Value::Nil));
                        mc.emit(Instruction::Return);
                        compiled_methods.push((mname.clone(), params.clone(), mc.chunk.instructions));
                    }
                }
                self.emit(Instruction::MakeClass {
                    name: name.clone(),
                    methods: compiled_methods,
                });
                self.emit(Instruction::StoreName(name.clone()));
            }

            Stmt::Return { value, .. } => {
                match value {
                    Some(expr) => self.compile_expr(expr)?,
                    None => { self.emit(Instruction::LoadConst(crate::value::Value::Nil)); }
                }
                self.emit(Instruction::Return);
            }

            Stmt::If { condition, then_branch, elif_branches, else_branch, .. } => {
                self.compile_expr(condition)?;
                let jump_else = self.emit(Instruction::JumpIfFalse(0));

                for s in then_branch {
                    self.compile_stmt(s)?;
                }

                // Collect end-of-then jump positions
                let mut end_jumps = vec![self.emit(Instruction::Jump(0))];

                // Patch the first jump to here (start of elif/else)
                let after_then = self.chunk.current_pos();
                self.chunk.patch_jump(jump_else, after_then);

                for (elif_cond, elif_body) in elif_branches {
                    self.compile_expr(elif_cond)?;
                    let skip = self.emit(Instruction::JumpIfFalse(0));
                    for s in elif_body {
                        self.compile_stmt(s)?;
                    }
                    end_jumps.push(self.emit(Instruction::Jump(0)));
                    let next = self.chunk.current_pos();
                    self.chunk.patch_jump(skip, next);
                }

                if let Some(else_stmts) = else_branch {
                    for s in else_stmts {
                        self.compile_stmt(s)?;
                    }
                }

                let end = self.chunk.current_pos();
                for j in end_jumps {
                    self.chunk.patch_jump(j, end);
                }
            }

            Stmt::While { condition, body, .. } => {
                let loop_start = self.chunk.current_pos();
                self.continue_targets.push(loop_start);
                self.break_patches.push(Vec::new());

                self.compile_expr(condition)?;
                let exit_jump = self.emit(Instruction::JumpIfFalse(0));

                for s in body {
                    self.compile_stmt(s)?;
                }

                self.emit(Instruction::Jump(loop_start));

                let loop_end = self.chunk.current_pos();
                self.chunk.patch_jump(exit_jump, loop_end);

                // Patch all break jumps
                let breaks = self.break_patches.pop().unwrap_or_default();
                for b in breaks {
                    self.chunk.patch_jump(b, loop_end);
                }
                self.continue_targets.pop();
            }

            Stmt::For { var, iterable, body, .. } => {
                self.compile_expr(iterable)?;
                self.emit(Instruction::GetIter);
                let loop_start = self.chunk.current_pos();
                self.continue_targets.push(loop_start);
                self.break_patches.push(Vec::new());

                let exit_jump = self.emit(Instruction::ForIter(0));
                self.emit(Instruction::StoreIter(var.clone()));

                for s in body {
                    self.compile_stmt(s)?;
                }

                self.emit(Instruction::Jump(loop_start));

                let loop_end = self.chunk.current_pos();
                self.chunk.patch_jump(exit_jump, loop_end);

                let breaks = self.break_patches.pop().unwrap_or_default();
                for b in breaks {
                    self.chunk.patch_jump(b, loop_end);
                }
                self.continue_targets.pop();
            }

            Stmt::Try { body, catch_var, catch_body, .. } => {
                // Compile body into a sub-chunk
                let mut body_compiler = Compiler::new("<try-body>");
                for s in body {
                    body_compiler.compile_stmt(s)?;
                }
                body_compiler.emit(Instruction::LoadConst(crate::value::Value::Nil));
                body_compiler.emit(Instruction::Return);

                // Compile catch into a sub-chunk
                let mut catch_compiler = Compiler::new("<catch-body>");
                for s in catch_body {
                    catch_compiler.compile_stmt(s)?;
                }
                catch_compiler.emit(Instruction::LoadConst(crate::value::Value::Nil));
                catch_compiler.emit(Instruction::Return);

                self.emit(Instruction::TryCatch {
                    body_code: body_compiler.chunk.instructions,
                    catch_var: catch_var.clone(),
                    catch_code: catch_compiler.chunk.instructions,
                });
            }

            Stmt::Print { value, .. } => {
                self.compile_expr(value)?;
                self.emit(Instruction::Print);
            }

            Stmt::ModuleAdd { name, .. } => {
                self.emit(Instruction::LoadModule(name.clone()));
                self.emit(Instruction::StoreName(name.clone()));
            }

            Stmt::LibraryAdd { path, .. } => {
                let lib_name = std::path::Path::new(path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(path)
                    .to_string();
                self.emit(Instruction::LoadLibrary(path.clone()));
                self.emit(Instruction::StoreName(lib_name));
            }

            Stmt::Break { .. } => {
                let idx = self.emit(Instruction::Jump(0));
                if let Some(patches) = self.break_patches.last_mut() {
                    patches.push(idx);
                }
            }

            Stmt::Continue { .. } => {
                let target = self.continue_targets.last().copied().unwrap_or(0);
                self.emit(Instruction::Jump(target));
            }

            Stmt::ExprStmt { expr, .. } => {
                self.compile_expr(expr)?;
                self.emit(Instruction::Pop);
            }
        }
        Ok(())
    }

    fn compile_assign_target(&mut self, target: &AssignTarget) -> Result<()> {
        match target {
            AssignTarget::Ident(name) => {
                self.emit(Instruction::StoreName(name.clone()));
            }
            AssignTarget::Field(obj, field) => {
                self.compile_expr(obj)?;
                self.emit(Instruction::SetField(field.clone()));
            }
            AssignTarget::Index(obj, idx) => {
                self.compile_expr(obj)?;
                self.compile_expr(idx)?;
                self.emit(Instruction::SetIndex);
            }
        }
        Ok(())
    }

    // ── Expressions ───────────────────────────────────────────────────────────

    fn compile_expr(&mut self, expr: &Expr) -> Result<()> {
        match expr {
            Expr::Number(n) => {
                self.emit(Instruction::LoadConst(crate::value::Value::Number(*n)));
            }
            Expr::StringLit(s) => {
                self.emit(Instruction::LoadConst(crate::value::Value::Str(s.clone())));
            }
            Expr::Bool(b) => {
                self.emit(Instruction::LoadConst(crate::value::Value::Bool(*b)));
            }
            Expr::Nil => {
                self.emit(Instruction::LoadConst(crate::value::Value::Nil));
            }
            Expr::SelfRef(_) => {
                self.emit(Instruction::LoadName("self".to_string()));
            }
            Expr::Ident(name) => {
                self.emit(Instruction::LoadName(name.clone()));
            }
            Expr::List(elems) => {
                for e in elems {
                    self.compile_expr(e)?;
                }
                self.emit(Instruction::MakeList(elems.len()));
            }
            Expr::Map(pairs) => {
                for (k, v) in pairs {
                    self.compile_expr(k)?;
                    self.compile_expr(v)?;
                }
                self.emit(Instruction::MakeMap(pairs.len()));
            }
            Expr::BinOp { op, left, right, .. } => {
                match op {
                    BinOp::And => {
                        self.compile_expr(left)?;
                        let skip = self.emit(Instruction::JumpIfFalse(0));
                        self.emit(Instruction::Pop);
                        self.compile_expr(right)?;
                        let end = self.chunk.current_pos();
                        self.chunk.patch_jump(skip, end);
                    }
                    BinOp::Or => {
                        self.compile_expr(left)?;
                        let skip = self.emit(Instruction::JumpIfTrue(0));
                        self.emit(Instruction::Pop);
                        self.compile_expr(right)?;
                        let end = self.chunk.current_pos();
                        self.chunk.patch_jump(skip, end);
                    }
                    _ => {
                        self.compile_expr(left)?;
                        self.compile_expr(right)?;
                        let instr = match op {
                            BinOp::Add => Instruction::Add,
                            BinOp::Sub => Instruction::Sub,
                            BinOp::Mul => Instruction::Mul,
                            BinOp::Div => Instruction::Div,
                            BinOp::Mod => Instruction::Mod,
                            BinOp::Eq => Instruction::Eq,
                            BinOp::NotEq => Instruction::NotEq,
                            BinOp::Lt => Instruction::Lt,
                            BinOp::LtEq => Instruction::LtEq,
                            BinOp::Gt => Instruction::Gt,
                            BinOp::GtEq => Instruction::GtEq,
                            BinOp::And | BinOp::Or => unreachable!(),
                        };
                        self.emit(instr);
                    }
                }
            }
            Expr::UnaryOp { op, operand, .. } => {
                self.compile_expr(operand)?;
                match op {
                    UnaryOp::Not => self.emit(Instruction::Not),
                    UnaryOp::Neg => self.emit(Instruction::Neg),
                };
            }
            Expr::Call { callee, args, .. } => {
                self.compile_expr(callee)?;
                for a in args {
                    self.compile_expr(a)?;
                }
                self.emit(Instruction::Call(args.len()));
            }
            Expr::MethodCall { object, method, args, .. } => {
                self.compile_expr(object)?;
                for a in args {
                    self.compile_expr(a)?;
                }
                self.emit(Instruction::CallMethod {
                    method: method.clone(),
                    arity: args.len(),
                });
            }
            Expr::FieldAccess { object, field, .. } => {
                self.compile_expr(object)?;
                self.emit(Instruction::GetField(field.clone()));
            }
            Expr::Index { object, index, .. } => {
                self.compile_expr(object)?;
                self.compile_expr(index)?;
                self.emit(Instruction::GetIndex);
            }
            Expr::Lambda { params, body, .. } => {
                let mut fc = Compiler::new("<lambda>");
                for s in body {
                    fc.compile_stmt(s)?;
                }
                fc.emit(Instruction::LoadConst(crate::value::Value::Nil));
                fc.emit(Instruction::Return);
                self.emit(Instruction::MakeFunc {
                    name: None,
                    params: params.clone(),
                    code: fc.chunk.instructions,
                });
            }
        }
        Ok(())
    }

    fn emit(&mut self, instr: Instruction) -> usize {
        self.chunk.emit(instr)
    }
}
