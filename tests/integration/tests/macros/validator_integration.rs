//! Integration tests for validator support in Model derive macro

use reinhardt_macros::{model, Model};
use reinhardt_orm::Model as ModelTrait;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[model(app_label = "test_app", table_name = "users")]
struct User {
	#[field(primary_key = true)]
	id: i64,

	#[field(max_length = 100, email = true)]
	email: String,

	#[field(max_length = 200, url = true)]
	website: String,

	#[field(max_length = 100, min_length = 3)]
	username: String,

	#[field(min_value = 0, max_value = 120)]
	age: i32,
}

#[test]
fn test_email_validator() {
	let fields = User::field_metadata();

	let email_field = fields
		.iter()
		.find(|f| f.name == "email")
		.expect("email field should exist");

	assert!(email_field.attributes.contains_key("email"));
	assert_eq!(
		email_field.attributes.get("email"),
		Some(&reinhardt_orm::fields::FieldKwarg::Bool(true))
	);
}

#[test]
fn test_url_validator() {
	let fields = User::field_metadata();

	let website_field = fields
		.iter()
		.find(|f| f.name == "website")
		.expect("website field should exist");

	assert!(website_field.attributes.contains_key("url"));
	assert_eq!(
		website_field.attributes.get("url"),
		Some(&reinhardt_orm::fields::FieldKwarg::Bool(true))
	);
}

#[test]
fn test_min_length_validator() {
	let fields = User::field_metadata();

	let username_field = fields
		.iter()
		.find(|f| f.name == "username")
		.expect("username field should exist");

	assert!(username_field.attributes.contains_key("min_length"));
	assert_eq!(
		username_field.attributes.get("min_length"),
		Some(&reinhardt_orm::fields::FieldKwarg::Uint(3))
	);
}

#[test]
fn test_min_max_value_validators() {
	let fields = User::field_metadata();

	let age_field = fields
		.iter()
		.find(|f| f.name == "age")
		.expect("age field should exist");

	assert!(age_field.attributes.contains_key("min_value"));
	assert_eq!(
		age_field.attributes.get("min_value"),
		Some(&reinhardt_orm::fields::FieldKwarg::Int(0))
	);

	assert!(age_field.attributes.contains_key("max_value"));
	assert_eq!(
		age_field.attributes.get("max_value"),
		Some(&reinhardt_orm::fields::FieldKwarg::Int(120))
	);
}

#[test]
fn test_multiple_validators_on_single_field() {
	#[derive(Serialize, Deserialize)]
	#[model(app_label = "test_app", table_name = "products")]
	struct Product {
		#[field(primary_key = true)]
		id: i64,

		#[field(max_length = 200, min_length = 10, url = true)]
		product_url: String,

		#[field(min_value = 1, max_value = 9999)]
		price: i32,
	}

	let fields = Product::field_metadata();

	// Check product_url has multiple validators
	let url_field = fields
		.iter()
		.find(|f| f.name == "product_url")
		.expect("product_url field should exist");

	assert!(url_field.attributes.contains_key("url"));
	assert!(url_field.attributes.contains_key("min_length"));
	assert!(url_field.attributes.contains_key("max_length"));

	// Check price has min and max value validators
	let price_field = fields
		.iter()
		.find(|f| f.name == "price")
		.expect("price field should exist");

	assert!(price_field.attributes.contains_key("min_value"));
	assert!(price_field.attributes.contains_key("max_value"));
}

#[test]
fn test_no_validators() {
	#[derive(Serialize, Deserialize)]
	#[model(app_label = "test_app", table_name = "simple_model")]
	struct SimpleModel {
		#[field(primary_key = true)]
		id: i64,

		#[field(max_length = 100)]
		name: String,
	}

	let fields = SimpleModel::field_metadata();

	let name_field = fields
		.iter()
		.find(|f| f.name == "name")
		.expect("name field should exist");

	// Should only have max_length, no validator attributes
	assert!(name_field.attributes.contains_key("max_length"));
	assert!(!name_field.attributes.contains_key("email"));
	assert!(!name_field.attributes.contains_key("url"));
	assert!(!name_field.attributes.contains_key("min_length"));
	assert!(!name_field.attributes.contains_key("min_value"));
	assert!(!name_field.attributes.contains_key("max_value"));
}
