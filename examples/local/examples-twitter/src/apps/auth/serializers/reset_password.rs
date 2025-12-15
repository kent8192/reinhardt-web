//! Reset password serializers
//!
//! Serializers for password reset endpoints

use reinhardt::rest::{Schema, ToSchema};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request data for password reset
#[derive(Debug, Serialize, Deserialize, Validate, Schema)]
pub struct ResetPasswordRequest {
	/// User's email address
	#[validate(email(message = "Invalid email format"))]
	#[validate(length(min = 1, message = "Email cannot be empty"))]
	pub email: String,
}

/// Response data for password reset request
///
/// Note: In production, the reset_token should NOT be returned in the response.
/// Instead, it should be sent via email. This is for development/testing purposes only.
#[derive(Debug, Serialize, Deserialize, Schema)]
pub struct ResetPasswordResponse {
	/// Password reset token (for development only - in production, send via email)
	pub reset_token: String,
}

/// Request data for confirming password reset with new password
#[derive(Debug, Serialize, Deserialize, Validate, Schema)]
pub struct ResetPasswordConfirmRequest {
	/// Password reset token received via email
	#[validate(length(min = 1, message = "Token cannot be empty"))]
	pub token: String,

	/// New password to set
	#[validate(length(min = 8, message = "Password must be at least 8 characters"))]
	pub new_password: String,

	/// Password confirmation (must match new_password)
	#[validate(length(
		min = 8,
		message = "Password confirmation must be at least 8 characters"
	))]
	pub new_password_confirmation: String,
}

impl ResetPasswordConfirmRequest {
	/// Validates that new_password and new_password_confirmation match
	pub fn passwords_match(&self) -> bool {
		self.new_password == self.new_password_confirmation
	}
}

/// Response data for password reset confirmation
#[derive(Debug, Serialize, Deserialize, Schema)]
pub struct ResetPasswordConfirmResponse {
	/// Success message
	pub message: String,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_valid_reset_password_request() {
		let request = ResetPasswordRequest {
			email: "test@example.com".to_string(),
		};

		assert!(request.validate().is_ok());
	}

	#[test]
	fn test_invalid_email_format() {
		let request = ResetPasswordRequest {
			email: "notanemail".to_string(),
		};

		assert!(request.validate().is_err());
	}

	#[test]
	fn test_empty_email() {
		let request = ResetPasswordRequest {
			email: "".to_string(),
		};

		assert!(request.validate().is_err());
	}

	#[test]
	fn test_valid_reset_password_confirm_request() {
		let request = ResetPasswordConfirmRequest {
			token: "abc123-token".to_string(),
			new_password: "newpassword123".to_string(),
			new_password_confirmation: "newpassword123".to_string(),
		};

		assert!(request.validate().is_ok());
		assert!(request.passwords_match());
	}

	#[test]
	fn test_password_mismatch() {
		let request = ResetPasswordConfirmRequest {
			token: "abc123-token".to_string(),
			new_password: "newpassword123".to_string(),
			new_password_confirmation: "differentpassword".to_string(),
		};

		assert!(request.validate().is_ok()); // Validator passes (length ok)
		assert!(!request.passwords_match()); // But passwords don't match
	}

	#[test]
	fn test_short_password() {
		let request = ResetPasswordConfirmRequest {
			token: "abc123-token".to_string(),
			new_password: "short".to_string(),
			new_password_confirmation: "short".to_string(),
		};

		assert!(request.validate().is_err());
	}

	#[test]
	fn test_empty_token() {
		let request = ResetPasswordConfirmRequest {
			token: "".to_string(),
			new_password: "newpassword123".to_string(),
			new_password_confirmation: "newpassword123".to_string(),
		};

		assert!(request.validate().is_err());
	}
}
