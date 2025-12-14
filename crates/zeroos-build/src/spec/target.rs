/// This follows the standard target triple format: {arch}-{vendor}-{sys}[-{abi}]
#[derive(Debug, Clone)]
pub struct TargetConfig {
    pub arch: String,

    pub vendor: String,

    pub os: String,
    /// ABI (e.g., "musl", "gnu", "" for none)
    pub abi: String,
}

impl TargetConfig {
    /// Parameters follow target triple order: arch, vendor, os, abi
    pub fn new(arch: String, vendor: String, os: String, abi: String) -> Self {
        Self {
            arch,
            vendor,
            os,
            abi,
        }
    }

    /// Format: {arch}-{vendor}-{os}[-{abi}]
    pub fn target_triple(&self) -> String {
        if self.abi.is_empty() {
            format!("{}-{}-{}", self.arch, self.vendor, self.os)
        } else {
            format!("{}-{}-{}-{}", self.arch, self.vendor, self.os, self.abi)
        }
    }
}
