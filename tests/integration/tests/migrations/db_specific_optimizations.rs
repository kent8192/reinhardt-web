//! Database-Specific Optimization Tests
//!
//! Tests that verify database-specific optimization features are correctly utilized
//! by the migrations system. Each database has unique features that can improve
//! performance, reduce locking, or enable specialized functionality.
//!
//! **Test Coverage:**
//! - PostgreSQL: CONCURRENTLY, DEFERRABLE, partial indexes, expression indexes, GiST, GIN, EXCLUDE
//! - MySQL: ALGORITHM=INSTANT/INPLACE, LOCK=NONE, FULLTEXT, SPATIAL, PARTITION
//! - SQLite: Table recreation workarounds, WITHOUT ROWID
//! - CockroachDB: INTERLEAVE, AS OF SYSTEM TIME
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container
//! - mysql_container: MySQL database container
//! - cockroachdb_container: CockroachDB container (when available)
//!
//! **Note**: Some tests may be marked as `#[ignore]` if the feature is not yet
//! implemented in reinhardt-db migrations. These serve as documentation for
//! future implementation.

use reinhardt_backends::DatabaseConnection;
use reinhardt_migrations::{
	executor::DatabaseMigrationExecutor, ColumnDefinition, FieldType, Migration, Operation,
};
use reinhardt_test::fixtures::{mysql_container, postgres_container};
use rstest::*;
use sqlx::{MySqlPool, PgPool};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Test Helper Functions
// ============================================================================

fn leak_str(s: impl Into<String>) -> &'static str {
	Box::leak(s.into().into_boxed_str())
}

/// Create a simple migration for testing
fn create_test_migration(
	app: &'static str,
	name: &'static str,
	operations: Vec<Operation>,
) -> Migration {
	Migration {
		app_label: app,
		name,
		operations,
		dependencies: vec![],
		replaces: vec![],
		atomic: true,
		initial: None,
		state_only: false,
		database_only: false,
	}
}

/// Create a basic column definition
fn create_basic_column(name: &'static str, type_def: FieldType) -> ColumnDefinition {
	ColumnDefinition {
		name,
		type_definition: type_def,
		not_null: false,
		unique: false,
		primary_key: false,
		auto_increment: false,
		default: None,
	}
}

/// Create an auto-increment primary key column
fn create_auto_pk_column(name: &'static str, type_def: FieldType) -> ColumnDefinition {
	ColumnDefinition {
		name,
		type_definition: type_def,
		not_null: true,
		unique: false,
		primary_key: true,
		auto_increment: true,
		default: None,
	}
}

// ============================================================================
// PostgreSQL-Specific Optimization Tests
// ============================================================================

/// Test CREATE INDEX CONCURRENTLY (non-blocking index creation)
///
/// **Test Intent**: Verify that CONCURRENTLY option creates index without locking table
///
/// **PostgreSQL Feature**: CREATE INDEX CONCURRENTLY allows index creation without
/// blocking writes to the table. Critical for production systems with large tables.
///
/// **Note**: Currently marked as ignore - waiting for CONCURRENTLY support in migrations
#[rstest]
#[ignore = "CONCURRENTLY support not yet implemented in reinhardt-db migrations"]
#[tokio::test]
async fn test_postgres_create_index_concurrently(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let connection = DatabaseConnection::connect_postgres(&url)
		.await
		.expect("Failed to connect to PostgreSQL");

	let mut executor = DatabaseMigrationExecutor::new(connection.clone());

	// Create table first
	let create_table = create_test_migration(
		"testapp",
		"0001_create_users",
		vec![Operation::CreateTable {
			name: leak_str("users"),
			columns: vec![
				create_auto_pk_column("id", FieldType::Integer),
				create_basic_column("email", FieldType::VarChar(255)),
			],
			constraints: vec![],
		}],
	);

	executor
		.apply_migrations(&[create_table])
		.await
		.expect("Failed to create table");

	// TODO: Add CreateIndexConcurrently operation
	// let create_index = create_test_migration(
	// 	"testapp",
	// 	"0002_create_email_index",
	// 	vec![Operation::CreateIndexConcurrently {
	// 		table: leak_str("users"),
	// 		name: leak_str("idx_users_email_concurrent"),
	// 		columns: vec![leak_str("email")],
	// 		unique: false,
	// 	}],
	// );
	//
	// executor.apply_migrations(&[create_index])
	// 	.await
	// 	.expect("Failed to create concurrent index");

	// Verify index was created
	// let index_exists: bool = sqlx::query_scalar(
	// 	"SELECT EXISTS(SELECT 1 FROM pg_indexes WHERE indexname = $1)"
	// )
	// .bind("idx_users_email_concurrent")
	// .fetch_one(pool.as_ref())
	// .await
	// .expect("Failed to check index");
	//
	// assert!(index_exists, "Concurrent index should exist");
}

