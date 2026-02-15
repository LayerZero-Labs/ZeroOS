fn main() {
    // Declare the custom cfg for backtrace mode
    println!("cargo:rustc-check-cfg=cfg(zeroos_backtrace, values(\"off\", \"dwarf\", \"frame-pointers\"))");
}
