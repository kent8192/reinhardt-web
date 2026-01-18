//! ContentType Model Integration Tests
//!
//! These tests verify ContentType model operations with real PostgreSQL database.
//!
//! **Test Coverage:**
//! - ContentType model CRUD operations
//! - ContentType uniqueness constraints (app_label + model)
//! - ContentType metadata storage and retrieval
//! - ContentType query operations
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container

use ::testcontainers::{ContainerAsync, GenericImage};
use reinhardt_db::orm::manager::reinitialize_database;
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sqlx::{PgPool, Row};
use std::sync::Arc;

// ============================================================================
// ContentType CRUD Operations Tests
// ============================================================================

/// Test ContentType table creation and structure
///
/// **Test Intent**: Verify ContentType table can be created with correct schema
/// (id, app_label, model columns with UNIQUE constraint)
///
/// **Integration Point**: ContentType model → PostgreSQL table schema
///
/// **Not Intent**: Content type registration, permission system
#[rstest]
#[tokio::test]
async fn test_contenttype_table_creation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create ContentType table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS contenttypes_contenttype (
			id SERIAL PRIMARY KEY,
			app_label VARCHAR(100) NOT NULL,
			model VARCHAR(100) NOT NULL,
			UNIQUE(app_label, model)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create ContentType table");

	// Verify table exists
	let exists: bool = sqlx::query_scalar(
		r#"
		SELECT EXISTS (
			SELECT FROM information_schema.tables
			WHERE table_name = 'contenttypes_contenttype'
		)
		"#,
	)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to check table existence");

	assert!(exists, "ContentType table should exist");
}

