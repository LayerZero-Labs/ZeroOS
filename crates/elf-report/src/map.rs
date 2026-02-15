use std::{fs, path::Path};

use anyhow::{Context, Result};

use crate::types::MapSymbol;

fn parse_hex_u64(s: &str) -> Option<u64> {
    let s = s.trim();
    let s = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    u64::from_str_radix(s, 16).ok()
}

fn looks_like_object_path(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return false;
    }
    if s.starts_with("*(") || s.starts_with('*') || s.starts_with('(') {
        return false;
    }

    // Common forms:
    // - /abs/path/foo.o
    // - relative/foo.o
    // - libfoo.a(bar.o)
    // - /lib/libc.so.6
    s.contains(".o") || s.contains(".a(") || s.contains(".so")
}

fn is_plain_section_label(name: &str) -> bool {
    // In GNU ld maps, output section headers appear as plain section names
    // like `.text`, `.rodata`, `.data`, etc.
    // We want to keep `.text.<symbol>` / `.rodata.<symbol>` but drop the plain labels.
    if !name.starts_with('.') {
        return false;
    }
    let rest = &name[1..];
    if rest.is_empty() {
        return true;
    }
    !rest.contains('.')
}

pub(crate) fn is_plain_section_label_public(name: &str) -> bool {
    is_plain_section_label(name)
}

/// Best-effort parser for GNU ld / lld map files.
///
/// Strategy:
/// - Prefer parsing the GNU ld "Linker script and memory map" block structure, where symbol
///   lines often include only an address (no size). In that case, sizes are inferred by taking
///   the delta to the next symbol, and the last symbol is bounded by the containing input section.
/// - Also keep a permissive fallback for map formats that already provide (addr, size, name).
///
/// Attribution is then done by intersecting address ranges with ELF section ranges.
pub(crate) fn parse_map_symbols(map_path: &Path) -> Result<Vec<MapSymbol>> {
    let text = fs::read_to_string(map_path)
        .with_context(|| format!("reading map {}", map_path.display()))?;

    Ok(parse_map_symbols_text(&text))
}

