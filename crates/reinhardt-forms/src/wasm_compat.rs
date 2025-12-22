//! WASM Compatibility Layer for Forms (Week 5 Day 1-2)
//!
//! This module provides serializable metadata structures that allow Django-style
//! Forms to be rendered on the client-side (WASM) without requiring the full
//! `Form` struct with its trait objects and non-serializable closures.
//!
//! ## Architecture
//!
//! The metadata extraction follows this pattern:
//!
//! ```text
//! Server-side:                      Client-side (WASM):
//! ┌──────────┐                     ┌──────────────┐
//! │   Form   │ ─ to_metadata() ─> │ FormMetadata │
//! │          │                     │              │
//! │ (traits, │                     │ (plain data, │
//! │ closures)│                     │ serializable)│
//! └──────────┘                     └──────────────┘
//!                                         │
//!                                         ▼
//!                                  ┌──────────────┐
//!                                  │FormComponent │
//!                                  │  (WASM UI)   │
//!                                  └──────────────┘
//! ```
//!
//! ## Example
//!
//! ```
//! use reinhardt_forms::{Form, CharField, Field};
//! use reinhardt_forms::wasm_compat::{FormMetadata, FormExt};
//!
//! // Server-side: Create form
//! let mut form = Form::new();
//! form.add_field(Box::new(CharField::new("username".to_string())));
//! form.enable_csrf(Some("secret".to_string()));
//!
//! // Extract metadata for client
//! let metadata: FormMetadata = form.to_metadata();
//!
//! // Serialize and send to WASM
//! let json = serde_json::to_string(&metadata).unwrap();
//! ```

use crate::field::Widget;
use crate::form::Form;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Serializable form metadata for client-side rendering (Week 5 Day 1)
///
/// This structure contains all information needed to render a form on the
/// client-side without requiring the full `Form` struct with its trait objects.
///
/// ## Fields
///
/// - `fields`: Metadata for each form field
/// - `initial`: Initial values for the form (form-level)
/// - `csrf_token`: CSRF token for security (if enabled)
/// - `prefix`: Field name prefix (for multiple forms on same page)
/// - `is_bound`: Whether the form has been bound with data
/// - `errors`: Validation errors (if any)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormMetadata {
	/// Field metadata list
	pub fields: Vec<FieldMetadata>,

	/// Initial values (form-level)
	pub initial: HashMap<String, serde_json::Value>,

	/// CSRF token (if enabled)
	pub csrf_token: Option<String>,

	/// Field name prefix
	pub prefix: String,

	/// Whether the form has been bound with data
	pub is_bound: bool,

	/// Validation errors (field name -> error messages)
	pub errors: HashMap<String, Vec<String>>,
}

/// Serializable field metadata for client-side rendering (Week 5 Day 1)
///
/// This structure contains all information needed to render a single form field
/// on the client-side.
///
/// ## Fields
///
/// - `name`: Field name (used as form data key)
/// - `label`: Human-readable label (defaults to field name if None)
/// - `required`: Whether the field is required
/// - `help_text`: Help text displayed below the field
/// - `widget`: Widget type for rendering (TextInput, Select, etc.)
/// - `initial`: Initial value for this field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMetadata {
	/// Field name
	pub name: String,

	/// Human-readable label (optional)
	pub label: Option<String>,

	/// Whether the field is required
	pub required: bool,

	/// Help text (optional)
	pub help_text: Option<String>,

	/// Widget type for rendering
	pub widget: Widget,

	/// Initial value (optional)
	pub initial: Option<serde_json::Value>,
}

/// Extension trait for Form to extract metadata (Week 5 Day 1)
///
/// This trait provides the `to_metadata()` method that converts a `Form`
/// into a serializable `FormMetadata` structure.
pub trait FormExt {
	/// Extract serializable metadata from the form
	///
	/// This method creates a `FormMetadata` structure containing all
	/// information needed to render the form on the client-side.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::{Form, CharField, Field};
	/// use reinhardt_forms::wasm_compat::{FormMetadata, FormExt};
	///
	/// let mut form = Form::new();
	/// form.add_field(Box::new(CharField::new("email".to_string())));
	///
	/// let metadata: FormMetadata = form.to_metadata();
	/// assert_eq!(metadata.fields.len(), 1);
	/// assert_eq!(metadata.fields[0].name, "email");
	/// ```
	fn to_metadata(&self) -> FormMetadata;
}

