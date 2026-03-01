//! End-to-end cycle integration tests for migration system
//!
//! Tests the full cycle: define schema -> apply migration -> INSERT/SELECT data -> verify round-trip.
//! Validates that the migration system produces real, usable database tables.

use reinhardt_db::migrations::{ColumnDefinition, FieldType, Operation};
use reinhardt_test::fixtures::PostgresTableCreator;
use reinhardt_test::fixtures::postgres_table_creator;
use rstest::*;
use serial_test::serial;
use sqlx::Row;

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

// ============================================================================
// Happy Path Tests (E2E-HP-01 to E2E-HP-10)
// ============================================================================

/// E2E-HP-01: Basic CRUD - create table, insert, select, verify values match
#[rstest]
#[serial(e2e_cycle)]
#[tokio::test]
async fn test_e2e_hp01_basic_crud(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = vec![create_table(
		"e2e_hp01_table",
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
	creator.apply(schema).await.unwrap();
	let pool = creator.pool();

	// Act
	sqlx::query("INSERT INTO e2e_hp01_table (name, active) VALUES ($1, $2)")
		.bind("test")
		.bind(true)
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row = sqlx::query("SELECT id, name, active FROM e2e_hp01_table WHERE name = $1")
		.bind("test")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let id: i32 = row.get("id");
	let name: String = row.get("name");
	let active: bool = row.get("active");
	assert!(id > 0);
	assert_eq!(name, "test");
	assert_eq!(active, true);
}

/// E2E-HP-02: Auto-increment PK - insert without specifying id, verify id > 0
#[rstest]
#[serial(e2e_cycle)]
#[tokio::test]
async fn test_e2e_hp02_auto_increment_pk(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = vec![create_table(
		"e2e_hp02_table",
		vec![col_pk_auto("id"), col_nn("label", FieldType::VarChar(50))],
	)];
	creator.apply(schema).await.unwrap();
	let pool = creator.pool();

	// Act
	sqlx::query("INSERT INTO e2e_hp02_table (label) VALUES ($1)")
		.bind("auto_test")
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row = sqlx::query("SELECT id FROM e2e_hp02_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let id: i32 = row.get("id");
	assert!(id > 0);
}

/// E2E-HP-03: auto_now_add timestamp - insert with default CURRENT_TIMESTAMP, verify non-null
#[rstest]
#[serial(e2e_cycle)]
#[tokio::test]
async fn test_e2e_hp03_auto_now_add_timestamp(
	#[future] postgres_table_creator: PostgresTableCreator,
) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = vec![create_table(
		"e2e_hp03_table",
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
	creator.apply(schema).await.unwrap();
	let pool = creator.pool();

	// Act - insert without specifying created_at
	sqlx::query("INSERT INTO e2e_hp03_table DEFAULT VALUES")
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row = sqlx::query("SELECT id, created_at FROM e2e_hp03_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
	assert!(created_at.timestamp() > 0);
}

/// E2E-HP-04: Multiple rows - insert 3 rows, verify count and values
#[rstest]
#[serial(e2e_cycle)]
#[tokio::test]
async fn test_e2e_hp04_multiple_rows(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = vec![create_table(
		"e2e_hp04_table",
		vec![col_pk_auto("id"), col_nn("value", FieldType::VarChar(50))],
	)];
	creator.apply(schema).await.unwrap();
	let pool = creator.pool();

	// Act
	for val in &["alpha", "beta", "gamma"] {
		sqlx::query("INSERT INTO e2e_hp04_table (value) VALUES ($1)")
			.bind(*val)
			.execute(pool.as_ref())
			.await
			.unwrap();
	}

	let rows = sqlx::query("SELECT id, value FROM e2e_hp04_table ORDER BY id")
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 3);
	assert_eq!(rows[0].get::<String, _>("value"), "alpha");
	assert_eq!(rows[1].get::<String, _>("value"), "beta");
	assert_eq!(rows[2].get::<String, _>("value"), "gamma");
}

/// E2E-HP-05: Boolean default - insert without bool field, verify default applied
#[rstest]
#[serial(e2e_cycle)]
#[tokio::test]
async fn test_e2e_hp05_boolean_default(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = vec![create_table(
		"e2e_hp05_table",
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
	creator.apply(schema).await.unwrap();
	let pool = creator.pool();

	// Act - insert without specifying is_active
	sqlx::query("INSERT INTO e2e_hp05_table DEFAULT VALUES")
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row = sqlx::query("SELECT is_active FROM e2e_hp05_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let is_active: bool = row.get("is_active");
	assert_eq!(is_active, false);
}

/// E2E-HP-06: Integer default - insert without int field, verify default applied
#[rstest]
#[serial(e2e_cycle)]
#[tokio::test]
async fn test_e2e_hp06_integer_default(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = vec![create_table(
		"e2e_hp06_table",
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
	creator.apply(schema).await.unwrap();
	let pool = creator.pool();

	// Act
	sqlx::query("INSERT INTO e2e_hp06_table DEFAULT VALUES")
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row = sqlx::query("SELECT score FROM e2e_hp06_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let score: i32 = row.get("score");
	assert_eq!(score, 0);
}

/// E2E-HP-07: Nullable field NULL - insert NULL, verify field is NULL
#[rstest]
#[serial(e2e_cycle)]
#[tokio::test]
async fn test_e2e_hp07_nullable_field_null(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = vec![create_table(
		"e2e_hp07_table",
		vec![col_pk_auto("id"), col("description", FieldType::Text)],
	)];
	creator.apply(schema).await.unwrap();
	let pool = creator.pool();

	// Act
	sqlx::query("INSERT INTO e2e_hp07_table (description) VALUES ($1)")
		.bind(None::<String>)
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row = sqlx::query("SELECT description FROM e2e_hp07_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let desc: Option<String> = row.get("description");
	assert_eq!(desc, None);
}

/// E2E-HP-08: Nullable field with value - insert actual value, verify preserved
#[rstest]
#[serial(e2e_cycle)]
#[tokio::test]
async fn test_e2e_hp08_nullable_field_with_value(
	#[future] postgres_table_creator: PostgresTableCreator,
) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = vec![create_table(
		"e2e_hp08_table",
		vec![col_pk_auto("id"), col("description", FieldType::Text)],
	)];
	creator.apply(schema).await.unwrap();
	let pool = creator.pool();

	// Act
	sqlx::query("INSERT INTO e2e_hp08_table (description) VALUES ($1)")
		.bind("some description")
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row = sqlx::query("SELECT description FROM e2e_hp08_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let desc: Option<String> = row.get("description");
	assert_eq!(desc, Some("some description".to_string()));
}

/// E2E-HP-09: UUID PK - insert with UUID primary key, verify preserved
#[rstest]
#[serial(e2e_cycle)]
#[tokio::test]
async fn test_e2e_hp09_uuid_pk(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = vec![create_table(
		"e2e_hp09_table",
		vec![
			ColumnDefinition {
				name: "id".to_string(),
				type_definition: FieldType::Uuid,
				not_null: true,
				unique: false,
				primary_key: true,
				auto_increment: false,
				default: None,
			},
			col_nn("label", FieldType::VarChar(50)),
		],
	)];
	creator.apply(schema).await.unwrap();
	let pool = creator.pool();

	// Act
	let test_uuid = uuid::Uuid::new_v4();
	sqlx::query("INSERT INTO e2e_hp09_table (id, label) VALUES ($1, $2)")
		.bind(test_uuid)
		.bind("uuid_test")
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row = sqlx::query("SELECT id, label FROM e2e_hp09_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let retrieved_uuid: uuid::Uuid = row.get("id");
	assert_eq!(retrieved_uuid, test_uuid);
	assert_eq!(row.get::<String, _>("label"), "uuid_test");
}

/// E2E-HP-10: Multi-type table - table with many types, verify all round-trip
#[rstest]
#[serial(e2e_cycle)]
#[tokio::test]
async fn test_e2e_hp10_multi_type_table(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = vec![create_table(
		"e2e_hp10_table",
		vec![
			col_pk_auto("id"),
			col_nn("int_val", FieldType::Integer),
			col_nn("str_val", FieldType::VarChar(100)),
			col_nn("bool_val", FieldType::Boolean),
			col_nn("float_val", FieldType::Float),
			col_nn("double_val", FieldType::Double),
			col_nn("date_val", FieldType::Date),
			col_nn("ts_val", FieldType::TimestampTz),
		],
	)];
	creator.apply(schema).await.unwrap();
	let pool = creator.pool();

	// Act
	let test_date = chrono::NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
	let test_ts = chrono::Utc::now();
	sqlx::query(
		"INSERT INTO e2e_hp10_table (int_val, str_val, bool_val, float_val, double_val, date_val, ts_val) \
		 VALUES ($1, $2, $3, $4, $5, $6, $7)"
	)
		.bind(42i32)
		.bind("hello")
		.bind(true)
		.bind(3.14f32)
		.bind(2.718281828f64)
		.bind(test_date)
		.bind(test_ts)
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row = sqlx::query("SELECT * FROM e2e_hp10_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(row.get::<i32, _>("int_val"), 42);
	assert_eq!(row.get::<String, _>("str_val"), "hello");
	assert_eq!(row.get::<bool, _>("bool_val"), true);
	let float_val: f32 = row.get("float_val");
	assert!((float_val - 3.14).abs() < 0.01);
	let double_val: f64 = row.get("double_val");
	assert!((double_val - 2.718281828).abs() < 0.000001);
	assert_eq!(row.get::<chrono::NaiveDate, _>("date_val"), test_date);
	let ts_retrieved: chrono::DateTime<chrono::Utc> = row.get("ts_val");
	assert!((ts_retrieved - test_ts).num_seconds().abs() < 2);
}

// ============================================================================
// Error Path Tests (E2E-EP-01 to E2E-EP-05)
// ============================================================================

/// E2E-EP-01: INSERT NULL for NOT NULL field - expect DB error
#[rstest]
#[serial(e2e_cycle)]
#[tokio::test]
async fn test_e2e_ep01_null_for_not_null(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = vec![create_table(
		"e2e_ep01_table",
		vec![
			col_pk_auto("id"),
			col_nn("required_field", FieldType::VarChar(100)),
		],
	)];
	creator.apply(schema).await.unwrap();
	let pool = creator.pool();

	// Act
	let result = sqlx::query("INSERT INTO e2e_ep01_table (required_field) VALUES ($1)")
		.bind(None::<String>)
		.execute(pool.as_ref())
		.await;

	// Assert
	assert!(result.is_err());
}

/// E2E-EP-02: INSERT duplicate unique value - expect unique constraint error
#[rstest]
#[serial(e2e_cycle)]
#[tokio::test]
async fn test_e2e_ep02_duplicate_unique(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = vec![create_table(
		"e2e_ep02_table",
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
	creator.apply(schema).await.unwrap();
	let pool = creator.pool();

	// Act - first insert succeeds
	sqlx::query("INSERT INTO e2e_ep02_table (email) VALUES ($1)")
		.bind("user@example.com")
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Second insert with same value should fail
	let result = sqlx::query("INSERT INTO e2e_ep02_table (email) VALUES ($1)")
		.bind("user@example.com")
		.execute(pool.as_ref())
		.await;

	// Assert
	assert!(result.is_err());
}

/// E2E-EP-03: INSERT 256 chars into VarChar(255) - expect DB error
#[rstest]
#[serial(e2e_cycle)]
#[tokio::test]
async fn test_e2e_ep03_varchar_overflow(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = vec![create_table(
		"e2e_ep03_table",
		vec![
			col_pk_auto("id"),
			col_nn("short_text", FieldType::VarChar(255)),
		],
	)];
	creator.apply(schema).await.unwrap();
	let pool = creator.pool();

	// Act - insert string exceeding varchar length
	let long_string = "x".repeat(256);
	let result = sqlx::query("INSERT INTO e2e_ep03_table (short_text) VALUES ($1)")
		.bind(&long_string)
		.execute(pool.as_ref())
		.await;

	// Assert
	assert!(result.is_err());
}

/// E2E-EP-04: INSERT string into integer column - expect DB type error
#[rstest]
#[serial(e2e_cycle)]
#[tokio::test]
async fn test_e2e_ep04_type_mismatch(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = vec![create_table(
		"e2e_ep04_table",
		vec![col_pk_auto("id"), col_nn("amount", FieldType::Integer)],
	)];
	creator.apply(schema).await.unwrap();
	let pool = creator.pool();

	// Act - use raw SQL to bypass type checking in sqlx bindings
	let result = sqlx::query("INSERT INTO e2e_ep04_table (amount) VALUES ('not_a_number')")
		.execute(pool.as_ref())
		.await;

	// Assert
	assert!(result.is_err());
}

/// E2E-EP-05: SELECT from non-existent table - expect DB error
#[rstest]
#[serial(e2e_cycle)]
#[tokio::test]
async fn test_e2e_ep05_select_nonexistent_table(
	#[future] postgres_table_creator: PostgresTableCreator,
) {
	// Arrange
	let creator = postgres_table_creator.await;
	let pool = creator.pool();

	// Act
	let result = sqlx::query("SELECT * FROM e2e_ep05_nonexistent_table")
		.fetch_all(pool.as_ref())
		.await;

	// Assert
	assert!(result.is_err());
}

// ============================================================================
// Edge Case Tests (E2E-EC-01 to E2E-EC-05)
// ============================================================================

/// E2E-EC-01: SELECT from empty table - expect 0 rows
#[rstest]
#[serial(e2e_cycle)]
#[tokio::test]
async fn test_e2e_ec01_select_empty_table(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = vec![create_table(
		"e2e_ec01_table",
		vec![col_pk_auto("id"), col("name", FieldType::Text)],
	)];
	creator.apply(schema).await.unwrap();
	let pool = creator.pool();

	// Act
	let rows = sqlx::query("SELECT * FROM e2e_ec01_table")
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 0);
}

/// E2E-EC-02: INSERT exactly 255 chars into VarChar(255) - expect success
#[rstest]
#[serial(e2e_cycle)]
#[tokio::test]
async fn test_e2e_ec02_varchar_exact_limit(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = vec![create_table(
		"e2e_ec02_table",
		vec![
			col_pk_auto("id"),
			col_nn("bounded_text", FieldType::VarChar(255)),
		],
	)];
	creator.apply(schema).await.unwrap();
	let pool = creator.pool();

	// Act
	let exact_string = "a".repeat(255);
	sqlx::query("INSERT INTO e2e_ec02_table (bounded_text) VALUES ($1)")
		.bind(&exact_string)
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row = sqlx::query("SELECT bounded_text FROM e2e_ec02_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let retrieved: String = row.get("bounded_text");
	assert_eq!(retrieved.len(), 255);
	assert_eq!(retrieved, exact_string);
}

/// E2E-EC-03: INSERT 3 rows, verify sequential auto-increment IDs
#[rstest]
#[serial(e2e_cycle)]
#[tokio::test]
async fn test_e2e_ec03_sequential_ids(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = vec![create_table(
		"e2e_ec03_table",
		vec![col_pk_auto("id"), col_nn("name", FieldType::VarChar(50))],
	)];
	creator.apply(schema).await.unwrap();
	let pool = creator.pool();

	// Act
	for name in &["first", "second", "third"] {
		sqlx::query("INSERT INTO e2e_ec03_table (name) VALUES ($1)")
			.bind(*name)
			.execute(pool.as_ref())
			.await
			.unwrap();
	}

	let rows = sqlx::query("SELECT id FROM e2e_ec03_table ORDER BY id")
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let ids: Vec<i32> = rows.iter().map(|r| r.get("id")).collect();
	assert_eq!(ids, vec![1, 2, 3]);
}

/// E2E-EC-04: INSERT then UPDATE non-PK field, verify PK unchanged
#[rstest]
#[serial(e2e_cycle)]
#[tokio::test]
async fn test_e2e_ec04_update_non_pk(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = vec![create_table(
		"e2e_ec04_table",
		vec![col_pk_auto("id"), col_nn("status", FieldType::VarChar(20))],
	)];
	creator.apply(schema).await.unwrap();
	let pool = creator.pool();

	// Act - insert
	sqlx::query("INSERT INTO e2e_ec04_table (status) VALUES ($1)")
		.bind("pending")
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Get original id
	let row_before = sqlx::query("SELECT id, status FROM e2e_ec04_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();
	let original_id: i32 = row_before.get("id");

	// Update status
	sqlx::query("UPDATE e2e_ec04_table SET status = $1 WHERE id = $2")
		.bind("completed")
		.bind(original_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row_after = sqlx::query("SELECT id, status FROM e2e_ec04_table WHERE id = $1")
		.bind(original_id)
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(row_after.get::<i32, _>("id"), original_id);
	assert_eq!(row_after.get::<String, _>("status"), "completed");
}

/// E2E-EC-05: INSERT then DELETE then INSERT - new row gets next ID
#[rstest]
#[serial(e2e_cycle)]
#[tokio::test]
async fn test_e2e_ec05_delete_and_reinsert(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = vec![create_table(
		"e2e_ec05_table",
		vec![col_pk_auto("id"), col_nn("value", FieldType::VarChar(50))],
	)];
	creator.apply(schema).await.unwrap();
	let pool = creator.pool();

	// Act - insert first row
	sqlx::query("INSERT INTO e2e_ec05_table (value) VALUES ($1)")
		.bind("first")
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Delete the row
	sqlx::query("DELETE FROM e2e_ec05_table WHERE value = $1")
		.bind("first")
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Insert new row
	sqlx::query("INSERT INTO e2e_ec05_table (value) VALUES ($1)")
		.bind("second")
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row = sqlx::query("SELECT id, value FROM e2e_ec05_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert - auto-increment should continue from 2, not reuse 1
	let id: i32 = row.get("id");
	assert_eq!(id, 2);
	assert_eq!(row.get::<String, _>("value"), "second");
}
