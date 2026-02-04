use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[model(app_label = "test", table_name = "users")]
struct User {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field(max_length = 100)]
	first_name: String,
	#[field(max_length = 100)]
	last_name: String,

	#[field(
		max_length = 201,
		generated = "first_name || ' ' || last_name",
		generated_stored = true
	)]
	full_name: String,
}

fn main() {}
