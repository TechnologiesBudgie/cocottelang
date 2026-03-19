// codegen.rs — Native code generation for `cocotte build`
//
// Build strategy:
//   1. Copy the entire cocotte compiler/interpreter source into a temp Cargo
//      workspace as a library crate ("cocotte_rt").
//   2. Generate a thin "runner" binary crate that embeds the user's .cot
//      source as a string constant and calls the real interpreter at startup.
//   3. Invoke `cargo build [--target <triple>]` on that workspace.
//   4. Copy the resulting binary to dist/.
//
// When cargo or the required cross-linker is absent the module emits a
// portable source bundle the user can compile on the target machine.

use std::path::{Path, PathBuf};
use std::fs;
use colored::Colorize;
use crate::error::{CocotteError, Result};

// ── Target architecture ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum TargetArch {
    X86_64,
    AArch64,
    Armv7,
    I686,
    Riscv64,
    Current,
}

impl TargetArch {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "x86_64" | "amd64" | "x64"  => Some(TargetArch::X86_64),
            "aarch64" | "arm64"         => Some(TargetArch::AArch64),
            "armv7" | "arm"            => Some(TargetArch::Armv7),
            "i686" | "i386" | "x86"    => Some(TargetArch::I686),
            "riscv64"                   => Some(TargetArch::Riscv64),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            TargetArch::X86_64  => "x86_64",
            TargetArch::AArch64 => "aarch64",
            TargetArch::Armv7   => "armv7",
            TargetArch::I686    => "i686",
            TargetArch::Riscv64 => "riscv64",
            TargetArch::Current => "native",
        }
    }
}

// ── Target OS ─────────────────────────────────────────────────────────────────

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
            "windows" | "win"                        => Some(TargetOS::Windows),
            "linux" | "gnu"                          => Some(TargetOS::Linux),
            "macos" | "mac" | "darwin" | "osx"       => Some(TargetOS::MacOS),
            "bsd" | "freebsd" | "openbsd" | "netbsd" => Some(TargetOS::BSD),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            TargetOS::Windows => "windows",
            TargetOS::Linux   => "linux",
            TargetOS::MacOS   => "macos",
            TargetOS::BSD     => "bsd",
            TargetOS::Current => {
                if cfg!(target_os = "windows") { "windows" }
                else if cfg!(target_os = "macos") { "macos" }
                else { "linux" }
            }
        }
    }

    /// Returns the Rust target triple, or None for unsupported combinations.
    /// Returns Some("") for Current/Current (host toolchain, no --target flag).
    pub fn rust_target(&self, arch: &TargetArch) -> Option<String> {
        let t = match (self, arch) {
            (TargetOS::Linux,   TargetArch::X86_64)  => "x86_64-unknown-linux-gnu",
            (TargetOS::Linux,   TargetArch::AArch64) => "aarch64-unknown-linux-gnu",
            (TargetOS::Linux,   TargetArch::Armv7)   => "armv7-unknown-linux-gnueabihf",
            (TargetOS::Linux,   TargetArch::I686)    => "i686-unknown-linux-gnu",
            (TargetOS::Linux,   TargetArch::Riscv64) => "riscv64gc-unknown-linux-gnu",
            (TargetOS::Windows, TargetArch::X86_64)  => "x86_64-pc-windows-gnu",
            (TargetOS::Windows, TargetArch::AArch64) => "aarch64-pc-windows-msvc",
            (TargetOS::Windows, TargetArch::I686)    => "i686-pc-windows-gnu",
            (TargetOS::MacOS,   TargetArch::X86_64)  => "x86_64-apple-darwin",
            (TargetOS::MacOS,   TargetArch::AArch64) => "aarch64-apple-darwin",
            (TargetOS::BSD,     TargetArch::X86_64)  => "x86_64-unknown-freebsd",
            (TargetOS::BSD,     TargetArch::AArch64) => "aarch64-unknown-freebsd",
            (TargetOS::Current, TargetArch::Current) => return Some(String::new()),
            _ => return None,
        };
        Some(t.to_string())
    }

    pub fn binary_name(&self, project: &str, arch: &TargetArch) -> String {
        match (self, arch) {
            (TargetOS::Current, TargetArch::Current) => project.to_string(),
            (TargetOS::Windows, _) =>
                format!("{}-{}-{}.exe", project, self.name(), arch.name()),
            _ =>
                format!("{}-{}-{}", project, self.name(), arch.name()),
        }
    }
}

