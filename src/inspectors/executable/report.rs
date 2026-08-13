use std::env;
use std::path::Path;
use crate::resolver::path::{find_all_in_path, PathSearchResult, SearchOrderEntry};
use crate::ecosystems::print_ecosystem_details;
use crate::inspectors::environment::print_env_report;
use crate::inspectors::services::print_services_report;

use super::version::query_version;
use super::installation::detect_installation_source;
use super::conflict::{print_conflict_diagnostics, share_installation_root};
use super::projects::scan_projects;

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const GRAY: &str = "\x1b[90m";

/// Prints a smart analysis report for the matched executable.
pub fn print_executable_report(query: &str, all: bool, conflict: bool, show_env: bool, json: bool) {
    let query_clean = query.trim_end_matches(".exe").to_lowercase();
    let (all_results, search_order) = find_all_in_path(query);

    if all_results.is_empty() {
        if json {
            println!("{{}}");
        } else {
            println!("\n{}{}Could not resolve executable '{}' in system PATH.{}", BOLD, YELLOW, query, RESET);
        }
        return;
    }

    if json {
        print_executable_json(&query_clean, &all_results);
        return;
    }

    let app_display_name = match query_clean.as_str() {
        "node" => "Node.js",
        "python" => "Python",
        "docker" => "Docker",
        "rustc" | "cargo" | "rustup" => "Rust",
        "java" | "javac" => "Java",
        other => other,
    };

    if all {
        print_all_installations(&query_clean, app_display_name, &all_results);
        return;
    }

    let active = &all_results[0];

    println!("\n{}{}=== Smart System Report: {} ==={}", BOLD, CYAN, app_display_name, RESET);
    
    println!("\n{}Executable:{}", BOLD, RESET);
    println!("  {}", active.resolved_path.display());
    if active.is_shim {
        println!("  {}(resolved from Scoop shim: {}){}", GRAY, active.original_path.display(), RESET);
    }

    println!("\n{}Version:{}", BOLD, RESET);
    match query_version(&active.resolved_path) {
        Some(v) => println!("  {}{}{}", GREEN, v, RESET),
        None => println!("  {}(could not retrieve version){}", GRAY, RESET),
    }

    println!("\n{}Installed through:{}", BOLD, RESET);
    let install_info = detect_installation_source(&active.resolved_path);
    print!("  {}{}{}", GREEN, install_info.manager, RESET);
    if let Some(detail) = install_info.detail {
        print!(" - {}{}{}", GRAY, detail, RESET);
    }
    println!();

    print_ecosystem_details(&query_clean);

    println!("\n{}PATH entries:{}", BOLD, RESET);
    let path_entries = count_path_entries(&active.resolved_path);
    println!("  {} entry(ies) in system PATH point here", path_entries);

    println!("\n{}Why does `{}` resolve here?{}", BOLD, query, RESET);
    print_path_search_order(query, &search_order);

    println!("\n{}Environment variables:{}", BOLD, RESET);
    print_env_report(&query_clean, show_env);

    println!("\n{}Projects currently using it:{}", BOLD, RESET);
    let projects = scan_projects(&query_clean);
    if projects.is_empty() {
        println!("  {}(none found in ~/Projects){}", GRAY, RESET);
    } else {
        for proj in projects {
            let display_path = if let Ok(profile) = env::var("USERPROFILE") {
                proj.to_string_lossy().replace(&profile, "~")
            } else {
                proj.to_string_lossy().to_string()
            };
            println!("  {}", display_path);
        }
    }

    println!("\n{}Services depending on/related to it:{}", BOLD, RESET);
    print_services_report(&query_clean);

    if all_results.len() > 1 {
        println!("\n{}⚠ Multiple installations detected. Run 'why {} --all' for details.{}", YELLOW, query, RESET);
    }

    if conflict {
        print_conflict_diagnostics(&query_clean, &active.resolved_path, &all_results);
    }
}

