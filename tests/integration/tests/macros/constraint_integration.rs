//! Integration tests for check constraint support in Model derive macro

use reinhardt_macros::{model, Model};
use reinhardt_orm::Model as ModelTrait;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[model(app_label = "test_app", table_name = "products")]
struct Product {
	#[field(primary_key = true)]
	id: i64,

	#[field(max_length = 100)]
	name: String,

	#[field(check = "price > 0")]
	price: f64,

	#[field(check = "quantity >= 0")]
	quantity: i32,
}

#[test]
fn test_constraint_metadata() {
	let constraints = Product::constraint_metadata();

	// Should have two check constraints
	assert_eq!(constraints.len(), 2);

	// Check price constraint
	let price_constraint = constraints
		.iter()
		.find(|c| c.name == "price_check")
		.expect("price_check constraint should exist");
	assert_eq!(price_constraint.definition, "price > 0");

	// Check quantity constraint
	let quantity_constraint = constraints
		.iter()
		.find(|c| c.name == "quantity_check")
		.expect("quantity_check constraint should exist");
	assert_eq!(quantity_constraint.definition, "quantity >= 0");
}

#[test]
fn test_multiple_constraints() {
	#[derive(Serialize, Deserialize)]
	#[model(app_label = "test_app", table_name = "users")]
	struct User {
		#[field(primary_key = true)]
		id: i64,

		#[field(max_length = 100, check = "length(email) > 5")]
		email: String,

		#[field(check = "age >= 18")]
		age: i32,

		#[field(check = "balance >= 0")]
		balance: f64,
	}

	let constraints = User::constraint_metadata();
	assert_eq!(constraints.len(), 3);

	let constraint_names: Vec<String> = constraints.iter().map(|c| c.name.clone()).collect();
	assert!(constraint_names.contains(&"email_check".to_string()));
	assert!(constraint_names.contains(&"age_check".to_string()));
	assert!(constraint_names.contains(&"balance_check".to_string()));
}

#[test]
fn test_no_constraints() {
	#[derive(Serialize, Deserialize)]
	#[model(app_label = "test_app", table_name = "simple_model")]
	struct SimpleModel {
		#[field(primary_key = true)]
		id: i64,

		#[field(max_length = 100)]
		name: String,
	}

	let constraints = SimpleModel::constraint_metadata();
	assert_eq!(constraints.len(), 0);
}

#[test]
fn test_constraint_with_complex_expression() {
	#[derive(Serialize, Deserialize)]
	#[model(app_label = "test_app", table_name = "complex_constraints")]
	struct ComplexConstraints {
		#[field(primary_key = true)]
		id: i64,

		#[field(max_length = 50, check = "start_date < end_date")]
		start_date: String,

		#[field(check = "discount >= 0 AND discount <= 100")]
		discount: f64,
	}

	let constraints = ComplexConstraints::constraint_metadata();
	assert_eq!(constraints.len(), 2);

	let start_date_constraint = constraints
		.iter()
		.find(|c| c.name == "start_date_check")
		.expect("start_date_check constraint should exist");
	assert_eq!(start_date_constraint.definition, "start_date < end_date");

	let discount_constraint = constraints
		.iter()
		.find(|c| c.name == "discount_check")
		.expect("discount_check constraint should exist");
	assert_eq!(
		discount_constraint.definition,
		"discount >= 0 AND discount <= 100"
	);
}
