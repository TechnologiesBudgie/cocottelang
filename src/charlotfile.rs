// charlotfile.rs — Charlotfile task runner for Cocotte
// Parses and executes the TOML-style Charlotfile for multi-language projects.
// Supports Cocotte, Rust (cargo), React (npm), Python, C/C++ commands.

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use colored::Colorize;
use crate::error::{CocotteError, Result};

/// A parsed Charlotfile
#[derive(Debug)]
pub struct Charlotfile {
    pub project_name: String,
    pub author: String,
    pub variables: HashMap<String, String>,
    pub tasks: HashMap<String, Vec<TaskStep>>,
}

/// A single step in a task (can be a shell command or a sub-task dependency)
#[derive(Debug, Clone)]
pub enum TaskStep {
    /// Shell command to execute
    Command(String),
    /// Reference to another task: `task: name`
    Dep(String),
}

/// Parse a Charlotfile from its path
pub fn parse_charlotfile(path: &Path) -> Result<Charlotfile> {
    let content = std::fs::read_to_string(path)?;
    parse_content(&content)
}

fn parse_content(content: &str) -> Result<Charlotfile> {
    let mut project_name = String::from("Unnamed");
    let mut author = String::new();
    let mut variables: HashMap<String, String> = HashMap::new();
    let mut tasks: HashMap<String, Vec<TaskStep>> = HashMap::new();
    let mut current_section: Option<String> = None;
    let mut current_task: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments and blank lines
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Section header: [project], [tasks.build], [variables], etc.
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = &trimmed[1..trimmed.len()-1];
            if section == "project" {
                current_section = Some("project".to_string());
                current_task = None;
            } else if section == "variables" || section == "vars" {
                current_section = Some("variables".to_string());
                current_task = None;
            } else if let Some(task_name) = section.strip_prefix("tasks.") {
                current_section = Some("task".to_string());
                current_task = Some(task_name.to_string());
                tasks.entry(task_name.to_string()).or_insert_with(Vec::new);
            } else {
                current_section = Some(section.to_string());
                current_task = None;
            }
            continue;
        }

        match current_section.as_deref() {
            Some("project") => {
                if let Some((k, v)) = split_kv(trimmed) {
                    match k.trim() {
                        "name" => project_name = unquote(v.trim()),
                        "author" => author = unquote(v.trim()),
                        _ => {}
                    }
                }
            }
            Some("variables") => {
                if let Some((k, v)) = split_kv(trimmed) {
                    variables.insert(k.trim().to_string(), unquote(v.trim()));
                }
            }
            Some("task") => {
                if let Some(ref task_name) = current_task.clone() {
                    let steps = tasks.entry(task_name.clone()).or_insert_with(Vec::new);
                    // Expand variables in command
                    let cmd = expand_vars(trimmed, &variables);
                    if let Some(dep) = cmd.strip_prefix("task:") {
                        steps.push(TaskStep::Dep(dep.trim().to_string()));
                    } else {
                        steps.push(TaskStep::Command(cmd));
                    }
                }
            }
            _ => {
                // Lines not in a section default to task commands if we have a current task
                if let Some(ref task_name) = current_task.clone() {
                    let cmd = expand_vars(trimmed, &variables);
                    tasks.entry(task_name.clone())
                        .or_insert_with(Vec::new)
                        .push(TaskStep::Command(cmd));
                }
            }
        }
    }

    Ok(Charlotfile { project_name, author, variables, tasks })
}

fn split_kv(s: &str) -> Option<(&str, &str)> {
    s.find('=').map(|i| (&s[..i], &s[i+1..]))
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"'))
        || (s.starts_with('\'') && s.ends_with('\''))
    {
        s[1..s.len()-1].to_string()
    } else {
        s.to_string()
    }
}

fn expand_vars(s: &str, vars: &HashMap<String, String>) -> String {
    let mut result = s.to_string();
    for (k, v) in vars {
        result = result.replace(&format!("${{{}}}", k), v);
        result = result.replace(&format!("${}", k), v);
    }
    result
}

