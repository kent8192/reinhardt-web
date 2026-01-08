//! Unit tests for introspect type mapping module
//!
//! Tests for SQL to Rust type mapping including:
//! - Integer type mappings
//! - String type mappings
//! - Date/time type mappings
//! - PostgreSQL-specific types
//! - Nullable field handling
//! - Auto-increment handling
//! - Type override functionality
//!
//! **Test Categories:**
//! - Equivalence partitioning: Type groups
//! - Decision table: nullable × auto_increment → `Option<T>`
//! - Boundary values: VARCHAR lengths, DECIMAL precision

use reinhardt_migrations::{FieldType, ForeignKeyAction, TypeMapper, TypeMappingError};
use rstest::*;
use std::collections::HashMap;

// ============================================================================
// Equivalence Partitioning: Integer Types
// ============================================================================

/// Test integer type partition
///
/// **Test Intent**: Verify all integer types map to correct Rust types
#[rstest]
#[case(FieldType::BigInteger, "i64")]
#[case(FieldType::Integer, "i32")]
#[case(FieldType::SmallInteger, "i16")]
#[case(FieldType::TinyInt, "i8")]
#[case(FieldType::MediumInt, "i32")]
fn test_integer_type_mapping(#[case] field_type: FieldType, #[case] expected: &str) {
	let mapper = TypeMapper::default();
	let result = mapper.field_type_to_rust_string(&field_type, false, false);
	assert_eq!(result.unwrap(), expected);
}

// ============================================================================
// Equivalence Partitioning: String Types
// ============================================================================

/// Test string type partition
///
/// **Test Intent**: Verify all string types map to String
#[rstest]
#[case(FieldType::VarChar(255), "String")]
#[case(FieldType::VarChar(100), "String")]
#[case(FieldType::Char(10), "String")]
#[case(FieldType::Char(1), "String")]
#[case(FieldType::Text, "String")]
#[case(FieldType::TinyText, "String")]
#[case(FieldType::MediumText, "String")]
#[case(FieldType::LongText, "String")]
fn test_string_type_mapping(#[case] field_type: FieldType, #[case] expected: &str) {
	let mapper = TypeMapper::default();
	let result = mapper.field_type_to_rust_string(&field_type, false, false);
	assert_eq!(result.unwrap(), expected);
}

// ============================================================================
// Equivalence Partitioning: Date/Time Types
// ============================================================================

/// Test date/time type partition
///
/// **Test Intent**: Verify all date/time types map to chrono types
#[rstest]
#[case(FieldType::Date, "chrono::NaiveDate")]
#[case(FieldType::Time, "chrono::NaiveTime")]
#[case(FieldType::DateTime, "chrono::NaiveDateTime")]
#[case(FieldType::TimestampTz, "chrono::DateTime<chrono::Utc>")]
fn test_datetime_type_mapping(#[case] field_type: FieldType, #[case] expected: &str) {
	let mapper = TypeMapper::default();
	let result = mapper.field_type_to_rust_string(&field_type, false, false);
	assert_eq!(result.unwrap(), expected);
}

// ============================================================================
// Equivalence Partitioning: Numeric Types
// ============================================================================

/// Test numeric type partition
///
/// **Test Intent**: Verify floating point and decimal types
#[rstest]
#[case(FieldType::Float, "f32")]
#[case(FieldType::Double, "f64")]
#[case(FieldType::Real, "f32")]
#[case(FieldType::Decimal { precision: 10, scale: 2 }, "rust_decimal::Decimal")]
fn test_numeric_type_mapping(#[case] field_type: FieldType, #[case] expected: &str) {
	let mapper = TypeMapper::default();
	let result = mapper.field_type_to_rust_string(&field_type, false, false);
	assert_eq!(result.unwrap(), expected);
}

// ============================================================================
// Equivalence Partitioning: Binary Types
// ============================================================================

