use anyhow::{Context, Result};
use mini_template as ztpl;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct LinkerConfig {
    pub memory_origin: usize,

    pub memory_size: usize,

    pub heap_size: Option<usize>,

    pub stack_size: usize,

    pub backtrace: bool,

    template: Option<String>,
}

impl Default for LinkerConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl LinkerConfig {
    pub fn new() -> Self {
        Self {
            memory_origin: DEFAULT_MEMORY_ORIGIN,
            memory_size: DEFAULT_MEMORY_SIZE,
            heap_size: None,
            stack_size: DEFAULT_STACK_SIZE,
            backtrace: false,
            template: None,
        }
    }

    pub fn with_heap_size(mut self, size: usize) -> Self {
        self.heap_size = Some(size);
        self
    }

    pub fn with_stack_size(mut self, size: usize) -> Self {
        self.stack_size = size;
        self
    }

    pub fn with_memory(mut self, origin: usize, size: usize) -> Self {
        self.memory_origin = origin;
        self.memory_size = size;
        self
    }

    pub fn with_template(mut self, template: String) -> Self {
        self.template = Some(template);
        self
    }

    pub fn with_backtrace(mut self, backtrace: bool) -> Self {
        self.backtrace = backtrace;
        self
    }

    pub fn heap_size(&self) -> usize {
        self.heap_size
            .unwrap_or_else(|| self.memory_size.saturating_sub(self.stack_size))
    }
}

pub const DEFAULT_MEMORY_ORIGIN: usize = 0x8000_0000;

pub const DEFAULT_MEMORY_SIZE: usize = 128 * 1024 * 1024;

pub const DEFAULT_STACK_SIZE: usize = 4 * 1024 * 1024;

impl LinkerConfig {
    pub fn render(&self, template: Option<String>) -> String {
        let origin = format!("{:#x}", self.memory_origin);
        let mem_size = format!("{:#x}", self.memory_size);
        let heap_size = format!("{:#x}", self.heap_size());
        let stack_size = format!("{:#x}", self.stack_size);

        let template = template
            .as_deref()
            .or(self.template.as_deref())
            .unwrap_or(LINKER_SCRIPT_TEMPLATE);
        let ctx = ztpl::Context::new()
            .with_bool("backtrace", self.backtrace)
            .with_str("MEMORY_ORIGIN", origin)
            .with_str("MEMORY_SIZE", mem_size)
            .with_str("HEAP_SIZE", heap_size)
            .with_str("STACK_SIZE", stack_size);

        ztpl::render(template, &ctx).unwrap_or_else(|_| template.to_string())
    }
}

const LINKER_SCRIPT_TEMPLATE: &str = include_str!("files/linker.ld.template");

pub fn generate_linker_script(config: &LinkerConfig, output_path: &Path) -> Result<()> {
    let script_content = config.render(None);
    fs::write(output_path, script_content)
        .with_context(|| format!("Failed to write linker script to {}", output_path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LinkerConfig::default();
        assert_eq!(config.memory_origin, 0x8000_0000);
        assert_eq!(config.memory_size, 128 * 1024 * 1024);
        assert_eq!(config.stack_size, 1024 * 1024);
        assert!(config.heap_size.is_none());
    }

    #[test]
    fn test_heap_size_calculation() {
        let config = LinkerConfig::new()
            .with_memory(0x80000000, 128 * 1024 * 1024)
            .with_stack_size(8 * 1024 * 1024);

        assert_eq!(config.heap_size(), 120 * 1024 * 1024);
    }

    #[test]
    fn test_explicit_heap_size() {
        let config = LinkerConfig::new().with_heap_size(64 * 1024 * 1024);

        assert_eq!(config.heap_size(), 64 * 1024 * 1024);
    }
}
