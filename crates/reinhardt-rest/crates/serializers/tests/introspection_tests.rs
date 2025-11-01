//! Integration tests for field introspection in ModelSerializer

use reinhardt_orm::Model;
use reinhardt_serializers::ModelSerializer;
use reinhardt_serializers::introspection::{FieldInfo, FieldIntrospector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
	id: Option<i64>,
	username: String,
	email: String,
	password: String,
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
fn test_with_introspector() {
	let mut introspector = FieldIntrospector::new();
	introspector.register_field(FieldInfo::new("id", "i64").optional().primary_key());
	introspector.register_field(FieldInfo::new("username", "String"));
	introspector.register_field(FieldInfo::new("email", "String"));
	introspector.register_field(FieldInfo::new("password", "String"));

	let serializer = ModelSerializer::<User>::new().with_introspector(introspector);

	assert!(serializer.introspector().is_some());
}

#[test]
fn test_field_names_from_introspector() {
	let mut introspector = FieldIntrospector::new();
	introspector.register_field(FieldInfo::new("id", "i64"));
	introspector.register_field(FieldInfo::new("username", "String"));
	introspector.register_field(FieldInfo::new("email", "String"));

	let serializer = ModelSerializer::<User>::new().with_introspector(introspector);

	let fields = serializer.field_names();
	assert_eq!(fields.len(), 3);
	assert!(fields.contains(&"id".to_string()));
	assert!(fields.contains(&"username".to_string()));
	assert!(fields.contains(&"email".to_string()));
}

#[test]
fn test_field_names_from_meta_when_no_introspector() {
	let serializer =
		ModelSerializer::<User>::new().with_fields(vec!["id".to_string(), "username".to_string()]);

	let fields = serializer.field_names();
	assert_eq!(fields.len(), 2);
	assert!(fields.contains(&"id".to_string()));
	assert!(fields.contains(&"username".to_string()));
}

#[test]
fn test_required_fields() {
	let mut introspector = FieldIntrospector::new();
	introspector.register_field(FieldInfo::new("id", "i64").optional());
	introspector.register_field(FieldInfo::new("username", "String"));
	introspector.register_field(FieldInfo::new("email", "String"));

	let serializer = ModelSerializer::<User>::new().with_introspector(introspector);

	let required = serializer.required_fields();
	assert_eq!(required.len(), 2);
	assert!(required.iter().any(|f| f.name == "username"));
	assert!(required.iter().any(|f| f.name == "email"));
}

#[test]
fn test_optional_fields() {
	let mut introspector = FieldIntrospector::new();
	introspector.register_field(FieldInfo::new("id", "i64").optional());
	introspector.register_field(FieldInfo::new("username", "String"));
	introspector.register_field(FieldInfo::new("email", "String").optional());

	let serializer = ModelSerializer::<User>::new().with_introspector(introspector);

	let optional = serializer.optional_fields();
	assert_eq!(optional.len(), 2);
	assert!(optional.iter().any(|f| f.name == "id"));
	assert!(optional.iter().any(|f| f.name == "email"));
}

#[test]
fn test_primary_key_field() {
	let mut introspector = FieldIntrospector::new();
	introspector.register_field(FieldInfo::new("id", "i64").optional().primary_key());
	introspector.register_field(FieldInfo::new("username", "String"));

	let serializer = ModelSerializer::<User>::new().with_introspector(introspector);

	let pk = serializer.primary_key_field();
	assert!(pk.is_some());
	assert_eq!(pk.unwrap().name, "id");
	assert!(pk.unwrap().is_primary_key);
}

#[test]
fn test_introspector_with_meta_filtering() {
	let mut introspector = FieldIntrospector::new();
	introspector.register_field(FieldInfo::new("id", "i64"));
	introspector.register_field(FieldInfo::new("username", "String"));
	introspector.register_field(FieldInfo::new("email", "String"));
	introspector.register_field(FieldInfo::new("password", "String"));

	// Use meta to exclude password field
	let serializer = ModelSerializer::<User>::new()
		.with_introspector(introspector)
		.with_exclude(vec!["password".to_string()]);

	// Field names come from introspector
	let fields = serializer.field_names();
	assert_eq!(fields.len(), 4); // Introspector returns all 4 fields

	// But meta configuration can be used for filtering during serialization
	assert!(serializer.meta().is_field_included("id"));
	assert!(serializer.meta().is_field_included("username"));
	assert!(serializer.meta().is_field_included("email"));
	assert!(!serializer.meta().is_field_included("password"));
}

#[test]
fn test_introspector_with_read_only_fields() {
	let mut introspector = FieldIntrospector::new();
	introspector.register_field(FieldInfo::new("id", "i64").optional().primary_key());
	introspector.register_field(FieldInfo::new("username", "String"));
	introspector.register_field(FieldInfo::new("email", "String"));

	let serializer = ModelSerializer::<User>::new()
		.with_introspector(introspector)
		.with_read_only_fields(vec!["id".to_string()]);

	assert!(serializer.meta().is_read_only("id"));
	assert!(!serializer.meta().is_read_only("username"));
	assert!(!serializer.meta().is_read_only("email"));
}

#[test]
fn test_introspector_with_write_only_fields() {
	let mut introspector = FieldIntrospector::new();
	introspector.register_field(FieldInfo::new("username", "String"));
	introspector.register_field(FieldInfo::new("email", "String"));
	introspector.register_field(FieldInfo::new("password", "String"));

	let serializer = ModelSerializer::<User>::new()
		.with_introspector(introspector)
		.with_write_only_fields(vec!["password".to_string()]);

	assert!(serializer.meta().is_write_only("password"));
	assert!(!serializer.meta().is_write_only("username"));
	assert!(!serializer.meta().is_write_only("email"));
}

#[test]
fn test_no_introspector_no_fields_returns_empty() {
	let serializer = ModelSerializer::<User>::new();

	let fields = serializer.field_names();
	assert_eq!(fields.len(), 0);

	let required = serializer.required_fields();
	assert_eq!(required.len(), 0);

	let optional = serializer.optional_fields();
	assert_eq!(optional.len(), 0);

	let pk = serializer.primary_key_field();
	assert!(pk.is_none());
}
