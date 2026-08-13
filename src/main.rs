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

    // 1. Route based on subcommands
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
        }
    }

    // 2. Route based on positional query
    if let Some(query) = cli.query {
        if query == "." || query.to_lowercase() == "project" {
            inspectors::project::inspect_current_project(cli.json);
            return;
        }

        // Numeric → PID process inspection
        if query.chars().all(|c| c.is_ascii_digit()) {
            inspectors::process::inspect_process(&query);
            return;
        }

        // Try to resolve as an executable on PATH → build cross-system chain
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
