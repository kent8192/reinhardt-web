use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

include!("../support.rs");

#[model(table_name = "tenants")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Tenant {
	#[field(primary_key = true)]
	id: i64,
	#[field(max_length = 120)]
	name: String,
}

fn default_tenant() -> model_info::RelationInfo<Tenant> {
	model_info::RelationInfo::new(0)
}

#[model(table_name = "documents")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Document {
	#[field(primary_key = true)]
	id: i64,
	#[field(max_length = 120)]
	title: String,
	tenant_id: i64,
	#[serde(skip_serializing, skip_deserializing, default = "default_tenant")]
	#[rel(foreign_key)]
	tenant: db::associations::ForeignKeyField<Tenant>,
}

fn main() {
	let info = DocumentInfo {
		id: 1,
		title: "private".to_string(),
		tenant: model_info::RelationInfo::new(42),
	};
	let serialized = serde_json::to_string(&info).unwrap();
	assert_eq!(serialized, r#"{"id":1,"title":"private"}"#);

	let decoded: DocumentInfo = serde_json::from_str(
		r#"{"id":1,"title":"private","tenant":{"id":999}}"#,
	)
	.unwrap();
	let model: Document = decoded.into();
	assert_eq!(model.tenant_id, 0);
}
