//! ORM Composite Primary Key Integration Tests
//!
//! Comprehensive tests for composite primary key functionality with real PostgreSQL database.
//!
//! **Test Coverage:**
//! - Normal: Composite primary key uniqueness enforcement
//! - Normal: WHERE clause queries with composite PK
//! - Combination: Composite PK + Foreign Key relationships
//! - Edge Case: NULL values in composite PK (should fail)
//! - Error: Duplicate composite PK insertion (should fail)
//! - Advanced: Multi-column composite PK with 3+ columns
//! - Advanced: Composite PK with different data types
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container from reinhardt-test
//! - init_db: Initialize ORM database connection
//! - test_schema: Setup test tables with composite PKs

use reinhardt_core::macros::model;
use reinhardt_db::orm::{Model, init_database};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sea_query::{ColumnDef, ForeignKey, ForeignKeyAction, Iden, PostgresQueryBuilder, Table};
use serde::{Deserialize, Serialize};
use serial_test::serial;
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Model Definitions
// ============================================================================

/// CompositeUser model with composite primary key (tenant_id, user_id)
///
/// Allow dead_code: Model defined for testing, may not be directly instantiated in all tests
#[allow(dead_code)]
#[model(app_label = "orm_test", table_name = "composite_users")]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct CompositeUser {
	#[field(primary_key = true)]
	tenant_id: i32,
	#[field(primary_key = true)]
	user_id: i32,
	#[field(max_length = 200)]
	username: String,
	#[field(max_length = 200)]
	email: String,
}

/// MultiKeyProduct model with 3-column composite PK (region, category, product_code)
///
/// Allow dead_code: Model defined for testing, may not be directly instantiated in all tests
#[allow(dead_code)]
#[model(app_label = "orm_test", table_name = "multi_key_products")]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct MultiKeyProduct {
	#[field(primary_key = true, max_length = 50)]
	region: String,
	#[field(primary_key = true, max_length = 50)]
	category: String,
	#[field(primary_key = true, max_length = 50)]
	product_code: String,
	#[field(max_length = 200)]
	name: String,
	price: i32,
}

/// CompositeType model with mixed-type composite PK (int_key, text_key, bigint_key)
///
/// Allow dead_code: Model defined for testing, may not be directly instantiated in all tests
#[allow(dead_code)]
#[model(app_label = "orm_test", table_name = "composite_types")]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct CompositeType {
	#[field(primary_key = true)]
	int_key: i32,
	#[field(primary_key = true, max_length = 50)]
	text_key: String,
	#[field(primary_key = true)]
	bigint_key: i64,
	#[field(max_length = 200)]
	data: String,
}

// ============================================================================
// SeaQuery Table Definitions for Schema Creation
// ============================================================================

#[derive(Iden)]
#[iden = "composite_users"]
enum CompositeUsers {
	Table,
	#[iden = "tenant_id"]
	TenantId,
	#[iden = "user_id"]
	UserId,
	#[iden = "username"]
	Username,
	#[iden = "email"]
	Email,
}

#[derive(Iden)]
#[iden = "user_settings"]
enum UserSettings {
	Table,
	#[iden = "id"]
	Id,
	#[iden = "tenant_id"]
	TenantId,
	#[iden = "user_id"]
	UserId,
	#[iden = "setting_key"]
	SettingKey,
	#[iden = "setting_value"]
	SettingValue,
}

#[derive(Iden)]
#[iden = "multi_key_products"]
enum MultiKeyProducts {
	Table,
	#[iden = "region"]
	Region,
	#[iden = "category"]
	Category,
	#[iden = "product_code"]
	ProductCode,
	#[iden = "name"]
	Name,
	#[iden = "price"]
	Price,
}

#[derive(Iden)]
#[iden = "composite_types"]
enum CompositeTypes {
	Table,
	#[iden = "int_key"]
	IntKey,
	#[iden = "text_key"]
	TextKey,
	#[iden = "bigint_key"]
	BigintKey,
	#[iden = "data"]
	Data,
}

