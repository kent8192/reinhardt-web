//! Basic admin macro usage with required attributes

use reinhardt::admin::panel::ModelAdmin;
use reinhardt_macros::{admin, model};
use serde::{Deserialize, Serialize};

#[model(app_label = "test", table_name = "users")]
#[derive(Serialize, Deserialize)]
struct User {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field(max_length = 100)]
	username: String,

	#[field(max_length = 255, null = true)]
	email: Option<String>,
}

#[admin(model, for = User, name = "User")]
pub struct UserAdmin;

fn main() {
	// Compile test only - verify the macro expands without errors
	let admin = UserAdmin;
	assert_eq!(admin.model_name(), "User");
}
