fn main() {
    // Fail fast: this crate is Linux-only.
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if os != "linux" {
        panic!(
            "{} is Linux-only; build with a Linux target (e.g. `*-unknown-linux-*`).",
            env!("CARGO_PKG_NAME")
        );
    }
}
