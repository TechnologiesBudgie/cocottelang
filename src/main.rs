// main.rs — Cocotte CLI entry point
// The `cocotte` command: init, run, build, add, test, clean, package, exec

mod ast;
mod lexer;
mod parser;
mod error;
mod value;
mod environment;
mod interpreter;
mod builtins;
mod modules;
mod compiler;
mod bytecode;
mod vm;
mod charlotfile;
mod codegen;

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::fs;

use crate::error::{CocotteError, Result};
use crate::interpreter::Interpreter;
use crate::codegen::{BuildOptions, TargetOS, init_project, build_project};
use crate::charlotfile::{parse_charlotfile, exec_task, list_tasks};

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "cocotte",
    version = env!("CARGO_PKG_VERSION"),
    about = "🐦 The Cocotte programming language — beginner-friendly, really.",
    long_about = "Cocotte is an English-like programming language with both interpreted and compiled modes.\nUse `cocotte run` for instant execution, `cocotte build` for optimized native binaries.",
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create a new Cocotte project
    Init {
        /// Project name
        name: String,
    },

    /// Run a Cocotte program (interpreted mode)
    Run {
        /// Source file to run (default: src/main.cot)
        #[arg(default_value = "src/main.cot")]
        file: PathBuf,

        /// Enable debug/verbose output
        #[arg(long, short)]
        debug: bool,

        /// Use bytecode VM instead of tree-walk interpreter
        #[arg(long)]
        bytecode: bool,
    },

    /// Compile a Cocotte project to a native binary
    Build {
        /// Source file (default: src/main.cot)
        #[arg(default_value = "src/main.cot")]
        file: PathBuf,

        /// Target operating systems (windows, linux, macos, bsd)
        #[arg(long, value_name = "OS", num_args = 1..)]
        os: Vec<String>,

        /// Build in release mode (optimized)
        #[arg(long)]
        release: bool,

        /// Include debug symbols
        #[arg(long)]
        symbols: bool,

        /// Verbose build output
        #[arg(long, short)]
        verbose: bool,

        /// Output directory
        #[arg(long, default_value = "dist")]
        out: PathBuf,
    },

    /// Add a module from the registry or a local library
    Add {
        /// Module name or path to .cotlib file
        target: String,
    },

    /// Run tests (files ending in _test.cot)
    Test {
        /// Test file pattern
        #[arg(default_value = "tests")]
        dir: PathBuf,

        #[arg(long, short)]
        verbose: bool,
    },

    /// Remove build artifacts
    Clean,

    /// Package the project into a distributable archive
    Package {
        /// Output format (zip, tar)
        #[arg(long, default_value = "zip")]
        format: String,
    },

    /// Execute a task from the Charlotfile
    Exec {
        /// Task name (or "list" to show all tasks)
        task: String,

        #[arg(long, short)]
        verbose: bool,
    },

    /// Start the interactive REPL
    Repl,

    /// Show disassembled bytecode for a source file
    Disasm {
        file: PathBuf,
    },
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    print_banner();
    let cli = Cli::parse();
    let result = dispatch(cli);
    if let Err(e) = result {
        if e.is_signal() {
            // Shouldn't happen at top level, but handle gracefully
            std::process::exit(0);
        }
        e.report(None);
        std::process::exit(1);
    }
}

fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init { name } => {
            cmd_init(&name)
        }
        Commands::Run { file, debug, bytecode } => {
            cmd_run(&file, debug, bytecode)
        }
        Commands::Build { file, os, release, symbols, verbose, out } => {
            cmd_build(&file, &os, release, symbols, verbose, &out)
        }
        Commands::Add { target } => {
            cmd_add(&target)
        }
        Commands::Test { dir, verbose } => {
            cmd_test(&dir, verbose)
        }
        Commands::Clean => {
            cmd_clean()
        }
        Commands::Package { format } => {
            cmd_package(&format)
        }
        Commands::Exec { task, verbose } => {
            cmd_exec(&task, verbose)
        }
        Commands::Repl => {
            cmd_repl()
        }
        Commands::Disasm { file } => {
            cmd_disasm(&file)
        }
    }
}

// ── Command implementations ───────────────────────────────────────────────────

fn cmd_init(name: &str) -> Result<()> {
    init_project(name)
}

