extern crate std;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tracing::{debug, info};

use super::{find_toolchain, ToolchainConfig, ToolchainPaths};

#[derive(Debug, Clone)]
pub struct InstallConfig {
    /// Toolchain architecture (e.g. "riscv64", "riscv32")
    pub arch: String,
    /// Install root directory (e.g. "~/.zeroos/musl")
    pub output_dir: String,
    /// GitHub repo in "owner/name" form.
    pub repo: Option<String>,
    /// Optional Git tag (e.g. "musl-toolchain-musl-1.2.3-gcc-9.4.0"). If not set, uses latest.
    pub tag: Option<String>,
    /// If true, replace any existing install.
    pub force: bool,
}

impl Default for InstallConfig {
    fn default() -> Self {
        let output_dir = dirs::home_dir()
            .map(|home| home.join(".zeroos/musl").to_string_lossy().to_string())
            .unwrap_or_else(|| "/usr/local".to_string());

        Self {
            arch: "riscv64".to_string(),
            output_dir,
            repo: None,
            tag: None,
            force: false,
        }
    }
}

fn host_platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "Darwin"
    } else if cfg!(target_os = "linux") {
        "Linux"
    } else if cfg!(target_os = "windows") {
        "Windows"
    } else {
        "Unknown"
    }
}

fn host_arch() -> &'static str {
    let a = std::env::consts::ARCH;
    match (host_platform(), a) {
        ("Darwin", "aarch64") => "arm64",
        ("Darwin", "x86_64") => "x86_64",
        ("Linux", "aarch64") => "aarch64",
        ("Linux", "x86_64") => "x86_64",
        ("Windows", "aarch64") => "arm64",
        ("Windows", "x86_64") => "x86_64",
        _ => a,
    }
}

fn default_repo_from_git() -> Option<String> {
    let out = Command::new("git")
        .args(["config", "--get", "remote.origin.url"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if url.is_empty() {
        return None;
    }

    // https://github.com/ORG/REPO.git
    if let Some(rest) = url.strip_prefix("https://github.com/") {
        let rest = rest.strip_suffix(".git").unwrap_or(rest);
        let mut parts = rest.split('/');
        let owner = parts.next()?;
        let repo = parts.next()?;
        return Some(format!("{}/{}", owner, repo));
    }

    // git@github.com:ORG/REPO.git
    if let Some(rest) = url.strip_prefix("git@github.com:") {
        let rest = rest.strip_suffix(".git").unwrap_or(rest);
        let mut parts = rest.split('/');
        let owner = parts.next()?;
        let repo = parts.next()?;
        return Some(format!("{}/{}", owner, repo));
    }

    None
}

fn run(cmd: &mut Command) -> Result<(), String> {
    debug!("Running command: {:?}", cmd);
    let status = cmd
        .status()
        .map_err(|e| format!("Failed to run {:?}: {}", cmd, e))?;
    if !status.success() {
        return Err(format!(
            "Command failed: {:?} (exit={:?})",
            cmd,
            status.code()
        ));
    }
    Ok(())
}

fn ensure_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("Failed to create dir {}: {}", path.display(), e))
}

fn github_token() -> Option<String> {
    std::env::var("GITHUB_TOKEN")
        .ok()
        .or_else(|| std::env::var("ZEROOS_GITHUB_TOKEN").ok())
        .filter(|s| !s.trim().is_empty())
}

fn add_github_api_headers(cmd: &mut Command) {
    cmd.arg("-H")
        .arg("Accept: application/vnd.github+json")
        .arg("-H")
        .arg("X-GitHub-Api-Version: 2022-11-28")
        .arg("-H")
        .arg("User-Agent: zeroos-build");

    if let Some(token) = github_token() {
        // Avoid GitHub API rate limits / anonymous restrictions on hosted runners.
        cmd.arg("-H")
            .arg(format!("Authorization: Bearer {}", token));
    }
}

fn find_asset_download_url(
    repo: &str,
    tag: Option<&str>,
    platform: &str,
    arch: &str,
) -> Result<String, String> {
    let api_url = if let Some(tag) = tag {
        format!(
            "https://api.github.com/repos/{}/releases/tags/{}",
            repo, tag
        )
    } else {
        format!("https://api.github.com/repos/{}/releases/latest", repo)
    };

    let tmp = tempfile::Builder::new()
        .prefix("zeroos-musl-release-")
        .tempfile()
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    let tmp_path = tmp.path().to_path_buf();

    let mut cmd = Command::new("curl");
    cmd.arg("-fsSL")
        .arg("--retry")
        .arg("5")
        .arg("--retry-all-errors")
        .arg("--retry-delay")
        .arg("1")
        .arg("-o")
        .arg(&tmp_path)
        .arg(&api_url);
    add_github_api_headers(&mut cmd);
    run(&mut cmd)?;

    let bytes = fs::read(&tmp_path).map_err(|e| {
        format!(
            "Failed to read GitHub API response {}: {}",
            tmp_path.display(),
            e
        )
    })?;
    let v: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| format!("Invalid JSON from GitHub API: {}", e))?;

    let assets = v
        .get("assets")
        .and_then(|a| a.as_array())
        .ok_or_else(|| "GitHub API response missing `assets` array".to_string())?;

    let suffix = format!("-{}-{}.tar.gz", platform, arch);
    for asset in assets {
        let name = asset.get("name").and_then(|n| n.as_str()).unwrap_or("");
        if !name.starts_with("zeroos-musl-toolchain-") || !name.ends_with(&suffix) {
            continue;
        }
        let url = asset
            .get("browser_download_url")
            .and_then(|u| u.as_str())
            .ok_or_else(|| format!("Asset {} missing browser_download_url", name))?;
        return Ok(url.to_string());
    }

    Err(format!(
        "No matching toolchain asset found for {} {} in repo {} (tag={:?})",
        platform, arch, repo, tag
    ))
}

