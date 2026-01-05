//! Auth test helpers
//!
//! Common utilities for auth endpoint tests

use reinhardt::core::serde::json::{Value, json};

/// Create a valid registration request body
pub fn valid_register_request() -> Value {
	json!({
		"email": "newuser@example.com",
		"username": "newuser",
		"password": "password123",
		"password_confirmation": "password123"
	})
}

/// Create a valid signin request body
pub fn valid_signin_request(email: &str, password: &str) -> Value {
	json!({
		"email": email,
		"password": password
	})
}

/// Create a valid verify password request body
pub fn valid_verify_password_request(password: &str) -> Value {
	json!({
		"password": password
	})
}

/// Create a valid change password request body
pub fn valid_change_password_request(
	current_password: &str,
	new_password: &str,
	new_password_confirmation: &str,
) -> Value {
	json!({
		"current_password": current_password,
		"new_password": new_password,
		"new_password_confirmation": new_password_confirmation
	})
}

/// Create a valid reset password request body
pub fn valid_reset_password_request(email: &str) -> Value {
	json!({
		"email": email
	})
}
