use clap::Parser;

/// `cargo matrix`: run a curated matrix of cargo commands (targets/features) from a YAML config.
#[derive(Parser, Debug)]
#[command(name = "cargo-matrix", bin_name = "cargo-matrix", version, about)]
struct Cli {
    #[command(flatten)]
    args: cargo_matrix::MatrixArgs,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = cargo_matrix::run(cli.args) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
