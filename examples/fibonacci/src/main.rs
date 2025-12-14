#![cfg_attr(target_os = "none", no_std)]
#![no_main]

use fibonacci::fibonacci;

cfg_if::cfg_if! {
    if #[cfg(target_os = "none")] {

        use platform::println;
    } else {

        use std::println;
    }
}

#[no_mangle]
fn main() -> ! {
    debug::writeln!("[BOOT] main");
    let result = fibonacci(10);
    debug::writeln!("[BOOT] fibonacci(10) = {}", result);
    debug::writeln!("[BOOT] About to call println!");
    println!("fibonacci(10) = {}", result);
    debug::writeln!("[BOOT] println! completed!");
    debug::writeln!("[BOOT] Test PASSED!");
    platform::exit(0)
}
