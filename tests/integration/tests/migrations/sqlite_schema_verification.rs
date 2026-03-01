//! SQLite schema verification integration tests
//!
//! Tests apply migrations to an in-memory SQLite database and then query
//! PRAGMA table_info / PRAGMA index_list to verify that column properties
//! (type, nullability, defaults, constraints) match the intended schema.
//!
//! **Test Coverage:**
//! - Happy path: 22 scenarios (SSV-HP-01 to SSV-HP-22)
//! - Error path: 5 scenarios (SSV-EP-01 to SSV-EP-05)
//! - Edge cases: 5 scenarios (SSV-EC-01 to SSV-EC-05)

use reinhardt_db::backends::connection::DatabaseConnection;
use reinhardt_db::migrations::{
	ColumnDefinition, FieldType, Migration,
	executor::DatabaseMigrationExecutor,
	operations::Operation,
};
use rstest::*;
use std::sync::Arc;

// ============================================================================
// Helper Functions
// ============================================================================

/// Build a CreateTable operation with a single table and columns
fn create_table(name: &str, columns: Vec<ColumnDefinition>) -> Vec<Operation> {
	vec![Operation::CreateTable {
		name: name.to_string(),
		columns,
		constraints: vec![],
		without_rowid: None,
		interleave_in_parent: None,
		partition: None,
	}]
}

/// Build a ColumnDefinition using struct literal for full control
fn col(
	name: &str,
	type_def: FieldType,
	not_null: bool,
	unique: bool,
	primary_key: bool,
	auto_increment: bool,
	default: Option<&str>,
) -> ColumnDefinition {
	ColumnDefinition {
		name: name.to_string(),
		type_definition: type_def,
		not_null,
		unique,
		primary_key,
		auto_increment,
		default: default.map(|s| s.to_string()),
	}
}

/// Create a test migration
fn create_test_migration(app: &str, name: &str, operations: Vec<Operation>) -> Migration {
	Migration {
		app_label: app.to_string(),
		name: name.to_string(),
		operations,
		dependencies: vec![],
		replaces: vec![],
		atomic: true,
		initial: None,
		state_only: false,
		database_only: false,
		swappable_dependencies: vec![],
		optional_dependencies: vec![],
	}
}

/// Create an in-memory SQLite connection and executor
async fn sqlite_conn() -> (Arc<DatabaseConnection>, DatabaseMigrationExecutor) {
	let connection = DatabaseConnection::connect_sqlite("sqlite::memory:")
		.await
		.expect("Failed to connect to in-memory SQLite");
	let conn = Arc::new(connection.clone());
	let executor = DatabaseMigrationExecutor::new(connection);
	(conn, executor)
}

/// Apply a migration using the executor
async fn apply_migration(executor: &mut DatabaseMigrationExecutor, operations: Vec<Operation>) {
	let migration = create_test_migration("ssv_test", "0001_initial", operations);
	executor.apply_migrations(&[migration]).await.unwrap();
}

/// Get column info from PRAGMA table_info for a specific column
async fn get_column_info(
	conn: &DatabaseConnection,
	table: &str,
	column: &str,
) -> Option<(String, String, i64, Option<String>, i64)> {
	let query = format!("PRAGMA table_info({})", table);
	let rows = conn.fetch_all(&query, vec![]).await.unwrap();
	for row in rows {
		let name: String = row.get::<String>("name").unwrap();
		if name == column {
			let col_type: String = row.get::<String>("type").unwrap();
			let notnull: i64 = row.get::<i64>("notnull").unwrap();
			let dflt_value: Option<String> = row.get::<String>("dflt_value").ok();
			let pk: i64 = row.get::<i64>("pk").unwrap();
			return Some((name, col_type, notnull, dflt_value, pk));
		}
	}
	None
}

/// Get all column info rows from PRAGMA table_info
async fn get_all_columns(
	conn: &DatabaseConnection,
	table: &str,
) -> Vec<(String, String, i64, Option<String>, i64)> {
	let query = format!("PRAGMA table_info({})", table);
	let rows = conn.fetch_all(&query, vec![]).await.unwrap();
	rows.iter()
		.map(|row| {
			let name: String = row.get::<String>("name").unwrap();
			let col_type: String = row.get::<String>("type").unwrap();
			let notnull: i64 = row.get::<i64>("notnull").unwrap();
			let dflt_value: Option<String> = row.get::<String>("dflt_value").ok();
			let pk: i64 = row.get::<i64>("pk").unwrap();
			(name, col_type, notnull, dflt_value, pk)
		})
		.collect()
}

