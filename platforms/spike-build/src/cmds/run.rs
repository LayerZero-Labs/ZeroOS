use anyhow::{Context, Result};
use clap::Args;
use log::debug;
use std::path::{Path, PathBuf};
use std::process::{exit, Command, Stdio};
use std::{io::BufRead, io::BufReader, io::Write};

use build::host::backtrace as sym;

#[derive(Args, Debug)]
pub struct RunArgs {
    #[arg(value_name = "BINARY")]
    pub binary: PathBuf,

    /// Path to spike executable (defaults to `spike` in PATH; falls back to common install dirs)
    #[arg(long, env = "SPIKE_PATH")]
    pub spike: Option<PathBuf>,

    #[arg(long, default_value = "RV64IMAC")]
    pub isa: String,

    #[arg(long, short = 'n', default_value = "1000000")]
    pub instructions: u64,

    /// Symbolize `stack backtrace:` frame addresses using addr2line on the host
    #[arg(long, default_value_t = true)]
    pub symbolize_backtrace: bool,

    /// Path to addr2line binary (defaults to `riscv64-unknown-elf-addr2line` if found)
    #[arg(long, env = "RISCV_ADDR2LINE")]
    pub addr2line: Option<PathBuf>,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub spike_args: Vec<String>,
}

pub fn run_command(args: RunArgs) -> Result<()> {
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

    let spike_path = resolve_spike(args.spike.as_deref())
        .ok_or_else(|| anyhow::anyhow!("spike not found (set SPIKE_PATH or add it to PATH)"))?;

    let mut spike_cmd = Command::new(&spike_path);
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
    let spike_cmd_str = format!("{} {}", spike_path.display(), args_vec.join(" "));
    debug!("Spike command: {}", spike_cmd_str);

    // Stream spike output so we can optionally symbolize backtraces.
    spike_cmd.stdout(Stdio::piped());
    spike_cmd.stderr(Stdio::inherit());

    let mut child = spike_cmd
        .spawn()
        .context("Failed to execute spike (is it installed?)")?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture spike stdout"))?;

    let mut reader = BufReader::new(stdout);
    let mut out = std::io::stdout().lock();

    let addr2line = if args.symbolize_backtrace {
        sym::resolve_addr2line(args.addr2line.as_deref())
    } else {
        None
    };

    // Backtrace symbolization state: buffer contiguous frame lines and rewrite them.
    let mut pending_frames: Vec<(usize, String)> = Vec::new(); // (frame_no, addr_hex)
    let mut in_backtrace = false;

    let mut line = String::new();
    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .context("Failed to read spike stdout")?;
        if n == 0 {
            break;
        }

        if line.trim_end() == "stack backtrace:" {
            in_backtrace = true;
            pending_frames.clear();
            out.write_all(line.as_bytes()).ok();
            out.flush().ok();
            continue;
        }

        if in_backtrace {
            if let Some((frame_no, addr_hex)) = sym::parse_backtrace_unknown_frame(&line) {
                pending_frames.push((frame_no, addr_hex));
                continue;
            }

            if !pending_frames.is_empty() {
                flush_symbolized_frames(
                    &mut out,
                    &args.binary,
                    addr2line.as_deref(),
                    &pending_frames,
                );
                pending_frames.clear();
            }
            in_backtrace = false;
        }

        out.write_all(line.as_bytes()).ok();
        out.flush().ok();
    }

    if in_backtrace && !pending_frames.is_empty() {
        flush_symbolized_frames(
            &mut out,
            &args.binary,
            addr2line.as_deref(),
            &pending_frames,
        );
    }

    let status = child.wait().context("Failed to wait for spike process")?;

    if !status.success() {
        exit(status.code().unwrap_or(1));
    }

    Ok(())
}

fn resolve_spike(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = explicit {
        return Some(p.to_path_buf());
    }
    // Prefer PATH.
    if let Some(p) = sym::which("spike") {
        return Some(p);
    }
    // Common install locations (keep this list aligned with team conventions).
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let candidates: Vec<PathBuf> = [
        home.as_ref().map(|h| h.join(".local/bin/spike")),
        Some(PathBuf::from("/opt/riscv/bin/spike")),
    ]
    .into_iter()
    .flatten()
    .collect();

    candidates.into_iter().find(|p| p.is_file())
}

fn flush_symbolized_frames(
    out: &mut dyn Write,
    bin: &Path,
    addr2line: Option<&Path>,
    frames: &[(usize, String)],
) {
    for (frame_no, addr_hex) in frames {
        let addr = format!("0x{}", addr_hex);
        let sym_str = addr2line
            .and_then(|a2l| sym::symbolize_pc_with_fallback(bin, a2l, addr_hex))
            .unwrap_or_else(|| "<unknown>".to_string());

        let _ = writeln!(out, "{:>4}: {:>18} - {}", frame_no, addr, sym_str);
    }
    let _ = out.flush();
}
