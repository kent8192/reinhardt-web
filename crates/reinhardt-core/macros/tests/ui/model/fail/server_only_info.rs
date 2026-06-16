use reinhardt_macros::model;

include!("../support.rs");

#[model(table_name = "secrets", server_only)]
struct Secret {
	#[field(primary_key = true)]
	id: i64,
}

fn assert_info_model<T: model_info::InfoModel<PrimaryKey = i64>>() {}

fn main() {
	assert_info_model::<Secret>();
	let _ = SecretInfo { id: 1 };
}
