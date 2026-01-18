//! SerializerMethodField - Computed field based on serializer methods
//!
//! This module provides `SerializerMethodField`, which allows you to add custom
//! computed fields to serializers by defining methods that calculate the field value.
//! Inspired by Django REST Framework's SerializerMethodField.

use serde_json::Value;
use std::collections::HashMap;

/// A field that gets its value by calling a method on the serializer.
///
/// This is useful for adding computed or derived fields to your serialized output
/// that don't directly correspond to model fields.
///
/// # Examples
///
/// ```
/// use reinhardt_rest::serializers::SerializerMethodField;
/// use serde_json::{json, Value};
/// use std::collections::HashMap;
///
/// // Define a method context with computed values
/// let mut context = HashMap::new();
/// context.insert("full_name".to_string(), json!("John Doe"));
///
/// let field = SerializerMethodField::new("full_name");
/// // Verify the field retrieves the correct value from context
/// let value = field.get_value(&context).unwrap();
/// assert_eq!(value, json!("John Doe"));
/// ```
#[derive(Debug, Clone)]
pub struct SerializerMethodField {
	/// Name of the method to call (and the field name in output)
	pub method_name: String,

	/// Optional custom method name (if different from field name)
	pub custom_method_name: Option<String>,

	/// Whether this field is read-only (method fields are always read-only)
	pub read_only: bool,
}

impl SerializerMethodField {
	/// Create a new SerializerMethodField
	///
	/// # Arguments
	///
	/// * `method_name` - The name of the method to call to get the field value
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::serializers::SerializerMethodField;
	///
	/// let field = SerializerMethodField::new("get_full_name");
	/// // Verify the field is created successfully
	/// let _: SerializerMethodField = field;
	/// ```
	pub fn new(method_name: impl Into<String>) -> Self {
		Self {
			method_name: method_name.into(),
			custom_method_name: None,
			read_only: true,
		}
	}

	/// Set a custom method name different from the field name
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::serializers::SerializerMethodField;
	///
	/// let field = SerializerMethodField::new("full_name")
	///     .method_name("compute_full_name");
	/// // Verify the custom method name is set correctly
	/// assert_eq!(field.get_method_name(), "compute_full_name");
	/// ```
	pub fn method_name(mut self, name: impl Into<String>) -> Self {
		self.custom_method_name = Some(name.into());
		self
	}

	/// Get the value from the method context
	///
	/// # Arguments
	///
	/// * `context` - A HashMap containing pre-computed method values
	///
	/// # Returns
	///
	/// The computed value as a `serde_json::Value`
	pub fn get_value(&self, context: &HashMap<String, Value>) -> Result<Value, MethodFieldError> {
		let lookup_name = self
			.custom_method_name
			.as_ref()
			.unwrap_or(&self.method_name);

		context
			.get(lookup_name)
			.cloned()
			.ok_or_else(|| MethodFieldError::MethodNotFound(lookup_name.clone()))
	}

	/// Get the actual method name to use for lookup
	pub fn get_method_name(&self) -> &str {
		self.custom_method_name
			.as_ref()
			.unwrap_or(&self.method_name)
	}
}

/// Error type for method field operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum MethodFieldError {
	/// Method was not found in the context
	#[error("Method '{0}' not found in serializer context")]
	MethodNotFound(String),

	/// Error computing method value
	#[error("Error computing method value: {0}")]
	ComputationError(String),
}

/// Trait for serializers that support method fields
///
/// Implement this trait to provide method field computation capabilities
/// to your serializer.
pub trait MethodFieldProvider {
	/// Compute all method field values for the given instance
	///
	/// # Arguments
	///
	/// * `instance` - The instance to compute method values for
	///
	/// # Returns
	///
	/// A HashMap mapping method names to their computed values
	fn compute_method_fields(&self, instance: &Value) -> HashMap<String, Value>;

	/// Compute a specific method field value
	///
	/// # Arguments
	///
	/// * `method_name` - The name of the method to compute
	/// * `instance` - The instance to compute the value for
	///
	/// # Returns
	///
	/// The computed value
	fn compute_method(&self, method_name: &str, instance: &Value) -> Option<Value>;
}

