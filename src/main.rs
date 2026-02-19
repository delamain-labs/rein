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
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate { file, ast } => {
            let exit_code = commands::validate::run_validate(&file, ast);
            process::exit(exit_code);
        }
    }
}
