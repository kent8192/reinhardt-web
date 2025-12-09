//! Profile test helpers
//!
//! Common utilities for profile endpoint tests

use reinhardt::core::serde::json::{json, Value};

/// Create a valid create profile request body
pub fn valid_create_profile_request() -> Value {
	json!({
		"bio": "Test bio for profile",
		"avatar_url": "https://example.com/avatar.jpg",
		"location": "Tokyo, Japan",
		"website": "https://example.com"
	})
}

/// Create a minimal create profile request body (only required fields)
pub fn minimal_create_profile_request() -> Value {
	json!({})
}

/// Create a valid update profile request body
pub fn valid_update_profile_request() -> Value {
	json!({
		"bio": "Updated bio",
		"location": "Osaka, Japan"
	})
}

/// Create a partial update profile request body
pub fn partial_update_profile_request() -> Value {
	json!({
		"bio": "Only bio updated"
	})
}
