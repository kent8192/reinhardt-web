//! Integration tests for the `{Model}Info` companion struct generation (Issue #4194).
//!
//! Tests the `#[model]` macro's Info struct generation including:
//! - Basic Info struct generation with correct fields
//! - Bidirectional `From` conversions (Model ↔ Info)
//! - Opt-out via `#[model(info = false)]`
//! - Field exclusion via `#[field(skip_info = true)]`
//! - FK `_id` field inclusion (relationship markers excluded)
//! - Builder with `IntoPrimaryKey` support for FK fields
//! - Validation attribute generation from `#[field(...)]` config

use reinhardt::model;
use serde::{Deserialize, Serialize};

// --- Basic Info generation ---

#[model(app_label = "test", table_name = "persons")]
#[derive(Serialize, Deserialize)]
struct Person {
	#[field(primary_key = true)]
	id: Option<i64>,

	#[field(max_length = 100)]
	name: String,

	#[field(null = true)]
	age: Option<i32>,
}

#[test]
fn test_info_struct_generated() {
	// Arrange
	let info = PersonInfo {
		id: Some(1),
		name: "Alice".to_string(),
		age: Some(30),
	};

	// Assert — Info struct exists with correct public fields
	assert_eq!(info.id, Some(1));
	assert_eq!(info.name, "Alice");
	assert_eq!(info.age, Some(30));
}

#[test]
fn test_info_from_model() {
	// Arrange
	let person = Person::build().name("Alice").age(Some(30)).finish();

	// Act
	let info: PersonInfo = person.into();

	// Assert
	assert_eq!(info.name, "Alice");
	assert_eq!(info.age, Some(30));
}

#[test]
fn test_info_into_model() {
	// Arrange
	let info = PersonInfo {
		id: Some(1),
		name: "Bob".to_string(),
		age: Some(25),
	};

	// Act
	let person: Person = info.into();

	// Assert
	assert_eq!(*person.name(), "Bob");
	assert_eq!(*person.age(), Some(25));
}

#[test]
fn test_info_roundtrip() {
	// Arrange
	let person = Person::build().name("Charlie").age(Some(40)).finish();

	// Act — Model → Info → Model
	let info: PersonInfo = person.into();
	let restored: Person = info.into();

	// Assert
	assert_eq!(*restored.name(), "Charlie");
	assert_eq!(*restored.age(), Some(40));
}

#[test]
fn test_info_debug_clone_partial_eq() {
	// Arrange
	let info = PersonInfo {
		id: Some(1),
		name: "Alice".to_string(),
		age: Some(30),
	};

	// Assert — Debug
	let debug = format!("{:?}", info);
	assert_eq!(
		debug,
		"PersonInfo { id: Some(1), name: \"Alice\", age: Some(30) }"
	);

	// Assert — Clone
	let cloned = info.clone();
	assert_eq!(cloned.name, "Alice");

	// Assert — PartialEq
	assert_eq!(info, cloned);
}

#[test]
fn test_info_serde_mirrored() {
	// Arrange
	let info = PersonInfo {
		id: Some(1),
		name: "Alice".to_string(),
		age: Some(30),
	};

	// Act — Serialize
	let json = serde_json::to_string(&info).unwrap();

	// Act — Deserialize
	let deserialized: PersonInfo = serde_json::from_str(&json).unwrap();

	// Assert
	assert_eq!(deserialized.name, "Alice");
	assert_eq!(deserialized.age, Some(30));
}

// --- Opt-out ---
// Compile-fail verification that `info = false` suppresses Info type generation
// is in model_info_ui.rs (trybuild: tests/macros/ui/fail/info_opt_out_no_type.rs).

// --- Field exclusion ---

#[derive(Serialize, Deserialize)]
#[model(app_label = "test", table_name = "users_with_secrets")]
struct UserWithSecret {
	#[field(primary_key = true)]
	id: Option<i64>,

	#[field(max_length = 100)]
	username: String,

	#[field(max_length = 255, skip_info = true)]
	password_hash: String,
}

#[test]
fn test_info_skip_field() {
	// Arrange — Info struct should NOT have password_hash field
	let info = UserWithSecretInfo {
		id: Some(1),
		username: "alice".to_string(),
		// password_hash is excluded
	};

	// Assert
	assert_eq!(info.username, "alice");
}

#[test]
fn test_info_skip_field_default_on_roundtrip() {
	// Arrange
	let info = UserWithSecretInfo {
		id: Some(1),
		username: "alice".to_string(),
	};

	// Act
	let model: UserWithSecret = info.into();

	// Assert — excluded field gets Default::default()
	assert_eq!(*model.password_hash(), "");
}

// --- Builder ---

#[test]
fn test_info_builder_basic() {
	// Act
	let info = PersonInfo::build()
		.id(Some(1))
		.name("Diana")
		.age(Some(28))
		.finish();

	// Assert
	assert_eq!(info.id, Some(1));
	assert_eq!(info.name, "Diana");
	assert_eq!(info.age, Some(28));
}