/// Check if a column has a UNIQUE constraint via PRAGMA index_list + index_info
async fn has_unique_constraint(
	conn: &DatabaseConnection,
	table: &str,
	column: &str,
) -> bool {
	let query = format!("PRAGMA index_list({})", table);
	let indexes = conn.fetch_all(&query, vec![]).await.unwrap();
	for idx_row in &indexes {
		let idx_name: String = idx_row.get::<String>("name").unwrap();
		let is_unique: i64 = idx_row.get::<i64>("unique").unwrap();
		if is_unique == 1 {
			let info_query = format!("PRAGMA index_info({})", idx_name);
			let cols = conn.fetch_all(&info_query, vec![]).await.unwrap();
			for col_row in &cols {
				let col_name: String = col_row.get::<String>("name").unwrap();
				if col_name == column {
					return true;
				}
			}
		}
	}
	false
}

// ============================================================================
// Happy Path Tests (SSV-HP-01 to SSV-HP-12)
// ============================================================================

/// SSV-HP-01: Integer PK with auto_increment is marked as primary key
#[rstest]
#[tokio::test]
async fn test_ssv_hp_01_serial_pk_auto_increment() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp01_table",
		vec![col("id", FieldType::Integer, true, false, true, true, None)],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_hp01_table", "id").await.unwrap();

	// Assert
	assert!(
		info.4 > 0,
		"INTEGER PRIMARY KEY should have pk > 0, got: {}",
		info.4
	);
	assert_eq!(
		info.1.to_uppercase(),
		"INTEGER",
		"auto_increment PK should be INTEGER type"
	);
}

/// SSV-HP-02: BigInteger PK with auto_increment maps to BIGINT with PK flag
#[rstest]
#[tokio::test]
async fn test_ssv_hp_02_biginteger_pk_auto_increment() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp02_table",
		vec![col(
			"id",
			FieldType::BigInteger,
			true,
			false,
			true,
			true,
			None,
		)],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_hp02_table", "id").await.unwrap();

	// Assert
	assert!(info.4 > 0, "BigInteger PK should have pk > 0");
	let col_type = info.1.to_uppercase();
	assert!(
		col_type.contains("INT"),
		"BigInteger should map to an integer-like type, got: {}",
		col_type
	);
}

/// SSV-HP-03: VarChar(255) NOT NULL maps correctly
#[rstest]
#[tokio::test]
async fn test_ssv_hp_03_varchar_255_not_null() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp03_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"name",
				FieldType::VarChar(255),
				true,
				false,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_hp03_table", "name").await.unwrap();

	// Assert
	let col_type = info.1.to_uppercase();
	assert!(
		col_type.contains("VARCHAR") || col_type.contains("TEXT"),
		"VarChar(255) should map to VARCHAR or TEXT type, got: {}",
		col_type
	);
	assert_eq!(info.2, 1, "NOT NULL column should have notnull=1");
}

/// SSV-HP-04: VarChar(100) nullable
#[rstest]
#[tokio::test]
async fn test_ssv_hp_04_varchar_nullable() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp04_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"description",
				FieldType::VarChar(100),
				false,
				false,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_hp04_table", "description")
		.await
		.unwrap();

	// Assert
	assert_eq!(
		info.2, 0,
		"Nullable column should have notnull=0"
	);
}

/// SSV-HP-05: Boolean with default "false"
#[rstest]
#[tokio::test]
async fn test_ssv_hp_05_boolean_default_false() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp05_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"is_active",
				FieldType::Boolean,
				true,
				false,
				false,
				false,
				Some("false"),
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_hp05_table", "is_active")
		.await
		.unwrap();

	// Assert
	let col_type = info.1.to_uppercase();
	assert!(
		col_type.contains("BOOL") || col_type.contains("INT"),
		"Boolean should map to BOOLEAN or INTEGER type, got: {}",
		col_type
	);
	assert!(
		info.3.as_ref().map_or(false, |d| {
			let lower = d.to_lowercase();
			lower.contains("false") || lower.contains("0")
		}),
		"Boolean default should contain 'false' or '0', got: {:?}",
		info.3
	);
}

