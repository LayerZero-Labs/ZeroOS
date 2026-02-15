/// Require exactly one cfg value to be set for a given cfg key.
///
/// This macro generates compile-time checks to ensure that exactly one of the
/// provided cfg values is active for a given configuration key.
///
/// # Example
///
/// ```rust,ignore
/// use zeroos_macros::require_exactly_one_cfg;
///
/// // Ensure exactly one target OS is configured
/// require_exactly_one_cfg!(target_os: "linux", "windows", "macos");
/// ```
///
/// This will:
/// 1. Check that at least one value is set (compile error if none)
/// 2. Check that at most one value is set (compile error if multiple)
///
/// Works with any cfg key-value pairs, both built-in (e.g., `target_os`, `target_arch`)
/// and custom cfgs defined in your project.
///
/// # Refactoring Example
///
/// **Before (15 lines of repetitive code):**
/// ```rust,ignore
/// // Ensure exactly one target OS is configured
/// #[cfg(not(any(
///     target_os = "linux",
///     target_os = "windows",
///     target_os = "macos"
/// )))]
/// compile_error!("No target OS selected!");
///
/// // Prevent multiple OS targets from being active simultaneously
/// #[cfg(all(target_os = "linux", target_os = "windows"))]
/// compile_error!("Multiple target OS selected: linux and windows");
///
/// #[cfg(all(target_os = "linux", target_os = "macos"))]
/// compile_error!("Multiple target OS selected: linux and macos");
///
/// #[cfg(all(target_os = "windows", target_os = "macos"))]
/// compile_error!("Multiple target OS selected: windows and macos");
/// ```
///
/// **After (2 lines with DRY macro):**
/// ```rust,ignore
/// use zeroos_macros::require_exactly_one_cfg;
/// require_exactly_one_cfg!(target_os: "linux", "windows", "macos");
/// ```
///
/// **Benefits:**
/// - DRY: No repetitive compile_error statements
/// - Maintainable: Add new values easily
/// - Clear: Intent is obvious
/// - Reusable: Works for any cfg key-value validation
#[macro_export]
macro_rules! require_exactly_one_cfg {
    ($cfg_key:ident: $($value:literal),+ $(,)?) => {
        // Check that at least one is set
        #[cfg(not(any(
            $($cfg_key = $value),+
        )))]
        compile_error!(concat!(
            "No ",
            stringify!($cfg_key),
            " mode selected! Expected one of: ",
            $($value, ", "),+
        ));

        // Check that at most one is set (generate all pairwise combinations)
        $crate::__require_exactly_one_cfg_pairs!($cfg_key: $($value),+);
    };
}

/// Internal helper macro to generate pairwise conflict checks.
///
/// For each pair of values, generates a compile_error if both are set.
#[doc(hidden)]
#[macro_export]
macro_rules! __require_exactly_one_cfg_pairs {
    // Base case: single value (no pairs to check)
    ($cfg_key:ident: $single:literal) => {};

    // Recursive case: check first value against all others, then recurse
    ($cfg_key:ident: $first:literal, $($rest:literal),+) => {
        $(
            #[cfg(all($cfg_key = $first, $cfg_key = $rest))]
            compile_error!(concat!(
                "Multiple ",
                stringify!($cfg_key),
                " modes selected: ",
                $first,
                " and ",
                $rest
            ));
        )+

        // Recurse with remaining values
        $crate::__require_exactly_one_cfg_pairs!($cfg_key: $($rest),+);
    };
}

/// Require at most one cfg value to be set for a given cfg key.
///
/// Similar to `require_exactly_one_cfg`, but allows zero values to be set.
///
/// # Example
///
/// ```rust,ignore
/// use zeroos_macros::require_at_most_one_cfg;
///
/// require_at_most_one_cfg!(
///     build_mode: "debug", "release", "profile"
/// );
/// ```
#[macro_export]
macro_rules! require_at_most_one_cfg {
    ($cfg_key:ident: $($value:literal),+ $(,)?) => {
        // Only check for conflicts (allow none to be set)
        $crate::__require_exactly_one_cfg_pairs!($cfg_key: $($value),+);
    };
}
