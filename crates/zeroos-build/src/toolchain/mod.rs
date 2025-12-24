extern crate std;

mod discovery;
mod install;

pub use discovery::{discover_toolchain, validate_toolchain_path, ToolchainPaths};
pub use install::{get_or_install_toolchain, install_musl_toolchain, InstallConfig};

use std::format;
use std::path::{Path, PathBuf};
use std::string::{String, ToString};

use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct ToolchainConfig {
    pub arch: String,

    pub search_dirs: Vec<PathBuf>,
}

impl Default for ToolchainConfig {
    fn default() -> Self {
        Self {
            arch: "riscv64".to_string(),
            search_dirs: Vec::new(),
        }
    }
}

pub fn find_toolchain(config: &ToolchainConfig) -> Option<ToolchainPaths> {
    for search_dir in &config.search_dirs {
        let toolchain_base = search_dir.join(format!("{}-linux-musl", config.arch));
        if let Ok(paths) = validate_toolchain_path(&toolchain_base, &config.arch) {
            return Some(ToolchainPaths {
                musl_lib: paths.0,
                gcc_lib: paths.1,
            });
        }
    }

    None
}

pub fn resolve_toolchain_paths(
    musl_lib_arg: Option<PathBuf>,
    gcc_lib_arg: Option<PathBuf>,
    config: &ToolchainConfig,
) -> std::result::Result<ToolchainPaths, std::string::String> {
    if let (Some(musl_lib), Some(gcc_lib)) = (musl_lib_arg.clone(), gcc_lib_arg.clone()) {
        validate_musl_lib(&musl_lib)?;
        validate_gcc_lib(&gcc_lib)?;

        return Ok(ToolchainPaths { musl_lib, gcc_lib });
    }

    if let Some(musl_lib) = musl_lib_arg {
        validate_musl_lib(&musl_lib)?;

        let base = musl_lib
            .parent()
            .ok_or_else(|| "Invalid musl lib path: no parent directory".to_string())?;
        let gcc_base = base
            .join("lib/gcc")
            .join(format!("{}-linux-musl", config.arch));

        if gcc_base.exists() {
            if let Ok(gcc_lib) = find_gcc_version_dir(&gcc_base) {
                return Ok(ToolchainPaths { musl_lib, gcc_lib });
            }
        }

        if let Some(gcc_lib) = gcc_lib_arg {
            validate_gcc_lib(&gcc_lib)?;
            return Ok(ToolchainPaths { musl_lib, gcc_lib });
        }

        return Err("Could not find GCC library relative to musl lib.\n\
             Please specify gcc_lib_path or install toolchain completely."
            .to_string());
    }

    discover_toolchain(&config.arch)
        .ok_or_else(|| format!("Toolchain not found for architecture: {}", config.arch))
}

fn validate_musl_lib(musl_lib: &Path) -> std::result::Result<(), std::string::String> {
    if !musl_lib.join("libc.a").exists() {
        return Err(format!(
            "Invalid musl lib path: libc.a not found in {}",
            musl_lib.display()
        ));
    }
    Ok(())
}

fn validate_gcc_lib(gcc_lib: &Path) -> std::result::Result<(), std::string::String> {
    if !gcc_lib.join("libgcc.a").exists() {
        return Err(format!(
            "Invalid gcc lib path: libgcc.a not found in {}",
            gcc_lib.display()
        ));
    }
    Ok(())
}

fn find_gcc_version_dir(gcc_base: &Path) -> std::result::Result<PathBuf, std::string::String> {
    if !gcc_base.exists() {
        return Err(format!(
            "GCC base directory not found: {}",
            gcc_base.display()
        ));
    }

    let entries =
        std::fs::read_dir(gcc_base).map_err(|e| format!("Failed to read GCC directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && path.join("libgcc.a").exists() {
            return Ok(path);
        }
    }

    Err(format!(
        "No GCC version directory with libgcc.a found in {}",
        gcc_base.display()
    ))
}

#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub arch: String,

    pub output_dir: String,
    /// GCC configuration flags (e.g., "--with-arch=rv64ima --with-abi=lp64")
    pub gcc_config: Option<String>,

    pub jobs: Option<usize>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        let output_dir = dirs::home_dir()
            .map(|home| home.join(".zeroos/musl").to_string_lossy().to_string())
            .unwrap_or_else(|| "/usr/local".to_string());

        Self {
            arch: "riscv64".to_string(),
            output_dir,
            gcc_config: Some("--with-arch=rv64ima --with-abi=lp64".to_string()),
            jobs: None,
        }
    }
}

pub fn build_musl_toolchain(
    config: &BuildConfig,
) -> std::result::Result<ToolchainPaths, std::string::String> {
    use std::fs;
    use std::io::Write;
    use std::process::{Command, Stdio};

    let temp_dir = tempfile::Builder::new()
        .prefix(&format!("zeroos-musl-build-{}-", config.arch))
        .tempdir()
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;
    let temp_dir_path = temp_dir.path().to_path_buf();

    let script_path = temp_dir_path.join("musl-toolchain.sh");
    let script_content = include_str!("../files/musl-toolchain.sh");

    let mut file = fs::File::create(&script_path)
        .map_err(|e| format!("Failed to create script file: {}", e))?;
    file.write_all(script_content.as_bytes())
        .map_err(|e| format!("Failed to write script file: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path)
            .map_err(|e| format!("Failed to get script permissions: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms)
            .map_err(|e| format!("Failed to set script permissions: {}", e))?;
    }

    let mut cmd = Command::new("bash");
    cmd.arg(&script_path)
        .arg(&config.arch)
        .current_dir(&temp_dir_path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    cmd.env("OUTPUT", &config.output_dir);

    if let Some(ref gcc_config) = config.gcc_config {
        cmd.env("GCC_CONFIG_FOR_TARGET", gcc_config);
    }

    cmd.env("WORKDIR", &temp_dir_path);

    info!("Building musl toolchain for {}", config.arch);
    info!("This will take 5-10 minutes.");
    info!("Output directory: {}", config.output_dir);
    debug!(
        "Running: WORKDIR={} OUTPUT={} GCC_CONFIG_FOR_TARGET=\"{}\" bash {} {}",
        temp_dir_path.display(),
        config.output_dir,
        config.gcc_config.as_ref().unwrap_or(&"(none)".to_string()),
        script_path.display(),
        config.arch
    );

    let status = cmd
        .status()
        .map_err(|e| format!("Failed to execute build script: {}", e))?;

    if !status.success() {
        return Err(format!(
            "Build failed with exit code {:?}. See log above for details.",
            status.code()
        ));
    }

    info!("Build completed successfully");

    let _ = fs::remove_dir_all(&temp_dir);

    let toolchain_config = ToolchainConfig {
        arch: config.arch.clone(),
        search_dirs: vec![PathBuf::from(&config.output_dir)],
    };

    find_toolchain(&toolchain_config)
        .ok_or_else(|| format!("Built toolchain not found at {}", config.output_dir))
}
