//! Integration tests for validators with ModelSerializer

use reinhardt_orm::Model;
use reinhardt_serializers::validator_config::ValidatorConfig;
use reinhardt_serializers::validators::{UniqueTogetherValidator, UniqueValidator};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
	id: Option<i64>,
	username: String,
	email: String,
	age: Option<i32>,
}

impl Model for User {
	type PrimaryKey = i64;
	fn table_name() -> &'static str {
		"users"
	}
	fn primary_key(&self) -> Option<&Self::PrimaryKey> {
		self.id.as_ref()
	}
	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}
}

#[test]
fn test_unique_validator_basic() {
	let validator = UniqueValidator::<User>::new("username");
	assert_eq!(validator.field_name(), "username");
}

#[test]
fn test_unique_validator_with_custom_message() {
	let validator = UniqueValidator::<User>::new("username")
		.with_message("Username must be unique across all users");

	assert_eq!(validator.field_name(), "username");
}

#[test]
fn test_unique_together_validator_basic() {
	let validator = UniqueTogetherValidator::<User>::new(vec!["username", "email"]);

	assert_eq!(validator.field_names().len(), 2);
	assert_eq!(validator.field_names()[0], "username");
	assert_eq!(validator.field_names()[1], "email");
}

#[test]
fn test_unique_together_validator_with_custom_message() {
	let validator = UniqueTogetherValidator::<User>::new(vec!["username", "email"])
		.with_message("The combination of username and email must be unique");

	assert_eq!(validator.field_names().len(), 2);
}

#[test]
fn test_validator_config_single_unique() {
	let mut config = ValidatorConfig::<User>::new();
	assert!(!config.has_validators());

	config.add_unique_validator(UniqueValidator::new("username"));

	assert!(config.has_validators());
	assert_eq!(config.unique_validators().len(), 1);
	assert_eq!(config.unique_together_validators().len(), 0);
}

#[test]
fn test_validator_config_multiple_validators() {
	let mut config = ValidatorConfig::<User>::new();

	// Add multiple unique validators
	config.add_unique_validator(UniqueValidator::new("username"));
	config.add_unique_validator(UniqueValidator::new("email"));

	// Add unique together validator
	config.add_unique_together_validator(UniqueTogetherValidator::new(vec!["username", "email"]));

	assert!(config.has_validators());
	assert_eq!(config.unique_validators().len(), 2);
	assert_eq!(config.unique_together_validators().len(), 1);
}

#[test]
fn test_validator_config_builder_pattern() {
	let mut config = ValidatorConfig::<User>::new();

	config.add_unique_validator(
		UniqueValidator::new("username").with_message("Username already exists"),
	);
	config.add_unique_validator(UniqueValidator::new("email").with_message("Email already exists"));
	config.add_unique_together_validator(
		UniqueTogetherValidator::new(vec!["username", "email"])
			.with_message("Username and email combination already exists"),
	);

	assert_eq!(config.unique_validators().len(), 2);
	assert_eq!(config.unique_together_validators().len(), 1);
}

#[test]
fn test_validator_field_name_access() {
	let username_validator = UniqueValidator::<User>::new("username");
	let email_validator = UniqueValidator::<User>::new("email");

	assert_eq!(username_validator.field_name(), "username");
	assert_eq!(email_validator.field_name(), "email");
}

#[test]
fn test_unique_together_multiple_fields() {
	let validator = UniqueTogetherValidator::<User>::new(vec!["username", "email", "age"]);

	let field_names = validator.field_names();
	assert_eq!(field_names.len(), 3);
	assert!(field_names.contains(&"username".to_string()));
	assert!(field_names.contains(&"email".to_string()));
	assert!(field_names.contains(&"age".to_string()));
}

#[test]
fn test_validator_cloning() {
	let validator1 = UniqueValidator::<User>::new("username").with_message("Custom message");
	let validator2 = validator1.clone();

	assert_eq!(validator1.field_name(), validator2.field_name());
}

#[test]
fn test_validator_debug_output() {
	let validator = UniqueValidator::<User>::new("username");
	let debug_output = format!("{:?}", validator);

	// Should contain field name in debug output
	assert!(debug_output.contains("username"));
}

#[test]
fn test_validator_config_empty() {
	let config = ValidatorConfig::<User>::new();

	assert!(!config.has_validators());
	assert_eq!(config.unique_validators().len(), 0);
	assert_eq!(config.unique_together_validators().len(), 0);
}

#[test]
fn test_validator_config_default() {
	let config = ValidatorConfig::<User>::default();

	assert!(!config.has_validators());
}

#[test]
fn test_unique_together_with_single_field() {
	// Edge case: unique together with only one field
	let validator = UniqueTogetherValidator::<User>::new(vec!["username"]);

	assert_eq!(validator.field_names().len(), 1);
	assert_eq!(validator.field_names()[0], "username");
}

#[test]
fn test_validator_config_accessor_methods() {
	let mut config = ValidatorConfig::<User>::new();

	config.add_unique_validator(UniqueValidator::new("username"));
	config.add_unique_together_validator(UniqueTogetherValidator::new(vec!["username", "email"]));

	// Test accessor methods
	let unique_validators = config.unique_validators();
	assert_eq!(unique_validators.len(), 1);
	assert_eq!(unique_validators[0].field_name(), "username");

	let unique_together = config.unique_together_validators();
	assert_eq!(unique_together.len(), 1);
	assert_eq!(unique_together[0].field_names().len(), 2);
}

