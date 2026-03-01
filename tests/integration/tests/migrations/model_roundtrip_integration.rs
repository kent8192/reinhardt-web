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

use reinhardt_db::migrations::{
	ConstraintDefinition, DatabaseSchema, FieldState, FieldType, ModelState, ProjectState,
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
