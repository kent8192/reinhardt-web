use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

include!("../support.rs");

#[model(table_name = "books")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Book {
	#[field(primary_key = true)]
	id: i64,
	#[field(max_length = 120)]
	title: String,
}

fn assert_info_model<T: model_info::InfoModel<PrimaryKey = i64>>() {}

fn main() {
	assert_info_model::<Book>();
	let _ = BookInfo {
		id: 1,
		title: "hello".to_string(),
	};
}
