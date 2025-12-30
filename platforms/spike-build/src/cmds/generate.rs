use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use log::info;
use std::fs;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum GenerateCmd {
    Target(SpikeGenerateTargetArgs),
    Linker(SpikeGenerateLinkerArgs),
}

#[derive(Args, Debug)]
pub struct SpikeGenerateTargetArgs {
    #[command(flatten)]
    pub base: build::cmds::GenerateTargetArgs,

    #[arg(long, short = 'o')]
    pub output: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct SpikeGenerateLinkerArgs {
    #[command(flatten)]
    pub base: build::cmds::GenerateLinkerArgs,

    #[arg(long, short = 'o', default_value = "linker.ld")]
    pub output: PathBuf,
}

pub fn generate_target_command(cli_args: SpikeGenerateTargetArgs) -> Result<()> {
    use build::cmds::generate_target_spec;
    use build::spec::{load_target_profile, parse_target_triple};

    let target_triple = if let Some(profile_name) = &cli_args.base.profile {
        load_target_profile(profile_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown profile: {}", profile_name))?
            .config
            .target_triple()
    } else if let Some(target) = &cli_args.base.target {
        parse_target_triple(target)
            .ok_or_else(|| anyhow::anyhow!("Cannot parse target triple: {}", target))?
            .target_triple()
    } else {
        anyhow::bail!("Either --profile or --target is required");
    };

    let json_content =
        generate_target_spec(&cli_args.base, build::spec::TargetRenderOptions::default())
            .map_err(|e| anyhow::anyhow!("{}", e))?;

    let output_path = cli_args
        .output
        .unwrap_or_else(|| PathBuf::from(format!("{}.json", target_triple)));

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory: {}", parent.display()))?;
    }

    fs::write(&output_path, &json_content)
        .with_context(|| format!("Failed to write target spec to {}", output_path.display()))?;

    info!("Generated target spec: {}", output_path.display());
    info!("Target triple: {}", target_triple);

    Ok(())
}

pub fn generate_linker_command(cli_args: SpikeGenerateLinkerArgs) -> Result<()> {
    use build::cmds::generate_linker_script;

    let result = generate_linker_script(&cli_args.base)?;

    if let Some(parent) = cli_args.output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory: {}", parent.display()))?;
    }

    fs::write(&cli_args.output, &result.script_content).with_context(|| {
        format!(
            "Failed to write linker script to {}",
            cli_args.output.display()
        )
    })?;

    info!("Generated linker script: {}", cli_args.output.display());

    Ok(())
}
