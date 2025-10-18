//! String validators

use crate::{ValidationError, ValidationResult, Validator};
use regex::Regex;

/// Minimum length validator
pub struct MinLengthValidator {
    min: usize,
}

impl MinLengthValidator {
    /// Creates a new MinLengthValidator with the specified minimum length.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_validators::{MinLengthValidator, Validator};
    ///
    /// let validator = MinLengthValidator::new(5);
    /// assert!(validator.validate("hello").is_ok());
    /// assert!(validator.validate("hi").is_err());
    /// ```
    pub fn new(min: usize) -> Self {
        Self { min }
    }
}

impl Validator<String> for MinLengthValidator {
    fn validate(&self, value: &String) -> ValidationResult<()> {
        if value.len() >= self.min {
            Ok(())
        } else {
            Err(ValidationError::TooShort {
                length: value.len(),
                min: self.min,
            })
        }
    }
}

impl Validator<str> for MinLengthValidator {
    fn validate(&self, value: &str) -> ValidationResult<()> {
        if value.len() >= self.min {
            Ok(())
        } else {
            Err(ValidationError::TooShort {
                length: value.len(),
                min: self.min,
            })
        }
    }
}

/// Maximum length validator
pub struct MaxLengthValidator {
    max: usize,
}

impl MaxLengthValidator {
    /// Creates a new MaxLengthValidator with the specified maximum length.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_validators::{MaxLengthValidator, Validator};
    ///
    /// let validator = MaxLengthValidator::new(10);
    /// assert!(validator.validate("hello").is_ok());
    /// assert!(validator.validate("hello world").is_err());
    /// ```
    pub fn new(max: usize) -> Self {
        Self { max }
    }
}

impl Validator<String> for MaxLengthValidator {
    fn validate(&self, value: &String) -> ValidationResult<()> {
        if value.len() <= self.max {
            Ok(())
        } else {
            Err(ValidationError::TooLong {
                length: value.len(),
                max: self.max,
            })
        }
    }
}

impl Validator<str> for MaxLengthValidator {
    fn validate(&self, value: &str) -> ValidationResult<()> {
        if value.len() <= self.max {
            Ok(())
        } else {
            Err(ValidationError::TooLong {
                length: value.len(),
                max: self.max,
            })
        }
    }
}

/// Regex validator
pub struct RegexValidator {
    regex: Regex,
    message: String,
}

impl RegexValidator {
    /// Creates a new RegexValidator with the specified regex pattern.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_validators::{RegexValidator, Validator};
    ///
    /// let validator = RegexValidator::new(r"^\d{3}-\d{4}$").unwrap();
    /// assert!(validator.validate("123-4567").is_ok());
    /// assert!(validator.validate("invalid").is_err());
    /// ```
    pub fn new(pattern: &str) -> Result<Self, regex::Error> {
        Ok(Self {
            regex: Regex::new(pattern)?,
            message: format!("Value must match pattern: {}", pattern),
        })
    }
    /// Sets a custom error message for the validator.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_validators::{RegexValidator, Validator};
    ///
    /// let validator = RegexValidator::new(r"^\d+$")
    ///     .unwrap()
    ///     .with_message("Value must contain only digits");
    ///
    /// assert!(validator.validate("12345").is_ok());
    /// assert!(validator.validate("abc").is_err());
    /// ```
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }
}

impl Validator<String> for RegexValidator {
    fn validate(&self, value: &String) -> ValidationResult<()> {
        if self.regex.is_match(value) {
            Ok(())
        } else {
            Err(ValidationError::PatternMismatch(self.message.clone()))
        }
    }
}

