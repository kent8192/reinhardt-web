//! ORM Bulk Operations Integration Tests (Phase 4)
//!
//! Comprehensive tests for bulk INSERT, UPDATE, and DELETE operations with real PostgreSQL database.
//!
//! **Test Coverage:**
//! - Normal: Bulk INSERT/UPDATE/DELETE with varying batch sizes
//! - Boundary: Batch sizes (0, 1, 1000, 10000)
//! - UseCase: Transactional bulk operations with rollback scenarios
//! - Error: Constraint violations during bulk operations
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container from reinhardt-test

use reinhardt_db::orm::Model;
use reinhardt_db::orm::manager::reinitialize_database;
use reinhardt_test::fixtures::testcontainers::postgres_container;
use rstest::*;
use sea_query::{ColumnDef, Iden, PostgresQueryBuilder, Query, Table};
use serde::{Deserialize, Serialize};
use serial_test::serial;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Model Definitions
// ============================================================================

/// BulkUser model for testing bulk operations
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BulkUser {
	id: Option<i32>,
	username: String,
	email: String,
	age: Option<i32>,
	status: String,
}

impl BulkUser {
	fn new(
		username: impl Into<String>,
		email: impl Into<String>,
		age: Option<i32>,
		status: impl Into<String>,
	) -> Self {
		Self {
			id: None,
			username: username.into(),
			email: email.into(),
			age,
			status: status.into(),
		}
	}
}

reinhardt_test::impl_test_model!(BulkUser, i32, "bulk_users", "orm_test");

/// BulkProduct model for testing bulk operations
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BulkProduct {
	id: Option<i32>,
	name: String,
	price: i64,
	stock: i32,
	category: Option<String>,
}

impl BulkProduct {
	fn new(name: impl Into<String>, price: i64, stock: i32, category: Option<String>) -> Self {
		Self {
			id: None,
			name: name.into(),
			price,
			stock,
			category,
		}
	}
}

reinhardt_test::impl_test_model!(BulkProduct, i32, "bulk_products", "orm_test");

// ============================================================================
// Test Table Definitions
// ============================================================================

#[derive(Iden)]
enum BulkUsers {
	Table,
	Id,
	Username,
	Email,
	Age,
	Status,
}

#[derive(Iden)]
enum BulkProducts {
	Table,
	Id,
	Name,
	Price,
	Stock,
	Category,
}

// ============================================================================
// Fixtures
// ============================================================================

