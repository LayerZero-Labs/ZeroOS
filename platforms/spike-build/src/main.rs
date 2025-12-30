mod cmds;

use clap::Parser;
use log::debug;
use std::process::exit;

#[derive(Parser)]
#[command(name = "cargo-spike")]
#[command(bin_name = "cargo")]
#[command(about = "Build and run for Spike RISC-V simulator", version, long_about = None)]
enum Cli {
    #[command(name = "spike", subcommand)]
    Spike(SpikeCmd),
}

#[derive(clap::Subcommand, Debug)]
enum SpikeCmd {
    Build(cmds::build::SpikeBuildArgs),
    Run(cmds::run::RunArgs),
    #[command(subcommand)]
    Generate(cmds::generate::GenerateCmd),
}

fn main() {
    env_logger::Builder::from_default_env()
        .format_timestamp(None)
        .format_module_path(false)
        .init();

    debug!("cargo-spike starting");

    let Cli::Spike(cmd) = Cli::parse();
    let result = match cmd {
        SpikeCmd::Build(args) => cmds::build::build_command(args),
        SpikeCmd::Run(args) => cmds::run::run_command(args),
        SpikeCmd::Generate(gen_cmd) => match gen_cmd {
            cmds::generate::GenerateCmd::Target(args) => {
                cmds::generate::generate_target_command(args)
            }
            cmds::generate::GenerateCmd::Linker(args) => {
                cmds::generate::generate_linker_command(args)
            }
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {:#}", e);
        exit(1);
    }
}
