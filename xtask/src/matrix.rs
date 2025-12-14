use std::collections::BTreeMap;

use clap::Args;

use crate::sh::{ShOptions, StreamMode};

#[derive(Args, Debug)]
pub struct MatrixArgs {
    /// Path to YAML config (defaults to `<workspace>/matrix.yaml`)
    #[arg(long)]
    pub config: Option<std::path::PathBuf>,

    /// Which command to run. This can be either:
    /// - a name from `commands:` (recommended), or
    /// - an inline command template string.
    ///
    /// Per-entry `command:` overrides this.
    #[arg(long)]
    pub command: Option<String>,

    /// Only run matrix entries for these packages (repeatable).
    ///
    /// Example: `xtask matrix -p zeroos -p spike-platform --command check`
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
    /// `target: riscv64gc-unknown-linux-musl`
    One(String),
    /// `target: [ ... ]` (supports scalars and nested lists, so YAML aliases can expand cleanly)
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
    /// Per-entry overrides for named commands from the top-level `commands:` map.
    ///
    /// Example:
    ///   command: build
    ///   commands:
    ///     build: cargo spike build --package {package} --target "{target}" --no-default-features {features_flag} --quiet
    #[serde(default)]
    commands: BTreeMap<String, String>,
    command: Option<String>,
    package: String,
    target: Targets,
    #[serde(default)]
    features: Vec<FeatureSpec>,
}

fn load_config(path: &std::path::Path) -> Result<MatrixConfig, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    Ok(serde_yaml::from_slice(&bytes)?)
}

struct Step {
    name: String,
    cmd: String,
}

fn render_template(
    template: &str,
    workspace: &std::path::Path,
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

fn host_target() -> Result<String, Box<dyn std::error::Error>> {
    // `rustc -vV` prints a line like: `host: x86_64-unknown-linux-gnu`
    let opts = crate::sh::ShOptions {
        stdout: crate::sh::StreamMode::Pipe,
        stderr: crate::sh::StreamMode::Pipe,
        quiet: true,
        ..Default::default()
    };
    let out = crate::sh!(options(opts), "rustc", ["-vV"])?;
    let s = out.1;
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("host:") {
            return Ok(rest.trim().to_string());
        }
    }
    Err("rustc -vV output missing host line".into())
}

pub fn run(args: MatrixArgs) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = crate::findup::workspace_root()?;
    let config_path = args
        .config
        .clone()
        .unwrap_or_else(|| workspace.join("matrix.yaml"));
    let cfg = load_config(&config_path)?;

    let opts = ShOptions {
        stdout: StreamMode::Inherit,
        stderr: StreamMode::Inherit,
        cwd: Some(workspace.clone()),
        quiet: true,
    };

    let mut steps: Vec<Step> = Vec::new();

    for (i, cmd) in cfg.pre.iter().enumerate() {
        steps.push(Step {
            name: format!("pre:{}", i + 1),
            cmd: cmd.clone(),
        });
    }

    let default_cmd_name = args.command.clone();
    let host = host_target()?;

    for entry in &cfg.entries {
        if !args.packages.is_empty() && !args.packages.iter().any(|p| p == &entry.package) {
            continue;
        }

        let cmd_name = entry
            .command
            .as_ref()
            .or(default_cmd_name.as_ref())
            .ok_or_else(|| -> Box<dyn std::error::Error> {
                "no command selected (pass --command <name> or set `command:` per entry)".into()
            })?;
        // Command can be either:
        // - a key into `commands:` (recommended), optionally overridden by entry.commands, or
        // - an inline command template string.
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
        if args.verbose {
            println!("{}", step.cmd);
        }
        let out = crate::sh!(options(opts.clone()), &step.cmd)?;
        if args.verbose {
            debug_assert!(out.0.success());
            if !out.1.is_empty() {
                print!("{}", out.1);
            }
            if !out.2.is_empty() {
                eprint!("{}", out.2);
            }
        }
    }

    println!("[matrix] done");
    Ok(())
}