// ============================================================================
// Test Fixtures
// ============================================================================

/// Fixture: Initialize database connection
///
/// Dependencies: postgres_container (testcontainers)
#[fixture]
async fn init_db(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) -> (ContainerAsync<GenericImage>, Arc<PgPool>) {
	let (container, pool, _port, url) = postgres_container.await;
	init_database(&url).await.unwrap();
	(container, pool)
}

// ============================================================================
// Helper Functions for Schema Creation
// ============================================================================

/// Create composite_users table with composite PK (tenant_id, user_id)
async fn create_composite_users_table(pool: &PgPool) -> Result<(), sqlx::Error> {
	let create_table = Table::create()
		.table(CompositeUsers::Table)
		.if_not_exists()
		.col(
			ColumnDef::new(CompositeUsers::TenantId)
				.integer()
				.not_null(),
		)
		.col(ColumnDef::new(CompositeUsers::UserId).integer().not_null())
		.col(ColumnDef::new(CompositeUsers::Username).string().not_null())
		.col(ColumnDef::new(CompositeUsers::Email).string().not_null())
		.primary_key(
			sea_query::Index::create()
				.col(CompositeUsers::TenantId)
				.col(CompositeUsers::UserId)
				.primary(),
		)
		.to_owned();

	let sql = create_table.to_string(PostgresQueryBuilder);
	sqlx::query(&sql).execute(pool).await?;
	Ok(())
}

/// Create user_settings table with FK to composite_users
async fn create_user_settings_table(pool: &PgPool) -> Result<(), sqlx::Error> {
	let create_settings = Table::create()
		.table(UserSettings::Table)
		.if_not_exists()
		.col(
			ColumnDef::new(UserSettings::Id)
				.integer()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new(UserSettings::TenantId).integer().not_null())
		.col(ColumnDef::new(UserSettings::UserId).integer().not_null())
		.col(ColumnDef::new(UserSettings::SettingKey).string().not_null())
		.col(
			ColumnDef::new(UserSettings::SettingValue)
				.string()
				.not_null(),
		)
		.foreign_key(
			ForeignKey::create()
				.name("fk_user_settings_user")
				.from(
					UserSettings::Table,
					(UserSettings::TenantId, UserSettings::UserId),
				)
				.to(
					CompositeUsers::Table,
					(CompositeUsers::TenantId, CompositeUsers::UserId),
				)
				.on_delete(ForeignKeyAction::Cascade)
				.on_update(ForeignKeyAction::Cascade),
		)
		.to_owned();

	let sql = create_settings.to_string(PostgresQueryBuilder);
	sqlx::query(&sql).execute(pool).await?;
	Ok(())
}

/// Create multi_key_products table with 3-column composite PK
async fn create_multi_key_products_table(pool: &PgPool) -> Result<(), sqlx::Error> {
	let create_table = Table::create()
		.table(MultiKeyProducts::Table)
		.if_not_exists()
		.col(ColumnDef::new(MultiKeyProducts::Region).string().not_null())
		.col(
			ColumnDef::new(MultiKeyProducts::Category)
				.string()
				.not_null(),
		)
		.col(
			ColumnDef::new(MultiKeyProducts::ProductCode)
				.string()
				.not_null(),
		)
		.col(ColumnDef::new(MultiKeyProducts::Name).string().not_null())
		.col(ColumnDef::new(MultiKeyProducts::Price).integer().not_null())
		.primary_key(
			sea_query::Index::create()
				.col(MultiKeyProducts::Region)
				.col(MultiKeyProducts::Category)
				.col(MultiKeyProducts::ProductCode)
				.primary(),
		)
		.to_owned();

	let sql = create_table.to_string(PostgresQueryBuilder);
	sqlx::query(&sql).execute(pool).await?;
	Ok(())
}

