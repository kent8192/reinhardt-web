use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

include!("../support.rs");

use db::orm::Model;

#[model(app_label = "external", table_name = "external_tags")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Tag {
	#[field(primary_key = true)]
	id: i64,
}

#[model(app_label = "source", table_name = "source_posts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Post {
	#[field(primary_key = true)]
	id: i64,
	#[rel(many_to_many)]
	tags: db::associations::ManyToManyField<Post, Tag>,
}

fn main() {
	let _ = Post::field_metadata();
}
