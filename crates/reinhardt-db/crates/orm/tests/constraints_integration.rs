//! Database Constraints Integration Tests
//!
//! This module tests database constraint enforcement including:
//! - UNIQUE constraints
//! - CHECK constraints
//! - Foreign key constraints (INSERT violations and DELETE CASCADE)
//! - NOT NULL constraints
//! - PRIMARY KEY constraints
//! - Multiple constraint combinations
//!
//! # Test Strategy
//!
//! Tests cover abnormal cases where constraint violations should occur:
//! - Duplicate values for UNIQUE constraints
//! - Invalid values for CHECK constraints
//! - Referential integrity violations for FK constraints
//! - NULL values for NOT NULL constraints
//! - Duplicate primary keys
//!
//! # Decision Table
//!
//! | Constraint Type | Test Scenario                | Expected Result         |
//! |----------------|------------------------------|-------------------------|
//! | UNIQUE         | Duplicate value insert       | Error                   |
//! | CHECK          | Value violates condition     | Error                   |
//! | FK             | Insert with invalid FK       | Error                   |
//! | FK             | Delete parent (CASCADE)      | Child deleted           |
//! | NOT NULL       | Insert NULL value            | Error                   |
//! | PRIMARY KEY    | Duplicate PK insert          | Error                   |
//! | Multiple       | Violate multiple constraints | Error (first violation) |

use reinhardt_core::macros::model;
use reinhardt_db::orm::manager::reinitialize_database;
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sea_query::{
	ColumnDef, Expr, ExprTrait, ForeignKey, ForeignKeyAction, Iden, PostgresQueryBuilder, Query,
	Table,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

/// Users table identifier
#[derive(Debug, Clone, Copy, Iden)]
enum Users {
	Table,
	Id,
	Email,
	Age,
	Status,
}

/// Products table identifier
#[derive(Debug, Clone, Copy, Iden)]
enum Products {
	Table,
	Id,
	Name,
	Price,
}

/// Orders table identifier (references Products)
#[derive(Debug, Clone, Copy, Iden)]
enum Orders {
	Table,
	Id,
	ProductId,
	Quantity,
}

/// Profiles table identifier (references Users)
#[derive(Debug, Clone, Copy, Iden)]
enum Profiles {
	Table,
	Id,
	UserId,
	Bio,
}

// ============================================================================
// ORM Model Definitions
// ============================================================================

/// User model for constraint testing with ORM
#[allow(dead_code)]
#[model(app_label = "constraints_test", table_name = "users")]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct User {
	#[field(primary_key = true)]
	id: Option<i32>,
	#[field(max_length = 255, unique = true)]
	email: String,
	#[field(check = "age >= 0")]
	age: i32,
	#[field(max_length = 50)]
	status: String,
}

/// Product model for FK constraint testing
#[allow(dead_code)]
#[model(app_label = "constraints_test", table_name = "products")]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Product {
	#[field(primary_key = true)]
	id: Option<i32>,
	#[field(max_length = 255)]
	name: String,
	price: i32,
}

/// Order model for FK constraint testing (references Product)
#[allow(dead_code)]
#[model(app_label = "constraints_test", table_name = "orders")]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Order {
	#[field(primary_key = true)]
	id: Option<i32>,
	product_id: i32,
	quantity: i32,
}

/// Profile model for FK constraint testing (references User)
#[allow(dead_code)]
#[model(app_label = "constraints_test", table_name = "profiles")]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Profile {
	#[field(primary_key = true)]
	id: Option<i32>,
	user_id: i32,
	#[field(null = true, max_length = 1000)]
	bio: Option<String>,
}

