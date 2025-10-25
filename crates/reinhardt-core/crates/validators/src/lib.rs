//! Advanced validators for Reinhardt
//!
//! This crate provides Django-style validators for common validation needs,
//! as well as compile-time validated database identifier types.

pub mod email;
pub mod errors;
pub mod identifier;
pub mod numeric;
pub mod reserved;
pub mod string;
pub mod uniqueness;
pub mod url;

pub use email::EmailValidator;
pub use errors::{ValidationError, ValidationResult};
pub use identifier::{ConstraintName, FieldName, IdentifierValidationError, TableName};
pub use numeric::{MaxValueValidator, MinValueValidator, RangeValidator};
pub use string::{
    DateTimeValidator, DateValidator, IPAddressValidator, JSONValidator, MaxLengthValidator,
    MinLengthValidator, RegexValidator, SlugValidator, TimeValidator, UUIDValidator,
};
pub use uniqueness::UniqueValidator;
pub use url::UrlValidator;

/// Re-export commonly used types
pub mod prelude {
    pub use crate::email::*;
    pub use crate::errors::*;
    pub use crate::identifier::*;
    pub use crate::numeric::*;
    pub use crate::string::*;
    pub use crate::uniqueness::*;
    pub use crate::url::*;
}

/// Trait for validators
pub trait Validator<T: ?Sized> {
    fn validate(&self, value: &T) -> ValidationResult<()>;
}

/// Extension trait for ORM validators with custom error messages
pub trait OrmValidator: Validator<str> {
    /// Get the error message for this validator
    fn message(&self) -> String;
}

/// Extension trait for settings validators
pub trait SettingsValidator: Send + Sync {
    /// Validate a specific key-value pair
    fn validate_setting(&self, key: &str, value: &serde_json::Value) -> ValidationResult<()>;

    /// Get validator description
    fn description(&self) -> String;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests for validator trait - based on Django validators/tests.py
    #[test]
    fn test_min_length_validator_trait() {
        let validator = MinLengthValidator::new(5);
        assert!(validator.validate("hello").is_ok());
        assert!(validator.validate("hi").is_err());
    }

    #[test]
    fn test_max_length_validator_trait() {
        let validator = MaxLengthValidator::new(10);
        assert!(validator.validate("hello").is_ok());
        assert!(validator.validate("hello world!").is_err());
    }

    #[test]
    fn test_min_value_validator_trait() {
        let validator = MinValueValidator::new(10);
        assert!(validator.validate(&15).is_ok());
        assert!(validator.validate(&5).is_err());
    }

    #[test]
    fn test_max_value_validator_trait() {
        let validator = MaxValueValidator::new(100);
        assert!(validator.validate(&50).is_ok());
        assert!(validator.validate(&150).is_err());
    }

    #[test]
    fn test_range_validator_trait() {
        let validator = RangeValidator::new(10, 20);
        assert!(validator.validate(&15).is_ok());
        assert!(validator.validate(&5).is_err());
        assert!(validator.validate(&25).is_err());
    }

    #[test]
    fn test_email_validator_trait() {
        let validator = EmailValidator::new();
        assert!(validator.validate("test@example.com").is_ok());
        assert!(validator.validate("invalid").is_err());
    }

    #[test]
    fn test_url_validator_trait() {
        let validator = UrlValidator::new();
        assert!(validator.validate("http://example.com").is_ok());
        assert!(validator.validate("invalid").is_err());
    }

    #[test]
    fn test_regex_validator_trait() {
        let validator = RegexValidator::new(r"^\d+$").unwrap();
        assert!(validator.validate("12345").is_ok());
        assert!(validator.validate("abc").is_err());
    }

    // Test combining validators
    #[test]
    fn test_multiple_validators() {
        let min_validator = MinLengthValidator::new(3);
        let max_validator = MaxLengthValidator::new(10);

        let value = "test";
        assert!(min_validator.validate(value).is_ok());
        assert!(max_validator.validate(value).is_ok());

        let too_short = "hi";
        assert!(min_validator.validate(too_short).is_err());
        assert!(max_validator.validate(too_short).is_ok());

        let too_long = "this is way too long";
        assert!(min_validator.validate(too_long).is_ok());
        assert!(max_validator.validate(too_long).is_err());
    }

    // Test prelude exports
    #[test]
    fn test_prelude_exports() {
        use crate::prelude::*;

        let min = MinLengthValidator::new(1);
        let max = MaxLengthValidator::new(10);
        let email = EmailValidator::new();
        let url = UrlValidator::new();
        let min_val = MinValueValidator::new(0);
        let max_val = MaxValueValidator::new(100);
        let range = RangeValidator::new(0, 100);

        // Just verify they compile and are usable
        assert!(min.validate("x").is_ok());
        assert!(max.validate("x").is_ok());
        assert!(email.validate("test@example.com").is_ok());
        assert!(url.validate("http://example.com").is_ok());
        assert!(min_val.validate(&50).is_ok());
        assert!(max_val.validate(&50).is_ok());
        assert!(range.validate(&50).is_ok());
    }
}
