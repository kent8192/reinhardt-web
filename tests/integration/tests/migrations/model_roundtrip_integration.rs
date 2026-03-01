//! Integration tests for model roundtrip: ProjectState + ModelState + FieldState → DatabaseSchema → ColumnSchema
//!
//! These tests verify that `ProjectState::to_database_schema()` correctly maps
//! `FieldState` properties (field_type, nullable, params) to `ColumnSchema` properties
//! (data_type, nullable, primary_key, auto_increment, default).
//!
//! ## Test Categories
//!
//! - **RT-HP-01 to RT-HP-14**: Happy path type mapping (ProjectState → ColumnSchema)
//! - **RT-HP-15 to RT-HP-24**: Happy path attribute mapping (unique, default, index, etc.)
//! - **RT-MACRO-01 to RT-MACRO-06**: Macro simulation tests (expose bugs #1700, #1701, #1702)
//! - **RT-EC-01 to RT-EC-09**: Edge cases
//! - **RT-HP-25 to RT-HP-38**: PostgreSQL-specific type mapping
//! - **RT-TN-01 to RT-TN-08**: Table name conversion
//! - **RT-MACRO-07 to RT-MACRO-12**: Additional macro simulation tests
//! - **RT-COMB-01 to RT-COMB-05**: Attribute combination tests
//! - **RT-APP-01 to RT-APP-03**: Multi-app schema tests

use reinhardt_db::migrations::{
	ConstraintDefinition, DatabaseSchema, FieldState, FieldType, IndexDefinition, ModelState,
	ProjectState,
};
use rstest::*;

// ============================================================================
// Helpers
// ============================================================================

/// Build a FieldState with additional params
fn field_with_params(
	name: &str,
	ft: FieldType,
	nullable: bool,
	params: Vec<(&str, &str)>,
) -> FieldState {
	let mut f = FieldState::new(name, ft, nullable);
	for (k, v) in params {
		f.params.insert(k.to_string(), v.to_string());
	}
	f
}

/// Build a single-model ProjectState and return (DatabaseSchema, table_name)
fn build_single_model_schema(
	app: &str,
	model: &str,
	fields: Vec<(String, FieldState)>,
) -> DatabaseSchema {
	let mut state = ProjectState::new();
	let mut m = ModelState::new(app, model);
	for (name, field) in fields {
		m.fields.insert(name, field);
	}
	state.add_model(m);
	state.to_database_schema()
}

/// Simulate what the macro CURRENTLY produces for an i64 PK field (bug #1700).
/// The macro does NOT set auto_increment param.
fn simulate_macro_i64_pk() -> FieldState {
	// Bug #1700: macro never adds "auto_increment" to params
	field_with_params(
		"id",
		FieldType::BigInteger,
		false,
		vec![("primary_key", "true")],
	)
}

/// Simulate what the macro CURRENTLY produces for a non-Option String field (bug #1701).
/// The macro generates not_null: false, which means nullable: true.
fn simulate_macro_non_option_string() -> FieldState {
	// Bug #1701: macro sets nullable=true for non-Option fields
	FieldState::new("title", FieldType::VarChar(255), true)
}

/// Simulate what the macro CURRENTLY produces for DateTime<Utc> with auto_now_add (bug #1702).
/// The macro maps it to FieldType::DateTime instead of FieldType::TimestampTz.
fn simulate_macro_datetime_utc_auto_now_add() -> FieldState {
	// Bug #1702: macro uses DateTime instead of TimestampTz for DateTime<Utc>
	field_with_params(
		"created_at",
		FieldType::DateTime,
		false,
		vec![("auto_now_add", "true")],
	)
}

// ============================================================================
// Happy Path — Type Mapping (RT-HP-01 to RT-HP-14)
// ============================================================================

#[rstest]
fn test_rt_hp_01_big_integer_primary_key() {
	// Arrange
	let field = field_with_params(
		"id",
		FieldType::BigInteger,
		false,
		vec![("primary_key", "true"), ("auto_increment", "true")],
	);
	let schema = build_single_model_schema("testapp", "Article", vec![("id".to_string(), field)]);

	// Act
	let table = schema.tables.get("article").expect("table should exist");
	let col = table.columns.get("id").expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::BigInteger);
	assert!(col.primary_key, "id should be primary key");
	assert!(col.auto_increment, "i64 PK should have auto_increment");
	assert!(!col.nullable, "non-Option PK should not be nullable");
}

#[rstest]
fn test_rt_hp_02_integer_field() {
	// Arrange
	let field = FieldState::new("count", FieldType::Integer, false);
	let schema =
		build_single_model_schema("testapp", "Counter", vec![("count".to_string(), field)]);

	// Act
	let table = schema.tables.get("counter").expect("table should exist");
	let col = table.columns.get("count").expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::Integer);
	assert!(!col.nullable);
	assert!(!col.primary_key);
	assert!(!col.auto_increment);
}

#[rstest]
fn test_rt_hp_03_small_integer_field() {
	// Arrange
	let field = FieldState::new("priority", FieldType::SmallInteger, false);
	let schema =
		build_single_model_schema("testapp", "Task", vec![("priority".to_string(), field)]);

	// Act
	let table = schema.tables.get("task").expect("table should exist");
	let col = table.columns.get("priority").expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::SmallInteger);
	assert!(!col.nullable);
}

#[rstest]
fn test_rt_hp_04_varchar_field() {
	// Arrange
	let field = FieldState::new("name", FieldType::VarChar(100), false);
	let schema =
		build_single_model_schema("testapp", "Category", vec![("name".to_string(), field)]);

	// Act
	let table = schema.tables.get("category").expect("table should exist");
	let col = table.columns.get("name").expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::VarChar(100));
	assert!(!col.nullable);
}

#[rstest]
fn test_rt_hp_05_text_field() {
	// Arrange
	let field = FieldState::new("content", FieldType::Text, true);
	let schema = build_single_model_schema("testapp", "Post", vec![("content".to_string(), field)]);

	// Act
	let table = schema.tables.get("post").expect("table should exist");
	let col = table.columns.get("content").expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::Text);
	assert!(col.nullable, "Option<String> should be nullable");
}

