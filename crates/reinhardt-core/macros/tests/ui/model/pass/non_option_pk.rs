//! Model with non-Option primary key

use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

#[model(app_label = "test", table_name = "posts")]
#[derive(Serialize, Deserialize)]
struct Post {
	#[field(primary_key = true)]
	id: i64,

	#[field(max_length = 200)]
	title: String,

	#[field(max_length = 10000)]
	content: String,
}

fn main() {
	// Compile test only - just verify the macro expands without errors
	let _post = Post {
		id: 42,
		title: "Test Post".to_string(),
		content: "Content here".to_string(),
	};
}
