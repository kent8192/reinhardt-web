//! Error handling tests for SerializerError and ValidatorError
//!
//! These tests verify that errors are properly constructed, formatted,
//! and provide useful information for debugging and user feedback.

use reinhardt_rest::serializers::{SerializerError, ValidatorError};
use std::collections::HashMap;

#[test]
fn test_validator_error_unique_violation() {
	let error = ValidatorError::UniqueViolation {
		field_name: "username".to_string(),
		value: "alice".to_string(),
		message: "Username must be unique".to_string(),
	};

	assert_eq!(error.message(), "Username must be unique");
	assert_eq!(error.field_names(), vec!["username"]);
	assert!(error.is_uniqueness_violation());
	assert!(!error.is_database_error());

	let display = format!("{}", error);
	assert_eq!(
		display,
		"Unique violation on field 'username' with value 'alice': Username must be unique"
	);
}

#[test]
fn test_validator_error_unique_together_violation() {
	let mut values = HashMap::new();
	values.insert("username".to_string(), "alice".to_string());
	values.insert("email".to_string(), "alice@example.com".to_string());

	let error = ValidatorError::UniqueTogetherViolation {
		field_names: vec!["username".to_string(), "email".to_string()],
		values,
		message: "Username and email combination must be unique".to_string(),
	};

	assert_eq!(
		error.message(),
		"Username and email combination must be unique"
	);
	assert_eq!(error.field_names(), vec!["username", "email"]);
	assert!(error.is_uniqueness_violation());
	assert!(!error.is_database_error());

	let display = format!("{}", error);
	assert_eq!(
		display,
		"Unique together violation on fields [username, email] with values (email=alice@example.com, username=alice): Username and email combination must be unique"
	);
}

#[test]
fn test_validator_error_required_field() {
	let error = ValidatorError::RequiredField {
		field_name: "username".to_string(),
		message: "This field is required".to_string(),
	};

	assert_eq!(error.message(), "This field is required");
	assert_eq!(error.field_names(), vec!["username"]);
	assert!(!error.is_uniqueness_violation());
	assert!(!error.is_database_error());

	let display = format!("{}", error);
	assert_eq!(display, "Required field 'username': This field is required");
}

#[test]
fn test_validator_error_field_validation() {
	let error = ValidatorError::FieldValidation {
		field_name: "age".to_string(),
		value: "-5".to_string(),
		constraint: "must be positive".to_string(),
		message: "Age must be a positive number".to_string(),
	};

	assert_eq!(error.message(), "Age must be a positive number");
	assert_eq!(error.field_names(), vec!["age"]);
	assert!(!error.is_uniqueness_violation());
	assert!(!error.is_database_error());

	let display = format!("{}", error);
	assert_eq!(
		display,
		"Field 'age' with value '-5' failed constraint 'must be positive': Age must be a positive number"
	);
}

#[test]
fn test_validator_error_database_error() {
	let error = ValidatorError::DatabaseError {
		message: "Connection refused".to_string(),
		source: Some("PostgreSQL".to_string()),
	};

	assert_eq!(error.message(), "Connection refused");
	assert!(error.field_names().is_empty());
	assert!(!error.is_uniqueness_violation());
	assert!(error.is_database_error());

	let display = format!("{}", error);
	assert_eq!(
		display,
		"Database error: Connection refused (source: PostgreSQL)"
	);
}

#[test]
fn test_validator_error_database_error_without_source() {
	let error = ValidatorError::DatabaseError {
		message: "Query timeout".to_string(),
		source: None,
	};

	assert_eq!(error.message(), "Query timeout");
	assert!(error.is_database_error());

	let display = format!("{}", error);
	assert_eq!(display, "Database error: Query timeout");
}

#[test]
fn test_validator_error_custom() {
	let error = ValidatorError::Custom {
		message: "Custom validation error".to_string(),
	};

	assert_eq!(error.message(), "Custom validation error");
	assert!(error.field_names().is_empty());
	assert!(!error.is_uniqueness_violation());
	assert!(!error.is_database_error());
}

#[test]
fn test_serializer_error_validation() {
	let validator_error = ValidatorError::UniqueViolation {
		field_name: "email".to_string(),
		value: "test@example.com".to_string(),
		message: "Email already exists".to_string(),
	};

	let error = SerializerError::validation(validator_error.clone());

	assert!(error.is_validation_error());
	assert_eq!(error.message(), "Email already exists");

	let validator = error.as_validator_error();
	assert!(validator.is_some());
	assert_eq!(validator.unwrap().message(), "Email already exists");
}

#[test]
fn test_serializer_error_unique_violation_helper() {
	let error = SerializerError::unique_violation(
		"username".to_string(),
		"alice".to_string(),
		"Username already taken".to_string(),
	);

	assert!(error.is_validation_error());
	assert_eq!(error.message(), "Username already taken");

	let validator = error.as_validator_error().unwrap();
	assert_eq!(validator.field_names(), vec!["username"]);
	assert!(validator.is_uniqueness_violation());
}

