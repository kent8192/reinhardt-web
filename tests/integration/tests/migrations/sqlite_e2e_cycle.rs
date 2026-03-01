//! SQLite end-to-end cycle integration tests for migration system
//!
//! Tests the full cycle: define schema -> apply migration -> INSERT/SELECT data -> verify round-trip.
//! Validates that the migration system produces real, usable SQLite database tables.
//!
//! Adapted from the PostgreSQL e2e cycle tests with SQLite-specific behaviors:
//! - In-memory database (no container needed)
//! - `?` placeholders instead of `$1`, `$2`
//! - No VARCHAR length enforcement
//! - Boolean stored as INTEGER (0/1)
//! - No UUID native type
//! - No RETURNING clause

use reinhardt_db::backends::connection::DatabaseConnection;
use reinhardt_db::backends::types::QueryValue;
use reinhardt_db::migrations::{
	ColumnDefinition, FieldType, Migration, Operation,
	executor::DatabaseMigrationExecutor,
};
use rstest::*;
use std::sync::Arc;

// ============================================================================
// Helper functions
// ============================================================================

/// Create a column definition with common defaults
fn col(name: &str, type_def: FieldType) -> ColumnDefinition {
	ColumnDefinition {
		name: name.to_string(),
		type_definition: type_def,
		not_null: false,
		unique: false,
		primary_key: false,
		auto_increment: false,
		default: None,
	}
}

/// Create a NOT NULL column definition
fn col_nn(name: &str, type_def: FieldType) -> ColumnDefinition {
	ColumnDefinition {
		name: name.to_string(),
		type_definition: type_def,
		not_null: true,
		unique: false,
		primary_key: false,
		auto_increment: false,
		default: None,
	}
}

/// Create an auto-increment primary key column
fn col_pk_auto(name: &str) -> ColumnDefinition {
	ColumnDefinition {
		name: name.to_string(),
		type_definition: FieldType::Integer,
		not_null: true,
		unique: false,
		primary_key: true,
		auto_increment: true,
		default: None,
	}
}

/// Create a simple CreateTable operation
fn create_table(name: &str, columns: Vec<ColumnDefinition>) -> Operation {
	Operation::CreateTable {
		name: name.to_string(),
		columns,
		constraints: vec![],
		without_rowid: None,
		interleave_in_parent: None,
		partition: None,
	}
}

/// Create a nullable column definition
fn col_nullable(name: &str, type_def: FieldType) -> ColumnDefinition {
	ColumnDefinition {
		name: name.to_string(),
		type_definition: type_def,
		not_null: false,
		unique: false,
		primary_key: false,
		auto_increment: false,
		default: None,
	}
}

/// Create a column definition with a default value
fn col_default(name: &str, type_def: FieldType, default: &str) -> ColumnDefinition {
	ColumnDefinition {
		name: name.to_string(),
		type_definition: type_def,
		not_null: false,
		unique: false,
		primary_key: false,
		auto_increment: false,
		default: Some(default.to_string()),
	}
}

/// Create a unique NOT NULL column definition
fn col_unique(name: &str, type_def: FieldType) -> ColumnDefinition {
	ColumnDefinition {
		name: name.to_string(),
		type_definition: type_def,
		not_null: true,
		unique: true,
		primary_key: false,
		auto_increment: false,
		default: None,
	}
}

/// Connect to in-memory SQLite and create a migration executor
async fn setup_sqlite() -> (Arc<DatabaseConnection>, DatabaseMigrationExecutor) {
	let connection = DatabaseConnection::connect_sqlite("sqlite::memory:")
		.await
		.expect("Failed to connect to in-memory SQLite");
	let conn = Arc::new(connection.clone());
	let executor = DatabaseMigrationExecutor::new(connection);
	(conn, executor)
}

/// Apply a schema (list of operations) via migration executor
async fn apply_schema(executor: &mut DatabaseMigrationExecutor, operations: Vec<Operation>) {
	let migration = Migration::new("test_migration", operations);
	executor
		.apply(&migration)
		.await
		.expect("Failed to apply migration");
}

// ============================================================================
// Happy Path Tests (SE2E-HP-01 to SE2E-HP-20, skipping HP-09)
// ============================================================================

/// SE2E-HP-01: Basic CRUD - create table, insert, select, verify values match
#[rstest]
#[tokio::test]
async fn test_se2e_hp01_basic_crud() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_hp01_table",
		vec![
			col_pk_auto("id"),
			col_nn("name", FieldType::VarChar(100)),
			ColumnDefinition {
				name: "active".to_string(),
				type_definition: FieldType::Boolean,
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: Some("false".to_string()),
			},
		],
	)];
	apply_schema(&mut executor, schema).await;

	// Act
	conn.execute(
		"INSERT INTO se2e_hp01_table (name, active) VALUES (?, ?)",
		vec!["test".into(), true.into()],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all(
			"SELECT id, name, active FROM se2e_hp01_table WHERE name = ?",
			vec!["test".into()],
		)
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	let id: i64 = rows[0].get("id").unwrap();
	let name: String = rows[0].get("name").unwrap();
	let active: bool = rows[0].get("active").unwrap();
	assert!(id > 0);
	assert_eq!(name, "test");
	assert_eq!(active, true);
}

