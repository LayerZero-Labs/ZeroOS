mod arch;
mod llvm;
mod profiles;
mod target;
mod utils;

pub use arch::{extract_base_arch, get_arch_spec, ArchSpec};
pub use llvm::LLVMConfig;
pub use profiles::{
    list_profiles, load_target_profile, TargetProfile, PROFILE_RISCV64IMAC_ZERO_LINUX_MUSL,
};
pub use target::TargetConfig;
pub use utils::{parse_target_triple, TargetRenderOptions};

const GENERIC_LINUX_TEMPLATE: &str = include_str!("../files/generic-linux.json.template");
