use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::resolver::path::{find_all_in_path, PathSearchResult, SearchOrderEntry};
use crate::ecosystems::print_ecosystem_details;
use crate::inspectors::environment::print_env_report;
use crate::inspectors::services::print_services_report;

// ANSI colors
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const GRAY: &str = "\x1b[90m";

#[derive(Debug)]
pub struct InstallationInfo {
    pub manager: String,
    pub detail: Option<String>,
}

/// Core function that prints executable details based on flags.
pub fn print_executable_report(query: &str, all: bool, conflict: bool, show_env: bool) {
    let query_clean = query.trim_end_matches(".exe").to_lowercase();
    let (all_results, search_order) = find_all_in_path(query);

    if all_results.is_empty() {
        println!("\n{}{}Could not resolve executable '{}' in system PATH.{}", BOLD, YELLOW, query, RESET);
        return;
    }

    let app_display_name = match query_clean.as_str() {
        "node" => "Node.js",
        "python" => "Python",
        "docker" => "Docker",
        "rustc" | "cargo" | "rustup" => "Rust",
        "java" | "javac" => "Java",
        other => other,
    };

    if all {
        print_all_installations(&query_clean, app_display_name, &all_results);
        return;
    }

    let active = &all_results[0];

    println!("\n{}{}=== Smart System Report: {} ==={}", BOLD, CYAN, app_display_name, RESET);
    
    println!("\n{}Executable:{}", BOLD, RESET);
    println!("  {}", active.resolved_path.display());
    if active.is_shim {
        println!("  {}(resolved from Scoop shim: {}){}", GRAY, active.original_path.display(), RESET);
    }

    println!("\n{}Version:{}", BOLD, RESET);
    match query_version(&active.resolved_path) {
        Some(v) => println!("  {}{}{}", GREEN, v, RESET),
        None => println!("  {}(could not retrieve version){}", GRAY, RESET),
    }

    println!("\n{}Installed through:{}", BOLD, RESET);
    let install_info = detect_installation_source(&active.resolved_path);
    print!("  {}{}{}", GREEN, install_info.manager, RESET);
    if let Some(detail) = install_info.detail {
        print!(" - {}{}{}", GRAY, detail, RESET);
    }
    println!();

    print_ecosystem_details(&query_clean);

    println!("\n{}PATH entries:{}", BOLD, RESET);
    let path_entries = count_path_entries(&active.resolved_path);
    println!("  {} entry(ies) in system PATH point here", path_entries);

    // Why does it resolve here?
    println!("\n{}Why does `{}` resolve here?{}", BOLD, query, RESET);
    print_path_search_order(query, &search_order);

    println!("\n{}Environment variables:{}", BOLD, RESET);
    print_env_report(&query_clean, show_env);

    println!("\n{}Projects currently using it:{}", BOLD, RESET);
    let projects = scan_projects(&query_clean);
    if projects.is_empty() {
        println!("  {}(none found in ~/Projects){}", GRAY, RESET);
    } else {
        for proj in projects {
            let display_path = if let Ok(profile) = env::var("USERPROFILE") {
                proj.to_string_lossy().replace(&profile, "~")
            } else {
                proj.to_string_lossy().to_string()
            };
            println!("  {}", display_path);
        }
    }

    println!("\n{}Services depending on/related to it:{}", BOLD, RESET);
    print_services_report(&query_clean);

    if all_results.len() > 1 {
        println!("\n{}⚠ Multiple installations detected. Run 'why {} --all' for details.{}", YELLOW, query, RESET);
    }

    if conflict {
        print_conflict_diagnostics(&query_clean, &active.resolved_path, &all_results);
    }
}

