#![cfg_attr(target_os = "none", no_std)]
#![no_main]

use core::arch::asm;

cfg_if::cfg_if! {
    if #[cfg(target_os = "none")] {
        use platform::println;
    } else {
        use std::println;
    }
}

// Linux RISC-V syscall numbers (rv64/rv32 use the same Linux syscall table).
// We hardcode to avoid depending on the `libc` crate, which doesn't build for `*-none-elf`.
const SYS_UNKNOWN: usize = 0x1fff;

#[no_mangle]
#[inline(never)]
unsafe fn syscall_unknown() -> isize {
    let ret: isize;
    asm!(
        "ecall",
        inlateout("a0") 0isize => ret,
        lateout("a1") _,
        lateout("a2") _,
        lateout("a3") _,
        lateout("a4") _,
        lateout("a5") _,
        lateout("a6") _,
        in("a7") SYS_UNKNOWN,
        options(nostack),
    );
    ret
}

#[no_mangle]
fn main() -> ! {
    debug::writeln!("[BOOT] syscall-cycles");
    println!("Test STARTED!");

    // Need multiple hits for `cargo xtask spike-syscall-instcount` to measure per-syscall cost.
    let iters = 10;
    let mut unknown_ret = 0isize;

    for _ in 0..iters {
        unknown_ret = unsafe { syscall_unknown() };
    }

    println!(
        "syscall:unknown(nr={}): best=<use spike log parser> (last ret={})",
        SYS_UNKNOWN, unknown_ret
    );
    println!("Test PASSED!");

    platform::exit(0)
}