fn cmd_run(file: &Path, debug: bool, use_vm: bool) -> Result<()> {
    let source = read_source(file)?;
    let source_lines: Vec<&str> = source.lines().collect();

    println!("{} {} {}\n",
        "▶".green().bold(),
        "Running".bold(),
        file.display().to_string().cyan()
    );

    // Lex
    let mut lexer = lexer::Lexer::new(&source);
    let tokens = lexer.tokenize().map_err(|e| {
        e.report(Some(&source_lines));
        e
    })?;

    // Parse
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().map_err(|e| {
        e.report(Some(&source_lines));
        e
    })?;

    if use_vm {
        // Compile to bytecode then run in VM
        println!("{} {}", "⚡".bold(), "Bytecode mode".dimmed());
        let compiler = compiler::Compiler::new("<main>");
        let chunk = compiler.compile_program(&program).map_err(|e| {
            e.report(Some(&source_lines));
            e
        })?;

        if debug {
            eprintln!("{}", chunk.disassemble());
        }

        let mut vm = vm::VM::new();
        vm.debug = debug;
        vm.project_root = find_project_root(file);
        vm.run(chunk).map_err(|e| {
            e.report(Some(&source_lines));
            e
        })?;
    } else {
        // Tree-walk interpreter
        let mut interp = Interpreter::new();
        interp.debug = debug;
        interp.project_root = find_project_root(file);
        interp.run(&program).map_err(|e| {
            e.report(Some(&source_lines));
            e
        })?;
    }

    Ok(())
}

fn cmd_build(
    file: &Path,
    os_targets: &[String],
    release: bool,
    symbols: bool,
    verbose: bool,
    out_dir: &Path,
) -> Result<()> {
    let project_name = detect_project_name(file);

    let targets: Vec<TargetOS> = if os_targets.is_empty() {
        vec![TargetOS::Current]
    } else {
        let mut ts = Vec::new();
        for os_str in os_targets {
            match TargetOS::from_str(os_str) {
                Some(t) => ts.push(t),
                None => {
                    eprintln!("{} Unknown OS target '{}'. Use: windows, linux, macos, bsd",
                        "⚠".yellow(), os_str);
                }
            }
        }
        if ts.is_empty() { vec![TargetOS::Current] } else { ts }
    };

    let mut opts = BuildOptions::new(&project_name, file.to_path_buf());
    opts.targets = targets;
    opts.release = release;
    opts.debug_symbols = symbols;
    opts.verbose = verbose;
    opts.output_dir = out_dir.to_path_buf();

    build_project(&opts)
}

fn cmd_add(target: &str) -> Result<()> {
    // Check if it's a path to a .cotlib file
    if target.ends_with(".cotlib") || Path::new(target).exists() {
        let path = Path::new(target);
        if !path.exists() {
            return Err(CocotteError::module_err(&format!(
                "Library file '{}' does not exist", target
            )));
        }

        // Copy to ./libraries/
        let dest_dir = Path::new("libraries");
        fs::create_dir_all(dest_dir)?;
        let dest = dest_dir.join(path.file_name().unwrap_or_default());
        fs::copy(path, &dest)?;

        println!("{} Library '{}' added to ./libraries/", "✓".green().bold(), target.cyan());

        // Update Millet.toml
        update_millet_library(target)?;
    } else {
        // Registry module
        println!("{} Fetching module '{}'…", "⬇".cyan().bold(), target.cyan());

        // Check if it's a built-in module
        let builtin_modules = ["charlotte", "math", "network", "json", "os"];
        if builtin_modules.contains(&target) {
            println!("{} Module '{}' is built-in — no download needed", "✓".green().bold(), target.cyan());
            println!("  Use it with: {}", format!("module add \"{}\"", target).white());
        } else {
            // In a real implementation, download from registry
            // Create a stub .cotmod file
            let dest_dir = Path::new("modules");
            fs::create_dir_all(dest_dir)?;
            let stub = format!(
                "# Module: {}\n# Downloaded from Cocotte registry\n# Replace with real implementation\n\nfunc placeholder()\n    print \"Module {} is not yet implemented\"\nend\n",
                target, target
            );
            let dest = dest_dir.join(format!("{}.cotmod", target));
            fs::write(&dest, stub)?;
            println!("{} Module stub created at {}", "✓".green().bold(), dest.display().to_string().cyan());
            println!("  {}", "Note: Replace with real registry download in production".dimmed());
        }

        // Update Millet.toml
        update_millet_module(target)?;
    }

    Ok(())
}

