use super::target::TargetConfig;
use super::GENERIC_LINUX_TEMPLATE;
use crate::spec::llvm::LLVMConfig;
use crate::spec::ArchSpec;
use mini_template as ztpl;

#[derive(Debug, Clone, Copy)]
pub struct TargetRenderOptions {
    pub backtrace: bool,
}

impl Default for TargetRenderOptions {
    fn default() -> Self {
        Self { backtrace: true }
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

        let ctx = ztpl::Context::new()
            .with_str("ARCH", arch_spec.arch)
            .with_str("CPU", arch_spec.cpu)
            .with_str("FEATURES", &llvm_config.features)
            .with_str("LLVM_TARGET", &llvm_config.llvm_target)
            .with_str("ABI", &llvm_config.abi)
            .with_str("DATA_LAYOUT", &llvm_config.data_layout)
            .with_str("POINTER_WIDTH", arch_spec.pointer_width)
            .with_str("ENDIAN", arch_spec.endian)
            .with_str("OS", &self.os)
            .with_str("ENV", &self.abi)
            .with_str("VENDOR", &self.vendor)
            .with_str("MAX_ATOMIC_WIDTH", arch_spec.max_atomic_width.to_string())
            // JSON booleans (rendered without quotes in template)
            .with_str("BACKTRACE", if opts.backtrace { "true" } else { "false" });

        ztpl::render(template, &ctx).map_err(|e| e.to_string())
    }
}
