mod cli;
mod resolver;
mod inspectors;
mod ecosystems;
mod platform;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    // 1. Route based on subcommands
    if let Some(cmd) = cli.command {
        match cmd {
            Commands::Port { port } => {
                inspectors::port::inspect_port(port);
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

        // If query is numeric, treat it as a PID process inspection
        if query.chars().all(|c| c.is_ascii_digit()) {
            inspectors::process::inspect_process(&query);
            return;
        }

        // Try to resolve as an executable on PATH
        let (resolved, _) = resolver::path::find_all_in_path(&query);
        if !resolved.is_empty() {
            inspectors::executable::print_executable_report(&query, cli.all, cli.conflict, cli.show_env, cli.json);
        } else {
            // Fallback: Check if it matches a running process name (like "chrome")
            inspectors::process::inspect_process(&query);
        }
    } else {
        // Default when run with no arguments: inspect the current directory project
        inspectors::project::inspect_current_project(cli.json);
    }
}