fn cmd_test(dir: &Path, verbose: bool) -> Result<()> {
    println!("{} {}", "🧪".bold(), "Running tests…".bold());

    let mut total = 0;
    let mut passed = 0;
    let mut failed = 0;

    // Find test files
    let test_files = find_test_files(dir);
    if test_files.is_empty() {
        println!("  {}", "No test files found (expected files ending in _test.cot or in tests/)".dimmed());
        return Ok(());
    }

    for test_file in &test_files {
        if verbose {
            println!("\n  {} {}", "▸".cyan(), test_file.display());
        }

        let source = match fs::read_to_string(test_file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  {} Cannot read {}: {}", "✗".red(), test_file.display(), e);
                failed += 1;
                total += 1;
                continue;
            }
        };

        let result = run_test_file(&source, test_file);
        total += 1;
        match result {
            Ok(count) => {
                passed += count;
                if verbose {
                    println!("    {} {} test(s) passed", "✓".green(), count);
                }
            }
            Err(e) => {
                failed += 1;
                println!("  {} {}", "✗".red(), test_file.display());
                e.report(None);
            }
        }
    }

    println!("\n{} Results: {}/{} passed",
        if failed == 0 { "✓".green().bold() } else { "✗".red().bold() },
        passed.to_string().green(),
        total
    );

    if failed > 0 {
        return Err(CocotteError::build_err(&format!("{} test(s) failed", failed)));
    }
    Ok(())
}

fn run_test_file(source: &str, path: &Path) -> Result<usize> {
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse()?;

    let mut interp = Interpreter::new();
    interp.project_root = find_project_root(path);
    interp.run(&program)?;

    // Count test assertions (simplified: count `assert` calls)
    let count = source.lines()
        .filter(|l| l.trim().starts_with("assert"))
        .count();
    Ok(count.max(1))
}

fn cmd_clean() -> Result<()> {
    let to_remove = ["dist", "build", ".cocotte_cache"];
    for dir in &to_remove {
        let p = Path::new(dir);
        if p.exists() {
            fs::remove_dir_all(p)?;
            println!("  {} Removed {}", "✓".green(), dir);
        }
    }
    // Remove temp build dirs
    let tmp_pattern = std::env::temp_dir();
    println!("{} Clean complete", "✓".green().bold());
    Ok(())
}

fn cmd_package(format: &str) -> Result<()> {
    let project_name = detect_project_name(Path::new("src/main.cot"));
    println!("{} Packaging {} as .{}…", "📦".bold(), project_name.cyan(), format);

    let dist = Path::new("dist");
    if !dist.exists() {
        println!("  {}", "Nothing to package — run `cocotte build` first".yellow());
        return Ok(());
    }

    match format {
        "tar" => {
            let out = format!("{}.tar.gz", project_name);
            let status = std::process::Command::new("tar")
                .args(["-czf", &out, "dist/"])
                .status();
            match status {
                Ok(s) if s.success() => {
                    println!("{} Package created: {}", "✓".green().bold(), out.cyan());
                }
                _ => {
                    println!("{} tar not available — package dist/ manually", "⚠".yellow());
                }
            }
        }
        "zip" | _ => {
            let out = format!("{}.zip", project_name);
            let status = std::process::Command::new("zip")
                .args(["-r", &out, "dist/"])
                .status();
            match status {
                Ok(s) if s.success() => {
                    println!("{} Package created: {}", "✓".green().bold(), out.cyan());
                }
                _ => {
                    // Simple fallback: just tell user
                    println!("{} zip not available.", "⚠".yellow());
                    println!("  Package the dist/ directory manually.");
                }
            }
        }
    }

    Ok(())
}

fn cmd_exec(task: &str, verbose: bool) -> Result<()> {
    let charlotfile_path = Path::new("Charlotfile");
    if !charlotfile_path.exists() {
        return Err(CocotteError::build_err(
            "No Charlotfile found in the current directory.\nRun `cocotte init` to create one."
        ));
    }

    let cf = parse_charlotfile(charlotfile_path)?;

    if task == "list" {
        list_tasks(&cf);
        return Ok(());
    }

    exec_task(&cf, task, verbose)
}

fn cmd_repl() -> Result<()> {
    println!("{}", "🐦 Cocotte REPL — type 'exit' or Ctrl+D to quit\n".cyan().bold());

    let mut interp = Interpreter::new();
    let mut line_num = 0;

    loop {
        line_num += 1;
        let prompt = format!("cocotte({})> ", line_num);

        use std::io::{self, Write, BufRead};
        print!("{}", prompt.green());
        io::stdout().flush().ok();

        let mut line = String::new();
        match io::stdin().lock().read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(_) => break,
        }

        let input = line.trim();
        if input.is_empty() { continue; }
        if input == "exit" || input == "quit" { break; }

        // Multi-line accumulation: keep reading until unmatched `end`
        let full_input = if needs_block(input) {
            let mut acc = input.to_string();
            loop {
                print!("{}", "...  > ".dimmed());
                io::stdout().flush().ok();
                let mut cont = String::new();
                match io::stdin().lock().read_line(&mut cont) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
                acc.push('\n');
                acc.push_str(cont.trim_end());
                if cont.trim() == "end" || cont.trim().starts_with("end") {
                    break;
                }
            }
            acc
        } else {
            input.to_string()
        };

        let result = (|| -> Result<crate::value::Value> {
            let mut lex = lexer::Lexer::new(&full_input);
            let tokens = lex.tokenize()?;
            let mut par = parser::Parser::new(tokens);
            let prog = par.parse()?;
            interp.run(&prog)
        })();

        match result {
            Ok(val) => {
                if !matches!(val, crate::value::Value::Nil) {
                    println!("{} {}", "=>".dimmed(), val.to_repr().cyan());
                }
            }
            Err(e) if e.is_signal() => {}
            Err(e) => {
                e.report(None);
            }
        }
    }

    println!("{}", "\nGoodbye! 🐦".cyan());
    Ok(())
}

