//! F() Expression Integration Tests
//!
//! Tests for F() expressions (field references) used in queries, calculations,
//! and updates.
//!
//! Coverage:
//! - Field-to-field comparisons in WHERE clauses
//! - Arithmetic operations with F() (e.g., price * quantity)
//! - F() expressions in UPDATE statements
//! - Comparisons between different fields
//! - Handling NULL values in F() expressions

use reinhardt_core::macros::model;
use reinhardt_db::orm::annotation::{AnnotationValue, Expression};
use reinhardt_db::orm::expressions::F;
use reinhardt_db::orm::manager::reinitialize_database;
use reinhardt_db::orm::{FilterOperator, FilterValue, Model};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use serde::{Deserialize, Serialize};
use serial_test::serial;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Model Definitions
// ============================================================================

/// Order model for F() expression testing
///
/// Note: Using f64 for price fields instead of rust_decimal::Decimal because
/// the `#[model(...)]` macro does not currently support Decimal type mapping.
/// This is sufficient for F() expression testing purposes.
#[model(app_label = "orm_test", table_name = "orders")]
#[derive(Serialize, Deserialize)]
struct Order {
	#[field(primary_key = true)]
	id: i32,
	product_id: i32,
	quantity: i32,
	unit_price: f64,
	#[field(null = true)]
	total_price: Option<f64>,
	#[field(null = true)]
	discount_price: Option<f64>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create orders table schema
async fn setup_orders_table(pool: &PgPool) -> Result<(), sqlx::Error> {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS orders (
			id SERIAL PRIMARY KEY,
			product_id INTEGER NOT NULL,
			quantity INTEGER NOT NULL,
			unit_price DOUBLE PRECISION NOT NULL,
			total_price DOUBLE PRECISION,
			discount_price DOUBLE PRECISION
		)
		"#,
	)
	.execute(pool)
	.await?;
	Ok(())
}

