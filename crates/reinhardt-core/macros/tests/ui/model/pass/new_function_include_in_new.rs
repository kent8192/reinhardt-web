//! Test that `#[field(include_in_new = ...)]` controls new() function parameter inclusion

use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

/// Model demonstrating include_in_new control
#[model(app_label = "test", table_name = "test_product")]
#[derive(Serialize, Deserialize)]
pub struct Product {
	#[field(primary_key = true)]
	pub id: Option<i32>,

	#[field(max_length = 255)]
	pub name: String,

	// Optional field explicitly included via include_in_new = true
	#[field(max_length = 100, include_in_new = true)]
	pub category: Option<String>,

	// Optional field excluded via include_in_new = false
	#[field(max_length = 500, include_in_new = false)]
	pub description: Option<String>,
}

fn main() {
	// new() should require name, and category (due to include_in_new = true)
	// id is Option<i32> primary key (auto-excluded from new())
	// description is NOT included (due to include_in_new = false)
	let product = Product::new("Widget", Some("Electronics".to_string()));

	assert_eq!(product.name, "Widget");
	assert_eq!(product.category, Some("Electronics".to_string()));
	// description should be default (None)
	assert!(product.description.is_none());
	assert!(product.id.is_none());
}
