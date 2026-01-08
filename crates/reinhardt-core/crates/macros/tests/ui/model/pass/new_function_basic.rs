//! Test that `#[model]` generates new() function with correct parameters

use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

/// Basic model with `Option<i32>` primary key and optional field with default
#[model(app_label = "test", table_name = "test_user")]
#[derive(Serialize, Deserialize)]
pub struct TestUser {
	#[field(primary_key = true)]
	pub id: Option<i32>,

	#[field(max_length = 150)]
	pub username: String,

	#[field(max_length = 255)]
	pub email: String,

	#[field(default = true)]
	pub is_active: bool,
}

fn main() {
	// new() should require username, email, is_active
	// id is Option<i32> primary key (auto-excluded from new())
	let user = TestUser::new("alice", "alice@example.com", true);

	// Verify user fields are set correctly
	assert_eq!(user.username, "alice");
	assert_eq!(user.email, "alice@example.com");
	assert!(user.is_active);
	assert!(user.id.is_none());
}
