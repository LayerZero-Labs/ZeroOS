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
    let packages = if args.workspace.workspace || args.workspace.package.is_empty() {
        Vec::new()
    } else {
        args.workspace.package.clone()
    };

    let commands = ["fix", "fmt", "check", "test"];
    for (i, cmd) in commands.iter().enumerate() {
        println!(
            "[massage {}/{}] matrix --command {}",
            i + 1,
            commands.len(),
            cmd
        );
        cargo_matrix::run(cargo_matrix::MatrixArgs {
            config: None,
            command: Some((*cmd).to_string()),
            packages: packages.clone(),
            verbose: args.verbose,
        })
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    }

    println!("[massage] done");
    Ok(())
}