/// Test DEFERRABLE INITIALLY DEFERRED constraints
///
/// **Test Intent**: Verify that DEFERRABLE constraints can be deferred to transaction end
///
/// **PostgreSQL Feature**: DEFERRABLE INITIALLY DEFERRED allows constraint checking
/// to be deferred until COMMIT, useful for circular foreign key relationships.
///
/// **Note**: Currently marked as ignore - waiting for DEFERRABLE support in migrations
#[rstest]
#[ignore = "DEFERRABLE support not yet implemented in reinhardt-db migrations"]
#[tokio::test]
async fn test_postgres_deferrable_constraint(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	// TODO: Add DEFERRABLE option to Constraint::ForeignKey
	// This would allow:
	// - Circular FK relationships
	// - Bulk data loading without constraint violations
	// - More flexible transaction handling

	// Example usage (when implemented):
	// Constraint::ForeignKey {
	// 	name: leak_str("fk_deferrable"),
	// 	columns: vec![leak_str("ref_id")],
	// 	to_table: leak_str("other_table"),
	// 	to_columns: vec![leak_str("id")],
	// 	on_delete: ForeignKeyAction::NoAction,
	// 	on_update: ForeignKeyAction::NoAction,
	// 	deferrable: Some(DeferrableOption::InitiallyDeferred),
	// }
}

/// Test partial indexes (indexes with WHERE clause)
///
/// **Test Intent**: Verify that partial indexes can be created
///
/// **PostgreSQL Feature**: Partial indexes only index rows matching a WHERE condition,
/// reducing index size and improving performance for filtered queries.
///
/// **Note**: Currently marked as ignore - waiting for partial index support
#[rstest]
#[ignore = "Partial index support not yet implemented in reinhardt-db migrations"]
#[tokio::test]
async fn test_postgres_partial_index(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	// TODO: Add where_clause parameter to CreateIndex operation
	// Example:
	// Operation::CreateIndex {
	// 	table: leak_str("orders"),
	// 	name: leak_str("idx_active_orders"),
	// 	columns: vec![leak_str("status")],
	// 	unique: false,
	// 	where_clause: Some("status = 'active'"), // Only index active orders
	// }
	//
	// Expected SQL: CREATE INDEX idx_active_orders ON orders(status) WHERE status = 'active'
}

/// Test expression indexes (indexes on computed expressions)
///
/// **Test Intent**: Verify that expression indexes can be created
///
/// **PostgreSQL Feature**: Expression indexes allow indexing computed values like
/// LOWER(email) for case-insensitive searches.
///
/// **Note**: Currently marked as ignore - waiting for expression index support
#[rstest]
#[ignore = "Expression index support not yet implemented in reinhardt-db migrations"]
#[tokio::test]
async fn test_postgres_expression_index(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	// TODO: Add expression parameter to CreateIndex operation
	// Example:
	// Operation::CreateExpressionIndex {
	// 	table: leak_str("users"),
	// 	name: leak_str("idx_email_lower"),
	// 	expression: leak_str("LOWER(email)"),
	// 	unique: false,
	// }
	//
	// Expected SQL: CREATE INDEX idx_email_lower ON users(LOWER(email))
}

/// Test GiST index creation
///
/// **Test Intent**: Verify that GiST indexes can be created
///
/// **PostgreSQL Feature**: GiST (Generalized Search Tree) indexes support complex data types
/// like geometric types, full-text search, and custom types.
///
/// **Note**: Currently marked as ignore - waiting for GiST index support
#[rstest]
#[ignore = "GiST index support not yet implemented in reinhardt-db migrations"]
#[tokio::test]
async fn test_postgres_gist_index(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	// TODO: Add index_type parameter to CreateIndex operation
	// Example:
	// Operation::CreateIndex {
	// 	table: leak_str("locations"),
	// 	name: leak_str("idx_geo_gist"),
	// 	columns: vec![leak_str("coordinates")],
	// 	unique: false,
	// 	index_type: Some(IndexType::GiST),
	// }
	//
	// Expected SQL: CREATE INDEX idx_geo_gist ON locations USING GIST(coordinates)
}

