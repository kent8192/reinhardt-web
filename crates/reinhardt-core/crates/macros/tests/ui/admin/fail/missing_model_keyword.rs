//! Missing `model` keyword should fail

use reinhardt_macros::{admin, model};
use serde::{Deserialize, Serialize};

#[model(app_label = "test", table_name = "users")]
#[derive(Serialize, Deserialize)]
struct User {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field(max_length = 100)]
	username: String,
}

#[admin(for = User, name = "User")]
pub struct UserAdmin;

fn main() {}
