use clap::Args;

/// Massage packages by running cargo fix, clippy, fmt, check, and test
#[derive(Args, Debug)]
pub struct MassageArgs {
    #[command(flatten)]
    workspace: clap_cargo::Workspace,

    /// Enable verbose output (show warnings)
    #[arg(long = "verbose")]
    pub verbose: bool,
}

pub fn run(args: MassageArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Build target flags: either "--workspace" or per-package `-p` flags.
    let target_flags = if args.workspace.workspace || args.workspace.package.is_empty() {
        "--workspace".to_string()
    } else {
        args.workspace
            .package
            .iter()
            .map(|p| format!("-p {p}"))
            .collect::<Vec<_>>()
            .join(" ")
    };

    // Build the massage script using a template, then execute it with `sh!`.
    let script = format!(
        r#"
set -e

echo [1/5] Running cargo fix...
cargo fix --allow-dirty --allow-staged --quiet {target_flags}

echo [2/5] Running cargo clippy --fix...
cargo clippy --fix --allow-dirty --allow-staged --quiet {target_flags}

echo [3/5] Running cargo fmt...
cargo fmt --all --quiet

echo [4/5] Running cargo check...
cargo check --quiet {target_flags}

echo [5/5] Running cargo test...
RUST_BACKTRACE=1 cargo nextest run --no-tests pass {target_flags}
"#,
        target_flags = target_flags
    );

    let opts = if args.verbose {
        crate::sh::ShOptions::default()
    } else {
        crate::sh::ShOptions {
            quiet: true,
            ..Default::default()
        }
    };
    crate::sh!(options(opts), script)?;
    Ok(())
}