/// Test GIN index creation (for full-text search)
///
/// **Test Intent**: Verify that GIN indexes can be created
///
/// **PostgreSQL Feature**: GIN (Generalized Inverted Index) is optimized for indexing
/// array values, JSONB, and full-text search.
///
/// **Note**: Currently marked as ignore - waiting for GIN index support
#[rstest]
#[ignore = "GIN index support not yet implemented in reinhardt-db migrations"]
#[tokio::test]
async fn test_postgres_gin_index(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	// TODO: Add index_type parameter to CreateIndex operation
	// Example:
	// Operation::CreateIndex {
	// 	table: leak_str("articles"),
	// 	name: leak_str("idx_content_gin"),
	// 	columns: vec![leak_str("content_tsv")],
	// 	unique: false,
	// 	index_type: Some(IndexType::GIN),
	// }
	//
	// Expected SQL: CREATE INDEX idx_content_gin ON articles USING GIN(content_tsv)
}

/// Test EXCLUDE constraint
///
/// **Test Intent**: Verify that EXCLUDE constraints can be created
///
/// **PostgreSQL Feature**: EXCLUDE constraints prevent overlapping ranges or conflicting
/// values using GiST indexes. Useful for scheduling, reservations, etc.
///
/// **Note**: Currently marked as ignore - waiting for EXCLUDE support
#[rstest]
#[ignore = "EXCLUDE constraint support not yet implemented in reinhardt-db migrations"]
#[tokio::test]
async fn test_postgres_exclude_constraint(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	// TODO: Add Constraint::Exclude variant
	// Example:
	// Constraint::Exclude {
	// 	name: leak_str("exclude_overlapping_dates"),
	// 	elements: vec![
	// 		("room_id", "="),
	// 		("daterange(start_date, end_date)", "&&"),
	// 	],
	// }
	//
	// Expected SQL:
	// ALTER TABLE bookings ADD CONSTRAINT exclude_overlapping_dates
	// EXCLUDE USING GIST (room_id WITH =, daterange(start_date, end_date) WITH &&)
}

/// Test trigram similarity index (pg_trgm extension)
///
/// **Test Intent**: Verify that trigram indexes can be created for fuzzy text search
///
/// **PostgreSQL Feature**: pg_trgm extension enables fuzzy string matching using
/// trigram similarity. Useful for autocomplete, typo-tolerant search.
///
/// **Note**: Currently marked as ignore - waiting for extension + GIN index support
#[rstest]
#[ignore = "pg_trgm extension support not yet implemented in reinhardt-db migrations"]
#[tokio::test]
async fn test_postgres_trigram_index(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	// TODO: Add RunSQL support for CREATE EXTENSION + GIN operator class support
	// Example:
	// 1. Operation::RunSQL { sql: "CREATE EXTENSION IF NOT EXISTS pg_trgm", ... }
	// 2. Operation::CreateIndex {
	// 	table: leak_str("products"),
	// 	name: leak_str("idx_name_trgm"),
	// 	columns: vec![leak_str("name")],
	// 	unique: false,
	// 	index_type: Some(IndexType::GIN),
	// 	operator_class: Some("gin_trgm_ops"),
	// }
	//
	// Expected SQL: CREATE INDEX idx_name_trgm ON products USING GIN(name gin_trgm_ops)
}

// ============================================================================
// MySQL-Specific Optimization Tests
// ============================================================================

/// Test ALGORITHM=INSTANT (non-blocking ALTER TABLE in MySQL 8.0+)
///
/// **Test Intent**: Verify that ALGORITHM=INSTANT is used for compatible operations
///
/// **MySQL Feature**: ALGORITHM=INSTANT allows instant schema changes without table copy
/// for operations like adding columns with defaults, renaming columns, etc.
///
/// **Note**: Currently marked as ignore - waiting for ALGORITHM support
#[rstest]
#[ignore = "ALGORITHM=INSTANT support not yet implemented in reinhardt-db migrations"]
#[tokio::test]
async fn test_mysql_algorithm_instant(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<MySqlPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = mysql_container.await;

	// TODO: Add algorithm parameter to ALTER TABLE operations
	// Example:
	// Operation::AddColumn {
	// 	table: leak_str("users"),
	// 	column: create_basic_column("middle_name", FieldType::VarChar(100)),
	// 	algorithm: Some(AlgorithmType::Instant),
	// }
	//
	// Expected SQL: ALTER TABLE users ADD COLUMN middle_name VARCHAR(100), ALGORITHM=INSTANT
}

