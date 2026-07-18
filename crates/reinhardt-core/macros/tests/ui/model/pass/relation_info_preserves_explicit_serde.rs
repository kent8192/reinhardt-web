use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

include!("../support.rs");

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

fn main() {
	let _info = DocumentInfo {
		id: 1,
		title: "private".to_string(),
		tenant: model_info::RelationInfo::new(42),
	};
}
