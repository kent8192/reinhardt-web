//! Integration tests for the Model derive macro (via #[model] attribute)
//!
//! Tests the interaction between:
//! - reinhardt-macros (Model derive macro)
//! - reinhardt-orm (Model trait)
//! - reinhardt-migrations (model_registry)

use reinhardt_macros::model;
use reinhardt_migrations::model_registry::global_registry;
use reinhardt_orm::Model as ModelTrait;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[model(app_label = "test_app", table_name = "test_users")]
struct TestUser {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field(max_length = 100, null = false)]
	username: String,

	#[field(max_length = 255)]
	email: String,

	#[field(null = true)]
	age: Option<i32>,

	#[field(default = "true")]
	is_active: bool,
}

#[test]
fn test_model_trait_implementation() {
	// Verify Model trait methods are correctly implemented
	assert_eq!(TestUser::table_name(), "test_users");
	assert_eq!(TestUser::app_label(), "test_app");
	assert_eq!(TestUser::primary_key_field(), "id");
}

#[test]
fn test_field_metadata_generation() {
	// Get field metadata
	let fields = TestUser::field_metadata();

	// Should have 5 fields
	assert_eq!(fields.len(), 5, "Expected 5 fields");

	// Check id field
	let id_field = fields.iter().find(|f| f.name == "id");
	assert!(id_field.is_some(), "id field not found");
	let id_field = id_field.unwrap();
	assert_eq!(id_field.field_type, "reinhardt.orm.models.IntegerField");
	assert!(id_field.primary_key, "id should be primary key");
	assert!(id_field.nullable, "id should be nullable (Option<i32>)");

	// Check username field
	let username_field = fields.iter().find(|f| f.name == "username");
	assert!(username_field.is_some(), "username field not found");
	let username_field = username_field.unwrap();
	assert_eq!(username_field.field_type, "reinhardt.orm.models.CharField");
	assert!(!username_field.nullable, "username should not be nullable");
	assert!(
		username_field.attributes.contains_key("max_length"),
		"username should have max_length attribute"
	);

	// Check email field
	let email_field = fields.iter().find(|f| f.name == "email");
	assert!(email_field.is_some(), "email field not found");
	let email_field = email_field.unwrap();
	assert_eq!(email_field.field_type, "reinhardt.orm.models.CharField");
	assert!(
		email_field.attributes.contains_key("max_length"),
		"email should have max_length attribute"
	);

	// Check age field
	let age_field = fields.iter().find(|f| f.name == "age");
	assert!(age_field.is_some(), "age field not found");
	let age_field = age_field.unwrap();
	assert_eq!(age_field.field_type, "reinhardt.orm.models.IntegerField");
	assert!(age_field.nullable, "age should be nullable");

	// Check is_active field
	let is_active_field = fields.iter().find(|f| f.name == "is_active");
	assert!(is_active_field.is_some(), "is_active field not found");
	let is_active_field = is_active_field.unwrap();
	assert_eq!(
		is_active_field.field_type,
		"reinhardt.orm.models.BooleanField"
	);
}

#[test]
fn test_model_registration() {
	// Verify the model was automatically registered via ctor
	let registry = global_registry();
	let models = registry.get_models();

	// Find our test model
	let test_model = models
		.iter()
		.find(|m| m.app_label == "test_app" && m.model_name == "TestUser");

	assert!(
		test_model.is_some(),
		"TestUser should be registered in global registry"
	);

	let test_model = test_model.unwrap();
	assert_eq!(test_model.table_name, "test_users");

	// Verify fields were registered
	assert_eq!(test_model.fields.len(), 5, "Expected 5 registered fields");

	// Verify field names
	assert!(test_model.fields.contains_key("id"));
	assert!(test_model.fields.contains_key("username"));
	assert!(test_model.fields.contains_key("email"));
	assert!(test_model.fields.contains_key("age"));
	assert!(test_model.fields.contains_key("is_active"));
}

#[test]
fn test_primary_key_access() {
	// Test with None primary key
	let mut user = TestUser {
		id: None,
		username: "testuser".to_string(),
		email: "test@example.com".to_string(),
		age: Some(25),
		is_active: true,
	};

	// Initially no primary key
	assert!(
		user.primary_key().is_none(),
		"New user should have no primary key"
	);

	// Set primary key
	user.set_primary_key(42);
	assert_eq!(
		user.primary_key(),
		Some(42),
		"Primary key should be set to 42"
	);

	// Test with Some primary key from the start
	let user_with_id = TestUser {
		id: Some(100),
		username: "anotheruser".to_string(),
		email: "another@example.com".to_string(),
		age: None,
		is_active: false,
	};

	assert_eq!(
		user_with_id.primary_key(),
		Some(100),
		"User should have primary key 100"
	);
}

#[test]
fn test_multiple_models_registration() {
	// Define another model to ensure multiple models can be registered
	#[derive(Serialize, Deserialize)]
	#[model(app_label = "test_app", table_name = "test_posts")]
	#[allow(dead_code)]
	struct TestPost {
		#[field(primary_key = true)]
		id: Option<i64>,

		#[field(max_length = 200)]
		title: String,
	}

	// Verify both models are registered
	let registry = global_registry();
	let models = registry.get_models();

	let user_model = models
		.iter()
		.find(|m| m.model_name == "TestUser" && m.app_label == "test_app");
	let post_model = models
		.iter()
		.find(|m| m.model_name == "TestPost" && m.app_label == "test_app");

	assert!(user_model.is_some(), "TestUser should be registered");
	assert!(post_model.is_some(), "TestPost should be registered");
}
