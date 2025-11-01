//! Integration tests for Serializers with MethodField and Validation
//!
//! Tests the integration between SerializerMethodField and field/object validation

use reinhardt_serializers::{
	FieldValidator, MethodFieldProvider, MethodFieldRegistry, ObjectValidator,
	SerializerMethodField, ValidationError, ValidationResult, validate_fields,
};
use serde_json::{Value, json};
use std::collections::HashMap;

// Test serializer for User with method fields and validation
struct UserSerializer {
	method_fields: MethodFieldRegistry,
}

impl UserSerializer {
	fn new() -> Self {
		let mut method_fields = MethodFieldRegistry::new();
		method_fields.register("full_name", SerializerMethodField::new("full_name"));
		method_fields.register(
			"display_age",
			SerializerMethodField::new("display_age").method_name("get_age_display"),
		);

		Self { method_fields }
	}

	fn get_full_name(&self, first_name: &str, last_name: &str) -> String {
		format!("{} {}", first_name, last_name)
	}

	fn get_age_display(&self, age: i64) -> String {
		format!("{} years old", age)
	}
}

impl MethodFieldProvider for UserSerializer {
	fn compute_method_fields(&self, instance: &Value) -> HashMap<String, Value> {
		let mut context = HashMap::new();

		if let Some(obj) = instance.as_object() {
			// Compute full_name
			if let (Some(first), Some(last)) = (
				obj.get("first_name").and_then(|v| v.as_str()),
				obj.get("last_name").and_then(|v| v.as_str()),
			) {
				let full_name = self.get_full_name(first, last);
				context.insert("full_name".to_string(), json!(full_name));
			}

			// Compute display_age
			if let Some(age) = obj.get("age").and_then(|v| v.as_i64()) {
				let display = self.get_age_display(age);
				context.insert("get_age_display".to_string(), json!(display));
			}
		}

		context
	}

	fn compute_method(&self, method_name: &str, instance: &Value) -> Option<Value> {
		let context = self.compute_method_fields(instance);
		context.get(method_name).cloned()
	}
}

// Validators
struct AgeValidator;

impl FieldValidator for AgeValidator {
	fn validate(&self, value: &Value) -> ValidationResult {
		if let Some(age) = value.as_i64() {
			if age >= 0 && age <= 150 {
				Ok(())
			} else {
				Err(ValidationError::field_error(
					"age",
					"Age must be between 0 and 150",
				))
			}
		} else {
			Err(ValidationError::field_error("age", "Age must be a number"))
		}
	}
}

struct EmailValidator;

impl FieldValidator for EmailValidator {
	fn validate(&self, value: &Value) -> ValidationResult {
		if let Some(email) = value.as_str() {
			if email.contains('@') && email.contains('.') {
				Ok(())
			} else {
				Err(ValidationError::field_error(
					"email",
					"Invalid email format",
				))
			}
		} else {
			Err(ValidationError::field_error(
				"email",
				"Email must be a string",
			))
		}
	}
}

struct NameConsistencyValidator;

impl ObjectValidator for NameConsistencyValidator {
	fn validate(&self, data: &HashMap<String, Value>) -> ValidationResult {
		let first_name = data.get("first_name").and_then(|v| v.as_str());
		let last_name = data.get("last_name").and_then(|v| v.as_str());

		if let (Some(first), Some(last)) = (first_name, last_name) {
			if first.is_empty() || last.is_empty() {
				return Err(ValidationError::object_error(
					"Both first name and last name are required",
				));
			}
		}

		Ok(())
	}
}

#[test]
fn test_serializer_with_method_fields() {
	let serializer = UserSerializer::new();
	let user_data = json!({
		"first_name": "John",
		"last_name": "Doe",
		"age": 30,
		"email": "john.doe@example.com"
	});

	let context = serializer.compute_method_fields(&user_data);

	// Check method field values
	assert_eq!(context.get("full_name").unwrap(), &json!("John Doe"));
	assert_eq!(
		context.get("get_age_display").unwrap(),
		&json!("30 years old")
	);

	// Check method field retrieval
	let full_name_field = serializer.method_fields.get("full_name").unwrap();
	let full_name_value = full_name_field.get_value(&context).unwrap();
	assert_eq!(full_name_value, json!("John Doe"));
}

#[test]
fn test_serializer_with_field_validation_success() {
	let mut validators: HashMap<String, Box<dyn FieldValidator>> = HashMap::new();
	validators.insert("age".to_string(), Box::new(AgeValidator));
	validators.insert("email".to_string(), Box::new(EmailValidator));

	let mut data = HashMap::new();
	data.insert("age".to_string(), json!(25));
	data.insert("email".to_string(), json!("user@example.com"));

	let result = validate_fields(&data, &validators);
	assert!(result.is_ok());
}

