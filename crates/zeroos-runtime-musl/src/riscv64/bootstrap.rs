use crate::build_musl_stack;
use core::arch::naked_asm;

use foundation::__main_entry;

extern "C" {
    fn __libc_start_main(
        main_fn: extern "C" fn(i32, *const *const u8, *const *const u8) -> i32,
        argc: i32,
        argv: *const *const u8,
        init: extern "C" fn(),
        fini: extern "C" fn(),
        ldso_dummy: Option<extern "C" fn()>,
    ) -> i32;

}

#[no_mangle]
pub extern "C" fn _init() {}

#[no_mangle]
pub extern "C" fn _fini() {}

static PROGRAM_NAME: &[u8] = b"zerokernel\0";

#[no_mangle]
extern "C" fn __boot_trace_runtime() {
    debug::writeln!("[BOOT] __runtime_bootstrap");
}

// SAFETY: `MUSL_BUILD_BUFFER` must be large enough for `build_musl_stack` output.
// adjust `MUSL_BUFFER_SIZE` if musl stack layout grows.
const MUSL_BUFFER_SIZE: usize = 512;
const MUSL_BUFFER_BYTES: usize = MUSL_BUFFER_SIZE * core::mem::size_of::<usize>();

static mut MUSL_BUILD_BUFFER: [usize; MUSL_BUFFER_SIZE] = [0; MUSL_BUFFER_SIZE];

unsafe fn build_musl_in_buffer() -> usize {
    let buffer_ptr = core::ptr::addr_of_mut!(MUSL_BUILD_BUFFER) as *mut usize;
    let buffer_bottom = buffer_ptr as usize;
    let buffer_top = buffer_ptr.add(MUSL_BUFFER_SIZE) as usize;

    let size = build_musl_stack(buffer_top, buffer_bottom, PROGRAM_NAME);

    if size > MUSL_BUFFER_BYTES {
        panic!(
            "Musl stack overflow! Used {} bytes, buffer is {} bytes",
            size, MUSL_BUFFER_BYTES
        );
    }

    size
}

/// # Safety
/// Must only be entered by early boot code.
#[unsafe(naked)]
#[no_mangle]
pub unsafe extern "C" fn __runtime_bootstrap() -> ! {
    naked_asm!(
        "   call    {trace_runtime}",
        "   call    {build_impl}",
        "   mv      t2, a0",
        "   la      t0, {buffer}",
        "   li      t1, {buffer_bytes}",
        "   add     t0, t0, t1",
        "   sub     t6, t0, t2",
        "   addi    sp, sp, -512",
        "   sub     t3, sp, t2",
        "   mv      t5, t3",

        "1:",
        "   beqz    t2, 2f",
        "   ld      t1, 0(t6)",
        "   sd      t1, 0(t3)",
        "   addi    t6, t6, 8",
        "   addi    t3, t3, 8",
        "   addi    t2, t2, -8",
        "   j       1b",
        "2:",
        "   mv      sp, t5",

        "   la      a0, {main}",         // a0 = main function
        "   ld      a1, 0(sp)",          // a1 = argc (int, 64-bit)
        "   addi    a2, sp, 8",          // a2 = argv (char**)
        "   la      a3, {init}",         // a3 = _init
        "   la      a4, {fini}",         // a4 = _fini
        "   li      a5, 0",              // a5 = NULL (ldso_dummy)
        "   li      a5, 0",

        "   tail    {libc_start_main}",

        trace_runtime = sym __boot_trace_runtime,
        build_impl = sym build_musl_in_buffer,
        buffer = sym MUSL_BUILD_BUFFER,
        buffer_bytes = const MUSL_BUFFER_BYTES,
        main = sym __main_entry,
        init = sym _init,
        fini = sym _fini,
        libc_start_main = sym __libc_start_main,
    )
}
