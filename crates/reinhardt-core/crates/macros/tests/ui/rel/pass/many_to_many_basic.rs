//! Test basic many_to_many relationship attribute

use reinhardt::model;
use reinhardt::db::associations::ManyToManyField;

#[model(app_label = "test")]
pub struct Tag {
	#[field(primary_key = true)]
	pub id: i64,
	pub name: String,
}

#[model(app_label = "test")]
pub struct Article {
	#[field(primary_key = true)]
	pub id: i64,
	pub title: String,
	#[rel(many_to_many, to = Tag, related_name = "articles")]
	pub tags: ManyToManyField<Tag>,
}

fn main() {}
