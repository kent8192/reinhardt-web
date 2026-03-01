//! Integration tests for schema verification via information_schema
//!
//! Tests apply migrations and then query PostgreSQL's `information_schema`
//! to verify that database-level column properties (type, nullability,
//! defaults, constraints) match the intended schema definition.
//!
//! **Test Coverage:**
//! - Happy path: 12 scenarios (SV-HP-01 to SV-HP-12)
//! - Error path: 5 scenarios (SV-EP-01 to SV-EP-05)
//! - Edge cases: 5 scenarios (SV-EC-01 to SV-EC-05)
//!
//! **Fixtures Used:**
//! - `postgres_table_creator`: PostgreSQL schema management helper

use reinhardt_db::migrations::{ColumnDefinition, FieldType, Operation};
use reinhardt_test::fixtures::PostgresTableCreator;
use reinhardt_test::fixtures::postgres_table_creator;
use rstest::*;
use serial_test::serial;
use sqlx::Row;

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

// ============================================================================
// Happy Path Tests (SV-HP-01 to SV-HP-12)
// ============================================================================

/// SV-HP-01: Integer PK with auto_increment has sequence default
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_hp_01_serial_pk_auto_increment(
	#[future] postgres_table_creator: PostgresTableCreator,
) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_hp01_table",
		vec![col("id", FieldType::Integer, true, false, true, true, None)],
	);
	creator.apply(schema).await.unwrap();

	// Act
	let pool = creator.pool();
	let row = sqlx::query(
		"SELECT column_default, is_nullable, data_type \
		 FROM information_schema.columns \
		 WHERE table_name = 'sv_hp01_table' AND column_name = 'id'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let col_default: Option<String> = row.get("column_default");
	assert!(
		col_default
			.as_ref()
			.map_or(false, |d| d.contains("nextval") || d.contains("identity")),
		"auto_increment PK should have sequence default, got: {:?}",
		col_default
	);
}

/// SV-HP-02: BigInteger PK with auto_increment maps to bigint with sequence
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_hp_02_biginteger_pk_auto_increment(
	#[future] postgres_table_creator: PostgresTableCreator,
) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_hp02_table",
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
	creator.apply(schema).await.unwrap();

	// Act
	let pool = creator.pool();
	let row = sqlx::query(
		"SELECT data_type, column_default \
		 FROM information_schema.columns \
		 WHERE table_name = 'sv_hp02_table' AND column_name = 'id'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let data_type: String = row.get("data_type");
	assert_eq!(data_type, "bigint", "BigInteger should map to bigint");

	let col_default: Option<String> = row.get("column_default");
	assert!(
		col_default
			.as_ref()
			.map_or(false, |d| d.contains("nextval") || d.contains("identity")),
		"auto_increment BigInteger PK should have sequence default, got: {:?}",
		col_default
	);
}

/// SV-HP-03: VarChar(255) NOT NULL maps correctly
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_hp_03_varchar_255_not_null(
	#[future] postgres_table_creator: PostgresTableCreator,
) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_hp03_table",
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
	creator.apply(schema).await.unwrap();

	// Act
	let pool = creator.pool();
	let row = sqlx::query(
		"SELECT data_type, character_maximum_length, is_nullable \
		 FROM information_schema.columns \
		 WHERE table_name = 'sv_hp03_table' AND column_name = 'name'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let data_type: String = row.get("data_type");
	assert_eq!(data_type, "character varying");

	let max_length: Option<i32> = row.get("character_maximum_length");
	assert_eq!(
		max_length,
		Some(255),
		"VarChar(255) should have max length 255"
	);

	let is_nullable: String = row.get("is_nullable");
	assert_eq!(
		is_nullable, "NO",
		"NOT NULL column should have is_nullable='NO'"
	);
}