#[test]
fn test_multiple_unique_validators_same_field() {
	// Multiple validators for the same field (different messages)
	let mut config = ValidatorConfig::<User>::new();

	config.add_unique_validator(UniqueValidator::new("username").with_message("Username taken"));
	config.add_unique_validator(
		UniqueValidator::new("username").with_message("Choose a different username"),
	);

	// Both should be registered (application decides which to use)
	assert_eq!(config.unique_validators().len(), 2);
}

#[test]
fn test_validator_config_complex_scenario() {
	let mut config = ValidatorConfig::<User>::new();

	// Complex scenario: multiple unique fields and combinations
	config.add_unique_validator(UniqueValidator::new("username"));
	config.add_unique_validator(UniqueValidator::new("email"));

	// Username + email must be unique together
	config.add_unique_together_validator(UniqueTogetherValidator::new(vec!["username", "email"]));

	// Username + age must be unique together (for testing purposes)
	config.add_unique_together_validator(UniqueTogetherValidator::new(vec!["username", "age"]));

	assert_eq!(config.unique_validators().len(), 2);
	assert_eq!(config.unique_together_validators().len(), 2);
	assert!(config.has_validators());
}

// Tests for ModelSerializer integration
use reinhardt_serializers::ModelSerializer;

#[test]
fn test_model_serializer_with_unique_validator() {
	let serializer =
		ModelSerializer::<User>::new().with_unique_validator(UniqueValidator::new("username"));

	let validators = serializer.validators();
	assert!(validators.has_validators());
	assert_eq!(validators.unique_validators().len(), 1);
	assert_eq!(validators.unique_validators()[0].field_name(), "username");
}

#[test]
fn test_model_serializer_with_unique_together_validator() {
	let serializer = ModelSerializer::<User>::new()
		.with_unique_together_validator(UniqueTogetherValidator::new(vec!["username", "email"]));

	let validators = serializer.validators();
	assert!(validators.has_validators());
	assert_eq!(validators.unique_together_validators().len(), 1);
	assert_eq!(
		validators.unique_together_validators()[0]
			.field_names()
			.len(),
		2
	);
}

#[test]
fn test_model_serializer_with_multiple_validators() {
	let serializer = ModelSerializer::<User>::new()
		.with_unique_validator(UniqueValidator::new("username"))
		.with_unique_validator(UniqueValidator::new("email"))
		.with_unique_together_validator(UniqueTogetherValidator::new(vec!["username", "email"]));

	let validators = serializer.validators();
	assert!(validators.has_validators());
	assert_eq!(validators.unique_validators().len(), 2);
	assert_eq!(validators.unique_together_validators().len(), 1);
}

#[test]
fn test_model_serializer_validators_builder_pattern() {
	let serializer = ModelSerializer::<User>::new()
		.with_unique_validator(
			UniqueValidator::new("username").with_message("Username must be unique"),
		)
		.with_unique_validator(UniqueValidator::new("email").with_message("Email must be unique"))
		.with_unique_together_validator(
			UniqueTogetherValidator::new(vec!["username", "email"])
				.with_message("Username and email combination must be unique"),
		);

	let validators = serializer.validators();
	assert_eq!(validators.unique_validators().len(), 2);
	assert_eq!(validators.unique_together_validators().len(), 1);
}

#[test]
fn test_model_serializer_no_validators_by_default() {
	let serializer = ModelSerializer::<User>::new();

	let validators = serializer.validators();
	assert!(!validators.has_validators());
	assert_eq!(validators.unique_validators().len(), 0);
	assert_eq!(validators.unique_together_validators().len(), 0);
}

// Error handling tests
use reinhardt_serializers::DatabaseValidatorError;

#[test]
fn test_database_validator_error_unique_constraint_violation() {
	let error = DatabaseValidatorError::UniqueConstraintViolation {
		field: "username".to_string(),
		value: "alice".to_string(),
		table: "users".to_string(),
		message: Some("Username already exists".to_string()),
	};

	let error_message = error.to_string();
	assert_eq!(
		error_message,
		"Unique constraint violated: username = 'alice' already exists in table users"
	);
}

#[test]
fn test_database_validator_error_unique_together_violation() {
	let error = DatabaseValidatorError::UniqueTogetherViolation {
		fields: vec!["username".to_string(), "email".to_string()],
		values: vec!["alice".to_string(), "alice@example.com".to_string()],
		table: "users".to_string(),
		message: Some("Combination already exists".to_string()),
	};

	let error_message = error.to_string();
	assert_eq!(
		error_message,
		"Unique together constraint violated: fields ([\"username\", \"email\"]) with values ([\"alice\", \"alice@example.com\"]) already exist in table users"
	);
}

#[test]
fn test_database_validator_error_database_error() {
	let error = DatabaseValidatorError::DatabaseError {
		message: "Connection timeout".to_string(),
		query: Some("SELECT COUNT(*) FROM users WHERE username = $1".to_string()),
	};

	let error_message = error.to_string();
	assert_eq!(
		error_message,
		"Database error during validation: Connection timeout"
	);
}

#[test]
fn test_database_validator_error_field_not_found() {
	let error = DatabaseValidatorError::FieldNotFound {
		field: "unknown_field".to_string(),
	};

	let error_message = error.to_string();
	assert_eq!(
		error_message,
		"Required field 'unknown_field' not found in validation data"
	);
}

#[test]
fn test_database_validator_error_conversion_to_serializer_error() {
	use reinhardt_serializers::SerializerError;

	let validator_error = DatabaseValidatorError::UniqueConstraintViolation {
		field: "email".to_string(),
		value: "test@example.com".to_string(),
		table: "users".to_string(),
		message: None,
	};

	let serializer_error: SerializerError = validator_error.into();
	let error_message = format!("{:?}", serializer_error);
	assert!(error_message.contains("email") || error_message.contains("test@example.com"));
}
