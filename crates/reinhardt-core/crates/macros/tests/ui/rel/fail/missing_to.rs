//! Test error when 'to' parameter is missing for foreign_key

use reinhardt::model;

#[model(app_label = "test")]
pub struct Post {
	#[field(primary_key = true)]
	pub id: i64,
	pub title: String,
	#[rel(foreign_key)]
	pub author_id: i64,
}

fn main() {}