/// SV-HP-04: VarChar(100) nullable
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_hp_04_varchar_nullable(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_hp04_table",
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
	creator.apply(schema).await.unwrap();

	// Act
	let pool = creator.pool();
	let row = sqlx::query(
		"SELECT is_nullable \
		 FROM information_schema.columns \
		 WHERE table_name = 'sv_hp04_table' AND column_name = 'description'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let is_nullable: String = row.get("is_nullable");
	assert_eq!(
		is_nullable, "YES",
		"Nullable column should have is_nullable='YES'"
	);
}

/// SV-HP-05: Boolean with default "false"
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_hp_05_boolean_default_false(
	#[future] postgres_table_creator: PostgresTableCreator,
) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_hp05_table",
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
	creator.apply(schema).await.unwrap();

	// Act
	let pool = creator.pool();
	let row = sqlx::query(
		"SELECT data_type, column_default \
		 FROM information_schema.columns \
		 WHERE table_name = 'sv_hp05_table' AND column_name = 'is_active'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let data_type: String = row.get("data_type");
	assert_eq!(data_type, "boolean");

	let col_default: Option<String> = row.get("column_default");
	assert!(
		col_default.as_ref().map_or(false, |d| d.contains("false")),
		"Boolean default should contain 'false', got: {:?}",
		col_default
	);
}

/// SV-HP-06: TimestampTz maps to 'timestamp with time zone'
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_hp_06_timestamp_tz(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_hp06_table",
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
	creator.apply(schema).await.unwrap();

	// Act
	let pool = creator.pool();
	let row = sqlx::query(
		"SELECT data_type \
		 FROM information_schema.columns \
		 WHERE table_name = 'sv_hp06_table' AND column_name = 'created_at'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let data_type: String = row.get("data_type");
	assert_eq!(data_type, "timestamp with time zone");
}

/// SV-HP-07: DateTime maps to 'timestamp without time zone'
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_hp_07_datetime_no_tz(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_hp07_table",
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
	creator.apply(schema).await.unwrap();

	// Act
	let pool = creator.pool();
	let row = sqlx::query(
		"SELECT data_type \
		 FROM information_schema.columns \
		 WHERE table_name = 'sv_hp07_table' AND column_name = 'event_time'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let data_type: String = row.get("data_type");
	assert_eq!(data_type, "timestamp without time zone");
}

/// SV-HP-08: Integer NOT NULL
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_hp_08_integer_not_null(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_hp08_table",
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
	creator.apply(schema).await.unwrap();

	// Act
	let pool = creator.pool();
	let row = sqlx::query(
		"SELECT data_type, is_nullable \
		 FROM information_schema.columns \
		 WHERE table_name = 'sv_hp08_table' AND column_name = 'quantity'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let data_type: String = row.get("data_type");
	assert_eq!(data_type, "integer");

	let is_nullable: String = row.get("is_nullable");
	assert_eq!(is_nullable, "NO");
}

/// SV-HP-09: Uuid maps to 'uuid'
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_hp_09_uuid(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_hp09_table",
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
	creator.apply(schema).await.unwrap();

	// Act
	let pool = creator.pool();
	let row = sqlx::query(
		"SELECT data_type \
		 FROM information_schema.columns \
		 WHERE table_name = 'sv_hp09_table' AND column_name = 'external_id'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let data_type: String = row.get("data_type");
	assert_eq!(data_type, "uuid");
}

/// SV-HP-10: Double maps to 'double precision'
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_hp_10_double(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_hp10_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col("score", FieldType::Double, false, false, false, false, None),
		],
	);
	creator.apply(schema).await.unwrap();

	// Act
	let pool = creator.pool();
	let row = sqlx::query(
		"SELECT data_type \
		 FROM information_schema.columns \
		 WHERE table_name = 'sv_hp10_table' AND column_name = 'score'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let data_type: String = row.get("data_type");
	assert_eq!(data_type, "double precision");
}

/// SV-HP-11: Float maps to 'real'
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_hp_11_float(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_hp11_table",
		vec![
			col("id", FieldType::Integer, true, false, true, true, None),
			col("weight", FieldType::Float, false, false, false, false, None),
		],
	);
	creator.apply(schema).await.unwrap();

	// Act
	let pool = creator.pool();
	let row = sqlx::query(
		"SELECT data_type \
		 FROM information_schema.columns \
		 WHERE table_name = 'sv_hp11_table' AND column_name = 'weight'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let data_type: String = row.get("data_type");
	assert_eq!(data_type, "real");
}

/// SV-HP-12: VarChar(255) unique has UNIQUE constraint
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_hp_12_varchar_unique(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_hp12_table",
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
	creator.apply(schema).await.unwrap();

	// Act
	let pool = creator.pool();
	let row = sqlx::query(
		"SELECT tc.constraint_type \
		 FROM information_schema.table_constraints tc \
		 JOIN information_schema.constraint_column_usage ccu \
		   ON tc.constraint_name = ccu.constraint_name \
		 WHERE tc.table_name = 'sv_hp12_table' \
		   AND ccu.column_name = 'email' \
		   AND tc.constraint_type = 'UNIQUE'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	let constraint_type: String = row.get("constraint_type");
	assert_eq!(constraint_type, "UNIQUE");
}

// ============================================================================
// Error Path Tests (SV-EP-01 to SV-EP-05)
// ============================================================================

/// SV-EP-01: INSERT NULL into NOT NULL column fails
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_ep_01_null_into_not_null(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_ep01_table",
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
	creator.apply(schema).await.unwrap();

	// Act
	let pool = creator.pool();
	let result = sqlx::query("INSERT INTO sv_ep01_table (name) VALUES (NULL)")
		.execute(pool.as_ref())
		.await;

	// Assert
	assert!(
		result.is_err(),
		"INSERT NULL into NOT NULL column should fail"
	);
	let err = result.unwrap_err().to_string();
	assert!(
		err.contains("null") || err.contains("NOT NULL") || err.contains("not-null"),
		"Error should indicate NOT NULL violation, got: {}",
		err
	);
}

/// SV-EP-02: INSERT duplicate into unique column fails
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_ep_02_duplicate_unique(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_ep02_table",
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
	creator.apply(schema).await.unwrap();

	let pool = creator.pool();
	sqlx::query("INSERT INTO sv_ep02_table (email) VALUES ('test@example.com')")
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Act
	let result = sqlx::query("INSERT INTO sv_ep02_table (email) VALUES ('test@example.com')")
		.execute(pool.as_ref())
		.await;

	// Assert
	assert!(
		result.is_err(),
		"INSERT duplicate into unique column should fail"
	);
	let err = result.unwrap_err().to_string();
	assert!(
		err.contains("unique") || err.contains("duplicate"),
		"Error should indicate unique violation, got: {}",
		err
	);
}

/// SV-EP-03: INSERT string into integer column fails
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_ep_03_string_into_integer(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_ep03_table",
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
	creator.apply(schema).await.unwrap();

	// Act
	let pool = creator.pool();
	let result = sqlx::query("INSERT INTO sv_ep03_table (quantity) VALUES ('not_a_number')")
		.execute(pool.as_ref())
		.await;

	// Assert
	assert!(
		result.is_err(),
		"INSERT string into integer column should fail"
	);
	let err = result.unwrap_err().to_string();
	assert!(
		err.contains("invalid input syntax") || err.contains("type"),
		"Error should indicate type mismatch, got: {}",
		err
	);
}

/// SV-EP-04: INSERT 256 chars into VarChar(255) fails
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_ep_04_exceeds_varchar_length(
	#[future] postgres_table_creator: PostgresTableCreator,
) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_ep04_table",
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
	creator.apply(schema).await.unwrap();

	// Act
	let pool = creator.pool();
	let long_value = "x".repeat(256);
	let query_str = format!(
		"INSERT INTO sv_ep04_table (short_text) VALUES ('{}')",
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
		err.contains("value too long") || err.contains("varying(255)"),
		"Error should indicate length violation, got: {}",
		err
	);
}

/// SV-EP-05: INSERT without required NOT NULL column (no default) fails
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_ep_05_missing_not_null_column(
	#[future] postgres_table_creator: PostgresTableCreator,
) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_ep05_table",
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
	creator.apply(schema).await.unwrap();

	// Act: insert only optional_field, omitting required_field
	let pool = creator.pool();
	let result = sqlx::query("INSERT INTO sv_ep05_table (optional_field) VALUES ('some_value')")
		.execute(pool.as_ref())
		.await;

	// Assert
	assert!(
		result.is_err(),
		"INSERT without required NOT NULL column should fail"
	);
	let err = result.unwrap_err().to_string();
	assert!(
		err.contains("null") || err.contains("NOT NULL") || err.contains("not-null"),
		"Error should indicate NOT NULL violation, got: {}",
		err
	);
}

// ============================================================================
// Edge Case Tests (SV-EC-01 to SV-EC-05)
// ============================================================================

/// SV-EC-01: Serial PK has an associated sequence (pg_get_serial_sequence)
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_ec_01_serial_pk_sequence(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_ec01_table",
		vec![col("id", FieldType::Integer, true, false, true, true, None)],
	);
	creator.apply(schema).await.unwrap();

	// Act
	let pool = creator.pool();
	let row = sqlx::query("SELECT pg_get_serial_sequence('sv_ec01_table', 'id') AS seq")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	let seq: Option<String> = row.get("seq");
	assert!(
		seq.is_some(),
		"Serial PK should have an associated sequence, got None"
	);
}

/// SV-EC-02: DateTime and TimestampTz in same table have different data_type values
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_ec_02_datetime_vs_timestamptz(
	#[future] postgres_table_creator: PostgresTableCreator,
) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_ec02_table",
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
	creator.apply(schema).await.unwrap();

	// Act
	let pool = creator.pool();
	let rows = sqlx::query(
		"SELECT column_name, data_type \
		 FROM information_schema.columns \
		 WHERE table_name = 'sv_ec02_table' \
		   AND column_name IN ('local_time', 'utc_time') \
		 ORDER BY column_name",
	)
	.fetch_all(pool.as_ref())
	.await
	.unwrap();

	// Assert
	assert_eq!(rows.len(), 2, "Should have both time columns");

	let local_type: String = rows[0].get("data_type");
	let utc_type: String = rows[1].get("data_type");

	assert_eq!(local_type, "timestamp without time zone");
	assert_eq!(utc_type, "timestamp with time zone");
	assert_ne!(
		local_type, utc_type,
		"DateTime and TimestampTz should differ"
	);
}

/// SV-EC-03: Table with 8 different column types, all correctly mapped
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_ec_03_eight_column_types(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_ec03_table",
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
	creator.apply(schema).await.unwrap();

	// Act
	let pool = creator.pool();
	let rows = sqlx::query(
		"SELECT column_name, data_type \
		 FROM information_schema.columns \
		 WHERE table_name = 'sv_ec03_table' \
		 ORDER BY ordinal_position",
	)
	.fetch_all(pool.as_ref())
	.await
	.unwrap();

	// Assert
	assert_eq!(rows.len(), 8, "Should have 8 columns");

	let expected: Vec<(&str, &str)> = vec![
		("col_int", "integer"),
		("col_bigint", "bigint"),
		("col_varchar", "character varying"),
		("col_bool", "boolean"),
		("col_double", "double precision"),
		("col_float", "real"),
		("col_uuid", "uuid"),
		("col_tstz", "timestamp with time zone"),
	];

	for (row, (exp_name, exp_type)) in rows.iter().zip(expected.iter()) {
		let name: String = row.get("column_name");
		let dtype: String = row.get("data_type");
		assert_eq!(name, *exp_name, "Column name mismatch");
		assert_eq!(
			dtype, *exp_type,
			"Data type mismatch for column {}",
			exp_name
		);
	}
}

/// SV-EC-04: Boolean default false + Integer default 0
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_ec_04_multiple_defaults(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	let schema = create_table(
		"sv_ec04_table",
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
	creator.apply(schema).await.unwrap();

	// Act
	let pool = creator.pool();
	let rows = sqlx::query(
		"SELECT column_name, column_default \
		 FROM information_schema.columns \
		 WHERE table_name = 'sv_ec04_table' \
		   AND column_name IN ('flag', 'count') \
		 ORDER BY column_name",
	)
	.fetch_all(pool.as_ref())
	.await
	.unwrap();

	// Assert
	assert_eq!(rows.len(), 2);

	// 'count' comes first alphabetically
	let count_default: Option<String> = rows[0].get("column_default");
	assert!(
		count_default.as_ref().map_or(false, |d| d.contains("0")),
		"Integer default should contain '0', got: {:?}",
		count_default
	);

	let flag_default: Option<String> = rows[1].get("column_default");
	assert!(
		flag_default.as_ref().map_or(false, |d| d.contains("false")),
		"Boolean default should contain 'false', got: {:?}",
		flag_default
	);
}

/// SV-EC-05: Nullable PK (primary_key=true, not_null=false)
#[rstest]
#[tokio::test]
#[serial(schema_verify)]
async fn test_sv_ec_05_nullable_pk(#[future] postgres_table_creator: PostgresTableCreator) {
	// Arrange
	let mut creator = postgres_table_creator.await;
	// PostgreSQL PRIMARY KEY implicitly enforces NOT NULL,
	// so even if we set not_null=false, PK column should be NOT NULL
	let schema = create_table(
		"sv_ec05_table",
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
	creator.apply(schema).await.unwrap();

	// Act
	let pool = creator.pool();
	let row = sqlx::query(
		"SELECT is_nullable \
		 FROM information_schema.columns \
		 WHERE table_name = 'sv_ec05_table' AND column_name = 'id'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Assert
	// PostgreSQL enforces NOT NULL on PRIMARY KEY columns regardless of DDL,
	// so is_nullable should be 'NO'
	let is_nullable: String = row.get("is_nullable");
	assert_eq!(
		is_nullable, "NO",
		"PostgreSQL PK columns are always NOT NULL regardless of definition"
	);
}