/// Helper function to setup users table with various constraints
async fn setup_users_table(pool: &PgPool) -> Result<(), sqlx::Error> {
	let create_table = Table::create()
		.table(Users::Table)
		.if_not_exists()
		.col(
			ColumnDef::new(Users::Id)
				.integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(
			ColumnDef::new(Users::Email)
				.string_len(255)
				.not_null()
				.unique_key(),
		)
		.col(
			ColumnDef::new(Users::Age)
				.integer()
				.not_null()
				.check(Expr::col(Users::Age).gte(0)),
		)
		.col(ColumnDef::new(Users::Status).string_len(50).not_null())
		.to_string(PostgresQueryBuilder);

	sqlx::query(&create_table).execute(pool).await?;
	Ok(())
}

/// Helper function to setup products and orders tables with FK constraints
async fn setup_products_orders_tables(pool: &PgPool) -> Result<(), sqlx::Error> {
	// Create products table
	let create_products = Table::create()
		.table(Products::Table)
		.if_not_exists()
		.col(
			ColumnDef::new(Products::Id)
				.integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new(Products::Name).string_len(255).not_null())
		.col(ColumnDef::new(Products::Price).integer().not_null())
		.to_string(PostgresQueryBuilder);

	sqlx::query(&create_products).execute(pool).await?;

	// Create orders table with FK to products
	let create_orders = Table::create()
		.table(Orders::Table)
		.if_not_exists()
		.col(
			ColumnDef::new(Orders::Id)
				.integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new(Orders::ProductId).integer().not_null())
		.col(ColumnDef::new(Orders::Quantity).integer().not_null())
		.foreign_key(
			ForeignKey::create()
				.name("fk_orders_product_id")
				.from(Orders::Table, Orders::ProductId)
				.to(Products::Table, Products::Id)
				.on_delete(ForeignKeyAction::Cascade),
		)
		.to_string(PostgresQueryBuilder);

	sqlx::query(&create_orders).execute(pool).await?;
	Ok(())
}

/// Helper function to setup profiles table with FK to users (NO CASCADE)
async fn setup_profiles_table(pool: &PgPool) -> Result<(), sqlx::Error> {
	let create_profiles = Table::create()
		.table(Profiles::Table)
		.if_not_exists()
		.col(
			ColumnDef::new(Profiles::Id)
				.integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new(Profiles::UserId).integer().not_null())
		.col(ColumnDef::new(Profiles::Bio).text())
		.foreign_key(
			ForeignKey::create()
				.name("fk_profiles_user_id")
				.from(Profiles::Table, Profiles::UserId)
				.to(Users::Table, Users::Id)
				.on_delete(ForeignKeyAction::Restrict),
		)
		.to_string(PostgresQueryBuilder);

	sqlx::query(&create_profiles).execute(pool).await?;
	Ok(())
}

/// Test UNIQUE constraint violation
///
/// # Test Scenario
/// 1. Insert user with email "test@example.com"
/// 2. Attempt to insert another user with same email
/// 3. Verify that second insert fails with UNIQUE constraint error
///
/// Uses reinhardt_orm for database connection management.
#[rstest]
#[tokio::test]
async fn test_unique_constraint_violation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Setup
	setup_users_table(pool.as_ref()).await.unwrap();

	// Insert first user
	let insert_first = Query::insert()
		.into_table(Users::Table)
		.columns([Users::Email, Users::Age, Users::Status])
		.values_panic(["test@example.com".into(), 25.into(), "active".into()])
		.to_string(PostgresQueryBuilder);

	sqlx::query(&insert_first)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Attempt to insert duplicate email
	let insert_duplicate = Query::insert()
		.into_table(Users::Table)
		.columns([Users::Email, Users::Age, Users::Status])
		.values_panic(["test@example.com".into(), 30.into(), "active".into()])
		.to_string(PostgresQueryBuilder);

	let result = sqlx::query(&insert_duplicate).execute(pool.as_ref()).await;

	// Verify constraint violation
	assert!(result.is_err());
	let error = result.unwrap_err();
	let error_message = error.to_string();
	assert!(
		error_message.contains("unique") || error_message.contains("duplicate"),
		"Expected UNIQUE constraint error, got: {}",
		error_message
	);
}

/// Test CHECK constraint violation
///
/// # Test Scenario
/// 1. Attempt to insert user with negative age (violates CHECK age >= 0)
/// 2. Verify that insert fails with CHECK constraint error
///
/// Uses reinhardt_orm for database connection management.
#[rstest]
#[tokio::test]
async fn test_check_constraint_violation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Setup
	setup_users_table(pool.as_ref()).await.unwrap();

	// Attempt to insert user with negative age
	let insert_invalid = Query::insert()
		.into_table(Users::Table)
		.columns([Users::Email, Users::Age, Users::Status])
		.values_panic(["negative@example.com".into(), (-5).into(), "active".into()])
		.to_string(PostgresQueryBuilder);

	let result = sqlx::query(&insert_invalid).execute(pool.as_ref()).await;

	// Verify constraint violation
	assert!(result.is_err());
	let error = result.unwrap_err();
	let error_message = error.to_string();
	assert!(
		error_message.contains("check") || error_message.contains("constraint"),
		"Expected CHECK constraint error, got: {}",
		error_message
	);
}