/// SE2E-HP-02: Auto-increment PK - insert without specifying id, verify id > 0
#[rstest]
#[tokio::test]
async fn test_se2e_hp02_auto_increment_pk() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_hp02_table",
		vec![col_pk_auto("id"), col_nn("label", FieldType::VarChar(50))],
	)];
	apply_schema(&mut executor, schema).await;

	// Act
	conn.execute(
		"INSERT INTO se2e_hp02_table (label) VALUES (?)",
		vec!["auto_test".into()],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all("SELECT id FROM se2e_hp02_table", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	let id: i64 = rows[0].get("id").unwrap();
	assert!(id > 0);
}

/// SE2E-HP-03: auto_now_add timestamp - insert with default CURRENT_TIMESTAMP, verify non-null
#[rstest]
#[tokio::test]
async fn test_se2e_hp03_auto_now_add_timestamp() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_hp03_table",
		vec![
			col_pk_auto("id"),
			ColumnDefinition {
				name: "created_at".to_string(),
				type_definition: FieldType::TimestampTz,
				not_null: true,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: Some("CURRENT_TIMESTAMP".to_string()),
			},
		],
	)];
	apply_schema(&mut executor, schema).await;

	// Act - insert without specifying created_at
	conn.execute(
		"INSERT INTO se2e_hp03_table (id) VALUES (NULL)",
		vec![],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all("SELECT id, created_at FROM se2e_hp03_table", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	// created_at should be non-null - it will come back as a string in SQLite
	let created_at_value = rows[0].data.get("created_at").unwrap();
	assert_ne!(*created_at_value, QueryValue::Null);
}

/// SE2E-HP-04: Multiple rows - insert 3 rows, verify count and values
#[rstest]
#[tokio::test]
async fn test_se2e_hp04_multiple_rows() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_hp04_table",
		vec![col_pk_auto("id"), col_nn("value", FieldType::VarChar(50))],
	)];
	apply_schema(&mut executor, schema).await;

	// Act
	for val in &["alpha", "beta", "gamma"] {
		conn.execute(
			"INSERT INTO se2e_hp04_table (value) VALUES (?)",
			vec![(*val).into()],
		)
		.await
		.unwrap();
	}

	let rows = conn
		.fetch_all("SELECT id, value FROM se2e_hp04_table ORDER BY id", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 3);
	assert_eq!(rows[0].get::<String>("value").unwrap(), "alpha");
	assert_eq!(rows[1].get::<String>("value").unwrap(), "beta");
	assert_eq!(rows[2].get::<String>("value").unwrap(), "gamma");
}