/// Test binary type partition
///
/// **Test Intent**: Verify all binary types map to `Vec<u8>`
#[rstest]
#[case(FieldType::Binary, "Vec<u8>")]
#[case(FieldType::Blob, "Vec<u8>")]
#[case(FieldType::TinyBlob, "Vec<u8>")]
#[case(FieldType::MediumBlob, "Vec<u8>")]
#[case(FieldType::LongBlob, "Vec<u8>")]
#[case(FieldType::Bytea, "Vec<u8>")]
fn test_binary_type_mapping(#[case] field_type: FieldType, #[case] expected: &str) {
	let mapper = TypeMapper::default();
	let result = mapper.field_type_to_rust_string(&field_type, false, false);
	assert_eq!(result.unwrap(), expected);
}

// ============================================================================
// Equivalence Partitioning: JSON Types
// ============================================================================

/// Test JSON type partition
///
/// **Test Intent**: Verify JSON types map to serde_json::Value
#[rstest]
#[case(FieldType::Json, "serde_json::Value")]
#[case(FieldType::JsonBinary, "serde_json::Value")]
fn test_json_type_mapping(#[case] field_type: FieldType, #[case] expected: &str) {
	let mapper = TypeMapper::default();
	let result = mapper.field_type_to_rust_string(&field_type, false, false);
	assert_eq!(result.unwrap(), expected);
}

// ============================================================================
// Equivalence Partitioning: PostgreSQL-Specific Types
// ============================================================================

/// Test PostgreSQL-specific type partition
///
/// **Test Intent**: Verify PostgreSQL types map correctly
#[rstest]
#[case(FieldType::Uuid, "uuid::Uuid")]
#[case(FieldType::Boolean, "bool")]
#[case(FieldType::HStore, "std::collections::HashMap<String, String>")]
#[case(FieldType::CIText, "String")]
#[case(FieldType::TsVector, "String")]
#[case(FieldType::TsQuery, "String")]
fn test_postgres_specific_mapping(#[case] field_type: FieldType, #[case] expected: &str) {
	let mapper = TypeMapper::default();
	let result = mapper.field_type_to_rust_string(&field_type, false, false);
	assert_eq!(result.unwrap(), expected);
}

/// Test PostgreSQL range types
///
/// **Test Intent**: Verify range types map to tuple types
#[rstest]
#[case(FieldType::Int4Range, "(i32, i32)")]
#[case(FieldType::Int8Range, "(i64, i64)")]
#[case(FieldType::NumRange, "(Decimal, Decimal)")]
#[case(FieldType::DateRange, "(NaiveDate, NaiveDate)")]
#[case(FieldType::TsRange, "(NaiveDateTime, NaiveDateTime)")]
#[case(FieldType::TsTzRange, "(DateTime<Utc>, DateTime<Utc>)")]
fn test_postgres_range_mapping(#[case] field_type: FieldType, #[case] expected: &str) {
	let mapper = TypeMapper::default();
	let result = mapper.field_type_to_rust_string(&field_type, false, false);
	assert_eq!(result.unwrap(), expected);
}

// ============================================================================
// Equivalence Partitioning: MySQL-Specific Types
// ============================================================================

/// Test MySQL-specific type partition
///
/// **Test Intent**: Verify MySQL types map correctly
#[rstest]
#[case(FieldType::Year, "i16")]
#[case(FieldType::Enum { values: vec!["active".to_string()] }, "String")]
#[case(FieldType::Set { values: vec!["read".to_string()] }, "Vec<String>")]
fn test_mysql_specific_mapping(#[case] field_type: FieldType, #[case] expected: &str) {
	let mapper = TypeMapper::default();
	let result = mapper.field_type_to_rust_string(&field_type, false, false);
	assert_eq!(result.unwrap(), expected);
}

// ============================================================================
// Equivalence Partitioning: Relationship Types
// ============================================================================

/// Test relationship type partition
///
/// **Test Intent**: Verify FK types map to i64
#[rstest]
#[case(FieldType::ForeignKey { to_table: "users".to_string(), to_field: "id".to_string(), on_delete: ForeignKeyAction::Cascade }, "i64")]
#[case(FieldType::OneToOne { to: "profile".to_string(), on_delete: ForeignKeyAction::Cascade, on_update: ForeignKeyAction::Cascade }, "i64")]
fn test_relationship_type_mapping(#[case] field_type: FieldType, #[case] expected: &str) {
	let mapper = TypeMapper::default();
	let result = mapper.field_type_to_rust_string(&field_type, false, false);
	assert_eq!(result.unwrap(), expected);
}

