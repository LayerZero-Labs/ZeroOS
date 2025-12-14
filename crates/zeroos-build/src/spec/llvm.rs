#[derive(Debug, Clone)]
pub struct LLVMConfig {
    pub llvm_target: String,

    pub features: String,
    /// ABI/calling convention (e.g., "lp64", "ilp32")
    pub abi: String,
    /// LLVM data layout string
    pub data_layout: String,
}
