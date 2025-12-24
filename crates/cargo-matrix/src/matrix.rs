use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct MatrixArgs {
    /// Path to YAML config (defaults to `<workspace>/matrix.yaml`)
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Which command to run. This can be either:
    /// - a name from `commands:` (recommended), or
    /// - an inline command template string.
    ///
    /// Per-entry `command:` overrides this.
    #[arg(long)]
    pub command: Option<String>,

    /// Only run matrix entries for these packages (repeatable).
    ///
    /// Example: `cargo matrix --command check -p zeroos -p spike-platform`
    #[arg(short = 'p', long = "package")]
    pub packages: Vec<String>,

    /// Print commands as they run
    #[arg(long)]
    pub verbose: bool,
}

#[derive(serde::Deserialize)]
struct MatrixConfig {
    #[serde(default)]
    pre: Vec<String>,
    #[serde(default)]
    commands: BTreeMap<String, String>,
    entries: Vec<MatrixEntry>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum Targets {
    One(String),
    Many(Vec<TargetElem>),
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum TargetElem {
    One(String),
    Many(Vec<TargetElem>),
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum FeatureSpec {
    One(String),
    OneOf(Vec<String>),
}

#[derive(serde::Deserialize)]
struct MatrixEntry {
    #[serde(default)]
    commands: BTreeMap<String, String>,
    command: Option<String>,
    package: String,
    target: Targets,
    #[serde(default)]
    features: Vec<FeatureSpec>,
}

fn load_config(path: &Path) -> Result<MatrixConfig, String> {
    let bytes =
        std::fs::read(path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    serde_yaml::from_slice(&bytes).map_err(|e| format!("Invalid YAML {}: {}", path.display(), e))
}

fn find_upwards(start: &Path, filename: &str) -> Option<PathBuf> {
    let mut dir = if start.is_dir() {
        start.to_path_buf()
    } else {
        start.parent().unwrap_or(start).to_path_buf()
    };

    loop {
        let candidate = dir.join(filename);
        if candidate.exists() {
            return Some(candidate);
        }

        if !dir.pop() {
            break;
        }
    }

    None
}

fn workspace_root() -> Result<PathBuf, String> {
    let start = std::env::current_dir().map_err(|e| format!("Failed to get cwd: {}", e))?;

    // Prefer matrix.yaml as a marker (works for external repos without Cargo.lock).
    if let Some(m) = find_upwards(&start, "matrix.yaml") {
        return Ok(m.parent().unwrap_or(m.as_path()).to_path_buf());
    }

    // Fallback to Cargo.lock for compatibility with workspaces.
    let lock = find_upwards(&start, "Cargo.lock").ok_or_else(|| {
        "matrix.yaml/Cargo.lock not found (run from within a repo or pass --config)".to_string()
    })?;
    Ok(lock.parent().unwrap_or(lock.as_path()).to_path_buf())
}

fn host_target(workspace: &Path) -> Result<String, String> {
    let out = Command::new("rustc")
        .arg("-vV")
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to run rustc -vV: {}", e))?;

    if !out.status.success() {
        return Err(format!(
            "rustc -vV failed (exit={:?}): {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        ));
    }

    let s = String::from_utf8_lossy(&out.stdout);
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("host:") {
            return Ok(rest.trim().to_string());
        }
    }
    Err("rustc -vV output missing host line".to_string())
}

fn render_template(
    template: &str,
    workspace: &Path,
    package: &str,
    target: &str,
    features: &str,
    features_flag: &str,
) -> String {
    template
        .replace("{workspace}", &workspace.to_string_lossy())
        .replace("{package}", package)
        .replace("{target}", target)
        .replace("{features}", features)
        .replace("{features_flag}", features_flag)
}

fn run_shell(cmd: &str, cwd: &Path, verbose: bool) -> Result<(), String> {
    if verbose {
        println!("$ {}", cmd);
    }

    let status = if cfg!(windows) {
        // Best-effort: allow running under Windows if a POSIX shell is available.
        if Command::new("sh")
            .arg("-c")
            .arg("true")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .current_dir(cwd)
                .status()
        } else {
            Command::new("cmd")
                .args(["/C", cmd])
                .current_dir(cwd)
                .status()
        }
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(cwd)
            .status()
    }
    .map_err(|e| format!("Failed to execute shell: {}", e))?;

    if !status.success() {
        return Err(format!(
            "Command failed (exit={:?}): {}",
            status.code(),
            cmd
        ));
    }
    Ok(())
}

struct Step {
    name: String,
    cmd: String,
}

pub fn run(args: MatrixArgs) -> Result<(), String> {
    let command = args.command.clone();

    let workspace = workspace_root()?;
    let config_path = args
        .config
        .clone()
        .unwrap_or_else(|| workspace.join("matrix.yaml"));
    let cfg = load_config(&config_path)?;

    let host = host_target(&workspace)?;

    let mut steps: Vec<Step> = Vec::new();
    for (i, cmd) in cfg.pre.iter().enumerate() {
        steps.push(Step {
            name: format!("pre:{}", i + 1),
            cmd: cmd.clone(),
        });
    }

    for entry in &cfg.entries {
        if !args.packages.is_empty() && !args.packages.iter().any(|p| p == &entry.package) {
            continue;
        }

        let cmd_name = entry
            .command
            .as_ref()
            .or(command.as_ref())
            .ok_or_else(|| "no command selected (pass --command <name>)".to_string())?;

        let template: &str = entry
            .commands
            .get(cmd_name)
            .or_else(|| cfg.commands.get(cmd_name))
            .map(|s| s.as_str())
            .unwrap_or(cmd_name);

        let mut combos: Vec<Vec<String>> = vec![Vec::new()];
        for spec in &entry.features {
            match spec {
                FeatureSpec::One(f) => {
                    for c in &mut combos {
                        c.push(f.clone());
                    }
                }
                FeatureSpec::OneOf(group) => {
                    let mut next: Vec<Vec<String>> = Vec::new();
                    for opt in group {
                        for c in &combos {
                            let mut nc = c.clone();
                            nc.push(opt.clone());
                            next.push(nc);
                        }
                    }
                    combos = next;
                }
            }
        }

        fn flatten_targets<'a>(t: &'a TargetElem, out: &mut Vec<&'a str>) {
            match t {
                TargetElem::One(s) => out.push(s.as_str()),
                TargetElem::Many(v) => {
                    for inner in v {
                        flatten_targets(inner, out);
                    }
                }
            }
        }

        let targets: Vec<&str> = match &entry.target {
            Targets::One(t) => vec![t.as_str()],
            Targets::Many(ts) => {
                let mut out: Vec<&str> = Vec::new();
                for t in ts {
                    flatten_targets(t, &mut out);
                }
                out
            }
        };

        for target in targets {
            let target = if target == "host" {
                host.as_str()
            } else {
                target
            };
            let total = combos.len();
            for (idx, mut feats) in combos.iter().cloned().enumerate() {
                feats.sort();
                feats.dedup();
                let feat_str = feats.join(",");
                let features_flag = if feat_str.is_empty() {
                    String::new()
                } else {
                    format!(r##"--features "{feat_str}""##)
                };

                let cmd = render_template(
                    template,
                    &workspace,
                    &entry.package,
                    target,
                    &feat_str,
                    &features_flag,
                );

                let suffix = if total > 1 {
                    format!(" #{}/{}", idx + 1, total)
                } else {
                    String::new()
                };

                steps.push(Step {
                    name: format!("{} [{target}] ({cmd_name}){suffix}", entry.package),
                    cmd,
                });
            }
        }
    }

    for (i, step) in steps.iter().enumerate() {
        println!("[{}/{}] {}", i + 1, steps.len(), step.name);
        run_shell(&step.cmd, &workspace, args.verbose)?;
    }

    Ok(())
}
