use std::collections::HashMap;

use crate::symbol::extract_path_at_depth;
use crate::types::{FileReport, SymbolGroup, SymbolInfo};

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MiB", bytes as f64 / (1024.0 * 1024.0))
    }
}

pub fn render_markdown(reports: &[FileReport]) -> String {
    let mut s = String::new();
    s.push_str("# ELF size report\n\n");
    s.push_str(
        "This report summarizes ELF section sizes and (when symbols exist) the largest .text/.rodata symbols.\n\n",
    );

    for r in reports {
        s.push_str(&format!("## {}\n\n", r.path));
        s.push_str(&format!("- kind: `{}`\n", r.file_kind));
        s.push_str(&format!("- arch: `{}`\n", r.arch));
        s.push_str(&format!(
            "- stripped (best-effort): `{}`\n\n",
            r.is_stripped_guess
        ));

        if r.map_used {
            s.push_str("- map used: `true` (symbol names attributed via linker map)\n\n");
        }

        if !r.notes.is_empty() {
            s.push_str("### Notes\n\n");
            for n in &r.notes {
                s.push_str(&format!("- {}\n", n));
            }
            s.push('\n');
        }

        s.push_str("### Largest sections\n\n");

        // Calculate column widths for sections
        let sections_to_show: Vec<_> = r.sections.iter().take(30).collect();
        let max_section_name_len = sections_to_show
            .iter()
            .map(|s| s.name.len() + 2) // +2 for backticks
            .max()
            .unwrap_or(7)
            .max(7); // min width for "section"
        let max_size_len = sections_to_show
            .iter()
            .map(|s| format!("{}", s.size).len())
            .max()
            .unwrap_or(12)
            .max(12); // min width for "size (bytes)"
        let addr_width = 10; // "address" width

        s.push_str(&format!(
            "| {:<width_section$} | {:>width_size$} | {:>width_addr$} |\n",
            "section",
            "size (bytes)",
            "address",
            width_section = max_section_name_len,
            width_size = max_size_len,
            width_addr = addr_width
        ));
        s.push_str(&format!(
            "|{:-<width_section$}|{:-<width_size$}:|{:-<width_addr$}:|\n",
            "",
            "",
            "",
            width_section = max_section_name_len + 2,
            width_size = max_size_len + 2,
            width_addr = addr_width + 2
        ));

        for sec in &sections_to_show {
            let section_cell = format!("`{}`", sec.name);
            s.push_str(&format!(
                "| {:<width_section$} | {:>width_size$} | {:>#width_addr$x} |\n",
                section_cell,
                sec.size,
                sec.address,
                width_section = max_section_name_len,
                width_size = max_size_len,
                width_addr = addr_width
            ));
        }
        s.push('\n');

        if !r.top_text_symbols.is_empty() {
            s.push_str("### Top .text symbols\n\n");
            s.push_str("| size (bytes) | address | symbol |\n");
            s.push_str("|---:|---:|---|\n");
            for sym in &r.top_text_symbols {
                s.push_str(&format!(
                    "| {} | 0x{:x} | `{}` |\n",
                    sym.size, sym.address, sym.demangled
                ));
            }
            s.push('\n');
        }

        if !r.top_rodata_symbols.is_empty() {
            s.push_str("### Top .rodata symbols\n\n");
            s.push_str("| size (bytes) | address | symbol |\n");
            s.push_str("|---:|---:|---|\n");
            for sym in &r.top_rodata_symbols {
                s.push_str(&format!(
                    "| {} | 0x{:x} | `{}` |\n",
                    sym.size, sym.address, sym.demangled
                ));
            }
            s.push('\n');
        }
    }

    s
}

fn group_symbols_by_depth(symbols: &[SymbolInfo], depth: usize) -> Vec<SymbolGroup> {
    let mut groups: HashMap<String, Vec<SymbolInfo>> = HashMap::new();

    for sym in symbols {
        let path =
            extract_path_at_depth(&sym.demangled, depth).unwrap_or_else(|| "[native]".to_string());

        groups.entry(path).or_default().push(sym.clone());
    }

    let mut result: Vec<SymbolGroup> = groups
        .into_iter()
        .map(|(path, symbols)| {
            let total_size = symbols.iter().map(|s| s.size).sum();
            SymbolGroup {
                path,
                total_size,
                symbol_count: symbols.len(),
                symbols,
            }
        })
        .collect();

    result.sort_by_key(|g| std::cmp::Reverse(g.total_size));
    result
}

