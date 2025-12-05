//! Basic model with required attributes

use reinhardt_macros::{model, Model};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[model(app_label = "test", table_name = "users")]
struct User {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field(max_length = 100)]
	username: String,

	#[field(max_length = 255, null = true)]
	email: Option<String>,
}

fn main() {
	// Compile test only - just verify the macro expands without errors
	let _user = User {
		id: Some(1),
		username: "test".to_string(),
		email: None,
	};
}
