use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "cargo-zeroos")]
#[command(bin_name = "cargo")]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Zeroos(ZeroosArgs),
}

#[derive(Parser)]
struct ZeroosArgs {
    #[command(subcommand)]
    command: ZeroosCommands,
}

#[derive(Subcommand)]
enum ZeroosCommands {
    Build(ZeroosBuildArgs),

    BuildMusl(BuildMuslArgs),

    InstallMusl(InstallMuslArgs),

    FindMusl(FindMuslArgs),

    Generate(GenerateArgs),
}

#[derive(Parser)]
struct GenerateArgs {
    #[command(subcommand)]
    command: GenerateCmd,
}

#[derive(Subcommand)]
enum GenerateCmd {
    Target(ZeroosGenerateTargetArgs),

    Linker(ZeroosGenerateLinkerArgs),
}

#[derive(Args, Debug)]
struct ZeroosBuildArgs {
    #[command(flatten)]
    base: zeroos_build::cmds::BuildArgs,
}

#[derive(Args)]
struct ZeroosGenerateTargetArgs {
    #[command(flatten)]
    base: zeroos_build::cmds::GenerateTargetArgs,

    #[arg(long, short = 'o')]
    output: Option<PathBuf>,
}

#[derive(Args)]
struct ZeroosGenerateLinkerArgs {
    #[command(flatten)]
    base: zeroos_build::cmds::GenerateLinkerArgs,

    #[arg(long, short = 'o', default_value = "linker.ld")]
    output: PathBuf,
}

#[derive(Parser)]
struct BuildMuslArgs {
    #[arg(long, default_value = "riscv64")]
    arch: String,

    #[arg(long)]
    output: Option<String>,

    #[arg(long, default_value = "--with-arch=rv64ima --with-abi=lp64")]
    gcc_config: String,

    #[arg(long)]
    no_gcc_config: bool,
}

#[derive(Parser)]
struct FindMuslArgs {
    #[arg(long, default_value = "riscv64")]
    arch: String,
}

#[derive(Parser)]
struct InstallMuslArgs {
    #[arg(long, default_value = "riscv64")]
    arch: String,

    #[arg(long)]
    output: Option<String>,

    /// GitHub repo in owner/name form (defaults to origin remote or LayerZero-Labs/ZeroOS)
    #[arg(long)]
    repo: Option<String>,

    /// Git tag to install from (defaults to latest release)
    #[arg(long)]
    tag: Option<String>,

    /// Replace any existing install
    #[arg(long)]
    force: bool,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .with_level(true)
        .init();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Zeroos(args) => match args.command {
            ZeroosCommands::Build(args) => build_command(args),
            ZeroosCommands::BuildMusl(args) => {
                build_musl(args);
                Ok(())
            }
            ZeroosCommands::InstallMusl(args) => {
                install_musl(args);
                Ok(())
            }
            ZeroosCommands::FindMusl(args) => {
                find_musl(args);
                Ok(())
            }
            ZeroosCommands::Generate(gen_args) => match gen_args.command {
                GenerateCmd::Target(args) => generate_target_command(args),
                GenerateCmd::Linker(args) => generate_linker_command(args),
            },
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {:#}", e);
        exit(1);
    }
}

fn expand_tilde(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped).to_string_lossy().to_string();
        }
    }
    path.to_string()
}

fn build_musl(args: BuildMuslArgs) {
    let output_dir = match args.output {
        Some(path) => expand_tilde(&path),
        None => {
            let home = dirs::home_dir().expect("Could not determine home directory");
            home.join(".zeroos/musl").to_string_lossy().to_string()
        }
    };

    let gcc_config = if args.no_gcc_config {
        None
    } else {
        Some(args.gcc_config.clone())
    };

    let config = zeroos_build::toolchain::BuildConfig {
        arch: args.arch.clone(),
        output_dir: output_dir.clone(),
        gcc_config,
        jobs: None,
    };

    let gcc_config_display = config
        .gcc_config
        .as_ref()
        .map(|cfg| format!("\n  GCC config:   {}", cfg))
        .unwrap_or_default();

    let permission_warning = if output_dir.starts_with("/usr") || output_dir.starts_with("/opt") {
        format!(
            "\nNote: Installing to {} requires sudo/root privileges\n",
            output_dir
        )
    } else {
        String::new()
    };

    println!(
        "Building RISC-V musl toolchain\n  Architecture: {}\n  Output:       {}{}{}",
        args.arch, output_dir, gcc_config_display, permission_warning
    );

    match zeroos_build::toolchain::build_musl_toolchain(&config) {
        Ok(paths) => {
            print_toolchain_paths(
                &paths,
                &format!(
                    "Build successful!\n\nToolchain installed at {}",
                    config.output_dir
                ),
            );
        }
        Err(e) => {
            eprintln!(
                r"
Build failed!

Error: {}

Common issues:
  - Missing: git, make, gcc, g++
  - Permissions: need sudo for /usr/local (use --output ~/path instead)
  - Network: can't download musl-cross-make
  - Disk space: needs ~2GB",
                e
            );
            exit(1);
        }
    }
}

