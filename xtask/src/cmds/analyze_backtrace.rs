//! Comprehensive binary size analysis for all backtrace configurations.
//!
//! Builds and analyzes ALL combinations:
//! - Runtime: no-std, std  
//! - Backtrace: off, frame-pointers, dwarf (std only)
//! - Debug: 0, 1, 2
//! - Strip: false, true
//!
//! Total: 30 builds (5 mode combos × 3 debug × 2 strip)

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, clap::Args)]
pub struct AnalyzeBacktraceArgs {
    /// LTO mode: false, true, thin, fat
    #[arg(long, default_value = "fat")]
    lto: String,

    /// Number of top symbols to show in detailed analysis
    #[arg(long, default_value = "50")]
    top: usize,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct BuildConfig {
    runtime: String,
    backtrace: String,
    debug: u8,
    strip: bool,
}

pub fn run(args: AnalyzeBacktraceArgs) -> Result<()> {
    println!("=====================================================================");
    println!("Comprehensive Backtrace Size Analysis");
    println!("=====================================================================");
    println!();
    println!("Build matrix:");
    println!("  - Runtimes: no-std, std");
    println!("  - Backtrace: off, frame-pointers, dwarf (std only)");
    println!("  - Debug levels: 0, 1, 2");
    println!("  - Strip: false, true");
    println!("  - Total builds: 30");
    println!();
    println!("Settings:");
    println!("  - LTO: {}", args.lto);
    println!("  - Opt-level: z (size)");
    println!();

    let modes = vec![
        ("no-std", "off"),
        ("no-std", "frame-pointers"),
        ("std", "off"),
        ("std", "frame-pointers"),
        ("std", "dwarf"),
    ];

    let debug_levels = [0, 1, 2];
    let strip_values = [false, true];

    let total = modes.len() * debug_levels.len() * strip_values.len();
    let mut results: HashMap<BuildConfig, u64> = HashMap::new();
    let mut count = 0;

    for (runtime, backtrace) in &modes {
        for &debug in &debug_levels {
            for &strip in &strip_values {
                count += 1;
                let strip_str = if strip { "strip" } else { "no-strip" };
                print!(
                    "[{:2}/{}] {:8} + {:15} debug={} {}... ",
                    count, total, runtime, backtrace, debug, strip_str
                );
                std::io::Write::flush(&mut std::io::stdout()).ok();

                match build_binary(runtime, backtrace, debug, strip, &args.lto) {
                    Ok(size) => {
                        println!("{:>10}", bytefmt::format(size));
                        results.insert(
                            BuildConfig {
                                runtime: runtime.to_string(),
                                backtrace: backtrace.to_string(),
                                debug,
                                strip,
                            },
                            size,
                        );
                    }
                    Err(e) => {
                        println!("FAILED: {}", e);
                    }
                }
            }
        }
        println!();
    }

    print_results_table(&results, &modes)?;
    print_insights(&results, &modes)?;
    print_detailed_elf_analysis(&args.lto, args.top)?;

    Ok(())
}

fn build_binary(runtime: &str, backtrace: &str, debug: u8, strip: bool, lto: &str) -> Result<u64> {
    std::env::set_var("CARGO_PROFILE_RELEASE_DEBUG", debug.to_string());
    std::env::set_var("CARGO_PROFILE_RELEASE_STRIP", strip.to_string());
    std::env::set_var("CARGO_PROFILE_RELEASE_LTO", lto);
    std::env::set_var("CARGO_PROFILE_RELEASE_OPT_LEVEL", "z");

    let target = if runtime == "no-std" {
        "riscv64imac-unknown-none-elf"
    } else {
        "riscv64imac-zero-linux-musl"
    };

    let mut cmd = Command::new("cargo");
    cmd.arg("spike")
        .arg("build")
        .arg("-p")
        .arg("backtrace")
        .arg("--target")
        .arg(target);

    if runtime == "std" {
        cmd.arg("--mode").arg("std");
    }

    cmd.arg("--backtrace")
        .arg(backtrace)
        .arg("--")
        .arg("--quiet");

    if runtime == "std" {
        cmd.arg("--features").arg("std,with-spike");
    } else {
        cmd.arg("--features").arg("with-spike");
    }

    cmd.arg("--profile").arg("release");

    // Suppress warnings
    cmd.stderr(std::process::Stdio::null());

    let output = cmd.output()?;
    if !output.status.success() {
        anyhow::bail!("cargo spike build failed");
    }

    let bin_path = PathBuf::from(format!("target/{}/release/backtrace", target));
    Ok(std::fs::metadata(&bin_path)?.len())
}

fn print_results_table(results: &HashMap<BuildConfig, u64>, modes: &[(&str, &str)]) -> Result<()> {
    println!();
    println!("=====================================================================");
    println!("RESULTS: strip=false (with symbols/debug)");
    println!("=====================================================================");
    println!();
    println!(
        "{:<25} {:>12} {:>12} {:>12}",
        "Mode", "debug=0", "debug=1", "debug=2"
    );
    println!("{:-<25} {:-<12} {:-<12} {:-<12}", "", "", "", "");

    for (runtime, backtrace) in modes {
        let mode_name = format!("{} + {}", runtime, backtrace);
        let sizes: Vec<String> = [0, 1, 2]
            .iter()
            .map(|&d| {
                let config = BuildConfig {
                    runtime: runtime.to_string(),
                    backtrace: backtrace.to_string(),
                    debug: d,
                    strip: false,
                };
                results
                    .get(&config)
                    .map(|s| format!("{:>10}", bytefmt::format(*s)))
                    .unwrap_or_else(|| "N/A".to_string())
            })
            .collect();

        println!(
            "{:<25} {:>12} {:>12} {:>12}",
            mode_name, sizes[0], sizes[1], sizes[2]
        );
    }

    println!();
    println!("=====================================================================");
    println!("RESULTS: strip=true (production minimal)");
    println!("=====================================================================");
    println!();
    println!(
        "{:<25} {:>12} {:>12} {:>12}",
        "Mode", "debug=0", "debug=1", "debug=2"
    );
    println!("{:-<25} {:-<12} {:-<12} {:-<12}", "", "", "", "");

    for (runtime, backtrace) in modes {
        let mode_name = format!("{} + {}", runtime, backtrace);
        let sizes: Vec<String> = [0, 1, 2]
            .iter()
            .map(|&d| {
                let config = BuildConfig {
                    runtime: runtime.to_string(),
                    backtrace: backtrace.to_string(),
                    debug: d,
                    strip: true,
                };
                results
                    .get(&config)
                    .map(|s| format!("{:>10}", bytefmt::format(*s)))
                    .unwrap_or_else(|| "N/A".to_string())
            })
            .collect();

        println!(
            "{:<25} {:>12} {:>12} {:>12}",
            mode_name, sizes[0], sizes[1], sizes[2]
        );
    }

    Ok(())
}

fn print_detailed_elf_analysis(lto: &str, top: usize) -> Result<()> {
    println!();
    println!("=====================================================================");
    println!("DETAILED ELF ANALYSIS (std + off + debug=0 + strip=false)");
    println!("=====================================================================");
    println!();
    println!("Building unstripped binary for symbol analysis...");
    println!("Note: Using strip=false to preserve symbols for detailed breakdown.");
    println!();

    // Build with debug=0 but strip=false to preserve symbols for analysis
    std::env::set_var("CARGO_PROFILE_RELEASE_DEBUG", "0");
    std::env::set_var("CARGO_PROFILE_RELEASE_STRIP", "false");
    std::env::set_var("CARGO_PROFILE_RELEASE_LTO", lto);
    std::env::set_var("CARGO_PROFILE_RELEASE_OPT_LEVEL", "z");

    let target = "riscv64imac-zero-linux-musl";

    let mut cmd = Command::new("cargo");
    cmd.arg("spike")
        .arg("build")
        .arg("-p")
        .arg("backtrace")
        .arg("--target")
        .arg(target)
        .arg("--mode")
        .arg("std")
        .arg("--backtrace")
        .arg("off")
        .arg("--")
        .arg("--quiet")
        .arg("--features")
        .arg("std,with-spike")
        .arg("--profile")
        .arg("release");

    // Suppress warnings
    cmd.stderr(std::process::Stdio::null());

    let output = cmd.output()?;
    if !output.status.success() {
        println!("⚠ Failed to build binary for detailed analysis");
        return Ok(());
    }

    let bin_path = PathBuf::from(format!("target/{}/release/backtrace", target));

    // Analyze with elf-report library
    println!(
        "Running elf-report analysis with module-level grouping (depth=2, top={})...",
        top
    );
    println!();

    let reports = match elf_report::analyze_paths(std::slice::from_ref(&bin_path), &[None], top) {
        Ok(r) => r,
        Err(e) => {
            println!("⚠ elf-report analysis failed: {}", e);
            return Ok(());
        }
    };

    let output = elf_report::render_markdown_grouped(&reports, 2);
    println!("{}", output);

    println!();
    println!("NOTE: Production stripped size (strip=true) is shown in the main results table.");
    println!();

    // Show stripped binary sections for comparison
    println!("=====================================================================");
    println!("STRIPPED BINARY SECTIONS (std + off + debug=0 + strip=true)");
    println!("=====================================================================");
    println!();
    println!("Building stripped binary to show actual production sections...");

    // Build stripped version
    std::env::set_var("CARGO_PROFILE_RELEASE_DEBUG", "0");
    std::env::set_var("CARGO_PROFILE_RELEASE_STRIP", "true");
    std::env::set_var("CARGO_PROFILE_RELEASE_LTO", lto);
    std::env::set_var("CARGO_PROFILE_RELEASE_OPT_LEVEL", "z");

    let mut cmd = Command::new("cargo");
    cmd.arg("spike")
        .arg("build")
        .arg("-p")
        .arg("backtrace")
        .arg("--target")
        .arg(target)
        .arg("--mode")
        .arg("std")
        .arg("--backtrace")
        .arg("off")
        .arg("--")
        .arg("--quiet")
        .arg("--features")
        .arg("std,with-spike")
        .arg("--profile")
        .arg("release");

    cmd.stderr(std::process::Stdio::null());

    let output = cmd.output()?;
    if !output.status.success() {
        println!("⚠ Failed to build stripped binary");
        return Ok(());
    }

    let stripped_bin_path = PathBuf::from(format!("target/{}/release/backtrace", target));

    // Analyze stripped binary with elf-report library
    println!();
    let stripped_reports = match elf_report::analyze_paths(&[stripped_bin_path], &[None], 0) {
        Ok(r) => r,
        Err(e) => {
            println!("⚠ elf-report analysis failed: {}", e);
            return Ok(());
        }
    };

    // Show just the sections table
    if let Some(report) = stripped_reports.first() {
        println!("### Largest sections\n");

        let sections_to_show: Vec<_> = report.sections.iter().take(30).collect();
        let max_section_name_len = sections_to_show
            .iter()
            .map(|s| s.name.len() + 2)
            .max()
            .unwrap_or(7)
            .max(7);
        let max_size_len = sections_to_show
            .iter()
            .map(|s| format!("{}", s.size).len())
            .max()
            .unwrap_or(12)
            .max(12);
        let addr_width = 10;

        println!(
            "| {:<width_section$} | {:>width_size$} | {:>width_addr$} |",
            "section",
            "size (bytes)",
            "address",
            width_section = max_section_name_len,
            width_size = max_size_len,
            width_addr = addr_width
        );
        println!(
            "|{:-<width_section$}|{:-<width_size$}:|{:-<width_addr$}:|",
            "",
            "",
            "",
            width_section = max_section_name_len + 2,
            width_size = max_size_len + 2,
            width_addr = addr_width + 2
        );

        for sec in &sections_to_show {
            let section_cell = format!("`{}`", sec.name);
            println!(
                "| {:<width_section$} | {:>width_size$} | {:>#width_addr$x} |",
                section_cell,
                sec.size,
                sec.address,
                width_section = max_section_name_len,
                width_size = max_size_len,
                width_addr = addr_width
            );
        }
        println!();
    }

    println!();
    println!("EXPLANATION:");
    println!("  - The grouped symbols shown above are only the TOP symbols (default: 50)");
    println!("  - The .text section contains ALL code, including many small functions");
    println!(
        "  - Size breakdown: .text (code) + .rodata (constants) + .data + .eh_frame + etc = total"
    );

    Ok(())
}

fn print_insights(results: &HashMap<BuildConfig, u64>, _modes: &[(&str, &str)]) -> Result<()> {
    println!();
    println!("=====================================================================");
    println!("KEY INSIGHTS");
    println!("=====================================================================");
    println!();

    // Absolute minimum
    let min_config = BuildConfig {
        runtime: "no-std".to_string(),
        backtrace: "off".to_string(),
        debug: 0,
        strip: true,
    };
    if let Some(&min_size) = results.get(&min_config) {
        println!(
            "✓ Absolute minimum (no-std + off + debug=0 + strip): {}",
            bytefmt::format(min_size)
        );
    }

    // Strip impact
    let nostd_off_d0_nostrip = results.get(&BuildConfig {
        runtime: "no-std".to_string(),
        backtrace: "off".to_string(),
        debug: 0,
        strip: false,
    });
    let nostd_off_d0_strip = results.get(&BuildConfig {
        runtime: "no-std".to_string(),
        backtrace: "off".to_string(),
        debug: 0,
        strip: true,
    });
    if let (Some(&nostrip), Some(&strip)) = (nostd_off_d0_nostrip, nostd_off_d0_strip) {
        let saved = nostrip.saturating_sub(strip);
        println!(
            "✓ Strip impact (debug=0): {} → {} (saves {})",
            bytefmt::format(nostrip),
            bytefmt::format(strip),
            bytefmt::format(saved)
        );
        println!("  (Debug sections from dependencies removed)");
    }

    // Debug level impact
    let nostd_off_strip_d0 = results.get(&BuildConfig {
        runtime: "no-std".to_string(),
        backtrace: "off".to_string(),
        debug: 0,
        strip: true,
    });
    let nostd_off_strip_d1 = results.get(&BuildConfig {
        runtime: "no-std".to_string(),
        backtrace: "off".to_string(),
        debug: 1,
        strip: true,
    });
    if let (Some(&d0), Some(&d1)) = (nostd_off_strip_d0, nostd_off_strip_d1) {
        let cost = d1.saturating_sub(d0);
        println!("✓ Debug=1 cost (line tables): +{}", bytefmt::format(cost));
    }

    // Backtrace overhead (no-std, minimal)
    let nostd_off = results.get(&BuildConfig {
        runtime: "no-std".to_string(),
        backtrace: "off".to_string(),
        debug: 0,
        strip: true,
    });
    let nostd_fp = results.get(&BuildConfig {
        runtime: "no-std".to_string(),
        backtrace: "frame-pointers".to_string(),
        debug: 0,
        strip: true,
    });
    if let (Some(&off), Some(&fp)) = (nostd_off, nostd_fp) {
        let overhead = fp.saturating_sub(off);
        println!(
            "✓ Frame-pointer overhead (no-std, production): +{}",
            bytefmt::format(overhead)
        );
    }

    // DWARF overhead (std, minimal)
    let std_off = results.get(&BuildConfig {
        runtime: "std".to_string(),
        backtrace: "off".to_string(),
        debug: 0,
        strip: true,
    });
    let std_dwarf = results.get(&BuildConfig {
        runtime: "std".to_string(),
        backtrace: "dwarf".to_string(),
        debug: 0,
        strip: true,
    });
    if let (Some(&off), Some(&dwarf)) = (std_off, std_dwarf) {
        let overhead = (dwarf as i64 - off as i64).unsigned_abs();
        println!(
            "✓ DWARF overhead (std, production): +{}",
            bytefmt::format(overhead)
        );
    }

    println!();
    println!("Analysis complete!");
    Ok(())
}

mod bytefmt {
    pub fn format(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB"];
        let mut size = bytes as f64;
        let mut unit_idx = 0;

        while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
            size /= 1024.0;
            unit_idx += 1;
        }

        if unit_idx == 0 {
            format!("{} {}", bytes, UNITS[0])
        } else {
            format!("{:.1} {}", size, UNITS[unit_idx])
        }
    }
}