/// Create composite_types table with mixed-type composite PK
async fn create_composite_types_table(pool: &PgPool) -> Result<(), sqlx::Error> {
	let create_table = Table::create()
		.table(CompositeTypes::Table)
		.if_not_exists()
		.col(ColumnDef::new(CompositeTypes::IntKey).integer().not_null())
		.col(ColumnDef::new(CompositeTypes::TextKey).string().not_null())
		.col(
			ColumnDef::new(CompositeTypes::BigintKey)
				.big_integer()
				.not_null(),
		)
		.col(ColumnDef::new(CompositeTypes::Data).string().not_null())
		.primary_key(
			sea_query::Index::create()
				.col(CompositeTypes::IntKey)
				.col(CompositeTypes::TextKey)
				.col(CompositeTypes::BigintKey)
				.primary(),
		)
		.to_owned();

	let sql = create_table.to_string(PostgresQueryBuilder);
	sqlx::query(&sql).execute(pool).await?;
	Ok(())
}

// ============================================================================
// Normal: Composite Primary Key Uniqueness
// ============================================================================

/// Test composite primary key enforces uniqueness constraint
///
/// **Test Intent**: Verify composite PK (tenant_id, user_id) allows same user_id
/// across different tenants but prevents duplicates within same tenant
///
/// **Integration Point**: ORM Composite PK → PostgreSQL CONSTRAINT PRIMARY KEY (col1, col2)
///
/// **Not Intent**: Single-column PK, UNIQUE constraints
#[rstest]
#[tokio::test]
#[serial]
async fn test_composite_pk_uniqueness(
	#[future] init_db: (ContainerAsync<GenericImage>, Arc<PgPool>),
) {
	let (_container, pool) = init_db.await;
	create_composite_users_table(&pool).await.unwrap();

	// Insert user1 in tenant1 using ORM API (struct direct construction for composite PK)
	let user1_t1 = CompositeUser {
		tenant_id: 1,
		user_id: 100,
		username: "alice_tenant1".to_string(),
		email: "alice@tenant1.com".to_string(),
	};
	CompositeUser::objects()
		.create(&user1_t1)
		.await
		.expect("Failed to insert user1 in tenant1");

	// Insert user1 in tenant2 (same user_id, different tenant_id - should succeed)
	let user1_t2 = CompositeUser {
		tenant_id: 2,
		user_id: 100,
		username: "alice_tenant2".to_string(),
		email: "alice@tenant2.com".to_string(),
	};
	CompositeUser::objects()
		.create(&user1_t2)
		.await
		.expect("Should allow same user_id in different tenant");

	// Verify both records exist using direct SQL
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM composite_users WHERE user_id = $1")
		.bind(100)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count records");

	assert_eq!(
		count, 2,
		"Should have 2 records with user_id=100 in different tenants"
	);

	// Attempt to insert duplicate (tenant1, user100) - should fail
	let duplicate = CompositeUser {
		tenant_id: 1,
		user_id: 100,
		username: "alice_duplicate".to_string(),
		email: "alice_dup@tenant1.com".to_string(),
	};
	let duplicate_result = CompositeUser::objects().create(&duplicate).await;

	assert!(
		duplicate_result.is_err(),
		"Duplicate composite PK should violate PRIMARY KEY constraint"
	);

	let error_msg = duplicate_result.unwrap_err().to_string();
	assert!(
		error_msg.to_lowercase().contains("duplicate")
			|| error_msg.to_lowercase().contains("unique")
			|| error_msg.to_lowercase().contains("primary"),
		"Error should indicate PK violation: {}",
		error_msg
	);
}

// ============================================================================
// Normal: WHERE Clause Queries with Composite PK
// ============================================================================