/// Test foreign key constraint violation on INSERT
///
/// # Test Scenario
/// 1. Create products and orders tables
/// 2. Attempt to insert order with non-existent product_id
/// 3. Verify that insert fails with FK constraint error
///
/// Uses reinhardt_orm for database connection management.
#[rstest]
#[tokio::test]
async fn test_foreign_key_constraint_insert_violation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Setup
	setup_products_orders_tables(pool.as_ref()).await.unwrap();

	// Attempt to insert order with non-existent product_id
	let insert_invalid_fk = Query::insert()
		.into_table(Orders::Table)
		.columns([Orders::ProductId, Orders::Quantity])
		.values_panic([9999.into(), 10.into()])
		.to_string(PostgresQueryBuilder);

	let result = sqlx::query(&insert_invalid_fk).execute(pool.as_ref()).await;

	// Verify constraint violation
	assert!(result.is_err());
	let error = result.unwrap_err();
	let error_message = error.to_string();
	assert!(
		error_message.contains("foreign key") || error_message.contains("violates"),
		"Expected FK constraint error, got: {}",
		error_message
	);
}

/// Test foreign key constraint DELETE CASCADE behavior
///
/// # Test Scenario
/// 1. Insert product with id=1
/// 2. Insert order referencing product_id=1
/// 3. Delete product with id=1
/// 4. Verify that order is automatically deleted (CASCADE)
///
/// Uses reinhardt_orm for database connection management.
#[rstest]
#[tokio::test]
async fn test_foreign_key_constraint_delete_cascade(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Setup
	setup_products_orders_tables(pool.as_ref()).await.unwrap();

	// Insert product
	let insert_product = Query::insert()
		.into_table(Products::Table)
		.columns([Products::Name, Products::Price])
		.values_panic(["Laptop".into(), 1000.into()])
		.to_string(PostgresQueryBuilder);

	sqlx::query(&insert_product)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Get product id
	let product_id: i32 = sqlx::query_scalar("SELECT id FROM products WHERE name = 'Laptop'")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Insert order referencing product
	let insert_order = Query::insert()
		.into_table(Orders::Table)
		.columns([Orders::ProductId, Orders::Quantity])
		.values_panic([product_id.into(), 5.into()])
		.to_string(PostgresQueryBuilder);

	sqlx::query(&insert_order)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Verify order exists
	let order_count_before: i64 = sqlx::query_scalar(&format!(
		"SELECT COUNT(*) FROM orders WHERE product_id = {}",
		product_id
	))
	.fetch_one(pool.as_ref())
	.await
	.unwrap();
	assert_eq!(order_count_before, 1);

	// Delete product (should cascade to orders)
	let delete_product = Query::delete()
		.from_table(Products::Table)
		.and_where(Expr::col(Products::Id).eq(product_id))
		.to_string(PostgresQueryBuilder);

	sqlx::query(&delete_product)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Verify order is deleted (CASCADE)
	let order_count_after: i64 = sqlx::query_scalar(&format!(
		"SELECT COUNT(*) FROM orders WHERE product_id = {}",
		product_id
	))
	.fetch_one(pool.as_ref())
	.await
	.unwrap();
	assert_eq!(order_count_after, 0, "Order should be deleted via CASCADE");
}

/// Test NOT NULL constraint violation
///
/// # Test Scenario
/// 1. Attempt to insert user with NULL email (violates NOT NULL)
/// 2. Verify that insert fails with NOT NULL constraint error
///
/// Uses reinhardt_orm for database connection management.
#[rstest]
#[tokio::test]
async fn test_not_null_constraint_violation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Setup
	setup_users_table(pool.as_ref()).await.unwrap();

	// Attempt to insert user without required email field
	// In SeaQuery, NULL is handled explicitly, so we test NOT NULL constraint violation by omitting the email column
	let insert_without_email = Query::insert()
		.into_table(Users::Table)
		.columns([Users::Age, Users::Status])
		.values_panic([25.into(), "active".into()])
		.to_string(PostgresQueryBuilder);

	let result = sqlx::query(&insert_without_email)
		.execute(pool.as_ref())
		.await;

	// Verify constraint violation
	assert!(result.is_err());
	let error = result.unwrap_err();
	let error_message = error.to_string();
	assert!(
		error_message.contains("not null") || error_message.contains("null value"),
		"Expected NOT NULL constraint error, got: {}",
		error_message
	);
}

