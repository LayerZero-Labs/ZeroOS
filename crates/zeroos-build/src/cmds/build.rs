use anyhow::{Context, Result};
use log::{debug, info};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

use crate::spec::TargetRenderOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum StdMode {
    Std,
    NoStd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum BacktraceMode {
    Auto,
    Enable,
    Disable,
}

#[derive(clap::Args, Debug, Clone)]
pub struct BuildArgs {
    #[arg(long, short = 'p')]
    pub package: String,

    /// Backtrace policy for the guest.
    #[arg(long, value_enum, default_value = "auto")]
    pub backtrace: BacktraceMode,

    #[arg(long, default_value = "0x80000000")]
    pub memory_origin: String,

    #[arg(long, default_value = "128Mi")]
    pub memory_size: String,

    #[arg(long, default_value = "8Mi")]
    pub stack_size: String,

    #[arg(long, default_value = "64Mi")]
    pub heap_size: String,

    #[arg(long, value_enum, default_value = "no-std")]
    pub mode: StdMode,

    #[arg(long)]
    pub target: Option<String>,

    #[arg(long)]
    pub fully: bool,

    #[arg(long, env = "RISCV_MUSL_PATH")]
    pub musl_lib_path: Option<PathBuf>,

    #[arg(long, env = "RISCV_GCC_PATH")]
    pub gcc_lib_path: Option<PathBuf>,

    /// Arguments after `--` are forwarded to the underlying `cargo build` invocation.
    ///
    /// Example:
    ///   `cargo spike build -p foo --target ... --mode std -- --release --quiet`
    #[arg(trailing_var_arg = true)]
    pub cargo_args: Vec<String>,
}

pub const TARGET_NO_STD: &str = "riscv64imac-unknown-none-elf";

pub const TARGET_STD: &str = "riscv64imac-zero-linux-musl";

pub fn build_binary(
    workspace_root: &PathBuf,
    args: &BuildArgs,
    toolchain_paths: Option<(PathBuf, PathBuf)>,
    linker_template: Option<String>,
) -> Result<()> {
    info!(
        "Building binary for {:?} mode (fully: {})",
        args.mode, args.fully
    );
    debug!("Building package: {}", args.package);

    let memory_origin = parse_address(&args.memory_origin)?;
    let memory_size = parse_size::parse_size(&args.memory_size)? as usize;
    let stack_size = parse_size::parse_size(&args.stack_size)? as usize;
    let heap_size = parse_size::parse_size(&args.heap_size)? as usize;

    debug!("memory_origin: 0x{:x}", memory_origin);
    debug!("memory_size: 0x{:x} ({} bytes)", memory_size, memory_size);
    debug!("stack_size: 0x{:x} ({} bytes)", stack_size, stack_size);
    debug!("heap_size: 0x{:x} ({} bytes)", heap_size, heap_size);

    let default_target = match args.mode {
        StdMode::Std => TARGET_STD,
        StdMode::NoStd => TARGET_NO_STD,
    };
    let target = args.target.as_deref().unwrap_or(default_target);

    let build_std_arg = match (args.mode, args.fully) {
        (StdMode::Std, _) => Some("-Zbuild-std=core,alloc,std,panic_abort"),
        (StdMode::NoStd, true) => Some("-Zbuild-std=core,alloc,panic_abort"),
        (StdMode::NoStd, false) => None,
    };

    debug!("target: {}", target);
    debug!("build_std_arg: {:?}", build_std_arg);

    let target_dir = crate::project::get_target_directory(workspace_root)?;

    let profile = crate::project::detect_profile(&args.cargo_args);
    let backtrace_enabled = should_enable_backtrace(args, &profile);

    debug!("target_dir: {}", target_dir.display());
    debug!("target: {}", target);
    debug!("profile: {}", profile);

    let out_dir = target_dir.join(target).join(&profile);
    let crate_out_dir = out_dir.join("zeroos").join(&args.package);
    fs::create_dir_all(&crate_out_dir)?;
    let linker_script_path = crate_out_dir.join("linker.ld");

    let config = crate::linker::LinkerConfig::new()
        .with_memory(memory_origin, memory_size)
        .with_stack_size(stack_size)
        .with_heap_size(heap_size)
        .with_backtrace(backtrace_enabled);

    let config = if let Some(template) = linker_template {
        config.with_template(template)
    } else {
        config
    };

    write_linker_script(&linker_script_path, config)?;

    // For std mode with the built-in target profile, RUST_TARGET_PATH must be set.
    let rust_target_path = std::env::var("RUST_TARGET_PATH")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            if args.mode == StdMode::Std && target == TARGET_STD {
                let target_spec_path = crate_out_dir.join(format!("{}.json", target));
                write_target_spec(
                    target_spec_path,
                    target,
                    TargetRenderOptions {
                        backtrace: backtrace_enabled,
                    },
                )
                .ok();
                Some(crate_out_dir.clone())
            } else {
                None
            }
        });

    let mut link_args = vec![
        format!("-T{}", linker_script_path.display()),
        "--wrap=__lock".to_string(),
        "--wrap=__unlock".to_string(),
        "--wrap=__lockfile".to_string(),
        "--wrap=__unlockfile".to_string(),
    ];

    if let Some((musl_lib, gcc_lib)) = toolchain_paths {
        info!("Using musl lib: {}", musl_lib.display());
        info!("Using gcc lib:  {}", gcc_lib.display());

        link_args.extend(vec![
            format!("-L{}", musl_lib.display()),
            format!("-L{}", gcc_lib.display()),
            "-lgcc".to_string(),
        ]);
    }

    debug!("link_args count: {}", link_args.len());
    for (i, arg) in link_args.iter().enumerate() {
        debug!("  link_arg[{}]: {}", i, arg);
    }

    let mut rustflags_parts: Vec<String> = std::env::var("CARGO_ENCODED_RUSTFLAGS")
        .ok()
        .map(|s| s.split('\x1f').map(|s| s.to_string()).collect())
        .unwrap_or_default();

    // In unwind-table-based backtraces, we need DWARF CFI tables even with `panic=abort`.
    // This forces `.eh_frame` emission for Rust code when backtraces are enabled.
    if args.mode == StdMode::Std && backtrace_enabled {
        rustflags_parts.push("-C".to_string());
        rustflags_parts.push("force-unwind-tables=yes".to_string());
    }

    for arg in &link_args {
        rustflags_parts.push("-C".to_string());
        rustflags_parts.push(format!("link-arg={}", arg));
        rustflags_parts.push("-Zmacro-backtrace".to_string());
    }

    let encoded_rustflags = rustflags_parts.join("\x1f");
    debug!("CARGO_ENCODED_RUSTFLAGS: {:?}", encoded_rustflags);

    if let Some(ref path) = rust_target_path {
        debug!("RUST_TARGET_PATH: {}", path.display());
    }

    let mut cmd = Command::new("cargo");
    cmd.current_dir(workspace_root);
    debug!("cargo working directory: {}", workspace_root.display());

    cmd.env("CARGO_ENCODED_RUSTFLAGS", &encoded_rustflags);
    cmd.env("RUSTC_BOOTSTRAP", "1");

    if let Some(target_path) = &rust_target_path {
        cmd.env("RUST_TARGET_PATH", target_path);
    }

    cmd.arg("build");
    cmd.arg("--target").arg(target);

    cmd.arg("-p").arg(&args.package);

    if let Some(build_std) = build_std_arg {
        cmd.arg(build_std);
    }

    cmd.args(&args.cargo_args);

    let cargo_args_vec: Vec<String> = cmd
        .get_args()
        .map(|s| s.to_string_lossy().to_string())
        .collect();
    let cargo_args_str = cargo_args_vec.join(" ");
    debug!("cargo command: cargo {}", cargo_args_str);

    let status = cmd.status().context("Failed to execute cargo build")?;

    if !status.success() {
        exit(status.code().unwrap_or(1));
    }

    Ok(())
}

