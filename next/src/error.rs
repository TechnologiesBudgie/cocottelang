// error.rs — Error types and reporting for Cocotte
// All compiler/interpreter errors flow through here for consistent formatting

use colored::Colorize;
use std::fmt;

pub type Result<T> = std::result::Result<T, CocotteError>;

/// All Cocotte error kinds
#[derive(Debug, Clone)]
pub enum ErrorKind {
    Lexer,
    Parser,
    Runtime,
    Type,
    Module,
    IO,
    Build,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorKind::Lexer   => write!(f, "Lexer Error"),
            ErrorKind::Parser  => write!(f, "Parser Error"),
            ErrorKind::Runtime => write!(f, "Runtime Error"),
            ErrorKind::Type    => write!(f, "Type Error"),
            ErrorKind::Module  => write!(f, "Module Error"),
            ErrorKind::IO      => write!(f, "IO Error"),
            ErrorKind::Build   => write!(f, "Build Error"),
        }
    }
}

/// The main Cocotte error type
#[derive(Debug, Clone)]
pub struct CocotteError {
    pub kind: ErrorKind,
    pub message: String,
    pub line: Option<usize>,
    pub col: Option<usize>,
    pub hint: Option<String>,
    /// Control-flow signals: return value, break, continue
    pub signal: Option<Signal>,
}

/// Control-flow signals that piggyback on the error type for simplicity
#[derive(Debug, Clone)]
pub enum Signal {
    Return(crate::value::Value),
    Break,
    Continue,
}

impl CocotteError {
    pub fn lexer(line: usize, col: usize, msg: &str) -> Self {
        CocotteError { kind: ErrorKind::Lexer, message: msg.to_string(),
            line: Some(line), col: Some(col), hint: None, signal: None }
    }

    pub fn parser(line: usize, col: usize, msg: &str) -> Self {
        CocotteError { kind: ErrorKind::Parser, message: msg.to_string(),
            line: Some(line), col: Some(col), hint: None, signal: None }
    }

    pub fn runtime(msg: &str) -> Self {
        CocotteError { kind: ErrorKind::Runtime, message: msg.to_string(),
            line: None, col: None, hint: None, signal: None }
    }

    pub fn runtime_at(line: usize, col: usize, msg: &str) -> Self {
        CocotteError { kind: ErrorKind::Runtime, message: msg.to_string(),
            line: Some(line), col: Some(col), hint: None, signal: None }
    }

    pub fn type_err(msg: &str) -> Self {
        CocotteError { kind: ErrorKind::Type, message: msg.to_string(),
            line: None, col: None, hint: None, signal: None }
    }

    pub fn module_err(msg: &str) -> Self {
        CocotteError { kind: ErrorKind::Module, message: msg.to_string(),
            line: None, col: None, hint: None, signal: None }
    }

    pub fn io_err(msg: &str) -> Self {
        CocotteError { kind: ErrorKind::IO, message: msg.to_string(),
            line: None, col: None, hint: None, signal: None }
    }

    pub fn build_err(msg: &str) -> Self {
        CocotteError { kind: ErrorKind::Build, message: msg.to_string(),
            line: None, col: None, hint: None, signal: None }
    }

    pub fn with_hint(mut self, hint: &str) -> Self {
        self.hint = Some(hint.to_string());
        self
    }

    pub fn return_signal(val: crate::value::Value) -> Self {
        CocotteError { kind: ErrorKind::Runtime, message: String::new(),
            line: None, col: None, hint: None, signal: Some(Signal::Return(val)) }
    }

    pub fn break_signal() -> Self {
        CocotteError { kind: ErrorKind::Runtime, message: String::new(),
            line: None, col: None, hint: None, signal: Some(Signal::Break) }
    }

    pub fn continue_signal() -> Self {
        CocotteError { kind: ErrorKind::Runtime, message: String::new(),
            line: None, col: None, hint: None, signal: Some(Signal::Continue) }
    }

    pub fn is_signal(&self) -> bool {
        self.signal.is_some()
    }

    /// Print a formatted error to stderr
    pub fn report(&self, source_lines: Option<&[&str]>) {
        let header = format!("error[{}]: {}", self.kind, self.message);
        eprintln!("{}", header.red().bold());

        let loc = match (self.line, self.col) {
            (Some(l), Some(c)) => format!("  --> line {}, column {}", l, c),
            (Some(l), None)    => format!("  --> line {}", l),
            _                  => String::new(),
        };
        if !loc.is_empty() {
            eprintln!("{}", loc.dimmed());
        }

        // Show the offending source line if available
        if let (Some(lines), Some(line)) = (source_lines, self.line) {
            if line > 0 && line <= lines.len() {
                let src = lines[line - 1];
                eprintln!("   |");
                eprintln!("{:>3} | {}", line.to_string().cyan(), src);
                if let Some(col) = self.col {
                    let pad = " ".repeat(col + line.to_string().len() + 4);
                    eprintln!("   | {}^", pad);
                }
            }
        }

        if let Some(hint) = &self.hint {
            eprintln!("   = note: {}", hint.green());
        }
        eprintln!();
    }
}

impl fmt::Display for CocotteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self.line, self.col) {
            (Some(l), Some(c)) => write!(f, "[{}] {} (line {}, col {})", self.kind, self.message, l, c),
            (Some(l), None)    => write!(f, "[{}] {} (line {})", self.kind, self.message, l),
            _                  => write!(f, "[{}] {}", self.kind, self.message),
        }
    }
}

impl std::error::Error for CocotteError {}

impl From<std::io::Error> for CocotteError {
    fn from(e: std::io::Error) -> Self {
        CocotteError::io_err(&e.to_string())
    }
}
