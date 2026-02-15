use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    Md,
    Json,
}

#[derive(Debug, Parser)]
#[command(
    about = "Inspect ELF files: section sizes and (when available) largest symbols",
    long_about = None
)]
struct Args {
    /// One or more ELF paths to inspect.
    #[arg(value_name = "ELF")]
    paths: Vec<PathBuf>,

    /// Optional linker map file(s) for symbol attribution when the ELF is stripped.
    ///
    /// If multiple ELFs are provided, you may pass either:
    /// - one map (applied to all ELFs), or
    /// - the same number of maps as ELFs (paired by position).
    #[arg(long, value_name = "MAP")]
    map: Vec<PathBuf>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Md)]
    format: OutputFormat,

    /// Write output to a file instead of stdout.
    #[arg(long, value_name = "PATH")]
    out: Option<PathBuf>,

    /// Number of largest symbols to show per section.
    #[arg(long, default_value_t = 50)]
    top: usize,

    /// Group symbols by crate/module path depth.
    ///
    /// Examples:
    ///   -d 1: Group by crate (std, core, alloc)
    ///   -d 2: Group by module (std::fmt, core::iter)
    ///   Without -d: Show individual symbols
    #[arg(short = 'd', long)]
    depth: Option<usize>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.paths.is_empty() {
        anyhow::bail!("No input ELFs provided");
    }

    let map_paths = elf_report::normalize_map_args(&args.paths, &args.map)?;
    let reports = elf_report::analyze_paths(&args.paths, &map_paths, args.top)?;

    let out = match args.format {
        OutputFormat::Json => serde_json::to_string_pretty(&reports)?,
        OutputFormat::Md => {
            if let Some(depth) = args.depth {
                elf_report::render_markdown_grouped(&reports, depth)
            } else {
                elf_report::render_markdown(&reports)
            }
        }
    };

    match args.out {
        Some(path) => {
            fs::write(&path, out).with_context(|| format!("writing {}", path.display()))?;
        }
        None => {
            let mut stdout = io::BufWriter::new(io::stdout().lock());
            stdout.write_all(out.as_bytes())?;
        }
    }

    Ok(())
}