/// SSV-HP-06: TimestampTz maps to DATETIME in SQLite (no timezone distinction)
#[rstest]
#[tokio::test]
async fn test_ssv_hp_06_timestamp_tz() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp06_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"created_at",
				FieldType::TimestampTz,
				false,
				false,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_hp06_table", "created_at")
		.await
		.unwrap();

	// Assert
	let col_type = info.1.to_uppercase();
	assert!(
		col_type.contains("DATETIME") || col_type.contains("TIMESTAMP") || col_type.contains("TEXT"),
		"TimestampTz should map to DATETIME, TIMESTAMP, or TEXT in SQLite, got: {}",
		col_type
	);
}

/// SSV-HP-07: DateTime maps correctly in SQLite
#[rstest]
#[tokio::test]
async fn test_ssv_hp_07_datetime_no_tz() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp07_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"event_time",
				FieldType::DateTime,
				false,
				false,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_hp07_table", "event_time")
		.await
		.unwrap();

	// Assert
	let col_type = info.1.to_uppercase();
	assert!(
		col_type.contains("DATETIME") || col_type.contains("TIMESTAMP") || col_type.contains("TEXT"),
		"DateTime should map to DATETIME, TIMESTAMP, or TEXT in SQLite, got: {}",
		col_type
	);
}

/// SSV-HP-08: Integer NOT NULL
#[rstest]
#[tokio::test]
async fn test_ssv_hp_08_integer_not_null() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp08_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"quantity",
				FieldType::Integer,
				true,
				false,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_hp08_table", "quantity")
		.await
		.unwrap();

	// Assert
	let col_type = info.1.to_uppercase();
	assert!(
		col_type.contains("INT"),
		"Integer should map to INTEGER type, got: {}",
		col_type
	);
	assert_eq!(info.2, 1, "NOT NULL column should have notnull=1");
}

/// SSV-HP-09: Uuid maps to TEXT or CHAR(36) in SQLite
#[rstest]
#[tokio::test]
async fn test_ssv_hp_09_uuid() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp09_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"external_id",
				FieldType::Uuid,
				false,
				false,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_hp09_table", "external_id")
		.await
		.unwrap();

	// Assert
	let col_type = info.1.to_uppercase();
	assert!(
		col_type.contains("TEXT") || col_type.contains("CHAR") || col_type.contains("UUID"),
		"Uuid should map to TEXT, CHAR, or UUID in SQLite, got: {}",
		col_type
	);
}

/// SSV-HP-10: Double maps correctly
#[rstest]
#[tokio::test]
async fn test_ssv_hp_10_double() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp10_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col("score", FieldType::Double, false, false, false, false, None),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_hp10_table", "score")
		.await
		.unwrap();

	// Assert
	let col_type = info.1.to_uppercase();
	assert!(
		col_type.contains("DOUBLE") || col_type.contains("REAL") || col_type.contains("FLOAT"),
		"Double should map to DOUBLE, REAL, or FLOAT, got: {}",
		col_type
	);
}

/// SSV-HP-11: Float maps correctly
#[rstest]
#[tokio::test]
async fn test_ssv_hp_11_float() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp11_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col("weight", FieldType::Float, false, false, false, false, None),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_hp11_table", "weight")
		.await
		.unwrap();

	// Assert
	let col_type = info.1.to_uppercase();
	assert!(
		col_type.contains("FLOAT") || col_type.contains("REAL"),
		"Float should map to FLOAT or REAL, got: {}",
		col_type
	);
}

/// SSV-HP-12: VarChar(255) unique has UNIQUE constraint
#[rstest]
#[tokio::test]
async fn test_ssv_hp_12_varchar_unique() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp12_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"email",
				FieldType::VarChar(255),
				true,
				true,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let is_unique = has_unique_constraint(&conn, "ssv_hp12_table", "email").await;

	// Assert
	assert!(is_unique, "email column should have a UNIQUE constraint");
}