/// SE2E-HP-05: Boolean default - insert without bool field, verify default applied
#[rstest]
#[tokio::test]
async fn test_se2e_hp05_boolean_default() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_hp05_table",
		vec![
			col_pk_auto("id"),
			ColumnDefinition {
				name: "is_active".to_string(),
				type_definition: FieldType::Boolean,
				not_null: true,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: Some("false".to_string()),
			},
		],
	)];
	apply_schema(&mut executor, schema).await;

	// Act - insert without specifying is_active
	conn.execute(
		"INSERT INTO se2e_hp05_table (id) VALUES (NULL)",
		vec![],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all("SELECT is_active FROM se2e_hp05_table", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	let is_active: bool = rows[0].get("is_active").unwrap();
	assert_eq!(is_active, false);
}

/// SE2E-HP-06: Integer default - insert without int field, verify default applied
#[rstest]
#[tokio::test]
async fn test_se2e_hp06_integer_default() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_hp06_table",
		vec![
			col_pk_auto("id"),
			ColumnDefinition {
				name: "score".to_string(),
				type_definition: FieldType::Integer,
				not_null: true,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: Some("0".to_string()),
			},
		],
	)];
	apply_schema(&mut executor, schema).await;

	// Act
	conn.execute(
		"INSERT INTO se2e_hp06_table (id) VALUES (NULL)",
		vec![],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all("SELECT score FROM se2e_hp06_table", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	let score: i64 = rows[0].get("score").unwrap();
	assert_eq!(score, 0);
}

/// SE2E-HP-07: Nullable field NULL - insert NULL, verify field is NULL
#[rstest]
#[tokio::test]
async fn test_se2e_hp07_nullable_field_null() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_hp07_table",
		vec![col_pk_auto("id"), col("description", FieldType::Text)],
	)];
	apply_schema(&mut executor, schema).await;

	// Act
	conn.execute(
		"INSERT INTO se2e_hp07_table (description) VALUES (NULL)",
		vec![],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all("SELECT description FROM se2e_hp07_table", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	let desc = rows[0].data.get("description").unwrap();
	assert_eq!(*desc, QueryValue::Null);
}

/// SE2E-HP-08: Nullable field with value - insert actual value, verify preserved
#[rstest]
#[tokio::test]
async fn test_se2e_hp08_nullable_field_with_value() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_hp08_table",
		vec![col_pk_auto("id"), col("description", FieldType::Text)],
	)];
	apply_schema(&mut executor, schema).await;

	// Act
	conn.execute(
		"INSERT INTO se2e_hp08_table (description) VALUES (?)",
		vec!["some description".into()],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all("SELECT description FROM se2e_hp08_table", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	let desc: String = rows[0].get("description").unwrap();
	assert_eq!(desc, "some description");
}

// HP-09 (UUID PK auto-gen) is SKIPPED for SQLite - no native UUID type

/// SE2E-HP-10: Multi-type table - table with many types, verify all round-trip
#[rstest]
#[tokio::test]
async fn test_se2e_hp10_multi_type_table() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_hp10_table",
		vec![
			col_pk_auto("id"),
			col_nn("int_val", FieldType::Integer),
			col_nn("str_val", FieldType::VarChar(100)),
			col_nn("bool_val", FieldType::Boolean),
			col_nn("float_val", FieldType::Float),
			col_nn("double_val", FieldType::Double),
		],
	)];
	apply_schema(&mut executor, schema).await;

	// Act
	conn.execute(
		"INSERT INTO se2e_hp10_table (int_val, str_val, bool_val, float_val, double_val) VALUES (?, ?, ?, ?, ?)",
		vec![
			42i32.into(),
			"hello".into(),
			true.into(),
			QueryValue::Float(3.14),
			QueryValue::Float(2.718281828),
		],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all("SELECT * FROM se2e_hp10_table", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	let int_val: i64 = rows[0].get("int_val").unwrap();
	assert_eq!(int_val, 42);
	let str_val: String = rows[0].get("str_val").unwrap();
	assert_eq!(str_val, "hello");
	let bool_val: bool = rows[0].get("bool_val").unwrap();
	assert_eq!(bool_val, true);
	let float_val: f64 = rows[0].get("float_val").unwrap();
	assert!((float_val - 3.14).abs() < 0.01);
	let double_val: f64 = rows[0].get("double_val").unwrap();
	assert!((double_val - 2.718281828).abs() < 0.000001);
}

/// SE2E-HP-11: Decimal(10,2) INSERT and SELECT roundtrip
#[rstest]
#[tokio::test]
async fn test_se2e_hp11_decimal_roundtrip() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_hp11_table",
		vec![
			col_pk_auto("id"),
			col_nn(
				"price",
				FieldType::Decimal {
					precision: 10,
					scale: 2,
				},
			),
		],
	)];
	apply_schema(&mut executor, schema).await;

	// Act - SQLite stores decimal as text or real
	conn.execute(
		"INSERT INTO se2e_hp11_table (price) VALUES (?)",
		vec!["123.45".into()],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all("SELECT price FROM se2e_hp11_table", vec![])
		.await
		.unwrap();

	// Assert - SQLite may return as string or float depending on storage
	assert_eq!(rows.len(), 1);
	let price_val = rows[0].data.get("price").unwrap();
	match price_val {
		QueryValue::String(s) => assert_eq!(s, "123.45"),
		QueryValue::Float(f) => assert!((f - 123.45).abs() < 0.001),
		other => panic!("Unexpected price type: {:?}", other),
	}
}