/// Test querying records using composite PK in WHERE clause
///
/// **Test Intent**: Verify composite PK can be used in WHERE clause for precise lookups
///
/// **Integration Point**: ORM Composite PK query → PostgreSQL WHERE (col1 = $1 AND col2 = $2)
///
/// **Not Intent**: Single-column lookups, partial key queries
#[rstest]
#[tokio::test]
#[serial]
async fn test_composite_pk_where_clause(
	#[future] init_db: (ContainerAsync<GenericImage>, Arc<PgPool>),
) {
	let (_container, pool) = init_db.await;
	create_composite_users_table(&pool).await.unwrap();

	// Insert test data using ORM API
	for tenant_id in 1..=3 {
		for user_id in 100..=102 {
			let user = CompositeUser {
				tenant_id,
				user_id,
				username: format!("user{}_{}", tenant_id, user_id),
				email: format!("user{}@tenant{}.com", user_id, tenant_id),
			};
			CompositeUser::objects()
				.create(&user)
				.await
				.expect("Failed to insert test data");
		}
	}

	// Query with WHERE clause using composite PK (direct SQL for now)
	let result = sqlx::query(
		"SELECT tenant_id, user_id, username, email FROM composite_users WHERE tenant_id = $1 AND user_id = $2"
	)
	.bind(2)
	.bind(101)
	.fetch_one(pool.as_ref())
	.await
	.expect("Query failed");

	use sqlx::Row;
	let username: String = result.get("username");
	let email: String = result.get("email");

	assert_eq!(username, "user2_101");
	assert_eq!(email, "user101@tenant2.com");

	// Verify only one record matches
	let count: i64 = sqlx::query_scalar(
		"SELECT COUNT(*) FROM composite_users WHERE tenant_id = $1 AND user_id = $2",
	)
	.bind(2)
	.bind(101)
	.fetch_one(pool.as_ref())
	.await
	.expect("Count query failed");

	assert_eq!(count, 1, "Composite PK should return exactly one record");
}

// ============================================================================
// Combination: Composite PK + Foreign Key Relationship
// ============================================================================

/// Test foreign key referencing composite primary key
///
/// **Test Intent**: Verify FK can reference composite PK and enforces referential integrity
///
/// **Integration Point**: ORM Composite PK FK → PostgreSQL FOREIGN KEY (col1, col2) REFERENCES table(pk1, pk2)
///
/// **Not Intent**: Single-column FK, ON DELETE CASCADE behavior
#[rstest]
#[tokio::test]
#[serial]
async fn test_composite_pk_with_fk(#[future] init_db: (ContainerAsync<GenericImage>, Arc<PgPool>)) {
	let (_container, pool) = init_db.await;
	create_composite_users_table(&pool).await.unwrap();
	create_user_settings_table(&pool).await.unwrap();

	// Insert parent record using ORM API
	let user = CompositeUser {
		tenant_id: 1,
		user_id: 200,
		username: "bob".to_string(),
		email: "bob@tenant1.com".to_string(),
	};
	CompositeUser::objects()
		.create(&user)
		.await
		.expect("Failed to insert parent");

	// Insert child record with valid FK (using direct SQL due to separate table)
	sqlx::query("INSERT INTO user_settings (tenant_id, user_id, setting_key, setting_value) VALUES ($1, $2, $3, $4)")
		.bind(1)
		.bind(200)
		.bind("theme")
		.bind("dark")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert child with valid FK");

	// Attempt to insert child with invalid FK (non-existent parent)
	let invalid_fk_result = sqlx::query("INSERT INTO user_settings (tenant_id, user_id, setting_key, setting_value) VALUES ($1, $2, $3, $4)")
		.bind(999)
		.bind(999)
		.bind("theme")
		.bind("light")
		.execute(pool.as_ref())
		.await;

	assert!(
		invalid_fk_result.is_err(),
		"FK constraint should prevent insertion with non-existent parent"
	);

	let fk_error = invalid_fk_result.unwrap_err().to_string();
	assert!(
		fk_error.to_lowercase().contains("foreign")
			|| fk_error.to_lowercase().contains("violat")
			|| fk_error.to_lowercase().contains("constraint"),
		"Error should indicate FK violation: {}",
		fk_error
	);

	// Verify CASCADE DELETE works (delete parent using direct SQL)
	sqlx::query("DELETE FROM composite_users WHERE tenant_id = $1 AND user_id = $2")
		.bind(1)
		.bind(200)
		.execute(pool.as_ref())
		.await
		.expect("Failed to delete parent");

	let settings_count: i64 = sqlx::query_scalar(
		"SELECT COUNT(*) FROM user_settings WHERE tenant_id = $1 AND user_id = $2",
	)
	.bind(1)
	.bind(200)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to count child records");

	assert_eq!(
		settings_count, 0,
		"CASCADE DELETE should remove child records"
	);
}