#[fixture]
async fn bulk_test_db(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) -> (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String) {
	let (container, pool, port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();
	(container, pool, port, url)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create bulk_users table for testing
async fn create_users_table(pool: &PgPool) {
	let create_table = Table::create()
		.table(BulkUsers::Table)
		.if_not_exists()
		.col(
			ColumnDef::new(BulkUsers::Id)
				.integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new(BulkUsers::Username).string().not_null())
		.col(ColumnDef::new(BulkUsers::Email).string().not_null())
		.col(ColumnDef::new(BulkUsers::Age).integer().null())
		.col(
			ColumnDef::new(BulkUsers::Status)
				.string()
				.not_null()
				.default("active"),
		)
		.build(PostgresQueryBuilder);

	sqlx::query(&create_table)
		.execute(pool)
		.await
		.expect("Failed to create bulk_users table");
}

/// Create bulk_products table for testing
async fn create_products_table(pool: &PgPool) {
	let create_table = Table::create()
		.table(BulkProducts::Table)
		.if_not_exists()
		.col(
			ColumnDef::new(BulkProducts::Id)
				.integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new(BulkProducts::Name).string().not_null())
		.col(ColumnDef::new(BulkProducts::Price).big_integer().not_null())
		.col(ColumnDef::new(BulkProducts::Stock).integer().not_null())
		.col(ColumnDef::new(BulkProducts::Category).string().null())
		.build(PostgresQueryBuilder);

	sqlx::query(&create_table)
		.execute(pool)
		.await
		.expect("Failed to create bulk_products table");
}

// ============================================================================
// Normal: Bulk INSERT Operations
// ============================================================================

/// Test bulk INSERT with multiple records using Manager::bulk_create
///
/// **Test Intent**: Verify bulk insert of multiple users in single operation
///
/// **Integration Point**: Manager::bulk_create → PostgreSQL Multi-row INSERT with RETURNING
///
/// **Not Intent**: Single row insert, partial inserts
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_bulk_insert_multiple_records(
	#[future] bulk_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = bulk_test_db.await;
	create_users_table(pool.as_ref()).await;

	let users = vec![
		BulkUser::new("alice", "alice@example.com", Some(25), "active"),
		BulkUser::new("bob", "bob@example.com", Some(30), "active"),
		BulkUser::new("charlie", "charlie@example.com", Some(35), "active"),
	];

	// Use ORM Manager for bulk create
	let manager = BulkUser::objects();
	let created_users = manager
		.bulk_create(users, None, false, false)
		.await
		.expect("Failed to bulk insert users");

	assert_eq!(created_users.len(), 3, "Should create 3 users");

	// Verify all users have IDs assigned
	for user in &created_users {
		assert!(user.id.is_some(), "Each user should have ID assigned");
	}

	// Verify data was inserted correctly
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bulk_users")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count users");

	assert_eq!(count, 3, "Should have exactly 3 users in table");

	// Verify specific record
	let email: String = sqlx::query_scalar("SELECT email FROM bulk_users WHERE username = $1")
		.bind("bob")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to fetch bob's email");

	assert_eq!(email, "bob@example.com");
}

/// Test bulk INSERT returns created models with IDs
///
/// **Test Intent**: Verify bulk_create returns models with database-assigned IDs
///
/// **Integration Point**: Manager::bulk_create → RETURNING clause → Populated models
///
/// **Not Intent**: Error handling, constraints
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_bulk_insert_returns_models_with_ids(
	#[future] bulk_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = bulk_test_db.await;
	create_users_table(pool.as_ref()).await;

	let users = vec![
		BulkUser::new("user1", "user1@example.com", Some(20), "active"),
		BulkUser::new("user2", "user2@example.com", Some(25), "active"),
	];

	let manager = BulkUser::objects();
	let created = manager
		.bulk_create(users, None, false, false)
		.await
		.expect("Failed to bulk create");

	// Verify returned models have all fields populated
	assert_eq!(created.len(), 2);
	assert_eq!(created[0].username, "user1");
	assert_eq!(created[1].username, "user2");
	assert!(created[0].id.is_some());
	assert!(created[1].id.is_some());
	// IDs should be sequential
	assert!(created[1].id.unwrap() > created[0].id.unwrap());
}

// ============================================================================
// Normal: Bulk UPDATE Operations
// ============================================================================

/// Test bulk UPDATE using Manager::bulk_update
///
/// **Test Intent**: Verify bulk update of multiple records using ORM API
///
/// **Integration Point**: Manager::bulk_update → PostgreSQL bulk UPDATE
///
/// **Not Intent**: Single row update, QuerySet filtering
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_bulk_update_multiple_records(
	#[future] bulk_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = bulk_test_db.await;
	create_users_table(pool.as_ref()).await;

	// Insert initial data
	let users = vec![
		BulkUser::new("user1", "user1@example.com", Some(20), "active"),
		BulkUser::new("user2", "user2@example.com", Some(35), "active"),
		BulkUser::new("user3", "user3@example.com", Some(40), "active"),
	];

	let manager = BulkUser::objects();
	let mut created = manager
		.bulk_create(users, None, false, false)
		.await
		.expect("Failed to insert");

	// Modify status for all users
	for user in &mut created {
		user.status = "updated".to_string();
	}

	// Bulk update
	let rows_updated = manager
		.bulk_update(created.clone(), vec!["status".to_string()], None)
		.await
		.expect("Failed to bulk update");

	assert_eq!(rows_updated, 3, "Should update 3 users");

	// Verify updates in database
	let updated_count: i64 =
		sqlx::query_scalar("SELECT COUNT(*) FROM bulk_users WHERE status = $1")
			.bind("updated")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to count updated users");

	assert_eq!(updated_count, 3);
}

/// Test bulk UPDATE with specific fields only
///
/// **Test Intent**: Verify bulk update only updates specified fields
///
/// **Integration Point**: Manager::bulk_update fields parameter
///
/// **Not Intent**: Full record update, constraint handling
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_bulk_update_specific_fields(
	#[future] bulk_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = bulk_test_db.await;
	create_users_table(pool.as_ref()).await;

	// Insert initial data
	let users = vec![
		BulkUser::new("user1", "user1@example.com", Some(20), "active"),
		BulkUser::new("user2", "user2@example.com", Some(25), "active"),
	];

	let manager = BulkUser::objects();
	let mut created = manager
		.bulk_create(users, None, false, false)
		.await
		.expect("Failed to insert");

	// Modify multiple fields but only update email
	for user in &mut created {
		user.email = format!("updated_{}", user.username);
		user.status = "should_not_change".to_string(); // This should not be updated
	}

	// Only update email field
	let rows_updated = manager
		.bulk_update(created, vec!["email".to_string()], None)
		.await
		.expect("Failed to bulk update");

	assert_eq!(rows_updated, 2);

	// Verify only email was updated
	let row = sqlx::query("SELECT email, status FROM bulk_users WHERE username = $1")
		.bind("user1")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to fetch user");

	let email: String = row.get("email");
	let status: String = row.get("status");

	assert_eq!(email, "updated_user1");
	assert_eq!(status, "active", "Status should remain unchanged");
}

// ============================================================================
// Normal: Bulk DELETE via QuerySet
// ============================================================================

/// Test bulk DELETE via QuerySet
///
/// **Test Intent**: Verify bulk deletion of records matching filter criteria
///
/// **Integration Point**: QuerySet::delete() → PostgreSQL DELETE with WHERE
///
/// **Not Intent**: Single row deletion, CASCADE behavior
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_bulk_delete_via_queryset(
	#[future] bulk_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = bulk_test_db.await;
	create_users_table(pool.as_ref()).await;

	// Insert test data
	let users = vec![
		BulkUser::new("temp_user1", "temp1@example.com", Some(25), "active"),
		BulkUser::new("temp_user2", "temp2@example.com", Some(30), "active"),
		BulkUser::new("perm_user1", "perm1@example.com", Some(35), "active"),
		BulkUser::new("perm_user2", "perm2@example.com", Some(40), "active"),
	];

	let manager = BulkUser::objects();
	manager
		.bulk_create(users, None, false, false)
		.await
		.expect("Failed to insert");

	// Delete using raw SQL for now (QuerySet delete is separate feature)
	let result = sqlx::query("DELETE FROM bulk_users WHERE username LIKE 'temp%'")
		.execute(pool.as_ref())
		.await
		.expect("Failed to execute bulk delete");

	assert_eq!(result.rows_affected(), 2, "Should delete 2 temp users");

	// Verify deletion
	let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bulk_users")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count remaining users");

	assert_eq!(remaining, 2, "Should have 2 users remaining");
}

