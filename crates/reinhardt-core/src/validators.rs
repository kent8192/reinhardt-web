//! Advanced validators for Reinhardt
//!
//! This crate provides Django-style validators for common validation needs,
//! as well as compile-time validated database identifier types.
//!
//! ## Planned Features
//!
//! ### Medium Priority
//!
//! 1. **Custom Error Messages (Extended)**: Extend `.with_message()` to all validators
//!    - Consistent API across all validators
//!    - Custom message templates
//!    - Maintain default error messages as fallback
//!
//! ### Lower Priority
//!
//! 1. **Internationalization (i18n)**: Multi-language error messages
//!    - Fluent-based message system
//!    - Language-specific error messages
//!    - Locale fallback support
//!
//! 2. **Serialization Support**: Serialize/deserialize validators for storage
//!    - Serde integration with optional feature flag
//!    - Custom serializers for Regex patterns
//!    - Validator configuration persistence
//!
//! 3. **Schema Validation**: JSON Schema and other schema format support
//!    - JSON Schema validation
//!    - Integration with `jsonschema` crate
//!    - Custom schema formats
//!
//! 4. **Performance Optimizations**:
//!    - Lazy Regex Compilation: Compile regex patterns only when needed
//!    - Validator Caching: Cache compiled validators for reuse
//!    - Parallel Validation: Run independent validators concurrently with rayon

pub(crate) mod lazy_patterns;

pub mod color;
pub mod composition;
pub mod conditional;
pub mod credit_card;
pub mod custom_regex;
pub mod email;
pub mod errors;
pub mod existence;
pub mod file_type;
pub mod iban;
pub mod identifier;
pub mod image;
pub mod ip_address;
pub mod numeric;
pub mod phone_number;
pub mod postal_code;
pub mod reserved;
pub mod string;
pub mod uniqueness;
pub mod url;

#[cfg(feature = "serde")]
pub mod serialization;

#[cfg(feature = "jsonschema")]
pub mod schema;

#[cfg(feature = "parallel")]
pub mod parallel;

#[cfg(feature = "i18n")]
pub mod i18n;

pub use color::{ColorFormat, ColorValidator};
pub use composition::{AndValidator, OrValidator};
pub use conditional::ConditionalValidator;
pub use credit_card::{CardType, CreditCardValidator};
pub use custom_regex::CustomRegexValidator;
pub use email::EmailValidator;
pub use errors::{ValidationError, ValidationResult};
pub use existence::ExistsValidator;
pub use file_type::{FileSizeValidator, FileTypeValidator};
pub use iban::IBANValidator;
pub use identifier::{ConstraintName, FieldName, IdentifierValidationError, TableName};
pub use image::ImageDimensionValidator;
pub use ip_address::IPAddressValidator;
pub use numeric::{MaxValueValidator, MinValueValidator, RangeValidator};
pub use phone_number::PhoneNumberValidator;
pub use postal_code::{Country, PostalCodeValidator};
pub use string::{
	DateTimeValidator, DateValidator, JSONValidator, MaxLengthValidator, MinLengthValidator,
	RegexValidator, SlugValidator, TimeValidator, UUIDValidator,
};
pub use uniqueness::UniqueValidator;
pub use url::UrlValidator;

/// Re-export commonly used types
pub mod prelude {
	pub use super::color::*;
	pub use super::composition::*;
	pub use super::conditional::*;
	pub use super::credit_card::*;
	pub use super::custom_regex::*;
	pub use super::email::*;
	pub use super::errors::*;
	pub use super::existence::*;
	pub use super::file_type::*;
	pub use super::iban::*;
	pub use super::identifier::*;
	pub use super::image::*;
	pub use super::ip_address::*;
	pub use super::numeric::*;
	pub use super::phone_number::*;
	pub use super::postal_code::*;
	pub use super::string::*;
	pub use super::uniqueness::*;
	pub use super::url::*;
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
	use rstest::rstest;

	// Integration tests for validator trait - based on Django validators/tests.py
	#[rstest]
	fn test_min_length_validator_trait() {
		let validator = MinLengthValidator::new(5);
		assert!(validator.validate("hello").is_ok());
		assert!(validator.validate("hi").is_err());
	}

	#[rstest]
	fn test_max_length_validator_trait() {
		let validator = MaxLengthValidator::new(10);
		assert!(validator.validate("hello").is_ok());
		assert!(validator.validate("hello world!").is_err());
	}

	#[rstest]
	fn test_min_value_validator_trait() {
		let validator = MinValueValidator::new(10);
		assert!(validator.validate(&15).is_ok());
		assert!(validator.validate(&5).is_err());
	}

	#[rstest]
	fn test_max_value_validator_trait() {
		let validator = MaxValueValidator::new(100);
		assert!(validator.validate(&50).is_ok());
		assert!(validator.validate(&150).is_err());
	}

	#[rstest]
	fn test_range_validator_trait() {
		let validator = RangeValidator::new(10, 20);
		assert!(validator.validate(&15).is_ok());
		assert!(validator.validate(&5).is_err());
		assert!(validator.validate(&25).is_err());
	}

	#[rstest]
	fn test_email_validator_trait() {
		let validator = EmailValidator::new();
		assert!(validator.validate("test@example.com").is_ok());
		assert!(validator.validate("invalid").is_err());
	}

	#[rstest]
	fn test_url_validator_trait() {
		let validator = UrlValidator::new();
		assert!(validator.validate("http://example.com").is_ok());
		assert!(validator.validate("invalid").is_err());
	}

	#[rstest]
	fn test_regex_validator_trait() {
		let validator = RegexValidator::new(r"^\d+$").unwrap();
		assert!(validator.validate("12345").is_ok());
		assert!(validator.validate("abc").is_err());
	}

	// Test combining validators
	#[rstest]
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
	#[rstest]
	fn test_prelude_exports() {
		use crate::validators::*;

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