fn parse_map_symbols_text(text: &str) -> Vec<MapSymbol> {
    #[derive(Debug)]
    struct BlockSymbol {
        address: u64,
        explicit_size: Option<u64>,
        name: String,
    }

    #[derive(Debug)]
    struct Block {
        start: u64,
        end: u64,
        _section: String,
        symbols: Vec<BlockSymbol>,
    }

    fn finish_block(block: Block, out: &mut Vec<MapSymbol>) {
        if block.symbols.is_empty() {
            return;
        }

        let mut syms = block.symbols;
        syms.sort_by_key(|s| s.address);

        for i in 0..syms.len() {
            let cur = &syms[i];
            let next_addr = syms.get(i + 1).map(|s| s.address).unwrap_or(block.end);

            let mut size = cur
                .explicit_size
                .unwrap_or_else(|| next_addr.saturating_sub(cur.address));

            // Clamp size to the containing input-section block.
            if cur.address >= block.end {
                continue;
            }
            let max_len = block.end.saturating_sub(cur.address);
            if size > max_len {
                size = max_len;
            }
            if size == 0 {
                continue;
            }

            out.push(MapSymbol {
                address: cur.address,
                size,
                name: cur.name.clone(),
            });
        }
    }

    let mut out = Vec::new();
    let mut fallback = Vec::new();
    let mut cur_block: Option<Block> = None;

    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }

        // Ignore common headers / boilerplate.
        if line.starts_with("Merging program properties")
            || line.starts_with("Updated property")
            || line.starts_with("Removed property")
            || line.starts_with("As-needed library")
            || line.starts_with("Discarded input sections")
            || line.starts_with("Memory Configuration")
            || line.starts_with("Linker script and memory map")
            || line.starts_with("LOAD ")
            || line.starts_with("START GROUP")
            || line.starts_with("END GROUP")
        {
            continue;
        }

        let trimmed = line.trim_start();
        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        // Detect input-section contribution blocks:
        //   .text  0xADDR  0xSIZE  /path/to/file.o
        //   .rodata 0xADDR 0xSIZE  libfoo.a(bar.o)
        if tokens.len() >= 4 && is_plain_section_label(tokens[0]) {
            if let (Some(addr), Some(sz)) = (parse_hex_u64(tokens[1]), parse_hex_u64(tokens[2])) {
                let rest = tokens[3..].join(" ");
                if sz > 0 && looks_like_object_path(&rest) {
                    if let Some(b) = cur_block.take() {
                        finish_block(b, &mut out);
                    }
                    cur_block = Some(Block {
                        start: addr,
                        end: addr.saturating_add(sz),
                        _section: tokens[0].to_string(),
                        symbols: Vec::new(),
                    });
                    continue;
                }
            }
        }

        // Output-section header lines like `.text 0x.. 0x..` are block boundaries.
        if tokens.len() == 3 && is_plain_section_label(tokens[0]) {
            if let (Some(_addr), Some(sz)) = (parse_hex_u64(tokens[1]), parse_hex_u64(tokens[2])) {
                if sz > 0 {
                    if let Some(b) = cur_block.take() {
                        finish_block(b, &mut out);
                    }
                    continue;
                }
            }
        }

        // If we're inside an input-section block, parse symbol lines.
        // Common GNU ld form:
        //   0xADDR                symbol
        // Sometimes includes a size:
        //   0xADDR 0xSIZE symbol
        if let Some(block) = cur_block.as_mut() {
            if tokens.len() >= 2 {
                if let Some(addr) = parse_hex_u64(tokens[0]) {
                    if addr < block.start || addr >= block.end {
                        continue;
                    }

                    let mut explicit_size: Option<u64> = None;
                    let mut name: Option<&str> = None;

                    if tokens.len() >= 3 {
                        if let Some(sz) = parse_hex_u64(tokens[1]) {
                            explicit_size = Some(sz);
                            name = tokens
                                .iter()
                                .skip(2)
                                .find(|t| parse_hex_u64(t).is_none())
                                .copied();
                        }
                    }

                    if name.is_none() {
                        name = tokens
                            .iter()
                            .skip(1)
                            .find(|t| parse_hex_u64(t).is_none())
                            .copied();
                    }

                    let Some(name) = name else {
                        continue;
                    };
                    if name == "*fill*" || name.starts_with('*') || name == "PROVIDE" {
                        continue;
                    }
                    if looks_like_object_path(name) {
                        continue;
                    }
                    if name.starts_with('.') {
                        // Drop local labels like `.L...` which are generally not actionable.
                        continue;
                    }

                    block.symbols.push(BlockSymbol {
                        address: addr,
                        explicit_size,
                        name: name.to_string(),
                    });

                    continue;
                }
            }
        }

        // Fallback: extract any line that contains an address + size + name.
        // Common shapes:
        // 1) <name> 0xADDR 0xSIZE <obj>
        // 2) 0xADDR 0xSIZE <name> <obj>
        // 3) <name> 0xADDR 0xSIZE
        if tokens.len() >= 3 {
            let mut name: Option<&str> = None;
            let mut addr: Option<u64> = None;
            let mut size: Option<u64> = None;

            // shape 1
            if let (Some(a), Some(sz)) = (parse_hex_u64(tokens[1]), parse_hex_u64(tokens[2])) {
                name = Some(tokens[0]);
                addr = Some(a);
                size = Some(sz);
            }

            // shape 2
            if name.is_none() {
                if let (Some(a), Some(sz)) = (parse_hex_u64(tokens[0]), parse_hex_u64(tokens[1])) {
                    for t in tokens.iter().skip(2) {
                        if parse_hex_u64(t).is_none() {
                            name = Some(t);
                            addr = Some(a);
                            size = Some(sz);
                            break;
                        }
                    }
                }
            }

            let (Some(address), Some(size), Some(name)) = (addr, size, name) else {
                continue;
            };
            if size == 0 {
                continue;
            }
            if name == "*fill*" || name.starts_with('*') || name == "PROVIDE" {
                continue;
            }
            if looks_like_object_path(name) {
                continue;
            }
            if is_plain_section_label(name) {
                continue;
            }

            fallback.push(MapSymbol {
                address,
                size,
                name: name.to_string(),
            });
        }
    }

    if let Some(b) = cur_block.take() {
        finish_block(b, &mut out);
    }

    // Merge fallback symbols, preferring the larger size if duplicates exist.
    out.extend(fallback);
    out.retain(|s| !(s.name.is_empty() || s.name == "*fill*" || is_plain_section_label(&s.name)));
    out.sort_by(|a, b| (a.address, &a.name).cmp(&(b.address, &b.name)));
    out.dedup_by(|a, b| {
        if a.address == b.address && a.name == b.name {
            if b.size > a.size {
                a.size = b.size;
            }
            true
        } else {
            false
        }
    });

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gnu_ld_block_symbols_and_infers_sizes() {
        let map = r#"
Linker script and memory map

.text          0x0000000000000640      0x18c
 *(.text .stub .text.* .gnu.linkonce.t.*)
 .text          0x0000000000000768       0x64 /tmp/cc1j4yF2.o
                0x0000000000000768                foo
                0x0000000000000788                bar
                0x00000000000007b4                main
"#;

        let syms = parse_map_symbols_text(map);

        let foo = syms.iter().find(|s| s.name == "foo").unwrap();
        let bar = syms.iter().find(|s| s.name == "bar").unwrap();
        let main = syms.iter().find(|s| s.name == "main").unwrap();

        assert_eq!(foo.address, 0x768);
        assert_eq!(bar.address, 0x788);
        assert_eq!(main.address, 0x7b4);

        // Sizes inferred by deltas and block end (0x768 + 0x64 = 0x7cc).
        assert_eq!(foo.size, 0x20);
        assert_eq!(bar.size, 0x2c);
        assert_eq!(main.size, 0x18);
    }

    #[test]
    fn drops_plain_section_labels_from_fallback_parser() {
        let map = r#"
.text           0x0000000000000640      0x18c
.text._Z3foov   0x0000000000000768       0x20 /tmp/x.o
"#;
        let syms = parse_map_symbols_text(map);
        assert!(!syms.iter().any(|s| s.name == ".text"));
        assert!(syms.iter().any(|s| s.name == ".text._Z3foov"));
    }
}
