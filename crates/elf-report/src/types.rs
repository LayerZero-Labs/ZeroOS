use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FileReport {
    pub path: String,
    pub file_kind: String,
    pub arch: String,
    pub is_stripped_guess: bool,
    pub map_used: bool,
    pub sections: Vec<SectionInfo>,
    pub top_text_symbols: Vec<SymbolInfo>,
    pub top_rodata_symbols: Vec<SymbolInfo>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct SectionInfo {
    pub name: String,
    pub size: u64,
    pub address: u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub demangled: String,
    pub size: u64,
    pub address: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct MapSymbol {
    pub(crate) address: u64,
    pub(crate) size: u64,
    pub(crate) name: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct SymbolGroup {
    pub path: String,
    pub total_size: u64,
    pub symbol_count: usize,
    pub symbols: Vec<SymbolInfo>,
}
