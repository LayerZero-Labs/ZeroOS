use anyhow::{Context, Result};
use clap::Parser;
use log::{debug, info};
use std::fs;
use std::path::PathBuf;
use std::process::{exit, Command};

use build::cmds::{BuildArgs, StdMode};

fn find_file_named(
    root: &std::path::Path,
    file_name: &str,
    max_depth: usize,
) -> Result<Option<PathBuf>> {
    if max_depth == 0 {
        return Ok(None);
    }

    let mut entries: Vec<_> = std::fs::read_dir(root)
        .with_context(|| format!("Failed to list directory: {}", root.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        if path.is_file() && path.file_name().and_then(|n| n.to_str()) == Some(file_name) {
            return Ok(Some(path));
        }
        if path.is_dir() {
            if let Some(found) = find_file_named(&path, file_name, max_depth - 1)? {
                return Ok(Some(found));
            }
        }
    }

    Ok(None)
}

fn find_spike_platform_linker_template(workspace_root: &std::path::Path) -> Result<PathBuf> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .arg("--manifest-path")
        .arg(workspace_root.join("Cargo.toml"))
        .output()
        .context("Failed to run `cargo metadata`")?;

    if !output.status.success() {
        anyhow::bail!(
            "`cargo metadata` failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let v: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("Failed to parse cargo metadata JSON")?;

    let packages = v
        .get("packages")
        .and_then(|p| p.as_array())
        .ok_or_else(|| anyhow::anyhow!("cargo metadata JSON: missing `packages` array"))?;

    for pkg in packages {
        if pkg.get("name").and_then(|n| n.as_str()) != Some("spike-platform") {
            continue;
        }

        let manifest = pkg
            .get("manifest_path")
            .and_then(|m| m.as_str())
            .ok_or_else(|| {
                anyhow::anyhow!("cargo metadata JSON: spike-platform missing manifest_path")
            })?;

        let manifest_dir = std::path::Path::new(manifest)
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid manifest_path for spike-platform"))?;

        let src_tpl = manifest_dir.join("src/linker.ld.template");
        if src_tpl.exists() {
            return Ok(src_tpl);
        }

        if let Some(found) = find_file_named(manifest_dir, "linker.ld.template", 3)? {
            return Ok(found);
        }

        anyhow::bail!(
            "spike-platform linker template not found under {} (expected `src/linker.ld.template`)",
            manifest_dir.display()
        );
    }

    anyhow::bail!("spike-platform package not found in `cargo metadata` output")
}

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
    Build(SpikeBuildArgs),

    Run(RunArgs),

    #[command(subcommand)]
    Generate(GenerateCmd),
}

#[derive(clap::Subcommand, Debug)]
enum GenerateCmd {
    Target(SpikeGenerateTargetArgs),

    Linker(SpikeGenerateLinkerArgs),
}

#[derive(clap::Args, Debug)]
struct SpikeBuildArgs {
    #[command(flatten)]
    base: BuildArgs,
}

#[derive(clap::Args, Debug)]
struct RunArgs {
    #[arg(value_name = "BINARY")]
    binary: PathBuf,

    #[arg(long, default_value = "RV64IMAC")]
    isa: String,

    #[arg(long, short = 'n', default_value = "1000000")]
    instructions: u64,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub spike_args: Vec<String>,
}

#[derive(clap::Args, Debug)]
struct SpikeGenerateTargetArgs {
    #[command(flatten)]
    base: build::cmds::GenerateTargetArgs,

    #[arg(long, short = 'o')]
    output: Option<PathBuf>,
}

#[derive(clap::Args, Debug)]
struct SpikeGenerateLinkerArgs {
    #[command(flatten)]
    base: build::cmds::GenerateLinkerArgs,

    #[arg(long, short = 'o', default_value = "linker.ld")]
    output: PathBuf,
}

fn main() {
    env_logger::Builder::from_default_env()
        .format_timestamp(None)
        .format_module_path(false)
        .init();

    debug!("cargo-spike starting");

    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        exit(1);
    }
}