#[rstest]
fn test_rt_hp_06_boolean_field() {
	// Arrange
	let field = FieldState::new("is_active", FieldType::Boolean, false);
	let schema =
		build_single_model_schema("testapp", "User", vec![("is_active".to_string(), field)]);

	// Act
	let table = schema.tables.get("user").expect("table should exist");
	let col = table.columns.get("is_active").expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::Boolean);
	assert!(!col.nullable);
}

#[rstest]
fn test_rt_hp_07_float_field() {
	// Arrange
	let field = FieldState::new("score", FieldType::Float, false);
	let schema = build_single_model_schema("testapp", "Rating", vec![("score".to_string(), field)]);

	// Act
	let table = schema.tables.get("rating").expect("table should exist");
	let col = table.columns.get("score").expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::Float);
}

#[rstest]
fn test_rt_hp_08_double_field() {
	// Arrange
	let field = FieldState::new("amount", FieldType::Double, false);
	let schema =
		build_single_model_schema("testapp", "Payment", vec![("amount".to_string(), field)]);

	// Act
	let table = schema.tables.get("payment").expect("table should exist");
	let col = table.columns.get("amount").expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::Double);
}

#[rstest]
fn test_rt_hp_09_datetime_field() {
	// Arrange
	let field = FieldState::new("event_time", FieldType::DateTime, false);
	let schema =
		build_single_model_schema("testapp", "Event", vec![("event_time".to_string(), field)]);

	// Act
	let table = schema.tables.get("event").expect("table should exist");
	let col = table
		.columns
		.get("event_time")
		.expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::DateTime);
}

#[rstest]
fn test_rt_hp_10_timestamp_tz_field() {
	// Arrange
	let field = FieldState::new("created_at", FieldType::TimestampTz, false);
	let schema =
		build_single_model_schema("testapp", "Audit", vec![("created_at".to_string(), field)]);

	// Act
	let table = schema.tables.get("audit").expect("table should exist");
	let col = table
		.columns
		.get("created_at")
		.expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::TimestampTz);
}

#[rstest]
fn test_rt_hp_11_date_field() {
	// Arrange
	let field = FieldState::new("birth_date", FieldType::Date, true);
	let schema =
		build_single_model_schema("testapp", "Person", vec![("birth_date".to_string(), field)]);

	// Act
	let table = schema.tables.get("person").expect("table should exist");
	let col = table
		.columns
		.get("birth_date")
		.expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::Date);
	assert!(col.nullable);
}

#[rstest]
fn test_rt_hp_12_time_field() {
	// Arrange
	let field = FieldState::new("start_time", FieldType::Time, false);
	let schema = build_single_model_schema(
		"testapp",
		"Schedule",
		vec![("start_time".to_string(), field)],
	);

	// Act
	let table = schema.tables.get("schedule").expect("table should exist");
	let col = table
		.columns
		.get("start_time")
		.expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::Time);
}

#[rstest]
fn test_rt_hp_13_uuid_field() {
	// Arrange
	let field = FieldState::new("external_id", FieldType::Uuid, false);
	let schema = build_single_model_schema(
		"testapp",
		"Resource",
		vec![("external_id".to_string(), field)],
	);

	// Act
	let table = schema.tables.get("resource").expect("table should exist");
	let col = table
		.columns
		.get("external_id")
		.expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::Uuid);
}

#[rstest]
fn test_rt_hp_14_decimal_field() {
	// Arrange
	let field = FieldState::new(
		"price",
		FieldType::Decimal {
			precision: 10,
			scale: 2,
		},
		false,
	);
	let schema =
		build_single_model_schema("testapp", "Product", vec![("price".to_string(), field)]);

	// Act
	let table = schema.tables.get("product").expect("table should exist");
	let col = table.columns.get("price").expect("column should exist");

	// Assert
	assert_eq!(
		col.data_type,
		FieldType::Decimal {
			precision: 10,
			scale: 2
		}
	);
	assert!(!col.nullable);
}

// ============================================================================
// Happy Path — Attribute Mapping (RT-HP-15 to RT-HP-24)
// ============================================================================

#[rstest]
fn test_rt_hp_15_primary_key_param() {
	// Arrange
	let field = field_with_params(
		"id",
		FieldType::Integer,
		false,
		vec![("primary_key", "true")],
	);
	let schema = build_single_model_schema("testapp", "Item", vec![("id".to_string(), field)]);

	// Act
	let table = schema.tables.get("item").expect("table should exist");
	let col = table.columns.get("id").expect("column should exist");

	// Assert
	assert!(col.primary_key);
	assert!(
		!col.auto_increment,
		"PK without auto_increment param should not auto-increment"
	);
}

#[rstest]
fn test_rt_hp_16_auto_increment_param() {
	// Arrange
	let field = field_with_params(
		"id",
		FieldType::BigInteger,
		false,
		vec![("primary_key", "true"), ("auto_increment", "true")],
	);
	let schema = build_single_model_schema("testapp", "Sequence", vec![("id".to_string(), field)]);

	// Act
	let table = schema.tables.get("sequence").expect("table should exist");
	let col = table.columns.get("id").expect("column should exist");

	// Assert
	assert!(col.primary_key);
	assert!(col.auto_increment);
}

#[rstest]
fn test_rt_hp_17_default_value_string() {
	// Arrange
	let field = field_with_params(
		"status",
		FieldType::VarChar(50),
		false,
		vec![("default", "active")],
	);
	let schema =
		build_single_model_schema("testapp", "Account", vec![("status".to_string(), field)]);

	// Act
	let table = schema.tables.get("account").expect("table should exist");
	let col = table.columns.get("status").expect("column should exist");

	// Assert
	assert_eq!(col.default, Some("active".to_string()));
}

