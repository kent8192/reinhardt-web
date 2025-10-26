//! Path matching utilities

use crate::PathPattern;
use std::collections::HashMap;

/// Check if a path matches a pattern
pub fn path_matches(path: &str, pattern: &str) -> bool {
    if let Ok(pat) = PathPattern::new(pattern) {
        pat.is_match(path)
    } else {
        path == pattern
    }
}

/// Extract parameters from a path using a pattern
pub fn extract_params(path: &str, pattern: &str) -> HashMap<String, String> {
    if let Ok(pat) = PathPattern::new(pattern) {
        pat.extract_params(path).unwrap_or_default()
    } else {
        HashMap::new()
    }
}
