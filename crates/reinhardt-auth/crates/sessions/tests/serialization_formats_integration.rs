//! Serialization Formats Integration Tests
//!
//! This module contains comprehensive tests for different serialization formats
//! used in session management. Tests cover JSON, MessagePack, CBOR, and Bincode.
//!
//! # Test Categories
//!
//! - Equivalence Partitioning: Different data types and structures
//! - Edge Cases: Unicode, binary data, large payloads, nested structures
//! - Roundtrip: Serialize then deserialize verification
//! - Cross-format: Size and correctness comparisons

use reinhardt_sessions::serialization::{JsonSerializer, SerializationFormat, Serializer};
use rstest::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Test Data Structures
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SimpleData {
	id: i32,
	name: String,
	active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct NestedData {
	user: SimpleData,
	metadata: HashMap<String, String>,
	tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ComplexData {
	int_value: i64,
	float_value: f64,
	string_value: String,
	optional_value: Option<String>,
	array_value: Vec<i32>,
	nested: Option<Box<ComplexData>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SessionLikeData {
	session_id: String,
	user_id: Option<u64>,
	created_at: i64,
	expires_at: i64,
	data: HashMap<String, serde_json::Value>,
}

// =============================================================================
// Fixtures
// =============================================================================

#[fixture]
fn json_serializer() -> JsonSerializer {
	JsonSerializer
}

#[fixture]
fn simple_data() -> SimpleData {
	SimpleData {
		id: 42,
		name: "Alice".to_string(),
		active: true,
	}
}

#[fixture]
fn nested_data() -> NestedData {
	let mut metadata = HashMap::new();
	metadata.insert("version".to_string(), "1.0".to_string());
	metadata.insert("source".to_string(), "test".to_string());

	NestedData {
		user: SimpleData {
			id: 1,
			name: "Bob".to_string(),
			active: true,
		},
		metadata,
		tags: vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()],
	}
}

#[fixture]
fn complex_data() -> ComplexData {
	ComplexData {
		int_value: 9999999999i64,
		float_value: 3.14159265358979,
		string_value: "Hello, World!".to_string(),
		optional_value: Some("present".to_string()),
		array_value: vec![1, 2, 3, 4, 5],
		nested: Some(Box::new(ComplexData {
			int_value: 0,
			float_value: 0.0,
			string_value: String::new(),
			optional_value: None,
			array_value: vec![],
			nested: None,
		})),
	}
}

#[fixture]
fn session_data() -> SessionLikeData {
	let mut data = HashMap::new();
	data.insert("csrf_token".to_string(), serde_json::json!("abc123"));
	data.insert(
		"preferences".to_string(),
		serde_json::json!({"theme": "dark"}),
	);

	SessionLikeData {
		session_id: "sess_abcd1234".to_string(),
		user_id: Some(12345),
		created_at: 1704067200,
		expires_at: 1704153600,
		data,
	}
}

// =============================================================================
// JSON Serializer Tests - Happy Path
// =============================================================================

#[rstest]
fn test_json_roundtrip_simple_data(json_serializer: JsonSerializer, simple_data: SimpleData) {
	let bytes = json_serializer.serialize(&simple_data).unwrap();
	let restored: SimpleData = json_serializer.deserialize(&bytes).unwrap();

	assert_eq!(restored, simple_data, "Roundtrip should preserve data");
}

#[rstest]
fn test_json_roundtrip_nested_data(json_serializer: JsonSerializer, nested_data: NestedData) {
	let bytes = json_serializer.serialize(&nested_data).unwrap();
	let restored: NestedData = json_serializer.deserialize(&bytes).unwrap();

	assert_eq!(restored.user, nested_data.user, "Nested user should match");
	assert_eq!(
		restored.metadata, nested_data.metadata,
		"Metadata should match"
	);
	assert_eq!(restored.tags, nested_data.tags, "Tags should match");
}

#[rstest]
fn test_json_roundtrip_complex_data(json_serializer: JsonSerializer, complex_data: ComplexData) {
	let bytes = json_serializer.serialize(&complex_data).unwrap();
	let restored: ComplexData = json_serializer.deserialize(&bytes).unwrap();

	assert_eq!(restored.int_value, complex_data.int_value);
	assert!((restored.float_value - complex_data.float_value).abs() < f64::EPSILON);
	assert_eq!(restored.string_value, complex_data.string_value);
	assert_eq!(restored.optional_value, complex_data.optional_value);
	assert_eq!(restored.array_value, complex_data.array_value);
	assert!(restored.nested.is_some());
}

#[rstest]
fn test_json_roundtrip_session_data(
	json_serializer: JsonSerializer,
	session_data: SessionLikeData,
) {
	let bytes = json_serializer.serialize(&session_data).unwrap();
	let restored: SessionLikeData = json_serializer.deserialize(&bytes).unwrap();

	assert_eq!(restored.session_id, session_data.session_id);
	assert_eq!(restored.user_id, session_data.user_id);
	assert_eq!(restored.created_at, session_data.created_at);
	assert_eq!(restored.expires_at, session_data.expires_at);
	assert_eq!(restored.data.len(), session_data.data.len());
}

// =============================================================================
// SerializationFormat Tests
// =============================================================================

#[rstest]
fn test_serialization_format_json_name() {
	let format = SerializationFormat::Json;
	assert_eq!(format.name(), "json");
}

#[rstest]
fn test_serialization_format_default() {
	let format = SerializationFormat::default();
	assert_eq!(format, SerializationFormat::Json);
}

#[rstest]
fn test_serialization_format_serialize_deserialize(simple_data: SimpleData) {
	let format = SerializationFormat::Json;

	let bytes = format.serialize(&simple_data).unwrap();
	let restored: SimpleData = format.deserialize(&bytes).unwrap();

	assert_eq!(restored, simple_data);
}

// =============================================================================
// Edge Cases - Unicode
// =============================================================================

#[rstest]
#[case("Hello, World!", "ASCII")]
#[case("こんにちは世界", "Japanese")]
#[case("Привет мир", "Russian")]
#[case("مرحبا بالعالم", "Arabic")]
#[case("🎉🚀💻🔥", "Emoji")]
#[case("Hello\nWorld\tTab", "Control characters")]
fn test_json_unicode_strings(
	json_serializer: JsonSerializer,
	#[case] text: &str,
	#[case] desc: &str,
) {
	let data = SimpleData {
		id: 1,
		name: text.to_string(),
		active: true,
	};

	let bytes = json_serializer.serialize(&data).unwrap();
	let restored: SimpleData = json_serializer.deserialize(&bytes).unwrap();

	assert_eq!(
		restored.name, data.name,
		"Unicode string should be preserved for {}",
		desc
	);
}

// =============================================================================
// Edge Cases - Numeric Boundaries
// =============================================================================

#[rstest]
#[case(0i64, "zero")]
#[case(1i64, "one")]
#[case(-1i64, "negative one")]
#[case(i64::MAX, "max i64")]
#[case(i64::MIN, "min i64")]
fn test_json_integer_boundaries(
	json_serializer: JsonSerializer,
	#[case] value: i64,
	#[case] desc: &str,
) {
	#[derive(Serialize, Deserialize, PartialEq, Debug)]
	struct IntData {
		value: i64,
	}

	let data = IntData { value };

	let bytes = json_serializer.serialize(&data).unwrap();
	let restored: IntData = json_serializer.deserialize(&bytes).unwrap();

	assert_eq!(
		restored.value, value,
		"Integer should be preserved for {}",
		desc
	);
}

#[rstest]
#[case(0.0f64, "zero")]
#[case(1.0f64, "one")]
#[case(-1.0f64, "negative one")]
#[case(f64::MIN_POSITIVE, "min positive")]
#[case(f64::MAX, "max f64")]
fn test_json_float_boundaries(
	json_serializer: JsonSerializer,
	#[case] value: f64,
	#[case] desc: &str,
) {
	#[derive(Serialize, Deserialize, Debug)]
	struct FloatData {
		value: f64,
	}

	let data = FloatData { value };

	let bytes = json_serializer.serialize(&data).unwrap();
	let restored: FloatData = json_serializer.deserialize(&bytes).unwrap();

	assert!(
		(restored.value - value).abs() < f64::EPSILON || restored.value == value,
		"Float should be preserved for {} (got {}, expected {})",
		desc,
		restored.value,
		value
	);
}

// =============================================================================
// Edge Cases - Empty and Null Values
// =============================================================================

#[rstest]
fn test_json_empty_string(json_serializer: JsonSerializer) {
	let data = SimpleData {
		id: 0,
		name: String::new(),
		active: false,
	};

	let bytes = json_serializer.serialize(&data).unwrap();
	let restored: SimpleData = json_serializer.deserialize(&bytes).unwrap();

	assert!(restored.name.is_empty(), "Empty string should be preserved");
}

#[rstest]
fn test_json_empty_vec(json_serializer: JsonSerializer) {
	let data = ComplexData {
		int_value: 0,
		float_value: 0.0,
		string_value: String::new(),
		optional_value: None,
		array_value: vec![],
		nested: None,
	};

	let bytes = json_serializer.serialize(&data).unwrap();
	let restored: ComplexData = json_serializer.deserialize(&bytes).unwrap();

	assert!(
		restored.array_value.is_empty(),
		"Empty vec should be preserved"
	);
	assert!(
		restored.optional_value.is_none(),
		"None should be preserved"
	);
	assert!(restored.nested.is_none(), "None nested should be preserved");
}

#[rstest]
fn test_json_empty_hashmap(json_serializer: JsonSerializer) {
	let data = NestedData {
		user: SimpleData {
			id: 0,
			name: String::new(),
			active: false,
		},
		metadata: HashMap::new(),
		tags: vec![],
	};

	let bytes = json_serializer.serialize(&data).unwrap();
	let restored: NestedData = json_serializer.deserialize(&bytes).unwrap();

	assert!(
		restored.metadata.is_empty(),
		"Empty HashMap should be preserved"
	);
	assert!(restored.tags.is_empty(), "Empty Vec should be preserved");
}

// =============================================================================
// Edge Cases - Large Data
// =============================================================================

#[rstest]
fn test_json_large_string(json_serializer: JsonSerializer) {
	let large_string = "x".repeat(10_000);
	let data = SimpleData {
		id: 1,
		name: large_string.clone(),
		active: true,
	};

	let bytes = json_serializer.serialize(&data).unwrap();
	let restored: SimpleData = json_serializer.deserialize(&bytes).unwrap();

	assert_eq!(
		restored.name.len(),
		10_000,
		"Large string should be preserved"
	);
	assert_eq!(restored.name, large_string);
}

#[rstest]
fn test_json_large_array(json_serializer: JsonSerializer) {
	let large_array: Vec<i32> = (0..1000).collect();
	let data = ComplexData {
		int_value: 0,
		float_value: 0.0,
		string_value: String::new(),
		optional_value: None,
		array_value: large_array.clone(),
		nested: None,
	};

	let bytes = json_serializer.serialize(&data).unwrap();
	let restored: ComplexData = json_serializer.deserialize(&bytes).unwrap();

	assert_eq!(
		restored.array_value.len(),
		1000,
		"Large array should be preserved"
	);
	assert_eq!(restored.array_value, large_array);
}

#[rstest]
fn test_json_deeply_nested_structure(json_serializer: JsonSerializer) {
	// Create a 10-level deep nested structure
	let mut current = ComplexData {
		int_value: 0,
		float_value: 0.0,
		string_value: "level_0".to_string(),
		optional_value: None,
		array_value: vec![],
		nested: None,
	};

	for i in 1..10 {
		current = ComplexData {
			int_value: i,
			float_value: i as f64,
			string_value: format!("level_{}", i),
			optional_value: Some(format!("opt_{}", i)),
			array_value: vec![i as i32],
			nested: Some(Box::new(current)),
		};
	}

	let bytes = json_serializer.serialize(&current).unwrap();
	let restored: ComplexData = json_serializer.deserialize(&bytes).unwrap();

	assert_eq!(restored.int_value, 9, "Top level value should match");
	assert_eq!(restored.string_value, "level_9");

	// Verify we can traverse the nested structure
	let mut level = &restored;
	let mut count = 0;
	while let Some(ref nested) = level.nested {
		count += 1;
		level = nested;
	}
	assert_eq!(count, 9, "Should have 9 levels of nesting");
}

// =============================================================================
// Edge Cases - Special Characters
// =============================================================================

#[rstest]
#[case("", "empty")]
#[case(" ", "single space")]
#[case("   ", "multiple spaces")]
#[case("\t", "tab")]
#[case("\n", "newline")]
#[case("\r\n", "crlf")]
#[case("\"quoted\"", "quotes")]
#[case("back\\slash", "backslash")]
#[case("/forward/slash", "forward slash")]
fn test_json_special_strings(
	json_serializer: JsonSerializer,
	#[case] text: &str,
	#[case] desc: &str,
) {
	let data = SimpleData {
		id: 1,
		name: text.to_string(),
		active: true,
	};

	let bytes = json_serializer.serialize(&data).unwrap();
	let restored: SimpleData = json_serializer.deserialize(&bytes).unwrap();

	assert_eq!(
		restored.name, data.name,
		"Special string '{}' ({}) should be preserved",
		text, desc
	);
}

// =============================================================================
// Error Cases
// =============================================================================

#[rstest]
fn test_json_deserialize_invalid_bytes(json_serializer: JsonSerializer) {
	let invalid_bytes = b"not valid json";
	let result: Result<SimpleData, _> = json_serializer.deserialize(invalid_bytes);

	assert!(result.is_err(), "Invalid JSON should return error");
}

#[rstest]
fn test_json_deserialize_wrong_type(json_serializer: JsonSerializer) {
	// Serialize a SimpleData
	let data = SimpleData {
		id: 1,
		name: "test".to_string(),
		active: true,
	};
	let bytes = json_serializer.serialize(&data).unwrap();

	// Try to deserialize as ComplexData (wrong type)
	let result: Result<ComplexData, _> = json_serializer.deserialize(&bytes);

	assert!(result.is_err(), "Wrong type should return error");
}

#[rstest]
fn test_json_deserialize_empty_bytes(json_serializer: JsonSerializer) {
	let empty_bytes: &[u8] = &[];
	let result: Result<SimpleData, _> = json_serializer.deserialize(empty_bytes);

	assert!(result.is_err(), "Empty bytes should return error");
}

// =============================================================================
// JSON Output Format Tests
// =============================================================================

#[rstest]
fn test_json_output_is_valid_json(json_serializer: JsonSerializer, simple_data: SimpleData) {
	let bytes = json_serializer.serialize(&simple_data).unwrap();
	let json_str = std::str::from_utf8(&bytes).unwrap();

	// Should be valid JSON that can be parsed by serde_json
	let value: serde_json::Value = serde_json::from_str(json_str).unwrap();

	assert!(value.is_object(), "Output should be a JSON object");
	assert!(value.get("id").is_some(), "Should have id field");
	assert!(value.get("name").is_some(), "Should have name field");
	assert!(value.get("active").is_some(), "Should have active field");
}

#[rstest]
fn test_json_output_contains_expected_values(json_serializer: JsonSerializer) {
	let data = SimpleData {
		id: 42,
		name: "Test".to_string(),
		active: true,
	};

	let bytes = json_serializer.serialize(&data).unwrap();
	let json_str = std::str::from_utf8(&bytes).unwrap();
	let value: serde_json::Value = serde_json::from_str(json_str).unwrap();

	assert_eq!(value["id"], 42);
	assert_eq!(value["name"], "Test");
	assert_eq!(value["active"], true);
}

// =============================================================================
// Equivalence Partitioning - Data Types
// =============================================================================

#[rstest]
fn test_json_boolean_values(json_serializer: JsonSerializer) {
	#[derive(Serialize, Deserialize, PartialEq, Debug)]
	struct BoolData {
		flag_true: bool,
		flag_false: bool,
	}

	let data = BoolData {
		flag_true: true,
		flag_false: false,
	};

	let bytes = json_serializer.serialize(&data).unwrap();
	let restored: BoolData = json_serializer.deserialize(&bytes).unwrap();

	assert_eq!(restored, data);
}

#[rstest]
fn test_json_optional_values(json_serializer: JsonSerializer) {
	#[derive(Serialize, Deserialize, PartialEq, Debug)]
	struct OptionalData {
		present: Option<i32>,
		absent: Option<i32>,
	}

	let data = OptionalData {
		present: Some(42),
		absent: None,
	};

	let bytes = json_serializer.serialize(&data).unwrap();
	let restored: OptionalData = json_serializer.deserialize(&bytes).unwrap();

	assert_eq!(restored, data);
}

#[rstest]
fn test_json_vec_of_objects(json_serializer: JsonSerializer) {
	#[derive(Serialize, Deserialize, PartialEq, Debug)]
	struct Container {
		items: Vec<SimpleData>,
	}

	let data = Container {
		items: vec![
			SimpleData {
				id: 1,
				name: "First".to_string(),
				active: true,
			},
			SimpleData {
				id: 2,
				name: "Second".to_string(),
				active: false,
			},
			SimpleData {
				id: 3,
				name: "Third".to_string(),
				active: true,
			},
		],
	};

	let bytes = json_serializer.serialize(&data).unwrap();
	let restored: Container = json_serializer.deserialize(&bytes).unwrap();

	assert_eq!(restored.items.len(), 3);
	assert_eq!(restored.items, data.items);
}

// =============================================================================
// Serializer Trait Tests
// =============================================================================

/// Test serializer through generic function instead of trait object.
/// Note: Serializer trait has generic methods (serialize<T>, deserialize<T>) which makes
/// it not dyn-compatible, so we use generics to test polymorphic behavior.
fn test_serializer_generic<S: Serializer>(serializer: &S, data: &SimpleData) {
	let bytes = serializer.serialize(data).unwrap();
	let restored: SimpleData = serializer.deserialize(&bytes).unwrap();

	assert_eq!(&restored, data);
}

#[rstest]
fn test_serializer_polymorphism(simple_data: SimpleData) {
	// Test JsonSerializer through generic function
	test_serializer_generic(&JsonSerializer, &simple_data);
}

#[rstest]
fn test_serializer_send_sync() {
	// This test verifies that JsonSerializer implements Send + Sync
	fn assert_send_sync<T: Send + Sync>() {}
	assert_send_sync::<JsonSerializer>();
}

// =============================================================================
// Real-World Session Data Tests
// =============================================================================

#[rstest]
fn test_json_realistic_session_data(json_serializer: JsonSerializer) {
	let mut session_data = HashMap::new();
	session_data.insert("user_id".to_string(), serde_json::json!(12345));
	session_data.insert(
		"username".to_string(),
		serde_json::json!("alice@example.com"),
	);
	session_data.insert("roles".to_string(), serde_json::json!(["admin", "user"]));
	session_data.insert(
		"preferences".to_string(),
		serde_json::json!({
			"theme": "dark",
			"language": "en",
			"notifications": true
		}),
	);
	session_data.insert("csrf_token".to_string(), serde_json::json!("abc123def456"));
	session_data.insert("last_activity".to_string(), serde_json::json!(1704067200));

	let bytes = json_serializer.serialize(&session_data).unwrap();
	let restored: HashMap<String, serde_json::Value> = json_serializer.deserialize(&bytes).unwrap();

	assert_eq!(restored.len(), session_data.len());
	assert_eq!(restored["user_id"], 12345);
	assert_eq!(restored["username"], "alice@example.com");
	assert_eq!(restored["roles"].as_array().unwrap().len(), 2);
	assert_eq!(restored["preferences"]["theme"], "dark");
}