#[rstest]
fn test_rt_hp_18_default_value_boolean() {
	// Arrange
	let field = field_with_params(
		"is_published",
		FieldType::Boolean,
		false,
		vec![("default", "false")],
	);
	let schema = build_single_model_schema(
		"testapp",
		"Document",
		vec![("is_published".to_string(), field)],
	);

	// Act
	let table = schema.tables.get("document").expect("table should exist");
	let col = table
		.columns
		.get("is_published")
		.expect("column should exist");

	// Assert
	assert_eq!(col.default, Some("false".to_string()));
}

#[rstest]
fn test_rt_hp_19_nullable_field() {
	// Arrange
	let field = FieldState::new("bio", FieldType::Text, true);
	let schema = build_single_model_schema("testapp", "Profile", vec![("bio".to_string(), field)]);

	// Act
	let table = schema.tables.get("profile").expect("table should exist");
	let col = table.columns.get("bio").expect("column should exist");

	// Assert
	assert!(col.nullable, "Option field should be nullable");
	assert_eq!(col.default, None);
}

#[rstest]
fn test_rt_hp_20_non_nullable_field() {
	// Arrange
	let field = FieldState::new("email", FieldType::VarChar(255), false);
	let schema = build_single_model_schema("testapp", "Member", vec![("email".to_string(), field)]);

	// Act
	let table = schema.tables.get("member").expect("table should exist");
	let col = table.columns.get("email").expect("column should exist");

	// Assert
	assert!(!col.nullable, "non-Option field should not be nullable");
}

#[rstest]
fn test_rt_hp_21_multiple_fields_on_one_model() {
	// Arrange
	let id_field = field_with_params(
		"id",
		FieldType::BigInteger,
		false,
		vec![("primary_key", "true"), ("auto_increment", "true")],
	);
	let name_field = FieldState::new("name", FieldType::VarChar(100), false);
	let desc_field = FieldState::new("description", FieldType::Text, true);

	let schema = build_single_model_schema(
		"testapp",
		"Widget",
		vec![
			("id".to_string(), id_field),
			("name".to_string(), name_field),
			("description".to_string(), desc_field),
		],
	);

	// Act
	let table = schema.tables.get("widget").expect("table should exist");

	// Assert
	assert_eq!(table.columns.len(), 3);
	assert!(table.columns.get("id").expect("id").primary_key);
	assert_eq!(
		table.columns.get("name").expect("name").data_type,
		FieldType::VarChar(100)
	);
	assert!(
		table
			.columns
			.get("description")
			.expect("description")
			.nullable
	);
}

#[rstest]
fn test_rt_hp_22_table_name_snake_case_conversion() {
	// Arrange
	let field = FieldState::new("id", FieldType::Integer, false);
	let schema = build_single_model_schema("testapp", "BlogPost", vec![("id".to_string(), field)]);

	// Act & Assert
	assert!(
		schema.tables.get("blog_post").is_some(),
		"CamelCase model name should convert to snake_case table name"
	);
}

#[rstest]
fn test_rt_hp_23_no_default_value() {
	// Arrange
	let field = FieldState::new("value", FieldType::Integer, false);
	let schema =
		build_single_model_schema("testapp", "Setting", vec![("value".to_string(), field)]);

	// Act
	let table = schema.tables.get("setting").expect("table should exist");
	let col = table.columns.get("value").expect("column should exist");

	// Assert
	assert_eq!(
		col.default, None,
		"field without default param should have no default"
	);
}

#[rstest]
fn test_rt_hp_24_default_value_integer() {
	// Arrange
	let field = field_with_params(
		"retry_count",
		FieldType::Integer,
		false,
		vec![("default", "0")],
	);
	let schema =
		build_single_model_schema("testapp", "Job", vec![("retry_count".to_string(), field)]);

	// Act
	let table = schema.tables.get("job").expect("table should exist");
	let col = table
		.columns
		.get("retry_count")
		.expect("column should exist");

	// Assert
	assert_eq!(col.default, Some("0".to_string()));
}

// ============================================================================
// Macro Simulation Tests (RT-MACRO-01 to RT-MACRO-06)
//
// These tests simulate what the #[model] macro ACTUALLY produces and assert
// what it SHOULD produce. They will FAIL, exposing bugs #1700, #1701, #1702.
// ============================================================================

#[rstest]
fn test_rt_macro_01_i64_pk_missing_auto_increment() {
	// Arrange — simulate macro output for i64 PK (bug #1700: no auto_increment)
	let field = simulate_macro_i64_pk();
	let schema = build_single_model_schema("testapp", "Article", vec![("id".to_string(), field)]);

	// Act
	let table = schema.tables.get("article").expect("table should exist");
	let col = table.columns.get("id").expect("column should exist");

	// Assert — this SHOULD be true, but macro never sets auto_increment (bug #1700)
	assert!(
		col.auto_increment,
		"BUG #1700: i64 PK should have auto_increment=true, but macro does not set it"
	);
}

#[rstest]
fn test_rt_macro_02_i32_pk_missing_auto_increment() {
	// Arrange — simulate macro output for i32 PK (same bug #1700)
	let field = field_with_params(
		"id",
		FieldType::Integer,
		false,
		vec![("primary_key", "true")],
		// Missing ("auto_increment", "true") — bug #1700
	);
	let schema = build_single_model_schema("testapp", "Comment", vec![("id".to_string(), field)]);

	// Act
	let table = schema.tables.get("comment").expect("table should exist");
	let col = table.columns.get("id").expect("column should exist");

	// Assert — this SHOULD be true for integer PK with auto-increment intent
	assert!(
		col.auto_increment,
		"BUG #1700: i32 PK should have auto_increment=true, but macro does not set it"
	);
}

#[rstest]
fn test_rt_macro_03_non_option_string_nullable_true() {
	// Arrange — simulate macro output for non-Option String (bug #1701: nullable=true)
	let field = simulate_macro_non_option_string();
	let schema = build_single_model_schema("testapp", "Post", vec![("title".to_string(), field)]);

	// Act
	let table = schema.tables.get("post").expect("table should exist");
	let col = table.columns.get("title").expect("column should exist");

	// Assert — non-Option String SHOULD be NOT nullable
	assert!(
		!col.nullable,
		"BUG #1701: non-Option String should be NOT NULL, but macro sets nullable=true"
	);
}

