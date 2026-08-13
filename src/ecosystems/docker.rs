use crate::resolver::path::find_all_in_path;

/// Prints associated Docker tools.
pub fn print_docker_details() {
    println!("\n\x1b[1mUsed by / Associated with:\x1b[0m");
    let tools = ["docker-compose", "dockerd"];
    for tool in &tools {
        let (resolved, _) = find_all_in_path(tool);
        if let Some(r) = resolved.first() {
            println!("  - {:<14} -> {}", tool, r.resolved_path.display());
        }
    }
}
