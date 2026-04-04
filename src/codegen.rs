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

    // ── Detect rustup "no default toolchain" in stderr ───────────────────────
    let is_rustup_no_toolchain = |stderr: &str| -> bool {
        stderr.contains("rustup could not choose a version")
            || stderr.contains("no default is configured")
            || stderr.contains("rustup default stable")
    };

    // ── Copy built binary to dist/ ────────────────────────────────────────────
    let copy_output = |profile: &str| -> Result<()> {
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
            None => step_warn("Warning", "cargo succeeded but binary not found in target/"),
        }
        Ok(())
    };

    let profile = if opts.release { "release" } else { "debug" };

    // ── First attempt: whatever cargo run_cargo_with_progress resolves ────────
    match run_cargo_with_progress(&cargo_args, &tmp_dir, opts.verbose) {
        Ok((status, stderr)) if status.success() => {
            if !stderr.trim().is_empty() { eprint!("{}", stderr); }
            copy_output(profile)?;
        }

        Ok((_, stderr)) if is_rustup_no_toolchain(&stderr) => {
            // rustup shim is on PATH but has no toolchain configured.
            // Silently bootstrap our own toolchain and retry.
            step("Bootstrap", "rustup has no default toolchain — installing bundled toolchain");
            match ensure_bundled_cargo(opts.verbose) {
                Some(cargo_path) => {
                    match run_cargo_with_progress_using(&cargo_path, &cargo_args, &tmp_dir, opts.verbose) {
                        Ok((s, se)) if s.success() => {
                            if !se.trim().is_empty() { eprint!("{}", se); }
                            copy_output(profile)?;
                        }
                        Ok((_, se)) => {
                            if !se.trim().is_empty() { eprint!("{}", se); }
                            step_warn("Warning", "bundled cargo build failed — emitting source bundle");
                            emit_source_bundle(opts, source, &label)?;
                        }
                        Err(e) => {
                            step_warn("Warning", &format!("bundled cargo error: {} — emitting source bundle", e));
                            emit_source_bundle(opts, source, &label)?;
                        }
                    }
                }
                None => {
                    step_warn("Warning", "bootstrap failed — emitting source bundle");
                    substep("fix your Rust install with: rustup default stable");
                    emit_source_bundle(opts, source, &label)?;
                }
            }
        }

        Ok((status, stderr)) => {
            // Real cargo build error — show output, emit source bundle.
            if !stderr.trim().is_empty() { eprint!("{}", stderr); }
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
            // cargo binary not on PATH at all — bootstrap from scratch.
            match ensure_bundled_cargo(opts.verbose) {
                Some(cargo_path) => {
                    match run_cargo_with_progress_using(&cargo_path, &cargo_args, &tmp_dir, opts.verbose) {
                        Ok((s, se)) if s.success() => {
                            if !se.trim().is_empty() { eprint!("{}", se); }
                            copy_output(profile)?;
                        }
                        Ok((_, se)) => {
                            if !se.trim().is_empty() { eprint!("{}", se); }
                            step_warn("Warning", "bundled cargo failed — emitting source bundle");
                            emit_source_bundle(opts, source, &label)?;
                        }
                        Err(e) => {
                            step_warn("Warning", &format!("bundled cargo error: {} — emitting source bundle", e));
                            emit_source_bundle(opts, source, &label)?;
                        }
                    }
                }
                None => {
                    step_warn("Warning", "cargo not found and bootstrap failed — emitting source bundle");
                    substep("install Rust from https://rustup.rs, or cocotte will auto-install it with internet access");
                    emit_source_bundle(opts, source, &label)?;
                }
            }
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
) -> std::io::Result<(std::process::ExitStatus, String)> {
    // Only use the PATH cargo here. If it's a broken rustup shim, the caller
    // will detect that from stderr and trigger ensure_bundled_cargo separately.
    let cargo_bin = which_cargo()?;
    run_cargo_with_progress_using(&cargo_bin, args, work_dir, verbose)
}

// ── run_cargo_with_progress_using ─────────────────────────────────────────────
// Same as run_cargo_with_progress but takes an explicit path to the cargo binary.
// Used when the bundled toolchain was bootstrapped and cargo is not on PATH.

fn run_cargo_with_progress_using(
    cargo:    &Path,
    args:     &[String],
    work_dir: &Path,
    verbose:  bool,
) -> std::io::Result<(std::process::ExitStatus, String)> {
    use std::io::BufRead;

    // Only redirect CARGO_HOME / RUSTUP_HOME when using the bundled toolchain
    // (i.e. when cargo lives inside our private ~/.cocotte/toolchain/ dir).
    // For the system cargo we leave env vars alone.
    let toolchain_dir   = cocotte_toolchain_dir();
    let is_bundled      = cargo.starts_with(&toolchain_dir);
    let cargo_home      = toolchain_dir.join("cargo");
    let rustup_home     = toolchain_dir.join("rustup");

    let mut cmd_base = std::process::Command::new(cargo);
    cmd_base.current_dir(work_dir);
    if is_bundled {
        cmd_base
            .env("CARGO_HOME",       &cargo_home)
            .env("RUSTUP_HOME",      &rustup_home)
            .env_remove("RUSTUP_TOOLCHAIN");
    }

    if verbose {
        let status = cmd_base
            .args(args)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()?;
        return Ok((status, String::new()));
    }

    let mut json_args = args.to_vec();
    json_args.push("--message-format=json-render-diagnostics".into());

    let mut child = cmd_base
        .args(&json_args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().expect("cargo stdout pipe");
    let stderr = child.stderr.take().expect("cargo stderr pipe");

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
            if json["reason"].as_str() == Some("compiler-artifact") {
                count += 1;
                let pkg = json["package_id"]
                    .as_str()
                    .and_then(|s| s.split('#').last())
                    .and_then(|s| s.split_whitespace().next())
                    .unwrap_or("?");
                let pkg_short = if pkg.len() > 22 { &pkg[..22] } else { pkg };
                let cycle  = count % (BAR_W * 2);
                let filled = (if cycle <= BAR_W { cycle } else { BAR_W * 2 - cycle }).max(1);
                let empty  = BAR_W.saturating_sub(filled);
                let bar    = format!("{}{}", "█".repeat(filled), "░".repeat(empty));
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

    if count > 0 {
        print!("\r\x1b[K");
        let _ = std::io::stdout().flush();
    }

    let status = child.wait()?;
    let captured_stderr = stderr_handle.join().unwrap_or_default();
    // Always return stderr to the caller — it decides whether to print it.
    Ok((status, captured_stderr))
}

// ── Bundled Cargo bootstrap ───────────────────────────────────────────────────
//
// When `cargo` is not on PATH, Cocotte can bootstrap its own copy of the Rust
// toolchain into ~/.cocotte/toolchain/ using rustup-init.
//
// Layout after bootstrap:
//   ~/.cocotte/toolchain/
//     rustup-init          (the rustup installer binary, cached)
//     cargo/               (CARGO_HOME — registry, bin/cargo, …)
//     rustup/              (RUSTUP_HOME — toolchains, …)
//
// The first build takes a few minutes (downloads ~300 MB); subsequent builds
// are instant because the toolchain is already installed.

fn cocotte_toolchain_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cocotte")
        .join("toolchain")
}

/// Returns the path to a `cargo` binary, bootstrapping one if needed.
/// Returns None only if the download/install fails.
/// Ensure a working cargo binary exists in ~/.cocotte/toolchain/.
/// NEVER returns the system PATH cargo — that's the caller's job.
/// This function only manages the private bundled toolchain.
fn ensure_bundled_cargo(verbose: bool) -> Option<PathBuf> {
    let toolchain_dir = cocotte_toolchain_dir();
    let cargo_home    = toolchain_dir.join("cargo");
    let cargo_bin     = cargo_home.join("bin").join(if cfg!(windows) { "cargo.exe" } else { "cargo" });

    // Already bootstrapped and the binary is a real executable (not a rustup shim).
    // We check it isn't a rustup shim by verifying it lives inside our own dir.
    if cargo_bin.exists() {
        return Some(cargo_bin);
    }

    // Need to bootstrap — download rustup-init and run it isolated from the
    // system rustup so it installs a real toolchain, not another shim.
    step("Bootstrap", "installing bundled Rust toolchain into ~/.cocotte/toolchain");
    substep("this only happens once; subsequent builds will be fast");

    if let Err(e) = fs::create_dir_all(&toolchain_dir) {
        step_err("Error", &format!("cannot create toolchain dir: {}", e));
        return None;
    }

    let rustup_init_path = toolchain_dir.join(if cfg!(windows) { "rustup-init.exe" } else { "rustup-init" });

    step("Downloading", "rustup-init");
    if verbose { substep(&format!("url: {}", rustup_init_url())); }

    if let Err(e) = download_file(rustup_init_url(), &rustup_init_path) {
        step_err("Error", &format!("download failed: {}", e));
        substep("check your internet connection or install Rust manually from https://rustup.rs");
        return None;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&rustup_init_path, fs::Permissions::from_mode(0o755));
    }

    step("Installing", "Rust stable toolchain (minimal profile)");
    substep("this may take a few minutes on first run…");

    let rustup_home = toolchain_dir.join("rustup");

    // Critical: set both CARGO_HOME and RUSTUP_HOME to our private dir,
    // and also set RUSTUP_HOME on the child so rustup-init doesn't touch
    // the system ~/.rustup at all.
    let status = std::process::Command::new(&rustup_init_path)
        .args([
            "--no-modify-path",
            "--default-toolchain", "stable",
            "--profile", "minimal",
            "-y",
        ])
        .env("CARGO_HOME",  &cargo_home)
        .env("RUSTUP_HOME", &rustup_home)
        // Remove the system RUSTUP_TOOLCHAIN override if any — it would cause
        // our freshly installed toolchain to be ignored.
        .env_remove("RUSTUP_TOOLCHAIN")
        .stdout(if verbose { std::process::Stdio::inherit() } else { std::process::Stdio::null() })
        .stderr(if verbose { std::process::Stdio::inherit() } else { std::process::Stdio::null() })
        .status();

    match status {
        Ok(s) if s.success() && cargo_bin.exists() => {
            step("Ready", &format!("bundled toolchain installed at {}", cargo_bin.display()));
            Some(cargo_bin)
        }
        Ok(s) if s.success() => {
            step_err("Error", "rustup-init succeeded but cargo binary not found — check disk space");
            None
        }
        Ok(s) => {
            step_err("Error", &format!("rustup-init exited {}", s));
            None
        }
        Err(e) => {
            step_err("Error", &format!("rustup-init failed to run: {}", e));
            None
        }
    }
}

/// Returns the rustup-init download URL for the current host triple.
fn rustup_init_url() -> &'static str {
    // Minimal set of triples; rustup.rs always serves the right one.
    if cfg!(all(target_os = "linux",   target_arch = "x86_64"))  {
        "https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init"
    } else if cfg!(all(target_os = "linux",   target_arch = "aarch64")) {
        "https://static.rust-lang.org/rustup/dist/aarch64-unknown-linux-gnu/rustup-init"
    } else if cfg!(all(target_os = "linux",   target_arch = "arm"))     {
        "https://static.rust-lang.org/rustup/dist/armv7-unknown-linux-gnueabihf/rustup-init"
    } else if cfg!(all(target_os = "macos",   target_arch = "x86_64"))  {
        "https://static.rust-lang.org/rustup/dist/x86_64-apple-darwin/rustup-init"
    } else if cfg!(all(target_os = "macos",   target_arch = "aarch64")) {
        "https://static.rust-lang.org/rustup/dist/aarch64-apple-darwin/rustup-init"
    } else if cfg!(target_os = "windows") {
        "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe"
    } else {
        // Generic fallback — rustup.rs auto-detects
        "https://sh.rustup.rs"
    }
}

