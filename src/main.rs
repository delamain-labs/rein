
mod commands;

use clap::{Parser, Subcommand};
use std::process;

#[derive(Parser)]
#[command(name = "rein", about = "The Rein agent runtime CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse and validate a .rein file
    Validate {
        /// Path to the .rein file
        file: std::path::PathBuf,
        /// Print AST as JSON instead of validating
        #[arg(long)]
        ast: bool,
    },
    /// Aggregate and display cost statistics from trace files
    Cost {
        /// Trace files or directories to analyze
        #[arg(required = true)]
        paths: Vec<std::path::PathBuf>,
    },
    /// Auto-format .rein files to canonical style
    Fmt {
        /// .rein files to format
        #[arg(required = true)]
        files: Vec<std::path::PathBuf>,
        /// Check formatting without writing changes
        #[arg(long)]
        check: bool,
    },
    /// Initialize a new Rein project
    Init {
        /// Directory name for the new project
        #[arg(default_value = "my-rein-project")]
        name: std::path::PathBuf,
    },
    /// Run an agent defined in a .rein file
    Run {
        /// Path to the .rein file
        file: std::path::PathBuf,
        /// User prompt message to send to the agent
        #[arg(short, long)]
        message: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate { file, ast } => {
            let exit_code = commands::validate::run_validate(&file, ast);
            process::exit(exit_code);
        }
        Command::Cost { paths } => {
            let exit_code = commands::cost::run_cost(&paths);
            process::exit(exit_code);
        }
        Command::Fmt { files, check } => {
            let exit_code = commands::fmt::run_fmt(&files, check);
            process::exit(exit_code);
        }
        Command::Init { name } => {
            let exit_code = commands::init::run_init(&name);
            process::exit(exit_code);
        }
        Command::Run { file, message } => {
            let exit_code = commands::run::run_agent(&file, message.as_deref());
            process::exit(exit_code);
        }
    }
}