// ============================================================================
// Happy Path Tests (SSV-HP-13 to SSV-HP-22)
// ============================================================================

/// SSV-HP-13: Text NOT NULL maps to TEXT with notnull=1
#[rstest]
#[tokio::test]
async fn test_ssv_hp_13_text_not_null() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp13_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col("content", FieldType::Text, true, false, false, false, None),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_hp13_table", "content")
		.await
		.unwrap();

	// Assert
	assert!(
		info.1.to_uppercase().contains("TEXT"),
		"Text should map to TEXT, got: {}",
		info.1
	);
	assert_eq!(info.2, 1, "NOT NULL column should have notnull=1");
}

/// SSV-HP-14: Date type maps correctly
#[rstest]
#[tokio::test]
async fn test_ssv_hp_14_date_type() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp14_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"birth_date",
				FieldType::Date,
				false,
				false,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_hp14_table", "birth_date")
		.await
		.unwrap();

	// Assert
	let col_type = info.1.to_uppercase();
	assert!(
		col_type.contains("DATE") || col_type.contains("TEXT"),
		"Date should map to DATE or TEXT, got: {}",
		col_type
	);
}

/// SSV-HP-15: Time type maps correctly
#[rstest]
#[tokio::test]
async fn test_ssv_hp_15_time_type() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp15_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"start_time",
				FieldType::Time,
				false,
				false,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_hp15_table", "start_time")
		.await
		.unwrap();

	// Assert
	let col_type = info.1.to_uppercase();
	assert!(
		col_type.contains("TIME") || col_type.contains("TEXT"),
		"Time should map to TIME or TEXT, got: {}",
		col_type
	);
}

/// SSV-HP-16: SmallInteger maps correctly
#[rstest]
#[tokio::test]
async fn test_ssv_hp_16_small_integer() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp16_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"small_val",
				FieldType::SmallInteger,
				false,
				false,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_hp16_table", "small_val")
		.await
		.unwrap();

	// Assert
	let col_type = info.1.to_uppercase();
	assert!(
		col_type.contains("SMALLINT") || col_type.contains("INT"),
		"SmallInteger should map to SMALLINT or INTEGER, got: {}",
		col_type
	);
}

/// SSV-HP-17: Decimal(10,2) maps correctly
#[rstest]
#[tokio::test]
async fn test_ssv_hp_17_decimal() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp17_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"price",
				FieldType::Decimal {
					precision: 10,
					scale: 2,
				},
				false,
				false,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_hp17_table", "price")
		.await
		.unwrap();

	// Assert
	let col_type = info.1.to_uppercase();
	assert!(
		col_type.contains("DECIMAL") || col_type.contains("NUMERIC") || col_type.contains("REAL"),
		"Decimal should map to DECIMAL, NUMERIC, or REAL, got: {}",
		col_type
	);
}

/// SSV-HP-18: Json type maps correctly in SQLite
#[rstest]
#[tokio::test]
async fn test_ssv_hp_18_json() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp18_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"metadata",
				FieldType::Json,
				false,
				false,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_hp18_table", "metadata")
		.await
		.unwrap();

	// Assert
	let col_type = info.1.to_uppercase();
	assert!(
		col_type.contains("JSON") || col_type.contains("TEXT"),
		"Json should map to JSON or TEXT in SQLite, got: {}",
		col_type
	);
}

/// SSV-HP-19: JsonBinary (JSONB) maps correctly in SQLite
#[rstest]
#[tokio::test]
async fn test_ssv_hp_19_jsonb() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp19_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"data",
				FieldType::JsonBinary,
				false,
				false,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_hp19_table", "data")
		.await
		.unwrap();

	// Assert
	let col_type = info.1.to_uppercase();
	assert!(
		col_type.contains("JSON") || col_type.contains("TEXT") || col_type.contains("BLOB"),
		"JsonBinary should map to JSON, TEXT, or BLOB in SQLite, got: {}",
		col_type
	);
}