impl Validator<str> for RegexValidator {
    fn validate(&self, value: &str) -> ValidationResult<()> {
        if self.regex.is_match(value) {
            Ok(())
        } else {
            Err(ValidationError::PatternMismatch(self.message.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// // Tests based on Django validators/tests.py
    #[test]
    fn test_min_length_validator_valid() {
        let validator = MinLengthValidator::new(5);
        assert!(validator.validate("hello").is_ok());
        assert!(validator.validate("hello world").is_ok());
        assert!(validator.validate("12345").is_ok());
    }

    #[test]
    fn test_min_length_validator_invalid() {
        let validator = MinLengthValidator::new(5);
        let result = validator.validate("hi");
        assert!(result.is_err());
        if let Err(ValidationError::TooShort { length, min }) = result {
            assert_eq!(length, 2);
            assert_eq!(min, 5);
        } else {
            panic!("Expected TooShort error");
        }
    }

    #[test]
    fn test_min_length_validator_edge_cases() {
        let validator = MinLengthValidator::new(0);
        assert!(validator.validate("").is_ok());

        let validator = MinLengthValidator::new(1);
        assert!(validator.validate("a").is_ok());
        assert!(validator.validate("").is_err());
    }

    #[test]
    fn test_min_length_validator_unicode() {
        let validator = MinLengthValidator::new(3);
        // Unicode characters count as single characters in byte length
        assert!(validator.validate("abc").is_ok());
        assert!(validator.validate("日本語").is_ok()); // 9 bytes, 3 chars
    }

    #[test]
    fn test_max_length_validator_valid() {
        let validator = MaxLengthValidator::new(10);
        assert!(validator.validate("hello").is_ok());
        assert!(validator.validate("1234567890").is_ok());
        assert!(validator.validate("").is_ok());
    }

    #[test]
    fn test_max_length_validator_invalid() {
        let validator = MaxLengthValidator::new(10);
        let result = validator.validate("hello world");
        assert!(result.is_err());
        if let Err(ValidationError::TooLong { length, max }) = result {
            assert_eq!(length, 11);
            assert_eq!(max, 10);
        } else {
            panic!("Expected TooLong error");
        }
    }

    #[test]
    fn test_max_length_validator_edge_cases() {
        let validator = MaxLengthValidator::new(0);
        assert!(validator.validate("").is_ok());
        assert!(validator.validate("a").is_err());

        let validator = MaxLengthValidator::new(1);
        assert!(validator.validate("a").is_ok());
        assert!(validator.validate("ab").is_err());
    }

    /// // Based on Django test_regex_validator_flags
    #[test]
    fn test_regex_validator_basic() {
        let validator = RegexValidator::new(r"^\d{3}-\d{4}$").unwrap();
        assert!(validator.validate("123-4567").is_ok());
        assert!(validator.validate("invalid").is_err());
    }

    #[test]
    fn test_regex_validator_pattern_matching() {
        // URL-like pattern
        let validator = RegexValidator::new(r"^(?:[a-z0-9.-]*)://").unwrap();
        assert!(validator.validate("http://example.com").is_ok());
        assert!(validator.validate("https://example.com").is_ok());
        assert!(validator.validate("ftp://example.com").is_ok());
        assert!(validator.validate("invalid").is_err());
    }

    #[test]
    fn test_regex_validator_with_custom_message() {
        let validator = RegexValidator::new(r"^\d+$")
            .unwrap()
            .with_message("Value must contain only digits");

        assert!(validator.validate("12345").is_ok());

        let result = validator.validate("abc");
        assert!(result.is_err());
        if let Err(ValidationError::PatternMismatch(msg)) = result {
            assert_eq!(msg, "Value must contain only digits");
        } else {
            panic!("Expected PatternMismatch error");
        }
    }

    #[test]
    fn test_regex_validator_email_pattern() {
        let validator =
            RegexValidator::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        assert!(validator.validate("test@example.com").is_ok());
        assert!(validator.validate("user.name+tag@example.co.uk").is_ok());
        assert!(validator.validate("invalid@").is_err());
        assert!(validator.validate("@example.com").is_err());
        assert!(validator.validate("invalid").is_err());
    }

    #[test]
    fn test_regex_validator_slug_pattern() {
        let validator = RegexValidator::new(r"^[-a-zA-Z0-9_]+$").unwrap();
        assert!(validator.validate("valid-slug").is_ok());
        assert!(validator.validate("valid_slug_123").is_ok());
        assert!(validator.validate("invalid slug").is_err());
        assert!(validator.validate("invalid@slug").is_err());
    }

    #[test]
    fn test_regex_validator_empty_string() {
        let validator = RegexValidator::new(r"^.*$").unwrap();
        assert!(validator.validate("").is_ok());
        assert!(validator.validate("anything").is_ok());
    }

    #[test]
    fn test_regex_validator_special_characters() {
        // Test escaping special regex characters
        let validator = RegexValidator::new(r"^\d+\.\d+$").unwrap();
        assert!(validator.validate("1.5").is_ok());
        assert!(validator.validate("123.456").is_ok());
        assert!(validator.validate("1a5").is_err());
    }

    /// // Test both String and str implementations
    #[test]
    fn test_validators_work_with_string_types() {
        let min_validator = MinLengthValidator::new(3);
        let max_validator = MaxLengthValidator::new(10);

        // Test with &str
        assert!(min_validator.validate("test").is_ok());
        assert!(max_validator.validate("test").is_ok());

        // Test with String
        let s = String::from("test");
        assert!(min_validator.validate(&s).is_ok());
        assert!(max_validator.validate(&s).is_ok());
    }

    /// // Based on Django test_max_length_validator_message
    #[test]
    fn test_min_length_error_contains_correct_values() {
        let validator = MinLengthValidator::new(16);
        match validator.validate("short") {
            Err(ValidationError::TooShort { length, min }) => {
                assert_eq!(length, 5);
                assert_eq!(min, 16);
            }
            _ => panic!("Expected TooShort error with correct values"),
        }
    }

    #[test]
    fn test_max_length_error_contains_correct_values() {
        let validator = MaxLengthValidator::new(5);
        match validator.validate("toolong") {
            Err(ValidationError::TooLong { length, max }) => {
                assert_eq!(length, 7);
                assert_eq!(max, 5);
            }
            _ => panic!("Expected TooLong error with correct values"),
        }
    }
}
