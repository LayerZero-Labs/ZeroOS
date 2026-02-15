mod cmds;
mod findup;
mod sh;

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
    Massage(cmds::massage::MassageArgs),
    /// Run a curated matrix of cargo commands (targets/features) from config
    Matrix(cargo_matrix::MatrixArgs),
    /// Run GitHub Actions locally via `act` (forwards all args to the `act` CLI)
    Act(cmds::act::ActArgs),
    /// Measure syscall instruction-count "cost" using Spike commit logs.
    #[command(name = "spike-syscall-instcount")]
    SpikeSyscallInstCount(cmds::spike_syscall_instcount::SpikeSyscallInstCountArgs),
    /// Check workspace consistency (versions, dependencies)
    #[command(name = "check-workspace")]
    CheckWorkspace(cmds::check_workspace::CheckWorkspaceArgs),
    /// Analyze binary sizes for different backtrace modes
    #[command(name = "analyze-backtrace")]
    AnalyzeBacktrace(cmds::analyze_backtrace::AnalyzeBacktraceArgs),
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::Massage(args) => cmds::massage::run(args),
        Command::Matrix(args) => cargo_matrix::run(args).map_err(|e| e.into()),
        Command::Act(args) => cmds::act::run(args),
        Command::SpikeSyscallInstCount(args) => cmds::spike_syscall_instcount::run(args),
        Command::CheckWorkspace(args) => cmds::check_workspace::run(args).map_err(|e| e.into()),
        Command::AnalyzeBacktrace(args) => cmds::analyze_backtrace::run(args).map_err(|e| e.into()),
    }
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
