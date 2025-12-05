//! Test error for unknown rel attribute

use reinhardt::model;

#[model(app_label = "test")]
pub struct User {
	#[field(primary_key = true)]
	pub id: i64,
}

#[model(app_label = "test")]
pub struct Post {
	#[field(primary_key = true)]
	pub id: i64,
	#[rel(foreign_key, to = User, unknown_option = true)]
	pub author_id: i64,
}

fn main() {}