/// SE2E-HP-12: Text type INSERT large text and SELECT roundtrip
#[rstest]
#[tokio::test]
async fn test_se2e_hp12_large_text_roundtrip() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_hp12_table",
		vec![col_pk_auto("id"), col_nn("content", FieldType::Text)],
	)];
	apply_schema(&mut executor, schema).await;
	let large_text = "a".repeat(1000);

	// Act
	conn.execute(
		"INSERT INTO se2e_hp12_table (content) VALUES (?)",
		vec![large_text.clone().into()],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all("SELECT content FROM se2e_hp12_table", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	let content: String = rows[0].get("content").unwrap();
	assert_eq!(content.len(), 1000);
	assert_eq!(content, large_text);
}

/// SE2E-HP-13: SmallInteger INSERT and SELECT roundtrip
#[rstest]
#[tokio::test]
async fn test_se2e_hp13_smallinteger_roundtrip() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_hp13_table",
		vec![
			col_pk_auto("id"),
			col_nn("small_val", FieldType::SmallInteger),
		],
	)];
	apply_schema(&mut executor, schema).await;

	// Act
	conn.execute(
		"INSERT INTO se2e_hp13_table (small_val) VALUES (?)",
		vec![32000i32.into()],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all("SELECT small_val FROM se2e_hp13_table", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	let val: i64 = rows[0].get("small_val").unwrap();
	assert_eq!(val, 32000);
}

/// SE2E-HP-14: Date INSERT and SELECT roundtrip
#[rstest]
#[tokio::test]
async fn test_se2e_hp14_date_roundtrip() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_hp14_table",
		vec![col_pk_auto("id"), col_nn("birth_date", FieldType::Date)],
	)];
	apply_schema(&mut executor, schema).await;

	// Act - SQLite stores dates as text
	conn.execute(
		"INSERT INTO se2e_hp14_table (birth_date) VALUES (?)",
		vec!["2026-03-02".into()],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all("SELECT birth_date FROM se2e_hp14_table", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	let date: String = rows[0].get("birth_date").unwrap();
	assert_eq!(date, "2026-03-02");
}

/// SE2E-HP-15: Time INSERT and SELECT roundtrip
#[rstest]
#[tokio::test]
async fn test_se2e_hp15_time_roundtrip() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_hp15_table",
		vec![col_pk_auto("id"), col_nn("event_time", FieldType::Time)],
	)];
	apply_schema(&mut executor, schema).await;

	// Act - SQLite stores time as text
	conn.execute(
		"INSERT INTO se2e_hp15_table (event_time) VALUES (?)",
		vec!["14:30:00".into()],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all("SELECT event_time FROM se2e_hp15_table", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	let time: String = rows[0].get("event_time").unwrap();
	assert_eq!(time, "14:30:00");
}

/// SE2E-HP-16: Boolean INSERT true/false and SELECT roundtrip
#[rstest]
#[tokio::test]
async fn test_se2e_hp16_boolean_roundtrip() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_hp16_table",
		vec![col_pk_auto("id"), col_nn("flag", FieldType::Boolean)],
	)];
	apply_schema(&mut executor, schema).await;

	// Act
	conn.execute(
		"INSERT INTO se2e_hp16_table (flag) VALUES (?)",
		vec![true.into()],
	)
	.await
	.unwrap();
	conn.execute(
		"INSERT INTO se2e_hp16_table (flag) VALUES (?)",
		vec![false.into()],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all("SELECT flag FROM se2e_hp16_table ORDER BY id", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 2);
	let flag0: bool = rows[0].get("flag").unwrap();
	let flag1: bool = rows[1].get("flag").unwrap();
	assert_eq!(flag0, true);
	assert_eq!(flag1, false);
}

/// SE2E-HP-17: Multiple nullable columns with mixed NULL and non-NULL values
#[rstest]
#[tokio::test]
async fn test_se2e_hp17_nullable_columns() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_hp17_table",
		vec![
			col_pk_auto("id"),
			col_nullable("name", FieldType::VarChar(100)),
			col_nullable("age", FieldType::Integer),
			col_nullable("email", FieldType::VarChar(200)),
		],
	)];
	apply_schema(&mut executor, schema).await;

	// Act - insert with some NULL values
	conn.execute(
		"INSERT INTO se2e_hp17_table (name, age, email) VALUES (?, NULL, ?)",
		vec!["Alice".into(), "alice@example.com".into()],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all("SELECT name, age, email FROM se2e_hp17_table", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	let name: String = rows[0].get("name").unwrap();
	assert_eq!(name, "Alice");
	let age = rows[0].data.get("age").unwrap();
	assert_eq!(*age, QueryValue::Null);
	let email: String = rows[0].get("email").unwrap();
	assert_eq!(email, "alice@example.com");
}

/// SE2E-HP-18: Default values for multiple types applied when columns omitted
#[rstest]
#[tokio::test]
async fn test_se2e_hp18_default_values() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_hp18_table",
		vec![
			col_pk_auto("id"),
			col_nn("name", FieldType::VarChar(100)),
			col_default("status", FieldType::VarChar(20), "'active'"),
			col_default("score", FieldType::Integer, "0"),
			col_default("verified", FieldType::Boolean, "false"),
		],
	)];
	apply_schema(&mut executor, schema).await;

	// Act - insert without default columns
	conn.execute(
		"INSERT INTO se2e_hp18_table (name) VALUES (?)",
		vec!["Bob".into()],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all(
			"SELECT name, status, score, verified FROM se2e_hp18_table",
			vec![],
		)
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	let name: String = rows[0].get("name").unwrap();
	assert_eq!(name, "Bob");
	let status: String = rows[0].get("status").unwrap();
	assert_eq!(status, "active");
	let score: i64 = rows[0].get("score").unwrap();
	assert_eq!(score, 0);
	let verified: bool = rows[0].get("verified").unwrap();
	assert_eq!(verified, false);
}

