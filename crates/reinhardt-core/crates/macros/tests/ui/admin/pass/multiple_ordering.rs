//! Admin macro with multiple ordering specifications

use reinhardt_admin_core::ModelAdmin;
use reinhardt_macros::{admin, model};
use serde::{Deserialize, Serialize};

#[model(app_label = "test", table_name = "articles")]
#[derive(Serialize, Deserialize)]
struct Article {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field(max_length = 200)]
	title: String,

	#[field(max_length = 100)]
	category: String,

	created_at: i64,
}

#[admin(model,
	for = Article,
	name = "Article",
	ordering = [(category, asc), (created_at, desc)]
)]
pub struct ArticleAdmin;

fn main() {
	// Compile test only - verify multiple ordering specs work
	let admin = ArticleAdmin;
	assert_eq!(admin.model_name(), "Article");
	assert_eq!(admin.ordering(), vec!["category", "-created_at"]);
}