fn cmd_disasm(file: &Path) -> Result<()> {
    let source = read_source(file)?;

    let mut lex = lexer::Lexer::new(&source);
    let tokens = lex.tokenize()?;
    let mut par = parser::Parser::new(tokens);
    let prog = par.parse()?;

    let comp = compiler::Compiler::new("<main>");
    let chunk = comp.compile_program(&prog)?;

    println!("{}", chunk.disassemble());
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn read_source(file: &Path) -> Result<String> {
    if !file.exists() {
        return Err(CocotteError::io_err(&format!(
            "File '{}' not found.\n  Make sure you're in the project root, or specify the file path.",
            file.display()
        )));
    }
    Ok(fs::read_to_string(file)?)
}

fn find_project_root(file: &Path) -> PathBuf {
    let abs = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let mut dir = abs.parent().unwrap_or(Path::new(".")).to_path_buf();
    // Walk up until we find Millet.toml or give up
    for _ in 0..5 {
        if dir.join("Millet.toml").exists() {
            return dir;
        }
        if let Some(parent) = dir.parent() {
            dir = parent.to_path_buf();
        } else {
            break;
        }
    }
    file.parent().unwrap_or(Path::new(".")).to_path_buf()
}

fn detect_project_name(file: &Path) -> String {
    // Try Millet.toml first
    let root = find_project_root(file);
    let millet = root.join("Millet.toml");
    if let Ok(content) = fs::read_to_string(millet) {
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("name") {
                if let Some(val) = rest.split('=').nth(1) {
                    let name = val.trim().trim_matches('"').trim_matches('\'');
                    if !name.is_empty() {
                        return name.to_string();
                    }
                }
            }
        }
    }
    // Fall back to directory name
    root.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("cocotte_app")
        .to_string()
}

fn find_test_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if dir.is_file() {
        return vec![dir.to_path_buf()];
    }
    if !dir.exists() {
        return files;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e == "cot").unwrap_or(false) {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.ends_with("_test.cot") || name == "test.cot" {
                    files.push(p);
                }
            }
        }
    }
    files
}

fn needs_block(input: &str) -> bool {
    let kws = ["func ", "class ", "if ", "while ", "for ", "try "];
    kws.iter().any(|kw| input.starts_with(kw))
}

fn update_millet_module(module: &str) -> Result<()> {
    let path = Path::new("Millet.toml");
    if !path.exists() { return Ok(()); }
    let mut content = fs::read_to_string(path)?;
    // Simple append to modules list
    if !content.contains(module) {
        content = content.replace(
            "modules = [",
            &format!("modules = [\"{}\", ", module),
        );
        if !content.contains(module) {
            // Append a new dependencies section
            content.push_str(&format!("\n[dependencies]\nmodules = [\"{}\"]", module));
        }
        fs::write(path, content)?;
    }
    Ok(())
}

fn update_millet_library(lib: &str) -> Result<()> {
    let path = Path::new("Millet.toml");
    if !path.exists() { return Ok(()); }
    let mut content = fs::read_to_string(path)?;
    let lib_file = Path::new(lib).file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(lib);
    if !content.contains(lib_file) {
        content = content.replace(
            "libraries = [",
            &format!("libraries = [\"{}\", ", lib_file),
        );
        fs::write(path, content)?;
    }
    Ok(())
}

fn print_banner() {
    let banner = r#"
  ╔═══════════════════════════════════════╗
  ║    🐦  Cocotte Language Toolchain     ║
  ║    Beginner-friendly • AI-ready       ║
  ╚═══════════════════════════════════════╝
"#;
    // Only print banner for subcommands that warrant it
    let args: Vec<String> = std::env::args().collect();
    let show = args.get(1).map(|a| {
        matches!(a.as_str(), "run" | "build" | "init" | "repl")
    }).unwrap_or(false);

    if show {
        println!("{}", banner.cyan());
    }
}
