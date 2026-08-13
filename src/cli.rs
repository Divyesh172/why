use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "why")]
#[command(about = "Smart System & Project Inspector", long_about = None)]
pub struct Cli {
    /// The executable, process name, PID, or path (e.g. "node", "12480", "chrome", ".") to inspect.
    pub query: Option<String>,

    /// Show all installations of the executable found in system PATH.
    #[arg(short, long)]
    pub all: bool,

    /// Diagnose PATH resolution and version conflicts.
    #[arg(short, long)]
    pub conflict: bool,

    /// Show unmasked values of environment variables.
    #[arg(long)]
    pub show_env: bool,

    /// Output report in structured JSON format.
    #[arg(long)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Inspect what is using a specific port (e.g., "why port 8080")
    Port {
        /// The port number (e.g., 8080)
        port: u16,
    },
    /// Inspect the project in the current directory
    Project,
    /// View a specific environment variable's value safely (e.g., "why env GITHUB_TOKEN")
    Env {
        /// Name of the environment variable to retrieve
        name: String,
    },
}
