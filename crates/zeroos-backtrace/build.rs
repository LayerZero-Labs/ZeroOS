fn main() {
    // Declare expected cfg values for the zeroos_backtrace configuration
    println!(
        "cargo::rustc-check-cfg=cfg(zeroos_backtrace, values(\"off\", \"dwarf\", \"frame_pointers\"))"
    );
}