#[rstest]
fn test_rt_macro_04_non_option_integer_nullable_true() {
	// Arrange — simulate macro output for non-Option i32 (bug #1701)
	let field = FieldState::new("age", FieldType::Integer, true); // bug: nullable=true
	let schema = build_single_model_schema("testapp", "Person", vec![("age".to_string(), field)]);

	// Act
	let table = schema.tables.get("person").expect("table should exist");
	let col = table.columns.get("age").expect("column should exist");

	// Assert — non-Option i32 SHOULD be NOT nullable
	assert!(
		!col.nullable,
		"BUG #1701: non-Option i32 should be NOT NULL, but macro sets nullable=true"
	);
}

#[rstest]
fn test_rt_macro_05_datetime_utc_wrong_field_type() {
	// Arrange — simulate macro output for DateTime<Utc> with auto_now_add (bug #1702)
	let field = simulate_macro_datetime_utc_auto_now_add();
	let schema =
		build_single_model_schema("testapp", "Log", vec![("created_at".to_string(), field)]);

	// Act
	let table = schema.tables.get("log").expect("table should exist");
	let col = table
		.columns
		.get("created_at")
		.expect("column should exist");

	// Assert — DateTime<Utc> SHOULD map to TimestampTz, not DateTime
	assert_eq!(
		col.data_type,
		FieldType::TimestampTz,
		"BUG #1702: DateTime<Utc> with auto_now_add should be TimestampTz, not DateTime"
	);
}

#[rstest]
fn test_rt_macro_06_datetime_utc_auto_now_wrong_field_type() {
	// Arrange — simulate macro output for DateTime<Utc> with auto_now (same bug #1702)
	let field = field_with_params(
		"updated_at",
		FieldType::DateTime, // bug: should be TimestampTz
		false,
		vec![("auto_now", "true")],
	);
	let schema =
		build_single_model_schema("testapp", "Record", vec![("updated_at".to_string(), field)]);

	// Act
	let table = schema.tables.get("record").expect("table should exist");
	let col = table
		.columns
		.get("updated_at")
		.expect("column should exist");

	// Assert — DateTime<Utc> SHOULD map to TimestampTz
	assert_eq!(
		col.data_type,
		FieldType::TimestampTz,
		"BUG #1702: DateTime<Utc> with auto_now should be TimestampTz, not DateTime"
	);
}

// ============================================================================
// Edge Cases (RT-EC-01 to RT-EC-09)
// ============================================================================

#[rstest]
fn test_rt_ec_01_empty_model_no_fields() {
	// Arrange
	let schema = build_single_model_schema("testapp", "EmptyModel", vec![]);

	// Act
	let table = schema
		.tables
		.get("empty_model")
		.expect("table should exist");

	// Assert
	assert!(
		table.columns.is_empty(),
		"model with no fields should produce empty columns"
	);
}

#[rstest]
fn test_rt_ec_02_multiple_models_in_project() {
	// Arrange
	let mut state = ProjectState::new();

	let mut model_a = ModelState::new("app1", "Alpha");
	model_a.fields.insert(
		"id".to_string(),
		FieldState::new("id", FieldType::Integer, false),
	);
	state.add_model(model_a);

	let mut model_b = ModelState::new("app2", "Beta");
	model_b.fields.insert(
		"id".to_string(),
		FieldState::new("id", FieldType::BigInteger, false),
	);
	state.add_model(model_b);

	// Act
	let schema = state.to_database_schema();

	// Assert
	assert_eq!(schema.tables.len(), 2);
	assert!(schema.tables.contains_key("alpha"));
	assert!(schema.tables.contains_key("beta"));
}

#[rstest]
fn test_rt_ec_03_primary_key_false_param() {
	// Arrange — explicitly setting primary_key to "false"
	let field = field_with_params(
		"id",
		FieldType::Integer,
		false,
		vec![("primary_key", "false")],
	);
	let schema = build_single_model_schema("testapp", "NonPk", vec![("id".to_string(), field)]);

	// Act
	let table = schema.tables.get("non_pk").expect("table should exist");
	let col = table.columns.get("id").expect("column should exist");

	// Assert
	assert!(
		!col.primary_key,
		"primary_key='false' should not mark as PK"
	);
}

#[rstest]
fn test_rt_ec_04_json_field() {
	// Arrange
	let field = FieldState::new("metadata", FieldType::Json, true);
	let schema =
		build_single_model_schema("testapp", "Config", vec![("metadata".to_string(), field)]);

	// Act
	let table = schema.tables.get("config").expect("table should exist");
	let col = table.columns.get("metadata").expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::Json);
	assert!(col.nullable);
}

#[rstest]
fn test_rt_ec_05_jsonb_field() {
	// Arrange
	let field = FieldState::new("data", FieldType::JsonBinary, false);
	let schema = build_single_model_schema("testapp", "Store", vec![("data".to_string(), field)]);

	// Act
	let table = schema.tables.get("store").expect("table should exist");
	let col = table.columns.get("data").expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::JsonBinary);
}

#[rstest]
fn test_rt_ec_06_model_with_constraint() {
	// Arrange
	let mut state = ProjectState::new();
	let mut model = ModelState::new("testapp", "Booking");
	model.fields.insert(
		"start_date".to_string(),
		FieldState::new("start_date", FieldType::Date, false),
	);
	model.fields.insert(
		"end_date".to_string(),
		FieldState::new("end_date", FieldType::Date, false),
	);
	model.constraints.push(ConstraintDefinition {
		name: "booking_date_check".to_string(),
		constraint_type: "check".to_string(),
		fields: vec!["start_date".to_string(), "end_date".to_string()],
		expression: Some("start_date < end_date".to_string()),
		foreign_key_info: None,
	});
	state.add_model(model);

	// Act
	let schema = state.to_database_schema();
	let table = schema.tables.get("booking").expect("table should exist");

	// Assert
	assert_eq!(table.constraints.len(), 1);
	assert_eq!(table.constraints[0].name, "booking_date_check");
}

