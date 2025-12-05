use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

#[model(app_label = "test", table_name = "users")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field(max_length = 100, collate = "utf8mb4_unicode_ci")]
	name: String,
}

fn main() {}