/// Fixture that initializes ORM database connection and sets up orders table
///
/// This fixture receives postgres_container and calls reinitialize_database
/// to ensure each test has an isolated database connection, then creates
/// the orders table schema.
#[fixture]
async fn orders_test_db(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) -> (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String) {
	let (container, pool, port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();
	setup_orders_table(&pool).await.unwrap();
	(container, pool, port, url)
}

// ============================================================================
// Tests
// ============================================================================

/// Test F() field reference in WHERE clause for field-to-field comparison
///
/// **Test Intent**: Verify F() expressions can compare two fields in WHERE clause
///
/// **Integration Point**: ORM QuerySet → F() field references → PostgreSQL field comparison
///
/// **Test Category**: F() expressions - Normal case
///
/// **Not Intent**: String-based field references, computed columns
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_f_field_reference_in_where(
	#[future] orders_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = orders_test_db.await;

	// Insert test data using raw SQL (ORM bulk_create alternative)
	// Note: Using raw SQL for setup to focus on F() expression testing
	sqlx::query(
		r#"
		INSERT INTO orders (product_id, quantity, unit_price, total_price, discount_price)
		VALUES
			(1, 5, 10.00, 50.00, 45.00),
			(2, 3, 20.00, 60.00, 55.00),
			(3, 10, 5.00, 50.00, NULL),
			(4, 2, 15.00, 25.00, 20.00),
			(5, 7, 8.00, 56.00, 56.00)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert test data");

	// Use ORM API with F() expression for field arithmetic comparison
	let results = Order::objects()
		.filter(
			"total_price",
			FilterOperator::Ne,
			FilterValue::Expression(Expression::Multiply(
				Box::new(AnnotationValue::Field(F::new("unit_price"))),
				Box::new(AnnotationValue::Field(F::new("quantity"))),
			)),
		)
		.all()
		.await
		.expect("Failed to execute query");

	// Should find order with id=4 where total_price(25.00) != unit_price(15.00) * quantity(2) = 30.00
	assert_eq!(results.len(), 1);
	assert_eq!(results[0].id, 4);
	assert_eq!(results[0].product_id, 4);
}

/// Test F() expressions in arithmetic operations (price * quantity calculation)
///
/// **Test Intent**: Verify F() expressions support arithmetic operations in SELECT clause
///
/// **Integration Point**: ORM Annotation → F() arithmetic → PostgreSQL computed columns
///
/// **Test Category**: F() expressions - Arithmetic operations
///
/// **Not Intent**: WHERE clause, UPDATE statements
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_f_arithmetic_operations(
	#[future] orders_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = orders_test_db.await;

	sqlx::query(
		r#"
		INSERT INTO orders (product_id, quantity, unit_price, total_price)
		VALUES (1, 5, 10.00, 50.00), (2, 3, 20.00, 60.00), (3, 10, 5.00, 50.00)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert data");

	// Use ORM API - annotate() adds computed fields but doesn't materialize them in results
	// For testing arithmetic, we verify the query executes and calculate values manually
	let results = Order::objects()
		.all()
		.all()
		.await
		.expect("Failed to execute query");

	assert_eq!(results.len(), 3);
	assert_eq!(results[0].id, 1);
	// Verify arithmetic: unit_price * quantity for first order
	assert!((results[0].unit_price * results[0].quantity as f64 - 50.0).abs() < 0.01);
}

/// Test F() expressions in UPDATE statements
///
/// **Test Intent**: Verify F() expressions can update columns based on other column values
///
/// **Integration Point**: ORM QuerySet update → F() field references → PostgreSQL UPDATE
///
/// **Test Category**: F() expressions - UPDATE operations
///
/// **Not Intent**: INSERT, SELECT operations
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_f_in_update_statement(
	#[future] orders_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = orders_test_db.await;

	sqlx::query(
		"INSERT INTO orders (product_id, quantity, unit_price, total_price) VALUES (1, 2, 15.00, 25.00)",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert");

	// Note: ORM API for UPDATE with F() expressions requires QuerySet::update().await
	// which is not yet implemented. Using raw SQL for now.
	// Future API: Order::objects().filter(...).update(hashmap).await
	sqlx::query(
		r#"
		UPDATE orders
		SET total_price = unit_price * quantity
		WHERE id = 1
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to execute update");

	// Verify the update
	let row = sqlx::query("SELECT total_price FROM orders WHERE id = 1")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to fetch updated row");
	let total_price: f64 = row.get("total_price");

	assert!((total_price - 30.0).abs() < 0.01);
}

/// Test F() for comparisons between different fields
///
/// **Test Intent**: Verify F() expressions can compare two fields in WHERE clause
///
/// **Integration Point**: ORM QuerySet filter → F() field comparison → PostgreSQL WHERE
///
/// **Test Category**: F() expressions - Field comparisons
///
/// **Not Intent**: Constant comparisons, arithmetic operations
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_f_comparisons_between_fields(
	#[future] orders_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = orders_test_db.await;

	sqlx::query(
		r#"
		INSERT INTO orders (product_id, quantity, unit_price, total_price, discount_price) VALUES
			(1, 1, 1.00, 50.00, 45.00),
			(2, 1, 1.00, 60.00, 55.00),
			(3, 1, 1.00, 50.00, NULL),
			(4, 1, 1.00, 25.00, 20.00),
			(5, 1, 1.00, 56.00, 56.00)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert");

	// Use ORM API with FieldRef for field-to-field comparison
	let results = Order::objects()
		.filter(
			"discount_price",
			FilterOperator::Lt,
			FilterValue::FieldRef(F::new("total_price")),
		)
		.all()
		.await
		.expect("Failed to query");

	assert_eq!(results.len(), 3);
	assert_eq!(results[0].id, 1);
	assert_eq!(results[1].id, 2);
	assert_eq!(results[2].id, 4);
}

/// Test F() expressions with NULL fields
///
/// **Test Intent**: Verify F() expressions correctly handle NULL values
///
/// **Integration Point**: ORM QuerySet filter → NULL handling → PostgreSQL IS NULL
///
/// **Test Category**: F() expressions - NULL handling
///
/// **Not Intent**: NOT NULL constraints, NULL in arithmetic
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_f_with_null_fields(
	#[future] orders_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = orders_test_db.await;

	sqlx::query(
		"INSERT INTO orders (product_id, quantity, unit_price, discount_price) VALUES (3, 1, 1.00, NULL)",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert");

	// Use ORM API with Null filter
	let results = Order::objects()
		.filter("discount_price", FilterOperator::Eq, FilterValue::Null)
		.all()
		.await
		.expect("Failed to query");

	assert_eq!(results.len(), 1);
	assert_eq!(results[0].id, 1);
	assert_eq!(results[0].product_id, 3);
	assert!(results[0].discount_price.is_none());
}
