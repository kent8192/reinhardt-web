//! DM test helpers
//!
//! Common utilities for DM endpoint tests

use reinhardt::core::serde::json::{json, Value};
use uuid::Uuid;

/// Create a valid create room request body
pub fn valid_create_room_request(member_ids: Vec<Uuid>) -> Value {
	json!({
		"member_ids": member_ids
	})
}

/// Create a create room request with a name (for group chat)
pub fn create_group_room_request(name: &str, member_ids: Vec<Uuid>) -> Value {
	json!({
		"name": name,
		"member_ids": member_ids
	})
}

/// Create a valid create message request body
pub fn valid_create_message_request(content: &str) -> Value {
	json!({
		"content": content
	})
}
