//! Backtrace demo for ZeroOS
//!
//! This example demonstrates panic handling with stack traces.
//! Run with symbols preserved to see function names in the backtrace.
//!
//! Build for spike:
//!   cargo build -p backtrace --features with-spike --release
//!
//! Run on spike (with symbols for backtrace):
//!   spike pk target/riscv64imac-unknown-none-elf/release/backtrace

#![cfg_attr(target_os = "none", no_std)]
#![no_main]

cfg_if::cfg_if! {
    if #[cfg(target_os = "none")] {
        use platform::println;
    } else {
        use std::println;
    }
}

/// Entry point - demonstrates panic with nested stack frames.
#[no_mangle]
fn main() -> ! {
    debug::writeln!("[BACKTRACE] Starting backtrace demo");
    println!("Backtrace demo: about to trigger panic through nested calls");

    // Call through several layers to demonstrate stack trace
    level_one(true);

    // Should not reach here
    debug::writeln!("[BACKTRACE] ERROR: should have panicked!");
    platform::exit(1)
}

/// First level of nested calls
#[inline(never)]
fn level_one(should_panic: bool) {
    debug::writeln!("[BACKTRACE] level_one");
    level_two(should_panic);
}

/// Second level of nested calls
#[inline(never)]
fn level_two(should_panic: bool) {
    debug::writeln!("[BACKTRACE] level_two");
    level_three(should_panic);
}

/// Third level - triggers the panic
#[inline(never)]
fn level_three(should_panic: bool) {
    debug::writeln!("[BACKTRACE] level_three");
    if should_panic {
        panic!("intentional panic for backtrace demo");
    }
}