// ============================================================================
// Edge Case: NULL in Composite PK
// ============================================================================

/// Test composite PK rejects NULL values in any column
///
/// **Test Intent**: Verify composite PK enforces NOT NULL constraint on all key columns
///
/// **Integration Point**: ORM Composite PK → PostgreSQL PRIMARY KEY NOT NULL enforcement
///
/// **Not Intent**: Nullable FK columns, partial NULL support
#[rstest]
#[tokio::test]
#[serial]
async fn test_composite_pk_null_rejection(
	#[future] init_db: (ContainerAsync<GenericImage>, Arc<PgPool>),
) {
	let (_container, pool) = init_db.await;
	create_composite_users_table(&pool).await.unwrap();

	// Attempt to insert NULL in first key column (using direct SQL since ORM doesn't allow NULL in non-Option fields)
	let null_tenant_result = sqlx::query(
		"INSERT INTO composite_users (tenant_id, user_id, username, email) VALUES ($1, $2, $3, $4)",
	)
	.bind(Option::<i32>::None)
	.bind(300)
	.bind("charlie")
	.bind("charlie@test.com")
	.execute(pool.as_ref())
	.await;

	assert!(
		null_tenant_result.is_err(),
		"NULL in composite PK column should fail"
	);

	// Attempt to insert NULL in second key column
	let null_user_result = sqlx::query(
		"INSERT INTO composite_users (tenant_id, user_id, username, email) VALUES ($1, $2, $3, $4)",
	)
	.bind(1)
	.bind(Option::<i32>::None)
	.bind("dave")
	.bind("dave@test.com")
	.execute(pool.as_ref())
	.await;

	assert!(
		null_user_result.is_err(),
		"NULL in composite PK column should fail"
	);

	let null_error = null_user_result.unwrap_err().to_string();
	assert!(
		null_error.to_lowercase().contains("null")
			|| null_error.to_lowercase().contains("not null")
			|| null_error.to_lowercase().contains("violat"),
		"Error should indicate NOT NULL violation: {}",
		null_error
	);
}

// ============================================================================
// Error: Duplicate Composite PK Insertion
// ============================================================================

