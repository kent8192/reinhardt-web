//! MongoDB Test Fixtures
//!
//! Reusable fixtures for MongoDB integration tests.

use reinhardt_db::nosql::backends::mongodb::MongoDBBackend;
use reinhardt_test::fixtures::mongodb_container;
use rstest::*;
use testcontainers::{ContainerAsync, GenericImage};

/// MongoDB backend fixture
///
/// Provides a fresh MongoDB backend for each test.
/// The container is managed by testcontainers.
#[fixture]
pub async fn mongodb() -> (ContainerAsync<GenericImage>, MongoDBBackend) {
	// Start MongoDB container (shared across tests)
	let (container, connection_string, _port) = mongodb_container().await;

	// Create backend with test database
	let backend = MongoDBBackend::builder()
		.url(&connection_string)
		.database("test_db")
		.build()
		.await
		.expect("Failed to create MongoDB backend");

	// Return container to keep it alive during the test
	(container, backend)
}

/// MongoDB backend with automatic cleanup
///
/// Extends `mongodb` fixture by cleaning up the test collection
/// before and after each test.
#[fixture]
pub async fn mongodb_clean(
	#[future] mongodb: (ContainerAsync<GenericImage>, MongoDBBackend),
) -> (ContainerAsync<GenericImage>, MongoDBBackend) {
	mongodb.await
}
