#![allow(dead_code)]
use crate::resolver::path::find_all_in_path;

/// Prints associated Python tools (pip, poetry, etc.) and where they resolve.
pub fn print_python_details() {
    println!("\n\x1b[1mUsed by / Associated with:\x1b[0m");
    let tools = ["pip", "poetry", "pipenv", "conda", "uv"];
    for tool in &tools {
        let (resolved, _) = find_all_in_path(tool);
        if let Some(r) = resolved.first() {
            println!("  - {:<8} -> {}", tool, r.resolved_path.display());
        }
    }
}

