use super::target::TargetConfig;
use super::GENERIC_LINUX_TEMPLATE;
use crate::spec::llvm::LLVMConfig;
use crate::spec::ArchSpec;

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
    pub fn render(&self, arch_spec: &ArchSpec, llvm_config: &LLVMConfig) -> String {
        let template = GENERIC_LINUX_TEMPLATE;

        template
            .replace("{ARCH}", arch_spec.arch)
            .replace("{CPU}", arch_spec.cpu)
            .replace("{FEATURES}", &llvm_config.features)
            .replace("{LLVM_TARGET}", &llvm_config.llvm_target)
            .replace("{ABI}", &llvm_config.abi)
            .replace("{DATA_LAYOUT}", &llvm_config.data_layout)
            .replace("{POINTER_WIDTH}", arch_spec.pointer_width)
            .replace("{ENDIAN}", arch_spec.endian)
            .replace("{OS}", &self.os)
            .replace("{ENV}", &self.abi)
            .replace("{VENDOR}", &self.vendor)
            .replace(
                "{MAX_ATOMIC_WIDTH}",
                &arch_spec.max_atomic_width.to_string(),
            )
    }
}