/// Test duplicate composite PK insertion fails with clear error
///
/// **Test Intent**: Verify attempting to insert duplicate composite PK produces constraint violation error
///
/// **Integration Point**: ORM Composite PK → PostgreSQL duplicate key error
///
/// **Not Intent**: Upsert behavior, ON CONFLICT handling
#[rstest]
#[tokio::test]
#[serial]
async fn test_duplicate_composite_pk_error(
	#[future] init_db: (ContainerAsync<GenericImage>, Arc<PgPool>),
) {
	let (_container, pool) = init_db.await;
	create_composite_users_table(&pool).await.unwrap();

	// Insert first record using ORM API
	let original = CompositeUser {
		tenant_id: 5,
		user_id: 500,
		username: "original_user".to_string(),
		email: "original@test.com".to_string(),
	};
	CompositeUser::objects()
		.create(&original)
		.await
		.expect("Failed to insert original record");

	// Attempt to insert duplicate with different non-key columns
	let duplicate = CompositeUser {
		tenant_id: 5,
		user_id: 500,
		username: "duplicate_user".to_string(),
		email: "duplicate@test.com".to_string(),
	};
	let duplicate_result = CompositeUser::objects().create(&duplicate).await;

	assert!(
		duplicate_result.is_err(),
		"Duplicate composite PK should fail even with different non-key values"
	);

	let error = duplicate_result.unwrap_err();
	let error_msg = error.to_string();

	// Verify error message indicates duplicate key violation
	assert!(
		error_msg.to_lowercase().contains("duplicate")
			|| error_msg.to_lowercase().contains("unique")
			|| error_msg.to_lowercase().contains("primary"),
		"Error should clearly indicate duplicate key: {}",
		error_msg
	);

	// Verify database integrity - only one record exists
	let count: i64 = sqlx::query_scalar(
		"SELECT COUNT(*) FROM composite_users WHERE tenant_id = $1 AND user_id = $2",
	)
	.bind(5)
	.bind(500)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to count records");

	assert_eq!(
		count, 1,
		"Database should reject duplicate, maintaining only original record"
	);
}

// ============================================================================
// Advanced: Multi-Column Composite PK (3+ columns)
// ============================================================================

/// Test composite PK with 3+ columns
///
/// **Test Intent**: Verify composite PK works correctly with more than 2 columns
///
/// **Integration Point**: ORM Multi-column Composite PK → PostgreSQL PRIMARY KEY (col1, col2, col3, ...)
///
/// **Not Intent**: 2-column composite PK, single-column PK
#[rstest]
#[tokio::test]
#[serial]
async fn test_multi_column_composite_pk(
	#[future] init_db: (ContainerAsync<GenericImage>, Arc<PgPool>),
) {
	let (_container, pool) = init_db.await;
	create_multi_key_products_table(&pool).await.unwrap();

	// Insert product: (US, Electronics, LAPTOP001) using ORM API
	let product1 = MultiKeyProduct {
		region: "US".to_string(),
		category: "Electronics".to_string(),
		product_code: "LAPTOP001".to_string(),
		name: "ThinkPad X1".to_string(),
		price: 150000,
	};
	MultiKeyProduct::objects()
		.create(&product1)
		.await
		.expect("Failed to insert product 1");

	// Insert product: (US, Electronics, LAPTOP002) - same region+category, different code
	let product2 = MultiKeyProduct {
		region: "US".to_string(),
		category: "Electronics".to_string(),
		product_code: "LAPTOP002".to_string(),
		name: "MacBook Pro".to_string(),
		price: 200000,
	};
	MultiKeyProduct::objects()
		.create(&product2)
		.await
		.expect("Should allow different product_code");

	// Insert product: (US, Furniture, LAPTOP001) - same region+code, different category
	let product3 = MultiKeyProduct {
		region: "US".to_string(),
		category: "Furniture".to_string(),
		product_code: "LAPTOP001".to_string(),
		name: "Laptop Stand".to_string(),
		price: 5000,
	};
	MultiKeyProduct::objects()
		.create(&product3)
		.await
		.expect("Should allow different category");

	// Insert product: (EU, Electronics, LAPTOP001) - same category+code, different region
	let product4 = MultiKeyProduct {
		region: "EU".to_string(),
		category: "Electronics".to_string(),
		product_code: "LAPTOP001".to_string(),
		name: "ThinkPad X1 EU".to_string(),
		price: 160000,
	};
	MultiKeyProduct::objects()
		.create(&product4)
		.await
		.expect("Should allow different region");

	// Attempt duplicate (US, Electronics, LAPTOP001) - should fail
	let duplicate = MultiKeyProduct {
		region: "US".to_string(),
		category: "Electronics".to_string(),
		product_code: "LAPTOP001".to_string(),
		name: "Duplicate Product".to_string(),
		price: 100000,
	};
	let duplicate_result = MultiKeyProduct::objects().create(&duplicate).await;

	assert!(
		duplicate_result.is_err(),
		"Duplicate 3-column composite PK should fail"
	);

	// Query by full composite PK (using direct SQL)
	let result = sqlx::query("SELECT name, price FROM multi_key_products WHERE region = $1 AND category = $2 AND product_code = $3")
		.bind("US")
		.bind("Electronics")
		.bind("LAPTOP001")
		.fetch_one(pool.as_ref())
		.await
		.expect("Query by composite PK failed");

	use sqlx::Row;
	let name: String = result.get("name");
	let price: i32 = result.get("price");

	assert_eq!(name, "ThinkPad X1");
	assert_eq!(price, 150000);

	// Verify total records
	let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM multi_key_products")
		.fetch_one(pool.as_ref())
		.await
		.expect("Count failed");

	assert_eq!(
		total, 4,
		"Should have 4 unique combinations of (region, category, product_code)"
	);
}