/// Test ManyToMany type returns error
///
/// **Test Intent**: ManyToMany is not a column, should return error
#[rstest]
#[test]
fn test_many_to_many_returns_error() {
	let mapper = TypeMapper::default();
	let field_type = FieldType::ManyToMany {
		to: "tags".to_string(),
		through: Some("post_tags".to_string()),
	};
	let result = mapper.field_type_to_rust_string(&field_type, false, false);
	assert!(result.is_err());
	assert!(matches!(result, Err(TypeMappingError::UnsupportedType(_))));
}

// ============================================================================
// Decision Table: Nullable and Auto-Increment
// ============================================================================

/// Decision table for type mapping (nullable × auto_increment → type)
///
/// **Test Intent**: Verify nullable and auto_increment affect output correctly
///
/// | nullable | auto_increment | expected |
/// |----------|----------------|----------|
/// | false    | false          | i32      |
/// | true     | false          | `Option<i32>` |
/// | false    | true           | i32      |
/// | true     | true           | i32 (auto_increment overrides nullable) |
#[rstest]
#[case(false, false, "i32")]
#[case(true, false, "Option<i32>")]
#[case(false, true, "i32")]
#[case(true, true, "i32")] // auto_increment overrides nullable
fn test_nullable_auto_increment_decision_table(
	#[case] nullable: bool,
	#[case] auto_increment: bool,
	#[case] expected: &str,
) {
	let mapper = TypeMapper::default();
	let result = mapper.field_type_to_rust_string(&FieldType::Integer, nullable, auto_increment);
	assert_eq!(result.unwrap(), expected);
}

/// Test nullable with different types
///
/// **Test Intent**: Verify nullable wraps any base type in Option
#[rstest]
#[case(FieldType::VarChar(255), "Option<String>")]
#[case(FieldType::BigInteger, "Option<i64>")]
#[case(FieldType::Boolean, "Option<bool>")]
#[case(FieldType::DateTime, "Option<chrono::NaiveDateTime>")]
#[case(FieldType::Uuid, "Option<uuid::Uuid>")]
fn test_nullable_wrapping(#[case] field_type: FieldType, #[case] expected: &str) {
	let mapper = TypeMapper::default();
	let result = mapper.field_type_to_rust_string(&field_type, true, false);
	assert_eq!(result.unwrap(), expected);
}

// ============================================================================
// Type Override Tests
// ============================================================================

/// Test type override for specific column
///
/// **Test Intent**: Verify custom type overrides work
#[rstest]
#[test]
fn test_type_override_basic() {
	let mut overrides = HashMap::new();
	overrides.insert("users.status".to_string(), "UserStatus".to_string());

	let mapper = TypeMapper::new(overrides);

	assert_eq!(mapper.get_override("users", "status"), Some("UserStatus"));
	assert_eq!(mapper.get_override("users", "name"), None);
	assert_eq!(mapper.get_override("posts", "status"), None);
}

/// Test multiple overrides
///
/// **Test Intent**: Verify multiple overrides don't interfere
#[rstest]
#[test]
fn test_multiple_type_overrides() {
	let mut overrides = HashMap::new();
	overrides.insert("users.status".to_string(), "UserStatus".to_string());
	overrides.insert("users.role".to_string(), "UserRole".to_string());
	overrides.insert("posts.status".to_string(), "PostStatus".to_string());

	let mapper = TypeMapper::new(overrides);

	assert_eq!(mapper.get_override("users", "status"), Some("UserStatus"));
	assert_eq!(mapper.get_override("users", "role"), Some("UserRole"));
	assert_eq!(mapper.get_override("posts", "status"), Some("PostStatus"));
}

// ============================================================================
// Boundary Value Analysis
// ============================================================================

/// Test VARCHAR boundary values
///
/// **Test Intent**: Verify VARCHAR with various lengths
#[rstest]
#[case(1, "String")] // Minimum
#[case(255, "String")] // Common default
#[case(65535, "String")] // MySQL max
fn test_varchar_length_boundaries(#[case] length: u32, #[case] expected: &str) {
	let mapper = TypeMapper::default();
	let result = mapper.field_type_to_rust_string(&FieldType::VarChar(length), false, false);
	assert_eq!(result.unwrap(), expected);
}

