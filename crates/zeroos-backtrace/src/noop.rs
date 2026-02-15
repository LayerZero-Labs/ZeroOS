//! No-op backtrace implementation for minimal binary size.
//!
//! This implementation is selected when `--backtrace=off` is specified.
//! It provides empty implementations of all backtrace functions, allowing
//! dead code elimination to remove all backtrace-related code.
//!
//! CRITICAL: Sets a custom panic hook that does NOT use std::backtrace,
//! allowing the linker to eliminate std::backtrace_rs, gimli, and addr2line.

use crate::BacktraceCapture;

/// No-op backtrace implementation.
///
/// This struct provides empty implementations for all backtrace operations.
/// The compiler will optimize away all backtrace-related code when this
/// implementation is selected.
pub struct NoopBacktrace;

impl BacktraceCapture for NoopBacktrace {
    #[inline(always)]
    fn init() {
        #[cfg(not(target_os = "none"))]
        {
            extern crate std;
            use std::boxed::Box;
            use std::panic;
            use std::string::String;
            use std::sync::Once;

            // Set a custom panic hook that does NOT use backtrace.
            // This allows the linker to eliminate std::backtrace_rs and gimli as dead code.
            static INIT: Once = Once::new();

            INIT.call_once(|| {
                panic::set_hook(Box::new(|info| {
                    // Extract panic message
                    let msg = if let Some(&s) = info.payload().downcast_ref::<&str>() {
                        s
                    } else if let Some(s) = info.payload().downcast_ref::<String>() {
                        s.as_str()
                    } else {
                        ""
                    };

                    // Print panic info WITHOUT backtrace
                    if let Some(location) = info.location() {
                        if !msg.is_empty() {
                            std::eprintln!("\nthread panicked at {}:\n{}", location, msg);
                        } else {
                            std::eprintln!("\nthread panicked at {}", location);
                        }
                    } else if !msg.is_empty() {
                        std::eprintln!("\nthread panicked: {}", msg);
                    } else {
                        std::eprintln!("\nthread panicked");
                    }
                    // Explicitly do NOT call any backtrace functions here
                }));
            });
        }
    }

    #[inline(always)]
    unsafe fn print_backtrace() {
        // No backtrace output
    }
}
