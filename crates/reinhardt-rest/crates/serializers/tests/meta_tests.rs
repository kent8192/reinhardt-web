/// Integration tests for Meta configuration support in ModelSerializer
use reinhardt_orm::Model;
use reinhardt_serializers::{MetaConfig, ModelSerializer, Serializer};
use serde::{Deserialize, Serialize};

/// Test model: User
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
fn test_model_serializer_default_includes_all_fields() {
	let serializer = ModelSerializer::<User>::new();
	let user = User {
		id: Some(1),
		username: "alice".to_string(),
		email: "alice@example.com".to_string(),
		password: "secret".to_string(),
	};

	let json = serializer.serialize(&user).unwrap();
	let deserialized: serde_json::Value = serde_json::from_str(&json).unwrap();

	assert!(deserialized.get("id").is_some());
	assert!(deserialized.get("username").is_some());
	assert!(deserialized.get("email").is_some());
	assert!(deserialized.get("password").is_some());
}

#[test]
fn test_model_serializer_with_fields() {
	let serializer =
		ModelSerializer::<User>::new().with_fields(vec!["id".to_string(), "username".to_string()]);

	let meta = serializer.meta();
	assert!(meta.is_field_included("id"));
	assert!(meta.is_field_included("username"));
	assert!(!meta.is_field_included("email"));
	assert!(!meta.is_field_included("password"));
}

#[test]
fn test_model_serializer_with_exclude() {
	let serializer = ModelSerializer::<User>::new().with_exclude(vec!["password".to_string()]);

	let meta = serializer.meta();
	assert!(meta.is_field_included("id"));
	assert!(meta.is_field_included("username"));
	assert!(meta.is_field_included("email"));
	assert!(!meta.is_field_included("password"));
}

#[test]
fn test_model_serializer_with_read_only_fields() {
	let serializer = ModelSerializer::<User>::new().with_read_only_fields(vec!["id".to_string()]);

	let meta = serializer.meta();
	assert!(meta.is_read_only("id"));
	assert!(!meta.is_read_only("username"));
	assert!(!meta.is_read_only("email"));
}

#[test]
fn test_model_serializer_with_write_only_fields() {
	let serializer =
		ModelSerializer::<User>::new().with_write_only_fields(vec!["password".to_string()]);

	let meta = serializer.meta();
	assert!(meta.is_write_only("password"));
	assert!(!meta.is_write_only("username"));
	assert!(!meta.is_write_only("email"));
}

#[test]
fn test_model_serializer_builder_pattern() {
	let serializer = ModelSerializer::<User>::new()
		.with_fields(vec![
			"id".to_string(),
			"username".to_string(),
			"email".to_string(),
		])
		.with_read_only_fields(vec!["id".to_string()])
		.with_write_only_fields(vec!["password".to_string()]);

	let meta = serializer.meta();

	// Check field inclusion
	assert!(meta.is_field_included("id"));
	assert!(meta.is_field_included("username"));
	assert!(meta.is_field_included("email"));
	assert!(!meta.is_field_included("password")); // excluded by fields

	// Check read-only
	assert!(meta.is_read_only("id"));
	assert!(!meta.is_read_only("username"));

	// Check write-only
	assert!(meta.is_write_only("password"));
	assert!(!meta.is_write_only("email"));
}

#[test]
fn test_model_serializer_fields_and_exclude_combination() {
	// When both fields and exclude are specified, exclude takes precedence
	let serializer = ModelSerializer::<User>::new()
		.with_fields(vec![
			"id".to_string(),
			"username".to_string(),
			"password".to_string(),
		])
		.with_exclude(vec!["password".to_string()]);

	let meta = serializer.meta();

	assert!(meta.is_field_included("id"));
	assert!(meta.is_field_included("username"));
	assert!(!meta.is_field_included("password")); // excluded
}

#[test]
fn test_meta_config_effective_fields() {
	let all_fields = vec![
		"id".to_string(),
		"username".to_string(),
		"email".to_string(),
		"password".to_string(),
	];

	let config = MetaConfig::new()
		.with_fields(vec![
			"id".to_string(),
			"username".to_string(),
			"email".to_string(),
		])
		.with_exclude(vec!["email".to_string()]);

	let effective = config.effective_fields(&all_fields);

	assert_eq!(effective.len(), 2);
	assert!(effective.contains("id"));
	assert!(effective.contains("username"));
	assert!(!effective.contains("email")); // excluded
	assert!(!effective.contains("password")); // not in fields
}

#[test]
fn test_serializer_with_meta_preserves_functionality() {
	// Ensure that adding Meta doesn't break basic serialization
	let serializer =
		ModelSerializer::<User>::new().with_fields(vec!["id".to_string(), "username".to_string()]);

	let user = User {
		id: Some(1),
		username: "alice".to_string(),
		email: "alice@example.com".to_string(),
		password: "secret".to_string(),
	};

	// Serialization should still work
	let json = serializer.serialize(&user).unwrap();
	assert!(json.contains("alice"));

	// Deserialization should still work
	let user_json =
		r#"{"id":1,"username":"bob","email":"bob@example.com","password":"secret"}"#.to_string();
	let deserialized = serializer.deserialize(&user_json).unwrap();
	assert_eq!(deserialized.username, "bob");
}

#[test]
fn test_meta_config_accessors() {
	let config = MetaConfig::new()
		.with_fields(vec!["id".to_string(), "username".to_string()])
		.with_exclude(vec!["password".to_string()])
		.with_read_only_fields(vec!["id".to_string()])
		.with_write_only_fields(vec!["secret".to_string()]);

	// Test accessors
	assert_eq!(config.fields().unwrap().len(), 2);
	assert_eq!(config.excluded_fields().len(), 1);
	assert_eq!(config.read_only_fields().len(), 1);
	assert_eq!(config.write_only_fields().len(), 1);

	assert!(config.fields().unwrap().contains(&"id".to_string()));
	assert!(config.fields().unwrap().contains(&"username".to_string()));
	assert!(config.excluded_fields().contains(&"password".to_string()));
	assert!(config.read_only_fields().contains(&"id".to_string()));
	assert!(config.write_only_fields().contains(&"secret".to_string()));
}
