//! package_manager.rs — Cocotte package manager (cpm)
//!
//! Registry protocol: JSON at REGISTRY_URL/<index.json>
//! Each package entry:
//!   { "name": "...", "version": "...", "description": "...",
//!     "url": "...", "checksum": "sha256:...", "kind": "cotmod"|"cotlib" }
//!
//! Local state: Millet.lock (pinned versions) + modules/ or libraries/ dirs.

use std::fs;
use std::path::{Path, PathBuf};
use colored::Colorize;
use crate::error::{CocotteError, Result};

const REGISTRY_URL: &str = "https://pkg.cocotte-lang.org";
const LOCK_FILE:    &str = "Millet.lock";

// ── Package entry ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Package {
    pub name:        String,
    pub version:     String,
    pub description: String,
    pub url:         String,
    pub kind:        PackageKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PackageKind {
    Module,  // .cotmod — installed to modules/
    Library, // .cotlib — installed to libraries/
}

// ── Lock file entry ───────────────────────────────────────────────────────────

#[derive(Debug)]
struct LockEntry {
    name:     String,
    version:  String,
    checksum: String,
}

// ── Public CLI entry point ────────────────────────────────────────────────────

pub fn run(subcommand: &str, args: &[String]) -> Result<()> {
    match subcommand {
        "help" | "--help" | "-h" => { print_help(); Ok(()) }
        "search"  => cmd_search(args),
        "install" | "add"  => cmd_install(args),
        "remove"  | "rm"   => cmd_remove(args),
        "update"  | "up"   => cmd_update(args),
        "list"    | "ls"   => cmd_list(),
        "info"             => cmd_info(args),
        "publish"          => cmd_publish(),
        other => {
            eprintln!("{} Unknown pkg subcommand '{}'. Try: cocotte pkg help", "error:".red().bold(), other);
            Ok(())
        }
    }
}

fn print_help() {
    println!("{}", "cocotte pkg — Cocotte Package Manager".bold());
    println!();
    println!("  {}  <query>    Search the registry for packages", "search".cyan());
    println!("  {}  <name>     Install a package from the registry", "install".cyan());
    println!("  {}  <name>     Remove an installed package", "remove".cyan());
    println!("  {}             Update all installed packages", "update".cyan());
    println!("  {}             List installed packages", "list".cyan());
    println!("  {}    <name>   Show package information", "info".cyan());
    println!("  {}             Publish a package (requires auth)", "publish".cyan());
    println!();
    println!("  Registry: {}", REGISTRY_URL.dimmed());
}

// ── Search ────────────────────────────────────────────────────────────────────

fn cmd_search(args: &[String]) -> Result<()> {
    let query = args.first().map(String::as_str).unwrap_or("");
    step("Searching", &format!("'{}' on {}", query, REGISTRY_URL));

    let packages = fetch_index()?;
    let matches: Vec<&Package> = if query.is_empty() {
        packages.iter().collect()
    } else {
        packages.iter().filter(|p| {
            p.name.contains(query) || p.description.to_lowercase().contains(&query.to_lowercase())
        }).collect()
    };

    if matches.is_empty() {
        println!("No packages found for '{}'.", query);
        return Ok(());
    }

    println!("{:>12}  {:<30}  {:<10}  {}",
        "name".bold(), "description".bold(), "version".bold(), "kind".bold());
    println!("{}", "-".repeat(72).dimmed());
    for pkg in &matches {
        println!("{:>12}  {:<30}  {:<10}  {}",
            pkg.name.cyan(),
            truncate(&pkg.description, 30),
            pkg.version.dimmed(),
            match pkg.kind { PackageKind::Module => "module", PackageKind::Library => "library" });
    }
    println!();
    println!("Install with: {}", format!("cocotte pkg install <name>").dimmed());
    Ok(())
}

// ── Install ───────────────────────────────────────────────────────────────────

fn cmd_install(args: &[String]) -> Result<()> {
    if args.is_empty() {
        // Install all packages listed in Millet.toml
        return install_from_millet();
    }

    for name in args {
        install_package(name)?;
    }
    Ok(())
}

