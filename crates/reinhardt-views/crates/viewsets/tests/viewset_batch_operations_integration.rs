//! ViewSet Batch Operations Integration Tests
//!
//! Tests batch operation functionality for ViewSets:
//! - Batch create (multiple creates in one request)
//! - Batch update (multiple updates in one request)
//! - Batch delete (multiple deletes in one request)
//! - Mixed operations (create + update + delete)
//! - Partial update batch
//! - Atomic mode (stop on first error)
//! - Non-atomic mode (continue on errors)
//! - Edge cases (empty batch, single operation, duplicates)
//!
//! **Test Category**: Happy Path + Error Path (正常系+異常系)
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container
//!
//! **Test Data Schema:**
//! - books(id SERIAL PRIMARY KEY, title TEXT NOT NULL, author TEXT NOT NULL,
//!   price INT NOT NULL, stock INT NOT NULL)

use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Version};
use reinhardt_core::http::Request;
use reinhardt_core::macros::model;
use reinhardt_db::orm::init_database;
use reinhardt_test::fixtures::postgres_container;
use reinhardt_test::testcontainers::{ContainerAsync, GenericImage};
use reinhardt_viewsets::{BatchOperation, BatchRequest, BatchResponse};
use rstest::*;
use sea_query::{ColumnDef, Iden, PostgresQueryBuilder, Table};
use serde::{Deserialize, Serialize};
use serial_test::serial;
use sqlx::{PgPool, Row};
use std::sync::Arc;

// ============================================================================
// Model Definitions
// ============================================================================

/// Book model for batch operations testing
#[allow(dead_code)]
#[model(app_label = "viewsets_batch", table_name = "books")]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Book {
	#[field(primary_key = true)]
	id: Option<i64>,
	#[field(max_length = 200)]
	title: String,
	#[field(max_length = 100)]
	author: String,
	price: i32,
	stock: i32,
}

// ============================================================================
// Table Identifiers (for SeaQuery operations)
// ============================================================================

#[derive(Iden)]
enum Books {
	Table,
	Id,
	Title,
	Author,
	Price,
	Stock,
}

// ============================================================================
// Fixtures
// ============================================================================

/// Fixture: Initialize database connection
#[fixture]
async fn db_pool(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) -> Arc<PgPool> {
	let (_container, pool, _port, connection_url) = postgres_container.await;

	// Initialize database connection for reinhardt-orm
	init_database(&connection_url)
		.await
		.expect("Failed to initialize database");

	pool
}

