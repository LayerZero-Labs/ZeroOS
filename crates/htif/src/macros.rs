// Print macros for HTIF console output

#[macro_export]
macro_rules! println {
        ($($arg:tt)*) => {{
            use core::fmt::Write;
            let _ = core::writeln!($crate::DebugWriter, $($arg)*);
        }};
    }

#[macro_export]
macro_rules! eprintln {
    ($($arg:tt)*) => { $crate::println!($($arg)*); }
}
