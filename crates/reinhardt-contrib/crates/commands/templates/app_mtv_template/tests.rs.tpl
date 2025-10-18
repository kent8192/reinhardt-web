//! Tests for {{ app_name }}

// Re-export tests from tests module
pub use self::tests::*;

pub mod tests {
    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_example() {
            assert!(true);
        }
    }
}
