mod act;
mod check_workspace;
mod findup;
mod massage;
mod sh;
mod spike_syscall_instcount;

use clap::{Parser, Subcommand};

/// xtask command-line interface
#[derive(Parser)]
#[command(name = "xtask", version, about = "ZeroOS auxiliary tasks")]
struct Cli {
    /// Subcommand to run
    #[command(subcommand)]
    command: Command,
}

/// Supported subcommands
#[derive(Subcommand)]
enum Command {
    /// Run the 'massage' task
    Massage(massage::MassageArgs),
    /// Run a curated matrix of cargo commands (targets/features) from config
    Matrix(cargo_matrix::MatrixArgs),
    /// Run GitHub Actions locally via `act` (forwards all args to the `act` CLI)
    Act(act::ActArgs),
    /// Measure syscall instruction-count "cost" using Spike commit logs.
    #[command(name = "spike-syscall-instcount")]
    SpikeSyscallInstCount(spike_syscall_instcount::SpikeSyscallInstCountArgs),
    /// Check workspace consistency (versions, dependencies)
    #[command(name = "check-workspace")]
    CheckWorkspace(check_workspace::CheckWorkspaceArgs),
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::Massage(args) => massage::run(args),
        Command::Matrix(args) => cargo_matrix::run(args).map_err(|e| e.into()),
        Command::Act(args) => act::run(args),
        Command::SpikeSyscallInstCount(args) => spike_syscall_instcount::run(args),
        Command::CheckWorkspace(args) => check_workspace::run(args).map_err(|e| e.into()),
    }
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
