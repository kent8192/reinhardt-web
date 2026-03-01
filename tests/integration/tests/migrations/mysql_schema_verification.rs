//! MySQL schema verification integration tests
//!
//! Tests apply migrations and then query MySQL's `information_schema`
//! to verify that database-level column properties (type, nullability,
//! defaults, constraints) match the intended schema definition.
//!
//! **Test Coverage:**
//! - Happy path: 22 scenarios (MSV-HP-01 to MSV-HP-22)
//! - Error path: 5 scenarios (MSV-EP-01 to MSV-EP-05)
//! - Edge cases: 5 scenarios (MSV-EC-01 to MSV-EC-05)
//!
//! **Fixtures Used:**
//! - `mysql_container`: MySQL database container

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
// Helper Functions
// ============================================================================

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

/// Apply a migration using the MySQL executor pattern
async fn apply_migration(url: &str, operations: Vec<Operation>, app: &str, name: &str) {
	let connection = DatabaseConnection::connect_mysql(url)
		.await
		.expect("Failed to connect to MySQL");
	let mut executor = DatabaseMigrationExecutor::new(connection);
	let migration = create_test_migration(app, name, operations);
	executor
		.apply_migrations(&[migration])
		.await
		.expect("Failed to apply migration");
}

// ============================================================================
// Happy Path Tests (MSV-HP-01 to MSV-HP-12)
// ============================================================================

