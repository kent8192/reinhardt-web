use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

include!("../support.rs");

#[model(app_label = "default", table_name = "categories")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Category {
	#[field(primary_key = true)]
	id: i64,
	#[field(max_length = 120)]
	name: String,
}

#[model(app_label = "default", table_name = "articles")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Article {
	#[field(primary_key = true)]
	id: i64,
	#[field(max_length = 120)]
	title: String,
	#[rel(foreign_key)]
	category: db::associations::ForeignKeyField<Category>,
}

fn main() {
	let _info = ArticleInfo {
		id: 1,
		title: "visible relation".to_string(),
		category: model_info::RelationInfo::new(42),
	};
}