/// Execute a named task from a Charlotfile
pub fn exec_task(charlotfile: &Charlotfile, task_name: &str, verbose: bool) -> Result<()> {
    let steps = charlotfile.tasks.get(task_name).ok_or_else(|| {
        CocotteError::build_err(&format!(
            "Task '{}' not found in Charlotfile.\nAvailable tasks: {}",
            task_name,
            charlotfile.tasks.keys().cloned().collect::<Vec<_>>().join(", ")
        ))
    })?;

    println!("{} {} {}", "▶".green().bold(), "Running task:".bold(), task_name.cyan().bold());
    if !charlotfile.author.is_empty() {
        println!("  {} {}", "Project:".dimmed(), charlotfile.project_name.dimmed());
    }
    println!();

    let steps = steps.clone();
    for step in &steps {
        match step {
            TaskStep::Dep(dep_name) => {
                println!("  {} {}", "→ Dependency:".yellow(), dep_name);
                exec_task(charlotfile, dep_name, verbose)?;
            }
            TaskStep::Command(cmd) => {
                run_command(cmd, verbose)?;
            }
        }
    }

    println!("{} Task '{}' completed successfully\n", "✓".green().bold(), task_name.green());
    Ok(())
}

/// Execute a single shell command (handles `cd path && cmd` patterns)
fn run_command(cmd: &str, verbose: bool) -> Result<()> {
    println!("  {} {}", "$".cyan(), cmd.white());

    // Handle `cd dir && rest` pattern
    if let Some((cd_part, rest)) = parse_cd_and(cmd) {
        let dir = cd_part.trim();
        if verbose {
            println!("    (changing to directory: {})", dir);
        }
        let status = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", &format!("cd /d {} && {}", dir, rest)])
                .status()
        } else {
            Command::new("sh")
                .args(["-c", &format!("cd {} && {}", dir, rest)])
                .status()
        };
        check_status(status, cmd)?;
        return Ok(());
    }

    // Plain command
    let status = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", cmd]).status()
    } else {
        Command::new("sh").args(["-c", cmd]).status()
    };

    check_status(status, cmd)
}

fn parse_cd_and(cmd: &str) -> Option<(&str, &str)> {
    if let Some(cd_rest) = cmd.strip_prefix("cd ") {
        if let Some(idx) = cd_rest.find("&&") {
            let dir = cd_rest[..idx].trim();
            let rest = cd_rest[idx+2..].trim();
            return Some((dir, rest));
        }
    }
    None
}

fn check_status(
    status: std::io::Result<std::process::ExitStatus>,
    cmd: &str,
) -> Result<()> {
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(CocotteError::build_err(&format!(
            "Command failed with code {}: {}",
            s.code().unwrap_or(-1), cmd
        ))),
        Err(e) => Err(CocotteError::io_err(&format!(
            "Failed to execute '{}': {}", cmd, e
        ))),
    }
}

/// List all tasks in a Charlotfile
pub fn list_tasks(charlotfile: &Charlotfile) {
    println!("{}", "📋 Available tasks:".bold());
    let mut names: Vec<&String> = charlotfile.tasks.keys().collect();
    names.sort();
    for name in names {
        let steps = &charlotfile.tasks[name];
        println!("  {} {}", "▸".cyan(), name.white().bold());
        for step in steps {
            match step {
                TaskStep::Command(cmd) => println!("      $ {}", cmd.dimmed()),
                TaskStep::Dep(dep) => println!("      → (task) {}", dep.dimmed()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_charlotfile() {
        let content = r#"
[project]
name = "TestApp"
author = "Dev"

[tasks.build]
cocotte build --release
cd backend && cargo build --release

[tasks.clean]
cocotte clean
        "#;
        let cf = parse_content(content).unwrap();
        assert_eq!(cf.project_name, "TestApp");
        assert_eq!(cf.author, "Dev");
        assert!(cf.tasks.contains_key("build"));
        assert!(cf.tasks.contains_key("clean"));
        let build = &cf.tasks["build"];
        assert_eq!(build.len(), 2);
    }
}