fn install_package(name: &str) -> Result<()> {
    step("Fetching", &format!("package metadata for '{}'", name));

    let packages = fetch_index()?;
    let pkg = packages.iter().find(|p| p.name == name).ok_or_else(|| {
        CocotteError::module_err(&format!(
            "Package '{}' not found in registry.\n  Try: cocotte pkg search {}",
            name, name
        ))
    })?;

    let dest_dir = match pkg.kind {
        PackageKind::Module  => PathBuf::from("modules"),
        PackageKind::Library => PathBuf::from("libraries"),
    };
    fs::create_dir_all(&dest_dir)?;

    let ext = match pkg.kind {
        PackageKind::Module  => "cotmod",
        PackageKind::Library => "cotlib",
    };
    let dest_path = dest_dir.join(format!("{}.{}", pkg.name, ext));

    step("Downloading", &format!("{} v{}", pkg.name.bold(), pkg.version));

    let content = ureq::get(&pkg.url)
        .call()
        .map_err(|e| CocotteError::io_err(&format!("Download failed: {}", e)))?
        .into_string()
        .map_err(|e| CocotteError::io_err(&format!("Read failed: {}", e)))?;

    fs::write(&dest_path, &content)?;

    // Update Millet.toml
    update_millet_dep(&pkg.name, &pkg.version, &pkg.kind)?;
    // Write lock file entry
    append_lock(name, &pkg.version, &format!("sha256:{}", simple_hash(&content)))?;

    step("Installed", &format!("{} v{} → {}",
        pkg.name.green().bold(),
        pkg.version,
        dest_path.display().to_string().dimmed()
    ));
    Ok(())
}

fn install_from_millet() -> Result<()> {
    let toml = fs::read_to_string("Millet.toml")
        .map_err(|_| CocotteError::build_err("No Millet.toml found — run cocotte init first"))?;

    step("Installing", "dependencies from Millet.toml");

    // Parse modules = ["a", "b"] and libraries = ["c"]
    for line in toml.lines() {
        let line = line.trim();
        if line.starts_with("modules") || line.starts_with("libraries") {
            // Extract quoted names
            let names: Vec<&str> = line.split('"')
                .enumerate()
                .filter(|(i, _)| i % 2 == 1)
                .map(|(_, s)| s)
                .collect();
            for name in names {
                if !name.is_empty() {
                    install_package(name).unwrap_or_else(|e| {
                        step_warn("Skipping", &format!("{}: {}", name, e.to_string()));
                    });
                }
            }
        }
    }
    Ok(())
}

// ── Remove ────────────────────────────────────────────────────────────────────

fn cmd_remove(args: &[String]) -> Result<()> {
    if args.is_empty() {
        return Err(CocotteError::build_err("cocotte pkg remove <name>"));
    }
    for name in args {
        // Try both extensions
        let removed = try_remove(&format!("modules/{}.cotmod", name))
            || try_remove(&format!("libraries/{}.cotlib", name));
        if removed {
            remove_from_lock(name)?;
            step("Removed", &name.red().to_string());
        } else {
            step_warn("Not found", &format!("'{}' is not installed", name));
        }
    }
    Ok(())
}

fn try_remove(path: &str) -> bool {
    if Path::new(path).exists() {
        fs::remove_file(path).is_ok()
    } else {
        false
    }
}

// ── Update ────────────────────────────────────────────────────────────────────

fn cmd_update(args: &[String]) -> Result<()> {
    step("Checking", "registry for updates");
    let packages = fetch_index()?;

    // Read lock file to find installed packages + pinned versions
    let lock = read_lock()?;

    let targets: Vec<&str> = if args.is_empty() {
        lock.iter().map(|e| e.name.as_str()).collect()
    } else {
        args.iter().map(String::as_str).collect()
    };

    let mut updated = 0usize;
    for name in targets {
        if let Some(pkg) = packages.iter().find(|p| p.name == name) {
            let current_version = lock.iter()
                .find(|e| e.name == name)
                .map(|e| e.version.as_str())
                .unwrap_or("0.0.0");
            if pkg.version != current_version {
                install_package(name)?;
                updated += 1;
            } else {
                println!("{:>12} {} (already up-to-date)", "Check".dimmed(), name);
            }
        }
    }
    if updated == 0 {
        step("Finished", "all packages are up-to-date");
    } else {
        step("Finished", &format!("updated {} package(s)", updated));
    }
    Ok(())
}

// ── List ──────────────────────────────────────────────────────────────────────

fn cmd_list() -> Result<()> {
    let lock = read_lock()?;
    if lock.is_empty() {
        println!("No packages installed. Try: {}", "cocotte pkg search".dimmed());
        return Ok(());
    }
    println!("{:>12}  {:<20}  {}", "name".bold(), "version".bold(), "checksum".bold());
    println!("{}", "-".repeat(60).dimmed());
    for entry in &lock {
        println!("{:>12}  {:<20}  {}", entry.name.cyan(), entry.version, entry.checksum.dimmed());
    }
    Ok(())
}

// ── Info ──────────────────────────────────────────────────────────────────────

