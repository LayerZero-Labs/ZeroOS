fn main() {
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if arch != "riscv32" && arch != "riscv64" {
        panic!(
            "{} is RISC-V-only; build with a RISC-V target (current: {}).",
            env!("CARGO_PKG_NAME"),
            arch
        );
    }
}