#[rstest]
fn test_rt_ec_07_char_fixed_length_field() {
	// Arrange
	let field = FieldState::new("country_code", FieldType::Char(2), false);
	let schema = build_single_model_schema(
		"testapp",
		"Country",
		vec![("country_code".to_string(), field)],
	);

	// Act
	let table = schema.tables.get("country").expect("table should exist");
	let col = table
		.columns
		.get("country_code")
		.expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::Char(2));
}

#[rstest]
fn test_rt_ec_08_all_nullable_model() {
	// Arrange — model where every field is nullable
	let fields = vec![
		(
			"name".to_string(),
			FieldState::new("name", FieldType::VarChar(100), true),
		),
		(
			"email".to_string(),
			FieldState::new("email", FieldType::VarChar(255), true),
		),
		(
			"age".to_string(),
			FieldState::new("age", FieldType::Integer, true),
		),
	];
	let schema = build_single_model_schema("testapp", "OptionalProfile", fields);

	// Act
	let table = schema
		.tables
		.get("optional_profile")
		.expect("table should exist");

	// Assert
	for (_name, col) in &table.columns {
		assert!(col.nullable, "all fields should be nullable");
	}
}

#[rstest]
fn test_rt_ec_09_uuid_primary_key() {
	// Arrange — UUID as primary key (no auto_increment expected)
	let field = field_with_params("id", FieldType::Uuid, false, vec![("primary_key", "true")]);
	let schema = build_single_model_schema("testapp", "Session", vec![("id".to_string(), field)]);

	// Act
	let table = schema.tables.get("session").expect("table should exist");
	let col = table.columns.get("id").expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::Uuid);
	assert!(col.primary_key);
	assert!(!col.auto_increment, "UUID PK should not be auto-increment");
}

// ============================================================================
// PostgreSQL-Specific Type Mapping (RT-HP-25 to RT-HP-38)
// ============================================================================

#[rstest]
fn test_rt_hp_25_real_type() {
	// Arrange
	let field = FieldState::new("value", FieldType::Real, false);
	let schema =
		build_single_model_schema("testapp", "Measurement", vec![("value".to_string(), field)]);

	// Act
	let table = schema
		.tables
		.get("measurement")
		.expect("table should exist");
	let col = table.columns.get("value").expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::Real);
	assert!(!col.nullable);
}

#[rstest]
fn test_rt_hp_26_bytea_non_nullable() {
	// Arrange
	let field = FieldState::new("data", FieldType::Bytea, false);
	let schema =
		build_single_model_schema("testapp", "BinaryData", vec![("data".to_string(), field)]);

	// Act
	let table = schema
		.tables
		.get("binary_data")
		.expect("table should exist");
	let col = table.columns.get("data").expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::Bytea);
	assert!(!col.nullable);
}

#[rstest]
fn test_rt_hp_27_hstore_nullable() {
	// Arrange
	let field = FieldState::new("metadata", FieldType::HStore, true);
	let schema =
		build_single_model_schema("testapp", "KeyValue", vec![("metadata".to_string(), field)]);

	// Act
	let table = schema.tables.get("key_value").expect("table should exist");
	let col = table.columns.get("metadata").expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::HStore);
	assert!(col.nullable);
}

#[rstest]
fn test_rt_hp_28_citext_non_nullable() {
	// Arrange
	let field = FieldState::new("username", FieldType::CIText, false);
	let schema =
		build_single_model_schema("testapp", "Account", vec![("username".to_string(), field)]);

	// Act
	let table = schema.tables.get("account").expect("table should exist");
	let col = table.columns.get("username").expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::CIText);
	assert!(!col.nullable);
}

#[rstest]
fn test_rt_hp_29_array_integer() {
	// Arrange
	let field = FieldState::new(
		"scores",
		FieldType::Array(Box::new(FieldType::Integer)),
		false,
	);
	let schema =
		build_single_model_schema("testapp", "Result", vec![("scores".to_string(), field)]);

	// Act
	let table = schema.tables.get("result").expect("table should exist");
	let col = table.columns.get("scores").expect("column should exist");

	// Assert
	assert_eq!(
		col.data_type,
		FieldType::Array(Box::new(FieldType::Integer))
	);
}

#[rstest]
fn test_rt_hp_30_array_varchar_preserves_inner_param() {
	// Arrange
	let field = FieldState::new(
		"tags",
		FieldType::Array(Box::new(FieldType::VarChar(100))),
		true,
	);
	let schema = build_single_model_schema("testapp", "Article", vec![("tags".to_string(), field)]);

	// Act
	let table = schema.tables.get("article").expect("table should exist");
	let col = table.columns.get("tags").expect("column should exist");

	// Assert
	assert_eq!(
		col.data_type,
		FieldType::Array(Box::new(FieldType::VarChar(100)))
	);
	assert!(col.nullable);
}

#[rstest]
fn test_rt_hp_31_int4range() {
	// Arrange
	let field = FieldState::new("age_range", FieldType::Int4Range, false);
	let schema = build_single_model_schema(
		"testapp",
		"AgeFilter",
		vec![("age_range".to_string(), field)],
	);

	// Act
	let table = schema.tables.get("age_filter").expect("table should exist");
	let col = table.columns.get("age_range").expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::Int4Range);
}

#[rstest]
fn test_rt_hp_32_int8range() {
	// Arrange
	let field = FieldState::new("id_range", FieldType::Int8Range, false);
	let schema =
		build_single_model_schema("testapp", "IdFilter", vec![("id_range".to_string(), field)]);

	// Act
	let table = schema.tables.get("id_filter").expect("table should exist");
	let col = table.columns.get("id_range").expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::Int8Range);
}

#[rstest]
fn test_rt_hp_33_numrange() {
	// Arrange
	let field = FieldState::new("price_range", FieldType::NumRange, false);
	let schema = build_single_model_schema(
		"testapp",
		"PriceFilter",
		vec![("price_range".to_string(), field)],
	);

	// Act
	let table = schema
		.tables
		.get("price_filter")
		.expect("table should exist");
	let col = table
		.columns
		.get("price_range")
		.expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::NumRange);
}

