use anyhow::{Context, Result};
use clap::Args;
use log::debug;
use std::path::{Path, PathBuf};
use std::process::Command;

use build::cmds::build::{TARGET_NO_STD, TARGET_STD};
use build::cmds::{BuildArgs, StdMode};

#[derive(Args, Debug)]
pub struct SpikeBuildArgs {
    #[command(flatten)]
    pub base: BuildArgs,

    /// Emit the fully-rendered linker script used for this build.
    ///
    /// Supports placeholders: `<WORKSPACE>`, `<TARGET>`, `<PROFILE>`, `<PACKAGE>`.
    #[arg(
        long,
        value_name = "PATH",
        default_missing_value = "<PACKAGE>/linker.ld.<PROFILE>"
    )]
    pub emit_linker_script: Option<String>,

    /// Overwrite the emitted linker script if it already exists.
    #[arg(long)]
    pub force: bool,
}

pub fn build_command(args: SpikeBuildArgs) -> Result<()> {
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

    if let Some(out_tpl) = &args.emit_linker_script {
        emit_linker_script(&workspace_root, &args.base, out_tpl, args.force)?;
    }

    Ok(())
}

fn emit_linker_script(
    workspace_root: &Path,
    base: &BuildArgs,
    out_tpl: &str,
    force: bool,
) -> Result<()> {
    let target = base.target.as_deref().unwrap_or(match base.mode {
        StdMode::Std => TARGET_STD,
        StdMode::NoStd => TARGET_NO_STD,
    });
    let profile = build::project::detect_profile(&base.cargo_args);

    let target_dir = build::project::get_target_directory(&workspace_root.to_path_buf())?;
    let generated_linker = target_dir
        .join(target)
        .join(&profile)
        .join("zeroos")
        .join(&base.package)
        .join("linker.ld");

    if !generated_linker.exists() {
        anyhow::bail!(
            "Generated linker script not found (expected {}); was the build successful?",
            generated_linker.display()
        );
    }

    let out_path_str = expand_emit_path(
        out_tpl,
        workspace_root,
        &resolve_package_dir(workspace_root, &base.package)?,
        target,
        &profile,
        &base.package,
    );
    let out_path_raw = PathBuf::from(out_path_str);
    let out_path = out_path_raw;

    if out_path.exists() && !force {
        anyhow::bail!(
            "Refusing to overwrite existing linker script: {} (use --force)",
            out_path.display()
        );
    }
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    std::fs::copy(&generated_linker, &out_path).with_context(|| {
        format!(
            "Failed to copy linker script from {} to {}",
            generated_linker.display(),
            out_path.display()
        )
    })?;

    Ok(())
}

fn expand_emit_path(
    template: &str,
    workspace: &Path,
    package_dir: &Path,
    target: &str,
    profile: &str,
    _package: &str,
) -> String {
    template
        .replace("<WORKSPACE>", &workspace.display().to_string())
        .replace("<PACKAGE>", &package_dir.display().to_string())
        .replace("<TARGET>", target)
        .replace("<PROFILE>", profile)
}

fn resolve_package_dir(workspace_root: &Path, package_name: &str) -> Result<PathBuf> {
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
        if pkg.get("name").and_then(|n| n.as_str()) != Some(package_name) {
            continue;
        }
        let manifest = pkg
            .get("manifest_path")
            .and_then(|m| m.as_str())
            .ok_or_else(|| anyhow::anyhow!("cargo metadata JSON: package missing manifest_path"))?;
        let manifest_dir = Path::new(manifest)
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid manifest_path for {}", package_name))?;
        return Ok(manifest_dir.to_path_buf());
    }

    anyhow::bail!("Package not found in cargo metadata: {}", package_name)
}

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
