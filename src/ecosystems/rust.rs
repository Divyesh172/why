use crate::resolver::path::find_all_in_path;

/// Prints associated Rust tools.
pub fn print_rust_details() {
    println!("\n\x1b[1mUsed by / Associated with:\x1b[0m");
    let tools = ["cargo", "rustc", "rustup"];
    for tool in &tools {
        let (resolved, _) = find_all_in_path(tool);
        if let Some(r) = resolved.first() {
            println!("  - {:<8} -> {}", tool, r.resolved_path.display());
        }
    }
}
