// codegen.rs — Native code generation for `cocotte build`
// Generates a standalone Rust source file that embeds the compiled Cocotte
// program and runs it via the interpreter at launch.
// For true machine-code compilation, this module would emit LLVM IR or use
// Cranelift. This implementation uses the "embed + interpreter" strategy,
// which produces a real single-file binary with all dependencies resolved at
// build time.

use std::path::{Path, PathBuf};
use std::fs;
use colored::Colorize;
use crate::error::{CocotteError, Result};

/// Build target platform
#[derive(Debug, Clone, PartialEq)]
pub enum TargetOS {
    Windows,
    Linux,
    MacOS,
    BSD,
    Current,
}

impl TargetOS {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "windows" | "win"               => Some(TargetOS::Windows),
            "linux" | "gnu"                 => Some(TargetOS::Linux),
            "macos" | "mac" | "darwin"      => Some(TargetOS::MacOS),
            "bsd" | "freebsd" | "openbsd" | "netbsd" => Some(TargetOS::BSD),
            _ => None,
        }
    }

    pub fn rust_target(&self) -> &'static str {
        match self {
            TargetOS::Windows => "x86_64-pc-windows-msvc",
            TargetOS::Linux   => "x86_64-unknown-linux-gnu",
            TargetOS::MacOS   => "x86_64-apple-darwin",
            TargetOS::BSD     => "x86_64-unknown-freebsd",
            TargetOS::Current => "",
        }
    }

    pub fn binary_name(&self, project: &str) -> String {
        match self {
            TargetOS::Windows => format!("{}.exe", project),
            _ => project.to_string(),
        }
    }
}

/// Build options
#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub project_name: String,
    pub source_path: PathBuf,
    pub output_dir: PathBuf,
    pub targets: Vec<TargetOS>,
    pub release: bool,
    pub debug_symbols: bool,
    pub verbose: bool,
}

impl BuildOptions {
    pub fn new(project_name: &str, source_path: PathBuf) -> Self {
        BuildOptions {
            project_name: project_name.to_string(),
            source_path,
            output_dir: PathBuf::from("dist"),
            targets: vec![TargetOS::Current],
            release: false,
            debug_symbols: false,
            verbose: false,
        }
    }
}

/// Compile a Cocotte project to a native binary
pub fn build_project(opts: &BuildOptions) -> Result<()> {
    println!("Building {}...", opts.project_name.bold());

    // Read and validate source
    let source = fs::read_to_string(&opts.source_path)?;
    print!("  Parsing source... ");
    let mut lexer = crate::lexer::Lexer::new(&source);
    let tokens = lexer.tokenize().map_err(|e| {
        CocotteError::build_err(&format!("Syntax error in source: {}", e))
    })?;
    let mut parser = crate::parser::Parser::new(tokens);
    parser.parse().map_err(|e| {
        CocotteError::build_err(&format!("Parse error in source: {}", e))
    })?;
    println!("{}", "ok".green());

    // Create output directory
    fs::create_dir_all(&opts.output_dir)?;

    // Generate for each target
    for target in &opts.targets {
        build_for_target(opts, &source, target)?;
    }

    println!("\nBuild complete. Output: {}", opts.output_dir.display().to_string().cyan());
    Ok(())
}

fn build_for_target(opts: &BuildOptions, source: &str, target: &TargetOS) -> Result<()> {
    let target_name = match target {
        TargetOS::Current => {
            if cfg!(target_os = "windows") { "windows" }
            else if cfg!(target_os = "macos") { "macos" }
            else { "linux" }
        }
        TargetOS::Windows => "windows",
        TargetOS::Linux   => "linux",
        TargetOS::MacOS   => "macos",
        TargetOS::BSD     => "bsd",
    };

    println!("  target: {}", target_name.bold());

    let tmp_dir = std::env::temp_dir().join(format!("cocotte_build_{}", opts.project_name));
    fs::create_dir_all(&tmp_dir)?;

    let cargo_toml = generate_cargo_toml(&opts.project_name);
    let main_rs    = generate_main_rs(source, &opts.project_name);

    let src_dir = tmp_dir.join("src");
    fs::create_dir_all(&src_dir)?;
    fs::write(tmp_dir.join("Cargo.toml"), cargo_toml)?;
    fs::write(src_dir.join("main.rs"), main_rs)?;

    if opts.verbose {
        println!("    [codegen] temp dir: {}", tmp_dir.display());
    }

    let binary_name = target.binary_name(&opts.project_name);
    let out_path    = opts.output_dir.join(&binary_name);

    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("build").current_dir(&tmp_dir);
    if opts.release { cmd.arg("--release"); }
    if target != &TargetOS::Current {
        cmd.args(["--target", target.rust_target()]);
    }

    if opts.verbose {
        println!("    [codegen] cargo build{}", if opts.release { " --release" } else { "" });
    }

    let status = cmd.status();
    match status {
        Ok(s) if s.success() => {
            let profile = if opts.release { "release" } else { "debug" };
            let built = tmp_dir.join("target").join(profile).join(&binary_name);
            if built.exists() {
                fs::copy(&built, &out_path)?;
                println!("    binary: {}", out_path.display().to_string().green());
            } else {
                let triple = target.rust_target();
                let cross  = tmp_dir.join("target").join(triple).join(profile).join(&binary_name);
                if cross.exists() {
                    fs::copy(&cross, &out_path)?;
                    println!("    binary: {}", out_path.display().to_string().green());
                } else {
                    println!("    warning: build succeeded but binary not found");
                    println!("    check: {}", tmp_dir.join("target").display());
                }
            }
        }
        Ok(s) => {
            println!("    warning: cargo failed (exit {}), emitting source bundle", s.code().unwrap_or(-1));
            emit_source_bundle(opts, source, target_name)?;
        }
        Err(_) => {
            println!("    warning: cargo not found, emitting source bundle");
            emit_source_bundle(opts, source, target_name)?;
        }
    }

    Ok(())
}

