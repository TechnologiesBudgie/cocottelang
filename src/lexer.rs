// lexer.rs — Tokenizer for Cocotte source files
// Converts raw source text into a flat stream of tokens with position info

use crate::error::{CocotteError, Result};

/// All token kinds recognized by the Cocotte lexer
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Number(f64),
    StringLit(String),
    Bool(bool),
    Nil,

    // Identifiers & keywords
    Ident(String),
    Var,
    Func,
    Class,
    Return,
    If,
    Elif,
    Else,
    While,
    For,
    In,
    End,
    Break,
    Continue,
    And,
    Or,
    Not,
    Print,
    Module,
    Library,
    Add,
    Try,
    Catch,
    Self_,
    Divide,  // "divide" keyword for divide-by-zero example syntax
    By,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,       // ==
    NotEq,    // !=
    Lt,
    LtEq,
    Gt,
    GtEq,
    Assign,   // =
    Bang,     // !

    // Delimiters
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Dot,
    Colon,
    Newline,

    // Special
    Eof,
}

/// A single token with position info
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

impl Token {
    fn new(kind: TokenKind, line: usize, col: usize) -> Self {
        Token { kind, line, col }
    }
}

/// The Cocotte lexer: holds source text and current position
pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    /// Tokenize the entire source, returning a Vec of tokens
    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<char> {
        self.source.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.source.get(self.pos).copied();
        if let Some(c) = ch {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        ch
    }

    fn skip_whitespace_inline(&mut self) {
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' || c == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        // Skip single-line comment: # until end of line
        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }
            self.advance();
        }
    }

    fn read_string(&mut self, quote: char) -> Result<String> {
        let line = self.line;
        let col = self.col;
        let mut s = String::new();
        loop {
            match self.advance() {
                None => {
                    return Err(CocotteError::lexer(
                        line, col, "Unterminated string literal",
                    ))
                }
                Some(c) if c == quote => break,
                Some('\\') => {
                    match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('r') => s.push('\r'),
                        Some('\\') => s.push('\\'),
                        Some('"') => s.push('"'),
                        Some('\'') => s.push('\''),
                        Some(c) => {
                            s.push('\\');
                            s.push(c);
                        }
                        None => {
                            return Err(CocotteError::lexer(
                                line, col, "Unterminated escape in string",
                            ))
                        }
                    }
                }
                Some(c) => s.push(c),
            }
        }
        Ok(s)
    }

    fn read_number(&mut self, first: char) -> f64 {
        let mut s = String::new();
        s.push(first);
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        s.parse::<f64>().unwrap_or(0.0)
    }

    fn read_ident(&mut self, first: char) -> String {
        let mut s = String::new();
        s.push(first);
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        s
    }

    fn keyword_or_ident(s: &str) -> TokenKind {
        match s {
            "var" => TokenKind::Var,
            "func" => TokenKind::Func,
            "class" => TokenKind::Class,
            "return" => TokenKind::Return,
            "if" => TokenKind::If,
            "elif" => TokenKind::Elif,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "end" => TokenKind::End,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "print" => TokenKind::Print,
            "module" => TokenKind::Module,
            "library" => TokenKind::Library,
            "add" => TokenKind::Add,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "self" => TokenKind::Self_,
            "divide" => TokenKind::Divide,
            "by" => TokenKind::By,
            "true" => TokenKind::Bool(true),
            "false" => TokenKind::Bool(false),
            "nil" => TokenKind::Nil,
            other => TokenKind::Ident(other.to_string()),
        }
    }

    fn next_token(&mut self) -> Result<Token> {
        // Skip inline whitespace (spaces/tabs)
        self.skip_whitespace_inline();

        let line = self.line;
        let col = self.col;

        match self.advance() {
            None => Ok(Token::new(TokenKind::Eof, line, col)),
            Some('\n') => Ok(Token::new(TokenKind::Newline, line, col)),
            Some('#') => {
                // comment — skip to end of line then recurse
                self.skip_comment();
                self.next_token()
            }
            Some('"') => {
                let s = self.read_string('"')?;
                Ok(Token::new(TokenKind::StringLit(s), line, col))
            }
            Some('\'') => {
                let s = self.read_string('\'')?;
                Ok(Token::new(TokenKind::StringLit(s), line, col))
            }
            Some('+') => Ok(Token::new(TokenKind::Plus, line, col)),
            Some('-') => Ok(Token::new(TokenKind::Minus, line, col)),
            Some('*') => Ok(Token::new(TokenKind::Star, line, col)),
            Some('/') => Ok(Token::new(TokenKind::Slash, line, col)),
            Some('%') => Ok(Token::new(TokenKind::Percent, line, col)),
            Some('(') => Ok(Token::new(TokenKind::LParen, line, col)),
            Some(')') => Ok(Token::new(TokenKind::RParen, line, col)),
            Some('[') => Ok(Token::new(TokenKind::LBracket, line, col)),
            Some(']') => Ok(Token::new(TokenKind::RBracket, line, col)),
            Some('{') => Ok(Token::new(TokenKind::LBrace, line, col)),
            Some('}') => Ok(Token::new(TokenKind::RBrace, line, col)),
            Some(',') => Ok(Token::new(TokenKind::Comma, line, col)),
            Some('.') => Ok(Token::new(TokenKind::Dot, line, col)),
            Some(':') => Ok(Token::new(TokenKind::Colon, line, col)),
            Some('=') => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::new(TokenKind::Eq, line, col))
                } else {
                    Ok(Token::new(TokenKind::Assign, line, col))
                }
            }
            Some('!') => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::new(TokenKind::NotEq, line, col))
                } else {
                    Ok(Token::new(TokenKind::Bang, line, col))
                }
            }
            Some('<') => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::new(TokenKind::LtEq, line, col))
                } else {
                    Ok(Token::new(TokenKind::Lt, line, col))
                }
            }
            Some('>') => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::new(TokenKind::GtEq, line, col))
                } else {
                    Ok(Token::new(TokenKind::Gt, line, col))
                }
            }
            Some(c) if c.is_ascii_digit() => {
                let n = self.read_number(c);
                Ok(Token::new(TokenKind::Number(n), line, col))
            }
            Some(c) if c.is_alphabetic() || c == '_' => {
                let ident = self.read_ident(c);
                let kind = Self::keyword_or_ident(&ident);
                Ok(Token::new(kind, line, col))
            }
            Some(c) => Err(CocotteError::lexer(
                line,
                col,
                &format!("Unexpected character: '{}'", c),
            )),
        }
    }
}
