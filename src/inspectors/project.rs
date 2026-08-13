use std::env;
use std::fs;
use std::path::Path;
use crate::resolver::path::find_all_in_path;
use crate::platform::find_services;
use crate::inspectors::executable::query_version;

/// Scans the current directory to diagnose project environment dependencies using a structured tree.
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

    let env_file_path = current_dir.join(".env");
    let has_env = env_file_path.is_file();

    let mut check_failed = false;
    let mut reasons = Vec::new();

    println!("\n\x1b[1mProject: {}\x1b[0m", project_name);

    // 1. Node.js check
    if files.contains(&"package.json".to_string()) {
        println!("\nNode.js");
        println!(" ├─ required by package.json");
        
        let (resolved, _) = find_all_in_path("node");
        if let Some(res) = resolved.first() {
            let active_ver = query_version(&res.resolved_path).unwrap_or_else(|| "unknown".to_string());
            let constraint = check_node_version_requirement(&current_dir, &res.resolved_path);
            
            match constraint {
                Some(ref_err) => {
                    println!(" ├─ active version: {}", active_ver);
                    println!(" └─ \x1b[33m⚠ VERSION MISMATCH: {}\x1b[0m", ref_err);
                    check_failed = true;
                    reasons.push(format!("Node.js version conflict: {}", ref_err));
                }
                None => {
                    println!(" ├─ active version: {}", active_ver);
                    println!(" └─ \x1b[32m✓ available\x1b[0m");
                }
            }
        } else {
            println!(" └─ \x1b[31m✗ node executable not found on PATH\x1b[0m");
            check_failed = true;
            reasons.push("Missing runtime: Node.js is required but not installed.".to_string());
        }
    }

    // 2. Python check
    let py_file = ["requirements.txt", "pyproject.toml", "pipfile"].iter()
        .find(|f| files.contains(&f.to_string()));
    if let Some(filename) = py_file {
        println!("\nPython");
        println!(" ├─ required by {}", filename);
        let (resolved, _) = find_all_in_path("python");
        if let Some(res) = resolved.first() {
            let active_ver = query_version(&res.resolved_path).unwrap_or_else(|| "unknown".to_string());
            println!(" ├─ active version: {}", active_ver);
            println!(" └─ \x1b[32m✓ available\x1b[0m");
        } else {
            println!(" └─ \x1b[31m✗ python executable not found on PATH\x1b[0m");
            check_failed = true;
            reasons.push("Missing runtime: Python is required but not installed.".to_string());
        }
    }

    // 3. Rust check
    if files.contains(&"Cargo.toml".to_string()) {
        println!("\nRust");
        println!(" ├─ required by Cargo.toml");
        let (resolved, _) = find_all_in_path("cargo");
        if let Some(res) = resolved.first() {
            let active_ver = query_version(&res.resolved_path).unwrap_or_else(|| "unknown".to_string());
            println!(" ├─ active version: {}", active_ver);
            println!(" └─ \x1b[32m✓ available\x1b[0m");
        } else {
            println!(" └─ \x1b[31m✗ cargo executable not found on PATH\x1b[0m");
            check_failed = true;
            reasons.push("Missing toolchain: Rust (cargo) is required but not installed.".to_string());
        }
    }

    // 4. Go check
    if files.contains(&"go.mod".to_string()) {
        println!("\nGo");
        println!(" ├─ required by go.mod");
        let (resolved, _) = find_all_in_path("go");
        if let Some(res) = resolved.first() {
            let active_ver = query_version(&res.resolved_path).unwrap_or_else(|| "unknown".to_string());
            println!(" ├─ active version: {}", active_ver);
            println!(" └─ \x1b[32m✓ available\x1b[0m");
        } else {
            println!(" └─ \x1b[31m✗ go executable not found on PATH\x1b[0m");
            check_failed = true;
            reasons.push("Missing runtime: Go is required but not installed.".to_string());
        }
    }

    // 5. Java check
    let java_file = ["pom.xml", "build.gradle"].iter()
        .find(|f| files.contains(&f.to_string()));
    if let Some(filename) = java_file {
        println!("\nJava");
        println!(" ├─ required by {}", filename);
        let (resolved, _) = find_all_in_path("java");
        if let Some(res) = resolved.first() {
            let active_ver = query_version(&res.resolved_path).unwrap_or_else(|| "unknown".to_string());
            println!(" ├─ active version: {}", active_ver);
            println!(" └─ \x1b[32m✓ available\x1b[0m");
        } else {
            println!(" └─ \x1b[31m✗ java executable not found on PATH\x1b[0m");
            check_failed = true;
            reasons.push("Missing runtime: Java (JDK) is required but not installed.".to_string());
        }
    }

    // 6. Docker check
    if toolchain.contains(&"Docker Compose".to_string()) || toolchain.contains(&"Docker".to_string()) {
        println!("\nDocker");
        println!(" ├─ required by project files");
        let (resolved, _) = find_all_in_path("docker");
        if !resolved.is_empty() {
            println!(" └─ \x1b[32m✓ available\x1b[0m");
        } else {
            println!(" └─ \x1b[31m✗ docker executable not found on PATH\x1b[0m");
            check_failed = true;
            reasons.push("Missing toolchain: Docker is required but not installed.".to_string());
        }
    }

    // 7. Services check
    for svc in &required_services {
        println!("\n{}", svc);
        let filename = if files.contains(&"docker-compose.yml".to_string()) {
            "docker-compose.yml"
        } else {
            "docker-compose.yaml"
        };
        println!(" ├─ required by {}", filename);
        
        let port = match svc.as_str() {
            "PostgreSQL" => 5432,
            "MySQL" => 3306,
            "Redis" => 6379,
            "MongoDB" => 27017,
            _ => 0,
        };
        
        if port != 0 {
            println!(" ├─ port: {}", port);
        }

        let matches = find_services(&svc.to_lowercase());
        let is_running = matches.iter().any(|s| s.contains("(Running)"));
        
        let is_port_occupied = if port != 0 {
            crate::platform::get_port_info(port).is_some()
        } else {
            false
        };

        if is_running || is_port_occupied {
            println!(" └─ \x1b[32m✓ running\x1b[0m");
        } else {
            println!(" └─ \x1b[31m✗ stopped or offline\x1b[0m");
            check_failed = true;
            reasons.push(format!(
                "{} is stopped or offline. Suggested fix: run `docker compose up -d {}`",
                svc, svc.to_lowercase()
            ));
        }
    }

    // 8. Env files check
    if !required_services.is_empty() {
        let missing_env_vars = check_env_configuration(&env_file_path, &required_services);
        if !missing_env_vars.is_empty() {
            println!("\nConfiguration (.env)");
            if has_env {
                println!(" ├─ .env file detected");
            } else {
                println!(" ├─ \x1b[33m⚠ .env file missing\x1b[0m");
            }
            for var in &missing_env_vars {
                println!(" └─ \x1b[31m✗ {} is missing\x1b[0m", var);
                check_failed = true;
                reasons.push(format!("Configuration error: {} is missing in .env", var));
            }
        }
    }

    if check_failed {
        println!("\n\x1b[31m\x1b[1m✗ Project environment is incompatible.\x1b[0m");
        println!("\n\x1b[1mPrimary reason(s):\x1b[0m");
        for r in &reasons {
            println!("  - {}", r);
        }
    } else {
        println!("\n\x1b[32m\x1b[1m✓ Project environment is healthy and compatible!\x1b[0m");
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
                "active version is {} but package.json requires {}",
                active_version, req_version
            ));
        }
    } else if req_version.starts_with('^') {
        let req_num_str = req_version.trim_start_matches('^').trim();
        let req_major = req_num_str.split('.').next()?.parse::<u32>().ok()?;
        let active_major = clean_active.split('.').next()?.parse::<u32>().ok()?;
        if active_major != req_major {
            return Some(format!(
                "active version is {} but package.json requires {}",
                active_version, req_version
            ));
        }
    }
    None
}
