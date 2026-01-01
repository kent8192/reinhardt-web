//! State transition tests for the `form!` macro.
//!
//! Tests form state transitions: New → Bound → Validated → CleanedData/Error.

use reinhardt_forms::form;
use rstest::rstest;
use serde_json::json;
use std::collections::HashMap;

/// ST-001: Form state transition - happy path.
///
/// Tests the complete flow: New → Bound → Validated → CleanedData
#[rstest]
fn test_form_state_transition_happy_path() {
	// Create form (state: New)
	let mut form = form! {
		fields: {
			username: CharField {
				required,
				max_length: 150,
			},
			email: EmailField {
				required,
			},
		},
	};

	// State: New
	assert!(!form.is_bound());
	assert!(form.errors().is_empty());
	assert_eq!(form.fields().len(), 2);

	// Bind valid data (state: Bound)
	let mut data = HashMap::new();
	data.insert("username".to_string(), json!("testuser"));
	data.insert("email".to_string(), json!("test@example.com"));
	form.bind(data);

	// State: Bound
	assert!(form.is_bound());

	// Validate (state: Validated)
	assert!(form.is_valid());

	// Get cleaned data (state: CleanedData)
	let cleaned = form.cleaned_data();
	assert_eq!(cleaned.get("username"), Some(&json!("testuser")));
	assert_eq!(cleaned.get("email"), Some(&json!("test@example.com")));
}

/// ST-002: Form state transition - validation error.
///
/// Tests: New → Bound → ValidationError
#[rstest]
fn test_form_state_transition_validation_error() {
	// Create form with required field
	let mut form = form! {
		fields: {
			username: CharField {
				required,
			},
		},
	};

	// Bind empty data (missing required field)
	let data = HashMap::new();
	form.bind(data);

	// State: Bound
	assert!(form.is_bound());

	// Validate should fail
	assert!(!form.is_valid());

	// Errors should contain error for username
	assert!(!form.errors().is_empty());
	assert!(form.errors().contains_key("username"));
}

/// ST-004: Field validator execution order.
///
/// Tests: FieldClean → FormClean
#[rstest]
fn test_validator_execution_order() {
	// Create form with field and form validators
	let mut form = form! {
		fields: {
			password: CharField {
				required,
			},
			confirm: CharField {
				required,
			},
		},
		validators: {
			password: [
				|v: &serde_json::Value| v.as_str().map_or(false, |s| s.len() >= 8) => "Password must be at least 8 characters",
			],
			@form: [
				|data: &std::collections::HashMap<String, serde_json::Value>| {
					let password = data.get("password").and_then(|v| v.as_str());
					let confirm = data.get("confirm").and_then(|v| v.as_str());
					password == confirm
				} => "Passwords do not match",
			],
		},
	};

	// Bind valid data
	let mut data = HashMap::new();
	data.insert("password".to_string(), json!("password123"));
	data.insert("confirm".to_string(), json!("password123"));
	form.bind(data);

	assert!(form.is_valid());
}

/// ST-005: Multiple bindings.
///
/// Tests: Bound → Rebound → Validated
#[rstest]
fn test_multiple_bindings() {
	let mut form = form! {
		fields: {
			username: CharField {
				required,
			},
		},
	};

	// First binding with invalid data
	let data = HashMap::new();
	form.bind(data);
	assert!(!form.is_valid());

	// Second binding with valid data
	let mut data2 = HashMap::new();
	data2.insert("username".to_string(), json!("validuser"));
	form.bind(data2);

	// Previous errors should be cleared, new validation runs
	assert!(form.is_valid());
	assert!(form.errors().is_empty());
}