/// Test ContentType creation (INSERT)
///
/// **Test Intent**: Verify ContentType records can be inserted with app_label and model
///
/// **Integration Point**: ContentType model → PostgreSQL INSERT
///
/// **Not Intent**: Uniqueness validation, content type registration
#[rstest]
#[tokio::test]
async fn test_contenttype_creation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create table
	sqlx::query(
		r#"
		CREATE TABLE contenttypes_contenttype (
			id SERIAL PRIMARY KEY,
			app_label VARCHAR(100) NOT NULL,
			model VARCHAR(100) NOT NULL,
			UNIQUE(app_label, model)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert ContentType
	let id: i32 = sqlx::query_scalar(
		r#"
		INSERT INTO contenttypes_contenttype (app_label, model)
		VALUES ($1, $2)
		RETURNING id
		"#,
	)
	.bind("auth")
	.bind("user")
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to insert ContentType");

	assert!(id > 0, "Inserted ContentType should have positive ID");

	// Verify inserted data
	let result = sqlx::query("SELECT app_label, model FROM contenttypes_contenttype WHERE id = $1")
		.bind(id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to fetch ContentType");

	let app_label: String = result.get("app_label");
	let model: String = result.get("model");

	assert_eq!(app_label, "auth");
	assert_eq!(model, "user");
}

/// Test ContentType retrieval by ID (SELECT)
///
/// **Test Intent**: Verify ContentType can be retrieved by primary key
///
/// **Integration Point**: ContentType model → PostgreSQL SELECT by ID
///
/// **Not Intent**: Query optimization, bulk retrieval
#[rstest]
#[tokio::test]
async fn test_contenttype_retrieval_by_id(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create table and insert test data
	sqlx::query(
		r#"
		CREATE TABLE contenttypes_contenttype (
			id SERIAL PRIMARY KEY,
			app_label VARCHAR(100) NOT NULL,
			model VARCHAR(100) NOT NULL,
			UNIQUE(app_label, model)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	let id: i32 = sqlx::query_scalar(
		"INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2) RETURNING id",
	)
	.bind("auth")
	.bind("user")
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Retrieve ContentType by ID
	let result =
		sqlx::query("SELECT id, app_label, model FROM contenttypes_contenttype WHERE id = $1")
			.bind(id)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to retrieve ContentType by ID");

	let retrieved_id: i32 = result.get("id");
	let app_label: String = result.get("app_label");
	let model: String = result.get("model");

	assert_eq!(retrieved_id, id);
	assert_eq!(app_label, "auth");
	assert_eq!(model, "user");
}

/// Test ContentType update (UPDATE)
///
/// **Test Intent**: Verify ContentType model field can be updated
///
/// **Integration Point**: ContentType model → PostgreSQL UPDATE
///
/// **Not Intent**: Bulk updates, cascading updates
#[rstest]
#[tokio::test]
async fn test_contenttype_update(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create table and insert test data
	sqlx::query(
		r#"
		CREATE TABLE contenttypes_contenttype (
			id SERIAL PRIMARY KEY,
			app_label VARCHAR(100) NOT NULL,
			model VARCHAR(100) NOT NULL,
			UNIQUE(app_label, model)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	let id: i32 = sqlx::query_scalar(
		"INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2) RETURNING id",
	)
	.bind("auth")
	.bind("user")
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Update model field
	sqlx::query("UPDATE contenttypes_contenttype SET model = $1 WHERE id = $2")
		.bind("customuser")
		.bind(id)
		.execute(pool.as_ref())
		.await
		.expect("Failed to update ContentType");

	// Verify update
	let model: String =
		sqlx::query_scalar("SELECT model FROM contenttypes_contenttype WHERE id = $1")
			.bind(id)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to fetch updated ContentType");

	assert_eq!(model, "customuser");
}

/// Test ContentType deletion (DELETE)
///
/// **Test Intent**: Verify ContentType can be deleted from database
///
/// **Integration Point**: ContentType model → PostgreSQL DELETE
///
/// **Not Intent**: Cascade deletion, soft delete
#[rstest]
#[tokio::test]
async fn test_contenttype_deletion(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create table and insert test data
	sqlx::query(
		r#"
		CREATE TABLE contenttypes_contenttype (
			id SERIAL PRIMARY KEY,
			app_label VARCHAR(100) NOT NULL,
			model VARCHAR(100) NOT NULL,
			UNIQUE(app_label, model)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	let id: i32 = sqlx::query_scalar(
		"INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2) RETURNING id",
	)
	.bind("auth")
	.bind("user")
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Delete ContentType
	sqlx::query("DELETE FROM contenttypes_contenttype WHERE id = $1")
		.bind(id)
		.execute(pool.as_ref())
		.await
		.expect("Failed to delete ContentType");

	// Verify deletion
	let count: i64 =
		sqlx::query_scalar("SELECT COUNT(*) FROM contenttypes_contenttype WHERE id = $1")
			.bind(id)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to count ContentType");

	assert_eq!(count, 0, "ContentType should be deleted");
}

// ============================================================================
// ContentType Uniqueness Constraint Tests
// ============================================================================

/// Test ContentType uniqueness constraint (app_label, model)
///
/// **Test Intent**: Verify database enforces UNIQUE constraint on (app_label, model) pair
///
/// **Integration Point**: ContentType model → PostgreSQL UNIQUE constraint
///
/// **Not Intent**: Application-level validation, content type registry
#[rstest]
#[tokio::test]
async fn test_contenttype_uniqueness_constraint(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create table with UNIQUE constraint
	sqlx::query(
		r#"
		CREATE TABLE contenttypes_contenttype (
			id SERIAL PRIMARY KEY,
			app_label VARCHAR(100) NOT NULL,
			model VARCHAR(100) NOT NULL,
			UNIQUE(app_label, model)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert first ContentType
	sqlx::query("INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2)")
		.bind("auth")
		.bind("user")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert first ContentType");

	// Attempt to insert duplicate (same app_label + model)
	let result =
		sqlx::query("INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2)")
			.bind("auth")
			.bind("user")
			.execute(pool.as_ref())
			.await;

	assert!(
		result.is_err(),
		"Duplicate ContentType should fail due to UNIQUE constraint"
	);

	// Verify error is unique violation
	let error = result.unwrap_err();
	let error_string = error.to_string().to_lowercase();
	assert!(
		error_string.contains("unique") || error_string.contains("duplicate"),
		"Error should indicate unique constraint violation, got: {}",
		error
	);
}

/// Test ContentType allows different models for same app
///
/// **Test Intent**: Verify multiple models can exist for the same app_label
///
/// **Integration Point**: ContentType model → PostgreSQL UNIQUE constraint behavior
///
/// **Not Intent**: Model registration, schema generation
#[rstest]
#[tokio::test]
async fn test_contenttype_multiple_models_same_app(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create table
	sqlx::query(
		r#"
		CREATE TABLE contenttypes_contenttype (
			id SERIAL PRIMARY KEY,
			app_label VARCHAR(100) NOT NULL,
			model VARCHAR(100) NOT NULL,
			UNIQUE(app_label, model)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert multiple models for "auth" app
	sqlx::query("INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2)")
		.bind("auth")
		.bind("user")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert auth.user");

	sqlx::query("INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2)")
		.bind("auth")
		.bind("group")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert auth.group");

	sqlx::query("INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2)")
		.bind("auth")
		.bind("permission")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert auth.permission");

	// Verify all three were inserted
	let count: i64 =
		sqlx::query_scalar("SELECT COUNT(*) FROM contenttypes_contenttype WHERE app_label = $1")
			.bind("auth")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to count ContentTypes");

	assert_eq!(count, 3, "Should have 3 models for auth app");
}

/// Test ContentType allows same model name for different apps
///
/// **Test Intent**: Verify same model name can exist across different apps
///
/// **Integration Point**: ContentType model → PostgreSQL UNIQUE constraint scoping
///
/// **Not Intent**: Model naming conventions, app isolation
#[rstest]
#[tokio::test]
async fn test_contenttype_same_model_different_apps(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create table
	sqlx::query(
		r#"
		CREATE TABLE contenttypes_contenttype (
			id SERIAL PRIMARY KEY,
			app_label VARCHAR(100) NOT NULL,
			model VARCHAR(100) NOT NULL,
			UNIQUE(app_label, model)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert "user" model for different apps
	sqlx::query("INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2)")
		.bind("auth")
		.bind("user")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert auth.user");

	sqlx::query("INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2)")
		.bind("accounts")
		.bind("user")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert accounts.user");

	sqlx::query("INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2)")
		.bind("profiles")
		.bind("user")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert profiles.user");

	// Verify all three were inserted
	let count: i64 =
		sqlx::query_scalar("SELECT COUNT(*) FROM contenttypes_contenttype WHERE model = $1")
			.bind("user")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to count ContentTypes");

	assert_eq!(count, 3, "Should have 3 apps with 'user' model");
}

// ============================================================================
// ContentType Query Operations Tests
// ============================================================================

/// Test ContentType retrieval by app_label
///
/// **Test Intent**: Verify ContentType can be queried by app_label
///
/// **Integration Point**: ContentType model → PostgreSQL WHERE clause filtering
///
/// **Not Intent**: Complex queries, JOIN operations
#[rstest]
#[tokio::test]
async fn test_contenttype_query_by_app_label(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create table and insert test data
	sqlx::query(
		r#"
		CREATE TABLE contenttypes_contenttype (
			id SERIAL PRIMARY KEY,
			app_label VARCHAR(100) NOT NULL,
			model VARCHAR(100) NOT NULL,
			UNIQUE(app_label, model)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert multiple ContentTypes for different apps
	for (app, model) in [
		("auth", "user"),
		("auth", "group"),
		("contenttypes", "contenttype"),
	] {
		sqlx::query("INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2)")
			.bind(app)
			.bind(model)
			.execute(pool.as_ref())
			.await
			.unwrap();
	}

	// Query ContentTypes for "auth" app
	let results = sqlx::query("SELECT model FROM contenttypes_contenttype WHERE app_label = $1")
		.bind("auth")
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to query ContentTypes by app_label");

	let models: Vec<String> = results.iter().map(|row| row.get("model")).collect();

	assert_eq!(models.len(), 2);
	assert!(models.contains(&"user".to_string()));
	assert!(models.contains(&"group".to_string()));
}

/// Test ContentType retrieval by model name
///
/// **Test Intent**: Verify ContentType can be queried by model field
///
/// **Integration Point**: ContentType model → PostgreSQL WHERE clause filtering
///
/// **Not Intent**: Full-text search, pattern matching
#[rstest]
#[tokio::test]
async fn test_contenttype_query_by_model(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create table and insert test data
	sqlx::query(
		r#"
		CREATE TABLE contenttypes_contenttype (
			id SERIAL PRIMARY KEY,
			app_label VARCHAR(100) NOT NULL,
			model VARCHAR(100) NOT NULL,
			UNIQUE(app_label, model)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert ContentTypes with same model name for different apps
	for (app, model) in [
		("auth", "user"),
		("accounts", "user"),
		("profiles", "profile"),
	] {
		sqlx::query("INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2)")
			.bind(app)
			.bind(model)
			.execute(pool.as_ref())
			.await
			.unwrap();
	}

	// Query ContentTypes by model name "user"
	let results = sqlx::query("SELECT app_label FROM contenttypes_contenttype WHERE model = $1")
		.bind("user")
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to query ContentTypes by model");

	let apps: Vec<String> = results.iter().map(|row| row.get("app_label")).collect();

	assert_eq!(apps.len(), 2);
	assert!(apps.contains(&"auth".to_string()));
	assert!(apps.contains(&"accounts".to_string()));
}

/// Test ContentType retrieval by exact match (app_label, model)
///
/// **Test Intent**: Verify ContentType can be queried by exact (app_label, model) pair
///
/// **Integration Point**: ContentType model → PostgreSQL composite WHERE clause
///
/// **Not Intent**: Partial matching, case-insensitive search
#[rstest]
#[tokio::test]
async fn test_contenttype_query_by_exact_match(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create table and insert test data
	sqlx::query(
		r#"
		CREATE TABLE contenttypes_contenttype (
			id SERIAL PRIMARY KEY,
			app_label VARCHAR(100) NOT NULL,
			model VARCHAR(100) NOT NULL,
			UNIQUE(app_label, model)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert multiple ContentTypes
	for (app, model) in [("auth", "user"), ("auth", "group"), ("accounts", "user")] {
		sqlx::query("INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2)")
			.bind(app)
			.bind(model)
			.execute(pool.as_ref())
			.await
			.unwrap();
	}

	// Query by exact (app_label, model) match
	let result =
		sqlx::query("SELECT id FROM contenttypes_contenttype WHERE app_label = $1 AND model = $2")
			.bind("auth")
			.bind("user")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to query ContentType by exact match");

	let id: i32 = result.get("id");
	assert!(id > 0, "Should find exact ContentType match");

	// Verify only one result for exact match
	let count: i64 = sqlx::query_scalar(
		"SELECT COUNT(*) FROM contenttypes_contenttype WHERE app_label = $1 AND model = $2",
	)
	.bind("auth")
	.bind("user")
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to count exact matches");

	assert_eq!(count, 1, "Should have exactly one match for (auth, user)");
}

/// Test ContentType list all operation
///
/// **Test Intent**: Verify all ContentTypes can be retrieved
///
/// **Integration Point**: ContentType model → PostgreSQL SELECT all
///
/// **Not Intent**: Pagination, ordering
#[rstest]
#[tokio::test]
async fn test_contenttype_list_all(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create table and insert test data
	sqlx::query(
		r#"
		CREATE TABLE contenttypes_contenttype (
			id SERIAL PRIMARY KEY,
			app_label VARCHAR(100) NOT NULL,
			model VARCHAR(100) NOT NULL,
			UNIQUE(app_label, model)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert multiple ContentTypes
	let test_data = vec![
		("auth", "user"),
		("auth", "group"),
		("auth", "permission"),
		("contenttypes", "contenttype"),
		("sessions", "session"),
	];

	for (app, model) in &test_data {
		sqlx::query("INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2)")
			.bind(app)
			.bind(model)
			.execute(pool.as_ref())
			.await
			.unwrap();
	}

	// List all ContentTypes
	let results = sqlx::query("SELECT app_label, model FROM contenttypes_contenttype")
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to list all ContentTypes");

	assert_eq!(results.len(), test_data.len());

	// Verify all inserted data is present
	for (expected_app, expected_model) in &test_data {
		let found = results.iter().any(|row| {
			let app: String = row.get("app_label");
			let model: String = row.get("model");
			&app == expected_app && &model == expected_model
		});
		assert!(
			found,
			"ContentType ({}, {}) should be in results",
			expected_app, expected_model
		);
	}
}
