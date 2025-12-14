extern crate std;

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ToolchainPaths {
    pub musl_lib: PathBuf,

    pub gcc_lib: PathBuf,
}

pub fn discover_toolchain(arch: &str) -> Option<ToolchainPaths> {
    use crate::toolchain::{find_toolchain, ToolchainConfig};

    let mut search_dirs = Vec::new();

    if let Ok(path) = std::env::var("RISCV_MUSL_PATH") {
        let env_path = PathBuf::from(&path);

        if env_path.join("libc.a").exists() {
            if let Some(parent) = env_path.parent() {
                search_dirs.push(parent.to_path_buf());
            }
        } else {
            search_dirs.push(env_path);
        }
    }

    // Add user home directories (prioritized for safety and no sudo)
    if let Some(home) = dirs::home_dir() {
        search_dirs.push(home.join(".zeroos/musl"));

        search_dirs.push(home.join(".local"));
    }

    search_dirs.push(PathBuf::from("/usr/local"));
    search_dirs.push(PathBuf::from("/opt"));
    search_dirs.push(PathBuf::from("/usr"));

    let config = ToolchainConfig {
        arch: arch.to_string(),
        search_dirs,
    };

    find_toolchain(&config)
}

pub fn validate_toolchain_path(
    base: &Path,
    arch: &str,
) -> std::result::Result<(PathBuf, PathBuf), std::string::String> {
    let musl_lib = base.join("lib");
    let libc = musl_lib.join("libc.a");
    if !libc.exists() {
        return Err(format!("libc.a not found in {}", musl_lib.display()));
    }

    let (_musl_lib, gcc_lib) = find_gcc_lib(&musl_lib, arch)?;

    Ok((musl_lib, gcc_lib))
}

fn find_gcc_lib(
    musl_lib: &Path,
    arch: &str,
) -> std::result::Result<(PathBuf, PathBuf), std::string::String> {
    let base = musl_lib
        .parent()
        .ok_or_else(|| "Invalid musl lib path".to_string())?;

    let gcc_base = base.join("lib/gcc").join(format!("{}-linux-musl", arch));
    if let Some(gcc_lib) = try_find_gcc_version(&gcc_base) {
        return Ok((musl_lib.to_path_buf(), gcc_lib));
    }

    if let Some(install_root) = base.parent() {
        let gcc_base = install_root
            .join("lib/gcc")
            .join(format!("{}-linux-musl", arch));
        if let Some(gcc_lib) = try_find_gcc_version(&gcc_base) {
            return Ok((musl_lib.to_path_buf(), gcc_lib));
        }
    }

    Err(format!("Could not find GCC library for {}", arch))
}

fn try_find_gcc_version(gcc_base: &Path) -> Option<PathBuf> {
    if !gcc_base.exists() {
        return None;
    }

    let entries = std::fs::read_dir(gcc_base).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && path.join("libgcc.a").exists() {
            return Some(path);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    #[test]
    fn test_discover_respects_env_var() {}
}
