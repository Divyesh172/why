use std::env;
use std::fs;
use std::path::PathBuf;

/// Scans the Projects directory for configurations using the queried runtime.
pub fn scan_projects(query: &str) -> Vec<PathBuf> {
    let mut matching_projects = Vec::new();
    let home = env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Abcom".to_string());
    let projects_dir = std::path::Path::new(&home).join("Projects");
    
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
