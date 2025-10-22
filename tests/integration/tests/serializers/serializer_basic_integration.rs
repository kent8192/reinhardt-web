//! Integration tests for serializer basic functionality
//!
//! These tests verify that reinhardt-serializers work correctly without ORM dependencies.

use reinhardt_serializers::{
    BooleanField, CharField, Deserializer, EmailField, Field, IntegerField, JsonSerializer,
    Serializer,
};
use serde::{Deserialize, Serialize};

// ============================================================================
// Basic Serialization Tests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct User {
    name: String,
    age: i64,
    active: bool,
}

#[test]
fn test_json_serializer_basic() {
    let serializer = JsonSerializer::<User>::new();
    let user = User {
        name: "Alice".to_string(),
        age: 30,
        active: true,
    };

    let result = serializer.serialize(&user).unwrap();
    let json_str = String::from_utf8(result).unwrap();

    assert!(json_str.contains("Alice"));
    assert!(json_str.contains("30"));
    assert!(json_str.contains("true"));
}

#[test]
fn test_json_serializer_deserialize() {
    let serializer = JsonSerializer::<User>::new();
    let json = r#"{"name":"Bob","age":25,"active":false}"#;

    let result = serializer.deserialize(json.as_bytes()).unwrap();

    assert_eq!(result.name, "Bob");
    assert_eq!(result.age, 25);
    assert_eq!(result.active, false);
}

#[test]
fn test_json_serializer_round_trip() {
    let serializer = JsonSerializer::<User>::new();
    let original = User {
        name: "Charlie".to_string(),
        age: 35,
        active: true,
    };

    let serialized = serializer.serialize(&original).unwrap();
    let deserialized = serializer.deserialize(&serialized).unwrap();

    assert_eq!(original, deserialized);
}

// ============================================================================
// Field Validation Tests
// ============================================================================

#[test]
fn test_char_field_validation() {
    let field = CharField::new().min_length(3).max_length(10);

    // Valid string
    assert!(field.validate(&"hello".to_string()).is_ok());

    // Too short
    let result = field.validate(&"ab".to_string());
    assert!(result.is_err());

    // Too long
    let result = field.validate(&"this is way too long".to_string());
    assert!(result.is_err());
}

#[test]
fn test_char_field_min_length_only() {
    let field = CharField::new().min_length(5);

    assert!(field.validate(&"hello".to_string()).is_ok());
    assert!(field.validate(&"hello world".to_string()).is_ok());
    assert!(field.validate(&"hi".to_string()).is_err());
}

#[test]
fn test_char_field_max_length_only() {
    let field = CharField::new().max_length(10);

    assert!(field.validate(&"hello".to_string()).is_ok());
    assert!(field.validate(&"hi".to_string()).is_ok());
    assert!(field.validate(&"this is too long".to_string()).is_err());
}

#[test]
fn test_integer_field_validation() {
    let field = IntegerField::new().min_value(0).max_value(100);

    // Valid values
    assert!(field.validate(&0).is_ok());
    assert!(field.validate(&50).is_ok());
    assert!(field.validate(&100).is_ok());

    // Invalid: too small
    assert!(field.validate(&-1).is_err());

    // Invalid: too large
    assert!(field.validate(&101).is_err());
}

#[test]
fn test_integer_field_min_value_only() {
    let field = IntegerField::new().min_value(1);

    assert!(field.validate(&1).is_ok());
    assert!(field.validate(&1000).is_ok());
    assert!(field.validate(&0).is_err());
}

#[test]
fn test_integer_field_max_value_only() {
    let field = IntegerField::new().max_value(50);

    assert!(field.validate(&50).is_ok());
    assert!(field.validate(&0).is_ok());
    assert!(field.validate(&51).is_err());
}

// ============================================================================
// Field Serialization Tests
// ============================================================================

#[test]
fn test_char_field_serialize() {
    let field = CharField::new();
    let value = "hello".to_string();

    let result = field.serialize(&value).unwrap();
    assert_eq!(result.as_str().unwrap(), "hello");
}

#[test]
fn test_char_field_deserialize() {
    let field = CharField::new();
    let json_value = serde_json::Value::String("world".to_string());

    let result = field.deserialize(&json_value).unwrap();
    assert_eq!(result, "world");
}

#[test]
fn test_integer_field_serialize() {
    let field = IntegerField::new();
    let value = 42i64;

    let result = field.serialize(&value).unwrap();
    assert_eq!(result.as_i64().unwrap(), 42);
}

#[test]
fn test_integer_field_deserialize() {
    let field = IntegerField::new();
    let json_value = serde_json::Value::Number(99.into());

    let result = field.deserialize(&json_value).unwrap();
    assert_eq!(result, 99);
}

#[test]
fn test_boolean_field_serialize() {
    let field = BooleanField::new();

    let result_true = field.serialize(&true).unwrap();
    assert_eq!(result_true.as_bool().unwrap(), true);

    let result_false = field.serialize(&false).unwrap();
    assert_eq!(result_false.as_bool().unwrap(), false);
}

