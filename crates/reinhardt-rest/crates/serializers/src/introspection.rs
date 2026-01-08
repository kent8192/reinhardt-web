//! Field introspection for automatic serializer field generation
//!
//! This module provides utilities to introspect model types and automatically
//! generate serializer field configurations.

use std::collections::HashMap;

/// Represents a field's metadata
#[derive(Debug, Clone, PartialEq)]
pub struct FieldInfo {
	/// Field name
	pub name: String,
	/// Rust type name
	pub type_name: String,
	/// Whether the field is optional (`Option<T>`)
	pub is_optional: bool,
	/// Whether the field is a collection (`Vec<T>`)
	pub is_collection: bool,
	/// Whether this is the primary key
	pub is_primary_key: bool,
}

impl FieldInfo {
	/// Create a new field info
	pub fn new(name: impl Into<String>, type_name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			type_name: type_name.into(),
			is_optional: false,
			is_collection: false,
			is_primary_key: false,
		}
	}

	/// Mark field as optional
	pub fn optional(mut self) -> Self {
		self.is_optional = true;
		self
	}

	/// Mark field as collection
	pub fn collection(mut self) -> Self {
		self.is_collection = true;
		self
	}

	/// Mark field as primary key
	pub fn primary_key(mut self) -> Self {
		self.is_primary_key = true;
		self
	}
}

/// Field introspector for extracting model field information
///
/// # Manual Registration Approach
///
/// The current implementation requires explicit `register_field()` calls.
/// This provides full control over introspection behavior and field metadata.
///
/// # Examples
///
/// ```
/// use reinhardt_serializers::introspection::{FieldInfo, FieldIntrospector};
///
/// let mut introspector = FieldIntrospector::new();
///
/// // Register fields manually
/// introspector.register_field(
///     FieldInfo::new("id", "i64").optional().primary_key()
/// );
/// introspector.register_field(
///     FieldInfo::new("username", "String")
/// );
///
/// let fields = introspector.get_fields();
/// assert_eq!(fields.len(), 2);
/// ```
///
/// # Future Enhancement
///
/// A proc-macro implementation is planned to automate field registration:
///
/// ```rust,no_run,ignore
/// #[derive(Introspectable)]
/// struct User {
///     id: i64,
///     username: String,
/// }
/// ```
///
/// This would eliminate manual registration while maintaining the same functionality.
pub struct FieldIntrospector {
	fields: Vec<FieldInfo>,
	field_map: HashMap<String, FieldInfo>,
}

impl FieldIntrospector {
	/// Create a new field introspector
	pub fn new() -> Self {
		Self {
			fields: Vec::new(),
			field_map: HashMap::new(),
		}
	}

	/// Register a field
	pub fn register_field(&mut self, field: FieldInfo) {
		self.field_map.insert(field.name.clone(), field.clone());
		self.fields.push(field);
	}

	/// Get all registered fields
	pub fn get_fields(&self) -> &[FieldInfo] {
		&self.fields
	}

	/// Get field names only
	pub fn field_names(&self) -> Vec<String> {
		self.fields.iter().map(|f| f.name.clone()).collect()
	}

	/// Get a specific field by name
	pub fn get_field(&self, name: &str) -> Option<&FieldInfo> {
		self.field_map.get(name)
	}

	/// Check if a field exists
	pub fn has_field(&self, name: &str) -> bool {
		self.field_map.contains_key(name)
	}

	/// Get optional fields
	pub fn optional_fields(&self) -> Vec<&FieldInfo> {
		self.fields.iter().filter(|f| f.is_optional).collect()
	}

	/// Get required fields (non-optional)
	pub fn required_fields(&self) -> Vec<&FieldInfo> {
		self.fields.iter().filter(|f| !f.is_optional).collect()
	}

	/// Get primary key field
	pub fn primary_key_field(&self) -> Option<&FieldInfo> {
		self.fields.iter().find(|f| f.is_primary_key)
	}

	/// Get collection fields
	pub fn collection_fields(&self) -> Vec<&FieldInfo> {
		self.fields.iter().filter(|f| f.is_collection).collect()
	}
}

impl Default for FieldIntrospector {
	fn default() -> Self {
		Self::new()
	}
}

/// Type mapping utilities for converting Rust types to serializer field types
pub struct TypeMapper;

impl TypeMapper {
	/// Check if a type is a string type
	pub fn is_string_type(type_name: &str) -> bool {
		matches!(type_name, "String" | "&str" | "str")
	}

	/// Check if a type is an integer type
	pub fn is_integer_type(type_name: &str) -> bool {
		matches!(
			type_name,
			"i8" | "i16"
				| "i32" | "i64"
				| "i128" | "isize"
				| "u8" | "u16"
				| "u32" | "u64"
				| "u128" | "usize"
		)
	}

	/// Check if a type is a float type
	pub fn is_float_type(type_name: &str) -> bool {
		matches!(type_name, "f32" | "f64")
	}

	/// Check if a type is a boolean type
	pub fn is_boolean_type(type_name: &str) -> bool {
		type_name == "bool"
	}

	/// Check if a type is numeric (integer or float)
	pub fn is_numeric_type(type_name: &str) -> bool {
		Self::is_integer_type(type_name) || Self::is_float_type(type_name)
	}