pub fn install_musl_toolchain(config: &InstallConfig) -> Result<ToolchainPaths, String> {
    let platform = host_platform();
    let arch = host_arch();

    let repo = config
        .repo
        .clone()
        .or_else(|| std::env::var("ZEROOS_MUSL_TOOLCHAIN_REPO").ok())
        .or_else(default_repo_from_git)
        .unwrap_or_else(|| "LayerZero-Labs/ZeroOS".to_string());

    let output_dir = PathBuf::from(&config.output_dir);
    let target_dir = output_dir.join(format!("{}-linux-musl", config.arch));

    if target_dir.exists() && !config.force {
        info!(
            "Toolchain already present at {} (arch={}); skipping install",
            target_dir.display(),
            config.arch
        );
        let toolchain_config = ToolchainConfig {
            arch: config.arch.clone(),
            search_dirs: vec![output_dir.clone()],
        };
        return find_toolchain(&toolchain_config)
            .ok_or_else(|| format!("Existing toolchain not valid at {}", target_dir.display()));
    }

    info!(
        "Installing musl toolchain from GitHub Releases\n  Repo:     {}\n  Tag:      {}\n  Host:     {} {}\n  Target:   {}-linux-musl\n  Output:   {}",
        repo,
        config.tag.clone().unwrap_or_else(|| "latest".to_string()),
        platform,
        arch,
        config.arch,
        output_dir.display(),
    );

    let url = find_asset_download_url(&repo, config.tag.as_deref(), platform, arch)?;
    info!("Downloading: {}", url);

    let tmp_dir = tempfile::Builder::new()
        .prefix("zeroos-musl-install-")
        .tempdir()
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let tmp_dir_path = tmp_dir.path().to_path_buf();
    let tarball = tmp_dir_path.join("toolchain.tar.gz");

    let mut dl = Command::new("curl");
    dl.arg("-fL")
        .arg("--retry")
        .arg("5")
        .arg("--retry-all-errors")
        .arg("--retry-delay")
        .arg("1")
        .arg("-o")
        .arg(&tarball)
        .arg(&url);
    run(&mut dl)?;

    // Extract into temp, then move into place.
    run(Command::new("tar")
        .arg("xzf")
        .arg(&tarball)
        .arg("-C")
        .arg(&tmp_dir_path))?;

    let extracted_root = tmp_dir_path.join("musl");
    if !extracted_root.exists() {
        return Err(format!(
            "Unexpected archive layout: expected {} to exist",
            extracted_root.display()
        ));
    }

    ensure_dir(output_dir.parent().unwrap_or(Path::new("/")))?;

    if output_dir.exists() {
        if config.force {
            fs::remove_dir_all(&output_dir)
                .map_err(|e| format!("Failed to remove {}: {}", output_dir.display(), e))?;
        } else {
            return Err(format!(
                "Output directory already exists: {} (use --force to replace)",
                output_dir.display()
            ));
        }
    }

    fs::rename(&extracted_root, &output_dir).map_err(|e| {
        format!(
            "Failed to move installed toolchain into place ({} -> {}): {}",
            extracted_root.display(),
            output_dir.display(),
            e
        )
    })?;

    let toolchain_config = ToolchainConfig {
        arch: config.arch.clone(),
        search_dirs: vec![output_dir.clone()],
    };

    find_toolchain(&toolchain_config)
        .ok_or_else(|| format!("Installed toolchain not found at {}", output_dir.display()))
}

/// Resolve the toolchain paths from args/env/default locations; if missing, try to install from
/// GitHub Releases (no source build fallback here).
pub fn get_or_install_toolchain(
    musl_lib_arg: Option<PathBuf>,
    gcc_lib_arg: Option<PathBuf>,
    config: &ToolchainConfig,
    install: &InstallConfig,
) -> Result<ToolchainPaths, String> {
    use super::resolve_toolchain_paths;

    match resolve_toolchain_paths(musl_lib_arg, gcc_lib_arg, config) {
        Ok(paths) => Ok(paths),
        Err(_e) => install_musl_toolchain(install),
    }
}
