use reinhardt_macros::model;

include!("../support.rs");

use db::orm::Model;

#[model(app_label = "default", table_name = "native_models")]
#[derive(Debug, Clone)]
struct NativeModel {
	#[field(primary_key = true)]
	id: i64,
	#[field(max_length = 120)]
	name: String,
}

fn main() {
	let _field_metadata: Vec<db::orm::inspection::FieldInfo> = NativeModel::field_metadata();
}
