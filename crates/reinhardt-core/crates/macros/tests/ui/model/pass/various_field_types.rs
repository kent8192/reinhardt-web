//! Model with various field types

use reinhardt_db::orm::Model as ModelTrait;
use reinhardt_macros::{model, Model};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
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
	assert_eq!(<ComplexModel as ModelTrait>::table_name(), "complex_model");
	assert_eq!(<ComplexModel as ModelTrait>::app_label(), "test");

	// Verify field metadata is generated
	let fields = <ComplexModel as ModelTrait>::field_metadata();
	assert_eq!(fields.len(), 8);
}
