//! Integration test utilities for Reinhardt
//!
//! This crate provides common utilities for integration testing
//! across multiple Reinhardt crates with HTTP framework integration.

use bytes::Bytes;
use reinhardt_core::http::Request;
use reinhardt_core::types::Handler;
use sqlx::{Pool, Postgres};
// NOTE: AssertSqlSafe trait was removed in newer sqlx versions (v0.6+)
// This import is no longer needed as the trait is not used in tests
// Reference: https://github.com/launchbadge/sqlx/blob/main/CHANGELOG.md
// use sqlx::AssertSqlSafe;
use std::net::SocketAddr;
use std::sync::Arc;

// pub mod flatpages_app;
pub mod message_middleware_mock;
pub mod messages_helpers;

// New shared modules
// pub mod flatpages_common;
pub mod db_transaction;
pub mod proxy;
pub mod validator_test_common;

// Settings integration tests
#[cfg(test)]
mod settings;

/// Test database setup
pub async fn setup_test_db() -> Pool<Postgres> {
	let database_url = std::env::var("TEST_DATABASE_URL")
		.unwrap_or_else(|_| "postgres://localhost/reinhardt_integration_test".into());

	Pool::<Postgres>::connect(&database_url)
		.await
		.expect("Failed to connect to test database")
}

/// Create test tables for flatpages using reinhardt-migrations
// Note: Test helper function - may appear unused but available for flatpages integration tests
#[allow(dead_code)]
pub async fn create_flatpages_tables(pool: &Pool<Postgres>) {
	use reinhardt_db::migrations::{ColumnDefinition, Migration, Operation, SqlDialect};

	// Define flatpages migration with all required tables
	let flatpages_migration = Migration::new("test_flatpages_schema", "test")
		.add_operation(Operation::CreateTable {
			name: "flatpages".to_string(),
			columns: vec![
				ColumnDefinition::new("id", "BIGSERIAL PRIMARY KEY"),
				ColumnDefinition::new("url", "VARCHAR(255) NOT NULL UNIQUE"),
				ColumnDefinition::new("title", "VARCHAR(255) NOT NULL"),
				ColumnDefinition::new("content", "TEXT NOT NULL"),
				ColumnDefinition::new("enable_comments", "BOOLEAN NOT NULL DEFAULT FALSE"),
				ColumnDefinition::new("template_name", "VARCHAR(255)"),
				ColumnDefinition::new("registration_required", "BOOLEAN NOT NULL DEFAULT FALSE"),
				ColumnDefinition::new("created_at", "TIMESTAMPTZ NOT NULL DEFAULT NOW()"),
				ColumnDefinition::new("updated_at", "TIMESTAMPTZ NOT NULL DEFAULT NOW()"),
			],
			constraints: vec![],
		})
		.add_operation(Operation::CreateTable {
			name: "sites".to_string(),
			columns: vec![
				ColumnDefinition::new("id", "BIGSERIAL PRIMARY KEY"),
				ColumnDefinition::new("domain", "VARCHAR(255) NOT NULL UNIQUE"),
				ColumnDefinition::new("name", "VARCHAR(255) NOT NULL"),
			],
			constraints: vec![],
		})
		.add_operation(Operation::CreateTable {
			name: "flatpage_sites".to_string(),
			columns: vec![
				ColumnDefinition::new("id", "BIGSERIAL PRIMARY KEY"),
				ColumnDefinition::new("flatpage_id", "BIGINT NOT NULL"),
				ColumnDefinition::new("site_id", "BIGINT NOT NULL"),
			],
			constraints: vec![
				"FOREIGN KEY (flatpage_id) REFERENCES flatpages(id) ON DELETE CASCADE".to_string(),
				"FOREIGN KEY (site_id) REFERENCES sites(id) ON DELETE CASCADE".to_string(),
				"UNIQUE(flatpage_id, site_id)".to_string(),
			],
		});

	// Generate and execute SQL for each operation
	for operation in &flatpages_migration.operations {
		let sql = operation.to_sql(&SqlDialect::Postgres);
		sqlx::query(sql.as_str())
			.execute(pool)
			.await
			.expect("Failed to create table");
	}
}