impl FormExt for Form {
	fn to_metadata(&self) -> FormMetadata {
		// Extract field metadata
		let fields = self
			.fields()
			.iter()
			.map(|field| FieldMetadata {
				name: field.name().to_string(),
				label: field.label().map(|s| s.to_string()),
				required: field.required(),
				help_text: field.help_text().map(|s| s.to_string()),
				widget: field.widget().clone(),
				initial: field.initial().cloned(),
			})
			.collect();

		// Extract CSRF token as string
		let csrf_token = self.csrf_token().map(|t| t.token().to_string());

		FormMetadata {
			fields,
			initial: self.initial().clone(),
			csrf_token,
			prefix: self.prefix().to_string(),
			is_bound: self.is_bound(),
			errors: self.errors().clone(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::fields::CharField;

	#[test]
	fn test_form_metadata_extraction() {
		let mut form = Form::new();
		form.add_field(Box::new(CharField::new("username".to_string())));
		form.add_field(Box::new(CharField::new("email".to_string())));

		let metadata = form.to_metadata();

		assert_eq!(metadata.fields.len(), 2);
		assert_eq!(metadata.fields[0].name, "username");
		assert_eq!(metadata.fields[1].name, "email");
		assert!(!metadata.is_bound);
		assert!(metadata.csrf_token.is_none());
	}

	#[test]
	fn test_form_metadata_with_csrf() {
		let mut form = Form::new();
		form.enable_csrf(Some("test-secret".to_string()));
		form.add_field(Box::new(CharField::new("data".to_string())));

		let metadata = form.to_metadata();

		assert!(metadata.csrf_token.is_some());
		assert_eq!(metadata.fields.len(), 1);
	}

	#[test]
	fn test_form_metadata_with_prefix() {
		let mut form = Form::with_prefix("user".to_string());
		form.add_field(Box::new(CharField::new("name".to_string())));

		let metadata = form.to_metadata();

		assert_eq!(metadata.prefix, "user");
		assert_eq!(metadata.fields.len(), 1);
	}

	#[test]
	fn test_form_metadata_serialization() {
		let mut form = Form::new();
		form.add_field(Box::new(CharField::new("test".to_string())));

		let metadata = form.to_metadata();

		// Test JSON serialization
		let json = serde_json::to_string(&metadata).expect("Failed to serialize");
		assert!(json.contains("\"name\":\"test\""));

		// Test deserialization
		let deserialized: FormMetadata =
			serde_json::from_str(&json).expect("Failed to deserialize");
		assert_eq!(deserialized.fields[0].name, "test");
	}

	#[test]
	fn test_field_metadata_with_all_attributes() {
		use crate::fields::CharField;

		let field = CharField::new("bio".to_string())
			.with_label("Biography")
			.with_help_text("Tell us about yourself")
			.required();

		let mut form = Form::new();
		form.add_field(Box::new(field));

		let metadata = form.to_metadata();
		let field_meta = &metadata.fields[0];

		assert_eq!(field_meta.name, "bio");
		assert_eq!(field_meta.label, Some("Biography".to_string()));
		assert_eq!(
			field_meta.help_text,
			Some("Tell us about yourself".to_string())
		);
		assert!(field_meta.required);
	}

	#[test]
	fn test_form_metadata_with_initial_values() {
		use serde_json::json;

		let mut initial = HashMap::new();
		initial.insert("username".to_string(), json!("john_doe"));
		initial.insert("age".to_string(), json!(25));

		let mut form = Form::with_initial(initial);
		form.add_field(Box::new(CharField::new("username".to_string())));

		let metadata = form.to_metadata();

		assert_eq!(metadata.initial.get("username"), Some(&json!("john_doe")));
		assert_eq!(metadata.initial.get("age"), Some(&json!(25)));
	}

	#[test]
	fn test_form_metadata_with_errors() {
		use serde_json::json;

		let mut form = Form::new();
		// Create a required field - empty value should fail validation
		form.add_field(Box::new(CharField::new("email".to_string()).required()));

		// Bind with invalid data to generate errors (empty string for required field)
		let mut data = HashMap::new();
		data.insert("email".to_string(), json!("")); // Empty required field should fail
		form.bind(data);

		// Validate to populate errors
		let is_valid = form.is_valid();

		let metadata = form.to_metadata();

		// Should have validation error for the required email field
		assert!(!is_valid);
		assert!(!metadata.errors.is_empty());
		assert!(metadata.errors.contains_key("email"));
	}
}