fn print_all_installations(_query_name: &str, display_name: &str, results: &[PathSearchResult]) {
    println!("\n{}{}=== Found {} {} installations ==={}", BOLD, CYAN, results.len(), display_name, RESET);
    
    for (i, res) in results.iter().enumerate() {
        let active_label = if i == 0 {
            format!(" {}{}[ACTIVE]{}", BOLD, GREEN, RESET)
        } else {
            "".to_string()
        };
        
        println!("\n{}{}. Installation:{}", BOLD, i + 1, active_label);
        println!("  Path:    {}", res.resolved_path.display());
        if res.is_shim {
            println!("  Shim:    {}", res.original_path.display());
        }
        
        match query_version(&res.resolved_path) {
            Some(v) => println!("  Version: {}{}{}", GREEN, v, RESET),
            None => println!("  Version: {}(could not retrieve version){}", GRAY, RESET),
        }
        
        let install_info = detect_installation_source(&res.resolved_path);
        print!("  Source:  {}", install_info.manager);
        if let Some(detail) = install_info.detail {
            print!(" ({})", detail);
        }
        println!();
    }
    
    if results.len() > 1 {
        println!("\n{}⚠ Multiple installations detected may cause path conflicts.{}", YELLOW, RESET);
    }
}

fn print_path_search_order(_query: &str, search_order: &[SearchOrderEntry]) {
    let mut matched = false;
    let mut printed_count = 0;
    for entry in search_order {
        if !entry.matches.is_empty() {
            println!("  \x1b[32m-> {} ← MATCH ({})\x1b[0m", entry.dir.display(), entry.matches[0].file_name().unwrap_or_default().to_string_lossy());
            matched = true;
            break;
        } else {
            if printed_count < 3 {
                println!("  - {}", entry.dir.display());
                printed_count += 1;
            } else if printed_count == 3 {
                println!("  - ... (remaining directories skipped)");
                printed_count += 1;
            }
        }
    }
    if !matched {
        println!("  (none matched)");
    }
}

fn print_executable_json(query: &str, results: &[PathSearchResult]) {
    if results.is_empty() {
        println!("{{}}");
        return;
    }
    
    let active = &results[0];
    let version = query_version(&active.resolved_path).unwrap_or_default();
    let install_info = detect_installation_source(&active.resolved_path);
    let installations_count = results.len();
    
    let mut conflicts = Vec::new();
    if installations_count > 1 {
        conflicts.push(format!("\"Multiple {} installations detected\"", query));
    }
    
    if query == "node" {
        let (npm_res, _) = find_all_in_path("npm");
        if let Some(npm) = npm_res.first() {
            if !share_installation_root(&active.resolved_path, &npm.resolved_path) {
                conflicts.push("\"npm resolves to a different installation than node\"".to_string());
            }
        }
    } else if query == "python" {
        let (pip_res, _) = find_all_in_path("pip");
        if let Some(pip) = pip_res.first() {
            if !share_installation_root(&active.resolved_path, &pip.resolved_path) {
                conflicts.push("\"pip resolves to a different installation than python\"".to_string());
            }
        }
    }
    
    println!("{{");
    println!("  \"name\": \"{}\",", query);
    println!("  \"resolved\": \"{}\",", active.resolved_path.display().to_string().replace('\\', "\\\\"));
    if active.is_shim {
        println!("  \"shim\": \"{}\",", active.original_path.display().to_string().replace('\\', "\\\\"));
    }
    println!("  \"version\": \"{}\",", version);
    println!("  \"manager\": \"{}\",", install_info.manager.to_lowercase());
    println!("  \"installations\": {},", installations_count);
    println!("  \"conflicts\": [{}]", conflicts.join(", "));
    println!("}}");
}

fn count_path_entries(path: &Path) -> usize {
    let parent_dir = match path.parent() {
        Some(p) => p.canonicalize().unwrap_or_else(|_| p.to_path_buf()),
        None => return 0,
    };
    
    let path_var = match env::var_os("PATH") {
        Some(p) => p,
        None => return 0,
    };
    
    let mut count = 0;
    for dir in env::split_paths(&path_var) {
        if let Ok(canon_dir) = dir.canonicalize() {
            if canon_dir == parent_dir {
                count += 1;
            }
        } else if dir == parent_dir {
            count += 1;
        }
    }
    count
}
