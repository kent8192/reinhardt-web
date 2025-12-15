//! Signin serializers
//!
//! Serializers for user signin endpoints

use reinhardt::rest::{Schema, ToSchema};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request data for user signin
#[derive(Debug, Serialize, Deserialize, Validate, Schema)]
pub struct SigninRequest {
	/// User's email address
	#[validate(email(message = "Invalid email format"))]
	#[validate(length(min = 1, message = "Email cannot be empty"))]
	pub email: String,

	/// Password
	#[validate(length(min = 1, message = "Password cannot be empty"))]
	pub password: String,
}

/// Response data for successful signin
#[derive(Debug, Serialize, Deserialize, Schema)]
pub struct SigninResponse {
	/// JWT token for API authentication
	pub token: String,
	/// User information
	pub user: SigninUserInfo,
}

/// User information returned in signin response
#[derive(Debug, Serialize, Deserialize, Schema)]
pub struct SigninUserInfo {
	/// User's unique identifier
	pub id: String,
	/// Username
	pub username: String,
	/// Email address
	pub email: String,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_valid_signin_request() {
		let request = SigninRequest {
			email: "test@example.com".to_string(),
			password: "password123".to_string(),
		};

		assert!(request.validate().is_ok());
	}

	#[test]
	fn test_invalid_email_format() {
		let request = SigninRequest {
			email: "notanemail".to_string(),
			password: "password123".to_string(),
		};

		assert!(request.validate().is_err());
	}

	#[test]
	fn test_empty_email() {
		let request = SigninRequest {
			email: "".to_string(),
			password: "password123".to_string(),
		};

		assert!(request.validate().is_err());
	}

	#[test]
	fn test_empty_password() {
		let request = SigninRequest {
			email: "test@example.com".to_string(),
			password: "".to_string(),
		};

		assert!(request.validate().is_err());
	}
}