/// Test ALGORITHM=INPLACE (in-place schema changes)
///
/// **Test Intent**: Verify that ALGORITHM=INPLACE is used for compatible operations
///
/// **MySQL Feature**: ALGORITHM=INPLACE modifies table structure without full table copy,
/// allowing concurrent DML operations.
#[rstest]
#[ignore = "ALGORITHM=INPLACE support not yet implemented in reinhardt-db migrations"]
#[tokio::test]
async fn test_mysql_algorithm_inplace(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<MySqlPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = mysql_container.await;

	// TODO: Add algorithm parameter to ALTER TABLE operations
	// Example:
	// Operation::CreateIndex {
	// 	table: leak_str("products"),
	// 	name: leak_str("idx_category"),
	// 	columns: vec![leak_str("category")],
	// 	unique: false,
	// 	algorithm: Some(AlgorithmType::Inplace),
	// }
	//
	// Expected SQL: CREATE INDEX idx_category ON products(category) ALGORITHM=INPLACE
}

/// Test LOCK=NONE (lock-free operations)
///
/// **Test Intent**: Verify that LOCK=NONE is used to allow concurrent writes
///
/// **MySQL Feature**: LOCK=NONE allows concurrent INSERT, UPDATE, DELETE during
/// schema changes (when compatible with operation).
#[rstest]
#[ignore = "LOCK=NONE support not yet implemented in reinhardt-db migrations"]
#[tokio::test]
async fn test_mysql_lock_none(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<MySqlPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = mysql_container.await;

	// TODO: Add lock_type parameter to ALTER TABLE operations
	// Example:
	// Operation::AddColumn {
	// 	table: leak_str("orders"),
	// 	column: create_basic_column("tracking_number", FieldType::VarChar(50)),
	// 	lock_type: Some(LockType::None),
	// }
	//
	// Expected SQL: ALTER TABLE orders ADD COLUMN tracking_number VARCHAR(50), LOCK=NONE
}

/// Test FULLTEXT INDEX creation
///
/// **Test Intent**: Verify that MySQL FULLTEXT indexes can be created
///
/// **MySQL Feature**: FULLTEXT indexes enable natural language full-text search
#[rstest]
#[ignore = "FULLTEXT index support not yet implemented in reinhardt-db migrations"]
#[tokio::test]
async fn test_mysql_fulltext_index(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<MySqlPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = mysql_container.await;

	// TODO: Add index_type parameter to CreateIndex operation
	// Example:
	// Operation::CreateIndex {
	// 	table: leak_str("articles"),
	// 	name: leak_str("ft_content"),
	// 	columns: vec![leak_str("title"), leak_str("body")],
	// 	unique: false,
	// 	index_type: Some(IndexType::FullText),
	// }
	//
	// Expected SQL: CREATE FULLTEXT INDEX ft_content ON articles(title, body)
}

/// Test SPATIAL INDEX creation
///
/// **Test Intent**: Verify that MySQL SPATIAL indexes can be created
///
/// **MySQL Feature**: SPATIAL indexes optimize geometric queries (GIS data)
#[rstest]
#[ignore = "SPATIAL index support not yet implemented in reinhardt-db migrations"]
#[tokio::test]
async fn test_mysql_spatial_index(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<MySqlPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = mysql_container.await;

	// TODO: Add index_type parameter to CreateIndex operation + GEOMETRY field type
	// Example:
	// Operation::CreateIndex {
	// 	table: leak_str("locations"),
	// 	name: leak_str("idx_coordinates"),
	// 	columns: vec![leak_str("coordinates")],
	// 	unique: false,
	// 	index_type: Some(IndexType::Spatial),
	// }
	//
	// Expected SQL: CREATE SPATIAL INDEX idx_coordinates ON locations(coordinates)
}

