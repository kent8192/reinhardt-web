//! Test basic many_to_many relationship attribute
//!
//! ManyToManyField uses two type parameters:
//! - First: Source model (the model containing this field)
//! - Second: Target model (the related model)
//!
//! The intermediate table is automatically generated based on Source and Target types.

use reinhardt::db::associations::ManyToManyField;
use reinhardt::model;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[model(app_label = "test", table_name = "tags")]
pub struct Tag {
	#[field(primary_key = true)]
	pub id: i64,
	#[field(max_length = 50)]
	pub name: String,
}

#[derive(Serialize, Deserialize)]
#[model(app_label = "test", table_name = "articles")]
pub struct Article {
	#[field(primary_key = true)]
	pub id: i64,
	#[field(max_length = 200)]
	pub title: String,

	// Article -> Tag relationship
	// Intermediate table: test_articles_tags
	#[rel(many_to_many, related_name = "articles")]
	pub tags: ManyToManyField<Article, Tag>,
}

fn main() {}
