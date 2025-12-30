use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolve an addr2line executable path.
///
/// - Uses `explicit` if provided
/// - Else tries `riscv64-unknown-elf-addr2line`, then `llvm-addr2line` in PATH
pub fn resolve_addr2line(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = explicit {
        return Some(p.to_path_buf());
    }
    find_in_path("riscv64-unknown-elf-addr2line").or_else(|| find_in_path("llvm-addr2line"))
}

fn find_in_path(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let cand = dir.join(bin);
        if cand.is_file() {
            return Some(cand);
        }
    }
    None
}

/// Find an executable in PATH (simple `which`).
pub fn which(bin: &str) -> Option<PathBuf> {
    find_in_path(bin)
}

/// Parse a Rust `stack backtrace:` frame line.
///
/// Example line:
/// `  19:         0x80019fa2 - <unknown>`
///
/// Returns `(frame_no, hex_addr_without_0x)` for frames that are `<unknown>`.
pub fn parse_backtrace_unknown_frame(line: &str) -> Option<(usize, String)> {
    if !line.contains(" - <unknown>") {
        return None;
    }
    let (left, _) = line.split_once(':')?;
    let frame_no: usize = left.trim().parse().ok()?;
    let hex_pos = line.find("0x")?;
    let after = &line[hex_pos + 2..];
    let hex: String = after
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .collect();
    if hex.is_empty() {
        return None;
    }
    Some((frame_no, hex))
}

pub fn parse_hex(s: &str) -> usize {
    usize::from_str_radix(s, 16).unwrap_or(0)
}

/// Symbolize a single PC using addr2line.
///
/// Returns a single-line string like:
/// `my_func at /path/file.rs:123`
pub fn symbolize_addr(bin: &Path, addr2line: &Path, addr: &str) -> Option<String> {
    let output = Command::new(addr2line)
        .args(["-e"])
        .arg(bin)
        .args(["-f", "-C", "-p"])
        .arg(addr)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() {
        return None;
    }

    // addr2line output can include an address prefix; keep the RHS if present.
    let s = if let Some((_, rhs)) = s.split_once(": ") {
        rhs.to_string()
    } else {
        s
    };

    // We prefer `fn at file:line`, but for early boot / assembly stubs we may only be able to
    // recover a symbol name with unknown location (e.g. `foo at ??:?`). Keep the symbol name in
    // that case instead of reporting `<unknown>`.
    let (func, loc) = s.split_once(" at ").unwrap_or((s.as_str(), ""));
    let func = func.trim();
    if func.is_empty() || func == "??" {
        return None;
    }
    if loc.contains("??:0") || loc.contains("??:?") || loc.is_empty() {
        return Some(func.to_string());
    }
    Some(format!("{func} at {loc}"))
}

/// Best-effort symbolize an unknown backtrace PC, with a RISC-V `pc-4` fallback.
pub fn symbolize_pc_with_fallback(
    bin: &Path,
    addr2line: &Path,
    addr_hex_no_0x: &str,
) -> Option<String> {
    let addr = format!("0x{}", addr_hex_no_0x);
    symbolize_addr(bin, addr2line, &addr).or_else(|| {
        let pc = parse_hex(addr_hex_no_0x);
        let addr_m4 = format!("0x{:x}", pc.saturating_sub(4));
        symbolize_addr(bin, addr2line, &addr_m4)
    })
}
