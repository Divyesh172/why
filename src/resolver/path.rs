use std::env;
use std::path::{Path, PathBuf};
use crate::resolver::shim::{is_scoop_shim, read_scoop_shim};

#[derive(Debug, Clone)]
pub struct PathSearchResult {
    pub original_path: PathBuf,
    pub resolved_path: PathBuf,
    pub is_shim: bool,
}

#[derive(Debug, Clone)]
pub struct SearchOrderEntry {
    pub dir: PathBuf,
    pub matches: Vec<PathBuf>,
}

/// Finds all matching instances of a query in the PATH, tracking search order.
pub fn find_all_in_path(query: &str) -> (Vec<PathSearchResult>, Vec<SearchOrderEntry>) {
    let mut results = Vec::new();
    let mut search_order = Vec::new();

    let query_path = Path::new(query);
    if query_path.is_absolute() || query_path.components().count() > 1 {
        if query_path.is_file() {
            let mut resolved = query_path.to_path_buf();
            let mut is_shim = false;
            if is_scoop_shim(&resolved) {
                if let Some(target) = read_scoop_shim(&resolved) {
                    resolved = target;
                    is_shim = true;
                }
            }
            results.push(PathSearchResult {
                original_path: query_path.to_path_buf(),
                resolved_path: resolved,
                is_shim,
            });
        }
        return (results, search_order);
    }

    let path_var = match env::var_os("PATH") {
        Some(p) => p,
        None => return (results, search_order),
    };
    let paths = env::split_paths(&path_var);

    let pathext_var = env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".to_string());
    let extensions: Vec<String> = pathext_var
        .split(';')
        .map(|s| s.to_uppercase())
        .collect();

    for dir in paths {
        let mut dir_matches = Vec::new();
        
        // 1. Try exact match
        let exact_path = dir.join(query);
        if exact_path.is_file() {
            dir_matches.push(exact_path);
        } else {
            // 2. Try extensions
            for ext in &extensions {
                let name_with_ext = format!("{}{}", query, ext.to_lowercase());
                let path_with_ext = dir.join(&name_with_ext);
                if path_with_ext.is_file() {
                    dir_matches.push(path_with_ext);
                    break;
                }
                
                let name_with_ext_up = format!("{}{}", query, ext);
                let path_with_ext_up = dir.join(&name_with_ext_up);
                if path_with_ext_up.is_file() {
                    dir_matches.push(path_with_ext_up);
                    break;
                }
            }
        }

        if !dir_matches.is_empty() {
            for match_path in &dir_matches {
                let mut resolved = match_path.clone();
                let mut is_shim = false;
                if is_scoop_shim(&resolved) {
                    if let Some(target) = read_scoop_shim(&resolved) {
                        resolved = target;
                        is_shim = true;
                    }
                }
                
                // Avoid duplicate resolved paths
                if !results.iter().any(|r| r.resolved_path == resolved) {
                    results.push(PathSearchResult {
                        original_path: match_path.clone(),
                        resolved_path: resolved,
                        is_shim,
                    });
                }
            }
        }

        search_order.push(SearchOrderEntry {
            dir: dir.clone(),
            matches: dir_matches,
        });
    }

    (results, search_order)
}