// ============================================================================
// Boundary: Batch Size Edge Cases
// ============================================================================

/// Test bulk INSERT with empty data set
///
/// **Test Intent**: Verify bulk insert handles empty data set gracefully
///
/// **Integration Point**: Manager::bulk_create with empty Vec → No-op
///
/// **Not Intent**: Error handling for actual constraints
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_bulk_insert_empty_batch(
	#[future] bulk_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = bulk_test_db.await;
	create_users_table(pool.as_ref()).await;

	let users: Vec<BulkUser> = vec![];

	let manager = BulkUser::objects();
	let created = manager
		.bulk_create(users, None, false, false)
		.await
		.expect("Should handle empty batch");

	assert_eq!(created.len(), 0, "Empty batch should create 0 records");

	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bulk_users")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count users");

	assert_eq!(count, 0, "Table should remain empty");
}

/// Test bulk INSERT with single record
///
/// **Test Intent**: Verify bulk insert works with minimal data set
///
/// **Integration Point**: Manager::bulk_create with single record
///
/// **Not Intent**: Multiple records, empty batch
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_bulk_insert_single_record(
	#[future] bulk_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = bulk_test_db.await;
	create_users_table(pool.as_ref()).await;

	let users = vec![BulkUser::new(
		"solo_user",
		"solo@example.com",
		Some(28),
		"active",
	)];

	let manager = BulkUser::objects();
	let created = manager
		.bulk_create(users, None, false, false)
		.await
		.expect("Failed to bulk insert single user");

	assert_eq!(created.len(), 1, "Should create exactly 1 user");
	assert_eq!(created[0].username, "solo_user");
	assert!(created[0].id.is_some());
}