/// Test PRIMARY KEY constraint violation
///
/// # Test Scenario
/// 1. Insert user with explicit id=1
/// 2. Attempt to insert another user with same id=1
/// 3. Verify that second insert fails with PRIMARY KEY constraint error
///
/// Uses reinhardt_orm for database connection management.
#[rstest]
#[tokio::test]
async fn test_primary_key_constraint_violation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Setup
	setup_users_table(pool.as_ref()).await.unwrap();

	// Insert first user with explicit id
	let insert_first = Query::insert()
		.into_table(Users::Table)
		.columns([Users::Id, Users::Email, Users::Age, Users::Status])
		.values_panic([
			1.into(),
			"first@example.com".into(),
			25.into(),
			"active".into(),
		])
		.to_string(PostgresQueryBuilder);

	sqlx::query(&insert_first)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Attempt to insert duplicate primary key
	let insert_duplicate_pk = Query::insert()
		.into_table(Users::Table)
		.columns([Users::Id, Users::Email, Users::Age, Users::Status])
		.values_panic([
			1.into(),
			"second@example.com".into(),
			30.into(),
			"active".into(),
		])
		.to_string(PostgresQueryBuilder);

	let result = sqlx::query(&insert_duplicate_pk)
		.execute(pool.as_ref())
		.await;

	// Verify constraint violation
	assert!(result.is_err());
	let error = result.unwrap_err();
	let error_message = error.to_string();
	assert!(
		error_message.contains("primary key")
			|| error_message.contains("duplicate")
			|| error_message.contains("unique"),
		"Expected PRIMARY KEY constraint error, got: {}",
		error_message
	);
}

/// Test multiple constraints combination
///
/// # Test Scenario
/// 1. Create users and profiles tables (FK RESTRICT)
/// 2. Insert user and profile
/// 3. Attempt to delete user (should fail due to FK RESTRICT)
/// 4. Verify proper constraint error handling
///
/// Uses reinhardt_orm for database connection management.
#[rstest]
#[tokio::test]
async fn test_multiple_constraints_combination(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Setup
	setup_users_table(pool.as_ref()).await.unwrap();
	setup_profiles_table(pool.as_ref()).await.unwrap();

	// Insert user
	let insert_user = Query::insert()
		.into_table(Users::Table)
		.columns([Users::Email, Users::Age, Users::Status])
		.values_panic(["user@example.com".into(), 25.into(), "active".into()])
		.to_string(PostgresQueryBuilder);

	sqlx::query(&insert_user)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Get user id
	let user_id: i32 = sqlx::query_scalar("SELECT id FROM users WHERE email = 'user@example.com'")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Insert profile referencing user
	let insert_profile = Query::insert()
		.into_table(Profiles::Table)
		.columns([Profiles::UserId, Profiles::Bio])
		.values_panic([user_id.into(), "Test bio".into()])
		.to_string(PostgresQueryBuilder);

	sqlx::query(&insert_profile)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Attempt to delete user (should fail due to FK RESTRICT)
	let delete_user = Query::delete()
		.from_table(Users::Table)
		.and_where(Expr::col(Users::Id).eq(user_id))
		.to_string(PostgresQueryBuilder);

	let result = sqlx::query(&delete_user).execute(pool.as_ref()).await;

	// Verify constraint violation
	assert!(result.is_err());
	let error = result.unwrap_err();
	let error_message = error.to_string();
	assert!(
		error_message.contains("foreign key") || error_message.contains("violates"),
		"Expected FK RESTRICT constraint error, got: {}",
		error_message
	);

	// Verify user still exists
	let user_count: i64 = sqlx::query_scalar(&format!(
		"SELECT COUNT(*) FROM users WHERE id = {}",
		user_id
	))
	.fetch_one(pool.as_ref())
	.await
	.unwrap();
	assert_eq!(
		user_count, 1,
		"User should not be deleted due to FK RESTRICT"
	);
}