fn print_all_installations(_query_name: &str, display_name: &str, results: &[PathSearchResult]) {
    println!("\n{}{}=== Found {} {} installations ==={}", BOLD, CYAN, results.len(), display_name, RESET);
    
    for (i, res) in results.iter().enumerate() {
        let active_label = if i == 0 {
            format!(" {}{}[ACTIVE]{}", BOLD, GREEN, RESET)
        } else {
            "".to_string()
        };
        
        println!("\n{}{}. Installation:{}", BOLD, i + 1, active_label);
        println!("  Path:    {}", res.resolved_path.display());
        if res.is_shim {
            println!("  Shim:    {}", res.original_path.display());
        }
        
        match query_version(&res.resolved_path) {
            Some(v) => println!("  Version: {}{}{}", GREEN, v, RESET),
            None => println!("  Version: {}(could not retrieve version){}", GRAY, RESET),
        }
        
        let install_info = detect_installation_source(&res.resolved_path);
        print!("  Source:  {}", install_info.manager);
        if let Some(detail) = install_info.detail {
            print!(" ({})", detail);
        }
        println!();
    }
    
    if results.len() > 1 {
        println!("\n{}⚠ Multiple installations detected may cause path conflicts.{}", YELLOW, RESET);
    }
}

fn print_path_search_order(_query: &str, search_order: &[SearchOrderEntry]) {
    let mut matched = false;
    let mut printed_count = 0;
    for entry in search_order {
        if !entry.matches.is_empty() {
            println!("  \x1b[32m-> {} ← MATCH ({})\x1b[0m", entry.dir.display(), entry.matches[0].file_name().unwrap_or_default().to_string_lossy());
            matched = true;
            break;
        } else {
            if printed_count < 3 {
                println!("  - {}", entry.dir.display());
                printed_count += 1;
            } else if printed_count == 3 {
                println!("  - ... (remaining directories skipped)");
                printed_count += 1;
            }
        }
    }
    if !matched {
        println!("  (none matched)");
    }
}

fn print_conflict_diagnostics(query_name: &str, active_path: &Path, all_results: &[PathSearchResult]) {
    println!("\n{}{}=== Conflict Diagnostics ==={}", BOLD, YELLOW, RESET);
    
    if all_results.len() > 1 {
        println!("  \x1b[33m⚠ Multiple {} installations detected on PATH:\x1b[0m", query_name);
        for res in all_results {
            println!("    - {}", res.resolved_path.display());
        }
    } else {
        println!("  ✓ No multiple installations found on PATH.");
    }

    if query_name == "node" {
        let (npm_res, _) = find_all_in_path("npm");
        if let Some(npm) = npm_res.first() {
            let node_dir_str = active_path.to_string_lossy().to_lowercase();
            let npm_dir_str = npm.resolved_path.to_string_lossy().to_lowercase();
            
            let both_fnm = node_dir_str.contains("fnm_multishells") && npm_dir_str.contains("fnm_multishells");
            let both_scoop = node_dir_str.contains("scoop") && npm_dir_str.contains("scoop");
            let both_program_files = node_dir_str.contains("program files") && npm_dir_str.contains("program files");
            
            if !both_fnm && !both_scoop && !both_program_files {
                println!("  \x1b[31m⚠ Conflict: npm resolves to a different installation than node.\x1b[0m");
                println!("    node: {}", active_path.display());
                println!("    npm:  {}", npm.resolved_path.display());
            } else {
                println!("  ✓ node and npm align under the same installation environment.");
            }
        }
    } else if query_name == "python" {
        let (pip_res, _) = find_all_in_path("pip");
        if let Some(pip) = pip_res.first() {
            let py_dir_str = active_path.to_string_lossy().to_lowercase();
            let pip_dir_str = pip.resolved_path.to_string_lossy().to_lowercase();
            
            let both_scoop = py_dir_str.contains("scoop") && pip_dir_str.contains("scoop");
            let both_python_install = py_dir_str.contains("python") && pip_dir_str.contains("python");
            
            if !both_scoop && !both_python_install {
                println!("  \x1b[31m⚠ Conflict: pip resolves to a different installation than python.\x1b[0m");
                println!("    python: {}", active_path.display());
                println!("    pip:    {}", pip.resolved_path.display());
            } else {
                println!("  ✓ python and pip align under the same installation environment.");
            }
        }
    }
}

pub fn query_version(path: &Path) -> Option<String> {
    for flag in &["--version", "-v", "-V"] {
        if let Some(ver) = run_version_cmd(path, flag) {
            return Some(ver);
        }
    }
    None
}

