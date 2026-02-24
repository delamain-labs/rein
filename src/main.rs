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
    /// Parse and validate a .rein file.
    ///
    /// Exit codes: 0 = valid, 1 = errors, 2 = valid with --strict warnings
    Validate {
        /// Path to the .rein file
        file: std::path::PathBuf,
        /// Print AST as JSON instead of validating
        #[arg(long)]
        ast: bool,
        /// Output format: human (default) or json
        #[arg(long, default_value = "human")]
        format: String,
        /// Warn when safety features parse but are not enforced at runtime
        #[arg(long)]
        strict: bool,
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
    /// Explain what a .rein file defines in plain language
    Explain {
        /// Path to the .rein file
        file: std::path::PathBuf,
    },
    /// Start the Rein Language Server (LSP)
    Lsp,
    /// Start the Rein API server
    Serve {
        /// Path to the .rein file to serve
        file: std::path::PathBuf,
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Port to listen on
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
    },
    /// Run an agent defined in a .rein file
    Run {
        /// Path to the .rein file
        file: std::path::PathBuf,
        /// User prompt message to send to the agent
        #[arg(short, long)]
        message: Option<String>,
        /// Show execution plan without calling APIs
        #[arg(long)]
        dry_run: bool,
        /// Run with a mock provider to demo enforcement (no API keys needed)
        #[arg(long)]
        demo: bool,
        /// Output trace as OpenTelemetry-compatible JSON
        #[arg(long)]
        otel: bool,
        /// Write approval audit events to a JSONL file at the given path
        #[arg(long)]
        audit_log: Option<std::path::PathBuf>,
    },
    /// Run scenario and eval blocks in a .rein file
    ///
    /// Exit codes: 0 = all pass, 1 = any failure
    Eval {
        /// Path to the .rein file
        file: std::path::PathBuf,
        /// Run only the named scenario
        #[arg(long)]
        scenario: Option<String>,
        /// Print full agent response per scenario
        #[arg(long)]
        verbose: bool,
        /// Run with a mock provider (no API keys needed)
        #[arg(long)]
        demo: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate {
            file,
            ast,
            format,
            strict,
        } => {
            let exit_code = commands::validate::run_validate(&file, ast, &format, strict);
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
        Command::Explain { file } => {
            let exit_code = commands::explain::run_explain(&file);
            process::exit(exit_code);
        }
        Command::Lsp => {
            let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
            rt.block_on(rein::lsp::run_lsp());
        }
        Command::Serve { file, host, port } => {
            let exit_code = commands::serve::run_serve(&file, &host, port);
            process::exit(exit_code);
        }
        Command::Run {
            file,
            message,
            dry_run,
            demo,
            otel,
            audit_log,
        } => {
            let exit_code = commands::run::run_agent(
                &file,
                message.as_deref(),
                dry_run,
                demo,
                otel,
                audit_log.as_deref(),
            );
            process::exit(exit_code);
        }
        Command::Eval {
            file,
            scenario,
            verbose,
            demo,
        } => {
            let exit_code =
                commands::eval::run_eval_command(&file, scenario.as_deref(), verbose, demo);
            process::exit(exit_code);
        }
    }
}