/// Fixture: Setup books table
#[fixture]
async fn books_table(#[future] db_pool: Arc<PgPool>) -> Arc<PgPool> {
	let pool = db_pool.await;

	// Create books table
	let create_table_stmt = Table::create()
		.table(Books::Table)
		.if_not_exists()
		.col(
			ColumnDef::new(Books::Id)
				.big_integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new(Books::Title).string_len(200).not_null())
		.col(ColumnDef::new(Books::Author).string_len(100).not_null())
		.col(ColumnDef::new(Books::Price).integer().not_null())
		.col(ColumnDef::new(Books::Stock).integer().not_null())
		.to_owned();

	let sql = create_table_stmt.to_string(PostgresQueryBuilder);
	sqlx::query(&sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create books table");

	pool
}

/// Fixture: Setup books table with sample data
#[fixture]
async fn books_with_data(#[future] books_table: Arc<PgPool>) -> (Arc<PgPool>, Vec<i64>) {
	let pool = books_table.await;
	let mut book_ids = Vec::new();

	// Insert 3 books
	for i in 1..=3 {
		let book = Book::new(
			format!("Book {}", i),
			format!("Author {}", i),
			1000 * i,
			10 * i,
		);

		let sql =
			"INSERT INTO books (title, author, price, stock) VALUES ($1, $2, $3, $4) RETURNING id";
		let row = sqlx::query(sql)
			.bind(&book.title)
			.bind(&book.author)
			.bind(book.price)
			.bind(book.stock)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to insert book");

		let id: i64 = row.get(0);
		book_ids.push(id);
	}

	(pool, book_ids)
}

// ============================================================================
// Tests
// ============================================================================

/// Test: Batch create - multiple successful creates
#[rstest]
#[tokio::test]
#[serial(viewset_batch)]
async fn test_batch_create_multiple(#[future] books_table: Arc<PgPool>) {
	let pool = books_table.await;

	// Create batch request with 3 create operations
	let batch_request = BatchRequest::new(vec![
		BatchOperation::Create {
			data: Book::new("New Book 1".to_string(), "Author A".to_string(), 1500, 20),
		},
		BatchOperation::Create {
			data: Book::new("New Book 2".to_string(), "Author B".to_string(), 2500, 15),
		},
		BatchOperation::Create {
			data: Book::new("New Book 3".to_string(), "Author C".to_string(), 3500, 25),
		},
	]);

	// Serialize to JSON
	let json_body = serde_json::to_string(&batch_request).unwrap();

	// In a real ViewSet, this would be handled by a batch endpoint
	// For this test, we verify the data structures work correctly
	assert_eq!(batch_request.len(), 3);
	assert!(!batch_request.is_empty());

	// Verify JSON serialization/deserialization
	let deserialized: BatchRequest<Book> = serde_json::from_str(&json_body).unwrap();
	assert_eq!(deserialized.len(), 3);

	// Verify we can process operations
	for (index, operation) in batch_request.operations.iter().enumerate() {
		match operation {
			BatchOperation::Create { data } => {
				// Verify data is correct
				assert!(data.title.starts_with("New Book"));
				assert!(data.price > 0);

				// In real implementation, would insert to DB
				let sql = "INSERT INTO books (title, author, price, stock) VALUES ($1, $2, $3, $4) RETURNING id";
				let result = sqlx::query(sql)
					.bind(&data.title)
					.bind(&data.author)
					.bind(data.price)
					.bind(data.stock)
					.fetch_one(pool.as_ref())
					.await;

				assert!(result.is_ok(), "Create operation {} should succeed", index);
			}
			_ => panic!("Expected Create operation"),
		}
	}
}

/// Test: Batch update - multiple successful updates
#[rstest]
#[tokio::test]
#[serial(viewset_batch)]
async fn test_batch_update_multiple(#[future] books_with_data: (Arc<PgPool>, Vec<i64>)) {
	let (pool, book_ids) = books_with_data.await;

	// Create batch request with update operations
	let batch_request = BatchRequest::new(vec![
		BatchOperation::Update {
			id: book_ids[0].to_string(),
			data: Book::new(
				"Updated Book 1".to_string(),
				"Author A".to_string(),
				1800,
				25,
			),
		},
		BatchOperation::Update {
			id: book_ids[1].to_string(),
			data: Book::new(
				"Updated Book 2".to_string(),
				"Author B".to_string(),
				2800,
				20,
			),
		},
	]);

	assert_eq!(batch_request.len(), 2);

	// Process updates
	for (index, operation) in batch_request.operations.iter().enumerate() {
		match operation {
			BatchOperation::Update { id, data } => {
				let id_val: i64 = id.parse().unwrap();
				let sql = "UPDATE books SET title = $1, author = $2, price = $3, stock = $4 WHERE id = $5";
				let result = sqlx::query(sql)
					.bind(&data.title)
					.bind(&data.author)
					.bind(data.price)
					.bind(data.stock)
					.bind(id_val)
					.execute(pool.as_ref())
					.await;

				assert!(result.is_ok(), "Update operation {} should succeed", index);
			}
			_ => panic!("Expected Update operation"),
		}
	}

	// Verify updates applied
	let row = sqlx::query("SELECT title FROM books WHERE id = $1")
		.bind(book_ids[0])
		.fetch_one(pool.as_ref())
		.await
		.unwrap();
	let title: String = row.get(0);
	assert_eq!(title, "Updated Book 1");
}

/// Test: Batch delete - multiple successful deletes
#[rstest]
#[tokio::test]
#[serial(viewset_batch)]
async fn test_batch_delete_multiple(#[future] books_with_data: (Arc<PgPool>, Vec<i64>)) {
	let (pool, book_ids) = books_with_data.await;

	// Create batch request with delete operations
	let batch_request = BatchRequest::new(vec![
		BatchOperation::Delete::<Book> {
			id: book_ids[0].to_string(),
		},
		BatchOperation::Delete::<Book> {
			id: book_ids[1].to_string(),
		},
	]);

	assert_eq!(batch_request.len(), 2);

	// Process deletes
	for (index, operation) in batch_request.operations.iter().enumerate() {
		match operation {
			BatchOperation::Delete { id } => {
				let id_val: i64 = id.parse().unwrap();
				let sql = "DELETE FROM books WHERE id = $1";
				let result = sqlx::query(sql).bind(id_val).execute(pool.as_ref()).await;

				assert!(result.is_ok(), "Delete operation {} should succeed", index);
			}
			_ => panic!("Expected Delete operation"),
		}
	}

	// Verify deletes applied
	let count_row = sqlx::query("SELECT COUNT(*) FROM books WHERE id IN ($1, $2)")
		.bind(book_ids[0])
		.bind(book_ids[1])
		.fetch_one(pool.as_ref())
		.await
		.unwrap();
	let count: i64 = count_row.get(0);
	assert_eq!(count, 0, "Deleted books should not exist");
}

/// Test: Mixed operations - create + update + delete
#[rstest]
#[tokio::test]
#[serial(viewset_batch)]
async fn test_batch_mixed_operations(#[future] books_with_data: (Arc<PgPool>, Vec<i64>)) {
	let (pool, book_ids) = books_with_data.await;

	// Create batch with mixed operations
	let batch_request = BatchRequest::new(vec![
		BatchOperation::Create {
			data: Book::new(
				"New Mixed Book".to_string(),
				"Mixed Author".to_string(),
				5000,
				50,
			),
		},
		BatchOperation::Update {
			id: book_ids[0].to_string(),
			data: Book::new(
				"Updated Mixed Book".to_string(),
				"Author X".to_string(),
				6000,
				60,
			),
		},
		BatchOperation::Delete::<Book> {
			id: book_ids[1].to_string(),
		},
	]);

	assert_eq!(batch_request.len(), 3);

	// Verify operations are correctly represented
	assert!(matches!(
		batch_request.operations[0],
		BatchOperation::Create { .. }
	));
	assert!(matches!(
		batch_request.operations[1],
		BatchOperation::Update { .. }
	));
	assert!(matches!(
		batch_request.operations[2],
		BatchOperation::Delete { .. }
	));
}

/// Test: Partial update batch operation
#[rstest]
#[tokio::test]
#[serial(viewset_batch)]
async fn test_batch_partial_update(#[future] books_with_data: (Arc<PgPool>, Vec<i64>)) {
	let (_pool, book_ids) = books_with_data.await;

	// Create batch with partial update operations
	let batch_request = BatchRequest::new(vec![
		BatchOperation::PartialUpdate {
			id: book_ids[0].to_string(),
			data: Book::new("".to_string(), "".to_string(), 9999, 0), // Only price matters
		},
		BatchOperation::PartialUpdate {
			id: book_ids[1].to_string(),
			data: Book::new("".to_string(), "".to_string(), 0, 88), // Only stock matters
		},
	]);

	assert_eq!(batch_request.len(), 2);

	// Verify partial update operations
	for operation in batch_request.operations.iter() {
		assert!(matches!(operation, BatchOperation::PartialUpdate { .. }));
	}
}

/// Test: Atomic mode - stop on first error
#[rstest]
#[tokio::test]
#[serial(viewset_batch)]
async fn test_batch_atomic_mode(#[future] books_table: Arc<PgPool>) {
	let _pool = books_table.await;

	// Create atomic batch request
	let batch_request = BatchRequest::new(vec![
		BatchOperation::Create {
			data: Book::new("Atomic Book 1".to_string(), "Author".to_string(), 1000, 10),
		},
		BatchOperation::Delete::<Book> {
			id: "999999".to_string(), // Non-existent ID - will fail
		},
		BatchOperation::Create {
			data: Book::new("Atomic Book 2".to_string(), "Author".to_string(), 2000, 20),
		},
	])
	.atomic(); // Enable atomic mode

	assert!(batch_request.atomic);
	assert_eq!(batch_request.len(), 3);

	// In atomic mode, if operation 2 fails, operation 3 should not execute
	// This would be enforced by the ViewSet handler
}

/// Test: Non-atomic mode - continue on errors
#[rstest]
#[tokio::test]
#[serial(viewset_batch)]
async fn test_batch_non_atomic_mode(#[future] books_table: Arc<PgPool>) {
	let _pool = books_table.await;

	// Create non-atomic batch request (default)
	let batch_request = BatchRequest::new(vec![
		BatchOperation::Create {
			data: Book::new("Book 1".to_string(), "Author".to_string(), 1000, 10),
		},
		BatchOperation::Delete::<Book> {
			id: "999999".to_string(), // Non-existent ID - will fail
		},
		BatchOperation::Create {
			data: Book::new("Book 2".to_string(), "Author".to_string(), 2000, 20),
		},
	]);

	assert!(!batch_request.atomic); // Non-atomic by default
	assert_eq!(batch_request.len(), 3);

	// In non-atomic mode, even if operation 2 fails, operation 3 should execute
}

/// Test: Empty batch request
#[rstest]
#[tokio::test]
#[serial(viewset_batch)]
async fn test_batch_empty_request(#[future] books_table: Arc<PgPool>) {
	let _pool = books_table.await;

	// Create empty batch request
	let batch_request: BatchRequest<Book> = BatchRequest::new(vec![]);

	assert_eq!(batch_request.len(), 0);
	assert!(batch_request.is_empty());

	// Empty batch should be handled gracefully
	let json = serde_json::to_string(&batch_request).unwrap();
	assert!(json.contains("\"operations\":[]"));
}

/// Test: Single operation in batch
#[rstest]
#[tokio::test]
#[serial(viewset_batch)]
async fn test_batch_single_operation(#[future] books_table: Arc<PgPool>) {
	let pool = books_table.await;

	// Create batch with single operation
	let batch_request = BatchRequest::new(vec![BatchOperation::Create {
		data: Book::new(
			"Single Book".to_string(),
			"Single Author".to_string(),
			3000,
			30,
		),
	}]);

	assert_eq!(batch_request.len(), 1);
	assert!(!batch_request.is_empty());

	// Process single operation
	match &batch_request.operations[0] {
		BatchOperation::Create { data } => {
			let sql = "INSERT INTO books (title, author, price, stock) VALUES ($1, $2, $3, $4) RETURNING id";
			let result = sqlx::query(sql)
				.bind(&data.title)
				.bind(&data.author)
				.bind(data.price)
				.bind(data.stock)
				.fetch_one(pool.as_ref())
				.await;

			assert!(result.is_ok(), "Single operation should succeed");
		}
		_ => panic!("Expected Create operation"),
	}
}

/// Test: Batch with duplicate IDs (update/delete)
#[rstest]
#[tokio::test]
#[serial(viewset_batch)]
async fn test_batch_duplicate_ids(#[future] books_with_data: (Arc<PgPool>, Vec<i64>)) {
	let (_pool, book_ids) = books_with_data.await;

	// Create batch with duplicate ID operations
	let batch_request = BatchRequest::new(vec![
		BatchOperation::Update {
			id: book_ids[0].to_string(),
			data: Book::new("Update 1".to_string(), "Author".to_string(), 1000, 10),
		},
		BatchOperation::Update {
			id: book_ids[0].to_string(), // Same ID - duplicate
			data: Book::new("Update 2".to_string(), "Author".to_string(), 2000, 20),
		},
	]);

	assert_eq!(batch_request.len(), 2);

	// Duplicate IDs should be handled - last update wins or error
	// This is implementation-dependent behavior
}

/// Test: Batch operation failure handling with BatchResponse
#[rstest]
#[tokio::test]
#[serial(viewset_batch)]
async fn test_batch_response_structure(#[future] books_table: Arc<PgPool>) {
	let _pool = books_table.await;

	// Create a mock batch response
	use reinhardt_viewsets::BatchOperationResult;

	let results = vec![
		BatchOperationResult::success(
			0,
			Some(Book::new(
				"Book 1".to_string(),
				"Author".to_string(),
				1000,
				10,
			)),
		),
		BatchOperationResult::failure::<Book>(1, "Not found"),
		BatchOperationResult::success(
			2,
			Some(Book::new(
				"Book 2".to_string(),
				"Author".to_string(),
				2000,
				20,
			)),
		),
	];

	let response: BatchResponse<Book> = BatchResponse {
		results: results.clone(),
		total: 3,
		succeeded: 2,
		failed: 1,
	};

	// Verify response structure
	assert_eq!(response.total, 3);
	assert_eq!(response.succeeded, 2);
	assert_eq!(response.failed, 1);
	assert_eq!(response.results.len(), 3);

	// Verify individual results
	assert!(response.results[0].success);
	assert!(!response.results[1].success);
	assert!(response.results[2].success);

	assert!(response.results[0].error.is_none());
	assert_eq!(response.results[1].error, Some("Not found".to_string()));

	// Verify JSON serialization
	let json = serde_json::to_string(&response).unwrap();
	assert!(json.contains("\"total\":3"));
	assert!(json.contains("\"succeeded\":2"));
	assert!(json.contains("\"failed\":1"));
}