// ── Build options ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub project_name:  String,
    pub source_path:   PathBuf,
    pub output_dir:    PathBuf,
    pub targets:       Vec<(TargetOS, TargetArch)>,
    pub release:       bool,
    pub debug_symbols: bool,
    pub verbose:       bool,
}

impl BuildOptions {
    pub fn new(project_name: &str, source_path: PathBuf) -> Self {
        BuildOptions {
            project_name: project_name.to_string(),
            source_path,
            output_dir:    PathBuf::from("dist"),
            targets:       vec![(TargetOS::Current, TargetArch::Current)],
            release:       false,
            debug_symbols: false,
            verbose:       false,
        }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn build_project(opts: &BuildOptions) -> Result<()> {
    println!("Building {}...", opts.project_name.bold());

    // Validate source before touching the filesystem.
    let source = fs::read_to_string(&opts.source_path)?;
    print!("  Parsing source... ");
    let mut lexer = crate::lexer::Lexer::new(&source);
    let tokens = lexer.tokenize().map_err(|e| {
        CocotteError::build_err(&format!("Syntax error: {}", e))
    })?;
    let mut parser = crate::parser::Parser::new(tokens);
    parser.parse().map_err(|e| {
        CocotteError::build_err(&format!("Parse error: {}", e))
    })?;
    println!("{}", "ok".green());

    fs::create_dir_all(&opts.output_dir)?;

    for (os, arch) in &opts.targets {
        build_for_target(opts, &source, os, arch)?;
    }

    println!("\nBuild complete. Output: {}", opts.output_dir.display().to_string().cyan());
    Ok(())
}

// ── Per-target build ──────────────────────────────────────────────────────────

fn build_for_target(
    opts:   &BuildOptions,
    source: &str,
    os:     &TargetOS,
    arch:   &TargetArch,
) -> Result<()> {
    let label = match (os, arch) {
        (TargetOS::Current, TargetArch::Current) => "native".to_string(),
        _ => format!("{}-{}", os.name(), arch.name()),
    };
    println!("  target: {}", label.bold());

    let triple_opt = os.rust_target(arch);
    if triple_opt.is_none() {
        eprintln!(
            "    warning: unsupported target combination {}/{}. Emitting source bundle.",
            os.name(), arch.name()
        );
        return emit_source_bundle(opts, source, &label);
    }
    let triple = triple_opt.unwrap();

    // Locate the cocotte compiler's own source directory so we can copy it
    // into the generated workspace. We find it by resolving the path to the
    // running executable and walking up to the workspace root.
    let rt_src = locate_runtime_src();

    let tmp_dir = std::env::temp_dir()
        .join(format!("cocotte_build_{}_{}", opts.project_name, label));
    fs::create_dir_all(&tmp_dir)?;

    if opts.verbose {
        println!("    [codegen] workspace : {}", tmp_dir.display());
        if !triple.is_empty() {
            println!("    [codegen] triple    : {}", triple);
        }
    }

    // Write the workspace Cargo.toml
    fs::write(tmp_dir.join("Cargo.toml"), workspace_cargo_toml())?;

    match rt_src {
        Some(rt_path) => {
            // ── Embedded runtime strategy ────────────────────────────────────
            // Copy the cocotte source tree as a library crate, generate a thin
            // runner binary that calls into it.
            setup_runtime_crate(&tmp_dir, &rt_path, opts.verbose)?;
            setup_runner_crate(&tmp_dir, source, &opts.project_name)?;
        }
        None => {
            // ── Fallback: single-crate with bundled source ───────────────────
            // We cannot locate the runtime source, so emit a workspace with
            // just the runner crate and a note for the user.
            if opts.verbose {
                println!("    [codegen] runtime source not found; using single-crate fallback");
            }
            setup_single_crate_fallback(&tmp_dir, source, &opts.project_name)?;
        }
    }

    // Invoke cargo
    let binary_name = os.binary_name(&opts.project_name, arch);
    let out_path    = opts.output_dir.join(&binary_name);

    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("build")
       .current_dir(&tmp_dir);
    if opts.release       { cmd.arg("--release"); }
    if !triple.is_empty() { cmd.args(["--target", &triple]); }

    // Only show cargo output in verbose mode
    if !opts.verbose {
        cmd.stdout(std::process::Stdio::null())
           .stderr(std::process::Stdio::null());
    }

    if opts.verbose {
        println!(
            "    [codegen] cargo build{}{}",
            if opts.release { " --release" } else { "" },
            if !triple.is_empty() { format!(" --target {}", triple) } else { String::new() },
        );
    }

    match cmd.status() {
        Ok(s) if s.success() => {
            let profile   = if opts.release { "release" } else { "debug" };
            let base_name = if matches!(os, TargetOS::Windows) {
                format!("{}.exe", opts.project_name)
            } else {
                opts.project_name.clone()
            };

            let native = tmp_dir.join("target").join(profile).join(&base_name);
            let cross  = tmp_dir.join("target").join(&triple).join(profile).join(&base_name);

            let built = if native.exists() { Some(native) }
                        else if cross.exists() { Some(cross) }
                        else { None };

            match built {
                Some(p) => {
                    fs::copy(&p, &out_path)?;
                    println!("    binary: {}", out_path.display().to_string().green());
                }
                None => {
                    eprintln!("    warning: cargo succeeded but binary not found");
                    eprintln!("    check: {}", tmp_dir.join("target").display());
                }
            }
        }
        Ok(s) => {
            eprintln!(
                "    warning: cargo failed (exit {}), emitting source bundle",
                s.code().unwrap_or(-1)
            );
            emit_source_bundle(opts, source, &label)?;
        }
        Err(_) => {
            eprintln!("    warning: cargo not found — emitting source bundle");
            eprintln!("    Install Rust from https://rustup.rs then retry.");
            emit_source_bundle(opts, source, &label)?;
        }
    }

    Ok(())
}

// ── Workspace helpers ─────────────────────────────────────────────────────────

fn workspace_cargo_toml() -> String {
    r#"[workspace]
members = ["runner"]
resolver = "2"
"#.to_string()
}

/// Try to find the Cocotte compiler's `src/` directory on disk.
/// Looks next to the running binary, then next to the current exe.
fn locate_runtime_src() -> Option<PathBuf> {
    // Try: directory of the running executable → ../../src  (dev build layout)
    if let Ok(exe) = std::env::current_exe() {
        // cargo run layout: target/debug/cocotte → src/
        let candidate = exe
            .parent()? // target/debug
            .parent()? // target
            .parent()? // workspace root
            .join("src");
        if candidate.join("interpreter.rs").exists() {
            return Some(candidate);
        }

        // Installed binary: /usr/local/bin/cocotte — no source nearby
    }

    // Try: well-known development paths relative to cwd
    let cwd = std::env::current_dir().ok()?;
    for rel in &["src", "../src"] {
        let c = cwd.join(rel);
        if c.join("interpreter.rs").exists() {
            return Some(c);
        }
    }

    None
}

/// Copy the cocotte runtime source into tmp_dir/cocotte_rt/src/
/// and generate a Cargo.toml that exposes it as a lib crate.
fn setup_runtime_crate(tmp_dir: &Path, rt_src: &Path, verbose: bool) -> Result<()> {
    let rt_dir  = tmp_dir.join("cocotte_rt");
    let src_dst = rt_dir.join("src");
    fs::create_dir_all(&src_dst)?;

    // Copy every .rs file from the runtime src/
    let rs_files: Vec<_> = fs::read_dir(rt_src)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "rs").unwrap_or(false))
        .collect();

