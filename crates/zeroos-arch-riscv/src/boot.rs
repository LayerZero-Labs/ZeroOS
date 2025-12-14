use core::arch::naked_asm;

/// # Safety
/// Must only be entered by firmware/boot code in a valid reset context.
#[unsafe(naked)]
#[link_section = ".text.boot"]
#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        // Initialize global pointer first (RISC-V ABI requirement)
        ".weak __global_pointer$",
        ".hidden __global_pointer$",
        ".option push",
        ".option norelax",
        "   lla     gp, __global_pointer$",
        ".option pop",

        ".weak __stack_top",
        ".hidden __stack_top",
        "   lla     sp, __stack_top",
        "   andi    sp, sp, -16",

        "   call    {trace_start}",

        "   tail    {bootstrap}",

        trace_start = sym __boot_trace_start,
        bootstrap = sym __bootstrap,
    )
}

/// # Safety
/// Must only be entered from `_start` during early boot.
#[unsafe(naked)]
#[no_mangle]
pub unsafe extern "C" fn __bootstrap() -> ! {
    naked_asm!(
        "   call    {trace_bootstrap}",
        "   call    {platform_bootstrap}",
        "   tail    {runtime_bootstrap}",

        // Safety: If main() returns, halt forever.
        // User should call exit() to terminate properly.
        // Using inline asm instead of `loop {}` or `spin_loop()` because:
        // - `loop {}` may be optimized away as undefined behavior
        // - `spin_loop()` generates `pause` only with Zihintpause extension,
        //   otherwise no instruction on RISC-V
        // - `j .` guarantees a single-instruction infinite loop
        "   j       .",

        trace_bootstrap = sym __boot_trace_bootstrap,
        platform_bootstrap = sym crate::__platform_bootstrap,
        runtime_bootstrap = sym crate::__runtime_bootstrap,
    )
}

#[no_mangle]
extern "C" fn __boot_trace_start() {
    debug::writeln!("[BOOT] _start");
}

#[no_mangle]
extern "C" fn __boot_trace_bootstrap() {
    debug::writeln!("[BOOT] __bootstrap");
}