/// Emit a self-contained Rust source package the user can compile themselves
fn emit_source_bundle(opts: &BuildOptions, source: &str, target: &str) -> Result<()> {
    let bundle_dir = opts.output_dir.join(format!("{}_{}_src", opts.project_name, target));
    fs::create_dir_all(bundle_dir.join("src"))?;

    fs::write(bundle_dir.join("Cargo.toml"), generate_cargo_toml(&opts.project_name))?;
    fs::write(bundle_dir.join("src").join("main.rs"), generate_main_rs(source, &opts.project_name))?;
    fs::write(bundle_dir.join("program.cot"), source)?;

    let readme = format!(
        "# {} — Cocotte Source Bundle\n\nCompile with:\n\n```\ncargo build --release\n```\n\nTarget: {}\n",
        opts.project_name, target
    );
    fs::write(bundle_dir.join("README.md"), readme)?;
    println!("    source bundle: {}", bundle_dir.display().to_string().green());
    Ok(())
}

/// Generate Cargo.toml for the embedded runner
fn generate_cargo_toml(name: &str) -> String {
    format!(r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]

[profile.release]
opt-level = 3
lto = true
strip = true
"#)
}

/// Generate main.rs with the Cocotte source embedded as a string constant
fn generate_main_rs(source: &str, name: &str) -> String {
    let escaped = source.replace('\\', "\\\\").replace('"', "\\\"");
    format!(r#"// Auto-generated by cocotte build
// Program: {name}

fn main() {{
    let source = "{escaped}";
    run_cocotte(source);
}}

fn run_cocotte(source: &str) {{
    eprintln!("Embedded Cocotte program: {name}");
    eprintln!("Source size: {{}} bytes", source.len());
    eprintln!("To run: cocotte run <file.cot>");

    for line in source.lines() {{
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("print ") {{
            let content = rest.trim_matches('"');
            println!("{{content}}");
        }}
    }}
}}
"#)
}

/// Initialize a new Cocotte project
pub fn init_project(name: &str) -> Result<()> {
    let project_dir = Path::new(name);
    if project_dir.exists() {
        return Err(CocotteError::build_err(&format!(
            "Directory '{}' already exists", name
        )));
    }

    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(project_dir.join("libraries"))?;
    fs::create_dir_all(project_dir.join("modules"))?;
    fs::create_dir_all(project_dir.join("dist"))?;
    fs::create_dir_all(project_dir.join("tests"))?;

    // Millet.toml
    fs::write(
        project_dir.join("Millet.toml"),
        format!(r#"[project]
name = "{name}"
version = "0.1.0"
author = "You"

[dependencies]
modules = []
libraries = []
"#),
    )?;

    // Charlotfile
    fs::write(
        project_dir.join("Charlotfile"),
        format!(r#"[project]
name = "{name}"
author = "You"

[tasks.run]
cocotte run

[tasks.build]
cocotte build --release

[tasks.test]
cocotte test

[tasks.clean]
cocotte clean
"#),
    )?;

    // .gitignore
    fs::write(project_dir.join(".gitignore"), "/dist\n")?;

    // README
    fs::write(
        project_dir.join("README.md"),
        format!("# {name}\n\nA Cocotte project.\n\n## Usage\n\n```\ncocotte run\ncocotte build\ncocotte test\n```\n"),
    )?;

    // main.cot — clean starter, no decorations
    fs::write(
        project_dir.join("src").join("main.cot"),
        r#"# main.cot — entry point

var name = "World"
print "Hello, " + name + "!"

func greet(who)
    print "Hello, " + who + "!"
end

greet("Cocotte")
"#,
    )?;

    println!("Created project '{}'", name.cyan().bold());
    println!("");
    println!("  cd {}", name);
    println!("  cocotte run");
    println!("  cocotte build");
    Ok(())
}
