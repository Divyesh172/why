#![allow(dead_code)]`nuse std::path::Path;
use crate::resolver::path::{find_all_in_path, PathSearchResult};

/// Checks if two files share a common installation folder hierarchy.
pub fn share_installation_root(path_a: &Path, path_b: &Path) -> bool {
    let a = match path_a.canonicalize() {
        Ok(p) => p,
        Err(_) => path_a.to_path_buf(),
    };
    let b = match path_b.canonicalize() {
        Ok(p) => p,
        Err(_) => path_b.to_path_buf(),
    };

    if let (Some(pa), Some(pb)) = (a.parent(), b.parent()) {
        if pa == pb {
            return true;
        }
        if let Some(gpa) = pa.parent() {
            if gpa == pb || Some(gpa) == pb.parent() {
                return true;
            }
        }
        if let Some(gpb) = pb.parent() {
            if gpb == pa || Some(gpb) == pa.parent() {
                return true;
            }
        }
    }
    false
}

/// Evaluates PATH conflicts for multiple binaries.
pub fn print_conflict_diagnostics(query_name: &str, active_path: &Path, all_results: &[PathSearchResult]) {
    const BOLD: &str = "\x1b[1m";
    const YELLOW: &str = "\x1b[33m";
    const RESET: &str = "\x1b[0m";
    const RED: &str = "\x1b[31m";

    println!("\n{}{}=== Conflict Diagnostics ==={}", BOLD, YELLOW, RESET);
    
    if all_results.len() > 1 {
        println!("  \x1b[33m⚠ Multiple {} installations detected on PATH:\x1b[0m", query_name);
        for res in all_results {
            println!("    - {}", res.resolved_path.display());
        }
    } else {
        println!("  ✓ No multiple installations found on PATH.");
    }

    if query_name == "node" {
        let (npm_res, _) = find_all_in_path("npm");
        if let Some(npm) = npm_res.first() {
            if !share_installation_root(active_path, &npm.resolved_path) {
                println!("  {}⚠ Conflict: npm resolves to a different installation than node.{}", RED, RESET);
                println!("    node: {}", active_path.display());
                println!("    npm:  {}", npm.resolved_path.display());
            } else {
                println!("  ✓ node and npm align under the same installation environment.");
            }
        }
    } else if query_name == "python" {
        let (pip_res, _) = find_all_in_path("pip");
        if let Some(pip) = pip_res.first() {
            if !share_installation_root(active_path, &pip.resolved_path) {
                println!("  {}⚠ Conflict: pip resolves to a different installation than python.{}", RED, RESET);
                println!("    python: {}", active_path.display());
                println!("    pip:    {}", pip.resolved_path.display());
            } else {
                println!("  ✓ python and pip align under the same installation environment.");
            }
        }
    }
}

