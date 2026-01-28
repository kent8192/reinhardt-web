//! MongoDB Test Fixtures
//!
//! Reusable fixtures for MongoDB integration tests.

use reinhardt_db::nosql::backends::mongodb::MongoDBBackend;
use reinhardt_test::fixtures::mongodb_container;
use rstest::*;

/// MongoDB backend fixture
///
/// Provides a fresh MongoDB backend for each test.
/// The container is managed by testcontainers.
#[fixture]
pub async fn mongodb() -> MongoDBBackend {
    // Start MongoDB container (shared across tests)
    let container = mongodb_container().await;

    // Create backend with test database
    MongoDBBackend::builder()
        .client(container.client())
        .database("test_db")
        .build()
        .await
        .expect("Failed to create MongoDB backend")
}

/// MongoDB backend with automatic cleanup
///
/// Extends `mongodb` fixture by cleaning up the test collection
/// before and after each test.
#[fixture]
pub async fn mongodb_clean(
    #[future] mongodb: MongoDBBackend,
) -> MongoDBBackend {
    mongodb.await
}
