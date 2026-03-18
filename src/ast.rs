// ast.rs — Abstract Syntax Tree node definitions for Cocotte
// Each variant represents a parsed language construct

/// Source location info for error reporting
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

impl Span {
    pub fn new(line: usize, col: usize) -> Self {
        Span { line, col }
    }
}

impl Default for Span {
    fn default() -> Self {
        Span { line: 1, col: 1 }
    }
}

/// Top-level program: a list of statements
#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

/// All statement types in Cocotte
#[derive(Debug, Clone)]
pub enum Stmt {
    /// var x = expr
    VarDecl {
        name: String,
        value: Expr,
        span: Span,
    },
    /// Assignment: x = expr  OR  obj.field = expr  OR  arr[idx] = expr
    Assign {
        target: AssignTarget,
        value: Expr,
        span: Span,
    },
    /// func name(params) ... end
    FuncDecl {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
        span: Span,
    },
    /// class Name ... end
    ClassDecl {
        name: String,
        methods: Vec<Stmt>,
        span: Span,
    },
    /// return expr
    Return {
        value: Option<Expr>,
        span: Span,
    },
    /// if cond ... elif ... else ... end
    If {
        condition: Expr,
        then_branch: Vec<Stmt>,
        elif_branches: Vec<(Expr, Vec<Stmt>)>,
        else_branch: Option<Vec<Stmt>>,
        span: Span,
    },
    /// while cond ... end
    While {
        condition: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    /// for item in iterable ... end
    For {
        var: String,
        iterable: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    /// try ... catch ErrType ... end
    Try {
        body: Vec<Stmt>,
        catch_var: Option<String>,
        catch_type: Option<String>,
        catch_body: Vec<Stmt>,
        span: Span,
    },
    /// print expr
    Print {
        value: Expr,
        span: Span,
    },
    /// module add "name"
    ModuleAdd {
        name: String,
        span: Span,
    },
    /// library add "path"
    LibraryAdd {
        path: String,
        span: Span,
    },
    /// break / continue
    Break { span: Span },
    Continue { span: Span },
    /// Standalone expression statement (e.g. function call)
    ExprStmt {
        expr: Expr,
        span: Span,
    },
}

/// Targets that can be assigned to
#[derive(Debug, Clone)]
pub enum AssignTarget {
    Ident(String),
    Field(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
}

/// All expression types in Cocotte
#[derive(Debug, Clone)]
pub enum Expr {
    /// Literal values
    Number(f64),
    StringLit(String),
    Bool(bool),
    Nil,
    /// Variable reference
    Ident(String),
    /// [elem, elem, ...]
    List(Vec<Expr>),
    /// { "key": val, ... }
    Map(Vec<(Expr, Expr)>),
    /// Binary operation: a + b, a == b, etc.
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    /// Unary operation: not x, -x
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },
    /// Function call: callee(args)
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    /// Method call: obj.method(args)
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
        span: Span,
    },
    /// Field access: obj.field
    FieldAccess {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    /// Index access: arr[idx]
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    /// Anonymous function: func(params) ... end
    Lambda {
        params: Vec<String>,
        body: Vec<Stmt>,
        span: Span,
    },
    /// self reference inside a class method
    SelfRef(Span),
}

/// Binary operators
#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
}

impl std::fmt::Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Div => write!(f, "/"),
            BinOp::Mod => write!(f, "%"),
            BinOp::Eq => write!(f, "=="),
            BinOp::NotEq => write!(f, "!="),
            BinOp::Lt => write!(f, "<"),
            BinOp::LtEq => write!(f, "<="),
            BinOp::Gt => write!(f, ">"),
            BinOp::GtEq => write!(f, ">="),
            BinOp::And => write!(f, "and"),
            BinOp::Or => write!(f, "or"),
        }
    }
}

/// Unary operators
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Not,
    Neg,
}
