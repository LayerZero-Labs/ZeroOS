//! Panic handler for no_std environments
//!
//! Provides a default panic handler that delegates to platform-specific abort.
//! Platforms must implement `__platform_abort()` which typically:
//! - Sets platform-specific panic bits/flags
//! - Calls platform exit with appropriate exit code
//!
//! ## Backtrace Support
//!
//! When the `backtrace` feature is enabled, the panic handler will print
//! stack frame addresses. See the [`crate::backtrace`] module for details.

use core::fmt::Write;
use core::panic::PanicInfo;

use crate::io::StdoutWriter;

#[cfg(feature = "backtrace")]
use crate::backtrace;

/// Standard signal number for abort (SIGABRT)
const SIGABRT: i32 = 6;

extern "C" {
    /// Platform-specific abort handler.
    ///
    /// This function must be provided by the platform (e.g., jolt-platform).
    /// It should set any platform-specific panic state and terminate execution
    /// with exit code `128 + sig` per Linux signal termination convention.
    ///
    /// # Arguments
    /// * `sig` - The signal number (typically SIGABRT=6 for panics)
    ///
    /// # Example implementation
    /// ```ignore
    /// #[no_mangle]
    /// pub extern "C" fn __platform_abort(sig: i32) -> ! {
    ///     // Set panic bit/flag for the platform
    ///     unsafe { platform_specific_panic_marker(); }
    ///     // Exit with signal-based code (128 + sig)
    ///     __platform_exit(128 + sig)
    /// }
    /// ```
    fn __platform_abort(sig: i32) -> !;
}

/// Default panic handler for no_std environments.
///
/// This handler:
/// 1. Prints panic information to stdout (via __platform_stdout_write)
/// 2. Optionally prints stack backtrace (with `backtrace` feature)
/// 3. Calls `__platform_abort()` to set panic state and exit
///
/// The platform must provide both `__platform_stdout_write()` and `__platform_abort()`.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut w = StdoutWriter;

    // Write panic location and message
    if let Some(location) = info.location() {
        let _ = writeln!(
            w,
            "PANIC: panicked at {}:{}:{}: {}",
            location.file(),
            location.line(),
            location.column(),
            info.message()
        );
    } else {
        let _ = writeln!(w, "PANIC: panicked: {}", info.message());
    }

    // Print stack backtrace if feature enabled
    #[cfg(feature = "backtrace")]
    backtrace::print_backtrace();

    // Call platform-specific abort handler with SIGABRT
    // This will exit with code 128 + 6 = 134
    unsafe { __platform_abort(SIGABRT) }
}