/// SE2E-HP-19: Unique constraint with INSERT then UPDATE preserving uniqueness
#[rstest]
#[tokio::test]
async fn test_se2e_hp19_unique_update() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_hp19_table",
		vec![
			col_pk_auto("id"),
			col_unique("code", FieldType::VarChar(50)),
		],
	)];
	apply_schema(&mut executor, schema).await;

	// Act - insert and then update the unique column
	conn.execute(
		"INSERT INTO se2e_hp19_table (code) VALUES (?)",
		vec!["ABC".into()],
	)
	.await
	.unwrap();
	conn.execute(
		"UPDATE se2e_hp19_table SET code = ? WHERE code = ?",
		vec!["XYZ".into(), "ABC".into()],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all("SELECT code FROM se2e_hp19_table", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	let code: String = rows[0].get("code").unwrap();
	assert_eq!(code, "XYZ");
}

/// SE2E-HP-20: INSERT row with all supported basic types at once
#[rstest]
#[tokio::test]
async fn test_se2e_hp20_all_basic_types() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_hp20_table",
		vec![
			col_pk_auto("id"),
			col_nn("int_col", FieldType::Integer),
			col_nn("varchar_col", FieldType::VarChar(100)),
			col_nn("bool_col", FieldType::Boolean),
			col_nn("float_col", FieldType::Float),
			col_nn("double_col", FieldType::Double),
			col_nn("date_col", FieldType::Date),
		],
	)];
	apply_schema(&mut executor, schema).await;

	// Act
	conn.execute(
		"INSERT INTO se2e_hp20_table (int_col, varchar_col, bool_col, float_col, double_col, date_col) \
		 VALUES (?, ?, ?, ?, ?, ?)",
		vec![
			42i32.into(),
			"hello".into(),
			true.into(),
			QueryValue::Float(3.14),
			QueryValue::Float(2.71828),
			"2026-03-02".into(),
		],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all(
			"SELECT int_col, varchar_col, bool_col, float_col, double_col, date_col FROM se2e_hp20_table",
			vec![],
		)
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	let int_col: i64 = rows[0].get("int_col").unwrap();
	assert_eq!(int_col, 42);
	let varchar_col: String = rows[0].get("varchar_col").unwrap();
	assert_eq!(varchar_col, "hello");
	let bool_col: bool = rows[0].get("bool_col").unwrap();
	assert_eq!(bool_col, true);
	let float_col: f64 = rows[0].get("float_col").unwrap();
	assert!((float_col - 3.14).abs() < 0.01);
	let double_col: f64 = rows[0].get("double_col").unwrap();
	assert!((double_col - 2.71828).abs() < 0.00001);
	let date_col: String = rows[0].get("date_col").unwrap();
	assert_eq!(date_col, "2026-03-02");
}

// ============================================================================
// Error Path Tests (SE2E-EP-01 to SE2E-EP-08)
// ============================================================================

/// SE2E-EP-01: INSERT NULL for NOT NULL field - expect DB error
#[rstest]
#[tokio::test]
async fn test_se2e_ep01_null_for_not_null() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_ep01_table",
		vec![
			col_pk_auto("id"),
			col_nn("required_field", FieldType::VarChar(100)),
		],
	)];
	apply_schema(&mut executor, schema).await;

	// Act
	let result = conn
		.execute(
			"INSERT INTO se2e_ep01_table (required_field) VALUES (NULL)",
			vec![],
		)
		.await;

	// Assert
	assert!(result.is_err());
}

/// SE2E-EP-02: INSERT duplicate unique value - expect unique constraint error
#[rstest]
#[tokio::test]
async fn test_se2e_ep02_duplicate_unique() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_ep02_table",
		vec![
			col_pk_auto("id"),
			ColumnDefinition {
				name: "email".to_string(),
				type_definition: FieldType::VarChar(255),
				not_null: true,
				unique: true,
				primary_key: false,
				auto_increment: false,
				default: None,
			},
		],
	)];
	apply_schema(&mut executor, schema).await;

	// Act - first insert succeeds
	conn.execute(
		"INSERT INTO se2e_ep02_table (email) VALUES (?)",
		vec!["user@example.com".into()],
	)
	.await
	.unwrap();

	// Second insert with same value should fail
	let result = conn
		.execute(
			"INSERT INTO se2e_ep02_table (email) VALUES (?)",
			vec!["user@example.com".into()],
		)
		.await;

	// Assert
	assert!(result.is_err());
}

