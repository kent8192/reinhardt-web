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

fn main() {
	let _info = DocumentInfo {
		id: 1,
		title: "private".to_string(),
		tenant: model_info::RelationInfo::new(42),
	};
}
