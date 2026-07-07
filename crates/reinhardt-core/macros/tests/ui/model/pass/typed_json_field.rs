use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

include!("../support.rs");

use db::orm::Model;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct StyleSettings {
	indent_width: u8,
	theme: String,
}

#[model(table_name = "writing_projects")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WritingProject {
	#[field(primary_key = true)]
	id: i64,
	#[field]
	style_settings: db::Json<StyleSettings>,
	#[field(null = true)]
	raw_payload: Option<db::Json<serde_json::Value>>,
}

fn main() {
	let project_info = WritingProjectInfo {
		id: 1,
		style_settings: db::Json::new(StyleSettings {
			indent_width: 2,
			theme: "paper".to_string(),
		}),
		raw_payload: Some(db::Json::new(serde_json::json!({ "language": "ja" }))),
	};

	assert_eq!(project_info.style_settings.indent_width, 2);
	assert_eq!(project_info.raw_payload.unwrap()["language"], "ja");

	let fields = WritingProject::field_metadata();
	let style_field = fields
		.iter()
		.find(|field| field.name == "style_settings")
		.unwrap();
	assert_eq!(style_field.field_type, "reinhardt.orm.models.JsonField");
	assert!(!style_field.nullable);

	let raw_payload_field = fields
		.iter()
		.find(|field| field.name == "raw_payload")
		.unwrap();
	assert_eq!(
		raw_payload_field.field_type,
		"reinhardt.orm.models.JsonField"
	);
	assert!(raw_payload_field.nullable);
}