/// SE2E-EP-03: INSERT 256 chars into VarChar(255) - SQLite does NOT enforce length limits
/// This is a SQLite-specific behavior: VarChar length is not enforced
#[rstest]
#[tokio::test]
async fn test_se2e_ep03_varchar_overflow_allowed() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_ep03_table",
		vec![
			col_pk_auto("id"),
			col_nn("short_text", FieldType::VarChar(255)),
		],
	)];
	apply_schema(&mut executor, schema).await;

	// Act - insert string exceeding varchar length; SQLite allows this
	let long_string = "x".repeat(256);
	let result = conn
		.execute(
			"INSERT INTO se2e_ep03_table (short_text) VALUES (?)",
			vec![long_string.clone().into()],
		)
		.await;

	// Assert - SQLite is permissive with text lengths
	assert!(result.is_ok());

	let rows = conn
		.fetch_all("SELECT short_text FROM se2e_ep03_table", vec![])
		.await
		.unwrap();
	assert_eq!(rows.len(), 1);
	let retrieved: String = rows[0].get("short_text").unwrap();
	assert_eq!(retrieved.len(), 256);
}

/// SE2E-EP-04: INSERT string into integer column - SQLite is loosely typed
/// SQLite may accept or coerce types depending on type affinity
#[rstest]
#[tokio::test]
async fn test_se2e_ep04_type_mismatch_loose() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_ep04_table",
		vec![col_pk_auto("id"), col_nn("amount", FieldType::Integer)],
	)];
	apply_schema(&mut executor, schema).await;

	// Act - SQLite has type affinity, not strict types.
	// Inserting a non-numeric string into INTEGER column succeeds due to loose typing.
	let result = conn
		.execute(
			"INSERT INTO se2e_ep04_table (amount) VALUES ('not_a_number')",
			vec![],
		)
		.await;

	// Assert - SQLite allows this due to type affinity (stores as text)
	assert!(result.is_ok());
}

/// SE2E-EP-05: SELECT from non-existent table - expect DB error
#[rstest]
#[tokio::test]
async fn test_se2e_ep05_select_nonexistent_table() {
	// Arrange
	let (conn, _executor) = setup_sqlite().await;

	// Act
	let result = conn
		.fetch_all("SELECT * FROM se2e_ep05_nonexistent_table", vec![])
		.await;

	// Assert
	assert!(result.is_err());
}

/// SE2E-EP-06: UPDATE to violate unique constraint returns error
#[rstest]
#[tokio::test]
async fn test_se2e_ep06_update_unique_violation() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_ep06_table",
		vec![
			col_pk_auto("id"),
			col_unique("code", FieldType::VarChar(50)),
		],
	)];
	apply_schema(&mut executor, schema).await;

	conn.execute(
		"INSERT INTO se2e_ep06_table (code) VALUES (?)",
		vec!["AAA".into()],
	)
	.await
	.unwrap();
	conn.execute(
		"INSERT INTO se2e_ep06_table (code) VALUES (?)",
		vec!["BBB".into()],
	)
	.await
	.unwrap();

	// Act - update second row to conflict with first row
	let result = conn
		.execute(
			"UPDATE se2e_ep06_table SET code = ? WHERE code = ?",
			vec!["AAA".into(), "BBB".into()],
		)
		.await;

	// Assert
	assert!(result.is_err());
}

/// SE2E-EP-07: INSERT SmallInteger overflow value - SQLite does not enforce range limits
/// SQLite stores all integers as up to 8-byte signed integers
#[rstest]
#[tokio::test]
async fn test_se2e_ep07_smallinteger_overflow_allowed() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_ep07_table",
		vec![
			col_pk_auto("id"),
			col_nn("small_val", FieldType::SmallInteger),
		],
	)];
	apply_schema(&mut executor, schema).await;

	// Act - 40000 exceeds SmallInteger max (32767) but SQLite allows it
	let result = conn
		.execute(
			"INSERT INTO se2e_ep07_table (small_val) VALUES (?)",
			vec![40000i32.into()],
		)
		.await;

	// Assert - SQLite does not enforce SmallInteger range
	assert!(result.is_ok());

	let rows = conn
		.fetch_all("SELECT small_val FROM se2e_ep07_table", vec![])
		.await
		.unwrap();
	assert_eq!(rows.len(), 1);
	let val: i64 = rows[0].get("small_val").unwrap();
	assert_eq!(val, 40000);
}

/// SE2E-EP-08: DELETE non-existent row succeeds with 0 rows affected
#[rstest]
#[tokio::test]
async fn test_se2e_ep08_delete_nonexistent_row() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_ep08_table",
		vec![col_pk_auto("id"), col_nn("name", FieldType::VarChar(50))],
	)];
	apply_schema(&mut executor, schema).await;

	// Act - delete from empty table
	let result = conn
		.execute(
			"DELETE FROM se2e_ep08_table WHERE id = ?",
			vec![999i32.into()],
		)
		.await
		.unwrap();

	// Assert
	assert_eq!(result.rows_affected, 0);
}

// ============================================================================
// Edge Case Tests (SE2E-EC-01 to SE2E-EC-10)
// ============================================================================