// ============================================================================
// Advanced: Composite PK with Different Data Types
// ============================================================================

/// Test composite PK with heterogeneous column types
///
/// **Test Intent**: Verify composite PK works with different data types (INT, TEXT, BIGINT)
///
/// **Integration Point**: ORM Type-safe Composite PK → PostgreSQL mixed-type PK
///
/// **Not Intent**: Same-type composite PK, single-type PK
#[rstest]
#[tokio::test]
#[serial]
async fn test_composite_pk_different_types(
	#[future] init_db: (ContainerAsync<GenericImage>, Arc<PgPool>),
) {
	let (_container, pool) = init_db.await;
	create_composite_types_table(&pool).await.unwrap();

	// Insert records with different type combinations using ORM API
	let type1 = CompositeType {
		int_key: 1,
		text_key: "alpha".to_string(),
		bigint_key: 1000000000000_i64,
		data: "data1".to_string(),
	};
	CompositeType::objects()
		.create(&type1)
		.await
		.expect("Failed to insert record 1");

	let type2 = CompositeType {
		int_key: 1,
		text_key: "alpha".to_string(),
		bigint_key: 2000000000000_i64,
		data: "data2".to_string(),
	};
	CompositeType::objects()
		.create(&type2)
		.await
		.expect("Should allow different bigint_key");

	let type3 = CompositeType {
		int_key: 1,
		text_key: "beta".to_string(),
		bigint_key: 1000000000000_i64,
		data: "data3".to_string(),
	};
	CompositeType::objects()
		.create(&type3)
		.await
		.expect("Should allow different text_key");

	let type4 = CompositeType {
		int_key: 2,
		text_key: "alpha".to_string(),
		bigint_key: 1000000000000_i64,
		data: "data4".to_string(),
	};
	CompositeType::objects()
		.create(&type4)
		.await
		.expect("Should allow different int_key");

	// Attempt duplicate (1, "alpha", 1000000000000)
	let duplicate = CompositeType {
		int_key: 1,
		text_key: "alpha".to_string(),
		bigint_key: 1000000000000_i64,
		data: "duplicate_data".to_string(),
	};
	let duplicate_result = CompositeType::objects().create(&duplicate).await;

	assert!(
		duplicate_result.is_err(),
		"Duplicate mixed-type composite PK should fail"
	);

	// Query by full composite PK with type safety (using direct SQL)
	let result = sqlx::query(
		"SELECT data FROM composite_types WHERE int_key = $1 AND text_key = $2 AND bigint_key = $3",
	)
	.bind(1_i32)
	.bind("alpha")
	.bind(1000000000000_i64)
	.fetch_one(pool.as_ref())
	.await
	.expect("Query failed");

	use sqlx::Row;
	let data: String = result.get("data");
	assert_eq!(data, "data1");

	// Verify all 4 records exist
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM composite_types")
		.fetch_one(pool.as_ref())
		.await
		.expect("Count failed");

	assert_eq!(
		count, 4,
		"Should have 4 unique combinations of mixed-type composite PK"
	);
}