fn run() -> Result<()> {
    let Cli::Spike(cmd) = Cli::parse();

    match cmd {
        SpikeCmd::Build(args) => build_command(args),
        SpikeCmd::Run(args) => run_command(args),
        SpikeCmd::Generate(gen_cmd) => match gen_cmd {
            GenerateCmd::Target(args) => generate_target_command(args),
            GenerateCmd::Linker(args) => generate_linker_command(args),
        },
    }
}

fn build_command(args: SpikeBuildArgs) -> Result<()> {
    debug!("build_command: {:?}", args);

    let workspace_root = build::cmds::find_workspace_root()?;
    debug!("workspace_root: {}", workspace_root.display());

    let linker_tpl_path = find_spike_platform_linker_template(&workspace_root)?;
    let linker_tpl = std::fs::read_to_string(&linker_tpl_path).with_context(|| {
        format!(
            "Failed to read spike-platform linker template: {}",
            linker_tpl_path.display()
        )
    })?;

    let fully = args.base.mode == StdMode::Std || args.base.fully;

    let toolchain_paths = if args.base.mode == StdMode::Std || fully {
        let tc_cfg = build::toolchain::ToolchainConfig::default();
        let install_cfg = build::toolchain::InstallConfig::default();
        let paths = match build::toolchain::get_or_install_toolchain(
            args.base.musl_lib_path.clone(),
            args.base.gcc_lib_path.clone(),
            &tc_cfg,
            &install_cfg,
        ) {
            Ok(p) => (p.musl_lib, p.gcc_lib),
            Err(e) => {
                eprintln!("Toolchain install failed: {}", e);
                eprintln!("Falling back to building toolchain from source...");
                build::cmds::get_or_build_toolchain(
                    args.base.musl_lib_path.clone(),
                    args.base.gcc_lib_path.clone(),
                    fully,
                )?
            }
        };
        Some(paths)
    } else {
        None
    };

    build::cmds::build_binary(
        &workspace_root,
        &args.base,
        toolchain_paths,
        Some(linker_tpl),
    )?;

    Ok(())
}

fn run_command(args: RunArgs) -> Result<()> {
    if !args.binary.exists() {
        anyhow::bail!("Binary not found: {}", args.binary.display());
    }

    debug!("Running binary: {}", args.binary.display());
    debug!("ISA: {}", args.isa);
    debug!(
        "Instructions: {}",
        if args.instructions == 0 {
            "unlimited".to_string()
        } else {
            args.instructions.to_string()
        }
    );

    println!("Running on Spike simulator...\n");

    let mut spike_cmd = Command::new("spike");
    spike_cmd.arg(format!("--isa={}", args.isa));

    if args.instructions > 0 {
        spike_cmd.arg(format!("--instructions={}", args.instructions));
    }

    spike_cmd.args(&args.spike_args);

    spike_cmd.arg(&args.binary);

    let args_vec: Vec<String> = spike_cmd
        .get_args()
        .map(|s| s.to_string_lossy().to_string())
        .collect();
    let spike_cmd_str = format!("spike {}", args_vec.join(" "));
    debug!("Spike command: {}", spike_cmd_str);

    let status = spike_cmd
        .status()
        .context("Failed to execute spike (is it installed?)")?;

    if !status.success() {
        exit(status.code().unwrap_or(1));
    }

    Ok(())
}

fn generate_target_command(cli_args: SpikeGenerateTargetArgs) -> Result<()> {
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
        return Err(anyhow::anyhow!("Either --profile or --target is required"));
    };

    let json_content =
        generate_target_spec(&cli_args.base).map_err(|e| anyhow::anyhow!("{}", e))?;

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

fn generate_linker_command(cli_args: SpikeGenerateLinkerArgs) -> Result<()> {
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
