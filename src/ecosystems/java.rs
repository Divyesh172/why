#![allow(dead_code)]`nuse crate::resolver::path::find_all_in_path;

/// Prints associated Java tools.
pub fn print_java_details() {
    println!("\n\x1b[1mUsed by / Associated with:\x1b[0m");
    let tools = ["javac", "jar", "mvn", "gradle", "kotlin"];
    for tool in &tools {
        let (resolved, _) = find_all_in_path(tool);
        if let Some(r) = resolved.first() {
            println!("  - {:<8} -> {}", tool, r.resolved_path.display());
        }
    }
}

