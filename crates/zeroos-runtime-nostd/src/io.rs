//! Basic I/O utilities for no_std environments.
//!
//! Provides a `StdoutWriter` that implements `core::fmt::Write` for
//! formatted output to the platform's stdout.

use core::fmt::Write;

extern "C" {
    /// Platform stdout write function.
    ///
    /// This function must be provided by the platform for outputting messages.
    /// It writes `len` bytes from `msg` to stdout.
    ///
    /// # Safety
    /// `msg` must be a valid pointer to `len` bytes of readable memory.
    pub fn __platform_stdout_write(msg: *const u8, len: usize);
}

/// A writer that outputs to platform stdout.
///
/// Implements `core::fmt::Write` to enable use with `write!` and `writeln!` macros.
///
/// # Example
/// ```ignore
/// use core::fmt::Write;
/// let mut writer = StdoutWriter;
/// writeln!(writer, "Hello, {}!", "world").ok();
/// ```
pub struct StdoutWriter;

impl Write for StdoutWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        unsafe {
            __platform_stdout_write(s.as_ptr(), s.len());
        }
        Ok(())
    }
}