fn run_version_cmd(path: &Path, flag: &str) -> Option<String> {
    let output = Command::new(path)
        .arg(flag)
        .output()
        .ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(v) = extract_version_from_text(&stdout) {
            return Some(v);
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        if let Some(v) = extract_version_from_text(&stderr) {
            return Some(v);
        }
    }
    None
}

fn extract_version_from_text(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '.');
        if clean.is_empty() {
            continue;
        }
        
        let first_char = clean.chars().next()?;
        let is_version_start = first_char.is_ascii_digit() 
            || (first_char == 'v' && clean.chars().nth(1).map_or(false, |c| c.is_ascii_digit()));
            
        if is_version_start {
            let dot_count = clean.chars().filter(|&c| c == '.').count();
            if dot_count >= 1 {
                return Some(clean.to_string());
            }
        }
    }
    None
}

pub fn detect_installation_source(path: &Path) -> InstallationInfo {
    let path_str = path.to_string_lossy().to_lowercase();
    
    if path_str.contains("scoop\\apps") {
        let parts: Vec<String> = path.components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect();
            
        if let Some(apps_idx) = parts.iter().position(|x| x.to_lowercase() == "apps") {
            if apps_idx + 1 < parts.len() {
                let app_name = &parts[apps_idx + 1];
                return InstallationInfo {
                    manager: "Scoop".to_string(),
                    detail: Some(format!("App directory: {}", app_name)),
                };
            }
        }
        return InstallationInfo {
            manager: "Scoop".to_string(),
            detail: None,
        };
    }
    
    if path_str.contains("fnm_multishells") || path_str.contains("fnm\\") {
        return InstallationInfo {
            manager: "fnm (Fast Node Manager)".to_string(),
            detail: Some("Managed Node.js environment".to_string()),
        };
    }

    if path_str.contains(".cargo\\bin") {
        return InstallationInfo {
            manager: "Cargo (Rust)".to_string(),
            detail: Some("Installed via 'cargo install'".to_string()),
        };
    }

    if path_str.contains("chocolatey\\bin") || path_str.contains("programdata\\chocolatey") {
        return InstallationInfo {
            manager: "Chocolatey".to_string(),
            detail: None,
        };
    }

    if path_str.contains("winget\\packages") || path_str.contains("microsoft\\winget") {
        return InstallationInfo {
            manager: "WinGet".to_string(),
            detail: None,
        };
    }

    InstallationInfo {
        manager: "System / Manual Installation".to_string(),
        detail: None,
    }
}

fn count_path_entries(path: &Path) -> usize {
    let parent_dir = match path.parent() {
        Some(p) => p.canonicalize().unwrap_or_else(|_| p.to_path_buf()),
        None => return 0,
    };
    
    let path_var = match env::var_os("PATH") {
        Some(p) => p,
        None => return 0,
    };
    
    let mut count = 0;
    for dir in env::split_paths(&path_var) {
        if let Ok(canon_dir) = dir.canonicalize() {
            if canon_dir == parent_dir {
                count += 1;
            }
        } else if dir == parent_dir {
            count += 1;
        }
    }
    count
}

fn scan_projects(query: &str) -> Vec<PathBuf> {
    let mut matching_projects = Vec::new();
    let home = env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Abcom".to_string());
    let projects_dir = Path::new(&home).join("Projects");
    
    if !projects_dir.is_dir() {
        return matching_projects;
    }
    
    let marker_files = match query {
        "node" | "npm" | "npx" => vec!["package.json"],
        "python" | "pip" => vec!["requirements.txt", "pyproject.toml", "pipfile"],
        "docker" => vec!["docker-compose.yml", "dockerfile", "docker-compose.yaml"],
        "cargo" | "rustc" | "rustup" | "rust" => vec!["cargo.toml"],
        _ => vec![],
    };
    
    if marker_files.is_empty() {
        return matching_projects;
    }
    
    if let Ok(entries) = fs::read_dir(&projects_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                for marker in &marker_files {
                    let marker_path = path.join(marker);
                    if marker_path.exists() {
                        matching_projects.push(path.clone());
                        break;
                    }
                }
            }
        }
    }
    
    matching_projects
}
