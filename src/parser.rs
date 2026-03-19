// parser.rs — Recursive-descent parser for Cocotte
// Converts a flat token stream into a structured AST

use crate::ast::*;
use crate::error::{CocotteError, Result};
use crate::lexer::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        // Filter out most newlines (we only care about them as statement terminators)
        Parser { tokens, pos: 0 }
    }

    // ── Token navigation ──────────────────────────────────────────────────────

    fn peek(&self) -> &Token {
        // Skip newlines when peeking (they only matter between statements)
        let mut i = self.pos;
        while i < self.tokens.len() {
            if self.tokens[i].kind != TokenKind::Newline {
                return &self.tokens[i];
            }
            i += 1;
        }
        &self.tokens[self.tokens.len() - 1]
    }

    fn advance(&mut self) -> &Token {
        // Skip newlines silently (statement-level separation is handled by the grammar)
        while self.pos < self.tokens.len() && self.tokens[self.pos].kind == TokenKind::Newline {
            self.pos += 1;
        }
        let tok = &self.tokens[self.pos];
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn skip_newlines(&mut self) {
        while self.pos < self.tokens.len() && self.tokens[self.pos].kind == TokenKind::Newline {
            self.pos += 1;
        }
    }

    fn check_exact(&self, kind: &TokenKind) -> bool {
        &self.peek().kind == kind
    }

    fn eat(&mut self, kind: &TokenKind) -> Result<&Token> {
        self.skip_newlines();
        let tok = &self.tokens[self.pos];
        if std::mem::discriminant(&tok.kind) == std::mem::discriminant(kind) {
            if self.pos < self.tokens.len() - 1 {
                self.pos += 1;
            }
            Ok(tok)
        } else {
            Err(CocotteError::parser(
                tok.line,
                tok.col,
                &format!("Expected {:?} but got {:?}", kind, tok.kind),
            ))
        }
    }

    fn eat_ident(&mut self) -> Result<String> {
        self.skip_newlines();
        let tok = self.tokens[self.pos].clone();
        match &tok.kind {
            TokenKind::Ident(s) => {
                if self.pos < self.tokens.len() - 1 {
                    self.pos += 1;
                }
                Ok(s.clone())
            }
            _ => Err(CocotteError::parser(
                tok.line,
                tok.col,
                &format!("Expected identifier but got {:?}", tok.kind),
            )),
        }
    }

    fn current_span(&self) -> Span {
        let tok = self.peek();
        Span::new(tok.line, tok.col)
    }

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    // ── Top-level parse ───────────────────────────────────────────────────────

    pub fn parse(&mut self) -> Result<Program> {
        let mut statements = Vec::new();
        self.skip_newlines();
        while !self.is_at_end() {
            statements.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        Ok(Program { statements })
    }

    // ── Statements ────────────────────────────────────────────────────────────

    fn parse_stmt(&mut self) -> Result<Stmt> {
        self.skip_newlines();
        let tok = self.peek().clone();
        match &tok.kind {
            TokenKind::Var => self.parse_var_decl(),
            TokenKind::Func => self.parse_func_decl(),
            TokenKind::Class => self.parse_class_decl(),
            TokenKind::Return => self.parse_return(),
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::Try => self.parse_try(),
            TokenKind::Print => self.parse_print(),
            TokenKind::Module => self.parse_module_add(),
            TokenKind::Library => self.parse_library_add(),
            TokenKind::Break => {
                let span = self.current_span();
                self.advance();
                Ok(Stmt::Break { span })
            }
            TokenKind::Continue => {
                let span = self.current_span();
                self.advance();
                Ok(Stmt::Continue { span })
            }
            _ => self.parse_expr_or_assign(),
        }
    }

    fn parse_var_decl(&mut self) -> Result<Stmt> {
        let span = self.current_span();
        self.eat(&TokenKind::Var)?;
        let name = self.eat_ident()?;
        self.eat(&TokenKind::Assign)?;
        let value = self.parse_expr()?;
        Ok(Stmt::VarDecl { name, value, span })
    }

    fn parse_func_decl(&mut self) -> Result<Stmt> {
        let span = self.current_span();
        self.eat(&TokenKind::Func)?;
        let name = self.eat_ident()?;
        let params = self.parse_param_list()?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(Stmt::FuncDecl { name, params, body, span })
    }

    fn parse_class_decl(&mut self) -> Result<Stmt> {
        let span = self.current_span();
        self.eat(&TokenKind::Class)?;
        let name = self.eat_ident()?;
        self.skip_newlines();
        let mut methods = Vec::new();
        while !self.check_exact(&TokenKind::End) && !self.is_at_end() {
            self.skip_newlines();
            if self.check_exact(&TokenKind::End) { break; }
            methods.push(self.parse_func_decl()?);
            self.skip_newlines();
        }
        self.eat(&TokenKind::End)?;
        Ok(Stmt::ClassDecl { name, methods, span })
    }

    fn parse_return(&mut self) -> Result<Stmt> {
        let span = self.current_span();
        self.eat(&TokenKind::Return)?;
        // Value is optional; if next token is end/elif/else/newline, return nil
        let value = if !self.is_block_terminator() {
            Some(self.parse_expr()?)
        } else {
            None
        };
        Ok(Stmt::Return { value, span })
    }

    fn parse_if(&mut self) -> Result<Stmt> {
        let span = self.current_span();
        self.eat(&TokenKind::If)?;
        let condition = self.parse_expr()?;
        self.skip_newlines();
        let mut then_branch = Vec::new();
        while !self.check_exact(&TokenKind::Elif)
            && !self.check_exact(&TokenKind::Else)
            && !self.check_exact(&TokenKind::End)
            && !self.is_at_end()
        {
            then_branch.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        let mut elif_branches = Vec::new();
        while self.check_exact(&TokenKind::Elif) {
            self.advance();
            let cond = self.parse_expr()?;
            self.skip_newlines();
            let mut elif_body = Vec::new();
            while !self.check_exact(&TokenKind::Elif)
                && !self.check_exact(&TokenKind::Else)
                && !self.check_exact(&TokenKind::End)
                && !self.is_at_end()
            {
                elif_body.push(self.parse_stmt()?);
                self.skip_newlines();
            }
            elif_branches.push((cond, elif_body));
        }
        let else_branch = if self.check_exact(&TokenKind::Else) {
            self.advance();
            self.skip_newlines();
            let mut else_body = Vec::new();
            while !self.check_exact(&TokenKind::End) && !self.is_at_end() {
                else_body.push(self.parse_stmt()?);
                self.skip_newlines();
            }
            Some(else_body)
        } else {
            None
        };
        self.eat(&TokenKind::End)?;
        Ok(Stmt::If { condition, then_branch, elif_branches, else_branch, span })
    }

    fn parse_while(&mut self) -> Result<Stmt> {
        let span = self.current_span();
        self.eat(&TokenKind::While)?;
        let condition = self.parse_expr()?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(Stmt::While { condition, body, span })
    }

    fn parse_for(&mut self) -> Result<Stmt> {
        let span = self.current_span();
        self.eat(&TokenKind::For)?;
        let var = self.eat_ident()?;
        self.eat(&TokenKind::In)?;
        let iterable = self.parse_expr()?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(Stmt::For { var, iterable, body, span })
    }

    fn parse_try(&mut self) -> Result<Stmt> {
        let span = self.current_span();
        self.eat(&TokenKind::Try)?;
        self.skip_newlines();
        let mut body = Vec::new();
        while !self.check_exact(&TokenKind::Catch) && !self.is_at_end() {
            body.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        self.eat(&TokenKind::Catch)?;
        // Optional catch variable name: `catch err` binds the error message to `err`
        let (catch_type, catch_var) = if let TokenKind::Ident(_) = self.peek().kind.clone() {
            let t = self.eat_ident()?;
            (None, Some(t))
        } else {
            (None, None)
        };
        self.skip_newlines();
        let mut catch_body = Vec::new();
        while !self.check_exact(&TokenKind::End) && !self.is_at_end() {
            catch_body.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        self.eat(&TokenKind::End)?;
        Ok(Stmt::Try { body, catch_var, catch_type, catch_body, span })
    }

    fn parse_print(&mut self) -> Result<Stmt> {
        let span = self.current_span();
        self.eat(&TokenKind::Print)?;
        let value = self.parse_expr()?;
        Ok(Stmt::Print { value, span })
    }

    fn parse_module_add(&mut self) -> Result<Stmt> {
        let span = self.current_span();
        self.eat(&TokenKind::Module)?;
        self.eat(&TokenKind::Add)?;
        let tok = self.peek().clone();
        if let TokenKind::StringLit(name) = tok.kind {
            self.advance();
            Ok(Stmt::ModuleAdd { name, span })
        } else {
            Err(CocotteError::parser(tok.line, tok.col, "Expected module name string"))
        }
    }

    fn parse_library_add(&mut self) -> Result<Stmt> {
        let span = self.current_span();
        self.eat(&TokenKind::Library)?;
        self.eat(&TokenKind::Add)?;
        let tok = self.peek().clone();
        if let TokenKind::StringLit(path) = tok.kind {
            self.advance();
            Ok(Stmt::LibraryAdd { path, span })
        } else {
            Err(CocotteError::parser(tok.line, tok.col, "Expected library path string"))
        }
    }

    fn parse_expr_or_assign(&mut self) -> Result<Stmt> {
        let span = self.current_span();
        let expr = self.parse_expr()?;
        // Check for assignment: expr = rhs
        if self.check_exact(&TokenKind::Assign) {
            self.advance();
            let value = self.parse_expr()?;
            let target = expr_to_assign_target(expr).ok_or_else(|| {
                CocotteError::parser(span.line, span.col, "Invalid assignment target")
            })?;
            return Ok(Stmt::Assign { target, value, span });
        }
        Ok(Stmt::ExprStmt { expr, span })
    }

    // Parse a block terminated by `end`
    fn parse_block(&mut self) -> Result<Vec<Stmt>> {
        let mut stmts = Vec::new();
        while !self.check_exact(&TokenKind::End) && !self.is_at_end() {
            // Allow elif/else to terminate a block (handled by parent)
            if self.check_exact(&TokenKind::Elif) || self.check_exact(&TokenKind::Else) {
                break;
            }
            stmts.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        self.eat(&TokenKind::End)?;
        Ok(stmts)
    }

    fn is_block_terminator(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::End
            | TokenKind::Elif
            | TokenKind::Else
            | TokenKind::Catch
            | TokenKind::Eof
            | TokenKind::Newline
        )
    }

    fn parse_param_list(&mut self) -> Result<Vec<String>> {
        self.eat(&TokenKind::LParen)?;
        let mut params = Vec::new();
        while !self.check_exact(&TokenKind::RParen) && !self.is_at_end() {
            params.push(self.eat_ident()?);
            if self.check_exact(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        self.eat(&TokenKind::RParen)?;
        Ok(params)
    }

    // ── Expressions ───────────────────────────────────────────────────────────

    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr> {
        let mut left = self.parse_and()?;
        while self.check_exact(&TokenKind::Or) {
            let span = self.current_span();
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinOp {
                op: BinOp::Or,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr> {
        let mut left = self.parse_equality()?;
        while self.check_exact(&TokenKind::And) {
            let span = self.current_span();
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::BinOp {
                op: BinOp::And,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Eq => BinOp::Eq,
                TokenKind::NotEq => BinOp::NotEq,
                _ => break,
            };
            let span = self.current_span();
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr> {
        let mut left = self.parse_addition()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::LtEq => BinOp::LtEq,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::GtEq => BinOp::GtEq,
                _ => break,
            };
            let span = self.current_span();
            self.advance();
            let right = self.parse_addition()?;
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_addition(&mut self) -> Result<Expr> {
        let mut left = self.parse_multiplication()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            let span = self.current_span();
            self.advance();
            let right = self.parse_multiplication()?;
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_multiplication(&mut self) -> Result<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            let span = self.current_span();
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr> {
        let span = self.current_span();
        match self.peek().kind.clone() {
            TokenKind::Not => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::UnaryOp { op: UnaryOp::Not, operand: Box::new(operand), span })
            }
            TokenKind::Minus => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::UnaryOp { op: UnaryOp::Neg, operand: Box::new(operand), span })
            }
            // Special: `divide X by Y` → X / Y
            TokenKind::Divide => {
                self.advance();
                let left = self.parse_primary()?;
                self.eat(&TokenKind::By)?;
                let right = self.parse_primary()?;
                Ok(Expr::BinOp {
                    op: BinOp::Div,
                    left: Box::new(left),
                    right: Box::new(right),
                    span,
                })
            }
            _ => self.parse_call_access(),
        }
    }

    fn parse_call_access(&mut self) -> Result<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            let span = self.current_span();
            match self.peek().kind.clone() {
                TokenKind::LParen => {
                    // Function call
                    let args = self.parse_arg_list()?;
                    expr = Expr::Call { callee: Box::new(expr), args, span };
                }
                TokenKind::Dot => {
                    self.advance();
                    let field = self.eat_ident()?;
                    // Check if it's a method call
                    if self.check_exact(&TokenKind::LParen) {
                        let args = self.parse_arg_list()?;
                        expr = Expr::MethodCall {
                            object: Box::new(expr),
                            method: field,
                            args,
                            span,
                        };
                    } else {
                        expr = Expr::FieldAccess {
                            object: Box::new(expr),
                            field,
                            span,
                        };
                    }
                }
                TokenKind::LBracket => {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.eat(&TokenKind::RBracket)?;
                    expr = Expr::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
                        span,
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_arg_list(&mut self) -> Result<Vec<Expr>> {
        self.eat(&TokenKind::LParen)?;
        let mut args = Vec::new();
        while !self.check_exact(&TokenKind::RParen) && !self.is_at_end() {
            args.push(self.parse_expr()?);
            if self.check_exact(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        self.eat(&TokenKind::RParen)?;
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::Number(n) => {
                self.advance();
                Ok(Expr::Number(n))
            }
            TokenKind::StringLit(s) => {
                self.advance();
                Ok(Expr::StringLit(s))
            }
            TokenKind::Bool(b) => {
                self.advance();
                Ok(Expr::Bool(b))
            }
            TokenKind::Nil => {
                self.advance();
                Ok(Expr::Nil)
            }
            TokenKind::Self_ => {
                self.advance();
                Ok(Expr::SelfRef(Span::new(tok.line, tok.col)))
            }
            TokenKind::Ident(_) => {
                let name = self.eat_ident()?;
                Ok(Expr::Ident(name))
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.eat(&TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut elems = Vec::new();
                while !self.check_exact(&TokenKind::RBracket) && !self.is_at_end() {
                    elems.push(self.parse_expr()?);
                    if self.check_exact(&TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.eat(&TokenKind::RBracket)?;
                Ok(Expr::List(elems))
            }
            TokenKind::LBrace => {
                self.advance();
                let mut pairs = Vec::new();
                while !self.check_exact(&TokenKind::RBrace) && !self.is_at_end() {
                    let key = self.parse_expr()?;
                    self.eat(&TokenKind::Colon)?;
                    let val = self.parse_expr()?;
                    pairs.push((key, val));
                    if self.check_exact(&TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.eat(&TokenKind::RBrace)?;
                Ok(Expr::Map(pairs))
            }
            TokenKind::Func => {
                let span = Span::new(tok.line, tok.col);
                self.advance();
                let params = self.parse_param_list()?;
                self.skip_newlines();
                let body = self.parse_block()?;
                Ok(Expr::Lambda { params, body, span })
            }
            _ => Err(CocotteError::parser(
                tok.line,
                tok.col,
                &format!(
                    "Unexpected token '{:?}'. Expected an expression (number, string, variable name, etc.)",
                    tok.kind
                ),
            )),
        }
    }
}

/// Convert an expression into an assignment target (if valid)
fn expr_to_assign_target(expr: Expr) -> Option<AssignTarget> {
    match expr {
        Expr::Ident(name) => Some(AssignTarget::Ident(name)),
        Expr::FieldAccess { object, field, .. } => {
            Some(AssignTarget::Field(object, field))
        }
        Expr::Index { object, index, .. } => {
            Some(AssignTarget::Index(object, index))
        }
        _ => None,
    }
}
