//! Verify password serializers
//!
//! Serializers for password verification endpoints

use reinhardt::rest::{Schema, ToSchema};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request data for password verification
#[derive(Debug, Serialize, Deserialize, Validate, Schema)]
pub struct VerifyPasswordRequest {
	/// Password to verify
	#[validate(length(min = 1, message = "Password cannot be empty"))]
	pub password: String,
}

/// Response data for password verification
#[derive(Debug, Serialize, Deserialize, Schema)]
pub struct VerifyPasswordResponse {
	/// Whether the password is valid
	pub valid: bool,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_valid_verify_password_request() {
		let request = VerifyPasswordRequest {
			password: "password123".to_string(),
		};

		assert!(request.validate().is_ok());
	}

	#[test]
	fn test_empty_password() {
		let request = VerifyPasswordRequest {
			password: "".to_string(),
		};

		assert!(request.validate().is_err());
	}
}