#[test]
fn test_serializer_error_unique_together_violation_helper() {
	let mut values = HashMap::new();
	values.insert("first_name".to_string(), "John".to_string());
	values.insert("last_name".to_string(), "Doe".to_string());

	let error = SerializerError::unique_together_violation(
		vec!["first_name".to_string(), "last_name".to_string()],
		values,
		"Name combination already exists".to_string(),
	);

	assert!(error.is_validation_error());
	assert_eq!(error.message(), "Name combination already exists");

	let validator = error.as_validator_error().unwrap();
	assert_eq!(validator.field_names(), vec!["first_name", "last_name"]);
}

#[test]
fn test_serializer_error_required_field_helper() {
	let error =
		SerializerError::required_field("password".to_string(), "Password is required".to_string());

	assert!(error.is_validation_error());
	assert_eq!(error.message(), "Password is required");

	let validator = error.as_validator_error().unwrap();
	assert_eq!(validator.field_names(), vec!["password"]);
}

#[test]
fn test_serializer_error_database_error_helper() {
	let error = SerializerError::database_error(
		"Connection pool exhausted".to_string(),
		Some("SQLx".to_string()),
	);

	assert!(error.is_validation_error());
	assert_eq!(error.message(), "Connection pool exhausted");

	let validator = error.as_validator_error().unwrap();
	assert!(validator.is_database_error());
}

#[test]
fn test_serializer_error_serde() {
	let error = SerializerError::Serde {
		message: "Invalid JSON format".to_string(),
	};

	assert!(!error.is_validation_error());
	assert_eq!(error.message(), "Invalid JSON format");
	assert!(error.as_validator_error().is_none());

	let display = format!("{}", error);
	assert_eq!(display, "Serde error: Invalid JSON format");
}

#[test]
fn test_serializer_error_other() {
	let error = SerializerError::new("Generic error".to_string());

	assert!(!error.is_validation_error());
	assert_eq!(error.message(), "Generic error");
	assert!(error.as_validator_error().is_none());
}

#[test]
fn test_serializer_error_display() {
	let validator_error = ValidatorError::UniqueViolation {
		field_name: "email".to_string(),
		value: "test@example.com".to_string(),
		message: "Email exists".to_string(),
	};

	let error = SerializerError::Validation(validator_error);
	let display = format!("{}", error);

	assert_eq!(
		display,
		"Unique violation on field 'email' with value 'test@example.com': Email exists"
	);
}

#[test]
fn test_serializer_error_source() {
	let validator_error = ValidatorError::DatabaseError {
		message: "Connection failed".to_string(),
		source: None,
	};

	let error = SerializerError::Validation(validator_error);

	// Test that error implements std::error::Error trait
	let err_ref: &dyn std::error::Error = &error;
	assert!(err_ref.source().is_some());
}

#[test]
fn test_validator_error_clone() {
	let error1 = ValidatorError::UniqueViolation {
		field_name: "username".to_string(),
		value: "alice".to_string(),
		message: "Already exists".to_string(),
	};

	let error2 = error1.clone();
	assert_eq!(error1.message(), error2.message());
	assert_eq!(error1.field_names(), error2.field_names());
}

#[test]
fn test_validator_error_equality() {
	let error1 = ValidatorError::Custom {
		message: "Test error".to_string(),
	};

	let error2 = ValidatorError::Custom {
		message: "Test error".to_string(),
	};

	let error3 = ValidatorError::Custom {
		message: "Different error".to_string(),
	};

	assert_eq!(error1, error2);
	assert_ne!(error1, error3);
}

#[test]
fn test_validator_error_debug() {
	let error = ValidatorError::UniqueViolation {
		field_name: "email".to_string(),
		value: "test@example.com".to_string(),
		message: "Email exists".to_string(),
	};

	let debug = format!("{:?}", error);
	assert!(debug.contains("UniqueViolation"));
	assert!(debug.contains("email"));
}

#[test]
fn test_serializer_error_debug() {
	let error = SerializerError::new("Test error".to_string());
	let debug = format!("{:?}", error);
	assert!(debug.contains("Other"));
	assert!(debug.contains("Test error"));
}

#[test]
fn test_multiple_field_names_in_unique_together() {
	let error = ValidatorError::UniqueTogetherViolation {
		field_names: vec![
			"country".to_string(),
			"city".to_string(),
			"postal_code".to_string(),
		],
		values: HashMap::new(),
		message: "Location already exists".to_string(),
	};

	let field_names = error.field_names();
	assert_eq!(field_names.len(), 3);
	assert!(field_names.contains(&"country"));
	assert!(field_names.contains(&"city"));
	assert!(field_names.contains(&"postal_code"));
}

#[test]
fn test_empty_values_in_unique_together() {
	let error = ValidatorError::UniqueTogetherViolation {
		field_names: vec!["field1".to_string(), "field2".to_string()],
		values: HashMap::new(),
		message: "Fields must be unique".to_string(),
	};

	let display = format!("{}", error);
	assert_eq!(
		display,
		"Unique together violation on fields [field1, field2] with values (): Fields must be unique"
	);
}
