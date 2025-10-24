//! Model with various field types

use reinhardt_macros::Model;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Model)]
#[model(app_label = "test", table_name = "complex_model")]
struct ComplexModel {
    #[field(primary_key = true)]
    id: Option<i64>,

    #[field]
    integer_field: i32,

    #[field]
    string_field: String,

    #[field]
    boolean_field: bool,

    #[field]
    float_field: f64,

    #[field(null = true)]
    optional_int: Option<i32>,

    #[field(null = true)]
    optional_string: Option<String>,

    #[field(default = "true")]
    has_default: bool,
}

fn main() {
    let model = ComplexModel {
        id: Some(1),
        integer_field: 42,
        string_field: "test".to_string(),
        boolean_field: true,
        float_field: 3.14,
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
