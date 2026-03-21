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

use std::io::Write;
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

// ── Pretty step printer (cargo-style) ────────────────────────────────────────

#[allow(dead_code)]
fn step(verb: &str, detail: &str) {
    // Right-align verb in 12 chars, bold green — just like cargo
    println!("{:>12} {}", verb.green().bold(), detail);
}

#[allow(dead_code)]
fn step_warn(verb: &str, detail: &str) {
    println!("{:>12} {}", verb.yellow().bold(), detail);
}

#[allow(dead_code)]
fn step_err(verb: &str, detail: &str) {
    eprintln!("{:>12} {}", verb.red().bold(), detail);
}

#[allow(dead_code)]
fn substep(detail: &str) {
    println!("             {}", detail.dimmed());
}

fn detect_version() -> String {
    if let Ok(s) = fs::read_to_string("Millet.toml") {
        for line in s.lines() {
            if let Some(rest) = line.strip_prefix("version") {
                if let Some(val) = rest.split('=').nth(1) {
                    let v = val.trim().trim_matches('"').trim_matches('\'');
                    if !v.is_empty() { return v.to_string(); }
                }
            }
        }
    }
    "0.1.0".to_string()
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn build_project(opts: &BuildOptions) -> Result<()> {
    let n_targets = opts.targets.len();
    let plural    = if n_targets == 1 { "target" } else { "targets" };

    step("Compiling", &format!(
        "{} v{} ({} {})",
        opts.project_name.bold(),
        detect_version(),
        n_targets,
        plural,
    ));

    // ── Parse / validate source ───────────────────────────────────────────────
    step("Checking", &opts.source_path.display().to_string());

    let source = fs::read_to_string(&opts.source_path)
        .map_err(|e| CocotteError::build_err(&format!("cannot read source: {}", e)))?;

    let mut lexer = crate::lexer::Lexer::new(&source);
    let tokens = lexer.tokenize().map_err(|e| {
        step_err("error[E0001]", &format!("syntax error in {}", opts.source_path.display()));
        CocotteError::build_err(&format!("Syntax error: {}", e))
    })?;

    let mut parser = crate::parser::Parser::new(tokens);
    let _stmts = parser.parse().map_err(|e| {
        step_err("error[E0002]", &format!("parse error in {}", opts.source_path.display()));
        CocotteError::build_err(&format!("Parse error: {}", e))
    })?;

    if opts.verbose {
        substep(&format!("{} top-level statement(s) parsed", _stmts.statements.len()));
    }

    // ── Code generation ───────────────────────────────────────────────────────
    step("Generating", "runner crate");

    fs::create_dir_all(&opts.output_dir)?;

    for (os, arch) in &opts.targets {
        build_for_target(opts, &source, os, arch)?;
    }

    let profile_label = if opts.release {
        format!("{} profile [{}]", "release", "optimized".cyan())
    } else {
        format!("{} profile [{}]", "dev", "unoptimized + debuginfo")
    };

    step("Finished", &format!(
        "{} — output: {}",
        profile_label,
        opts.output_dir.display().to_string().cyan(),
    ));

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

    step("Targeting", &label);

    let triple_opt = os.rust_target(arch);
    if triple_opt.is_none() {
        step_warn("Warning", &format!(
            "unsupported target {}/{} — emitting source bundle instead",
            os.name(), arch.name()
        ));
        return emit_source_bundle(opts, source, &label);
    }
    // Safety: None was handled by the early return above.
    let triple = triple_opt.unwrap();

    let rt_src  = locate_runtime_src();
    let tmp_dir = std::env::temp_dir()
        .join(format!("cocotte_build_{}_{}", opts.project_name, label));
    fs::create_dir_all(&tmp_dir)?;

    if opts.verbose {
        substep(&format!("workspace  : {}", tmp_dir.display()));
        if !triple.is_empty() {
            substep(&format!("triple     : {}", triple));
        }
    }

    step("Scaffolding", "Cargo workspace");
    fs::write(tmp_dir.join("Cargo.toml"), workspace_cargo_toml())?;

    match rt_src {
        Some(rt_path) => {
            if opts.verbose {
                substep(&format!("runtime src: {}", rt_path.display()));
            }
            step("Embedding", "cocotte runtime");
            setup_runtime_crate(&tmp_dir, &rt_path, opts.verbose)?;
            step("Writing", "runner binary crate");
            setup_runner_crate(&tmp_dir, source, &opts.project_name)?;
        }
        None => {
            if opts.verbose {
                substep("runtime source not found on disk — using single-crate fallback");
            }
            step_warn("Fallback", "runtime source unavailable, using stub runner");
            setup_single_crate_fallback(&tmp_dir, source, &opts.project_name)?;
        }
    }

    // ── Invoke cargo ──────────────────────────────────────────────────────────
    let binary_name = os.binary_name(&opts.project_name, arch);
    let out_path    = opts.output_dir.join(&binary_name);

    let cargo_args: Vec<String> = {
        let mut a = vec!["build".into()];
        if opts.release       { a.push("--release".into()); }
        if !triple.is_empty() { a.push("--target".into()); a.push(triple.clone()); }
        a
    };

    if opts.verbose {
        substep(&format!("cargo {}", cargo_args.join(" ")));
    }

    step("Linking", &format!(
        "{} (cargo{}{})",
        opts.project_name,
        if opts.release { " --release" } else { "" },
        if !triple.is_empty() { format!(" --target {}", triple) } else { String::new() },
    ));

    match run_cargo_with_progress(&cargo_args, &tmp_dir, opts.verbose) {
        Ok(status) if status.success() => {
            let profile   = if opts.release { "release" } else { "debug" };
            let base_name = if matches!(os, TargetOS::Windows) {
                format!("{}.exe", opts.project_name)
            } else {
                opts.project_name.clone()
            };

            let native_path = tmp_dir.join("target").join(profile).join(&base_name);
            let cross_path  = tmp_dir.join("target").join(&triple).join(profile).join(&base_name);

            match [native_path, cross_path].into_iter().find(|p| p.exists()) {
                Some(p) => {
                    fs::copy(&p, &out_path)?;
                    step("Compiled", &out_path.display().to_string().green().bold().to_string());
                }
                None => {
                    step_warn("Warning", "cargo succeeded but binary not found in target/");
                    if opts.verbose {
                        substep(&format!("searched: {}", tmp_dir.join("target").display()));
                    }
                }
            }
        }
        Ok(status) => {
            step_warn("Warning", &format!(
                "cargo exited {} — emitting source bundle",
                status.code().map(|c| c.to_string()).unwrap_or_else(|| "?".into()),
            ));
            if !opts.verbose {
                substep("re-run with -v / --verbose to see cargo's full output");
            }
            emit_source_bundle(opts, source, &label)?;
        }
        Err(_) => {
            step_warn("Warning", "cargo not found — emitting source bundle");
            substep("install Rust from https://rustup.rs then retry");
            emit_source_bundle(opts, source, &label)?;
        }
    }

    Ok(())
}

// ── Live cargo progress bar ───────────────────────────────────────────────────
//
// In normal mode  : runs cargo with --message-format=json-render-diagnostics,
//   reads compiler-artifact JSON events from stdout and draws a live animated
//   progress bar. Compiler warnings/errors still appear via stderr.
//
// In verbose mode : inherits stdio so cargo's own "Compiling … / Finished"
//   output (including its own TTY progress bar) passes straight through.

fn run_cargo_with_progress(
    args:     &[String],
    work_dir: &Path,
    verbose:  bool,
) -> std::io::Result<std::process::ExitStatus> {
    use std::io::BufRead;

    if verbose {
        // Let cargo own the terminal completely — its own bar + Compiling lines.
        return std::process::Command::new("cargo")
            .args(args)
            .current_dir(work_dir)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status();
    }

    // ── Non-verbose: clean animated progress bar ──────────────────────────────
    // --message-format=json-render-diagnostics  →  JSON events on stdout,
    //   human-readable diagnostics on stderr.
    //
    // We pipe BOTH stdout and stderr so nothing from cargo reaches the terminal
    // while the bar is animating (interleaved text would corrupt the \r rewrite).
    // After cargo exits we replay stderr so warnings/errors still reach the user.
    let mut json_args = args.to_vec();
    json_args.push("--message-format=json-render-diagnostics".into());

    let mut child = std::process::Command::new("cargo")
        .args(&json_args)
        .current_dir(work_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())   // captured — replayed after bar
        .spawn()?;

    let stdout = child.stdout.take().expect("cargo stdout pipe");
    let stderr = child.stderr.take().expect("cargo stderr pipe");

    // Drain stderr in a background thread so it never blocks cargo.
    let stderr_handle = std::thread::spawn(move || {
        let mut buf = String::new();
        use std::io::Read;
        let _ = std::io::BufReader::new(stderr).read_to_string(&mut buf);
        buf
    });

    let reader = std::io::BufReader::new(stdout);

    const BAR_W: usize = 28;
    let mut count: usize = 0;

    for line in reader.lines() {
        let line = match line { Ok(l) => l, Err(_) => break };

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
            // compiler-artifact fires once per compiled crate
            if json["reason"].as_str() == Some("compiler-artifact") {
                count += 1;

                // package_id: "registry+https://...#crate-name 1.2.3 (...)"
                //          or "path+file:///...#crate-name 0.1.0"
                let pkg = json["package_id"]
                    .as_str()
                    .and_then(|s| s.split('#').last())
                    .and_then(|s| s.split_whitespace().next())
                    .unwrap_or("?");

                let pkg_short = if pkg.len() > 22 { &pkg[..22] } else { pkg };

                // Bouncing bar — looks good without knowing the total crate count
                let cycle  = count % (BAR_W * 2);
                let filled = (if cycle <= BAR_W { cycle } else { BAR_W * 2 - cycle }).max(1);
                let empty  = BAR_W.saturating_sub(filled);
                let bar    = format!("{}{}", "█".repeat(filled), "░".repeat(empty));

                // \r + \x1b[K: go to start of line, erase to EOL, redraw
                print!(
                    "\r\x1b[K{:>12} {} {:>4}  {:<22}",
                    "Building".green().bold(),
                    bar.cyan(),
                    count,
                    pkg_short.dimmed(),
                );
                let _ = std::io::stdout().flush();
            }
        }
    }

    // Erase bar line cleanly before any subsequent step() output
    if count > 0 {
        print!("\r\x1b[K");
        let _ = std::io::stdout().flush();
    }

    let status = child.wait()?;

    // Replay captured stderr now that the bar is gone — warnings/errors appear
    // below the bar, not tangled inside it.
    let captured_stderr = stderr_handle.join().unwrap_or_default();
    if !captured_stderr.trim().is_empty() {
        eprint!("{}", captured_stderr);
    }

    Ok(status)
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
        .filter(|e| {
            let p = e.path();
            p.extension().map(|x| x == "rs").unwrap_or(false)
                && p.file_name().map(|n| n != "main.rs").unwrap_or(true)
        })
        .collect();

    for entry in &rs_files {
        let fname = entry.file_name();
        fs::copy(entry.path(), src_dst.join(&fname))?;
    }

    if verbose {
        substep(&format!("copied {} runtime source file(s)", rs_files.len()));
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
serde_json = "=1.0.96"
colored    = "2"
indexmap   = "=2.0.2"
dirs       = "5"
ureq       = { version = "2", features = ["json"] }
rusqlite   = { version = "0.31", features = ["bundled"] }
eframe     = { version = "0.29", optional = true, features = ["wgpu"] }
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
    // Declare the same modules as main.rs as pub so the runner can use them.
    // charlotte is conditionally compiled — must match the [features] in Cargo.toml.
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
#[cfg(feature = "gui")]
pub mod charlotte;
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
serde_json = "=1.0.96"
colored    = "2"
indexmap   = "=2.0.2"
dirs       = "5"
ureq       = { version = "2", features = ["json"] }
rusqlite   = { version = "0.31", features = ["bundled"] }
eframe     = { version = "0.29", optional = true, features = ["wgpu"] }
egui       = { version = "0.29", optional = true }

[features]
default = ["gui"]
gui     = ["eframe", "egui"]
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

    step("Bundling", &format!("source → {}", bundle_dir.display()));
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