fn find_musl(args: FindMuslArgs) {
    match zeroos_build::toolchain::discover_toolchain(&args.arch) {
        Some(paths) => {
            print_toolchain_paths(
                &paths,
                &format!("Found RISC-V musl toolchain for {}", args.arch),
            );
        }
        None => {
            eprintln!(
                r"
Toolchain not found for {}

To build the toolchain:
  cargo zeroos build-musl

Or set environment variables:
  export RISCV_MUSL_PATH=/path/to/musl/lib
  export RISCV_GCC_PATH=/path/to/gcc/lib",
                args.arch
            );
            exit(1);
        }
    }
}

fn install_musl(args: InstallMuslArgs) {
    let output_dir = match args.output {
        Some(path) => expand_tilde(&path),
        None => {
            let home = dirs::home_dir().expect("Could not determine home directory");
            home.join(".zeroos/musl").to_string_lossy().to_string()
        }
    };

    let config = zeroos_build::toolchain::InstallConfig {
        arch: args.arch.clone(),
        output_dir: output_dir.clone(),
        repo: args.repo.clone(),
        tag: args.tag.clone(),
        force: args.force,
    };

    println!(
        "Installing RISC-V musl toolchain\n  Architecture: {}\n  Output:       {}\n  Tag:          {}",
        args.arch,
        output_dir,
        args.tag.clone().unwrap_or_else(|| "latest".to_string()),
    );

    match zeroos_build::toolchain::install_musl_toolchain(&config) {
        Ok(paths) => {
            print_toolchain_paths(
                &paths,
                &format!(
                    "Install successful!\n\nToolchain installed at {}",
                    config.output_dir
                ),
            );
        }
        Err(e) => {
            eprintln!(
                r"
Install failed!

Error: {}

Common issues:
  - Missing: curl, tar
  - Network: can't reach GitHub Releases/API
  - Permissions: output dir not writable (use --output ~/path or --force)",
                e
            );
            exit(1);
        }
    }
}

fn print_toolchain_paths(paths: &zeroos_build::toolchain::ToolchainPaths, header: &str) {
    println!(
        r"
{}

Toolchain paths:
  Musl lib: {}
  GCC lib:  {}

Environment variables:
  export RISCV_MUSL_PATH={}
  export RISCV_GCC_PATH={}",
        header,
        paths.musl_lib.display(),
        paths.gcc_lib.display(),
        paths.musl_lib.display(),
        paths.gcc_lib.display()
    );
}

fn generate_target_command(cli_args: ZeroosGenerateTargetArgs) -> Result<()> {
    use zeroos_build::cmds::generate_target_spec;
    use zeroos_build::spec::{load_target_profile, parse_target_triple};

    debug!("Generating target spec with args: {:?}", cli_args.base);

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

fn generate_linker_command(cli_args: ZeroosGenerateLinkerArgs) -> Result<()> {
    use zeroos_build::cmds::generate_linker_script;

    debug!("Generating linker script with args: {:?}", cli_args.base);

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

fn build_command(args: ZeroosBuildArgs) -> Result<()> {
    use zeroos_build::cmds::{build_binary, find_workspace_root, get_or_build_toolchain, StdMode};

    debug!("build_command: {:?}", args);

    let workspace_root = find_workspace_root()?;
    debug!("workspace_root: {}", workspace_root.display());

    let fully = args.base.mode == StdMode::Std || args.base.fully;

    let toolchain_paths = if args.base.mode == StdMode::Std || fully {
        Some(get_or_build_toolchain(
            args.base.musl_lib_path.clone(),
            args.base.gcc_lib_path.clone(),
            fully,
        )?)
    } else {
        None
    };

    build_binary(&workspace_root, &args.base, toolchain_paths, None)?;

    Ok(())
}