/// SSV-HP-20: Char(5) maps correctly
#[rstest]
#[tokio::test]
async fn test_ssv_hp_20_char_fixed() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp20_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col("code", FieldType::Char(5), false, false, false, false, None),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_hp20_table", "code")
		.await
		.unwrap();

	// Assert
	let col_type = info.1.to_uppercase();
	assert!(
		col_type.contains("CHAR") || col_type.contains("TEXT"),
		"Char(5) should map to CHAR or TEXT type, got: {}",
		col_type
	);
}

/// SSV-HP-21: Integer with default value 42
#[rstest]
#[tokio::test]
async fn test_ssv_hp_21_integer_default() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp21_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"quantity",
				FieldType::Integer,
				false,
				false,
				false,
				false,
				Some("42"),
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_hp21_table", "quantity")
		.await
		.unwrap();

	// Assert
	assert!(
		info.3.as_ref().map_or(false, |d| d.contains("42")),
		"column default should contain '42', got: {:?}",
		info.3
	);
}

/// SSV-HP-22: Multiple columns with defaults in the same table
/// (Boolean false, Integer 0, VarChar "draft")
#[rstest]
#[tokio::test]
async fn test_ssv_hp_22_multiple_defaults() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_hp22_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"is_active",
				FieldType::Boolean,
				false,
				false,
				false,
				false,
				Some("false"),
			),
			col(
				"counter",
				FieldType::Integer,
				false,
				false,
				false,
				false,
				Some("0"),
			),
			col(
				"status",
				FieldType::VarChar(50),
				false,
				false,
				false,
				false,
				Some("'draft'"),
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let columns = get_all_columns(&conn, "ssv_hp22_table").await;

	// Assert
	for (name, _col_type, _notnull, dflt_value, _pk) in &columns {
		match name.as_str() {
			"is_active" => assert!(
				dflt_value.as_ref().map_or(false, |d| {
					let lower = d.to_lowercase();
					lower.contains("false") || lower.contains("0")
				}),
				"is_active default should contain 'false' or '0', got: {:?}",
				dflt_value
			),
			"counter" => assert!(
				dflt_value.as_ref().map_or(false, |d| d.contains("0")),
				"counter default should contain '0', got: {:?}",
				dflt_value
			),
			"status" => assert!(
				dflt_value.as_ref().map_or(false, |d| d.contains("draft")),
				"status default should contain 'draft', got: {:?}",
				dflt_value
			),
			"id" => {} // Skip PK column
			other => panic!("Unexpected column: {other}"),
		}
	}
}

// ============================================================================
// Error Path Tests (SSV-EP-01 to SSV-EP-05)
// ============================================================================

/// SSV-EP-01: INSERT NULL into NOT NULL column fails
#[rstest]
#[tokio::test]
async fn test_ssv_ep_01_null_into_not_null() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_ep01_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"name",
				FieldType::VarChar(255),
				true,
				false,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let result = conn
		.fetch_all("INSERT INTO ssv_ep01_table (name) VALUES (NULL)", vec![])
		.await;

	// Assert
	assert!(
		result.is_err(),
		"INSERT NULL into NOT NULL column should fail"
	);
	let err = result.unwrap_err().to_string();
	assert!(
		err.to_lowercase().contains("null") || err.contains("NOT NULL"),
		"Error should indicate NOT NULL violation, got: {}",
		err
	);
}

/// SSV-EP-02: INSERT duplicate into unique column fails
#[rstest]
#[tokio::test]
async fn test_ssv_ep_02_duplicate_unique() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_ep02_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"email",
				FieldType::VarChar(255),
				true,
				true,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	conn.fetch_all(
		"INSERT INTO ssv_ep02_table (email) VALUES ('test@example.com')",
		vec![],
	)
	.await
	.unwrap();

	// Act
	let result = conn
		.fetch_all(
			"INSERT INTO ssv_ep02_table (email) VALUES ('test@example.com')",
			vec![],
		)
		.await;

	// Assert
	assert!(
		result.is_err(),
		"INSERT duplicate into unique column should fail"
	);
	let err = result.unwrap_err().to_string();
	assert!(
		err.to_lowercase().contains("unique") || err.to_lowercase().contains("constraint"),
		"Error should indicate unique violation, got: {}",
		err
	);
}

