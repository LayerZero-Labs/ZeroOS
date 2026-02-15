/// Extract the crate/module path at the specified depth from a demangled symbol.
///
/// # Examples
/// ```ignore
/// assert_eq!(extract_path_at_depth("std::fmt::write", 1), Some("std".into()));
/// assert_eq!(extract_path_at_depth("std::fmt::write", 2), Some("std::fmt".into()));
/// assert_eq!(extract_path_at_depth("<core::iter::Skip<I> as Iterator>::next", 1), Some("core".into()));
/// assert_eq!(extract_path_at_depth("main", 1), None);
/// ```
pub fn extract_path_at_depth(demangled: &str, depth: usize) -> Option<String> {
    if depth == 0 {
        return None;
    }

    let mut working = demangled;

    // Handle trait impls: <Type as Trait>::method
    if let Some(stripped) = working.strip_prefix('<') {
        if let Some(as_pos) = stripped.find(" as ") {
            working = &stripped[..as_pos];
        } else if let Some(gt_pos) = stripped.find('>') {
            working = &stripped[..gt_pos];
        }
    }

    // Remove closure markers
    working = working.split("::{{closure}}").next().unwrap_or(working);

    // Remove generics
    if let Some(lt_pos) = working.find('<') {
        working = &working[..lt_pos];
    }

    // No :: means not a Rust symbol
    if !working.contains("::") {
        return None;
    }

    // Split and take first `depth` components
    let parts: Vec<&str> = working.split("::").collect();
    if parts.len() < depth {
        Some(parts.join("::"))
    } else {
        Some(parts[..depth].join("::"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_path() {
        assert_eq!(
            extract_path_at_depth("std::fmt::write", 1),
            Some("std".into())
        );
        assert_eq!(
            extract_path_at_depth("std::fmt::write", 2),
            Some("std::fmt".into())
        );
        assert_eq!(
            extract_path_at_depth("std::fmt::write", 3),
            Some("std::fmt::write".into())
        );
    }

    #[test]
    fn test_trait_impl() {
        let symbol = "<core::iter::Skip<I> as Iterator>::next";
        assert_eq!(extract_path_at_depth(symbol, 1), Some("core".into()));
        assert_eq!(extract_path_at_depth(symbol, 2), Some("core::iter".into()));
        assert_eq!(
            extract_path_at_depth(symbol, 3),
            Some("core::iter::Skip".into())
        );
    }

    #[test]
    fn test_trait_impl_without_as() {
        let symbol = "<std::collections::HashMap<K,V>>::new";
        assert_eq!(extract_path_at_depth(symbol, 1), Some("std".into()));
        assert_eq!(
            extract_path_at_depth(symbol, 2),
            Some("std::collections".into())
        );
    }

    #[test]
    fn test_closure() {
        let symbol = "std::io::read::{{closure}}";
        assert_eq!(extract_path_at_depth(symbol, 1), Some("std".into()));
        assert_eq!(extract_path_at_depth(symbol, 2), Some("std::io".into()));
        assert_eq!(
            extract_path_at_depth(symbol, 3),
            Some("std::io::read".into())
        );
    }

    #[test]
    fn test_c_symbols() {
        assert_eq!(extract_path_at_depth("main", 1), None);
        assert_eq!(extract_path_at_depth("_start", 1), None);
        assert_eq!(extract_path_at_depth("__init_cpu_features", 1), None);
    }

    #[test]
    fn test_nested_generics() {
        let symbol = "std::collections::HashMap<K,V>::insert";
        assert_eq!(extract_path_at_depth(symbol, 1), Some("std".into()));
        assert_eq!(
            extract_path_at_depth(symbol, 2),
            Some("std::collections".into())
        );
        assert_eq!(
            extract_path_at_depth(symbol, 3),
            Some("std::collections::HashMap".into())
        );
    }

    #[test]
    fn test_depth_exceeds_path() {
        assert_eq!(
            extract_path_at_depth("std::fmt", 10),
            Some("std::fmt".into())
        );
        assert_eq!(extract_path_at_depth("core", 5), None); // No ::
    }

    #[test]
    fn test_depth_zero() {
        assert_eq!(extract_path_at_depth("std::fmt::write", 0), None);
    }

    #[test]
    fn test_complex_trait_impl() {
        let symbol = "<alloc::vec::Vec<T> as core::ops::drop::Drop>::drop";
        assert_eq!(extract_path_at_depth(symbol, 1), Some("alloc".into()));
        assert_eq!(extract_path_at_depth(symbol, 2), Some("alloc::vec".into()));
        assert_eq!(
            extract_path_at_depth(symbol, 3),
            Some("alloc::vec::Vec".into())
        );
    }
}
