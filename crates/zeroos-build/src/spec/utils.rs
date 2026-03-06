use super::target::TargetConfig;
use super::GENERIC_LINUX_TEMPLATE;
use crate::spec::llvm::LLVMConfig;
use crate::spec::ArchSpec;
use mini_template as ztpl;

/// Returns the rustc minor version (e.g. 91 for 1.91.0), or None if it cannot be determined.
/// Respects the RUSTC env var so that callers running inside a Cargo build script use the
/// correct toolchain.
fn rustc_minor_version() -> Option<u32> {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let output = std::process::Command::new(rustc)
        .arg("--version")
        .output()
        .ok()?;
    // "rustc 1.94.0 (4a4ef493e 2026-03-02)"
    let stdout = String::from_utf8(output.stdout).ok()?;
    let version = stdout.split_whitespace().nth(1)?;
    version.split('.').nth(1)?.parse().ok()
}

#[derive(Debug, Clone, Copy)]
pub struct TargetRenderOptions {
    /// Whether to emit DWARF unwind tables (.eh_frame sections)
    pub emit_unwind_tables: bool,
}

impl Default for TargetRenderOptions {
    fn default() -> Self {
        Self {
            emit_unwind_tables: true,
        }
    }
}

pub fn parse_target_triple(target: &str) -> Option<TargetConfig> {
    // Parse target triple: {arch}-{vendor}-{sys}[-{abi}]

    //   - riscv64gc-unknown-linux-musl (with abi)
    //   - aarch64-apple-darwin (without abi)
    let parts: Vec<&str> = target.split('-').collect();
    if parts.len() < 3 || parts.len() > 4 {
        return None;
    }

    let arch = parts[0];
    let vendor = parts[1];
    let os = parts[2];
    let abi = if parts.len() == 4 {
        parts[3]
    } else {
        "" // No abi
    };

    Some(TargetConfig::new(
        arch.to_string(),
        vendor.to_string(),
        os.to_string(),
        abi.to_string(),
    ))
}

impl TargetConfig {
    pub fn render(
        &self,
        arch_spec: &ArchSpec,
        llvm_config: &LLVMConfig,
        opts: TargetRenderOptions,
    ) -> Result<String, String> {
        let template = GENERIC_LINUX_TEMPLATE;

        // target-pointer-width changed from a JSON string to a JSON integer in rustc 1.91
        // (rust-lang/rust#144443). Default to the integer form for unknown/new versions.
        let pointer_width_json = match rustc_minor_version() {
            Some(minor) if minor < 91 => format!("\"{}\"", arch_spec.pointer_width),
            _ => arch_spec.pointer_width.to_string(),
        };

        let ctx = ztpl::Context::new()
            .with_str("ARCH", arch_spec.arch)
            .with_str("CPU", arch_spec.cpu)
            .with_str("FEATURES", &llvm_config.features)
            .with_str("LLVM_TARGET", &llvm_config.llvm_target)
            .with_str("ABI", &llvm_config.abi)
            .with_str("DATA_LAYOUT", &llvm_config.data_layout)
            .with_str("POINTER_WIDTH", &pointer_width_json)
            .with_str("ENDIAN", arch_spec.endian)
            .with_str("OS", &self.os)
            .with_str("ENV", &self.abi)
            .with_str("VENDOR", &self.vendor)
            .with_str("MAX_ATOMIC_WIDTH", arch_spec.max_atomic_width.to_string())
            // JSON booleans (rendered without quotes in template)
            .with_str(
                "EMIT_UNWIND_TABLES",
                if opts.emit_unwind_tables {
                    "true"
                } else {
                    "false"
                },
            );

        ztpl::render(template, &ctx).map_err(|e| e.to_string())
    }
}
