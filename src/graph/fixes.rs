use std::process::Command;
use serde::Serialize;
use crate::inspectors::finding::Severity;
use crate::resolver::path::find_all_in_path;
use crate::graph::chains::build_executable_chain;

// ─────────────────────────────────────────────────────────────────────────────
// Data model
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub enum FixRisk {
    /// No state changes — only switches the active version, fully reversible.
    Low,
    /// Installs a new version or reconfigures something; easily undone.
    Medium,
    /// Modifies system state or requires elevated privileges.
    High,
}

#[derive(Debug, Serialize, Clone)]
pub struct FixOption {
    pub label: String,
    /// The shell command that would be run.
    pub command: String,
    pub risk: FixRisk,
    pub reversible: bool,
}

/// A complete, human-readable remediation plan for one diagnosed problem.
#[derive(Debug, Serialize)]
pub struct FixPlan {
    pub subject: String,
    pub problem: String,
    pub options: Vec<FixOption>,
    /// Presented to the user as the canonical run command, e.g. `why fix node`
    pub fix_command: String,
}

impl FixPlan {
    /// Prints the plan to stdout (dry-run by default).
    pub fn print_terminal(&self) {
        println!("\n\x1b[1mProblem:\x1b[0m");
        println!("  {}", self.problem);
        println!("\n\x1b[1mPossible fixes:\x1b[0m");
        for (i, opt) in self.options.iter().enumerate() {
            let risk_label = match opt.risk {
                FixRisk::Low    => "\x1b[32m[low risk]\x1b[0m",
                FixRisk::Medium => "\x1b[33m[medium risk]\x1b[0m",
                FixRisk::High   => "\x1b[31m[high risk]\x1b[0m",
            };
            let rev = if opt.reversible { "reversible" } else { "irreversible" };
            println!("\n  \x1b[1m{}.\x1b[0m {} {}", i + 1, opt.label, risk_label);
            println!("     Command : \x1b[36m{}\x1b[0m", opt.command);
            println!("     Impact  : {}", rev);
        }
        println!("\n\x1b[90mRun with --apply to execute the first option automatically.\x1b[0m");
        println!("Or copy any command above and run it yourself.\n");
    }

