//! Model with various field types

use reinhardt_macros::Model;
use serde::{Deserialize, Serialize};

// Required by Model derive macro
#[allow(unused_imports)]
use reinhardt_db::migrations as _;
#[allow(unused_imports)]
use reinhardt_db::orm::{self as _, Model as _};

#[derive(Debug, Clone, Serialize, Deserialize, Model)]
#[model(app_label = "test", table_name = "complex_model")]
struct ComplexModel {
	#[field(primary_key = true)]
	id: Option<i64>,

	integer_field: i32,

	#[field(max_length = 255)]
	string_field: String,

	boolean_field: bool,

	float_field: f64,

	#[field(null = true)]
	optional_int: Option<i32>,

	#[field(max_length = 255, null = true)]
	optional_string: Option<String>,

	#[field(default = "true")]
	has_default: bool,
}

fn main() {
	let _model = ComplexModel {
		id: Some(1),
		integer_field: 42,
		string_field: "test".to_string(),
		boolean_field: true,
		float_field: 3.15,
		optional_int: None,
		optional_string: Some("optional".to_string()),
		has_default: false,
	};

	// Verify Model trait is implemented
	assert_eq!(ComplexModel::table_name(), "complex_model");
	assert_eq!(ComplexModel::app_label(), "test");

	// Verify field metadata is generated
	let fields = ComplexModel::field_metadata();
	assert_eq!(fields.len(), 8);
}