	/// Extract inner type from `Option<T>`
	pub fn extract_option_type(type_name: &str) -> Option<String> {
		if type_name.starts_with("Option<") && type_name.ends_with('>') {
			let inner = &type_name[7..type_name.len() - 1];
			Some(inner.to_string())
		} else {
			None
		}
	}

	/// Extract inner type from `Vec<T>`
	pub fn extract_vec_type(type_name: &str) -> Option<String> {
		if type_name.starts_with("Vec<") && type_name.ends_with('>') {
			let inner = &type_name[4..type_name.len() - 1];
			Some(inner.to_string())
		} else {
			None
		}
	}

	/// Check if type is `Option<T>`
	pub fn is_option_type(type_name: &str) -> bool {
		type_name.starts_with("Option<")
	}

	/// Check if type is `Vec<T>`
	pub fn is_vec_type(type_name: &str) -> bool {
		type_name.starts_with("Vec<")
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_field_info_creation() {
		let field = FieldInfo::new("username", "String");
		assert_eq!(field.name, "username");
		assert_eq!(field.type_name, "String");
		assert!(!field.is_optional);
		assert!(!field.is_collection);
		assert!(!field.is_primary_key);
	}

	#[test]
	fn test_field_info_builders() {
		let field = FieldInfo::new("id", "i64").optional().primary_key();

		assert!(field.is_optional);
		assert!(field.is_primary_key);
		assert!(!field.is_collection);
	}

	#[test]
	fn test_field_introspector_registration() {
		let mut introspector = FieldIntrospector::new();

		introspector.register_field(FieldInfo::new("id", "i64").optional());
		introspector.register_field(FieldInfo::new("username", "String"));

		assert_eq!(introspector.get_fields().len(), 2);
		assert_eq!(introspector.field_names(), vec!["id", "username"]);
	}

	#[test]
	fn test_field_introspector_get_field() {
		let mut introspector = FieldIntrospector::new();
		introspector.register_field(FieldInfo::new("username", "String"));

		let field = introspector.get_field("username");
		assert!(field.is_some());
		assert_eq!(field.unwrap().name, "username");

		assert!(introspector.get_field("nonexistent").is_none());
	}

	#[test]
	fn test_field_introspector_optional_fields() {
		let mut introspector = FieldIntrospector::new();
		introspector.register_field(FieldInfo::new("id", "i64").optional());
		introspector.register_field(FieldInfo::new("username", "String"));
		introspector.register_field(FieldInfo::new("email", "String").optional());

		let optional = introspector.optional_fields();
		assert_eq!(optional.len(), 2);

		let required = introspector.required_fields();
		assert_eq!(required.len(), 1);
	}

	#[test]
	fn test_field_introspector_primary_key() {
		let mut introspector = FieldIntrospector::new();
		introspector.register_field(FieldInfo::new("id", "i64").primary_key());
		introspector.register_field(FieldInfo::new("username", "String"));

		let pk = introspector.primary_key_field();
		assert!(pk.is_some());
		assert_eq!(pk.unwrap().name, "id");
	}

	#[test]
	fn test_type_mapper_string_types() {
		assert!(TypeMapper::is_string_type("String"));
		assert!(TypeMapper::is_string_type("&str"));
		assert!(TypeMapper::is_string_type("str"));
		assert!(!TypeMapper::is_string_type("i64"));
	}

	#[test]
	fn test_type_mapper_integer_types() {
		assert!(TypeMapper::is_integer_type("i32"));
		assert!(TypeMapper::is_integer_type("i64"));
		assert!(TypeMapper::is_integer_type("u32"));
		assert!(!TypeMapper::is_integer_type("String"));
	}

	#[test]
	fn test_type_mapper_float_types() {
		assert!(TypeMapper::is_float_type("f32"));
		assert!(TypeMapper::is_float_type("f64"));
		assert!(!TypeMapper::is_float_type("i32"));
	}

	#[test]
	fn test_type_mapper_boolean_type() {
		assert!(TypeMapper::is_boolean_type("bool"));
		assert!(!TypeMapper::is_boolean_type("Boolean"));
	}

	#[test]
	fn test_type_mapper_extract_option_type() {
		let inner = TypeMapper::extract_option_type("Option<String>");
		assert_eq!(inner, Some("String".to_string()));

		let inner = TypeMapper::extract_option_type("Option<i64>");
		assert_eq!(inner, Some("i64".to_string()));

		let inner = TypeMapper::extract_option_type("String");
		assert_eq!(inner, None);
	}

	#[test]
	fn test_type_mapper_extract_vec_type() {
		let inner = TypeMapper::extract_vec_type("Vec<String>");
		assert_eq!(inner, Some("String".to_string()));

		let inner = TypeMapper::extract_vec_type("Vec<User>");
		assert_eq!(inner, Some("User".to_string()));

		let inner = TypeMapper::extract_vec_type("String");
		assert_eq!(inner, None);
	}

	#[test]
	fn test_type_mapper_is_option_type() {
		assert!(TypeMapper::is_option_type("Option<String>"));
		assert!(TypeMapper::is_option_type("Option<i64>"));
		assert!(!TypeMapper::is_option_type("String"));
	}

	#[test]
	fn test_type_mapper_is_vec_type() {
		assert!(TypeMapper::is_vec_type("Vec<String>"));
		assert!(TypeMapper::is_vec_type("Vec<User>"));
		assert!(!TypeMapper::is_vec_type("String"));
	}
}