    /// Executes option 0 (the recommended fix). Returns success/failure.
    pub fn apply_first(&self) -> Result<(), String> {
        let opt = self.options.first()
            .ok_or_else(|| "No fix options available.".to_string())?;

        println!("\n\x1b[1mApplying:\x1b[0m {}", opt.label);
        println!("  \x1b[36m{}\x1b[0m\n", opt.command);

        // Split the command string into program + args
        let parts: Vec<&str> = opt.command.splitn(2, ' ').collect();
        let program = parts[0];
        let args: Vec<&str> = if parts.len() > 1 {
            parts[1].split_whitespace().collect()
        } else {
            vec![]
        };

        let status = Command::new(program)
            .args(&args)
            .status()
            .map_err(|e| format!("Failed to run '{}': {}", opt.command, e))?;

        if status.success() {
            println!("\x1b[32m✓ Fix applied successfully.\x1b[0m");
            Ok(())
        } else {
            Err(format!("Command exited with status: {}", status))
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Fix generators
// ─────────────────────────────────────────────────────────────────────────────

/// Inspects the current state for `subject` and returns a FixPlan if a known
/// problem is detected. Returns `None` if everything looks healthy.
pub fn suggest_fixes(subject: &str) -> Option<FixPlan> {
    match subject.to_lowercase().as_str() {
        "node" | "nodejs" => suggest_node_fixes(),
        "python"          => suggest_runtime_fixes("python", "Python", &[
            ("scoop install python", FixRisk::Medium),
            ("winget install Python.Python.3", FixRisk::Medium),
        ]),
        "rust" | "cargo"  => suggest_runtime_fixes("cargo", "Rust", &[
            ("winget install Rustlang.Rustup", FixRisk::Medium),
            ("scoop install rustup", FixRisk::Medium),
        ]),
        "go"              => suggest_runtime_fixes("go", "Go", &[
            ("scoop install go", FixRisk::Medium),
            ("winget install GoLang.Go", FixRisk::Medium),
        ]),
        "docker"          => suggest_runtime_fixes("docker", "Docker", &[
            ("winget install Docker.DockerDesktop", FixRisk::Medium),
        ]),
        "postgresql" | "postgres" => Some(FixPlan {
            subject: "PostgreSQL".into(),
            problem: "PostgreSQL is required but is not running.".into(),
            fix_command: "why fix postgresql".into(),
            options: vec![
                FixOption {
                    label: "Start via Docker Compose".into(),
                    command: "docker compose up -d db".into(),
                    risk: FixRisk::Low,
                    reversible: true,
                },
                FixOption {
                    label: "Start the PostgreSQL Windows service".into(),
                    command: "sc start postgresql".into(),
                    risk: FixRisk::Medium,
                    reversible: true,
                },
            ],
        }),
        "redis"           => Some(FixPlan {
            subject: "Redis".into(),
            problem: "Redis is required but is not running.".into(),
            fix_command: "why fix redis".into(),
            options: vec![
                FixOption {
                    label: "Start via Docker Compose".into(),
                    command: "docker compose up -d redis".into(),
                    risk: FixRisk::Low,
                    reversible: true,
                },
                FixOption {
                    label: "Start the Redis Windows service".into(),
                    command: "sc start redis".into(),
                    risk: FixRisk::Medium,
                    reversible: true,
                },
            ],
        }),
        _ => {
            // Generic: check if binary is missing and suggest installation
            let (results, _) = find_all_in_path(subject);
            if results.is_empty() {
                Some(FixPlan {
                    subject: subject.to_string(),
                    problem: format!("'{}' was not found on PATH.", subject),
                    fix_command: format!("why fix {}", subject),
                    options: vec![
                        FixOption {
                            label: format!("Install via Scoop"),
                            command: format!("scoop install {}", subject),
                            risk: FixRisk::Medium,
                            reversible: true,
                        },
                        FixOption {
                            label: format!("Install via WinGet"),
                            command: format!("winget install {}", subject),
                            risk: FixRisk::Medium,
                            reversible: true,
                        },
                    ],
                })
            } else {
                None // No known problem detected
            }
        }
    }
}

fn suggest_node_fixes() -> Option<FixPlan> {
    let finding = build_executable_chain("node");

    // Only surface a fix plan when a problem was actually detected
    if finding.severity == Severity::Info {
        return None;
    }

    let cause = finding.cause.clone();

    // Detect which node version managers are available on PATH
    let has_fnm   = find_all_in_path("fnm").0.first().is_some();
    let has_scoop = find_all_in_path("scoop").0.first().is_some();
    let has_nvm   = find_all_in_path("nvm").0.first().is_some();

    // Try to extract the required version from the finding's graph node values
    let required_ver = finding.graph.nodes.iter()
        .find(|n| n.node_type == "Constraint")
        .map(|n| n.value.trim_start_matches(">=").trim_start_matches('^').split('.').next().unwrap_or("22").to_string())
        .unwrap_or_else(|| "22".to_string());

    let mut options = Vec::new();

    if has_fnm {
        options.push(FixOption {
            label: format!("Switch to Node {} using fnm (recommended)", required_ver),
            command: format!("fnm use {}", required_ver),
            risk: FixRisk::Low,
            reversible: true,
        });
        options.push(FixOption {
            label: format!("Install Node {} via fnm", required_ver),
            command: format!("fnm install {}", required_ver),
            risk: FixRisk::Medium,
            reversible: true,
        });
    }

    if has_nvm {
        options.push(FixOption {
            label: format!("Switch to Node {} using nvm", required_ver),
            command: format!("nvm use {}", required_ver),
            risk: FixRisk::Low,
            reversible: true,
        });
        options.push(FixOption {
            label: format!("Install Node {} via nvm", required_ver),
            command: format!("nvm install {}", required_ver),
            risk: FixRisk::Medium,
            reversible: true,
        });
    }

    if has_scoop {
        options.push(FixOption {
            label: format!("Install Node {} via Scoop", required_ver),
            command: format!("scoop install nodejs@{}", required_ver),
            risk: FixRisk::Medium,
            reversible: true,
        });
    }

    // Always offer direct installer as fallback
    options.push(FixOption {
        label: format!("Download Node {} installer from nodejs.org", required_ver),
        command: format!("start https://nodejs.org/download/release/latest-v{}.x/", required_ver),
        risk: FixRisk::Medium,
        reversible: true,
    });

    if options.is_empty() {
        options.push(FixOption {
            label: "Install a Node version manager first (fnm recommended)".into(),
            command: "winget install Schniz.fnm".into(),
            risk: FixRisk::Medium,
            reversible: true,
        });
    }

    Some(FixPlan {
        subject: "Node.js".into(),
        problem: cause,
        fix_command: "why fix node".into(),
        options,
    })
}

fn suggest_runtime_fixes(
    binary: &str,
    display: &str,
    install_options: &[(&str, FixRisk)],
) -> Option<FixPlan> {
    let (results, _) = find_all_in_path(binary);

    // Only return a plan if binary is genuinely missing
    if !results.is_empty() {
        return None;
    }

    let options = install_options.iter().map(|(cmd, risk)| {
        let manager = if cmd.starts_with("scoop") { "Scoop" }
            else if cmd.starts_with("winget") { "WinGet" }
            else { "system" };
        FixOption {
            label: format!("Install {} via {}", display, manager),
            command: cmd.to_string(),
            risk: risk.clone(),
            reversible: true,
        }
    }).collect();

    Some(FixPlan {
        subject: display.into(),
        problem: format!("'{}' was not found on PATH.", binary),
        fix_command: format!("why fix {}", binary),
        options,
    })
}
