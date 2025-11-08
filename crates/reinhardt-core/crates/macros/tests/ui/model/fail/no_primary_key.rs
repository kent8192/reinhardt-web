//! Model without primary key should fail to compile

use reinhardt_macros::Model;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Model)]
#[model(app_label = "test", table_name = "users")]
struct User {
	#[field]
	id: i32,

	#[field]
	username: String,
}

fn main() {}