fn write_target_spec(
    target_spec_path: impl AsRef<Path>,
    target: &str,
    render_opts: TargetRenderOptions,
) -> Result<(), anyhow::Error> {
    let path = target_spec_path.as_ref();
    debug!("Writing target spec to: {}", path.display());
    let target_spec_json = crate::cmds::generate_target_spec(
        &GenerateTargetArgs {
            profile: Some(target.to_string()),
            ..Default::default()
        },
        render_opts,
    )
    .map_err(|e| anyhow::anyhow!("Failed to generate target spec: {}", e))
    .unwrap();
    fs::write(path, target_spec_json)?;
    debug!("Target spec written successfully");
    Ok(())
}

fn write_linker_script(
    linker_script_path: impl AsRef<Path>,
    config: crate::linker::LinkerConfig,
) -> Result<(), anyhow::Error> {
    let path = linker_script_path.as_ref();
    debug!("Generating linker script: {}", path.display());
    let script_content = config.render(None);
    fs::write(path, script_content)?;
    debug!("Linker script written successfully");
    Ok(())
}

pub fn get_or_build_toolchain(
    musl_lib_arg: Option<PathBuf>,
    gcc_lib_arg: Option<PathBuf>,
    fully: bool,
) -> Result<(PathBuf, PathBuf)> {
    use crate::toolchain::{
        build_musl_toolchain, resolve_toolchain_paths, BuildConfig, ToolchainConfig,
    };

    let config = ToolchainConfig::default();

    match resolve_toolchain_paths(musl_lib_arg.clone(), gcc_lib_arg.clone(), &config) {
        Ok(paths) => Ok((paths.musl_lib, paths.gcc_lib)),
        Err(e) => {
            if fully {
                eprintln!("RISC-V musl toolchain not found: {}", e);
                eprintln!();
                eprintln!("Building toolchain (this will take 5-15 minutes)...");
                eprintln!("Output: ~/.zeroos/musl");
                eprintln!();

                let build_config = BuildConfig::default();
                let paths = build_musl_toolchain(&build_config)
                    .map_err(|e| anyhow::anyhow!("Failed to build toolchain: {}", e))?;

                eprintln!("Toolchain built successfully!");
                eprintln!();

                Ok((paths.musl_lib, paths.gcc_lib))
            } else {
                eprintln!("Error: RISC-V musl toolchain not found: {}", e);
                eprintln!();
                eprintln!("To build the toolchain:");
                eprintln!("  cargo zeroos build-musl");
                eprintln!();
                eprintln!("Or use --fully to build automatically:");
                eprintln!("  cargo zeroos build --fully");
                eprintln!();
                eprintln!("This installs to ~/.zeroos/musl by default (no sudo required).");
                anyhow::bail!("Toolchain not found");
            }
        }
    }
}

pub fn parse_address(s: &str) -> Result<usize> {
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        usize::from_str_radix(hex, 16)
    } else {
        s.parse::<usize>()
    }
    .with_context(|| format!("Invalid address: {}", s))
}

use crate::cmds::GenerateTargetArgs;

pub use crate::project::find_workspace_root;

fn should_enable_backtrace(args: &BuildArgs, profile: &str) -> bool {
    match args.backtrace {
        BacktraceMode::Enable => true,
        BacktraceMode::Disable => false,
        BacktraceMode::Auto => {
            // Default split:
            // - debug/dev profiles: on
            // - release/other profiles: off
            matches!(profile, "debug" | "dev")
        }
    }
}
