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
            println!("  {:<25} \x1b[33m⚠ [secret-like value hidden]\x1b[0m", key);
        } else if show_env {
            println!("  {:<25} = {}", key, val);
        } else {
            println!("  {:<25} \x1b[32m✓ [value hidden, use --show-env to display]\x1b[0m", key);
        }
    }
}

/// Detects if a variable name corresponds to a key, password, token, or auth credentials.
fn is_secret_key(key: &str) -> bool {
    let key_upper = key.to_uppercase();
    key_upper.contains("SECRET") 
        || key_upper.contains("KEY") 
        || key_upper.contains("PASSWORD") 
        || key_upper.contains("TOKEN") 
        || key_upper.contains("AUTH") 
        || key_upper.contains("PWD") 
        || key_upper.contains("PRIVATE")
        || key_upper.contains("CERT")
}
