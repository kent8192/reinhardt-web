//! MySQL end-to-end cycle integration tests for migration system
//!
//! Tests the full cycle: define schema -> apply migration -> INSERT/SELECT data -> verify round-trip.
//! Validates that the migration system produces real, usable database tables on MySQL.
//!
//! Adapted from PostgreSQL e2e cycle tests with MySQL-specific adjustments:
//! - Uses `?` placeholders instead of `$1, $2, ...`
//! - Uses `NaiveDateTime` instead of `DateTime<Utc>` for timestamps
//! - Skips UUID PK auto-generation test (HP-09) - MySQL has no native UUID type
//! - Uses `LAST_INSERT_ID()` instead of `RETURNING` clause
//! - Boolean stored as TINYINT(1)
//!
//! **Fixtures Used:**
//! - mysql_container: MySQL database container

use reinhardt_db::backends::DatabaseConnection;
use reinhardt_db::migrations::{
	ColumnDefinition, FieldType, Migration, Operation, executor::DatabaseMigrationExecutor,
};
use reinhardt_test::fixtures::mysql_container;
use rstest::*;
use serial_test::serial;
use sqlx::Row;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

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

/// Create a simple migration for testing
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

/// Apply migration operations using the MySQL migration executor
async fn apply_migration(
	url: &str,
	table_name: &str,
	operations: Vec<Operation>,
) {
	let connection = DatabaseConnection::connect_mysql(url)
		.await
		.expect("Failed to connect to MySQL");
	let mut executor = DatabaseMigrationExecutor::new(connection);
	let migration = create_test_migration("testapp", &format!("create_{}", table_name), operations);
	executor
		.apply_migrations(&[migration])
		.await
		.expect("Failed to apply migration");
}

// ============================================================================
// Happy Path Tests (E2E-HP-01 to E2E-HP-20, skipping HP-09)
// ============================================================================

