use std::path::Path;
use std::process::Command;

/// Dynamically queries an executable's version by trying multiple version flags.
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
