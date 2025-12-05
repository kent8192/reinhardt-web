//! Test basic one_to_one relationship attribute

use reinhardt::model;

#[model(app_label = "test")]
pub struct User {
	#[field(primary_key = true)]
	pub id: i64,
	pub name: String,
}

#[model(app_label = "test")]
pub struct Profile {
	#[field(primary_key = true)]
	pub id: i64,
	pub bio: String,
	#[rel(one_to_one, to = User, related_name = "profile")]
	pub user_id: i64,
}

fn main() {}
