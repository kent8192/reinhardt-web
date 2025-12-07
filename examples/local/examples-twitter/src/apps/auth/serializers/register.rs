//! Register serializers
//!
//! Serializers for user registration endpoints

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request data for user registration
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct RegisterRequest {
	/// User's email address
	#[validate(email(message = "Invalid email format"))]
	#[validate(length(min = 1, message = "Email cannot be empty"))]
	#[validate(custom(function = "validate_no_whitespace", message = "Email cannot contain only whitespace"))]
	pub email: String,

	/// Username
	#[validate(length(min = 1, max = 150, message = "Username must be 1-150 characters"))]
	#[validate(custom(function = "validate_no_whitespace", message = "Username cannot contain only whitespace"))]
	pub username: String,

	/// Password (minimum 8 characters)
	#[validate(length(min = 8, message = "Password must be at least 8 characters"))]
	#[validate(custom(function = "validate_no_whitespace", message = "Password cannot contain only whitespace"))]
	pub password: String,

	/// Password confirmation (must match password)
	#[validate(length(min = 1, message = "Password confirmation cannot be empty"))]
	pub password_confirmation: String,
}

impl RegisterRequest {
	/// Validate that passwords match
	pub fn validate_passwords_match(&self) -> Result<(), &'static str> {
		if self.password != self.password_confirmation {
			return Err("Passwords do not match");
		}
		Ok(())
	}
}

/// Validate that a string is not only whitespace
fn validate_no_whitespace(s: &str) -> Result<(), validator::ValidationError> {
	if s.trim().is_empty() {
		return Err(validator::ValidationError::new("whitespace_only"));
	}
	Ok(())
}

/// Response data for successful registration
#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterResponse {
	/// Success message
	pub message: String,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_valid_register_request() {
		let request = RegisterRequest {
			email: "test@example.com".to_string(),
			username: "testuser".to_string(),
			password: "password123".to_string(),
			password_confirmation: "password123".to_string(),
		};

		assert!(request.validate().is_ok());
		assert!(request.validate_passwords_match().is_ok());
	}

	#[test]
	fn test_invalid_email() {
		let request = RegisterRequest {
			email: "notanemail".to_string(),
			username: "testuser".to_string(),
			password: "password123".to_string(),
			password_confirmation: "password123".to_string(),
		};

		assert!(request.validate().is_err());
	}

	#[test]
	fn test_password_too_short() {
		let request = RegisterRequest {
			email: "test@example.com".to_string(),
			username: "testuser".to_string(),
			password: "pass".to_string(),
			password_confirmation: "pass".to_string(),
		};

		assert!(request.validate().is_err());
	}

	#[test]
	fn test_passwords_do_not_match() {
		let request = RegisterRequest {
			email: "test@example.com".to_string(),
			username: "testuser".to_string(),
			password: "password123".to_string(),
			password_confirmation: "different".to_string(),
		};

		assert!(request.validate_passwords_match().is_err());
	}

	#[test]
	fn test_whitespace_only_email() {
		let request = RegisterRequest {
			email: "   ".to_string(),
			username: "testuser".to_string(),
			password: "password123".to_string(),
			password_confirmation: "password123".to_string(),
		};

		assert!(request.validate().is_err());
	}

	#[test]
	fn test_whitespace_only_username() {
		let request = RegisterRequest {
			email: "test@example.com".to_string(),
			username: "   ".to_string(),
			password: "password123".to_string(),
			password_confirmation: "password123".to_string(),
		};

		assert!(request.validate().is_err());
	}

	#[test]
	fn test_whitespace_only_password() {
		let request = RegisterRequest {
			email: "test@example.com".to_string(),
			username: "testuser".to_string(),
			password: "   ".to_string(),
			password_confirmation: "   ".to_string(),
		};

		assert!(request.validate().is_err());
	}
}
