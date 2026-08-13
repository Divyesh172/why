use std::env;
use std::fs;
use std::path::Path;
use crate::resolver::path::find_all_in_path;
use crate::platform::windows::find_services;
use crate::inspectors::executable::query_version;

/// Scans the current directory to diagnose project environment dependencies.
pub fn inspect_current_project() {
    let current_dir = match env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            println!("\x1b[31mError retrieving current directory: {}\x1b[0m", e);
            return;
        }
    };
    
    let project_name = current_dir.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());
        
    println!("\n\x1b[1m\x1b[36m=== Project Diagnostic: {} ===\x1b[0m", project_name);
    
    let mut files = Vec::new();
    let mut languages = Vec::new();
    let mut toolchain = Vec::new();
    let mut required_services = Vec::new();

    let check_files = [
        ("package.json", "JavaScript/TypeScript", "Node.js"),
        ("requirements.txt", "Python", "pip"),
        ("pyproject.toml", "Python", "Poetry/pip"),
        ("Cargo.toml", "Rust", "Cargo"),
        ("go.mod", "Go", "Go"),
        ("pom.xml", "Java", "Maven"),
        ("build.gradle", "Java", "Gradle"),
        ("docker-compose.yml", "Multi", "Docker Compose"),
        ("docker-compose.yaml", "Multi", "Docker Compose"),
        ("Dockerfile", "Multi", "Docker"),
    ];

    for (filename, lang, tool) in &check_files {
        let path = current_dir.join(filename);
        if path.is_file() {
            files.push(filename.to_string());
            if !languages.contains(&lang.to_string()) && *lang != "Multi" {
                languages.push(lang.to_string());
            }
            if !toolchain.contains(&tool.to_string()) {
                toolchain.push(tool.to_string());
            }

            // Docker Compose parsing
            if *filename == "docker-compose.yml" || *filename == "docker-compose.yaml" {
                if let Ok(content) = fs::read_to_string(path) {
                    let content_lower = content.to_lowercase();
                    if content_lower.contains("postgres") || content_lower.contains("postgresql") {
                        required_services.push("PostgreSQL".to_string());
                    }
                    if content_lower.contains("redis") {
                        required_services.push("Redis".to_string());
                    }
                    if content_lower.contains("mysql") {
                        required_services.push("MySQL".to_string());
                    }
                    if content_lower.contains("mongodb") || content_lower.contains("mongo") {
                        required_services.push("MongoDB".to_string());
                    }
                }
            }
        }
    }

    if files.is_empty() {
        println!("\nNo project files (package.json, pom.xml, requirements.txt, Cargo.toml, etc.) found in this directory.");
        println!("Run 'why project' inside a codebase directory.");
        return;
    }

    // Check if .env file exists and add to listing
    let env_file_path = current_dir.join(".env");
    let has_env = env_file_path.is_file();
    if has_env {
        files.push(".env".to_string());
    }

    println!("\n\x1b[1mLanguages:\x1b[0m\n  {}", languages.join(", "));
    println!("\n\x1b[1mToolchain:\x1b[0m\n  {}", toolchain.join(", "));
    
    println!("\n\x1b[1mFiles:\x1b[0m");
    for f in &files {
        let env_marker = if f == ".env" { " (found)" } else { "" };
        println!("  - {}{}", f, env_marker);
    }

    println!("\n\x1b[1mEnvironment Check:\x1b[0m");
    let mut check_failed = false;
    let mut issues = Vec::new();

    // 1. Verify Runtimes
    for lang in &languages {
        let exe_name = match lang.as_str() {
            "JavaScript/TypeScript" => "node",
            "Python" => "python",
            "Rust" => "cargo",
            "Go" => "go",
            "Java" => "java",
            _ => "",
        };

        if !exe_name.is_empty() {
            let (resolved, _) = find_all_in_path(exe_name);
            if let Some(res) = resolved.first() {
                // Verify package.json engines constraints for Node
                if lang == "JavaScript/TypeScript" {
                    if let Some(warn) = check_node_version_requirement(&current_dir, &res.resolved_path) {
                        println!("  \x1b[33m⚠ JavaScript/TypeScript\x1b[0m ({})", warn);
                        issues.push(warn);
                    } else {
                        println!("  \x1b[32m✓ JavaScript/TypeScript\x1b[0m");
                    }
                } else {
                    println!("  \x1b[32m✓ {}\x1b[0m", lang);
                }
            } else {
                println!("  \x1b[31m✗ {}\x1b[0m (executable '{}' not found)", lang, exe_name);
                check_failed = true;
                issues.push(format!("Missing runtime for {}: install '{}' and add it to your PATH.", lang, exe_name));
            }
        }
    }

    // 2. Verify Docker Toolchain
    if toolchain.contains(&"Docker Compose".to_string()) || toolchain.contains(&"Docker".to_string()) {
        let (resolved, _) = find_all_in_path("docker");
        if !resolved.is_empty() {
            println!("  \x1b[32m✓ Docker\x1b[0m");
        } else {
            println!("  \x1b[31m✗ Docker\x1b[0m (executable 'docker' not found)");
            check_failed = true;
            issues.push("Missing toolchain: Docker is required. Install Docker Desktop or Docker engine.".to_string());
        }
    }

    // 3. Verify Dependent Services
    for svc in &required_services {
        let matches = find_services(&svc.to_lowercase());
        let is_running = matches.iter().any(|s| s.contains("(Running)"));
        
        if is_running {
            println!("  \x1b[32m✓ {}\x1b[0m (local service is active)", svc);
        } else {
            println!("  \x1b[31m✗ {}\x1b[0m (no active local service found)", svc);
            check_failed = true;
            
            if files.contains(&"docker-compose.yml".to_string()) || files.contains(&"docker-compose.yaml".to_string()) {
                issues.push(format!(
                    "Service offline: {} is required by docker-compose, but the service is stopped or inactive.\n    \x1b[1mSuggested command:\x1b[0m docker compose up -d {}",
                    svc, svc.to_lowercase()
                ));
            } else {
                issues.push(format!("Service offline: local {} service is not active.", svc));
            }
        }
    }

    // 4. Verify Configuration (.env checks)
    if !required_services.is_empty() {
        let missing_env_vars = check_env_configuration(&env_file_path, &required_services);
        for var in &missing_env_vars {
            check_failed = true;
            issues.push(format!("Configuration error: {} is missing in .env", var));
        }
    }

    if check_failed {
        println!("\n\x1b[33m\x1b[1m⚠ Project may not start.\x1b[0m");
        println!("\n\x1b[1mReason(s):\x1b[0m");
        for iss in &issues {
            println!("  - {}", iss);
        }
    } else {
        println!("\n\x1b[32m\x1b[1m✓ Environment looks healthy. Ready to build/start!\x1b[0m");
    }
    println!();
}

