//! Test fixtures for DML integration tests

// ============================================================================
// Models
// ============================================================================

use reinhardt_macros::model;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

#[model(table_name = "users")]
#[derive(Serialize, Deserialize, Clone)]
pub struct Users {
	#[field(primary_key = true)]
	pub id: i32,

	#[field(max_length = 255)]
	pub name: String,

	#[field(max_length = 255, unique = true)]
	pub email: String,

	pub age: Option<i32>,

	#[field(default = true)]
	pub active: bool,
}

#[model(table_name = "products")]
#[derive(Serialize, Deserialize, Clone)]
pub struct Products {
	#[field(primary_key = true)]
	pub id: i32,

	#[field(max_length = 255)]
	pub name: String,

	#[field(max_length = 100)]
	pub sku: String,

	pub price: i64,

	pub stock: i32,

	#[field(default = true)]
	pub available: bool,
}

#[model(table_name = "orders")]
#[derive(Serialize, Deserialize, Clone)]
pub struct Orders {
	#[field(primary_key = true)]
	pub id: i32,

	pub user_id: i32,

	pub total_amount: i64,

	#[field(max_length = 50)]
	pub status: String,
}

// ============================================================================
// Database Fixtures
// ============================================================================

use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

/// PostgreSQL database pool fixture
#[fixture]
pub async fn pg_pool(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) -> Arc<PgPool> {
	let (_container, pool, _port, _url) = postgres_container.await;
	pool
}

/// Empty database pool fixture (alias for pg_pool)
#[fixture]
pub async fn empty_db(#[future] pg_pool: Arc<PgPool>) -> Arc<PgPool> {
	pg_pool.await
}

// ============================================================================
// Table Fixtures
// ============================================================================

use sea_query::{ColumnDef, ForeignKey, ForeignKeyAction, PostgresQueryBuilder, Table};

/// Users table fixture
#[fixture]
pub async fn users_table(#[future] pg_pool: Arc<PgPool>) -> Arc<PgPool> {
	let pool = pg_pool.await;

	let create_table = Table::create()
		.table("users")
		.if_not_exists()
		.col(
			ColumnDef::new("id")
				.integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new("name").string_len(255).not_null())
		.col(
			ColumnDef::new("email")
				.string_len(255)
				.not_null()
				.unique_key(),
		)
		.col(ColumnDef::new("age").integer())
		.col(ColumnDef::new("active").boolean().default(true))
		.to_owned();

	let sql = create_table.build(PostgresQueryBuilder);
	sqlx::query(&sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create users table");

	pool
}

/// Products table fixture
#[fixture]
pub async fn products_table(#[future] pg_pool: Arc<PgPool>) -> Arc<PgPool> {
	let pool = pg_pool.await;

	let create_table = Table::create()
		.table("products")
		.if_not_exists()
		.col(
			ColumnDef::new("id")
				.integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new("name").string_len(255).not_null())
		.col(
			ColumnDef::new("sku")
				.string_len(100)
				.not_null()
				.unique_key(),
		)
		.col(ColumnDef::new("price").big_integer().not_null())
		.col(ColumnDef::new("stock").integer().not_null())
		.col(ColumnDef::new("available").boolean().default(true))
		.to_owned();

	let sql = create_table.build(PostgresQueryBuilder);
	sqlx::query(&sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create products table");

	pool
}

/// Orders table fixture (with foreign key to users)
#[fixture]
pub async fn orders_table(#[future] users_table: Arc<PgPool>) -> Arc<PgPool> {
	let pool = users_table.await;

	let create_table = Table::create()
		.table("orders")
		.if_not_exists()
		.col(
			ColumnDef::new("id")
				.integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new("user_id").integer().not_null())
		.col(ColumnDef::new("total_amount").big_integer().not_null())
		.col(ColumnDef::new("status").string_len(50).not_null())
		.foreign_key(
			ForeignKey::create()
				.name("fk_orders_user_id")
				.from("orders", "user_id")
				.to("users", "id")
				.on_delete(ForeignKeyAction::Cascade)
				.on_update(ForeignKeyAction::Cascade),
		)
		.to_owned();

	let sql = create_table.build(PostgresQueryBuilder);
	sqlx::query(&sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create orders table");

	pool
}

/// Users table with sample data fixture
#[fixture]
pub async fn users_with_data(#[future] users_table: Arc<PgPool>) -> (Arc<PgPool>, Vec<i32>) {
	let pool = users_table.await;

	// Insert sample users using SQL
	let id1: i32 = sqlx::query(
		"INSERT INTO users (name, email, age, active) VALUES ($1, $2, $3, $4) RETURNING id",
	)
	.bind("Alice")
	.bind("alice@example.com")
	.bind(30i32)
	.bind(true)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to insert user1")
	.get("id");

	let id2: i32 = sqlx::query(
		"INSERT INTO users (name, email, age, active) VALUES ($1, $2, $3, $4) RETURNING id",
	)
	.bind("Bob")
	.bind("bob@example.com")
	.bind(25i32)
	.bind(true)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to insert user2")
	.get("id");

	let id3: i32 = sqlx::query(
		"INSERT INTO users (name, email, age, active) VALUES ($1, $2, $3, $4) RETURNING id",
	)
	.bind("Charlie")
	.bind("charlie@example.com")
	.bind(35i32)
	.bind(true)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to insert user3")
	.get("id");

	(pool, vec![id1, id2, id3])
}

/// Products table with sample data fixture
#[fixture]
pub async fn products_with_data(#[future] products_table: Arc<PgPool>) -> (Arc<PgPool>, Vec<i32>) {
	let pool = products_table.await;

	// Insert sample products using SQL
	let id1: i32 = sqlx::query("INSERT INTO products (name, sku, price, stock, available) VALUES ($1, $2, $3, $4, $5) RETURNING id")
		.bind("Laptop")
		.bind("SKU-001")
		.bind(100000i64)
		.bind(10i32)
		.bind(true)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to insert product1")
		.get("id");

	let id2: i32 = sqlx::query("INSERT INTO products (name, sku, price, stock, available) VALUES ($1, $2, $3, $4, $5) RETURNING id")
		.bind("Mouse")
		.bind("SKU-002")
		.bind(2000i64)
		.bind(50i32)
		.bind(true)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to insert product2")
		.get("id");

	let id3: i32 = sqlx::query("INSERT INTO products (name, sku, price, stock, available) VALUES ($1, $2, $3, $4, $5) RETURNING id")
		.bind("Keyboard")
		.bind("SKU-003")
		.bind(5000i64)
		.bind(30i32)
		.bind(true)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to insert product3")
		.get("id");

	(pool, vec![id1, id2, id3])
}
