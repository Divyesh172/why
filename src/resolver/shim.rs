use std::fs;
use std::path::{Path, PathBuf};

/// Checks if the path is a Scoop shim executable.
pub fn is_scoop_shim(path: &Path) -> bool {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if extension.to_lowercase() != "exe" {
        return false;
    }
    let shim_path = path.with_extension("shim");
    shim_path.is_file()
}

/// Reads a Scoop `.shim` file and returns the target file path.
pub fn read_scoop_shim(path: &Path) -> Option<PathBuf> {
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