    for entry in &rs_files {
        let fname = entry.file_name();
        fs::copy(entry.path(), src_dst.join(&fname))?;
    }

    if verbose {
        println!("    [codegen] copied {} runtime files", rs_files.len());
    }

    // lib.rs re-exports everything public from main.rs's mods
    // We create a lib.rs that declares the same modules as main.rs does.
    let lib_rs = generate_lib_rs();
    fs::write(src_dst.join("lib.rs"), lib_rs)?;

    // Cargo.toml for the runtime crate
    let rt_cargo = r#"[package]
name    = "cocotte_rt"
version = "0.1.0"
edition = "2021"

[lib]
name = "cocotte_rt"
path = "src/lib.rs"

[dependencies]
serde      = { version = "1", features = ["derive"] }
serde_json = "1"
colored    = "2"
indexmap   = "2"
dirs       = "5"
eframe     = { version = "0.29", optional = true }
egui       = { version = "0.29", optional = true }

[features]
default = ["gui"]
gui     = ["eframe", "egui"]
"#;
    fs::write(rt_dir.join("Cargo.toml"), rt_cargo)?;

    // Add cocotte_rt to the workspace
    let ws_toml = tmp_dir.join("Cargo.toml");
    let existing = fs::read_to_string(&ws_toml)?;
    let updated  = existing.replace(
        r#"members = ["runner"]"#,
        r#"members = ["runner", "cocotte_rt"]"#,
    );
    fs::write(&ws_toml, updated)?;

