#[derive(Debug, Clone)]
pub struct ArchSpec {
    pub arch: &'static str,
    /// CPU name (e.g., "generic-rv64", "generic-rv32")
    pub cpu: &'static str,

    pub pointer_width: &'static str,
    /// Maximum atomic width in bits
    pub max_atomic_width: u32,
    /// Target endianness
    pub endian: &'static str,
}

pub fn extract_base_arch(arch: &str) -> &str {
    match arch {
        a if a.starts_with("riscv64") => "riscv64",
        a if a.starts_with("riscv32") => "riscv32",
        _ => arch,
    }
}

pub fn get_arch_spec(arch: &str) -> ArchSpec {
    let base = extract_base_arch(arch);
    match base {
        "riscv64" => ArchSpec {
            arch: "riscv64",
            cpu: "generic-rv64",
            pointer_width: "64",
            max_atomic_width: 64,
            endian: "little",
        },
        "riscv32" => ArchSpec {
            arch: "riscv32",
            cpu: "generic-rv32",
            pointer_width: "32",
            max_atomic_width: 32,
            endian: "little",
        },
        _ => panic!(
            "Unsupported architecture: {}. Currently only riscv64 and riscv32 are supported.",
            arch
        ),
    }
}