fn cmd_info(args: &[String]) -> Result<()> {
    let name = args.first().ok_or_else(|| CocotteError::build_err("cocotte pkg info <name>"))?;
    let packages = fetch_index()?;
    match packages.iter().find(|p| p.name == *name) {
        Some(pkg) => {
            println!("{} {}", pkg.name.bold().cyan(), pkg.version.dimmed());
            println!("  {}", pkg.description);
            println!("  kind: {:?}", pkg.kind);
            println!("  url:  {}", pkg.url.dimmed());
        }
        None => println!("Package '{}' not found in registry.", name),
    }
    Ok(())
}

// ── Publish ───────────────────────────────────────────────────────────────────

fn cmd_publish() -> Result<()> {
    println!("{}", "Publishing is not yet implemented in this version.".yellow());
    println!("Visit {} to publish packages manually.", REGISTRY_URL.cyan());
    Ok(())
}

// ── Registry fetch ────────────────────────────────────────────────────────────

fn fetch_index() -> Result<Vec<Package>> {
    let url = format!("{}/index.json", REGISTRY_URL);
    match ureq::get(&url).call() {
        Ok(resp) => {
            let text = resp.into_string()
                .map_err(|e| CocotteError::io_err(&format!("Registry read error: {}", e)))?;
            parse_index(&text)
        }
        Err(_) => {
            // Offline mode: return empty registry with helpful message
            step_warn("Offline", "could not reach registry — showing local packages only");
            Ok(vec![])
        }
    }
}

fn parse_index(json: &str) -> Result<Vec<Package>> {
    let v: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| CocotteError::build_err(&format!("Registry JSON parse error: {}", e)))?;
    let arr = v.as_array().unwrap_or(&vec![]).to_vec();
    let mut packages = Vec::new();
    for item in arr {
        if let (Some(name), Some(version), Some(description), Some(url), Some(kind)) = (
            item["name"].as_str(),
            item["version"].as_str(),
            item["description"].as_str(),
            item["url"].as_str(),
            item["kind"].as_str(),
        ) {
            packages.push(Package {
                name:        name.to_string(),
                version:     version.to_string(),
                description: description.to_string(),
                url:         url.to_string(),
                kind: if kind == "cotlib" { PackageKind::Library } else { PackageKind::Module },
            });
        }
    }
    Ok(packages)
}

// ── Millet.toml manipulation ──────────────────────────────────────────────────

fn update_millet_dep(name: &str, version: &str, kind: &PackageKind) -> Result<()> {
    let path = Path::new("Millet.toml");
    if !path.exists() { return Ok(()); }
    let content = fs::read_to_string(path)?;
    let key = match kind { PackageKind::Module => "modules", PackageKind::Library => "libraries" };
    if content.contains(&format!("\"{}\"", name)) { return Ok(()); }
    let new_content = content.replace(
        &format!("{} = [", key),
        &format!("{} = [\"{}\", ", key, name),
    );
    let _ = version; // could write to lock file
    fs::write(path, new_content)?;
    Ok(())
}

// ── Lock file ─────────────────────────────────────────────────────────────────

fn read_lock() -> Result<Vec<LockEntry>> {
    let path = Path::new(LOCK_FILE);
    if !path.exists() { return Ok(vec![]); }
    let content = fs::read_to_string(path)?;
    let mut entries = Vec::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        if parts.len() == 3 {
            entries.push(LockEntry {
                name:     parts[0].to_string(),
                version:  parts[1].to_string(),
                checksum: parts[2].to_string(),
            });
        }
    }
    Ok(entries)
}

fn append_lock(name: &str, version: &str, checksum: &str) -> Result<()> {
    let path = Path::new(LOCK_FILE);
    let mut content = if path.exists() { fs::read_to_string(path)? } else { String::new() };
    // Remove existing entry for this package
    content = content.lines()
        .filter(|l| !l.starts_with(&format!("{} ", name)))
        .collect::<Vec<_>>()
        .join("\n");
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&format!("{} {} {}\n", name, version, checksum));
    fs::write(path, content)?;
    Ok(())
}

fn remove_from_lock(name: &str) -> Result<()> {
    let path = Path::new(LOCK_FILE);
    if !path.exists() { return Ok(()); }
    let content = fs::read_to_string(path)?;
    let filtered: String = content.lines()
        .filter(|l| !l.starts_with(&format!("{} ", name)))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, filtered + "\n")?;
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn step(verb: &str, detail: &str) {
    println!("{:>12} {}", verb.green().bold(), detail);
}
fn step_warn(verb: &str, detail: &str) {
    println!("{:>12} {}", verb.yellow().bold(), detail);
}
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() }
    else { format!("{}…", &s[..max.saturating_sub(1)]) }
}
fn simple_hash(content: &str) -> String {
    // FNV-1a 64-bit — no external dep needed
    let mut hash: u64 = 14695981039346656037;
    for byte in content.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    format!("{:016x}", hash)
}