#[rstest]
fn test_rt_hp_34_daterange() {
	// Arrange
	let field = FieldState::new("valid_period", FieldType::DateRange, true);
	let schema = build_single_model_schema(
		"testapp",
		"Contract",
		vec![("valid_period".to_string(), field)],
	);

	// Act
	let table = schema.tables.get("contract").expect("table should exist");
	let col = table
		.columns
		.get("valid_period")
		.expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::DateRange);
	assert!(col.nullable);
}

#[rstest]
fn test_rt_hp_35_tsrange() {
	// Arrange
	let field = FieldState::new("event_period", FieldType::TsRange, false);
	let schema = build_single_model_schema(
		"testapp",
		"Event",
		vec![("event_period".to_string(), field)],
	);

	// Act
	let table = schema.tables.get("event").expect("table should exist");
	let col = table
		.columns
		.get("event_period")
		.expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::TsRange);
}

#[rstest]
fn test_rt_hp_36_tstzrange() {
	// Arrange
	let field = FieldState::new("meeting_time", FieldType::TsTzRange, false);
	let schema = build_single_model_schema(
		"testapp",
		"Meeting",
		vec![("meeting_time".to_string(), field)],
	);

	// Act
	let table = schema.tables.get("meeting").expect("table should exist");
	let col = table
		.columns
		.get("meeting_time")
		.expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::TsTzRange);
}

#[rstest]
fn test_rt_hp_37_tsvector() {
	// Arrange
	let field = FieldState::new("search_vector", FieldType::TsVector, true);
	let schema = build_single_model_schema(
		"testapp",
		"Document",
		vec![("search_vector".to_string(), field)],
	);

	// Act
	let table = schema.tables.get("document").expect("table should exist");
	let col = table
		.columns
		.get("search_vector")
		.expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::TsVector);
	assert!(col.nullable);
}

#[rstest]
fn test_rt_hp_38_tsquery() {
	// Arrange
	let field = FieldState::new("query", FieldType::TsQuery, false);
	let schema =
		build_single_model_schema("testapp", "SearchQuery", vec![("query".to_string(), field)]);

	// Act
	let table = schema
		.tables
		.get("search_query")
		.expect("table should exist");
	let col = table.columns.get("query").expect("column should exist");

	// Assert
	assert_eq!(col.data_type, FieldType::TsQuery);
	assert!(!col.nullable);
}

// ============================================================================
// Table Name Conversion (RT-TN-01 to RT-TN-08)
// ============================================================================

#[rstest]
fn test_rt_tn_01_http_response_table_name() {
	// Arrange
	let mut state = ProjectState::new();
	let m = ModelState::new("testapp", "HTTPResponse");
	state.add_model(m);

	// Act
	let schema = state.to_database_schema();

	// Assert
	assert!(
		schema.tables.contains_key("http_response"),
		"HTTPResponse should map to table 'http_response', got keys: {:?}",
		schema.tables.keys().collect::<Vec<_>>()
	);
}

#[rstest]
fn test_rt_tn_02_api_key_table_name() {
	// Arrange
	let mut state = ProjectState::new();
	let m = ModelState::new("testapp", "APIKey");
	state.add_model(m);

	// Act
	let schema = state.to_database_schema();

	// Assert
	assert!(
		schema.tables.contains_key("api_key"),
		"APIKey should map to table 'api_key', got keys: {:?}",
		schema.tables.keys().collect::<Vec<_>>()
	);
}

#[rstest]
fn test_rt_tn_03_xml_parser_table_name() {
	// Arrange
	let mut state = ProjectState::new();
	let m = ModelState::new("testapp", "XMLParser");
	state.add_model(m);

	// Act
	let schema = state.to_database_schema();

	// Assert
	assert!(
		schema.tables.contains_key("xml_parser"),
		"XMLParser should map to table 'xml_parser', got keys: {:?}",
		schema.tables.keys().collect::<Vec<_>>()
	);
}

#[rstest]
fn test_rt_tn_04_simple_user_table_name() {
	// Arrange
	let mut state = ProjectState::new();
	let m = ModelState::new("testapp", "User");
	state.add_model(m);

	// Act
	let schema = state.to_database_schema();

	// Assert
	assert!(
		schema.tables.contains_key("user"),
		"User should map to table 'user'"
	);
}

#[rstest]
fn test_rt_tn_05_single_word_order_table_name() {
	// Arrange
	let mut state = ProjectState::new();
	let m = ModelState::new("testapp", "Order");
	state.add_model(m);

	// Act
	let schema = state.to_database_schema();

	// Assert
	assert!(
		schema.tables.contains_key("order"),
		"Order should map to table 'order'"
	);
}

#[rstest]
fn test_rt_tn_06_blog_post_table_name() {
	// Arrange
	let mut state = ProjectState::new();
	let m = ModelState::new("testapp", "BlogPost");
	state.add_model(m);

	// Act
	let schema = state.to_database_schema();

	// Assert
	assert!(
		schema.tables.contains_key("blog_post"),
		"BlogPost should map to table 'blog_post'"
	);
}

#[rstest]
fn test_rt_tn_07_my_https_client_table_name() {
	// Arrange
	let mut state = ProjectState::new();
	let m = ModelState::new("testapp", "MyHTTPSClient");
	state.add_model(m);

	// Act
	let schema = state.to_database_schema();

	// Assert — Verify actual conversion output
	let table_name = &ModelState::new("testapp", "MyHTTPSClient").table_name;
	assert!(
		schema.tables.contains_key(table_name.as_str()),
		"MyHTTPSClient should map to table '{}', got keys: {:?}",
		table_name,
		schema.tables.keys().collect::<Vec<_>>()
	);
}

