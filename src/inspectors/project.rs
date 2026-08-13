use std::env;
use std::fs;
use std::path::Path;
use std::collections::HashMap;
use serde::Deserialize;

use crate::resolver::path::find_all_in_path;
use crate::platform::find_services;
use crate::inspectors::executable::query_version;
use crate::inspectors::finding::{Finding, Severity, Evidence};

#[derive(Deserialize, Debug)]
struct PackageJson {
    engines: Option<Engines>,
}

#[derive(Deserialize, Debug)]
struct Engines {
    node: Option<String>,
}

#[derive(Deserialize, Debug)]
struct DockerCompose {
    services: Option<HashMap<String, Service>>,
}

#[derive(Deserialize, Debug)]
struct Service {
    image: Option<String>,
}

/// Parses docker-compose configuration files to identify required databases/services.
fn get_docker_compose_services(path: &Path) -> Vec<String> {
    let mut services = Vec::new();
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return services,
    };
    
    let compose: DockerCompose = match serde_yaml::from_str(&content) {
        Ok(c) => c,
        Err(_) => return services,
    };
    
    if let Some(srvs) = compose.services {
        for (name, details) in srvs {
            let name_lower = name.to_lowercase();
            let image_lower = details.image.unwrap_or_default().to_lowercase();
            
            if name_lower.contains("postgres") || image_lower.contains("postgres") {
                services.push("PostgreSQL".to_string());
            } else if name_lower.contains("redis") || image_lower.contains("redis") {
                services.push("Redis".to_string());
            } else if name_lower.contains("mysql") || image_lower.contains("mysql") {
                services.push("MySQL".to_string());
            } else if name_lower.contains("mongodb") || image_lower.contains("mongo") {
                services.push("MongoDB".to_string());
            }
        }
    }
    
    services.sort();
    services.dedup();
    services
}

/// Helper to get the required node version string from package.json.
fn get_node_engine_requirement(project_dir: &Path) -> Option<String> {
    let pkg_json_path = project_dir.join("package.json");
    let content = fs::read_to_string(pkg_json_path).ok()?;
    let pkg: PackageJson = serde_json::from_str(&content).ok()?;
    pkg.engines?.node
}

