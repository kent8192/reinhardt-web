//! Validator re-exports available to shared native/WASM DTOs.

#[cfg(feature = "core")]
pub use reinhardt_core::validators::{
	CreditCardValidator, EmailValidator, IBANValidator, IPAddressValidator, PhoneNumberValidator,
	UrlValidator, Validate, ValidationError as ValidatorError, ValidationErrors, ValidationResult,
	Validator,
};
