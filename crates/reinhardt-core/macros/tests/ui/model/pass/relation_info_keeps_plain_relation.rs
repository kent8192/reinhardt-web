use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

include!("../support.rs");

#[model(table_name = "categories")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Category {
	#[field(primary_key = true)]
	id: i64,
	#[field(max_length = 120)]
	name: String,
}

#[model(table_name = "articles")]
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
	let info = ArticleInfo {
		id: 1,
		title: "visible relation".to_string(),
		category: model_info::RelationInfo::new(42),
	};
	let serialized = serde_json::to_string(&info).unwrap();
	assert_eq!(
		serialized,
		r#"{"id":1,"title":"visible relation","category":{"id":42}}"#
	);

	let decoded: ArticleInfo = serde_json::from_str(
		r#"{"id":1,"title":"visible relation","category":{"id":99}}"#,
	)
	.unwrap();
	let model: Article = decoded.into();
	assert_eq!(model.category_id, 99);
}
