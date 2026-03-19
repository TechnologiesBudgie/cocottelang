// bytecode.rs — Bytecode instruction set for the Cocotte compiler
// The AST is compiled to this flat instruction list, then executed by the VM.

use crate::value::Value;

/// A single bytecode instruction
#[derive(Debug, Clone)]
pub enum Instruction {
    // Stack management
    LoadConst(Value),      // Push a constant onto the stack
    LoadName(String),      // Push a variable's value
    StoreName(String),     // Pop stack → store in named variable
    Pop,                   // Discard top of stack
    Dup,                   // Duplicate top of stack

    // Arithmetic / logic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,                   // Unary minus
    Not,                   // Logical not

    // Comparison
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,

    // Control flow
    Jump(usize),           // Unconditional jump to instruction index
    JumpIfFalse(usize),    // Pop + jump if top is falsy
    JumpIfTrue(usize),     // Pop + jump if top is truthy (for `or`)

    // Functions
    MakeFunc {             // Build a function object from bytecode chunk
        name: Option<String>,
        params: Vec<String>,
        code: Vec<Instruction>,
    },
    Call(usize),           // Call with N args (pops N args + callee)
    Return,                // Return top of stack from current frame

    // Collections
    MakeList(usize),       // Pop N items → push list
    MakeMap(usize),        // Pop N key-value pairs → push map

    // Attribute / index access
    GetField(String),      // obj.field — pop obj, push field value
    SetField(String),      // obj.field = val — pops val then obj
    GetIndex,              // obj[idx] — pops idx then obj
    SetIndex,              // obj[idx] = val — pops val, idx, obj

    // Method calls
    CallMethod {
        method: String,
        arity: usize,
    },

    // Modules
    LoadModule(String),    // Load module by name into stack
    LoadLibrary(String),   // Load library by path into stack

    // Iteration
    GetIter,               // Convert top to iterator object
    ForIter(usize),        // Advance iterator; jump if exhausted
    StoreIter(String),     // Pop iterator item → store in var

    // Class
    MakeClass {
        name: String,
        // (method_name, params, bytecode)
        methods: Vec<(String, Vec<String>, Vec<Instruction>)>,
    },

    // Exception handling
    TryCatch {
        body_code: Vec<Instruction>,    // code to attempt
        catch_var: Option<String>,      // variable to bind error message
        catch_code: Vec<Instruction>,   // code to run on error
    },

    // Output
    Print,                 // Print top of stack
}

/// A compiled bytecode chunk (a function or top-level program)
#[derive(Debug, Clone)]
pub struct Chunk {
    pub name: String,
    pub instructions: Vec<Instruction>,
    /// Constant pool (indexed by LoadConst)
    pub constants: Vec<Value>,
}

impl Chunk {
    pub fn new(name: &str) -> Self {
        Chunk {
            name: name.to_string(),
            instructions: Vec::new(),
            constants: Vec::new(),
        }
    }

    pub fn emit(&mut self, instr: Instruction) -> usize {
        self.instructions.push(instr);
        self.instructions.len() - 1
    }

    pub fn patch_jump(&mut self, idx: usize, target: usize) {
        match &mut self.instructions[idx] {
            Instruction::Jump(ref mut t)
            | Instruction::JumpIfFalse(ref mut t)
            | Instruction::JumpIfTrue(ref mut t) => *t = target,
            Instruction::ForIter(ref mut t) => *t = target,
            _ => {}
        }
    }

    pub fn current_pos(&self) -> usize {
        self.instructions.len()
    }

    /// Disassemble for debugging
    pub fn disassemble(&self) -> String {
        let mut out = format!("=== Chunk: {} ===\n", self.name);
        for (i, instr) in self.instructions.iter().enumerate() {
            out.push_str(&format!("  {:04} {:?}\n", i, instr));
        }
        out
    }
}
