use core::arch::naked_asm;

extern "C" {
    fn __main_entry(argc: i32, argv: *const *const u8, envp: *const *const u8) -> i32;
}

/// # Safety
/// Must only be entered by early boot code.
#[unsafe(naked)]
#[no_mangle]
pub unsafe extern "C" fn __runtime_bootstrap() -> ! {
    naked_asm!(
        "   li      a0, 0",           // argc = 0
        "   li      a1, 0",           // argv = NULL
        "   li      a2, 0",           // envp = NULL
        "   tail    {main_entry}",    // tail call to __main_entry

        main_entry = sym __main_entry,
    )
}