/// Clean up test tables using reinhardt-migrations
// Note: Test helper function - may appear unused but available for test cleanup
#[allow(dead_code)]
pub async fn cleanup_test_tables(pool: &Pool<Postgres>) {
	use reinhardt_db::migrations::{Migration, Operation, SqlDialect};

	// Define cleanup migration - drop tables in reverse order
	let cleanup_migration = Migration::new("cleanup_test_schema", "test")
		.add_operation(Operation::DropTable {
			name: "flatpage_sites".to_string(),
		})
		.add_operation(Operation::DropTable {
			name: "flatpages".to_string(),
		})
		.add_operation(Operation::DropTable {
			name: "sites".to_string(),
		});

	// Generate and execute SQL for each operation
	for operation in &cleanup_migration.operations {
		let sql = operation.to_sql(&SqlDialect::Postgres);
		// Ignore errors during cleanup (tables might not exist)
		let _ = sqlx::query(sql.as_str()).execute(pool).await;
	}
}

/// Test server that can be spawned for integration tests
#[allow(dead_code)]
pub struct TestServer {
	pub addr: SocketAddr,
	pub pool: Pool<Postgres>,
}

#[allow(dead_code)]
impl TestServer {
	/// Create a new test server with the given router
	pub async fn new(_router: Arc<dyn Handler>) -> Self {
		let pool = setup_test_db().await;
		create_flatpages_tables(&pool).await;

		let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
			.await
			.expect("Failed to bind to address");
		let addr = listener.local_addr().expect("Failed to get local address");

		// Note: Server functionality temporarily disabled
		// until reinhardt-server provides a proper Server type

		// Give the server a moment to start
		tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

		Self { addr, pool }
	}

	/// Get the base URL for the test server
	pub fn url(&self, path: &str) -> String {
		format!("http://{}{}", self.addr, path)
	}
}

impl Drop for TestServer {
	fn drop(&mut self) {
		// Note: Cleanup is best-effort in Drop
		// For proper cleanup, use an explicit cleanup function
	}
}

pub use reinhardt_test::mock::SimpleHandler;

/// Helper to make HTTP requests in tests
pub async fn make_request(
	router: Arc<dyn Handler>,
	method: &str,
	uri: &str,
	body: Option<String>,
) -> (hyper::StatusCode, String) {
	make_request_with_headers(router, method, uri, body, vec![]).await
}

/// Helper to make HTTP requests with custom headers
pub async fn make_request_with_headers(
	router: Arc<dyn Handler>,
	method: &str,
	uri: &str,
	body: Option<String>,
	headers: Vec<(&str, &str)>,
) -> (hyper::StatusCode, String) {
	let method = method.parse::<hyper::Method>().expect("Invalid method");
	let uri = uri.parse::<hyper::Uri>().expect("Invalid URI");

	let mut header_map = hyper::HeaderMap::new();
	header_map.insert(
		hyper::header::CONTENT_TYPE,
		hyper::header::HeaderValue::from_static("application/json"),
	);

	// Add custom headers
	for (name, value) in headers {
		header_map.insert(
			name.parse::<hyper::header::HeaderName>().unwrap(),
			hyper::header::HeaderValue::from_str(value).unwrap(),
		);
	}

	let body_bytes = body.map(Bytes::from).unwrap_or_default();

	let request = Request::builder()
		.method(method)
		.uri(uri)
		.version(hyper::Version::HTTP_11)
		.headers(header_map)
		.body(body_bytes)
		.build()
		.expect("Failed to build request");

	let response = router
		.handle(request)
		.await
		.expect("Failed to execute request");

	let status = response.status;
	let body_text = String::from_utf8(response.body.to_vec()).expect("Invalid UTF-8 in response");

	(status, body_text)
}
