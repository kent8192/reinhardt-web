//! Model without primary key should fail to compile

use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[model(app_label = "test", table_name = "users")]
struct User {
	#[field]
	id: i32,

	#[field]
	username: String,
}

fn main() {}
