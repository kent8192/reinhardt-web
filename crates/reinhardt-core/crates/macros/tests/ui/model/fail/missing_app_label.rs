//! Model without app_label should fail to compile

use reinhardt_macros::{model, Model};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[model(table_name = "users")]
struct User {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field]
	username: String,
}

fn main() {}