/// Test table partitioning by RANGE
///
/// **Test Intent**: Verify that PARTITION BY RANGE can be created
///
/// **MySQL Feature**: RANGE partitioning splits table data by column value ranges,
/// improving query performance for time-series data.
#[rstest]
#[ignore = "Table partitioning support not yet implemented in reinhardt-db migrations"]
#[tokio::test]
async fn test_mysql_partition_by_range(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<MySqlPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = mysql_container.await;

	// TODO: Add partition parameter to CreateTable operation
	// Example:
	// Operation::CreateTable {
	// 	name: leak_str("sales"),
	// 	columns: vec![
	// 		create_auto_pk_column("id", FieldType::Integer),
	// 		create_basic_column("sale_date", FieldType::Date),
	// 	],
	// 	constraints: vec![],
	// 	composite_primary_key: None,
	// 	partition: Some(PartitionSpec::Range {
	// 		column: leak_str("sale_date"),
	// 		partitions: vec![
	// 			("p2023", "2024-01-01"),
	// 			("p2024", "2025-01-01"),
	// 		],
	// 	}),
	// }
}

/// Test table partitioning by HASH
///
/// **Test Intent**: Verify that PARTITION BY HASH can be created
///
/// **MySQL Feature**: HASH partitioning distributes rows evenly across partitions
/// using a hash function on a column.
#[rstest]
#[ignore = "Table partitioning support not yet implemented in reinhardt-db migrations"]
#[tokio::test]
async fn test_mysql_partition_by_hash(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<MySqlPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = mysql_container.await;

	// TODO: Add partition parameter to CreateTable operation
	// Example:
	// Operation::CreateTable {
	// 	name: leak_str("users"),
	// 	columns: vec![create_auto_pk_column("id", FieldType::Integer)],
	// 	constraints: vec![],
	// 	composite_primary_key: None,
	// 	partition: Some(PartitionSpec::Hash {
	// 		column: leak_str("id"),
	// 		num_partitions: 4,
	// 	}),
	// }
}

/// Test AUTO_INCREMENT initial value setting
///
/// **Test Intent**: Verify that AUTO_INCREMENT can start from a specific value
///
/// **MySQL Feature**: Setting AUTO_INCREMENT initial value is useful when merging data
/// or coordinating with external systems.
#[rstest]
#[tokio::test]
async fn test_mysql_auto_increment_initial_value(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<MySqlPool>, u16, String),
) {
	let (_container, pool, _port, url) = mysql_container.await;

	let connection = DatabaseConnection::connect_mysql(&url)
		.await
		.expect("Failed to connect to MySQL");

	let mut executor = DatabaseMigrationExecutor::new(connection.clone());

	// Create table with AUTO_INCREMENT
	let migration = create_test_migration(
		"testapp",
		"0001_create_users",
		vec![Operation::CreateTable {
			name: leak_str("users"),
			columns: vec![create_auto_pk_column("id", FieldType::Integer)],
			constraints: vec![],
		}],
	);

	executor
		.apply_migrations(&[migration])
		.await
		.expect("Failed to create table");

	// TODO: Add Operation::SetAutoIncrementValue
	// For now, we can use RunSQL as a workaround
	let set_auto_increment = create_test_migration(
		"testapp",
		"0002_set_auto_increment",
		vec![Operation::RunSQL {
			sql: leak_str("ALTER TABLE users AUTO_INCREMENT = 1000"),
			reverse_sql: None,
		}],
	);

	executor
		.apply_migrations(&[set_auto_increment])
		.await
		.expect("Failed to set AUTO_INCREMENT");

	// Verify AUTO_INCREMENT value
	let auto_increment: Option<u64> = sqlx::query_scalar(
		"SELECT AUTO_INCREMENT FROM information_schema.tables
		 WHERE table_schema = DATABASE() AND table_name = 'users'",
	)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to get AUTO_INCREMENT value");

	assert_eq!(auto_increment, Some(1000), "AUTO_INCREMENT should be 1000");
}

// ============================================================================
// SQLite-Specific Tests
// ============================================================================

