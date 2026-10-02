// The model macro emits `cfg(native)` arms; this test only validates generated serde behavior.
#![allow(unexpected_cfgs)]

use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

include!("ui/model/support.rs");

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

#[model(app_label = "default", table_name = "tenants")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Tenant {
	#[field(primary_key = true)]
	id: i64,
	#[field(max_length = 120)]
	name: String,
}

fn default_tenant() -> db::associations::ForeignKeyField<Tenant> {
	db::associations::ForeignKeyField::default()
}

#[model(app_label = "default", table_name = "documents")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Document {
	#[field(primary_key = true)]
	id: i64,
	#[field(max_length = 120)]
	title: String,
	#[serde(skip_serializing, skip_deserializing, default = "default_tenant")]
	#[rel(foreign_key)]
	tenant: db::associations::ForeignKeyField<Tenant>,
}

#[test]
fn relation_info_keeps_plain_relation_round_trip() {
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

	let decoded: ArticleInfo =
		serde_json::from_str(r#"{"id":1,"title":"visible relation","category":{"id":99}}"#)
			.unwrap();
	let model: Article = decoded.into();
	assert_eq!(model.category_id, 99);
}

#[test]
fn relation_info_preserves_explicit_serde_round_trip() {
	let info = DocumentInfo {
		id: 1,
		title: "private".to_string(),
		tenant: model_info::RelationInfo::new(42),
	};

	let serialized = serde_json::to_string(&info).unwrap();
	assert_eq!(serialized, r#"{"id":1,"title":"private"}"#);

	let decoded: DocumentInfo =
		serde_json::from_str(r#"{"id":1,"title":"private","tenant":{"id":999}}"#).unwrap();
	let model: Document = decoded.into();
	assert_eq!(model.tenant_id, 999);

	let decoded_without_relation: DocumentInfo =
		serde_json::from_str(r#"{"id":1,"title":"private"}"#).unwrap();
	let model_without_relation: Document = decoded_without_relation.into();
	assert_eq!(model_without_relation.tenant_id, 0);
}