/// Scans the current directory to diagnose project environment dependencies using the Finding model.
pub fn inspect_current_project(json: bool) {
    let current_dir = match env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            if json {
                println!("[]");
            } else {
                println!("\x1b[31mError retrieving current directory: {}\x1b[0m", e);
            }
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
                let services_list = get_docker_compose_services(&path);
                required_services.extend(services_list);
            }
        }
    }

    if files.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("\nNo project files (package.json, pom.xml, requirements.txt, Cargo.toml, etc.) found in this directory.");
            println!("Run 'why project' inside a codebase directory.");
        }
        return;
    }

    let env_file_path = current_dir.join(".env");
    let has_env = env_file_path.is_file();
    if has_env {
        files.push(".env".to_string());
    }

    let mut findings = Vec::new();

    // 1. Node.js check
    if files.contains(&"package.json".to_string()) {
        let (resolved, _) = find_all_in_path("node");
        if let Some(res) = resolved.first() {
            let active_ver = query_version(&res.resolved_path).unwrap_or_else(|| "unknown".to_string());
            let constraint = check_node_version_requirement(&current_dir, &res.resolved_path);
            
            match constraint {
                Some(ref_err) => {
                    findings.push(Finding {
                        severity: Severity::Error,
                        subject: "Node.js".to_string(),
                        cause: format!("Active Node.js version does not satisfy package.json requirement: {}", ref_err),
                        evidence: vec![
                            Evidence { label: "Required".to_string(), value: get_node_engine_requirement(&current_dir).unwrap_or_default() },
                            Evidence { label: "Active".to_string(), value: active_ver },
                            Evidence { label: "Resolved".to_string(), value: res.resolved_path.to_string_lossy().to_string() },
                        ],
                        suggestion: Some("Switch Node version to satisfy the constraint using your node manager (nvm, fnm, or scoop).".to_string()),
                    });
                }
                None => {
                    findings.push(Finding {
                        severity: Severity::Info,
                        subject: "Node.js".to_string(),
                        cause: "Node.js environment matches project requirement.".to_string(),
                        evidence: vec![
                            Evidence { label: "Required".to_string(), value: get_node_engine_requirement(&current_dir).unwrap_or_else(|| ">=18".to_string()) },
                            Evidence { label: "Active".to_string(), value: active_ver },
                        ],
                        suggestion: None,
                    });
                }
            }
        } else {
            findings.push(Finding {
                severity: Severity::Error,
                subject: "Node.js".to_string(),
                cause: "Node.js runtime is missing on PATH.".to_string(),
                evidence: vec![
                    Evidence { label: "Required by".to_string(), value: "package.json".to_string() },
                ],
                suggestion: Some("Install Node.js (via scoop install nodejs, fnm, or directly from nodejs.org).".to_string()),
            });
        }
    }

    // 2. Python check
    let py_file = ["requirements.txt", "pyproject.toml", "pipfile"].iter()
        .find(|f| files.contains(&f.to_string()));
    if let Some(filename) = py_file {
        let (resolved, _) = find_all_in_path("python");
        if let Some(res) = resolved.first() {
            let active_ver = query_version(&res.resolved_path).unwrap_or_else(|| "unknown".to_string());
            findings.push(Finding {
                severity: Severity::Info,
                subject: "Python".to_string(),
                cause: "Python runtime is available.".to_string(),
                evidence: vec![
                    Evidence { label: "Required by".to_string(), value: filename.to_string() },
                    Evidence { label: "Active version".to_string(), value: active_ver },
                ],
                suggestion: None,
            });
        } else {
            findings.push(Finding {
                severity: Severity::Error,
                subject: "Python".to_string(),
                cause: "Python executable is missing on PATH.".to_string(),
                evidence: vec![
                    Evidence { label: "Required by".to_string(), value: filename.to_string() },
                ],
                suggestion: Some("Install Python (via scoop install python, or download from python.org).".to_string()),
            });
        }
    }

    // 3. Rust check
    if files.contains(&"Cargo.toml".to_string()) {
        let (resolved, _) = find_all_in_path("cargo");
        if let Some(res) = resolved.first() {
            let active_ver = query_version(&res.resolved_path).unwrap_or_else(|| "unknown".to_string());
            findings.push(Finding {
                severity: Severity::Info,
                subject: "Rust".to_string(),
                cause: "Rust toolchain is active and available.".to_string(),
                evidence: vec![
                    Evidence { label: "Required by".to_string(), value: "Cargo.toml".to_string() },
                    Evidence { label: "Active version".to_string(), value: active_ver },
                ],
                suggestion: None,
            });
        } else {
            findings.push(Finding {
                severity: Severity::Error,
                subject: "Rust".to_string(),
                cause: "Cargo executable is missing on PATH.".to_string(),
                evidence: vec![
                    Evidence { label: "Required by".to_string(), value: "Cargo.toml".to_string() },
                ],
                suggestion: Some("Install Rust toolchain using rustup (from https://rustup.rs).".to_string()),
            });
        }
    }

    // 4. Go check
    if files.contains(&"go.mod".to_string()) {
        let (resolved, _) = find_all_in_path("go");
        if let Some(res) = resolved.first() {
            let active_ver = query_version(&res.resolved_path).unwrap_or_else(|| "unknown".to_string());
            findings.push(Finding {
                severity: Severity::Info,
                subject: "Go".to_string(),
                cause: "Go runtime is available.".to_string(),
                evidence: vec![
                    Evidence { label: "Required by".to_string(), value: "go.mod".to_string() },
                    Evidence { label: "Active version".to_string(), value: active_ver },
                ],
                suggestion: None,
            });
        } else {
            findings.push(Finding {
                severity: Severity::Error,
                subject: "Go".to_string(),
                cause: "Go executable is missing on PATH.".to_string(),
                evidence: vec![
                    Evidence { label: "Required by".to_string(), value: "go.mod".to_string() },
                ],
                suggestion: Some("Install Go runtime (via scoop install go, or from golang.org).".to_string()),
            });
        }
    }

    // 5. Java check
    let java_file = ["pom.xml", "build.gradle"].iter()
        .find(|f| files.contains(&f.to_string()));
    if let Some(filename) = java_file {
        let (resolved, _) = find_all_in_path("java");
        if let Some(res) = resolved.first() {
            let active_ver = query_version(&res.resolved_path).unwrap_or_else(|| "unknown".to_string());
            findings.push(Finding {
                severity: Severity::Info,
                subject: "Java".to_string(),
                cause: "Java Runtime Environment (JRE) is available.".to_string(),
                evidence: vec![
                    Evidence { label: "Required by".to_string(), value: filename.to_string() },
                    Evidence { label: "Active version".to_string(), value: active_ver },
                ],
                suggestion: None,
            });
        } else {
            findings.push(Finding {
                severity: Severity::Error,
                subject: "Java".to_string(),
                cause: "Java executable is missing on PATH.".to_string(),
                evidence: vec![
                    Evidence { label: "Required by".to_string(), value: filename.to_string() },
                ],
                suggestion: Some("Install a Java Development Kit (via scoop install openjdk, or from oracle/temurin).".to_string()),
            });
        }
    }

    // 6. Docker check
    if toolchain.contains(&"Docker Compose".to_string()) || toolchain.contains(&"Docker".to_string()) {
        let (resolved, _) = find_all_in_path("docker");
        if !resolved.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                subject: "Docker".to_string(),
                cause: "Docker is installed and available.".to_string(),
                evidence: vec![
                    Evidence { label: "Required by".to_string(), value: "Project container setup".to_string() },
                ],
                suggestion: None,
            });
        } else {
            findings.push(Finding {
                severity: Severity::Error,
                subject: "Docker".to_string(),
                cause: "Docker executable is missing on PATH.".to_string(),
                evidence: vec![
                    Evidence { label: "Required by".to_string(), value: "Project container setup".to_string() },
                ],
                suggestion: Some("Install Docker Desktop or Docker engine on your machine.".to_string()),
            });
        }
    }

    // 7. Services check
    for svc in &required_services {
        let port = match svc.as_str() {
            "PostgreSQL" => 5432,
            "MySQL" => 3306,
            "Redis" => 6379,
            "MongoDB" => 27017,
            _ => 0,
        };

        let matches = find_services(&svc.to_lowercase());
        let is_running = matches.iter().any(|s| s.contains("(Running)"));
        
        let is_port_occupied = if port != 0 {
            crate::platform::get_port_info(port).is_some()
        } else {
            false
        };

        let filename = if files.contains(&"docker-compose.yml".to_string()) {
            "docker-compose.yml"
        } else {
            "docker-compose.yaml"
        };

        if is_running || is_port_occupied {
            findings.push(Finding {
                severity: Severity::Info,
                subject: svc.clone(),
                cause: format!("{} is running and listening on port {}.", svc, port),
                evidence: vec![
                    Evidence { label: "Required by".to_string(), value: filename.to_string() },
                    Evidence { label: "Port".to_string(), value: port.to_string() },
                ],
                suggestion: None,
            });
        } else {
            findings.push(Finding {
                severity: Severity::Error,
                subject: svc.clone(),
                cause: format!("{} service is stopped or offline.", svc),
                evidence: vec![
                    Evidence { label: "Required by".to_string(), value: filename.to_string() },
                    Evidence { label: "Port".to_string(), value: port.to_string() },
                ],
                suggestion: Some(format!("Start the database container: run `docker compose up -d {}`", svc.to_lowercase())),
            });
        }
    }

    // 8. Env files check
    if !required_services.is_empty() {
        let missing_env_vars = check_env_configuration(&env_file_path, &required_services);
        if !missing_env_vars.is_empty() {
            let mut evidence = vec![
                Evidence { label: "Status".to_string(), value: if has_env { ".env file detected" } else { ".env file missing" }.to_string() }
            ];
            for var in &missing_env_vars {
                evidence.push(Evidence { label: "Missing variable".to_string(), value: var.clone() });
            }
            
            findings.push(Finding {
                severity: Severity::Error,
                subject: "Configuration (.env)".to_string(),
                cause: "Required database configuration parameters are missing in your environment file.".to_string(),
                evidence,
                suggestion: Some("Add the missing variables to your .env file with correct connection strings.".to_string()),
            });
        }
    }

    if json {
        if let Ok(json_str) = serde_json::to_string_pretty(&findings) {
            println!("{}", json_str);
        } else {
            println!("[]");
        }
    } else {
        println!("\n\x1b[1mProject: {}\x1b[0m", project_name);
        for finding in &findings {
            finding.print_terminal();
        }

        let has_errors = findings.iter().any(|f| f.severity == Severity::Error);
        if has_errors {
            println!("\n\x1b[31m\x1b[1m✗ Project environment is incompatible.\x1b[0m");
            println!("\n\x1b[1mPrimary reason(s):\x1b[0m");
            for f in &findings {
                if f.severity == Severity::Error {
                    println!("  - {}: {}", f.subject, f.cause);
                }
            }
        } else {
            println!("\n\x1b[32m\x1b[1m✓ Project environment is healthy and compatible!\x1b[0m");
        }
        println!();
    }
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
    
    let pkg: PackageJson = serde_json::from_str(&content).ok()?;
    let engines = pkg.engines?;
    let req_version = engines.node?;
    let req_version = req_version.trim();

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::env;

    #[test]
    fn test_parse_package_json() {
        let temp_dir = env::temp_dir().join("why_test_project");
        let _ = fs::create_dir_all(&temp_dir);
        let pkg_path = temp_dir.join("package.json");
        
        let pkg_content = r#"{
            "name": "test-project",
            "engines": {
                "node": ">=22"
            }
        }"#;
        fs::write(&pkg_path, pkg_content).unwrap();

        let pkg: PackageJson = serde_json::from_str(pkg_content).unwrap();
        assert_eq!(pkg.engines.unwrap().node.unwrap(), ">=22");
        
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_parse_docker_compose() {
        let temp_dir = env::temp_dir().join("why_test_compose");
        let _ = fs::create_dir_all(&temp_dir);
        let compose_path = temp_dir.join("docker-compose.yml");
        
        let compose_content = r#"
version: '3.8'
services:
  db:
    image: postgres:15-alpine
    ports:
      - "5432:5432"
  cache:
    image: redis:alpine
"#;
        fs::write(&compose_path, compose_content).unwrap();

        let services = get_docker_compose_services(&compose_path);
        assert!(services.contains(&"PostgreSQL".to_string()));
        assert!(services.contains(&"Redis".to_string()));
        
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