/// SE2E-EC-01: SELECT from empty table - expect 0 rows
#[rstest]
#[tokio::test]
async fn test_se2e_ec01_select_empty_table() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_ec01_table",
		vec![col_pk_auto("id"), col("name", FieldType::Text)],
	)];
	apply_schema(&mut executor, schema).await;

	// Act
	let rows = conn
		.fetch_all("SELECT * FROM se2e_ec01_table", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 0);
}

/// SE2E-EC-02: INSERT exactly 255 chars into VarChar(255) - expect success
#[rstest]
#[tokio::test]
async fn test_se2e_ec02_varchar_exact_limit() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_ec02_table",
		vec![
			col_pk_auto("id"),
			col_nn("bounded_text", FieldType::VarChar(255)),
		],
	)];
	apply_schema(&mut executor, schema).await;

	// Act
	let exact_string = "a".repeat(255);
	conn.execute(
		"INSERT INTO se2e_ec02_table (bounded_text) VALUES (?)",
		vec![exact_string.clone().into()],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all("SELECT bounded_text FROM se2e_ec02_table", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	let retrieved: String = rows[0].get("bounded_text").unwrap();
	assert_eq!(retrieved.len(), 255);
	assert_eq!(retrieved, exact_string);
}

/// SE2E-EC-03: INSERT 3 rows, verify sequential auto-increment IDs
#[rstest]
#[tokio::test]
async fn test_se2e_ec03_sequential_ids() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_ec03_table",
		vec![col_pk_auto("id"), col_nn("name", FieldType::VarChar(50))],
	)];
	apply_schema(&mut executor, schema).await;

	// Act
	for name in &["first", "second", "third"] {
		conn.execute(
			"INSERT INTO se2e_ec03_table (name) VALUES (?)",
			vec![(*name).into()],
		)
		.await
		.unwrap();
	}

	let rows = conn
		.fetch_all("SELECT id FROM se2e_ec03_table ORDER BY id", vec![])
		.await
		.unwrap();

	// Assert
	let ids: Vec<i64> = rows.iter().map(|r| r.get::<i64>("id").unwrap()).collect();
	assert_eq!(ids, vec![1, 2, 3]);
}

/// SE2E-EC-04: INSERT then UPDATE non-PK field, verify PK unchanged
#[rstest]
#[tokio::test]
async fn test_se2e_ec04_update_non_pk() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_ec04_table",
		vec![col_pk_auto("id"), col_nn("status", FieldType::VarChar(20))],
	)];
	apply_schema(&mut executor, schema).await;

	// Act - insert
	conn.execute(
		"INSERT INTO se2e_ec04_table (status) VALUES (?)",
		vec!["pending".into()],
	)
	.await
	.unwrap();

	// Get original id
	let rows = conn
		.fetch_all("SELECT id, status FROM se2e_ec04_table", vec![])
		.await
		.unwrap();
	let original_id: i64 = rows[0].get("id").unwrap();

	// Update status
	conn.execute(
		"UPDATE se2e_ec04_table SET status = ? WHERE id = ?",
		vec!["completed".into(), QueryValue::Int(original_id)],
	)
	.await
	.unwrap();

	let rows_after = conn
		.fetch_all(
			"SELECT id, status FROM se2e_ec04_table WHERE id = ?",
			vec![QueryValue::Int(original_id)],
		)
		.await
		.unwrap();

	// Assert
	assert_eq!(rows_after.len(), 1);
	let id_after: i64 = rows_after[0].get("id").unwrap();
	assert_eq!(id_after, original_id);
	let status: String = rows_after[0].get("status").unwrap();
	assert_eq!(status, "completed");
}

/// SE2E-EC-05: INSERT then DELETE then INSERT - new row gets next ID
#[rstest]
#[tokio::test]
async fn test_se2e_ec05_delete_and_reinsert() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_ec05_table",
		vec![col_pk_auto("id"), col_nn("value", FieldType::VarChar(50))],
	)];
	apply_schema(&mut executor, schema).await;

	// Act - insert first row
	conn.execute(
		"INSERT INTO se2e_ec05_table (value) VALUES (?)",
		vec!["first".into()],
	)
	.await
	.unwrap();

	// Delete the row
	conn.execute(
		"DELETE FROM se2e_ec05_table WHERE value = ?",
		vec!["first".into()],
	)
	.await
	.unwrap();

	// Insert new row
	conn.execute(
		"INSERT INTO se2e_ec05_table (value) VALUES (?)",
		vec!["second".into()],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all("SELECT id, value FROM se2e_ec05_table", vec![])
		.await
		.unwrap();

	// Assert - auto-increment should continue from 2, not reuse 1
	assert_eq!(rows.len(), 1);
	let id: i64 = rows[0].get("id").unwrap();
	assert_eq!(id, 2);
	let value: String = rows[0].get("value").unwrap();
	assert_eq!(value, "second");
}