/// Test DECIMAL precision/scale boundaries
///
/// **Test Intent**: Verify DECIMAL with various precision/scale
#[rstest]
#[case(1, 0, "rust_decimal::Decimal")] // Minimum precision
#[case(38, 0, "rust_decimal::Decimal")] // Maximum precision (most DBs)
#[case(10, 2, "rust_decimal::Decimal")] // Common money format
#[case(38, 38, "rust_decimal::Decimal")] // Max scale = precision
fn test_decimal_precision_boundaries(
	#[case] precision: u32,
	#[case] scale: u32,
	#[case] expected: &str,
) {
	let mapper = TypeMapper::default();
	let result =
		mapper.field_type_to_rust_string(&FieldType::Decimal { precision, scale }, false, false);
	assert_eq!(result.unwrap(), expected);
}

/// Test CHAR boundary values
///
/// **Test Intent**: Verify CHAR with various lengths
#[rstest]
#[case(1, "String")] // Minimum
#[case(255, "String")] // Maximum for most DBs
fn test_char_length_boundaries(#[case] length: u32, #[case] expected: &str) {
	let mapper = TypeMapper::default();
	let result = mapper.field_type_to_rust_string(&FieldType::Char(length), false, false);
	assert_eq!(result.unwrap(), expected);
}

// ============================================================================
// Array Type Tests
// ============================================================================

/// Test PostgreSQL array type mapping
///
/// **Test Intent**: Verify array types wrap inner type in Vec
#[rstest]
#[test]
fn test_array_type_integer() {
	let mapper = TypeMapper::default();
	let field_type = FieldType::Array(Box::new(FieldType::Integer));
	let result = mapper.field_type_to_rust(&field_type);
	assert!(result.is_ok());
	// The result should be Vec<i32>
	let tokens = result.unwrap().to_string();
	assert!(tokens.contains("Vec"));
}

/// Test nested array type (array of arrays)
///
/// **Test Intent**: Verify nested arrays are handled
#[rstest]
#[test]
fn test_array_type_nested() {
	let mapper = TypeMapper::default();
	let inner = FieldType::Array(Box::new(FieldType::Integer));
	let field_type = FieldType::Array(Box::new(inner));
	let result = mapper.field_type_to_rust(&field_type);
	assert!(result.is_ok());
}

/// Test array of various types
///
/// **Test Intent**: Verify arrays work with different inner types
#[rstest]
#[test]
fn test_array_type_text() {
	let mapper = TypeMapper::default();
	let field_type = FieldType::Array(Box::new(FieldType::Text));
	let result = mapper.field_type_to_rust(&field_type);
	assert!(result.is_ok());
}

// ============================================================================
// Custom Type Tests
// ============================================================================

/// Test custom type mapping
///
/// **Test Intent**: Verify custom types are passed through
#[rstest]
#[case("MyCustomType", "MyCustomType")]
#[case("CustomEnum", "CustomEnum")]
fn test_custom_type_mapping(#[case] type_name: &str, #[case] expected: &str) {
	let mapper = TypeMapper::default();
	let field_type = FieldType::Custom(type_name.to_string());
	let result = mapper.field_type_to_rust_string(&field_type, false, false);
	assert_eq!(result.unwrap(), expected);
}

/// Test custom type with invalid Rust syntax falls back to String
///
/// **Test Intent**: Verify invalid custom types default to String (in TokenStream version)
#[rstest]
#[test]
fn test_custom_type_invalid_syntax() {
	let mapper = TypeMapper::default();
	// This is an invalid Rust type name
	let field_type = FieldType::Custom("123invalid".to_string());
	// In string version, it just passes through
	let result = mapper.field_type_to_rust_string(&field_type, false, false);
	assert_eq!(result.unwrap(), "123invalid");
}

// ============================================================================
// Default TypeMapper Tests
// ============================================================================

/// Test default TypeMapper has no overrides
///
/// **Test Intent**: Verify default constructor creates empty mapper
#[rstest]
#[test]
fn test_default_type_mapper_empty() {
	let mapper = TypeMapper::default();
	assert_eq!(mapper.get_override("any", "column"), None);
}
