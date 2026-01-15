use clap::Args;

use std::ffi::OsString;
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;

#[derive(Args, Debug)]
pub struct ActArgs {
    /// Arguments to pass through to the `act` CLI.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

fn is_unix_socket(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.file_type().is_socket())
        .unwrap_or(false)
}

fn discover_remote_containers_ipc_from_devcontainers_env() -> Option<OsString> {
    // Prefer the env file that VS Code Dev Containers generates for interactive shells:
    //   /tmp/devcontainers-*/env-loginInteractiveShell.json
    let tmp = Path::new("/tmp");
    let mut candidates: Vec<PathBuf> = Vec::new();

    let entries = std::fs::read_dir(tmp).ok()?;
    for e in entries.flatten() {
        let name = e.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("devcontainers-") {
            continue;
        }
        let p = e.path().join("env-loginInteractiveShell.json");
        if p.is_file() {
            candidates.push(p);
        }
    }
    candidates.sort();

    for p in candidates {
        let raw = std::fs::read_to_string(&p).ok()?;
        let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
        let ipc = v.get("REMOTE_CONTAINERS_IPC")?.as_str()?;
        let ipc_path = Path::new(ipc);
        if is_unix_socket(ipc_path) {
            return Some(OsString::from(ipc));
        }
    }

    None
}

fn discover_remote_containers_ipc_from_tmp_socket_glob() -> Option<OsString> {
    // Fallback: look for /tmp/vscode-remote-containers-ipc-*.sock
    let tmp = Path::new("/tmp");
    let entries = std::fs::read_dir(tmp).ok()?;

    let mut sockets: Vec<PathBuf> = Vec::new();
    for e in entries.flatten() {
        let name = e.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("vscode-remote-containers-ipc-") && name.ends_with(".sock") {
            sockets.push(e.path());
        }
    }
    sockets.sort();

    for s in sockets {
        if is_unix_socket(&s) {
            return Some(s.into_os_string());
        }
    }
    None
}

fn maybe_inject_remote_containers_ipc(cmd: &mut std::process::Command) {
    if std::env::var_os("REMOTE_CONTAINERS_IPC").is_some() {
        return;
    }

    let ipc = discover_remote_containers_ipc_from_devcontainers_env()
        .or_else(discover_remote_containers_ipc_from_tmp_socket_glob);
    if let Some(ipc) = ipc {
        cmd.env("REMOTE_CONTAINERS_IPC", ipc);
    }
}

fn home_local_bin_dirs() -> Option<Vec<PathBuf>> {
    let home = std::env::var_os("HOME")?;
    let home = Path::new(&home);
    Some(vec![home.join(".local/bin"), home.join("bin")])
}

fn append_dirs_to_path(cmd: &mut std::process::Command, dirs: &[PathBuf]) {
    // Filter out missing dirs to keep PATH tidy (and avoid surprising entries).
    let mut dirs: Vec<PathBuf> = dirs.iter().filter(|p| p.is_dir()).cloned().collect();
    if dirs.is_empty() {
        return;
    }

    let existing = std::env::var_os("PATH").unwrap_or_default();
    let mut paths: Vec<PathBuf> = std::env::split_paths(&existing).collect();

    // Avoid duplicates: remove any existing occurrences of the dirs we’re about to append.
    paths.retain(|p| !dirs.iter().any(|d| d == p));

    // Append in the provided order.
    paths.append(&mut dirs);

    if let Ok(joined) = std::env::join_paths(paths) {
        cmd.env("PATH", joined);
    }
}

pub fn run(args: ActArgs) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = crate::findup::workspace_root()?;

    let mut cmd = std::process::Command::new("act");
    cmd.args(&args.args)
        .current_dir(workspace)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    // Make it easy to find locally installed CLIs (common in Dev Containers).
    // We *append* these so we don't override anything already on PATH; they act as a fallback.
    // Order matters: ~/.local/bin is typically preferred over ~/bin.
    if let Some(extra_dirs) = home_local_bin_dirs() {
        append_dirs_to_path(&mut cmd, &extra_dirs);
    }

    // In Dev Containers, Docker may be configured to use the dev-containers credential helper:
    //   ~/.docker/config.json: { "credsStore": "dev-containers-<id>" }
    // That helper expects REMOTE_CONTAINERS_IPC to be set; Cursor "attach" shells may not have it.
    maybe_inject_remote_containers_ipc(&mut cmd);

    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err("`act` not found in PATH".into());
        }
        Err(e) => return Err(e.into()),
    };

    if !status.success() {
        return Err(format!("`act` failed with exit code {:?}", status.code()).into());
    }
    Ok(())
}