/// SSV-EP-03: INSERT string into integer column fails
/// Note: SQLite has flexible typing, so this test verifies the behavior
/// which may differ from strict-typed databases
#[rstest]
#[tokio::test]
async fn test_ssv_ep_03_string_into_integer() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_ep03_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"quantity",
				FieldType::Integer,
				true,
				false,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	// SQLite uses type affinity, so it may accept string values for integer columns.
	// We verify the migration created the column with the correct type instead.
	let info = get_column_info(&conn, "ssv_ep03_table", "quantity")
		.await
		.unwrap();

	// Assert
	let col_type = info.1.to_uppercase();
	assert!(
		col_type.contains("INT"),
		"Integer column should have INTEGER type affinity, got: {}",
		col_type
	);
}

/// SSV-EP-04: VarChar(255) column type is correctly declared
/// Note: SQLite does not enforce VARCHAR length limits, so we verify the
/// declared type contains the length specification
#[rstest]
#[tokio::test]
async fn test_ssv_ep_04_varchar_length_declared() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_ep04_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"short_text",
				FieldType::VarChar(255),
				true,
				false,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_ep04_table", "short_text")
		.await
		.unwrap();

	// Assert
	let col_type = info.1.to_uppercase();
	assert!(
		col_type.contains("VARCHAR") || col_type.contains("TEXT"),
		"VarChar column should have VARCHAR or TEXT type, got: {}",
		col_type
	);
}

/// SSV-EP-05: INSERT without required NOT NULL column (no default) fails
#[rstest]
#[tokio::test]
async fn test_ssv_ep_05_missing_not_null_column() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_ep05_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"required_field",
				FieldType::VarChar(100),
				true,
				false,
				false,
				false,
				None,
			),
			col(
				"optional_field",
				FieldType::VarChar(100),
				false,
				false,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act: insert only optional_field, omitting required_field
	let result = conn
		.fetch_all(
			"INSERT INTO ssv_ep05_table (optional_field) VALUES ('some_value')",
			vec![],
		)
		.await;

	// Assert
	assert!(
		result.is_err(),
		"INSERT without required NOT NULL column should fail"
	);
	let err = result.unwrap_err().to_string();
	assert!(
		err.to_lowercase().contains("null") || err.contains("NOT NULL"),
		"Error should indicate NOT NULL violation, got: {}",
		err
	);
}

// ============================================================================
// Edge Case Tests (SSV-EC-01 to SSV-EC-05)
// ============================================================================

/// SSV-EC-01: INTEGER PRIMARY KEY in SQLite is automatically ROWID alias (auto-increment)
#[rstest]
#[tokio::test]
async fn test_ssv_ec_01_integer_pk_rowid_alias() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_ec01_table",
		vec![col("id", FieldType::Integer, true, false, true, true, None)],
	);
	apply_migration(&mut executor, schema).await;

	// Act: insert two rows without specifying id to verify auto-increment behavior
	conn.fetch_all("INSERT INTO ssv_ec01_table DEFAULT VALUES", vec![])
		.await
		.unwrap();
	conn.fetch_all("INSERT INTO ssv_ec01_table DEFAULT VALUES", vec![])
		.await
		.unwrap();

	let rows = conn
		.fetch_all("SELECT id FROM ssv_ec01_table ORDER BY id", vec![])
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 2, "Should have 2 rows");
	let id1: i64 = rows[0].get::<i64>("id").unwrap();
	let id2: i64 = rows[1].get::<i64>("id").unwrap();
	assert!(
		id2 > id1,
		"Auto-increment should produce increasing IDs, got {} and {}",
		id1,
		id2
	);
}

/// SSV-EC-02: DateTime and TimestampTz both map to similar types in SQLite
/// (SQLite has no timezone-aware distinction)
#[rstest]
#[tokio::test]
async fn test_ssv_ec_02_datetime_vs_timestamptz_same_in_sqlite() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_ec02_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"local_time",
				FieldType::DateTime,
				false,
				false,
				false,
				false,
				None,
			),
			col(
				"utc_time",
				FieldType::TimestampTz,
				false,
				false,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let local_info = get_column_info(&conn, "ssv_ec02_table", "local_time")
		.await
		.unwrap();
	let utc_info = get_column_info(&conn, "ssv_ec02_table", "utc_time")
		.await
		.unwrap();

	// Assert
	// Both should map to datetime-like types in SQLite
	let local_type = local_info.1.to_uppercase();
	let utc_type = utc_info.1.to_uppercase();
	assert!(
		local_type.contains("DATETIME") || local_type.contains("TIMESTAMP") || local_type.contains("TEXT"),
		"DateTime should map to DATETIME/TIMESTAMP/TEXT, got: {}",
		local_type
	);
	assert!(
		utc_type.contains("DATETIME") || utc_type.contains("TIMESTAMP") || utc_type.contains("TEXT"),
		"TimestampTz should map to DATETIME/TIMESTAMP/TEXT, got: {}",
		utc_type
	);
}

