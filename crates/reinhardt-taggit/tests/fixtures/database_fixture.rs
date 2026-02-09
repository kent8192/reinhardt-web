//! Database fixture for reinhardt-taggit tests
//!
//! Provides a clean database with taggit schema for each test.

use reinhardt_db::backends::DatabaseConnection;
use reinhardt_taggit::{Tag, TaggedItem};
use reinhardt_test::fixtures::{
	ModelSchemaInfo, create_tables_for_models,
	testcontainers::{ContainerAsync, GenericImage, postgres_container},
};
use rstest::fixture;
use std::sync::Arc;

/// PostgreSQL container with taggit schema (tags + tagged_items tables)
///
/// This fixture provides a fresh PostgreSQL database with the taggit
/// schema created. Each test gets an isolated database instance.
///
/// # Returns
///
/// Tuple of (container, pool, database_connection)
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_taggit_tests::fixtures::taggit_db;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_tag_creation(
///     #[future] taggit_db: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, DatabaseConnection),
/// ) {
///     let (_container, pool, db) = taggit_db.await;
///     // Use pool for raw SQL or db for ORM operations
/// }
/// ```
#[fixture]
pub async fn taggit_db(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) -> (
	ContainerAsync<GenericImage>,
	Arc<sqlx::PgPool>,
	DatabaseConnection,
) {
	let (container, pool, _port, url) = postgres_container.await;

	// Connect via reinhardt-db
	let connection = DatabaseConnection::connect_postgres(&url)
		.await
		.expect("Failed to connect to PostgreSQL");

	// Create taggit schema from model metadata
	let model_infos = vec![
		ModelSchemaInfo::from_model::<Tag>(),
		ModelSchemaInfo::from_model::<TaggedItem>(),
	];

	create_tables_for_models(&connection, model_infos)
		.await
		.expect("Failed to create taggit schema");

	(container, pool, connection)
}
