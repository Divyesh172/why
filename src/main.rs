use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// Premium ANSI color formatting
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const GRAY: &str = "\x1b[90m";

#[derive(Debug)]
struct InstallationInfo {
    manager: String,
    detail: Option<String>,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("{}why{} - System Inspector Tool", BOLD, RESET);
        println!("Usage: why <program>");
        std::process::exit(0);
    }

    let query = &args[1];
    let query_clean = query.trim_end_matches(".exe").to_lowercase();

    match resolve_executable(query) {
        Some(path) => {
            print_report(&query_clean, &path);
        }
        None => {
            println!("\n{}{}Could not resolve executable '{}' in system PATH.{}", BOLD, YELLOW, query, RESET);
        }
    }
}

/// Resolves the true executable path of a command.
fn resolve_executable(query: &str) -> Option<PathBuf> {
    let raw_path = find_in_path(query)?;
    
    // If it's a Scoop shim, resolve the actual target path
    if is_scoop_shim(&raw_path) {
        if let Some(resolved) = read_scoop_shim(&raw_path) {
            return Some(resolved);
        }
    }
    
    Some(raw_path)
}

/// Searches the directories in PATH for the query.
fn find_in_path(query: &str) -> Option<PathBuf> {
    let query_path = Path::new(query);
    if query_path.is_absolute() || query_path.components().count() > 1 {
        if query_path.is_file() {
            return Some(query_path.to_path_buf());
        }
        return None;
    }

    let path_var = env::var_os("PATH")?;
    let paths = env::split_paths(&path_var);

    let pathext_var = env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".to_string());
    let extensions: Vec<String> = pathext_var
        .split(';')
        .map(|s| s.to_uppercase())
        .collect();

    for dir in paths {
        let exact_path = dir.join(query);
        if exact_path.is_file() {
            return Some(exact_path);
        }

        for ext in &extensions {
            let name_with_ext = format!("{}{}", query, ext.to_lowercase());
            let path_with_ext = dir.join(&name_with_ext);
            if path_with_ext.is_file() {
                return Some(path_with_ext);
            }
            
            let name_with_ext_up = format!("{}{}", query, ext);
            let path_with_ext_up = dir.join(&name_with_ext_up);
            if path_with_ext_up.is_file() {
                return Some(path_with_ext_up);
            }
        }
    }

    None
}

/// Checks if the path is a Scoop shim executable.
fn is_scoop_shim(path: &Path) -> bool {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if extension.to_lowercase() != "exe" {
        return false;
    }
    let shim_path = path.with_extension("shim");
    shim_path.is_file()
}

/// Reads a Scoop `.shim` file and returns the target file path.
fn read_scoop_shim(path: &Path) -> Option<PathBuf> {
    let shim_path = path.with_extension("shim");
    let content = fs::read_to_string(shim_path).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("path") {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                let target_val = parts[1].trim().trim_matches('"');
                let target_path = PathBuf::from(target_val);
                if target_path.is_file() {
                    return Some(target_path);
                }
            }
        }
    }
    None
}