/// E2E-HP-01: Basic CRUD - create table, insert, select, verify values match
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_hp01_basic_crud(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_hp01_table",
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
	apply_migration(&url, "me2e_hp01_table", schema).await;

	// Act
	sqlx::query("INSERT INTO me2e_hp01_table (name, active) VALUES (?, ?)")
		.bind("test")
		.bind(true)
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row = sqlx::query("SELECT id, name, active FROM me2e_hp01_table WHERE name = ?")
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

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_hp01_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-HP-02: Auto-increment PK - insert without specifying id, verify id > 0
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_hp02_auto_increment_pk(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_hp02_table",
		vec![col_pk_auto("id"), col_nn("label", FieldType::VarChar(50))],
	)];
	apply_migration(&url, "me2e_hp02_table", schema).await;

	// Act
	sqlx::query("INSERT INTO me2e_hp02_table (label) VALUES (?)")
		.bind("auto_test")
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row = sqlx::query("SELECT id FROM me2e_hp02_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let id: i32 = row.get("id");
	assert!(id > 0);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_hp02_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-HP-03: auto_now_add timestamp - insert with default CURRENT_TIMESTAMP, verify non-null
///
/// MySQL uses DATETIME/TIMESTAMP without timezone. We use NaiveDateTime.
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_hp03_auto_now_add_timestamp(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_hp03_table",
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
	apply_migration(&url, "me2e_hp03_table", schema).await;

	// Act - insert without specifying created_at; MySQL needs at least one column
	// so we use an explicit column list with DEFAULT for created_at
	sqlx::query("INSERT INTO me2e_hp03_table (created_at) VALUES (DEFAULT)")
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row = sqlx::query("SELECT id, created_at FROM me2e_hp03_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let created_at: chrono::NaiveDateTime = row.get("created_at");
	assert!(created_at.and_utc().timestamp() > 0);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_hp03_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-HP-04: Multiple rows - insert 3 rows, verify count and values
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_hp04_multiple_rows(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_hp04_table",
		vec![col_pk_auto("id"), col_nn("value", FieldType::VarChar(50))],
	)];
	apply_migration(&url, "me2e_hp04_table", schema).await;

	// Act
	for val in &["alpha", "beta", "gamma"] {
		sqlx::query("INSERT INTO me2e_hp04_table (value) VALUES (?)")
			.bind(*val)
			.execute(pool.as_ref())
			.await
			.unwrap();
	}

	let rows = sqlx::query("SELECT id, value FROM me2e_hp04_table ORDER BY id")
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 3);
	assert_eq!(rows[0].get::<String, _>("value"), "alpha");
	assert_eq!(rows[1].get::<String, _>("value"), "beta");
	assert_eq!(rows[2].get::<String, _>("value"), "gamma");

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_hp04_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-HP-05: Boolean default - insert without bool field, verify default applied
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_hp05_boolean_default(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_hp05_table",
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
	apply_migration(&url, "me2e_hp05_table", schema).await;

	// Act - insert without specifying is_active
	sqlx::query("INSERT INTO me2e_hp05_table (is_active) VALUES (DEFAULT)")
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row = sqlx::query("SELECT is_active FROM me2e_hp05_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let is_active: bool = row.get("is_active");
	assert_eq!(is_active, false);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_hp05_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-HP-06: Integer default - insert without int field, verify default applied
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_hp06_integer_default(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_hp06_table",
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
	apply_migration(&url, "me2e_hp06_table", schema).await;

	// Act
	sqlx::query("INSERT INTO me2e_hp06_table (score) VALUES (DEFAULT)")
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row = sqlx::query("SELECT score FROM me2e_hp06_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let score: i32 = row.get("score");
	assert_eq!(score, 0);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_hp06_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-HP-07: Nullable field NULL - insert NULL, verify field is NULL
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_hp07_nullable_field_null(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_hp07_table",
		vec![col_pk_auto("id"), col("description", FieldType::Text)],
	)];
	apply_migration(&url, "me2e_hp07_table", schema).await;

	// Act
	sqlx::query("INSERT INTO me2e_hp07_table (description) VALUES (?)")
		.bind(None::<String>)
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row = sqlx::query("SELECT description FROM me2e_hp07_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let desc: Option<String> = row.get("description");
	assert_eq!(desc, None);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_hp07_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-HP-08: Nullable field with value - insert actual value, verify preserved
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_hp08_nullable_field_with_value(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_hp08_table",
		vec![col_pk_auto("id"), col("description", FieldType::Text)],
	)];
	apply_migration(&url, "me2e_hp08_table", schema).await;

	// Act
	sqlx::query("INSERT INTO me2e_hp08_table (description) VALUES (?)")
		.bind("some description")
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row = sqlx::query("SELECT description FROM me2e_hp08_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let desc: Option<String> = row.get("description");
	assert_eq!(desc, Some("some description".to_string()));

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_hp08_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

// E2E-HP-09: UUID PK - SKIPPED for MySQL
// MySQL has no native UUID type. UUID columns use CHAR(36)/VARCHAR(36)
// and cannot auto-generate UUIDs at the database level.

/// E2E-HP-10: Multi-type table - table with many types, verify all round-trip
///
/// MySQL adaptation: excludes UUID type, uses NaiveDateTime for timestamps
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_hp10_multi_type_table(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_hp10_table",
		vec![
			col_pk_auto("id"),
			col_nn("int_val", FieldType::Integer),
			col_nn("str_val", FieldType::VarChar(100)),
			col_nn("bool_val", FieldType::Boolean),
			col_nn("float_val", FieldType::Float),
			col_nn("double_val", FieldType::Double),
			col_nn("date_val", FieldType::Date),
		],
	)];
	apply_migration(&url, "me2e_hp10_table", schema).await;

	// Act
	let test_date = chrono::NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
	sqlx::query(
		"INSERT INTO me2e_hp10_table (int_val, str_val, bool_val, float_val, double_val, date_val) \
		 VALUES (?, ?, ?, ?, ?, ?)",
	)
	.bind(42i32)
	.bind("hello")
	.bind(true)
	.bind(3.14f32)
	.bind(2.718281828f64)
	.bind(test_date)
	.execute(pool.as_ref())
	.await
	.unwrap();

	let row = sqlx::query("SELECT * FROM me2e_hp10_table")
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

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_hp10_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-HP-11: Decimal(10,2) INSERT and SELECT roundtrip
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_hp11_decimal_roundtrip(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_hp11_table",
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
	apply_migration(&url, "me2e_hp11_table", schema).await;

	// Act
	sqlx::query("INSERT INTO me2e_hp11_table (price) VALUES (?)")
		.bind("123.45")
		.execute(pool.as_ref())
		.await
		.unwrap();
	let row = sqlx::query("SELECT CAST(price AS CHAR) as price FROM me2e_hp11_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let price: String = row.get("price");
	assert_eq!(price, "123.45");

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_hp11_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-HP-12: Text type INSERT large text and SELECT roundtrip
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_hp12_large_text_roundtrip(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_hp12_table",
		vec![col_pk_auto("id"), col_nn("content", FieldType::Text)],
	)];
	apply_migration(&url, "me2e_hp12_table", schema).await;
	let large_text = "a".repeat(1000);

	// Act
	sqlx::query("INSERT INTO me2e_hp12_table (content) VALUES (?)")
		.bind(&large_text)
		.execute(pool.as_ref())
		.await
		.unwrap();
	let row = sqlx::query("SELECT content FROM me2e_hp12_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let content: String = row.get("content");
	assert_eq!(content.len(), 1000);
	assert_eq!(content, large_text);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_hp12_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-HP-13: SmallInteger INSERT and SELECT roundtrip
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_hp13_smallinteger_roundtrip(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_hp13_table",
		vec![
			col_pk_auto("id"),
			col_nn("small_val", FieldType::SmallInteger),
		],
	)];
	apply_migration(&url, "me2e_hp13_table", schema).await;

	// Act
	sqlx::query("INSERT INTO me2e_hp13_table (small_val) VALUES (?)")
		.bind(32000_i16)
		.execute(pool.as_ref())
		.await
		.unwrap();
	let row = sqlx::query("SELECT small_val FROM me2e_hp13_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let val: i16 = row.get("small_val");
	assert_eq!(val, 32000);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_hp13_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-HP-14: Date INSERT and SELECT roundtrip
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_hp14_date_roundtrip(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_hp14_table",
		vec![col_pk_auto("id"), col_nn("birth_date", FieldType::Date)],
	)];
	apply_migration(&url, "me2e_hp14_table", schema).await;

	// Act
	let test_date = chrono::NaiveDate::from_ymd_opt(2026, 3, 2).unwrap();
	sqlx::query("INSERT INTO me2e_hp14_table (birth_date) VALUES (?)")
		.bind(test_date)
		.execute(pool.as_ref())
		.await
		.unwrap();
	let row = sqlx::query("SELECT birth_date FROM me2e_hp14_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let date: chrono::NaiveDate = row.get("birth_date");
	assert_eq!(date, test_date);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_hp14_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-HP-15: Time INSERT and SELECT roundtrip
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_hp15_time_roundtrip(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_hp15_table",
		vec![col_pk_auto("id"), col_nn("event_time", FieldType::Time)],
	)];
	apply_migration(&url, "me2e_hp15_table", schema).await;

	// Act
	let test_time = chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap();
	sqlx::query("INSERT INTO me2e_hp15_table (event_time) VALUES (?)")
		.bind(test_time)
		.execute(pool.as_ref())
		.await
		.unwrap();
	let row = sqlx::query("SELECT event_time FROM me2e_hp15_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let time: chrono::NaiveTime = row.get("event_time");
	assert_eq!(time, test_time);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_hp15_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-HP-16: Boolean INSERT true/false and SELECT roundtrip
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_hp16_boolean_roundtrip(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_hp16_table",
		vec![col_pk_auto("id"), col_nn("flag", FieldType::Boolean)],
	)];
	apply_migration(&url, "me2e_hp16_table", schema).await;

	// Act
	sqlx::query("INSERT INTO me2e_hp16_table (flag) VALUES (?)")
		.bind(true)
		.execute(pool.as_ref())
		.await
		.unwrap();
	sqlx::query("INSERT INTO me2e_hp16_table (flag) VALUES (?)")
		.bind(false)
		.execute(pool.as_ref())
		.await
		.unwrap();
	let rows = sqlx::query("SELECT flag FROM me2e_hp16_table ORDER BY id")
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 2);
	assert_eq!(rows[0].get::<bool, _>("flag"), true);
	assert_eq!(rows[1].get::<bool, _>("flag"), false);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_hp16_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-HP-17: Multiple nullable columns with mixed NULL and non-NULL values
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_hp17_nullable_columns(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_hp17_table",
		vec![
			col_pk_auto("id"),
			col_nullable("name", FieldType::VarChar(100)),
			col_nullable("age", FieldType::Integer),
			col_nullable("email", FieldType::VarChar(200)),
		],
	)];
	apply_migration(&url, "me2e_hp17_table", schema).await;

	// Act - insert with some NULL values
	sqlx::query("INSERT INTO me2e_hp17_table (name, age, email) VALUES (?, ?, ?)")
		.bind("Alice")
		.bind(None::<i32>)
		.bind("alice@example.com")
		.execute(pool.as_ref())
		.await
		.unwrap();
	let row = sqlx::query("SELECT name, age, email FROM me2e_hp17_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(
		row.get::<Option<String>, _>("name"),
		Some("Alice".to_string())
	);
	assert_eq!(row.get::<Option<i32>, _>("age"), None);
	assert_eq!(
		row.get::<Option<String>, _>("email"),
		Some("alice@example.com".to_string())
	);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_hp17_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-HP-18: Default values for multiple types applied when columns omitted
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_hp18_default_values(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_hp18_table",
		vec![
			col_pk_auto("id"),
			col_nn("name", FieldType::VarChar(100)),
			col_default("status", FieldType::VarChar(20), "'active'"),
			col_default("score", FieldType::Integer, "0"),
			col_default("verified", FieldType::Boolean, "false"),
		],
	)];
	apply_migration(&url, "me2e_hp18_table", schema).await;

	// Act - insert without default columns
	sqlx::query("INSERT INTO me2e_hp18_table (name) VALUES (?)")
		.bind("Bob")
		.execute(pool.as_ref())
		.await
		.unwrap();
	let row = sqlx::query("SELECT name, status, score, verified FROM me2e_hp18_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(row.get::<String, _>("name"), "Bob");
	assert_eq!(row.get::<Option<String>, _>("status"), Some("active".to_string()));
	assert_eq!(row.get::<Option<i32>, _>("score"), Some(0));
	assert_eq!(row.get::<Option<bool>, _>("verified"), Some(false));

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_hp18_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-HP-19: Unique constraint with INSERT then UPDATE preserving uniqueness
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_hp19_unique_update(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_hp19_table",
		vec![
			col_pk_auto("id"),
			col_unique("code", FieldType::VarChar(50)),
		],
	)];
	apply_migration(&url, "me2e_hp19_table", schema).await;

	// Act - insert and then update the unique column
	sqlx::query("INSERT INTO me2e_hp19_table (code) VALUES (?)")
		.bind("ABC")
		.execute(pool.as_ref())
		.await
		.unwrap();
	sqlx::query("UPDATE me2e_hp19_table SET code = ? WHERE code = ?")
		.bind("XYZ")
		.bind("ABC")
		.execute(pool.as_ref())
		.await
		.unwrap();
	let row = sqlx::query("SELECT code FROM me2e_hp19_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(row.get::<String, _>("code"), "XYZ");

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_hp19_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-HP-20: INSERT row with all supported basic types at once (MySQL version, no UUID)
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_hp20_all_basic_types(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_hp20_table",
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
	apply_migration(&url, "me2e_hp20_table", schema).await;

	// Act
	let test_date = chrono::NaiveDate::from_ymd_opt(2026, 3, 2).unwrap();
	sqlx::query(
		"INSERT INTO me2e_hp20_table (int_col, varchar_col, bool_col, float_col, double_col, date_col) \
		 VALUES (?, ?, ?, ?, ?, ?)",
	)
	.bind(42_i32)
	.bind("hello")
	.bind(true)
	.bind(3.14_f32)
	.bind(2.71828_f64)
	.bind(test_date)
	.execute(pool.as_ref())
	.await
	.unwrap();
	let row = sqlx::query("SELECT int_col, varchar_col, bool_col, float_col, double_col, date_col FROM me2e_hp20_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(row.get::<i32, _>("int_col"), 42);
	assert_eq!(row.get::<String, _>("varchar_col"), "hello");
	assert_eq!(row.get::<bool, _>("bool_col"), true);
	assert_eq!(row.get::<f32, _>("float_col"), 3.14_f32);
	assert_eq!(row.get::<f64, _>("double_col"), 2.71828_f64);
	assert_eq!(row.get::<chrono::NaiveDate, _>("date_col"), test_date);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_hp20_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

// ============================================================================
// Error Path Tests (E2E-EP-01 to E2E-EP-08)
// ============================================================================

/// E2E-EP-01: INSERT NULL for NOT NULL field - expect DB error
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_ep01_null_for_not_null(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_ep01_table",
		vec![
			col_pk_auto("id"),
			col_nn("required_field", FieldType::VarChar(100)),
		],
	)];
	apply_migration(&url, "me2e_ep01_table", schema).await;

	// Act
	let result = sqlx::query("INSERT INTO me2e_ep01_table (required_field) VALUES (?)")
		.bind(None::<String>)
		.execute(pool.as_ref())
		.await;

	// Assert
	assert!(result.is_err());

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_ep01_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-EP-02: INSERT duplicate unique value - expect unique constraint error
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_ep02_duplicate_unique(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_ep02_table",
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
	apply_migration(&url, "me2e_ep02_table", schema).await;

	// Act - first insert succeeds
	sqlx::query("INSERT INTO me2e_ep02_table (email) VALUES (?)")
		.bind("user@example.com")
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Second insert with same value should fail
	let result = sqlx::query("INSERT INTO me2e_ep02_table (email) VALUES (?)")
		.bind("user@example.com")
		.execute(pool.as_ref())
		.await;

	// Assert
	assert!(result.is_err());

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_ep02_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-EP-03: INSERT 256 chars into VarChar(255) - expect DB error
///
/// MySQL 8 in strict mode (default) rejects data that exceeds column length.
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_ep03_varchar_overflow(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_ep03_table",
		vec![
			col_pk_auto("id"),
			col_nn("short_text", FieldType::VarChar(255)),
		],
	)];
	apply_migration(&url, "me2e_ep03_table", schema).await;

	// Act - insert string exceeding varchar length
	let long_string = "x".repeat(256);
	let result = sqlx::query("INSERT INTO me2e_ep03_table (short_text) VALUES (?)")
		.bind(&long_string)
		.execute(pool.as_ref())
		.await;

	// Assert
	assert!(result.is_err());

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_ep03_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-EP-04: INSERT string into integer column - expect DB type error
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_ep04_type_mismatch(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_ep04_table",
		vec![col_pk_auto("id"), col_nn("amount", FieldType::Integer)],
	)];
	apply_migration(&url, "me2e_ep04_table", schema).await;

	// Act - use raw SQL to bypass type checking in sqlx bindings
	let result = sqlx::query("INSERT INTO me2e_ep04_table (amount) VALUES ('not_a_number')")
		.execute(pool.as_ref())
		.await;

	// Assert
	assert!(result.is_err());

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_ep04_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-EP-05: SELECT from non-existent table - expect DB error
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_ep05_select_nonexistent_table(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, _url) = mysql_container.await;

	// Act
	let result = sqlx::query("SELECT * FROM me2e_ep05_nonexistent_table")
		.fetch_all(pool.as_ref())
		.await;

	// Assert
	assert!(result.is_err());
}

/// E2E-EP-06: UPDATE to violate unique constraint returns error
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_ep06_update_unique_violation(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_ep06_table",
		vec![
			col_pk_auto("id"),
			col_unique("code", FieldType::VarChar(50)),
		],
	)];
	apply_migration(&url, "me2e_ep06_table", schema).await;

	sqlx::query("INSERT INTO me2e_ep06_table (code) VALUES (?)")
		.bind("AAA")
		.execute(pool.as_ref())
		.await
		.unwrap();
	sqlx::query("INSERT INTO me2e_ep06_table (code) VALUES (?)")
		.bind("BBB")
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Act - update second row to conflict with first row
	let result = sqlx::query("UPDATE me2e_ep06_table SET code = ? WHERE code = ?")
		.bind("AAA")
		.bind("BBB")
		.execute(pool.as_ref())
		.await;

	// Assert
	assert!(result.is_err());

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_ep06_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-EP-07: INSERT SmallInteger overflow value returns error
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_ep07_smallinteger_overflow(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_ep07_table",
		vec![
			col_pk_auto("id"),
			col_nn("small_val", FieldType::SmallInteger),
		],
	)];
	apply_migration(&url, "me2e_ep07_table", schema).await;

	// Act - 40000 exceeds SmallInteger max (32767); MySQL strict mode rejects this
	let result = sqlx::query("INSERT INTO me2e_ep07_table (small_val) VALUES (?)")
		.bind(40000_i32)
		.execute(pool.as_ref())
		.await;

	// Assert
	assert!(result.is_err());

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_ep07_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-EP-08: DELETE non-existent row succeeds with 0 rows affected
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_ep08_delete_nonexistent_row(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_ep08_table",
		vec![col_pk_auto("id"), col_nn("name", FieldType::VarChar(50))],
	)];
	apply_migration(&url, "me2e_ep08_table", schema).await;

	// Act - delete from empty table
	let result = sqlx::query("DELETE FROM me2e_ep08_table WHERE id = ?")
		.bind(999_i32)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(result.rows_affected(), 0);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_ep08_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