#[test]
fn test_boolean_field_deserialize() {
    let field = BooleanField::new();

    let json_true = serde_json::Value::Bool(true);
    assert_eq!(field.deserialize(&json_true).unwrap(), true);

    let json_false = serde_json::Value::Bool(false);
    assert_eq!(field.deserialize(&json_false).unwrap(), false);
}

// ============================================================================
// Email Field Tests
// ============================================================================

#[test]
fn test_email_field_validation() {
    let field = EmailField::new();

    // Valid emails
    assert!(field.validate(&"user@example.com".to_string()).is_ok());
    assert!(field
        .validate(&"test.user@example.co.uk".to_string())
        .is_ok());

    // Invalid emails
    assert!(field.validate(&"not-an-email".to_string()).is_err());
    assert!(field.validate(&"user@".to_string()).is_err());
    assert!(field.validate(&"@".to_string()).is_err());
}

#[test]
fn test_email_field_serialize_deserialize() {
    let field = EmailField::new();
    let email = "admin@example.com".to_string();

    // Serialize
    let serialized = field.serialize(&email).unwrap();
    assert_eq!(serialized.as_str().unwrap(), "admin@example.com");

    // Deserialize
    let json_value = serde_json::Value::String("test@example.com".to_string());
    let deserialized = field.deserialize(&json_value).unwrap();
    assert_eq!(deserialized, "test@example.com");
}

// ============================================================================
// Complex Nested Data Tests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Address {
    street: String,
    city: String,
    zip_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct UserWithAddress {
    name: String,
    email: String,
    address: Address,
}

#[test]
fn test_nested_serialization() {
    let serializer = JsonSerializer::<UserWithAddress>::new();
    let user = UserWithAddress {
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        address: Address {
            street: "123 Main St".to_string(),
            city: "Springfield".to_string(),
            zip_code: "12345".to_string(),
        },
    };

    let serialized = serializer.serialize(&user).unwrap();
    let json_str = String::from_utf8(serialized).unwrap();

    assert!(json_str.contains("John Doe"));
    assert!(json_str.contains("john@example.com"));
    assert!(json_str.contains("123 Main St"));
    assert!(json_str.contains("Springfield"));
}

#[test]
fn test_nested_deserialization() {
    let serializer = JsonSerializer::<UserWithAddress>::new();
    let json = r#"{
        "name": "Jane Smith",
        "email": "jane@example.com",
        "address": {
            "street": "456 Oak Ave",
            "city": "Boston",
            "zip_code": "02101"
        }
    }"#;

    let result = serializer.deserialize(json.as_bytes()).unwrap();

    assert_eq!(result.name, "Jane Smith");
    assert_eq!(result.email, "jane@example.com");
    assert_eq!(result.address.street, "456 Oak Ave");
    assert_eq!(result.address.city, "Boston");
    assert_eq!(result.address.zip_code, "02101");
}

// ============================================================================
// Array/List Serialization Tests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct UserList {
    users: Vec<User>,
}

#[test]
fn test_list_serialization() {
    let serializer = JsonSerializer::<UserList>::new();
    let user_list = UserList {
        users: vec![
            User {
                name: "Alice".to_string(),
                age: 30,
                active: true,
            },
            User {
                name: "Bob".to_string(),
                age: 25,
                active: false,
            },
        ],
    };

    let serialized = serializer.serialize(&user_list).unwrap();
    let json_str = String::from_utf8(serialized).unwrap();

    assert!(json_str.contains("Alice"));
    assert!(json_str.contains("Bob"));
}

#[test]
fn test_list_deserialization() {
    let serializer = JsonSerializer::<UserList>::new();
    let json = r#"{
        "users": [
            {"name": "Charlie", "age": 35, "active": true},
            {"name": "Diana", "age": 28, "active": false}
        ]
    }"#;

    let result = serializer.deserialize(json.as_bytes()).unwrap();

    assert_eq!(result.users.len(), 2);
    assert_eq!(result.users[0].name, "Charlie");
    assert_eq!(result.users[1].name, "Diana");
}

// ============================================================================
// Optional Fields Tests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct UserWithOptional {
    name: String,
    email: Option<String>,
    age: Option<i64>,
}

#[test]
fn test_optional_fields_present() {
    let serializer = JsonSerializer::<UserWithOptional>::new();
    let user = UserWithOptional {
        name: "Alice".to_string(),
        email: Some("alice@example.com".to_string()),
        age: Some(30),
    };

    let serialized = serializer.serialize(&user).unwrap();
    let deserialized = serializer.deserialize(&serialized).unwrap();

    assert_eq!(deserialized, user);
}

#[test]
fn test_optional_fields_absent() {
    let serializer = JsonSerializer::<UserWithOptional>::new();
    let user = UserWithOptional {
        name: "Bob".to_string(),
        email: None,
        age: None,
    };

    let serialized = serializer.serialize(&user).unwrap();
    let deserialized = serializer.deserialize(&serialized).unwrap();

    assert_eq!(deserialized.name, "Bob");
    assert_eq!(deserialized.email, None);
    assert_eq!(deserialized.age, None);
}