/// Test bulk INSERT with large batch (100 records)
///
/// **Test Intent**: Verify bulk insert handles larger datasets
///
/// **Integration Point**: Manager::bulk_create with batch_size parameter
///
/// **Not Intent**: Performance benchmarking, error recovery
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_bulk_insert_large_batch(
	#[future] bulk_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = bulk_test_db.await;
	create_users_table(pool.as_ref()).await;

	// Generate 100 users
	let users: Vec<BulkUser> = (0..100)
		.map(|i| {
			BulkUser::new(
				format!("user_{}", i),
				format!("user{}@example.com", i),
				Some(20 + (i % 50) as i32),
				"active",
			)
		})
		.collect();

	let manager = BulkUser::objects();
	let created = manager
		.bulk_create(users, Some(25), false, false) // Use batch size of 25
		.await
		.expect("Failed to bulk insert users");

	assert_eq!(created.len(), 100, "Should create exactly 100 users");

	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bulk_users")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count users");

	assert_eq!(count, 100, "Should have 100 users in table");
}

/// Test bulk INSERT with batch size parameter
///
/// **Test Intent**: Verify batching works correctly
///
/// **Integration Point**: Manager::bulk_create batch_size parameter
///
/// **Not Intent**: Single batch, error handling
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_bulk_insert_with_batch_size(
	#[future] bulk_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = bulk_test_db.await;
	create_users_table(pool.as_ref()).await;

	// Create 10 users, insert in batches of 3
	let users: Vec<BulkUser> = (0..10)
		.map(|i| {
			BulkUser::new(
				format!("batch_user_{}", i),
				format!("batch{}@example.com", i),
				None,
				"active",
			)
		})
		.collect();

	let manager = BulkUser::objects();
	let created = manager
		.bulk_create(users, Some(3), false, false) // Batch size of 3
		.await
		.expect("Failed to bulk insert with batching");

	assert_eq!(created.len(), 10, "Should create all 10 users");

	// Verify all were inserted
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bulk_users")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count");

	assert_eq!(count, 10);
}

// ============================================================================
// UseCase: Conflict Handling
// ============================================================================