/// Test ALTER TABLE limitation workaround (table recreation)
///
/// **Test Intent**: Verify that SQLite's ALTER TABLE limitations are handled
/// by recreating the table.
///
/// **SQLite Limitation**: SQLite has very limited ALTER TABLE support (can't drop columns,
/// change column types, etc.). The workaround is to:
/// 1. CREATE TABLE temp_table (with new schema)
/// 2. INSERT INTO temp_table SELECT * FROM old_table
/// 3. DROP TABLE old_table
/// 4. ALTER TABLE temp_table RENAME TO old_table
///
/// **Note**: This is handled automatically by the migration system for SQLite
#[rstest]
#[ignore = "SQLite table recreation workaround test - manual verification needed"]
#[tokio::test]
async fn test_sqlite_alter_table_via_recreation() {
	// TODO: This test requires SQLite-specific logic in migrations
	// The migration executor should detect SQLite and automatically use
	// the table recreation pattern for operations like:
	// - DropColumn
	// - AlterColumn (type change)
	// - RenameColumn (in old SQLite versions)

	// Example expected behavior:
	// User writes: Operation::DropColumn { table: "users", column: "email" }
	// SQLite executor generates:
	// 1. CREATE TABLE users_new (id INTEGER PRIMARY KEY)
	// 2. INSERT INTO users_new SELECT id FROM users
	// 3. DROP TABLE users
	// 4. ALTER TABLE users_new RENAME TO users
}

/// Test WITHOUT ROWID optimization
///
/// **Test Intent**: Verify that WITHOUT ROWID tables can be created
///
/// **SQLite Feature**: WITHOUT ROWID tables store data in the PRIMARY KEY index,
/// reducing storage and improving performance for tables with composite primary keys.
#[rstest]
#[ignore = "WITHOUT ROWID support not yet implemented in reinhardt-db migrations"]
#[tokio::test]
async fn test_sqlite_without_rowid() {
	// TODO: Add without_rowid parameter to CreateTable operation
	// Example:
	// Operation::CreateTable {
	// 	name: leak_str("settings"),
	// 	columns: vec![
	// 		create_basic_column("key", FieldType::VarChar(50)),
	// 		create_basic_column("value", FieldType::Text),
	// 	],
	// 	constraints: vec![],
	// 	composite_primary_key: Some(vec![leak_str("key")]),
	// 	without_rowid: Some(true), // SQLite-specific
	// }
	//
	// Expected SQL: CREATE TABLE settings (key VARCHAR(50), value TEXT, PRIMARY KEY(key)) WITHOUT ROWID
}

// ============================================================================
// CockroachDB-Specific Tests
// ============================================================================

/// Test interleaved tables (INTERLEAVE IN PARENT)
///
/// **Test Intent**: Verify that interleaved tables can be created
///
/// **CockroachDB Feature**: INTERLEAVE IN PARENT co-locates child table rows with
/// parent table rows, improving join performance for hierarchical data.
#[rstest]
#[ignore = "INTERLEAVE support not yet implemented in reinhardt-db migrations"]
#[tokio::test]
async fn test_cockroachdb_interleave_table() {
	// TODO: Add interleave parameter to CreateTable operation
	// Example:
	// Operation::CreateTable {
	// 	name: leak_str("orders"),
	// 	columns: vec![
	// 		create_basic_column("user_id", FieldType::Integer),
	// 		create_auto_pk_column("order_id", FieldType::Integer),
	// 	],
	// 	constraints: vec![],
	// 	composite_primary_key: Some(vec![leak_str("user_id"), leak_str("order_id")]),
	// 	interleave_in_parent: Some(("users", vec![leak_str("user_id")])),
	// }
	//
	// Expected SQL:
	// CREATE TABLE orders (
	// 	user_id INT,
	// 	order_id INT,
	// 	PRIMARY KEY(user_id, order_id)
	// ) INTERLEAVE IN PARENT users (user_id)
}

/// Test AS OF SYSTEM TIME (time-travel queries)
///
/// **Test Intent**: Verify that time-travel queries can be performed
///
/// **CockroachDB Feature**: AS OF SYSTEM TIME allows querying historical data,
/// useful for auditing, debugging, and consistent reads.
#[rstest]
#[ignore = "AS OF SYSTEM TIME is a query feature, not a migration feature"]
#[tokio::test]
async fn test_cockroachdb_time_travel_query() {
	// Note: AS OF SYSTEM TIME is a query feature, not a migration feature.
	// This test is included for completeness but doesn't directly relate to migrations.

	// Example usage (in application code, not migrations):
	// SELECT * FROM users AS OF SYSTEM TIME '-1h' WHERE id = 1
	//
	// This retrieves the user as it existed 1 hour ago.
}
