mod massage;
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
    Massage(massage::MassageArgs),
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Massage(args) => {
            if let Err(e) = massage::run(args) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
