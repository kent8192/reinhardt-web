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
	let style_settings: db::Json<StyleSettings> = db::Json::new(StyleSettings {
		indent_width: 2,
		theme: "paper".to_string(),
	});
	let raw_payload: Option<db::Json<serde_json::Value>> =
		Some(db::Json::new(serde_json::json!({ "language": "ja" })));

	let _project_info: WritingProjectInfo = WritingProjectInfo {
		id: 1,
		style_settings,
		raw_payload,
	};

	let _field_metadata: Vec<db::orm::inspection::FieldInfo> = WritingProject::field_metadata();
	let _field_metadata_fn: fn() -> Vec<db::orm::inspection::FieldInfo> =
		WritingProject::field_metadata;
}
