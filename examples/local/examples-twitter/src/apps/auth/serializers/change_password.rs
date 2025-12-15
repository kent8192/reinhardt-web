//! Change password serializers
//!
//! Serializers for password change endpoints

use reinhardt::rest::{Schema, ToSchema};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request data for password change
#[derive(Debug, Serialize, Deserialize, Validate, Schema)]
pub struct ChangePasswordRequest {
	/// Current password
	#[validate(length(min = 1, message = "Current password cannot be empty"))]
	pub current_password: String,

	/// New password (minimum 8 characters)
	#[validate(length(min = 8, message = "New password must be at least 8 characters"))]
	#[validate(custom(
		function = "validate_no_whitespace",
		message = "New password cannot contain only whitespace"
	))]
	pub new_password: String,

	/// New password confirmation (must match new_password)
	#[validate(length(min = 1, message = "Password confirmation cannot be empty"))]
	pub new_password_confirmation: String,
}

impl ChangePasswordRequest {
	/// Validate that new passwords match
	pub fn validate_passwords_match(&self) -> Result<(), &'static str> {
		if self.new_password != self.new_password_confirmation {
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

/// Response data for successful password change
#[derive(Debug, Serialize, Deserialize, Schema)]
pub struct ChangePasswordResponse {
	/// Success message
	pub message: String,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_valid_change_password_request() {
		let request = ChangePasswordRequest {
			current_password: "oldpassword".to_string(),
			new_password: "newpassword123".to_string(),
			new_password_confirmation: "newpassword123".to_string(),
		};

		assert!(request.validate().is_ok());
		assert!(request.validate_passwords_match().is_ok());
	}

	#[test]
	fn test_passwords_do_not_match() {
		let request = ChangePasswordRequest {
			current_password: "oldpassword".to_string(),
			new_password: "newpassword123".to_string(),
			new_password_confirmation: "different".to_string(),
		};

		assert!(request.validate_passwords_match().is_err());
	}

	#[test]
	fn test_new_password_too_short() {
		let request = ChangePasswordRequest {
			current_password: "oldpassword".to_string(),
			new_password: "short".to_string(),
			new_password_confirmation: "short".to_string(),
		};

		assert!(request.validate().is_err());
	}

	#[test]
	fn test_empty_current_password() {
		let request = ChangePasswordRequest {
			current_password: "".to_string(),
			new_password: "newpassword123".to_string(),
			new_password_confirmation: "newpassword123".to_string(),
		};

		assert!(request.validate().is_err());
	}

	#[test]
	fn test_whitespace_only_new_password() {
		let request = ChangePasswordRequest {
			current_password: "oldpassword".to_string(),
			new_password: "        ".to_string(),
			new_password_confirmation: "        ".to_string(),
		};

		assert!(request.validate().is_err());
	}
}
