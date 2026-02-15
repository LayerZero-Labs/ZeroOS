use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use clap::Args;

#[derive(Debug, Clone)]
struct Target {
    pc: u64,
    label: String,
}

#[derive(Debug, Clone)]
struct Stats {
    pc: u64,
    label: String,
    hits: usize,
    samples: Vec<u64>,
    counter: u64,
}

/// Measure syscall instruction-count "cost" (proxy) from Spike commit logs.
///
/// Important semantics:
/// - With exactly one `--target`, this tool is **syscall-ecall specific**: it assumes the target PC
///   is a 32-bit `ecall` and that returning from the trap resumes at `PC+4`. It reports per-syscall
///   instruction-count cost for each hit.
/// - With multiple `--target`s, it becomes a **PC hit-interval** tool: it reports instruction counts
///   between repeated hits of each target PC.
#[derive(Args, Debug)]
pub struct SpikeSyscallInstCountArgs {
    /// Path to Spike commit log (from `spike -l` / `--log-commits`)
    #[arg(long)]
    log: PathBuf,

    /// Target PCs to measure, as `0xaddr[:label]` (can be repeated)
    #[arg(long = "target", value_name = "PC[:label]", value_parser = parse_target)]
    targets: Vec<Target>,

    /// Dump one syscall instance's committed instruction trace to a file.
    ///
    /// Works only with exactly one `--target`. The trace starts at the target PC (typically an
    /// `ecall`) and stops once we see the first committed instruction at `PC+4` (normal return).
    #[arg(long, value_name = "PATH")]
    dump: Option<PathBuf>,
}

pub fn run(args: SpikeSyscallInstCountArgs) -> Result<(), Box<dyn std::error::Error>> {
    if args.targets.is_empty() {
        return Err("at least one --target is required (format: 0xADDR[:label])".into());
    }
    if args.dump.is_some() && args.targets.len() != 1 {
        return Err("--dump requires exactly one --target".into());
    }
    let single_target_mode = args.targets.len() == 1;
    let file = File::open(&args.log)?;
    let reader = BufReader::new(file);

    // Map pc -> stats (interval mode)
    let mut stats: HashMap<u64, Stats> = HashMap::new();
    for t in &args.targets {
        stats.insert(
            t.pc,
            Stats {
                pc: t.pc,
                label: t.label.clone(),
                hits: 0,
                samples: Vec::new(),
                counter: 0,
            },
        );
    }

    // Single-target mode state (only used when exactly one --target is provided)
    let mut ss_samples: Vec<u64> = Vec::new();
    let mut ss_hits_seen: usize = 0;
    let mut ss_capturing = false;
    let mut ss_count: u64 = 0;
    let (ss_target_pc, ss_return_pc) = if single_target_mode {
        let pc = args.targets[0].pc;
        (Some(pc), Some(pc.wrapping_add(4)))
    } else {
        (None, None)
    };

    for line in reader.lines() {
        let line = line?;
        let pc = match extract_pc(&line) {
            Some(pc) => pc,
            None => continue,
        };

        // Interval mode: count between repeated hits.
        if let Some(st) = stats.get_mut(&pc) {
            st.hits += 1;
            // Record instructions executed since the previous hit (excluding this hit instruction).
            st.samples.push(st.counter);
            st.counter = 0;
        }
        for st in stats.values_mut() {
            st.counter += 1;
        }

        // Single-target mode: count from target_pc (inclusive) until return_pc (exclusive).
        if let (Some(tpc), Some(rpc)) = (ss_target_pc, ss_return_pc) {
            if !ss_capturing && pc == tpc {
                ss_hits_seen += 1;
                ss_capturing = true;
                ss_count = 1; // include the ecall itself
            } else if ss_capturing {
                // Stop when we first reach the return PC (don't count that first user instruction).
                if pc == rpc {
                    ss_samples.push(ss_count);
                    ss_capturing = false;
                    ss_count = 0;
                } else {
                    ss_count += 1;
                }
            }
        }
    }

    println!("Parsed log: {}", args.log.display());
    if single_target_mode {
        print_stats(
            args.targets[0].pc,
            &args.targets[0].label,
            ss_hits_seen,
            &ss_samples,
        );
    } else {
        for t in &args.targets {
            let st = stats.get(&t.pc).expect("target missing in stats");
            print_stats(st.pc, &st.label, st.hits, &st.samples);
        }
    }

    if let Some(out_path) = &args.dump {
        let t = &args.targets[0];
        dump_one_instance(&args.log, t.pc, &t.label, out_path)?;
    }

    Ok(())
}