/// Helper struct for building serializers with method fields
#[derive(Debug, Clone)]
pub struct MethodFieldRegistry {
	/// Registered method fields
	fields: HashMap<String, SerializerMethodField>,
}

impl MethodFieldRegistry {
	/// Create a new method field registry
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::serializers::method_field::MethodFieldRegistry;
	///
	/// let registry = MethodFieldRegistry::new();
	/// // Verify the registry is created successfully
	/// let _: MethodFieldRegistry = registry;
	/// ```
	pub fn new() -> Self {
		Self {
			fields: HashMap::new(),
		}
	}

	/// Register a method field
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::serializers::{SerializerMethodField, method_field::MethodFieldRegistry};
	///
	/// let mut registry = MethodFieldRegistry::new();
	/// let field = SerializerMethodField::new("full_name");
	/// registry.register("full_name", field);
	/// // Verify the field is registered successfully
	/// assert!(registry.contains("full_name"));
	/// ```
	pub fn register(&mut self, name: impl Into<String>, field: SerializerMethodField) {
		self.fields.insert(name.into(), field);
	}

	/// Get a registered method field
	pub fn get(&self, name: &str) -> Option<&SerializerMethodField> {
		self.fields.get(name)
	}

	/// Get all registered method fields
	pub fn all(&self) -> &HashMap<String, SerializerMethodField> {
		&self.fields
	}

	/// Check if a field is registered
	pub fn contains(&self, name: &str) -> bool {
		self.fields.contains_key(name)
	}
}

impl Default for MethodFieldRegistry {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn test_serializer_method_field_new() {
		let field = SerializerMethodField::new("get_full_name");
		assert_eq!(field.method_name, "get_full_name");
		assert!(field.custom_method_name.is_none());
		assert!(field.read_only);
	}

	#[test]
	fn test_serializer_method_field_custom_method_name() {
		let field = SerializerMethodField::new("full_name").method_name("compute_full_name");

		assert_eq!(field.method_name, "full_name");
		assert_eq!(
			field.custom_method_name,
			Some("compute_full_name".to_string())
		);
		assert_eq!(field.get_method_name(), "compute_full_name");
	}

	#[test]
	fn test_get_value_success() {
		let mut context = HashMap::new();
		context.insert("full_name".to_string(), json!("John Doe"));

		let field = SerializerMethodField::new("full_name");
		let value = field.get_value(&context).unwrap();

		assert_eq!(value, json!("John Doe"));
	}

	#[test]
	fn test_get_value_with_custom_method() {
		let mut context = HashMap::new();
		context.insert("compute_name".to_string(), json!("Jane Smith"));

		let field = SerializerMethodField::new("full_name").method_name("compute_name");
		let value = field.get_value(&context).unwrap();

		assert_eq!(value, json!("Jane Smith"));
	}

	#[test]
	fn test_get_value_method_not_found() {
		let context = HashMap::new();
		let field = SerializerMethodField::new("missing_method");

		let result = field.get_value(&context);
		assert!(result.is_err());

		if let Err(MethodFieldError::MethodNotFound(name)) = result {
			assert_eq!(name, "missing_method");
		} else {
			panic!("Expected MethodNotFound error");
		}
	}

	#[test]
	fn test_method_field_registry() {
		let mut registry = MethodFieldRegistry::new();

		let field1 = SerializerMethodField::new("full_name");
		let field2 = SerializerMethodField::new("email");

		registry.register("full_name", field1);
		registry.register("email", field2);

		assert!(registry.contains("full_name"));
		assert!(registry.contains("email"));
		assert!(!registry.contains("nonexistent"));

		let retrieved = registry.get("full_name").unwrap();
		assert_eq!(retrieved.method_name, "full_name");
	}

	#[test]
	fn test_method_field_with_complex_value() {
		let mut context = HashMap::new();
		context.insert(
			"user_stats".to_string(),
			json!({
				"post_count": 42,
				"follower_count": 128,
				"engagement_rate": 0.15
			}),
		);

		let field = SerializerMethodField::new("user_stats");
		let value = field.get_value(&context).unwrap();

		assert_eq!(value["post_count"], 42);
		assert_eq!(value["follower_count"], 128);
	}
}
