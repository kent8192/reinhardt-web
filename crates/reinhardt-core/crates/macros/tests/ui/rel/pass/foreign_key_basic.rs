//! Test basic foreign_key relationship attribute

use reinhardt::model;

#[model(app_label = "test")]
pub struct User {
	#[field(primary_key = true)]
	pub id: i64,
	pub name: String,
}

#[model(app_label = "test")]
pub struct Post {
	#[field(primary_key = true)]
	pub id: i64,
	pub title: String,
	#[rel(foreign_key, to = User)]
	pub author_id: i64,
}

fn main() {}
