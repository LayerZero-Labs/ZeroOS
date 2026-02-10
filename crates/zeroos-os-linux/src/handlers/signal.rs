//! Signal handling for ZeroOS
//!
//! Implements lightweight signal support for zkVM environment.
//! Only handles SIGABRT for panic detection, other signals return ENOSYS.

use libc;

extern "C" {
    /// Platform-specific abort handler (provided by jolt-platform or similar).
    ///
    /// # Arguments
    /// * `sig` - The signal number that caused the abort (e.g., SIGABRT=6).
    ///   Exit code will be computed as `128 + sig` per Linux convention.
    fn __platform_abort(sig: i32) -> !;
}

/// Handle rt_sigaction syscall
///
/// For zkVM, we don't need full signal handler tables.
/// We accept all sigaction calls but ignore them (return success).
pub fn sys_rt_sigaction(_signum: usize, _act: usize, _oldact: usize, _sigsetsize: usize) -> isize {
    // Accept but ignore - we'll handle signals directly
    0
}

/// Handle rt_sigprocmask syscall
///
/// Signal masks aren't needed in zkVM - accept but ignore.
pub fn sys_rt_sigprocmask(_how: usize, _set: usize, _oldset: usize, _sigsetsize: usize) -> isize {
    // Accept but ignore
    0
}

/// Handle tkill syscall
///
/// Send signal to a thread. For zkVM, we only handle SIGABRT for panic detection.
pub fn sys_tkill(_tid: usize, sig: usize) -> isize {
    const SIGABRT: usize = 6;

    match sig {
        SIGABRT => {
            // This is a panic/abort - call platform-specific handler
            // Exit code will be 128 + 6 = 134 per Linux convention
            unsafe { __platform_abort(sig as i32) }
        }
        _ => {
            // Other signals not implemented
            -(libc::ENOSYS as isize)
        }
    }
}

/// Handle tgkill syscall
///
/// Similar to tkill but with thread group ID.
pub fn sys_tgkill(_tgid: usize, tid: usize, sig: usize) -> isize {
    // For single-threaded zkVM, tgkill behaves like tkill
    sys_tkill(tid, sig)
}