fn dump_one_instance(
    log_path: &PathBuf,
    target_pc: u64,
    label: &str,
    out_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let return_pc = target_pc.wrapping_add(4);
    let desired_hit = 1usize; // dump the first occurrence

    let file = File::open(log_path)?;
    let reader = BufReader::new(file);

    let mut hits_seen = 0usize;
    let mut capturing = false;
    // Capture ALL raw lines once we start (including trap/marker lines), but use extract_pc()
    // to detect the start/end instruction PCs.
    let mut captured: Vec<String> = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let pc = extract_pc(&line);

        if !capturing {
            if pc == Some(target_pc) {
                hits_seen += 1;
                if hits_seen == desired_hit {
                    capturing = true;
                    captured.push(line);
                }
            }
            continue;
        }

        // capturing: keep every line (even if it isn't an instruction commit line)
        captured.push(line);
        // Stop when we see the first committed instruction at return_pc.
        if pc == Some(return_pc) {
            break;
        }
    }

    if captured.is_empty() {
        return Err(format!(
            "dump: did not find any hit for {} @ 0x{:x}",
            label, target_pc
        )
        .into());
    }

    let mut out = File::create(out_path)?;
    writeln!(
        out,
        "# Dumped committed instructions: {} @ 0x{:x} return_pc=0x{:x}",
        label, target_pc, return_pc
    )?;
    for l in captured {
        writeln!(out, "{l}")?;
    }

    println!("Dumped instruction trace to {}", out_path.display());
    Ok(())
}

fn print_stats(pc: u64, label: &str, hits: usize, samples: &[u64]) {
    if samples.is_empty() {
        println!("- {} @ 0x{:x}: hits={} (no samples)", label, pc, hits);
        return;
    }

    let best = samples.iter().copied().min().unwrap();
    let max = samples.iter().copied().max().unwrap();
    let sum: u128 = samples.iter().copied().map(u128::from).sum();
    let avg = sum as f64 / samples.len() as f64;

    println!(
        "- {} @ 0x{:x}: samples={} hits={} best={} avg={:.2} max={}",
        label,
        pc,
        samples.len(),
        hits,
        best,
        avg,
        max
    );
}

fn parse_target(s: &str) -> Result<Target, String> {
    let (pc_str, label) = match s.split_once(':') {
        Some((pc, lbl)) => (pc, lbl.to_string()),
        None => (s, s.to_string()),
    };

    let pc = parse_hex_pc(pc_str)?;
    Ok(Target { pc, label })
}

fn parse_hex_pc(s: &str) -> Result<u64, String> {
    let trimmed = s.trim_start_matches("0x");
    u64::from_str_radix(trimmed, 16).map_err(|e| format!("invalid pc '{}': {}", s, e))
}

fn extract_pc(line: &str) -> Option<u64> {
    // Parse only Spike commit log instruction lines, which look like:
    //   "core   0: 0x0000000080000076 (0x00000073) ecall"
    // and NOT trap lines like:
    //   "core   0: exception trap_machine_ecall, epc 0x...."
    //
    // We enforce presence of "(0x" after the PC token to distinguish instructions.
    let core_idx = line.find("core")?;
    let after_core = &line[core_idx..];
    let colon_idx = after_core.find(':')?;
    let mut s = after_core[colon_idx + 1..].trim_start();
    if !s.starts_with("0x") {
        return None;
    }

    // Grab the PC token (up to whitespace).
    let pc_token_end = s.find(char::is_whitespace).unwrap_or(s.len());
    let pc_token = &s[..pc_token_end];
    s = &s[pc_token_end..];

    // Must have "(0x" after the PC token to qualify as an instruction line.
    if !s.contains("(0x") {
        return None;
    }

    let hex = pc_token.trim_start_matches("0x");
    if hex.is_empty() {
        return None;
    }
    u64::from_str_radix(hex, 16).ok()
}