#[rstest]
fn test_rt_tn_08_single_char_table_name() {
	// Arrange
	let mut state = ProjectState::new();
	let m = ModelState::new("testapp", "A");
	state.add_model(m);

	// Act
	let schema = state.to_database_schema();

	// Assert
	assert!(schema.tables.contains_key("a"), "A should map to table 'a'");
}

// ============================================================================
// Additional Macro Simulation (RT-MACRO-07 to RT-MACRO-12)
// ============================================================================

#[rstest]
fn test_rt_macro_07_option_i64_pk() {
	// Arrange — Option<i64> PK: nullable=true, primary_key=true, no auto_increment
	let field = field_with_params(
		"id",
		FieldType::BigInteger,
		true,
		vec![("primary_key", "true")],
	);
	let schema =
		build_single_model_schema("testapp", "NullablePk", vec![("id".to_string(), field)]);

	// Act
	let table = schema
		.tables
		.get("nullable_pk")
		.expect("table should exist");
	let col = table.columns.get("id").expect("column should exist");

	// Assert
	assert!(col.nullable, "Option<i64> PK should be nullable");
	assert!(col.primary_key, "should be primary key");
	assert!(
		!col.auto_increment,
		"nullable PK should not be auto-increment"
	);
}

#[rstest]
fn test_rt_macro_08_non_option_bool_nullable_true() {
	// Arrange — Simulate macro bug #1701: non-Option<bool> produces nullable=true
	let field = field_with_params("active", FieldType::Boolean, true, vec![]);
	let schema = build_single_model_schema("testapp", "Flag", vec![("active".to_string(), field)]);

	// Act
	let table = schema.tables.get("flag").expect("table should exist");
	let col = table.columns.get("active").expect("column should exist");

	// Assert — This SHOULD be false for non-Option bool, but macro bug sets nullable=true
	assert!(
		!col.nullable,
		"non-Option<bool> should not be nullable (bug #1701)"
	);
}

#[rstest]
fn test_rt_macro_09_non_option_f64_nullable_true() {
	// Arrange — Simulate macro bug #1701: non-Option<f64> produces nullable=true
	let field = field_with_params("score", FieldType::Double, true, vec![]);
	let schema = build_single_model_schema("testapp", "Score", vec![("score".to_string(), field)]);

	// Act
	let table = schema.tables.get("score").expect("table should exist");
	let col = table.columns.get("score").expect("column should exist");

	// Assert — This SHOULD be false for non-Option f64, but macro bug sets nullable=true
	assert!(
		!col.nullable,
		"non-Option<f64> should not be nullable (bug #1701)"
	);
}

#[rstest]
fn test_rt_macro_10_non_option_date_nullable_true() {
	// Arrange — Simulate macro bug #1701: non-Option<Date> produces nullable=true
	let field = field_with_params("birthday", FieldType::Date, true, vec![]);
	let schema =
		build_single_model_schema("testapp", "Person", vec![("birthday".to_string(), field)]);

	// Act
	let table = schema.tables.get("person").expect("table should exist");
	let col = table.columns.get("birthday").expect("column should exist");

	// Assert — This SHOULD be false for non-Option Date, but macro bug sets nullable=true
	assert!(
		!col.nullable,
		"non-Option<Date> should not be nullable (bug #1701)"
	);
}

#[rstest]
fn test_rt_macro_11_non_option_time_nullable_true() {
	// Arrange — Simulate macro bug #1701: non-Option<Time> produces nullable=true
	let field = field_with_params("start_time", FieldType::Time, true, vec![]);
	let schema = build_single_model_schema(
		"testapp",
		"Schedule",
		vec![("start_time".to_string(), field)],
	);

	// Act
	let table = schema.tables.get("schedule").expect("table should exist");
	let col = table
		.columns
		.get("start_time")
		.expect("column should exist");

	// Assert — This SHOULD be false for non-Option Time, but macro bug sets nullable=true
	assert!(
		!col.nullable,
		"non-Option<Time> should not be nullable (bug #1701)"
	);
}

#[rstest]
fn test_rt_macro_12_non_option_uuid_nullable_true() {
	// Arrange — Simulate macro bug #1701: non-Option<Uuid> produces nullable=true
	let field = field_with_params("token", FieldType::Uuid, true, vec![]);
	let schema = build_single_model_schema("testapp", "Token", vec![("token".to_string(), field)]);

	// Act
	let table = schema.tables.get("token").expect("table should exist");
	let col = table.columns.get("token").expect("column should exist");

	// Assert — This SHOULD be false for non-Option Uuid, but macro bug sets nullable=true
	assert!(
		!col.nullable,
		"non-Option<Uuid> should not be nullable (bug #1701)"
	);
}

// ============================================================================
// Attribute Combination Tests (RT-COMB-01 to RT-COMB-05)
// ============================================================================

#[rstest]
fn test_rt_comb_01_pk_unique_auto_increment() {
	// Arrange — PK + unique + auto_increment (all three attributes)
	let field = field_with_params(
		"id",
		FieldType::Integer,
		false,
		vec![
			("primary_key", "true"),
			("unique", "true"),
			("auto_increment", "true"),
		],
	);
	let schema =
		build_single_model_schema("testapp", "UniqueAuto", vec![("id".to_string(), field)]);

	// Act
	let table = schema
		.tables
		.get("unique_auto")
		.expect("table should exist");
	let col = table.columns.get("id").expect("column should exist");

	// Assert
	assert!(col.primary_key, "should be primary key");
	assert!(col.auto_increment, "should be auto-increment");
}

#[rstest]
fn test_rt_comb_02_nullable_default_null() {
	// Arrange — nullable + default="NULL"
	let field = field_with_params("bio", FieldType::Text, true, vec![("default", "NULL")]);
	let schema = build_single_model_schema("testapp", "Profile", vec![("bio".to_string(), field)]);

	// Act
	let table = schema.tables.get("profile").expect("table should exist");
	let col = table.columns.get("bio").expect("column should exist");

	// Assert
	assert!(col.nullable, "should be nullable");
	assert_eq!(
		col.default.as_deref(),
		Some("NULL"),
		"default should be NULL"
	);
}