/// Locate `cargo` on the system PATH.
fn which_cargo() -> std::io::Result<PathBuf> {
    let cargo_name = if cfg!(windows) { "cargo.exe" } else { "cargo" };
    std::env::var_os("PATH")
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "PATH not set"))?
        .to_string_lossy()
        .split(if cfg!(windows) { ';' } else { ':' })
        .map(|dir| PathBuf::from(dir).join(cargo_name))
        .find(|p| p.is_file())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "cargo not in PATH"))
}

/// Download a URL to a local file using ureq (already a dependency).
fn download_file(url: &str, dest: &Path) -> std::result::Result<(), String> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| format!("HTTP request failed: {}", e))?;
    let mut reader = response.into_reader();
    let mut file = fs::File::create(dest)
        .map_err(|e| format!("cannot create file: {}", e))?;
    std::io::copy(&mut reader, &mut file)
        .map_err(|e| format!("write failed: {}", e))?;
    Ok(())
}

fn workspace_cargo_toml() -> String {
    r#"[workspace]
members = ["runner"]
resolver = "2"

[profile.release]
opt-level = 3
lto       = true
strip     = true
"#.to_string()
}

/// Try to find the Cocotte compiler's `src/` directory on disk.
///
/// Search order:
///   1. `COCOTTE_SRC` environment variable — explicit override, always wins.
///   2. Dev-build layout: exe lives at `target/{profile}/cocotte`, so walk up
///      three levels to the workspace root and look for `src/`.
///   3. Well-known system installation paths (populated by `make install`):
///      `/usr/share/cocotte/src`, `/usr/local/share/cocotte/src`, etc.
///   4. Paths relative to the current working directory (dev convenience).
fn locate_runtime_src() -> Option<PathBuf> {
    // 1. Explicit env override — highest priority
    if let Ok(p) = std::env::var("COCOTTE_SRC") {
        let c = PathBuf::from(&p);
        if c.join("interpreter.rs").exists() {
            return Some(c);
        }
        // Env was set but path is wrong — warn and continue rather than silently
        // falling through to a different source tree.
        eprintln!(
            "warning: COCOTTE_SRC=\"{}\" set but interpreter.rs not found there",
            p
        );
    }

    // 2. Dev-build layout: target/debug/cocotte  →  ../../.. → src/
    if let Ok(exe) = std::env::current_exe() {
        if let Some(candidate) = exe
            .parent()           // target/debug  (or target/release)
            .and_then(|p| p.parent())   // target/
            .and_then(|p| p.parent())   // workspace root
            .map(|p| p.join("src"))
        {
            if candidate.join("interpreter.rs").exists() {
                return Some(candidate);
            }
        }
    }

    // 3. System install paths — populated by `make install`.
    //    Also checks XDG user data dir (~/.local/share/cocotte on Linux,
    //    ~/Library/Application Support/cocotte on macOS).
    let mut system_prefixes: Vec<PathBuf> = vec![
        PathBuf::from("/usr/share/cocotte"),
        PathBuf::from("/usr/local/share/cocotte"),
        PathBuf::from("/opt/cocotte"),
    ];
    if let Some(d) = dirs::data_local_dir() { system_prefixes.push(d.join("cocotte")); }
    if let Some(d) = dirs::data_dir()       { system_prefixes.push(d.join("cocotte")); }
    for prefix in &system_prefixes {
        let c = prefix.join("src");
        if c.join("interpreter.rs").exists() {
            return Some(c);
        }
    }

    // 4. Paths relative to cwd — useful when running `cocotte` from the
    //    language's own source tree without `cargo install`
    if let Ok(cwd) = std::env::current_dir() {
        for rel in &["src", "../src", "cocottelang/src", "../cocottelang/src"] {
            let c = cwd.join(rel);
            if c.join("interpreter.rs").exists() {
                return Some(c);
            }
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
pub mod runtime_ctx;
pub mod http_server;
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