// ============================================================================
// Edge Case Tests (E2E-EC-01 to E2E-EC-10)
// ============================================================================

/// E2E-EC-01: SELECT from empty table - expect 0 rows
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_ec01_select_empty_table(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_ec01_table",
		vec![col_pk_auto("id"), col("name", FieldType::Text)],
	)];
	apply_migration(&url, "me2e_ec01_table", schema).await;

	// Act
	let rows = sqlx::query("SELECT * FROM me2e_ec01_table")
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 0);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_ec01_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-EC-02: INSERT exactly 255 chars into VarChar(255) - expect success
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_ec02_varchar_exact_limit(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_ec02_table",
		vec![
			col_pk_auto("id"),
			col_nn("bounded_text", FieldType::VarChar(255)),
		],
	)];
	apply_migration(&url, "me2e_ec02_table", schema).await;

	// Act
	let exact_string = "a".repeat(255);
	sqlx::query("INSERT INTO me2e_ec02_table (bounded_text) VALUES (?)")
		.bind(&exact_string)
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row = sqlx::query("SELECT bounded_text FROM me2e_ec02_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let retrieved: String = row.get("bounded_text");
	assert_eq!(retrieved.len(), 255);
	assert_eq!(retrieved, exact_string);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_ec02_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-EC-03: INSERT 3 rows, verify sequential auto-increment IDs
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_ec03_sequential_ids(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_ec03_table",
		vec![col_pk_auto("id"), col_nn("name", FieldType::VarChar(50))],
	)];
	apply_migration(&url, "me2e_ec03_table", schema).await;

	// Act
	for name in &["first", "second", "third"] {
		sqlx::query("INSERT INTO me2e_ec03_table (name) VALUES (?)")
			.bind(*name)
			.execute(pool.as_ref())
			.await
			.unwrap();
	}

	let rows = sqlx::query("SELECT id FROM me2e_ec03_table ORDER BY id")
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let ids: Vec<i32> = rows.iter().map(|r| r.get("id")).collect();
	assert_eq!(ids, vec![1, 2, 3]);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_ec03_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-EC-04: INSERT then UPDATE non-PK field, verify PK unchanged
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_ec04_update_non_pk(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_ec04_table",
		vec![col_pk_auto("id"), col_nn("status", FieldType::VarChar(20))],
	)];
	apply_migration(&url, "me2e_ec04_table", schema).await;

	// Act - insert
	sqlx::query("INSERT INTO me2e_ec04_table (status) VALUES (?)")
		.bind("pending")
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Get original id
	let row_before = sqlx::query("SELECT id, status FROM me2e_ec04_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();
	let original_id: i32 = row_before.get("id");

	// Update status
	sqlx::query("UPDATE me2e_ec04_table SET status = ? WHERE id = ?")
		.bind("completed")
		.bind(original_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row_after = sqlx::query("SELECT id, status FROM me2e_ec04_table WHERE id = ?")
		.bind(original_id)
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(row_after.get::<i32, _>("id"), original_id);
	assert_eq!(row_after.get::<String, _>("status"), "completed");

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_ec04_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-EC-05: INSERT then DELETE then INSERT - new row gets next ID
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_ec05_delete_and_reinsert(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_ec05_table",
		vec![col_pk_auto("id"), col_nn("value", FieldType::VarChar(50))],
	)];
	apply_migration(&url, "me2e_ec05_table", schema).await;

	// Act - insert first row
	sqlx::query("INSERT INTO me2e_ec05_table (value) VALUES (?)")
		.bind("first")
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Delete the row
	sqlx::query("DELETE FROM me2e_ec05_table WHERE value = ?")
		.bind("first")
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Insert new row
	sqlx::query("INSERT INTO me2e_ec05_table (value) VALUES (?)")
		.bind("second")
		.execute(pool.as_ref())
		.await
		.unwrap();

	let row = sqlx::query("SELECT id, value FROM me2e_ec05_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert - auto-increment should continue from 2, not reuse 1
	let id: i32 = row.get("id");
	assert_eq!(id, 2);
	assert_eq!(row.get::<String, _>("value"), "second");

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_ec05_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-EC-06: INSERT 100 rows and verify COUNT
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_ec06_bulk_insert_count(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_ec06_table",
		vec![col_pk_auto("id"), col_nn("value", FieldType::Integer)],
	)];
	apply_migration(&url, "me2e_ec06_table", schema).await;

	// Act
	for i in 0..100 {
		sqlx::query("INSERT INTO me2e_ec06_table (value) VALUES (?)")
			.bind(i)
			.execute(pool.as_ref())
			.await
			.unwrap();
	}
	let row = sqlx::query("SELECT COUNT(*) as cnt FROM me2e_ec06_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let count: i64 = row.get("cnt");
	assert_eq!(count, 100);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_ec06_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-EC-07: INSERT then UPDATE all rows and verify
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_ec07_update_all_rows(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_ec07_table",
		vec![col_pk_auto("id"), col_nn("status", FieldType::VarChar(20))],
	)];
	apply_migration(&url, "me2e_ec07_table", schema).await;

	for _ in 0..5 {
		sqlx::query("INSERT INTO me2e_ec07_table (status) VALUES (?)")
			.bind("pending")
			.execute(pool.as_ref())
			.await
			.unwrap();
	}

	// Act - update all rows
	sqlx::query("UPDATE me2e_ec07_table SET status = ?")
		.bind("done")
		.execute(pool.as_ref())
		.await
		.unwrap();
	let rows = sqlx::query("SELECT status FROM me2e_ec07_table")
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 5);
	for row in &rows {
		assert_eq!(row.get::<String, _>("status"), "done");
	}

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_ec07_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-EC-08: Empty string INSERT into VarChar roundtrip
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_ec08_empty_string_roundtrip(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_ec08_table",
		vec![col_pk_auto("id"), col_nn("name", FieldType::VarChar(100))],
	)];
	apply_migration(&url, "me2e_ec08_table", schema).await;

	// Act
	sqlx::query("INSERT INTO me2e_ec08_table (name) VALUES (?)")
		.bind("")
		.execute(pool.as_ref())
		.await
		.unwrap();
	let row = sqlx::query("SELECT name FROM me2e_ec08_table")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(row.get::<String, _>("name"), "");

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_ec08_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-EC-09: Integer MIN and MAX value roundtrip
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_ec09_integer_min_max(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![create_table(
		"me2e_ec09_table",
		vec![col_pk_auto("id"), col_nn("value", FieldType::Integer)],
	)];
	apply_migration(&url, "me2e_ec09_table", schema).await;

	// Act
	sqlx::query("INSERT INTO me2e_ec09_table (value) VALUES (?)")
		.bind(i32::MIN)
		.execute(pool.as_ref())
		.await
		.unwrap();
	sqlx::query("INSERT INTO me2e_ec09_table (value) VALUES (?)")
		.bind(i32::MAX)
		.execute(pool.as_ref())
		.await
		.unwrap();
	let rows = sqlx::query("SELECT value FROM me2e_ec09_table ORDER BY id")
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 2);
	assert_eq!(rows[0].get::<i32, _>("value"), i32::MIN);
	assert_eq!(rows[1].get::<i32, _>("value"), i32::MAX);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_ec09_table")
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// E2E-EC-10: Multiple tables in same schema with independent INSERT/SELECT
#[rstest]
#[tokio::test]
#[serial(mysql_e2e)]
async fn test_mysql_e2e_ec10_multiple_tables(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let schema = vec![
		create_table(
			"me2e_ec10_alpha",
			vec![col_pk_auto("id"), col_nn("name", FieldType::VarChar(50))],
		),
		create_table(
			"me2e_ec10_beta",
			vec![col_pk_auto("id"), col_nn("value", FieldType::Integer)],
		),
	];
	apply_migration(&url, "me2e_ec10_tables", schema).await;

	// Act - insert into both tables independently
	sqlx::query("INSERT INTO me2e_ec10_alpha (name) VALUES (?)")
		.bind("alpha_row")
		.execute(pool.as_ref())
		.await
		.unwrap();
	sqlx::query("INSERT INTO me2e_ec10_beta (value) VALUES (?)")
		.bind(999_i32)
		.execute(pool.as_ref())
		.await
		.unwrap();

	let alpha_row = sqlx::query("SELECT name FROM me2e_ec10_alpha")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();
	let beta_row = sqlx::query("SELECT value FROM me2e_ec10_beta")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(alpha_row.get::<String, _>("name"), "alpha_row");
	assert_eq!(beta_row.get::<i32, _>("value"), 999);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS me2e_ec10_alpha")
		.execute(pool.as_ref())
		.await
		.unwrap();
	sqlx::query("DROP TABLE IF EXISTS me2e_ec10_beta")
		.execute(pool.as_ref())
		.await
		.unwrap();
}