#[rstest]
fn test_rt_comb_03_unique_default_value() {
	// Arrange — unique + default="draft"
	let field = field_with_params(
		"slug",
		FieldType::VarChar(255),
		false,
		vec![("unique", "true"), ("default", "draft")],
	);
	let schema = build_single_model_schema("testapp", "Page", vec![("slug".to_string(), field)]);

	// Act
	let table = schema.tables.get("page").expect("table should exist");
	let col = table.columns.get("slug").expect("column should exist");

	// Assert
	assert_eq!(
		col.default.as_deref(),
		Some("draft"),
		"default should be 'draft'"
	);
	assert!(!col.nullable);
}

#[rstest]
fn test_rt_comb_04_pk_nullable_contradictory() {
	// Arrange — PK + nullable (contradictory but allowed at state level)
	let field = field_with_params(
		"id",
		FieldType::Integer,
		true,
		vec![("primary_key", "true")],
	);
	let schema = build_single_model_schema("testapp", "Oddity", vec![("id".to_string(), field)]);

	// Act
	let table = schema.tables.get("oddity").expect("table should exist");
	let col = table.columns.get("id").expect("column should exist");

	// Assert — Verify both properties are preserved
	assert!(col.primary_key, "should be primary key");
	assert!(col.nullable, "nullable should be preserved as set");
}

#[rstest]
fn test_rt_comb_05_multiple_indexed_fields() {
	// Arrange — Multiple indexed fields on same model
	let field_email = FieldState::new("email", FieldType::VarChar(255), false);
	let field_username = FieldState::new("username", FieldType::VarChar(100), false);
	let field_name = FieldState::new("full_name", FieldType::VarChar(200), false);

	let mut state = ProjectState::new();
	let mut m = ModelState::new("testapp", "IndexedUser");
	m.fields.insert("email".to_string(), field_email);
	m.fields.insert("username".to_string(), field_username);
	m.fields.insert("full_name".to_string(), field_name);
	m.indexes.push(IndexDefinition {
		name: "idx_email".to_string(),
		fields: vec!["email".to_string()],
		unique: true,
	});
	m.indexes.push(IndexDefinition {
		name: "idx_username".to_string(),
		fields: vec!["username".to_string()],
		unique: true,
	});
	m.indexes.push(IndexDefinition {
		name: "idx_full_name".to_string(),
		fields: vec!["full_name".to_string()],
		unique: false,
	});
	state.add_model(m);
	let schema = state.to_database_schema();

	// Act
	let table = schema
		.tables
		.get("indexed_user")
		.expect("table should exist");

	// Assert — All three columns should exist
	assert!(table.columns.contains_key("email"));
	assert!(table.columns.contains_key("username"));
	assert!(table.columns.contains_key("full_name"));
	assert_eq!(table.indexes.len(), 3, "should have 3 indexes");
}

// ============================================================================
// Multi-App Schema Tests (RT-APP-01 to RT-APP-03)
// ============================================================================

#[rstest]
fn test_rt_app_01_two_apps_same_model_name() {
	// Arrange — Two apps with same model name "User"
	let mut state = ProjectState::new();

	let mut m1 = ModelState::new("auth", "User");
	m1.fields.insert(
		"name".to_string(),
		FieldState::new("name", FieldType::VarChar(100), false),
	);
	state.add_model(m1);

	let mut m2 = ModelState::new("blog", "User");
	m2.fields.insert(
		"name".to_string(),
		FieldState::new("name", FieldType::VarChar(100), false),
	);
	state.add_model(m2);

	// Act
	let schema = state.to_database_schema();

	// Assert — Both tables should exist (table names are derived from model name)
	assert!(
		schema.tables.len() >= 2,
		"should have at least 2 tables, got {}",
		schema.tables.len()
	);
}

#[rstest]
fn test_rt_app_02_to_database_schema_for_app_filters() {
	// Arrange — Two apps, filter by one
	let mut state = ProjectState::new();

	let mut m1 = ModelState::new("auth", "User");
	m1.fields.insert(
		"name".to_string(),
		FieldState::new("name", FieldType::VarChar(100), false),
	);
	state.add_model(m1);

	let mut m2 = ModelState::new("blog", "Post");
	m2.fields.insert(
		"title".to_string(),
		FieldState::new("title", FieldType::VarChar(200), false),
	);
	state.add_model(m2);

	// Act
	let auth_schema = state.to_database_schema_for_app("auth");
	let blog_schema = state.to_database_schema_for_app("blog");

	// Assert
	assert_eq!(
		auth_schema.tables.len(),
		1,
		"auth schema should have 1 table"
	);
	assert!(
		auth_schema.tables.contains_key("user"),
		"auth schema should contain 'user' table"
	);
	assert_eq!(
		blog_schema.tables.len(),
		1,
		"blog schema should have 1 table"
	);
	assert!(
		blog_schema.tables.contains_key("post"),
		"blog schema should contain 'post' table"
	);
}

#[rstest]
fn test_rt_app_03_three_apps_six_models() {
	// Arrange — Three apps, each with 2 models → 6 tables total
	let mut state = ProjectState::new();

	for (app, models) in [
		("auth", vec![("User", "name"), ("Group", "label")]),
		("blog", vec![("Post", "title"), ("Comment", "body")]),
		("shop", vec![("Product", "sku"), ("Order", "total")]),
	] {
		for (model, field_name) in models {
			let mut m = ModelState::new(app, model);
			m.fields.insert(
				field_name.to_string(),
				FieldState::new(field_name, FieldType::VarChar(100), false),
			);
			state.add_model(m);
		}
	}

	// Act
	let schema = state.to_database_schema();

	// Assert
	assert_eq!(schema.tables.len(), 6, "should have 6 tables total");
	assert!(schema.tables.contains_key("user"));
	assert!(schema.tables.contains_key("group"));
	assert!(schema.tables.contains_key("post"));
	assert!(schema.tables.contains_key("comment"));
	assert!(schema.tables.contains_key("product"));
	assert!(schema.tables.contains_key("order"));
}
