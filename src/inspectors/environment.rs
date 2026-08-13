#![allow(dead_code)]
use std::env;

/// Scans environment variables matching the query.
pub fn scan_env_variables(query: &str) -> Vec<(String, String)> {
    let mut matches = Vec::new();
    let query_upper = query.to_uppercase();
    
    for (key, val) in env::vars() {
        if key.to_uppercase().contains(&query_upper) {
            matches.push((key, val));
        }
    }
    matches
}

/// Prints environment variables with security masking.
pub fn print_env_report(query: &str, show_env: bool) {
    let matches = scan_env_variables(query);
    if matches.is_empty() {
        println!("  \x1b[90m(none found)\x1b[0m");
        return;
    }

    for (key, val) in matches {
        let is_secret = is_secret_key(&key);
        if is_secret {
            println!("  {:<25} \x1b[33m⚠ [secret masked: ********]\x1b[0m", key);
        } else if show_env {
            println!("  {:<25} = {}", key, val);
        } else {
            println!("  {:<25} \x1b[32m✓ [value hidden, use --show-env to display]\x1b[0m", key);
        }
    }
}

/// Explicitly prints a single environment variable, unmasking it.
pub fn print_single_env(name: &str) {
    let name_upper = name.to_uppercase();
    match env::var(&name_upper) {
        Ok(val) => {
            let is_secret = is_secret_key(&name_upper);
            println!("\n\x1b[1m\x1b[36m=== Environment Variable: {} ===\x1b[0m", name_upper);
            if is_secret {
                println!("  Value: \x1b[33m{}\x1b[0m (Unmasked via explicit query)", val);
            } else {
                println!("  Value: {}", val);
            }
            println!();
        }
        Err(_) => {
            println!("\n\x1b[31m⚠ Environment variable '{}' not found.\x1b[0m", name_upper);
        }
    }
}

/// Detects if a variable name corresponds to a key, password, token, or auth credentials.
pub fn is_secret_key(key: &str) -> bool {
    let key_upper = key.to_uppercase();
    key_upper.contains("SECRET") 
        || key_upper.contains("KEY") 
        || key_upper.contains("PASSWORD") 
        || key_upper.contains("TOKEN") 
        || key_upper.contains("AUTH") 
        || key_upper.contains("PWD") 
        || key_upper.contains("PRIVATE")
        || key_upper.contains("CERT")
        || key_upper.contains("DATABASE_URL")
        || key_upper.contains("CONN")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_secret_key() {
        assert!(is_secret_key("GITHUB_TOKEN"));
        assert!(is_secret_key("AWS_SECRET_ACCESS_KEY"));
        assert!(is_secret_key("DB_PASSWORD"));
        assert!(is_secret_key("DATABASE_URL"));
        assert!(!is_secret_key("PATH"));
        assert!(!is_secret_key("NODE_ENV"));
    }
}

