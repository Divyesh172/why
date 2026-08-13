use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "why")]
#[command(about = "Causal system & project diagnostic tool", long_about = None)]
pub struct Cli {
    /// The executable, process name, PID, or path to inspect (e.g. \"node\", \"12480\", \".\")
    pub query: Option<String>,

    /// Show all installations of the executable found in PATH
    #[arg(short, long)]
    pub all: bool,

    /// Diagnose PATH resolution conflicts between multiple installations
    #[arg(short, long)]
    pub conflict: bool,

    /// Show unmasked values of environment variables
    #[arg(long)]
    pub show_env: bool,

    /// Output the report in structured JSON format
    #[arg(long)]
    pub json: bool,

    /// Execute the first suggested fix automatically (use with `why fix <subject>`)
    #[arg(long)]
    pub apply: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Inspect what is using a specific port (e.g. `why port 8080`)
    Port {
        /// The port number (e.g. 8080)
        port: u16,
    },
    /// Inspect the project in the current directory
    Project,
    /// View a specific environment variable safely (e.g. `why env GITHUB_TOKEN`)
    Env {
        /// Name of the environment variable to retrieve
        name: String,
    },
    /// Suggest or apply a fix for a diagnosed problem (e.g. `why fix node`)
    Fix {
        /// The subject to fix — runtime, service, or binary name (e.g. node, postgresql, redis)
        subject: String,
    },
}