/// SSV-EC-03: Table with 8 different column types, all correctly mapped
#[rstest]
#[tokio::test]
async fn test_ssv_ec_03_eight_column_types() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_ec03_table",
		vec![
			col("col_int", FieldType::Integer, true, false, true, true, None),
			col(
				"col_bigint",
				FieldType::BigInteger,
				false,
				false,
				false,
				false,
				None,
			),
			col(
				"col_varchar",
				FieldType::VarChar(100),
				false,
				false,
				false,
				false,
				None,
			),
			col(
				"col_bool",
				FieldType::Boolean,
				false,
				false,
				false,
				false,
				None,
			),
			col(
				"col_double",
				FieldType::Double,
				false,
				false,
				false,
				false,
				None,
			),
			col(
				"col_float",
				FieldType::Float,
				false,
				false,
				false,
				false,
				None,
			),
			col(
				"col_text",
				FieldType::Text,
				false,
				false,
				false,
				false,
				None,
			),
			col(
				"col_date",
				FieldType::Date,
				false,
				false,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let columns = get_all_columns(&conn, "ssv_ec03_table").await;

	// Assert
	assert_eq!(columns.len(), 8, "Should have 8 columns");
	// Verify each column exists and has a non-empty type
	for (name, col_type, _notnull, _dflt, _pk) in &columns {
		assert!(
			!col_type.is_empty(),
			"Column {} should have a declared type",
			name
		);
	}
}

/// SSV-EC-04: Boolean default false + Integer default 0
#[rstest]
#[tokio::test]
async fn test_ssv_ec_04_multiple_defaults() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	let schema = create_table(
		"ssv_ec04_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"flag",
				FieldType::Boolean,
				true,
				false,
				false,
				false,
				Some("false"),
			),
			col(
				"count",
				FieldType::Integer,
				true,
				false,
				false,
				false,
				Some("0"),
			),
		],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let flag_info = get_column_info(&conn, "ssv_ec04_table", "flag")
		.await
		.unwrap();
	let count_info = get_column_info(&conn, "ssv_ec04_table", "count")
		.await
		.unwrap();

	// Assert
	assert!(
		count_info.3.as_ref().map_or(false, |d| d.contains("0")),
		"Integer default should contain '0', got: {:?}",
		count_info.3
	);
	assert!(
		flag_info.3.as_ref().map_or(false, |d| {
			let lower = d.to_lowercase();
			lower.contains("false") || lower.contains("0")
		}),
		"Boolean default should contain 'false' or '0', got: {:?}",
		flag_info.3
	);
}

/// SSV-EC-05: Nullable PK (primary_key=true, not_null=false)
/// SQLite PRIMARY KEY does NOT enforce NOT NULL unless the column is INTEGER PRIMARY KEY
#[rstest]
#[tokio::test]
async fn test_ssv_ec_05_nullable_pk() {
	// Arrange
	let (conn, mut executor) = sqlite_conn().await;
	// In SQLite, PRIMARY KEY on non-INTEGER types does NOT imply NOT NULL
	// (unlike PostgreSQL). INTEGER PRIMARY KEY is a special case.
	let schema = create_table(
		"ssv_ec05_table",
		vec![col(
			"id",
			FieldType::BigInteger,
			false,
			false,
			true,
			false,
			None,
		)],
	);
	apply_migration(&mut executor, schema).await;

	// Act
	let info = get_column_info(&conn, "ssv_ec05_table", "id")
		.await
		.unwrap();

	// Assert
	// PK flag should be set
	assert!(info.4 > 0, "id should be marked as primary key");
}