/// Inspects the file path to determine the installation source.
fn detect_installation_source(path: &Path) -> InstallationInfo {
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

/// Executes the binary with flags to extract the version.
fn query_version(path: &Path) -> Option<String> {
    for flag in &["--version", "-v", "-V"] {
        if let Some(ver) = run_version_cmd(path, flag) {
            return Some(ver);
        }
    }
    None
}

/// Runs the executable with a given flag and searches stdout/stderr for a version.
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

/// Helper to parse version string (e.g., "1.2.3" or "v12.4") from arbitrary output text.
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

/// Count how many entries in PATH point to the directory of this executable.
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

/// Scans environment variables matching the query.
fn scan_env_variables(query: &str) -> Vec<(String, String)> {
    let mut matches = Vec::new();
    let query_upper = query.to_uppercase();
    
    for (key, val) in env::vars() {
        if key.to_uppercase().contains(&query_upper) {
            matches.push((key, val));
        }
    }
    matches
}

/// Count global npm packages by scanning npm's global root directory.
fn count_npm_global_packages() -> Option<usize> {
    let appdata = env::var("APPDATA").ok()?;
    let npm_global = Path::new(&appdata).join("npm").join("node_modules");
    if npm_global.is_dir() {
        if let Ok(entries) = fs::read_dir(npm_global) {
            let count = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .count();
            return Some(count);
        }
    }
    None
}

/// Print ecosystem-specific details.
fn print_ecosystem_details(query_name: &str) {
    match query_name {
        "node" | "npm" | "npx" => {
            println!("\n{}Used by / Associated with:{}", BOLD, RESET);
            let tools = ["npm", "npx", "yarn", "pnpm"];
            for tool in &tools {
                if let Some(p) = resolve_executable(tool) {
                    println!("  - {:<6} -> {}", tool, p.display());
                }
            }
            if let Some(count) = count_npm_global_packages() {
                println!("\n{}Global packages:{}", BOLD, RESET);
                println!("  {}", count);
            }
        }
        "python" | "pip" => {
            println!("\n{}Used by / Associated with:{}", BOLD, RESET);
            let tools = ["pip", "poetry", "pipenv", "conda", "uv"];
            for tool in &tools {
                if let Some(p) = resolve_executable(tool) {
                    println!("  - {:<8} -> {}", tool, p.display());
                }
            }
        }
        "docker" => {
            println!("\n{}Used by / Associated with:{}", BOLD, RESET);
            let tools = ["docker-compose", "dockerd"];
            for tool in &tools {
                if let Some(p) = resolve_executable(tool) {
                    println!("  - {:<14} -> {}", tool, p.display());
                }
            }
        }
        "cargo" | "rustc" | "rustup" => {
            println!("\n{}Used by / Associated with:{}", BOLD, RESET);
            let tools = ["cargo", "rustc", "rustup"];
            for tool in &tools {
                if let Some(p) = resolve_executable(tool) {
                    println!("  - {:<8} -> {}", tool, p.display());
                }
            }
        }
        _ => {}
    }
}

/// Scan subdirectories of ~/Projects for workspace configuration files indicating tool usage.
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

/// Queries Windows services to find matches.
fn find_services(query: &str) -> Vec<String> {
    let mut services = Vec::new();
    let filter = format!("*{}*", query);
    let cmd = format!(
        "Get-Service -Name '{}' | ForEach-Object {{ `$_.Name + ' (' + `$_.Status + ')' }}",
        filter
    );
    
    let output = Command::new("powershell")
        .args(&["-NoProfile", "-Command", &cmd])
        .output();
        
    if let Ok(out) = output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    services.push(trimmed.to_string());
                }
            }
        }
    }
    
    services
}

/// Print the complete formatted analysis report.
fn print_report(query_name: &str, path: &Path) {
    let app_display_name = match query_name {
        "node" => "Node.js",
        "python" => "Python",
        "docker" => "Docker",
        "rustc" | "cargo" | "rustup" => "Rust",
        other => other,
    };

    println!("\n{}{}=== Smart System Report: {} ==={}", BOLD, CYAN, app_display_name, RESET);
    
    println!("\n{}Executable:{}", BOLD, RESET);
    println!("  {}", path.display());

    println!("\n{}Version:{}", BOLD, RESET);
    match query_version(path) {
        Some(v) => println!("  {}{}{}", GREEN, v, RESET),
        None => println!("  {}(could not retrieve version){}", GRAY, RESET),
    }

    println!("\n{}Installed through:{}", BOLD, RESET);
    let install_info = detect_installation_source(path);
    print!("  {}{}{}", GREEN, install_info.manager, RESET);
    if let Some(detail) = install_info.detail {
        print!(" - {}{}{}", GRAY, detail, RESET);
    }
    println!();

    print_ecosystem_details(query_name);

    println!("\n{}PATH entries:{}", BOLD, RESET);
    let path_entries = count_path_entries(path);
    println!("  {} entry(ies) in system PATH point here", path_entries);

    println!("\n{}Environment variables:{}", BOLD, RESET);
    let env_vars = scan_env_variables(query_name);
    if env_vars.is_empty() {
        println!("  {}(none found){}", GRAY, RESET);
    } else {
        for (key, val) in env_vars {
            println!("  {} = {}", key, val);
        }
    }

    println!("\n{}Projects currently using it:{}", BOLD, RESET);
    let projects = scan_projects(query_name);
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
    let services = find_services(query_name);
    if services.is_empty() {
        println!("  {}none{}", GRAY, RESET);
    } else {
        for service in services {
            if service.contains("(Running)") {
                println!("  {}", service.replace("(Running)", &format!("{}(Running){}", GREEN, RESET)));
            } else if service.contains("(Stopped)") {
                println!("  {}", service.replace("(Stopped)", &format!("{}(Stopped){}", GRAY, RESET)));
            } else {
                println!("  {}", service);
            }
        }
    }
    println!();
}