pub fn render_markdown_grouped(reports: &[FileReport], depth: usize) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "# ELF size report (grouped at depth {})\n\n",
        depth
    ));
    s.push_str("Symbols grouped by crate/module path.\n\n");

    for r in reports {
        s.push_str(&format!("## {}\n\n", r.path));
        s.push_str(&format!("- kind: `{}`\n", r.file_kind));
        s.push_str(&format!("- arch: `{}`\n", r.arch));
        s.push_str(&format!(
            "- stripped (best-effort): `{}`\n\n",
            r.is_stripped_guess
        ));

        if r.map_used {
            s.push_str("- map used: `true` (symbol names attributed via linker map)\n\n");
        }

        if !r.notes.is_empty() {
            s.push_str("### Notes\n\n");
            for n in &r.notes {
                s.push_str(&format!("- {}\n", n));
            }
            s.push('\n');
        }

        s.push_str("### Largest sections\n\n");

        // Calculate column widths for sections
        let sections_to_show: Vec<_> = r.sections.iter().take(30).collect();
        let max_section_name_len = sections_to_show
            .iter()
            .map(|s| s.name.len() + 2) // +2 for backticks
            .max()
            .unwrap_or(7)
            .max(7); // min width for "section"
        let max_size_len = sections_to_show
            .iter()
            .map(|s| format!("{}", s.size).len())
            .max()
            .unwrap_or(12)
            .max(12); // min width for "size (bytes)"
        let addr_width = 10; // "address" width

        s.push_str(&format!(
            "| {:<width_section$} | {:>width_size$} | {:>width_addr$} |\n",
            "section",
            "size (bytes)",
            "address",
            width_section = max_section_name_len,
            width_size = max_size_len,
            width_addr = addr_width
        ));
        s.push_str(&format!(
            "|{:-<width_section$}|{:-<width_size$}:|{:-<width_addr$}:|\n",
            "",
            "",
            "",
            width_section = max_section_name_len + 2,
            width_size = max_size_len + 2,
            width_addr = addr_width + 2
        ));

        for sec in &sections_to_show {
            let section_cell = format!("`{}`", sec.name);
            s.push_str(&format!(
                "| {:<width_section$} | {:>width_size$} | {:>#width_addr$x} |\n",
                section_cell,
                sec.size,
                sec.address,
                width_section = max_section_name_len,
                width_size = max_size_len,
                width_addr = addr_width
            ));
        }
        s.push('\n');

        if !r.top_text_symbols.is_empty() {
            s.push_str(&format!("### Top .text groups (depth {})\n\n", depth));
            let groups = group_symbols_by_depth(&r.top_text_symbols, depth);

            // Calculate column widths
            let max_path_len = groups
                .iter()
                .map(|g| g.path.len() + 2) // +2 for backticks
                .max()
                .unwrap_or(4)
                .max(4)
                .max(15); // min width for "**TOTAL SHOWN**"
            let size_width = 12; // "total size" width
            let symbols_width = 9; // "symbols" width

            s.push_str(&format!(
                "| {:<width_path$} | {:>width_size$} | {:>width_symbols$} |\n",
                "path",
                "total size",
                "symbols",
                width_path = max_path_len,
                width_size = size_width,
                width_symbols = symbols_width
            ));
            s.push_str(&format!(
                "|{:-<width_path$}|{:-<width_size$}:|{:-<width_symbols$}:|\n",
                "",
                "",
                "",
                width_path = max_path_len + 2,
                width_size = size_width + 2,
                width_symbols = symbols_width + 2
            ));

            for group in &groups {
                let path_cell = format!("`{}`", group.path);
                let size_cell = format_size(group.total_size);
                s.push_str(&format!(
                    "| {:<width_path$} | {:>width_size$} | {:>width_symbols$} |\n",
                    path_cell,
                    size_cell,
                    group.symbol_count,
                    width_path = max_path_len,
                    width_size = size_width,
                    width_symbols = symbols_width
                ));
            }

            // Add summary rows
            let total_shown: u64 = groups.iter().map(|g| g.total_size).sum();
            let total_symbols: usize = groups.iter().map(|g| g.symbol_count).sum();
            let text_section_size = r
                .sections
                .iter()
                .find(|s| s.name == ".text")
                .map(|s| s.size)
                .unwrap_or(0);
            let coverage = if text_section_size > 0 {
                (total_shown as f64 / text_section_size as f64) * 100.0
            } else {
                0.0
            };

            s.push_str(&format!(
                "|{:-<width_path$}|{:-<width_size$}:|{:-<width_symbols$}:|\n",
                "",
                "",
                "",
                width_path = max_path_len + 2,
                width_size = size_width + 2,
                width_symbols = symbols_width + 2
            ));
            s.push_str(&format!(
                "| {:<width_path$} | {:>width_size$} | {:>width_symbols$} |\n",
                "**TOTAL SHOWN**",
                format_size(total_shown),
                total_symbols,
                width_path = max_path_len,
                width_size = size_width,
                width_symbols = symbols_width
            ));
            s.push_str(&format!(
                "| {:<width_path$} | {:>width_size$} | {:>width_symbols$} |\n",
                "**.text section**",
                format_size(text_section_size),
                format!("{:.1}%", coverage),
                width_path = max_path_len,
                width_size = size_width,
                width_symbols = symbols_width
            ));

            s.push('\n');
        }

        if !r.top_rodata_symbols.is_empty() {
            s.push_str(&format!("### Top .rodata groups (depth {})\n\n", depth));
            let groups = group_symbols_by_depth(&r.top_rodata_symbols, depth);

            // Calculate column widths
            let max_path_len = groups
                .iter()
                .map(|g| g.path.len() + 2) // +2 for backticks
                .max()
                .unwrap_or(4)
                .max(4)
                .max(17); // min width for "**.rodata section**"
            let size_width = 12; // "total size" width
            let symbols_width = 9; // "symbols" width

            s.push_str(&format!(
                "| {:<width_path$} | {:>width_size$} | {:>width_symbols$} |\n",
                "path",
                "total size",
                "symbols",
                width_path = max_path_len,
                width_size = size_width,
                width_symbols = symbols_width
            ));
            s.push_str(&format!(
                "|{:-<width_path$}|{:-<width_size$}:|{:-<width_symbols$}:|\n",
                "",
                "",
                "",
                width_path = max_path_len + 2,
                width_size = size_width + 2,
                width_symbols = symbols_width + 2
            ));

            for group in &groups {
                let path_cell = format!("`{}`", group.path);
                let size_cell = format_size(group.total_size);
                s.push_str(&format!(
                    "| {:<width_path$} | {:>width_size$} | {:>width_symbols$} |\n",
                    path_cell,
                    size_cell,
                    group.symbol_count,
                    width_path = max_path_len,
                    width_size = size_width,
                    width_symbols = symbols_width
                ));
            }

            // Add summary rows
            let total_shown: u64 = groups.iter().map(|g| g.total_size).sum();
            let total_symbols: usize = groups.iter().map(|g| g.symbol_count).sum();
            let rodata_section_size = r
                .sections
                .iter()
                .find(|s| s.name == ".rodata")
                .map(|s| s.size)
                .unwrap_or(0);
            let coverage = if rodata_section_size > 0 {
                (total_shown as f64 / rodata_section_size as f64) * 100.0
            } else {
                0.0
            };

            s.push_str(&format!(
                "|{:-<width_path$}|{:-<width_size$}:|{:-<width_symbols$}:|\n",
                "",
                "",
                "",
                width_path = max_path_len + 2,
                width_size = size_width + 2,
                width_symbols = symbols_width + 2
            ));
            s.push_str(&format!(
                "| {:<width_path$} | {:>width_size$} | {:>width_symbols$} |\n",
                "**TOTAL SHOWN**",
                format_size(total_shown),
                total_symbols,
                width_path = max_path_len,
                width_size = size_width,
                width_symbols = symbols_width
            ));
            s.push_str(&format!(
                "| {:<width_path$} | {:>width_size$} | {:>width_symbols$} |\n",
                "**.rodata section**",
                format_size(rodata_section_size),
                format!("{:.1}%", coverage),
                width_path = max_path_len,
                width_size = size_width,
                width_symbols = symbols_width
            ));

            s.push('\n');
        }
    }

    s
}