/// MSV-HP-01: Integer PK with auto_increment has AUTO_INCREMENT extra
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_01_serial_pk_auto_increment(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp01_table",
		vec![col("id", FieldType::Integer, true, false, true, true, None)],
	);
	apply_migration(&url, ops, "testapp", "0001_hp01").await;

	// Act
	let row = sqlx::query(
		"SELECT column_default, is_nullable, data_type, extra \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp01_table' AND column_name = 'id'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let extra: String = row.get::<String, _>(3);
	assert!(
		extra.contains("auto_increment"),
		"auto_increment PK should have auto_increment in extra, got: {:?}",
		extra
	);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp01_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-HP-02: BigInteger PK with auto_increment maps to bigint with auto_increment
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_02_biginteger_pk_auto_increment(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp02_table",
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
	apply_migration(&url, ops, "testapp", "0001_hp02").await;

	// Act
	let row = sqlx::query(
		"SELECT data_type, extra \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp02_table' AND column_name = 'id'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let data_type: String = row.get::<String, _>(0);
	assert_eq!(data_type, "bigint", "BigInteger should map to bigint");

	let extra: String = row.get::<String, _>(1);
	assert!(
		extra.contains("auto_increment"),
		"auto_increment BigInteger PK should have auto_increment in extra, got: {:?}",
		extra
	);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp02_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-HP-03: VarChar(255) NOT NULL maps correctly
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_03_varchar_255_not_null(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp03_table",
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
	apply_migration(&url, ops, "testapp", "0001_hp03").await;

	// Act
	let row = sqlx::query(
		"SELECT data_type, character_maximum_length, is_nullable \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp03_table' AND column_name = 'name'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let data_type: String = row.get::<String, _>(0);
	assert_eq!(data_type, "varchar");

	let max_length: Option<i64> = row.get::<Option<i64>, _>(1);
	assert_eq!(
		max_length,
		Some(255),
		"VarChar(255) should have max length 255"
	);

	let is_nullable: String = row.get::<String, _>(2);
	assert_eq!(
		is_nullable, "NO",
		"NOT NULL column should have is_nullable='NO'"
	);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp03_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-HP-04: VarChar(100) nullable
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_04_varchar_nullable(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp04_table",
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
	apply_migration(&url, ops, "testapp", "0001_hp04").await;

	// Act
	let row = sqlx::query(
		"SELECT is_nullable \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp04_table' AND column_name = 'description'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let is_nullable: String = row.get::<String, _>(0);
	assert_eq!(
		is_nullable, "YES",
		"Nullable column should have is_nullable='YES'"
	);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp04_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-HP-05: Boolean with default "false" maps to tinyint with default '0'
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_05_boolean_default_false(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp05_table",
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
	apply_migration(&url, ops, "testapp", "0001_hp05").await;

	// Act
	let row = sqlx::query(
		"SELECT data_type, column_default, column_type \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp05_table' AND column_name = 'is_active'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let data_type: String = row.get::<String, _>(0);
	assert_eq!(data_type, "tinyint", "Boolean should map to tinyint in MySQL");

	let col_default: Option<String> = row.get::<Option<String>, _>(1);
	assert!(
		col_default
			.as_ref()
			.map_or(false, |d| d.contains("0") || d.contains("false")),
		"Boolean default false should be '0' in MySQL, got: {:?}",
		col_default
	);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp05_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-HP-06: TimestampTz maps to 'datetime' in MySQL
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_06_timestamp_tz(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp06_table",
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
	apply_migration(&url, ops, "testapp", "0001_hp06").await;

	// Act
	let row = sqlx::query(
		"SELECT data_type \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp06_table' AND column_name = 'created_at'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let data_type: String = row.get::<String, _>(0);
	assert_eq!(data_type, "datetime", "TimestampTz should map to datetime in MySQL");

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp06_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-HP-07: DateTime maps to 'datetime' in MySQL
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_07_datetime_no_tz(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp07_table",
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
	apply_migration(&url, ops, "testapp", "0001_hp07").await;

	// Act
	let row = sqlx::query(
		"SELECT data_type \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp07_table' AND column_name = 'event_time'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let data_type: String = row.get::<String, _>(0);
	assert_eq!(data_type, "datetime", "DateTime should map to datetime in MySQL");

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp07_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-HP-08: Integer NOT NULL
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_08_integer_not_null(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp08_table",
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
	apply_migration(&url, ops, "testapp", "0001_hp08").await;

	// Act
	let row = sqlx::query(
		"SELECT data_type, is_nullable \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp08_table' AND column_name = 'quantity'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let data_type: String = row.get::<String, _>(0);
	assert_eq!(data_type, "int");

	let is_nullable: String = row.get::<String, _>(1);
	assert_eq!(is_nullable, "NO");

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp08_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-HP-09: Uuid maps to 'char(36)' in MySQL
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_09_uuid(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp09_table",
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
	apply_migration(&url, ops, "testapp", "0001_hp09").await;

	// Act
	let row = sqlx::query(
		"SELECT data_type, character_maximum_length \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp09_table' AND column_name = 'external_id'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let data_type: String = row.get::<String, _>(0);
	assert_eq!(data_type, "char", "Uuid should map to char in MySQL");

	let max_length: Option<i64> = row.get::<Option<i64>, _>(1);
	assert_eq!(max_length, Some(36), "Uuid char should have length 36");

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp09_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-HP-10: Double maps to 'double' in MySQL
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_10_double(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp10_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col("score", FieldType::Double, false, false, false, false, None),
		],
	);
	apply_migration(&url, ops, "testapp", "0001_hp10").await;

	// Act
	let row = sqlx::query(
		"SELECT data_type \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp10_table' AND column_name = 'score'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let data_type: String = row.get::<String, _>(0);
	assert_eq!(data_type, "double", "Double should map to double in MySQL");

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp10_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-HP-11: Float maps to 'float' in MySQL
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_11_float(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp11_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col("weight", FieldType::Float, false, false, false, false, None),
		],
	);
	apply_migration(&url, ops, "testapp", "0001_hp11").await;

	// Act
	let row = sqlx::query(
		"SELECT data_type \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp11_table' AND column_name = 'weight'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let data_type: String = row.get::<String, _>(0);
	assert_eq!(data_type, "float", "Float should map to float in MySQL");

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp11_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-HP-12: VarChar(255) unique has UNIQUE constraint
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_12_varchar_unique(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp12_table",
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
	apply_migration(&url, ops, "testapp", "0001_hp12").await;

	// Act
	let row = sqlx::query(
		"SELECT COUNT(*) \
		 FROM information_schema.statistics \
		 WHERE table_schema = DATABASE() \
		   AND table_name = 'msv_hp12_table' \
		   AND column_name = 'email' \
		   AND non_unique = 0",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let unique_count: i64 = row.get::<i64, _>(0);
	assert!(
		unique_count > 0,
		"UNIQUE column should have a unique index entry"
	);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp12_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

// ============================================================================
// Happy Path Tests (MSV-HP-13 to MSV-HP-22)
// ============================================================================

/// MSV-HP-13: Text NOT NULL maps to data_type='text' and is_nullable='NO'
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_13_text_not_null(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp13_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col("content", FieldType::Text, true, false, false, false, None),
		],
	);
	apply_migration(&url, ops, "testapp", "0001_hp13").await;

	// Act
	let row = sqlx::query(
		"SELECT data_type, is_nullable \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp13_table' AND column_name = 'content'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	assert_eq!(row.get::<String, _>(0), "text");
	assert_eq!(row.get::<String, _>(1), "NO");

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp13_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-HP-14: Date type maps to data_type='date'
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_14_date_type(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp14_table",
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
	apply_migration(&url, ops, "testapp", "0001_hp14").await;

	// Act
	let row = sqlx::query(
		"SELECT data_type \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp14_table' AND column_name = 'birth_date'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	assert_eq!(row.get::<String, _>(0), "date");

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp14_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-HP-15: Time type maps to data_type='time'
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_15_time_type(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp15_table",
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
	apply_migration(&url, ops, "testapp", "0001_hp15").await;

	// Act
	let row = sqlx::query(
		"SELECT data_type \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp15_table' AND column_name = 'start_time'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	assert_eq!(row.get::<String, _>(0), "time");

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp15_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-HP-16: SmallInteger maps to data_type='smallint'
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_16_small_integer(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp16_table",
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
	apply_migration(&url, ops, "testapp", "0001_hp16").await;

	// Act
	let row = sqlx::query(
		"SELECT data_type \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp16_table' AND column_name = 'small_val'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	assert_eq!(row.get::<String, _>(0), "smallint");

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp16_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-HP-17: Decimal(10,2) maps to data_type='decimal' with precision=10 and scale=2
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_17_decimal(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp17_table",
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
	apply_migration(&url, ops, "testapp", "0001_hp17").await;

	// Act
	let row = sqlx::query(
		"SELECT data_type, numeric_precision, numeric_scale \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp17_table' AND column_name = 'price'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	assert_eq!(row.get::<String, _>(0), "decimal");
	assert_eq!(row.get::<i64, _>(1), 10);
	assert_eq!(row.get::<i64, _>(2), 2);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp17_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-HP-18: Json type maps to data_type='json'
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_18_json(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp18_table",
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
	apply_migration(&url, ops, "testapp", "0001_hp18").await;

	// Act
	let row = sqlx::query(
		"SELECT data_type \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp18_table' AND column_name = 'metadata'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	assert_eq!(row.get::<String, _>(0), "json");

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp18_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-HP-19: JsonBinary maps to data_type='json' in MySQL (no JSONB in MySQL)
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_19_jsonb_as_json(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp19_table",
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
	apply_migration(&url, ops, "testapp", "0001_hp19").await;

	// Act
	let row = sqlx::query(
		"SELECT data_type \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp19_table' AND column_name = 'data'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	// MySQL does not have JSONB; JsonBinary maps to JSON
	assert_eq!(row.get::<String, _>(0), "json");

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp19_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-HP-20: Char(5) maps to data_type='char' with character_maximum_length=5
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_20_char_fixed(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp20_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col("code", FieldType::Char(5), false, false, false, false, None),
		],
	);
	apply_migration(&url, ops, "testapp", "0001_hp20").await;

	// Act
	let row = sqlx::query(
		"SELECT data_type, character_maximum_length \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp20_table' AND column_name = 'code'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	assert_eq!(row.get::<String, _>(0), "char");
	assert_eq!(row.get::<i64, _>(1), 5);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp20_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-HP-21: Integer with default value 42 has column_default containing '42'
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_21_integer_default(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp21_table",
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
	apply_migration(&url, ops, "testapp", "0001_hp21").await;

	// Act
	let row = sqlx::query(
		"SELECT column_default \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp21_table' AND column_name = 'quantity'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let column_default: Option<String> = row.get::<Option<String>, _>(0);
	assert!(
		column_default.as_ref().map_or(false, |d| d.contains("42")),
		"column_default should contain '42', got: {:?}",
		column_default
	);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp21_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-HP-22: Multiple columns with defaults in the same table
/// (Boolean false, Integer 0, VarChar "draft")
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_hp_22_multiple_defaults(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_hp22_table",
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
	apply_migration(&url, ops, "testapp", "0001_hp22").await;

	// Act
	let rows = sqlx::query(
		"SELECT column_name, column_default \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_hp22_table' \
		   AND column_name IN ('is_active', 'counter', 'status') \
		 ORDER BY column_name",
	)
	.fetch_all(pool.as_ref())
	.await
	.unwrap();

	// Assert
	assert_eq!(rows.len(), 3);
	for row in &rows {
		let col_name: String = row.get::<String, _>(0);
		let col_default: Option<String> = row.get::<Option<String>, _>(1);
		match col_name.as_str() {
			"counter" => assert!(
				col_default.as_ref().map_or(false, |d| d.contains("0")),
				"counter default should contain '0', got: {:?}",
				col_default
			),
			"is_active" => assert!(
				col_default
					.as_ref()
					.map_or(false, |d| d.contains("0") || d.contains("false")),
				"is_active default should contain '0' (MySQL boolean false), got: {:?}",
				col_default
			),
			"status" => assert!(
				col_default.as_ref().map_or(false, |d| d.contains("draft")),
				"status default should contain 'draft', got: {:?}",
				col_default
			),
			_ => panic!("Unexpected column: {col_name}"),
		}
	}

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_hp22_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

// ============================================================================
// Error Path Tests (MSV-EP-01 to MSV-EP-05)
// ============================================================================

/// MSV-EP-01: INSERT NULL into NOT NULL column fails
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_ep_01_null_into_not_null(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_ep01_table",
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
	apply_migration(&url, ops, "testapp", "0001_ep01").await;

	// Act
	let result = sqlx::query("INSERT INTO msv_ep01_table (name) VALUES (NULL)")
		.execute(pool.as_ref())
		.await;

	// Assert
	assert!(
		result.is_err(),
		"INSERT NULL into NOT NULL column should fail"
	);
	let err = result.unwrap_err().to_string();
	assert!(
		err.contains("null") || err.contains("NULL") || err.contains("Column 'name' cannot be null"),
		"Error should indicate NOT NULL violation, got: {}",
		err
	);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_ep01_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-EP-02: INSERT duplicate into unique column fails
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_ep_02_duplicate_unique(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_ep02_table",
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
	apply_migration(&url, ops, "testapp", "0001_ep02").await;

	sqlx::query("INSERT INTO msv_ep02_table (email) VALUES ('test@example.com')")
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Act
	let result = sqlx::query("INSERT INTO msv_ep02_table (email) VALUES ('test@example.com')")
		.execute(pool.as_ref())
		.await;

	// Assert
	assert!(
		result.is_err(),
		"INSERT duplicate into unique column should fail"
	);
	let err = result.unwrap_err().to_string();
	assert!(
		err.contains("Duplicate") || err.contains("duplicate") || err.contains("unique"),
		"Error should indicate unique violation, got: {}",
		err
	);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_ep02_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-EP-03: INSERT string into integer column fails
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_ep_03_string_into_integer(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_ep03_table",
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
	apply_migration(&url, ops, "testapp", "0001_ep03").await;

	// Act
	// MySQL in strict mode rejects non-numeric strings for integer columns
	let result = sqlx::query("INSERT INTO msv_ep03_table (quantity) VALUES ('not_a_number')")
		.execute(pool.as_ref())
		.await;

	// Assert
	assert!(
		result.is_err(),
		"INSERT string into integer column should fail"
	);
	let err = result.unwrap_err().to_string();
	assert!(
		err.contains("Incorrect integer value") || err.contains("truncated") || err.contains("type"),
		"Error should indicate type mismatch, got: {}",
		err
	);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_ep03_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-EP-04: INSERT 256 chars into VarChar(255) fails
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_ep_04_exceeds_varchar_length(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_ep04_table",
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
	apply_migration(&url, ops, "testapp", "0001_ep04").await;

	// Act
	let long_value = "x".repeat(256);
	let query_str = format!(
		"INSERT INTO msv_ep04_table (short_text) VALUES ('{}')",
		long_value
	);
	let result = sqlx::query(&query_str).execute(pool.as_ref()).await;

	// Assert
	assert!(
		result.is_err(),
		"INSERT 256 chars into VarChar(255) should fail"
	);
	let err = result.unwrap_err().to_string();
	assert!(
		err.contains("Data too long") || err.contains("too long") || err.contains("truncated"),
		"Error should indicate length violation, got: {}",
		err
	);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_ep04_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-EP-05: INSERT without required NOT NULL column (no default) fails
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_ep_05_missing_not_null_column(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_ep05_table",
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
	apply_migration(&url, ops, "testapp", "0001_ep05").await;

	// Act: insert only optional_field, omitting required_field
	let result = sqlx::query("INSERT INTO msv_ep05_table (optional_field) VALUES ('some_value')")
		.execute(pool.as_ref())
		.await;

	// Assert
	assert!(
		result.is_err(),
		"INSERT without required NOT NULL column should fail"
	);
	let err = result.unwrap_err().to_string();
	assert!(
		err.contains("doesn't have a default value")
			|| err.contains("null")
			|| err.contains("NULL")
			|| err.contains("Field 'required_field'"),
		"Error should indicate NOT NULL violation, got: {}",
		err
	);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_ep05_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

// ============================================================================
// Edge Case Tests (MSV-EC-01 to MSV-EC-05)
// ============================================================================

/// MSV-EC-01: Auto-increment PK generates sequential values
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_ec_01_auto_increment_sequential(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_ec01_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col(
				"name",
				FieldType::VarChar(100),
				false,
				false,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&url, ops, "testapp", "0001_ec01").await;

	// Act
	sqlx::query("INSERT INTO msv_ec01_table (name) VALUES ('first')")
		.execute(pool.as_ref())
		.await
		.unwrap();
	sqlx::query("INSERT INTO msv_ec01_table (name) VALUES ('second')")
		.execute(pool.as_ref())
		.await
		.unwrap();

	let rows = sqlx::query("SELECT id FROM msv_ec01_table ORDER BY id")
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 2);
	let id1: i32 = rows[0].get::<i32, _>(0);
	let id2: i32 = rows[1].get::<i32, _>(0);
	assert_eq!(id1, 1, "First auto_increment should be 1");
	assert_eq!(id2, 2, "Second auto_increment should be 2");

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_ec01_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-EC-02: DateTime and TimestampTz both map to 'datetime' in MySQL
/// (Unlike PostgreSQL where they produce different data_type values)
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_ec_02_datetime_vs_timestamptz(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_ec02_table",
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
	apply_migration(&url, ops, "testapp", "0001_ec02").await;

	// Act
	let rows = sqlx::query(
		"SELECT column_name, data_type \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() \
		   AND table_name = 'msv_ec02_table' \
		   AND column_name IN ('local_time', 'utc_time') \
		 ORDER BY column_name",
	)
	.fetch_all(pool.as_ref())
	.await
	.unwrap();

	// Assert
	assert_eq!(rows.len(), 2, "Should have both time columns");

	let local_type: String = rows[0].get::<String, _>(1);
	let utc_type: String = rows[1].get::<String, _>(1);

	// In MySQL, both DateTime and TimestampTz map to 'datetime'
	assert_eq!(local_type, "datetime");
	assert_eq!(utc_type, "datetime");

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_ec02_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-EC-03: Table with 8 different column types, all correctly mapped
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_ec_03_eight_column_types(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_ec03_table",
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
				"col_uuid",
				FieldType::Uuid,
				false,
				false,
				false,
				false,
				None,
			),
			col(
				"col_tstz",
				FieldType::TimestampTz,
				false,
				false,
				false,
				false,
				None,
			),
		],
	);
	apply_migration(&url, ops, "testapp", "0001_ec03").await;

	// Act
	let rows = sqlx::query(
		"SELECT column_name, data_type \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_ec03_table' \
		 ORDER BY ordinal_position",
	)
	.fetch_all(pool.as_ref())
	.await
	.unwrap();

	// Assert
	assert_eq!(rows.len(), 8, "Should have 8 columns");

	let expected: Vec<(&str, &str)> = vec![
		("col_int", "int"),
		("col_bigint", "bigint"),
		("col_varchar", "varchar"),
		("col_bool", "tinyint"),
		("col_double", "double"),
		("col_float", "float"),
		("col_uuid", "char"),
		("col_tstz", "datetime"),
	];

	for (row, (exp_name, exp_type)) in rows.iter().zip(expected.iter()) {
		let name: String = row.get::<String, _>(0);
		let dtype: String = row.get::<String, _>(1);
		assert_eq!(name, *exp_name, "Column name mismatch");
		assert_eq!(
			dtype, *exp_type,
			"Data type mismatch for column {}",
			exp_name
		);
	}

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_ec03_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-EC-04: Boolean default false + Integer default 0
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_ec_04_multiple_defaults(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	let ops = create_table(
		"msv_ec04_table",
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
	apply_migration(&url, ops, "testapp", "0001_ec04").await;

	// Act
	let rows = sqlx::query(
		"SELECT column_name, column_default \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_ec04_table' \
		   AND column_name IN ('flag', 'count') \
		 ORDER BY column_name",
	)
	.fetch_all(pool.as_ref())
	.await
	.unwrap();

	// Assert
	assert_eq!(rows.len(), 2);

	// 'count' comes first alphabetically
	let count_default: Option<String> = rows[0].get::<Option<String>, _>(1);
	assert!(
		count_default.as_ref().map_or(false, |d| d.contains("0")),
		"Integer default should contain '0', got: {:?}",
		count_default
	);

	let flag_default: Option<String> = rows[1].get::<Option<String>, _>(1);
	assert!(
		flag_default
			.as_ref()
			.map_or(false, |d| d.contains("0") || d.contains("false")),
		"Boolean default should contain '0' (MySQL), got: {:?}",
		flag_default
	);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_ec04_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}

/// MSV-EC-05: Nullable PK (primary_key=true, not_null=false)
/// MySQL PK columns are always implicitly NOT NULL
#[rstest]
#[tokio::test]
#[serial(mysql_schema)]
async fn test_msv_ec_05_nullable_pk(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = mysql_container.await;
	// MySQL PRIMARY KEY implicitly enforces NOT NULL,
	// so even if we set not_null=false, PK column should be NOT NULL
	let ops = create_table(
		"msv_ec05_table",
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
	apply_migration(&url, ops, "testapp", "0001_ec05").await;

	// Act
	let row = sqlx::query(
		"SELECT is_nullable \
		 FROM information_schema.columns \
		 WHERE table_schema = DATABASE() AND table_name = 'msv_ec05_table' AND column_name = 'id'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	// MySQL enforces NOT NULL on PRIMARY KEY columns regardless of DDL,
	// so is_nullable should be 'NO'
	let is_nullable: String = row.get::<String, _>(0);
	assert_eq!(
		is_nullable, "NO",
		"MySQL PK columns are always NOT NULL regardless of definition"
	);

	// Cleanup
	sqlx::query("DROP TABLE IF EXISTS msv_ec05_table")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");
}
