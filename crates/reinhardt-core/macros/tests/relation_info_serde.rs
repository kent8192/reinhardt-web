// The model macro emits `cfg(native)` arms; this test only validates generated serde behavior.
#![allow(unexpected_cfgs)]

use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

include!("ui/model/support.rs");

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

#[model(table_name = "tenants")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Tenant {
	#[field(primary_key = true)]
	id: i64,
	#[field(max_length = 120)]
	name: String,
}

trait DefaultTenantRelation {
	fn default_tenant() -> Self;
}

impl DefaultTenantRelation for db::associations::ForeignKeyField<Tenant> {
	fn default_tenant() -> Self {
		Self::default()
	}
}

impl DefaultTenantRelation for model_info::RelationInfo<Tenant> {
	fn default_tenant() -> Self {
		model_info::RelationInfo::new(0)
	}
}

fn default_tenant<T: DefaultTenantRelation>() -> T {
	T::default_tenant()
}

#[model(table_name = "documents")]
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
	assert_eq!(model.tenant_id, 0);
}
