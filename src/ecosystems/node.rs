#![allow(dead_code)]`nuse std::env;
use std::fs;
use std::path::Path;
use crate::resolver::path::find_all_in_path;

/// Prints associated Node tools and global package counts.
pub fn print_node_details() {
    println!("\n\x1b[1mUsed by / Associated with:\x1b[0m");
    let tools = ["npm", "npx", "yarn", "pnpm"];
    for tool in &tools {
        let (resolved, _) = find_all_in_path(tool);
        if let Some(r) = resolved.first() {
            println!("  - {:<6} -> {}", tool, r.resolved_path.display());
        }
    }
    if let Some(count) = count_npm_global_packages() {
        println!("\n\x1b[1mGlobal packages:\x1b[0m");
        println!("  {}", count);
    }
}

fn count_npm_global_packages() -> Option<usize> {
    let appdata = env::var("APPDATA").ok()?;
    let npm_global = Path::new(&appdata).join("npm").join("node_modules");
    if npm_global.is_dir() {
        if let Ok(entries) = fs::read_dir(npm_global) {
            let count = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .count();
            return Some(count);
        }
    }
    None
}

