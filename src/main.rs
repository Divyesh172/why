mod cli;
mod resolver;
mod inspectors;
mod ecosystems;
mod platform;
mod graph;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    // 1. Route subcommands
    if let Some(cmd) = cli.command {
        match cmd {
            Commands::Port { port } => {
                let finding = graph::chains::build_port_chain(port);
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&finding)
                        .unwrap_or_else(|_| "{}".to_string()));
                } else {
                    finding.print_terminal();
                }
                return;
            }
            Commands::Project => {
                inspectors::project::inspect_current_project(cli.json);
                return;
            }
            Commands::Env { name } => {
                inspectors::environment::print_single_env(&name);
                return;
            }
            Commands::Fix { subject } => {
                handle_fix(&subject, cli.apply, cli.json);
                return;
            }
        }
    }

    // 2. Route positional query
    if let Some(query) = cli.query {
        // "why fix node" as a positional query (alternative syntax)
        if query == "fix" {
            println!("Usage: why fix <subject>  (e.g. why fix node)");
            return;
        }

        if query == "." || query.to_lowercase() == "project" {
            inspectors::project::inspect_current_project(cli.json);
            return;
        }

        // Numeric → PID
        if query.chars().all(|c| c.is_ascii_digit()) {
            inspectors::process::inspect_process(&query);
            return;
        }

        // Executable on PATH → cross-system chain
        let (resolved, _) = resolver::path::find_all_in_path(&query);
        if !resolved.is_empty() {
            let finding = graph::chains::build_executable_chain(&query);
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&finding)
                    .unwrap_or_else(|_| "{}".to_string()));
            } else {
                finding.print_terminal();
            }
        } else {
            // Fallback: search by running process name
            inspectors::process::inspect_process(&query);
        }
    } else {
        // Default (no args): inspect the current directory as a project
        inspectors::project::inspect_current_project(cli.json);
    }
}

fn handle_fix(subject: &str, apply: bool, json: bool) {
    match graph::fixes::suggest_fixes(subject) {
        Some(plan) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&plan)
                    .unwrap_or_else(|_| "{}".to_string()));
            } else if apply {
                plan.print_terminal();
                println!("\n\x1b[1mApplying first option...\x1b[0m");
                if let Err(e) = plan.apply_first() {
                    eprintln!("\x1b[31m✗ {}\x1b[0m", e);
                    std::process::exit(1);
                }
            } else {
                plan.print_terminal();
            }
        }
        None => {
            println!("\n\x1b[32m✓ '{}' looks healthy — no fixes needed.\x1b[0m\n", subject);
        }
    }
}