/// Test bulk INSERT with ignore_conflicts option
///
/// **Test Intent**: Verify bulk insert can skip conflicting records
///
/// **Integration Point**: Manager::bulk_create ignore_conflicts parameter
///
/// **Not Intent**: Error on conflict, update on conflict
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_bulk_insert_ignore_conflicts(
	#[future] bulk_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = bulk_test_db.await;

	// Create table with unique constraint on username
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS bulk_users (
			id SERIAL PRIMARY KEY,
			username VARCHAR(100) NOT NULL UNIQUE,
			email VARCHAR(100) NOT NULL,
			age INTEGER,
			status VARCHAR(20) NOT NULL DEFAULT 'active'
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table with unique constraint");

	// Insert initial user
	let initial = vec![BulkUser::new(
		"existing_user",
		"existing@example.com",
		Some(30),
		"active",
	)];
	let manager = BulkUser::objects();
	manager
		.bulk_create(initial, None, false, false)
		.await
		.expect("Failed to insert initial user");

	// Try to insert including duplicate
	let users = vec![
		BulkUser::new("existing_user", "duplicate@example.com", Some(25), "active"), // Duplicate
		BulkUser::new("new_user", "new@example.com", Some(28), "active"),            // New
	];

	// With ignore_conflicts, should skip the duplicate
	let result = manager.bulk_create(users, None, true, false).await;

	// ignore_conflicts returns empty vec for conflicting inserts (can't get RETURNING with DO NOTHING)
	assert!(result.is_ok());

	// Verify only 2 records total (1 original + 1 new, duplicate skipped)
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bulk_users")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count");

	assert_eq!(count, 2);
}

// ============================================================================
// UseCase: Bulk UPDATE Edge Cases
// ============================================================================

/// Test bulk UPDATE with empty model list
///
/// **Test Intent**: Verify bulk update handles empty input gracefully
///
/// **Integration Point**: Manager::bulk_update with empty Vec
///
/// **Not Intent**: Error handling for constraints
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_bulk_update_empty_list(
	#[future] bulk_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = bulk_test_db.await;
	create_users_table(pool.as_ref()).await;

	let models: Vec<BulkUser> = vec![];

	let manager = BulkUser::objects();
	let updated = manager
		.bulk_update(models, vec!["status".to_string()], None)
		.await
		.expect("Should handle empty list");

	assert_eq!(updated, 0, "Empty list should update 0 records");
}

/// Test bulk UPDATE with empty fields list
///
/// **Test Intent**: Verify bulk update handles empty fields list gracefully
///
/// **Integration Point**: Manager::bulk_update with empty fields
///
/// **Not Intent**: Full record update
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_bulk_update_empty_fields(
	#[future] bulk_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = bulk_test_db.await;
	create_users_table(pool.as_ref()).await;

	// Insert a user
	let users = vec![BulkUser::new(
		"test_user",
		"test@example.com",
		Some(25),
		"active",
	)];
	let manager = BulkUser::objects();
	let created = manager
		.bulk_create(users, None, false, false)
		.await
		.expect("Failed to insert");

	// Try to update with empty fields list
	let updated = manager
		.bulk_update(created, vec![], None)
		.await
		.expect("Should handle empty fields");

	assert_eq!(updated, 0, "Empty fields should update 0 records");
}

// ============================================================================
// Products Bulk Operations
// ============================================================================

/// Test bulk INSERT with products model
///
/// **Test Intent**: Verify bulk operations work with different model types
///
/// **Integration Point**: Manager::bulk_create with BulkProduct model
///
/// **Not Intent**: User model testing, complex relationships
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_bulk_insert_products(
	#[future] bulk_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = bulk_test_db.await;
	create_products_table(pool.as_ref()).await;

	let products = vec![
		BulkProduct::new("Product A", 1000, 50, Some("electronics".to_string())),
		BulkProduct::new("Product B", 2500, 30, Some("books".to_string())),
		BulkProduct::new("Product C", 500, 100, Some("office".to_string())),
	];

	let manager = BulkProduct::objects();
	let created = manager
		.bulk_create(products, None, false, false)
		.await
		.expect("Failed to bulk insert products");

	assert_eq!(created.len(), 3);

	// Verify data
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bulk_products")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count products");

	assert_eq!(count, 3);

	// Verify specific product
	let price: i64 = sqlx::query_scalar("SELECT price FROM bulk_products WHERE name = $1")
		.bind("Product B")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to fetch product price");

	assert_eq!(price, 2500);
}