#[test]
fn test_serializer_with_field_validation_failure() {
	let mut validators: HashMap<String, Box<dyn FieldValidator>> = HashMap::new();
	validators.insert("age".to_string(), Box::new(AgeValidator));

	let mut data = HashMap::new();
	data.insert("age".to_string(), json!(200)); // Invalid age

	let result = validate_fields(&data, &validators);
	assert!(result.is_err());
}

#[test]
fn test_serializer_with_object_validation_success() {
	let validator = NameConsistencyValidator;

	let mut data = HashMap::new();
	data.insert("first_name".to_string(), json!("Jane"));
	data.insert("last_name".to_string(), json!("Smith"));

	let result = validator.validate(&data);
	assert!(result.is_ok());
}

#[test]
fn test_serializer_with_object_validation_failure() {
	let validator = NameConsistencyValidator;

	let mut data = HashMap::new();
	data.insert("first_name".to_string(), json!(""));
	data.insert("last_name".to_string(), json!("Smith"));

	let result = validator.validate(&data);
	assert!(result.is_err());
}

#[test]
fn test_complete_serialization_with_validation_and_method_fields() {
	// Setup serializer with method fields
	let serializer = UserSerializer::new();

	// Setup validators
	let mut validators: HashMap<String, Box<dyn FieldValidator>> = HashMap::new();
	validators.insert("age".to_string(), Box::new(AgeValidator));
	validators.insert("email".to_string(), Box::new(EmailValidator));

	// Create valid user data
	let user_data = json!({
		"first_name": "Alice",
		"last_name": "Johnson",
		"age": 28,
		"email": "alice.johnson@example.com"
	});

	// Convert to HashMap for validation
	let mut data_map = HashMap::new();
	if let Some(obj) = user_data.as_object() {
		for (key, value) in obj {
			data_map.insert(key.clone(), value.clone());
		}
	}

	// Validate fields
	let validation_result = validate_fields(&data_map, &validators);
	assert!(validation_result.is_ok());

	// Validate object
	let object_validator = NameConsistencyValidator;
	let object_validation = object_validator.validate(&data_map);
	assert!(object_validation.is_ok());

	// Compute method fields
	let context = serializer.compute_method_fields(&user_data);

	// Verify method field values
	assert_eq!(context.get("full_name").unwrap(), &json!("Alice Johnson"));
	assert_eq!(
		context.get("get_age_display").unwrap(),
		&json!("28 years old")
	);
}

#[test]
fn test_serialization_with_multiple_validation_errors() {
	let mut validators: HashMap<String, Box<dyn FieldValidator>> = HashMap::new();
	validators.insert("age".to_string(), Box::new(AgeValidator));
	validators.insert("email".to_string(), Box::new(EmailValidator));

	let mut data = HashMap::new();
	data.insert("age".to_string(), json!(-5)); // Invalid age
	data.insert("email".to_string(), json!("not-an-email")); // Invalid email

	let result = validate_fields(&data, &validators);
	assert!(result.is_err());

	if let Err(ValidationError::MultipleErrors(errors)) = result {
		assert_eq!(errors.len(), 2);
	} else {
		panic!("Expected MultipleErrors");
	}
}

#[test]
fn test_method_field_with_missing_data() {
	let serializer = UserSerializer::new();

	// User data missing last_name
	let incomplete_data = json!({
		"first_name": "Bob",
		"age": 35
	});

	let context = serializer.compute_method_fields(&incomplete_data);

	// full_name should not be in context because last_name is missing
	assert!(context.get("full_name").is_none());

	// display_age should be present
	assert_eq!(
		context.get("get_age_display").unwrap(),
		&json!("35 years old")
	);
}

#[test]
fn test_custom_method_name_in_integration() {
	let mut method_fields = MethodFieldRegistry::new();

	// Register field with custom method name
	let field = SerializerMethodField::new("age_text").method_name("compute_age_description");
	method_fields.register("age_text", field);

	// Simulate method computation
	let mut context = HashMap::new();
	context.insert("compute_age_description".to_string(), json!("Adult"));

	let age_field = method_fields.get("age_text").unwrap();
	let value = age_field.get_value(&context).unwrap();

	assert_eq!(value, json!("Adult"));
}

#[test]
fn test_validation_with_valid_and_invalid_fields_mixed() {
	let mut validators: HashMap<String, Box<dyn FieldValidator>> = HashMap::new();
	validators.insert("age".to_string(), Box::new(AgeValidator));
	validators.insert("email".to_string(), Box::new(EmailValidator));

	let mut data = HashMap::new();
	data.insert("age".to_string(), json!(25)); // Valid
	data.insert("email".to_string(), json!("invalid")); // Invalid

	let result = validate_fields(&data, &validators);
	assert!(result.is_err());

	match result {
		Err(ValidationError::FieldError { field, message }) => {
			assert_eq!(field, "email");
			assert_eq!(message, "Invalid email format");
		}
		_ => panic!("Expected single FieldError"),
	}
}