fn check_env_configuration(env_path: &Path, services: &[String]) -> Vec<String> {
    let mut missing = Vec::new();
    let content = match fs::read_to_string(env_path) {
        Ok(c) => c,
        Err(_) => {
            for svc in services {
                if svc == "PostgreSQL" || svc == "MySQL" {
                    missing.push("DATABASE_URL".to_string());
                } else if svc == "MongoDB" {
                    missing.push("MONGO_URI".to_string());
                } else if svc == "Redis" {
                    missing.push("REDIS_URL".to_string());
                }
            }
            return missing;
        }
    };

    for svc in services {
        let expected_key = match svc.as_str() {
            "PostgreSQL" | "MySQL" => "DATABASE_URL",
            "Redis" => "REDIS_URL",
            "MongoDB" => "MONGO_URI",
            _ => "",
        };

        if !expected_key.is_empty() {
            let key_pattern = format!("{}=", expected_key);
            let has_key = content.lines().any(|l| l.trim().starts_with(&key_pattern));
            if !has_key {
                missing.push(expected_key.to_string());
            }
        }
    }

    missing
}

fn check_node_version_requirement(project_dir: &Path, node_path: &Path) -> Option<String> {
    let pkg_json_path = project_dir.join("package.json");
    let content = fs::read_to_string(pkg_json_path).ok()?;
    
    let engines_idx = content.find("\"engines\"")?;
    let content_after = &content[engines_idx..];
    let node_idx = content_after.find("\"node\"")?;
    let line_after_node = &content_after[node_idx..];
    
    let start_quote = line_after_node.find(':')? + 1;
    let rest = &line_after_node[start_quote..];
    let val_start = rest.find('"')? + 1;
    let val_end = rest[val_start..].find('"')? + val_start;
    let req_version = rest[val_start..val_end].trim();

    let active_version = query_version(node_path)?;
    let clean_active = active_version.trim_start_matches('v');
    
    if req_version.starts_with(">=") {
        let req_num_str = req_version.trim_start_matches(">=").trim();
        let req_major = req_num_str.split('.').next()?.parse::<u32>().ok()?;
        let active_major = clean_active.split('.').next()?.parse::<u32>().ok()?;
        if active_major < req_major {
            return Some(format!(
                "Node version mismatch: active is {} but package.json requires {}",
                active_version, req_version
            ));
        }
    } else if req_version.starts_with('^') {
        let req_num_str = req_version.trim_start_matches('^').trim();
        let req_major = req_num_str.split('.').next()?.parse::<u32>().ok()?;
        let active_major = clean_active.split('.').next()?.parse::<u32>().ok()?;
        if active_major != req_major {
            return Some(format!(
                "Node version mismatch: active is {} but package.json requires {}",
                active_version, req_version
            ));
        }
    }
    None
}