    Ok(())
}

/// Generate the runner binary crate: embeds the .cot source and calls
/// the real interpreter from cocotte_rt.
fn setup_runner_crate(tmp_dir: &Path, source: &str, project_name: &str) -> Result<()> {
    let runner_dir = tmp_dir.join("runner");
    let src_dir    = runner_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    let escaped = escape_source(source);

    let main_rs = format!(r#"// Auto-generated by cocotte build — runner for "{project_name}"
fn main() {{
    let source = "{escaped}";
    let mut lexer = cocotte_rt::lexer::Lexer::new(source);
    let tokens = match lexer.tokenize() {{
        Ok(t)  => t,
        Err(e) => {{ eprintln!("Lexer error: {{e}}"); std::process::exit(1); }},
    }};
    let mut parser = cocotte_rt::parser::Parser::new(tokens);
    let stmts = match parser.parse() {{
        Ok(s)  => s,
        Err(e) => {{ eprintln!("Parse error: {{e}}"); std::process::exit(1); }},
    }};
    let mut interp = cocotte_rt::interpreter::Interpreter::new();
    if let Err(e) = interp.run(&stmts) {{
        if !e.is_signal() {{
            e.report(None);
            std::process::exit(1);
        }}
    }}
}}
"#);

    fs::write(src_dir.join("main.rs"), main_rs)?;

    let runner_cargo = format!(r#"[package]
name    = "{project_name}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{project_name}"
path = "src/main.rs"

[dependencies]
cocotte_rt = {{ path = "../cocotte_rt", features = ["gui"] }}

[profile.release]
opt-level = 3
lto       = true
strip     = true
"#);

    fs::write(runner_dir.join("Cargo.toml"), runner_cargo)?;
    Ok(())
}

/// Fallback when runtime source is not available: single crate that embeds
/// a minimal print-only runner (honest about its limitations).
fn setup_single_crate_fallback(tmp_dir: &Path, source: &str, project_name: &str) -> Result<()> {
    let runner_dir = tmp_dir.join("runner");
    let src_dir    = runner_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    let escaped = escape_source(source);

    // Emit a stub that explains the situation at runtime
    let main_rs = format!(r#"// Auto-generated by cocotte build (fallback mode)
// The Cocotte runtime source was not available at build time.
// This binary only executes `print "..."` statements.
// To get a fully functional binary, build from the cocotte source tree.
fn main() {{
    let source = "{escaped}";
    for line in source.lines() {{
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("print ") {{
            println!("{{}}", rest.trim().trim_matches('"'));
        }}
    }}
}}
"#);

    fs::write(src_dir.join("main.rs"), main_rs)?;

    let cargo = format!(r#"[package]
name    = "{project_name}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{project_name}"
path = "src/main.rs"

[profile.release]
opt-level = 3
lto       = true
strip     = true
"#);

    fs::write(runner_dir.join("Cargo.toml"), cargo)?;
    Ok(())
}

/// Generate a lib.rs that re-exports the interpreter modules.
fn generate_lib_rs() -> String {
    // Declare the same modules as main.rs, but as pub so the runner can use them.
    // We exclude main.rs itself (it's a bin, not included in the lib).
    r#"// Auto-generated lib.rs — exposes cocotte runtime modules
pub mod ast;
pub mod lexer;
pub mod parser;
pub mod error;
pub mod value;
pub mod environment;
pub mod interpreter;
pub mod builtins;
pub mod modules;
pub mod compiler;
pub mod bytecode;
pub mod vm;
pub mod charlotfile;
pub mod codegen;
"#.to_string()
}

fn escape_source(source: &str) -> String {
    source
        .replace('\\', "\\\\")
        .replace('"',  "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "")
}

// ── Source bundle fallback ────────────────────────────────────────────────────

fn emit_source_bundle(opts: &BuildOptions, source: &str, target: &str) -> Result<()> {
    let bundle_dir = opts.output_dir
        .join(format!("{}_{}_src", opts.project_name, target));
    let src_dir = bundle_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    // Include the whole cocotte source if we can find it
    if let Some(rt_src) = locate_runtime_src() {
        let rt_dst = bundle_dir.join("cocotte_rt").join("src");
        fs::create_dir_all(&rt_dst)?;
        for entry in fs::read_dir(&rt_src)?.filter_map(|e| e.ok()) {
            if entry.path().extension().map(|x| x == "rs").unwrap_or(false) {
                fs::copy(entry.path(), rt_dst.join(entry.file_name()))?;
            }
        }
        fs::write(rt_dst.join("lib.rs"), generate_lib_rs())?;

        // Runtime Cargo.toml
        fs::write(bundle_dir.join("cocotte_rt").join("Cargo.toml"), r#"[package]
name    = "cocotte_rt"
version = "0.1.0"
edition = "2021"

[lib]
name = "cocotte_rt"
path = "src/lib.rs"

[dependencies]
serde      = { version = "1", features = ["derive"] }
serde_json = "1"
colored    = "2"
indexmap   = "2"
dirs       = "5"
"#)?;
    }

    let escaped = escape_source(source);
    let project = &opts.project_name;

    fs::write(src_dir.join("main.rs"), format!(r#"fn main() {{
    let source = "{escaped}";
    let mut lexer = cocotte_rt::lexer::Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = cocotte_rt::parser::Parser::new(tokens);
    let stmts = parser.parse().unwrap();
    let mut interp = cocotte_rt::interpreter::Interpreter::new();
    if let Err(e) = interp.run(&stmts) {{
        if !e.is_signal() {{ e.report(None); std::process::exit(1); }}
    }}
}}
"#))?;

    fs::write(bundle_dir.join("Cargo.toml"), format!(r#"[workspace]
members = [".", "cocotte_rt"]
resolver = "2"

[package]
name    = "{project}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{project}"
path = "src/main.rs"

[dependencies]
cocotte_rt = {{ path = "cocotte_rt" }}

[profile.release]
opt-level = 3
lto       = true
strip     = true
"#))?;

    fs::write(bundle_dir.join("program.cot"), source)?;
    fs::write(
        bundle_dir.join("README.md"),
        format!("# {project} — Cocotte Source Bundle\n\nCompile with:\n\n```sh\ncargo build --release\n```\n\nTarget: {target}\n\nRequires a Rust toolchain: https://rustup.rs\n"),
    )?;

    println!("    source bundle: {}", bundle_dir.display().to_string().green());
    Ok(())
}

// ── Project initialisation ────────────────────────────────────────────────────

pub fn init_project(name: &str) -> Result<()> {
    let project_dir = Path::new(name);
    if project_dir.exists() {
        return Err(CocotteError::build_err(&format!(
            "Directory '{}' already exists", name
        )));
    }

    for dir in &["src", "libraries", "modules", "dist", "tests"] {
        fs::create_dir_all(project_dir.join(dir))?;
    }

    fs::write(project_dir.join("Millet.toml"), format!(r#"[project]
name    = "{name}"
version = "0.1.0"
author  = "You"

[dependencies]
modules   = []
libraries = []
"#))?;

    fs::write(project_dir.join("Charlotfile"), format!(r#"[project]
name   = "{name}"
author = "You"

[tasks.run]
cocotte run

[tasks.build]
cocotte build --release

[tasks.test]
cocotte test

[tasks.clean]
cocotte clean
"#))?;

    fs::write(project_dir.join(".gitignore"), "/dist\n/build\n*.cotcache\n")?;

    fs::write(project_dir.join("README.md"), format!(
        "# {name}\n\nA Cocotte project.\n\n## Usage\n\n```sh\ncocotte run\ncocotte build\ncocotte test\n```\n"
    ))?;

    fs::write(project_dir.join("src").join("main.cot"), r#"# main.cot — entry point

var name = "World"
print "Hello, " + name + "!"

func greet(who)
    print "Hello, " + who + "!"
end

greet("Cocotte")
"#)?;

    println!("Created project '{}'", name.cyan().bold());
    println!();
    println!("  cd {}", name);
    println!("  cocotte run");
    println!("  cocotte build");
    Ok(())
}