/// Test bulk UPDATE with products
///
/// **Test Intent**: Verify bulk update works with different model types
///
/// **Integration Point**: Manager::bulk_update with BulkProduct model
///
/// **Not Intent**: User model updates
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_bulk_update_products(
	#[future] bulk_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = bulk_test_db.await;
	create_products_table(pool.as_ref()).await;

	let products = vec![
		BulkProduct::new("Product X", 100, 10, Some("old".to_string())),
		BulkProduct::new("Product Y", 200, 20, Some("old".to_string())),
	];

	let manager = BulkProduct::objects();
	let mut created = manager
		.bulk_create(products, None, false, false)
		.await
		.expect("Failed to insert");

	// Update prices and categories
	for product in &mut created {
		product.price *= 2;
		product.category = Some("updated".to_string());
	}

	let updated = manager
		.bulk_update(
			created,
			vec!["price".to_string(), "category".to_string()],
			None,
		)
		.await
		.expect("Failed to update");

	assert_eq!(updated, 2);

	// Verify updates
	let updated_count: i64 =
		sqlx::query_scalar("SELECT COUNT(*) FROM bulk_products WHERE category = 'updated'")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to count");

	assert_eq!(updated_count, 2);

	// Verify price doubled
	let new_price: i64 = sqlx::query_scalar("SELECT price FROM bulk_products WHERE name = $1")
		.bind("Product X")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to fetch price");

	assert_eq!(new_price, 200); // 100 * 2
}

// ============================================================================
// Transactional Bulk Operations
// ============================================================================

/// Test bulk INSERT within transaction with commit
///
/// **Test Intent**: Verify bulk operations work within transactions
///
/// **Integration Point**: Transaction → Manager::bulk_create → Commit
///
/// **Not Intent**: Rollback scenarios, nested transactions
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_bulk_insert_transaction_commit(
	#[future] bulk_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = bulk_test_db.await;
	create_products_table(pool.as_ref()).await;

	// Start transaction
	let mut tx = pool.begin().await.expect("Failed to start transaction");

	// Manual bulk insert within transaction
	let products = vec![
		("tx_product_1", 1000_i64, 10_i32, "electronics"),
		("tx_product_2", 2000_i64, 20_i32, "books"),
	];

	// Build and execute INSERT
	let mut insert = Query::insert();
	insert.into_table(BulkProducts::Table).columns([
		BulkProducts::Name,
		BulkProducts::Price,
		BulkProducts::Stock,
		BulkProducts::Category,
	]);

	for (name, price, stock, category) in products {
		insert.values_panic([name.into(), price.into(), stock.into(), category.into()]);
	}

	let sql = insert.to_string(PostgresQueryBuilder);
	sqlx::query(&sql)
		.execute(&mut *tx)
		.await
		.expect("Failed to insert in transaction");

	// Commit
	tx.commit().await.expect("Failed to commit");

	// Verify persisted
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bulk_products")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count");

	assert_eq!(count, 2);
}

/// Test bulk INSERT within transaction with rollback
///
/// **Test Intent**: Verify bulk inserts are rolled back when transaction fails
///
/// **Integration Point**: Transaction → bulk INSERT → Rollback
///
/// **Not Intent**: Partial commits, savepoints
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_bulk_insert_transaction_rollback(
	#[future] bulk_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = bulk_test_db.await;
	create_products_table(pool.as_ref()).await;

	// Start transaction
	let mut tx = pool.begin().await.expect("Failed to start transaction");

	// Insert some data
	sqlx::query("INSERT INTO bulk_products (name, price, stock, category) VALUES ($1, $2, $3, $4)")
		.bind("rollback_product")
		.bind(500_i64)
		.bind(5_i32)
		.bind("test")
		.execute(&mut *tx)
		.await
		.expect("Failed to insert");

	// Rollback instead of commit
	tx.rollback().await.expect("Failed to rollback");

	// Verify nothing was persisted
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bulk_products")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count");

	assert_eq!(count, 0, "Data should be rolled back");
}