/// SE2E-EC-06: INSERT 100 rows and verify COUNT
#[rstest]
#[tokio::test]
async fn test_se2e_ec06_bulk_insert_count() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_ec06_table",
		vec![col_pk_auto("id"), col_nn("value", FieldType::Integer)],
	)];
	apply_schema(&mut executor, schema).await;

	// Act
	for i in 0..100 {
		conn.execute(
			"INSERT INTO se2e_ec06_table (value) VALUES (?)",
			vec![QueryValue::Int(i)],
		)
		.await
		.unwrap();
	}

	let rows = conn
		.fetch_all("SELECT COUNT(*) as cnt FROM se2e_ec06_table", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	let count: i64 = rows[0].get("cnt").unwrap();
	assert_eq!(count, 100);
}

/// SE2E-EC-07: INSERT then UPDATE all rows and verify
#[rstest]
#[tokio::test]
async fn test_se2e_ec07_update_all_rows() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_ec07_table",
		vec![col_pk_auto("id"), col_nn("status", FieldType::VarChar(20))],
	)];
	apply_schema(&mut executor, schema).await;

	for _ in 0..5 {
		conn.execute(
			"INSERT INTO se2e_ec07_table (status) VALUES (?)",
			vec!["pending".into()],
		)
		.await
		.unwrap();
	}

	// Act - update all rows
	conn.execute(
		"UPDATE se2e_ec07_table SET status = ?",
		vec!["done".into()],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all("SELECT status FROM se2e_ec07_table", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 5);
	for row in &rows {
		let status: String = row.get("status").unwrap();
		assert_eq!(status, "done");
	}
}

/// SE2E-EC-08: Empty string INSERT into VarChar roundtrip
#[rstest]
#[tokio::test]
async fn test_se2e_ec08_empty_string_roundtrip() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_ec08_table",
		vec![col_pk_auto("id"), col_nn("name", FieldType::VarChar(100))],
	)];
	apply_schema(&mut executor, schema).await;

	// Act
	conn.execute(
		"INSERT INTO se2e_ec08_table (name) VALUES (?)",
		vec!["".into()],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all("SELECT name FROM se2e_ec08_table", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	let name: String = rows[0].get("name").unwrap();
	assert_eq!(name, "");
}

/// SE2E-EC-09: Integer MIN and MAX value roundtrip
#[rstest]
#[tokio::test]
async fn test_se2e_ec09_integer_min_max() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![create_table(
		"se2e_ec09_table",
		vec![col_pk_auto("id"), col_nn("value", FieldType::Integer)],
	)];
	apply_schema(&mut executor, schema).await;

	// Act
	conn.execute(
		"INSERT INTO se2e_ec09_table (value) VALUES (?)",
		vec![QueryValue::Int(i32::MIN as i64)],
	)
	.await
	.unwrap();
	conn.execute(
		"INSERT INTO se2e_ec09_table (value) VALUES (?)",
		vec![QueryValue::Int(i32::MAX as i64)],
	)
	.await
	.unwrap();

	let rows = conn
		.fetch_all("SELECT value FROM se2e_ec09_table ORDER BY id", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 2);
	let min_val: i64 = rows[0].get("value").unwrap();
	let max_val: i64 = rows[1].get("value").unwrap();
	assert_eq!(min_val, i32::MIN as i64);
	assert_eq!(max_val, i32::MAX as i64);
}

/// SE2E-EC-10: Multiple tables in same schema with independent INSERT/SELECT
#[rstest]
#[tokio::test]
async fn test_se2e_ec10_multiple_tables() {
	// Arrange
	let (conn, mut executor) = setup_sqlite().await;
	let schema = vec![
		create_table(
			"se2e_ec10_alpha",
			vec![col_pk_auto("id"), col_nn("name", FieldType::VarChar(50))],
		),
		create_table(
			"se2e_ec10_beta",
			vec![col_pk_auto("id"), col_nn("value", FieldType::Integer)],
		),
	];
	apply_schema(&mut executor, schema).await;

	// Act - insert into both tables independently
	conn.execute(
		"INSERT INTO se2e_ec10_alpha (name) VALUES (?)",
		vec!["alpha_row".into()],
	)
	.await
	.unwrap();
	conn.execute(
		"INSERT INTO se2e_ec10_beta (value) VALUES (?)",
		vec![999i32.into()],
	)
	.await
	.unwrap();

	let alpha_rows = conn
		.fetch_all("SELECT name FROM se2e_ec10_alpha", vec![])
		.await
		.unwrap();
	let beta_rows = conn
		.fetch_all("SELECT value FROM se2e_ec10_beta", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(alpha_rows.len(), 1);
	let alpha_name: String = alpha_rows[0].get("name").unwrap();
	assert_eq!(alpha_name, "alpha_row");
	assert_eq!(beta_rows.len(), 1);
	let beta_value: i64 = beta_rows[0].get("value").unwrap();
	assert_eq!(beta_value, 999);
}
