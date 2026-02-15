use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use object::{Object, ObjectSection, ObjectSymbol};
use rustc_demangle::demangle;

use crate::{
    map,
    types::{FileReport, MapSymbol, SectionInfo, SymbolInfo},
};

pub fn normalize_map_args(elfs: &[PathBuf], maps: &[PathBuf]) -> Result<Vec<Option<PathBuf>>> {
    if maps.is_empty() {
        return Ok(vec![None; elfs.len()]);
    }
    if maps.len() == 1 {
        return Ok(vec![Some(maps[0].clone()); elfs.len()]);
    }
    if maps.len() == elfs.len() {
        return Ok(maps.iter().cloned().map(Some).collect());
    }
    anyhow::bail!(
        "Invalid --map usage: got {} map(s) for {} ELF(s). Provide 0 maps, 1 map, or one map per ELF.",
        maps.len(),
        elfs.len()
    );
}

pub fn analyze_paths(
    paths: &[PathBuf],
    maps: &[Option<PathBuf>],
    top: usize,
) -> Result<Vec<FileReport>> {
    if paths.is_empty() {
        anyhow::bail!("No input ELFs provided");
    }
    if maps.len() != paths.len() {
        anyhow::bail!("Internal error: map list must match ELF list");
    }

    let mut reports = Vec::with_capacity(paths.len());
    for (i, p) in paths.iter().enumerate() {
        let map = maps.get(i).cloned().flatten();
        reports.push(analyze_path(p, map.as_deref(), top)?);
    }
    Ok(reports)
}

pub fn analyze_path(path: &Path, map_path: Option<&Path>, top: usize) -> Result<FileReport> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let file =
        object::File::parse(&*bytes).with_context(|| format!("parsing ELF {}", path.display()))?;

    let file_kind = format!("{:?}", file.kind());
    let arch = format!("{:?}", file.architecture());

    let mut sections: Vec<SectionInfo> = file
        .sections()
        .filter_map(|s| {
            let name = s.name().ok()?.to_string();
            Some(SectionInfo {
                name,
                size: s.size(),
                address: s.address(),
            })
        })
        .collect();
    sections.sort_by_key(|s| std::cmp::Reverse(s.size));

    let has_symtab_section = file.sections().any(|s| matches!(s.name(), Ok(".symtab")));
    let has_symbols = file.symbols().next().is_some();
    let mut is_stripped_guess = !(has_symtab_section && has_symbols);

    let mut notes = Vec::new();

    // Optional map-based symbol attribution.
    let map_symbols: Option<Vec<MapSymbol>> = if let Some(mp) = map_path {
        match map::parse_map_symbols(mp) {
            Ok(syms) => Some(syms),
            Err(e) => {
                notes.push(format!("Failed to parse map file {}: {e:#}", mp.display()));
                None
            }
        }
    } else {
        None
    };

    // Prefer ELF symtab symbols when present; otherwise, use map symbols if provided.
    let mut map_used = false;
    let (top_text_symbols, top_rodata_symbols) = if !is_stripped_guess {
        (
            top_symbols_in_section(&file, ".text", top),
            top_symbols_in_section(&file, ".rodata", top),
        )
    } else if let Some(map_syms) = &map_symbols {
        let text = top_symbols_from_map(&sections, map_syms, ".text", top);
        let rodata = top_symbols_from_map(&sections, map_syms, ".rodata", top);
        if text.is_empty() && rodata.is_empty() {
            notes.push(
                "ELF appears stripped and map did not yield any symbols in .text/.rodata ranges. Ensure the map corresponds to this exact build and includes symbol addresses/sizes."
                    .to_string(),
            );
        } else {
            map_used = true;
            is_stripped_guess = false; // for reporting purposes: we *do* have symbol names via map
        }
        (text, rodata)
    } else {
        notes.push(
            "No usable .symtab symbols detected (binary likely stripped). Provide --map <link.map> or build an unstripped analysis ELF to get symbol-level attribution."
                .to_string(),
        );
        (Vec::new(), Vec::new())
    };

    Ok(FileReport {
        path: path.display().to_string(),
        file_kind,
        arch,
        is_stripped_guess,
        map_used,
        sections,
        top_text_symbols,
        top_rodata_symbols,
        notes,
    })
}

fn top_symbols_in_section(
    file: &object::File<'_>,
    section_name: &str,
    top: usize,
) -> Vec<SymbolInfo> {
    let mut out = Vec::new();

    for sym in file.symbols() {
        let size = sym.size();
        if size == 0 {
            continue;
        }

        let Some(sec_idx) = sym.section_index() else {
            continue;
        };
        let Ok(sec) = file.section_by_index(sec_idx) else {
            continue;
        };
        let Ok(name) = sec.name() else {
            continue;
        };
        if name != section_name {
            continue;
        }

        let sym_name = sym.name().unwrap_or("<unknown>");
        let demangled = demangle(sym_name).to_string();

        out.push(SymbolInfo {
            name: sym_name.to_string(),
            demangled,
            size,
            address: sym.address(),
        });
    }

    out.sort_by_key(|s| std::cmp::Reverse(s.size));
    out.truncate(top);
    out
}

fn section_range(sections: &[SectionInfo], name: &str) -> Option<(u64, u64)> {
    sections
        .iter()
        .find(|s| s.name == name)
        .map(|s| (s.address, s.address.saturating_add(s.size)))
}

fn normalize_map_symbol_name(name: &str) -> &str {
    // Some maps emit entries like `.text._ZN...` or `.rodata._ZN...`.
    name.strip_prefix(".text.")
        .or_else(|| name.strip_prefix(".rodata."))
        .unwrap_or(name)
}

fn top_symbols_from_map(
    sections: &[SectionInfo],
    map_syms: &[MapSymbol],
    section_name: &str,
    top: usize,
) -> Vec<SymbolInfo> {
    let Some((start, end)) = section_range(sections, section_name) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for ms in map_syms {
        if ms.address < start || ms.address >= end {
            continue;
        }
        // Heuristic: only include plausible symbol names.
        if ms.name.starts_with('*') || ms.name.is_empty() {
            continue;
        }
        if map::is_plain_section_label_public(&ms.name) {
            continue;
        }
        let raw = normalize_map_symbol_name(&ms.name);
        let demangled = demangle(raw).to_string();
        out.push(SymbolInfo {
            name: raw.to_string(),
            demangled,
            size: ms.size,
            address: ms.address,
        });
    }

    out.sort_by_key(|s| std::cmp::Reverse(s.size));
    out.truncate(top);
    out
}
