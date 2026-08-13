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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::env;

    #[test]
    fn test_scoop_shim_parsing() {
        let temp_dir = env::temp_dir();
        let exe_path = temp_dir.join("test_command.exe");
        let shim_path = temp_dir.join("test_command.shim");

        let mut exe_file = File::create(&exe_path).unwrap();
        exe_file.write_all(b"mock exe").unwrap();

        let target_dir = temp_dir.join("MockPath");
        let _ = std::fs::create_dir_all(&target_dir);
        let target_path = target_dir.join("target.exe");
        let mut target_file = File::create(&target_path).unwrap();
        target_file.write_all(b"mock target").unwrap();

        let mut shim_file = File::create(&shim_path).unwrap();
        let shim_content = format!("path = \"{}\"\n", target_path.display());
        shim_file.write_all(shim_content.as_bytes()).unwrap();

        assert!(is_scoop_shim(&exe_path));

        let resolved = read_scoop_shim(&exe_path).unwrap();
        assert_eq!(resolved.canonicalize().unwrap(), target_path.canonicalize().unwrap());

        let _ = std::fs::remove_file(exe_path);
        let _ = std::fs::remove_file(shim_path);
        let _ = std::fs::remove_file(target_path);
        let _ = std::fs::remove_dir(target_dir);
    }
}
