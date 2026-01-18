//! GenericForeignKey Integration Tests
//!
//! These tests verify GenericForeignKey functionality with real PostgreSQL database,
//! including constraint validation, content type lookups, and relationship operations.
//!
//! **Test Coverage:**
//! - GenericForeignKey creation and basic operations
//! - Content type validation with database constraints
//! - GenericRelation reverse access
//! - Cross-model generic relationships
//! - Query operations across content types
//! - Foreign key constraint validation
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container

use reinhardt_db::contenttypes::{
	ContentType, ContentTypePersistenceBackend, generic_fk::GenericForeignKeyField,
	persistence::ContentTypePersistence,
};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

#[cfg(feature = "database")]
use reinhardt_db::contenttypes::generic_fk::constraints::GenericForeignKeyConstraints;

// ============================================================================
// Test Fixtures
// ============================================================================

/// Initialize sqlx::any drivers (required for AnyPool usage)
#[fixture]
fn init_drivers() {
	sqlx::any::install_default_drivers();
}

// ============================================================================
// GenericForeignKey Basic Operations Tests
// ============================================================================

/// Test GenericForeignKey creation and basic field operations
///
/// **Test Intent**: Verify GenericForeignKey can be created and basic field
/// operations (set, get, is_set, clear) work correctly
///
/// **Integration Point**: GenericForeignKeyField → ContentType
///
/// **Not Intent**: Database operations, constraint validation
#[rstest]
#[tokio::test]
async fn test_gfk_basic_operations(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	// Create ContentType
	let ct = ContentType::new("blog", "Post").with_id(1);

	// Create GenericForeignKey
	let mut gfk = GenericForeignKeyField::new();
	assert!(!gfk.is_set());

	// Set GFK to point to content type + object
	gfk.set(&ct, 42);
	assert!(gfk.is_set());
	assert_eq!(gfk.content_type_id(), Some(1));
	assert_eq!(gfk.object_id(), Some(42));

	// Clear GFK
	gfk.clear();
	assert!(!gfk.is_set());
	assert_eq!(gfk.content_type_id(), None);
	assert_eq!(gfk.object_id(), None);
}

/// Test GenericForeignKey with_values constructor
///
/// **Test Intent**: Verify GenericForeignKey can be created with initial values
/// using the with_values constructor
///
/// **Integration Point**: GenericForeignKeyField::with_values
///
/// **Not Intent**: Database operations, content type validation
#[rstest]
#[tokio::test]
async fn test_gfk_with_values(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	// Create GFK with initial values
	let gfk = GenericForeignKeyField::with_values(Some(5), Some(99));

	assert!(gfk.is_set());
	assert_eq!(gfk.content_type_id(), Some(5));
	assert_eq!(gfk.object_id(), Some(99));
}

/// Test GenericForeignKey partial set detection
///
/// **Test Intent**: Verify is_set() correctly detects when GFK is only
/// partially set (only content_type_id OR only object_id)
///
/// **Integration Point**: GenericForeignKeyField::is_set
///
/// **Not Intent**: Full GFK operations, database validation
#[rstest]
#[tokio::test]
async fn test_gfk_partial_set(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let mut gfk = GenericForeignKeyField::new();

	// Only set content_type_id
	gfk.set_content_type_id(Some(10));
	assert!(
		!gfk.is_set(),
		"GFK with only content_type_id should not be set"
	);

	// Only set object_id
	let mut gfk2 = GenericForeignKeyField::new();
	gfk2.set_object_id(Some(20));
	assert!(!gfk2.is_set(), "GFK with only object_id should not be set");

	// Set both
	gfk.set_object_id(Some(20));
	assert!(gfk.is_set(), "GFK with both fields should be set");
}

// ============================================================================
// Database Constraint Validation Tests
// ============================================================================

/// Test GenericForeignKey content type validation with database
///
/// **Test Intent**: Verify GenericForeignKey can validate that content_type_id
/// references an existing ContentType in the database
///
/// **Integration Point**: GenericForeignKeyConstraints::validate_content_type
///
/// **Not Intent**: Object existence validation, data integrity beyond content type
#[cfg(feature = "database")]
#[rstest]
#[tokio::test]
async fn test_gfk_validate_content_type(
	_init_drivers: (),
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	// Create AnyPool from connection URL for database-agnostic operations
	let any_pool = Arc::new(
		sqlx::AnyPool::connect(&url)
			.await
			.expect("Failed to connect to AnyPool"),
	);

	let persistence = ContentTypePersistence::from_pool(any_pool, &url);
	persistence
		.create_table()
		.await
		.expect("Failed to create ContentType table");

	// Create and save a ContentType
	let ct = persistence
		.save(&ContentType::new("auth", "User"))
		.await
		.expect("Failed to save ContentType");
	let ct_id = ct.id.unwrap();

	// Create GFK pointing to valid content type
	let gfk = GenericForeignKeyField::with_values(Some(ct_id), Some(123));

	// Should validate successfully
	let is_valid = gfk
		.validate_content_type(&persistence)
		.await
		.expect("Failed to validate");
	assert!(is_valid, "GFK with valid content type should validate");

	// Create GFK pointing to non-existent content type
	let invalid_gfk = GenericForeignKeyField::with_values(Some(9999), Some(456));
	let is_valid = invalid_gfk
		.validate_content_type(&persistence)
		.await
		.expect("Failed to validate");
	assert!(
		!is_valid,
		"GFK with invalid content type should not validate"
	);
}

/// Test GenericForeignKey retrieval of validated content type from database
///
/// **Test Intent**: Verify GenericForeignKey can retrieve the full ContentType
/// object from database after validation
///
/// **Integration Point**: GenericForeignKeyConstraints::get_validated_content_type
///
/// **Not Intent**: Content type creation, registry operations
#[cfg(feature = "database")]
#[rstest]
#[tokio::test]
async fn test_gfk_get_validated_content_type(
	_init_drivers: (),
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	// Create AnyPool from connection URL for database-agnostic operations
	let any_pool = Arc::new(
		sqlx::AnyPool::connect(&url)
			.await
			.expect("Failed to connect to AnyPool"),
	);

	let persistence = ContentTypePersistence::from_pool(any_pool, &url);
	persistence
		.create_table()
		.await
		.expect("Failed to create ContentType table");

	// Create and save a ContentType
	let ct = persistence
		.save(&ContentType::new("shop", "Product"))
		.await
		.expect("Failed to save ContentType");
	let ct_id = ct.id.unwrap();

	// Create GFK pointing to it
	let gfk = GenericForeignKeyField::with_values(Some(ct_id), Some(789));

	// Retrieve validated content type
	let validated_ct = gfk
		.get_validated_content_type(&persistence)
		.await
		.expect("Failed to get validated content type");

	assert!(validated_ct.is_some(), "Should retrieve content type");
	let validated_ct = validated_ct.unwrap();
	assert_eq!(validated_ct.app_label, "shop");
	assert_eq!(validated_ct.model, "Product");
	assert_eq!(validated_ct.id, Some(ct_id));
}

/// Test GenericForeignKey validation with unset GFK
///
/// **Test Intent**: Verify that an unset GenericForeignKey (no content_type_id)
/// fails validation correctly
///
/// **Integration Point**: GenericForeignKeyConstraints::validate_content_type with unset GFK
///
/// **Not Intent**: Valid GFK validation, partial GFK validation
#[cfg(feature = "database")]
#[rstest]
#[tokio::test]
async fn test_gfk_validate_unset(
	_init_drivers: (),
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	// Create AnyPool from connection URL for database-agnostic operations
	let any_pool = Arc::new(
		sqlx::AnyPool::connect(&url)
			.await
			.expect("Failed to connect to AnyPool"),
	);

	let persistence = ContentTypePersistence::from_pool(any_pool, &url);
	persistence
		.create_table()
		.await
		.expect("Failed to create ContentType table");

	// Create unset GFK
	let gfk = GenericForeignKeyField::new();

	// Should not validate
	let is_valid = gfk
		.validate_content_type(&persistence)
		.await
		.expect("Failed to validate");
	assert!(!is_valid, "Unset GFK should not validate");

	// Should return None for content type
	let ct = gfk
		.get_validated_content_type(&persistence)
		.await
		.expect("Failed to get content type");
	assert!(
		ct.is_none(),
		"Unset GFK should return None for content type"
	);
}
// ============================================================================
// Generic Relationship Tests
// ============================================================================

/// Test creating generic relationship table with GFK fields
///
/// **Test Intent**: Verify we can create database tables that use GenericForeignKey
/// pattern (content_type_id + object_id columns)
///
/// **Integration Point**: Database schema with GFK pattern
///
/// **Not Intent**: Relationship queries, cascade behavior
#[rstest]
#[tokio::test]
async fn test_create_generic_relationship_table(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

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

	// Create table with GenericForeignKey pattern (e.g., Comment table)
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS comments (
			id SERIAL PRIMARY KEY,
			content_type_id INTEGER NOT NULL,
			object_id BIGINT NOT NULL,
			text TEXT NOT NULL,
			FOREIGN KEY (content_type_id) REFERENCES contenttypes_contenttype(id)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create comments table");

	// Verify table exists
	let table_exists: bool = sqlx::query_scalar(
		"SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'comments')",
	)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to check table existence");

	assert!(table_exists, "Comments table should exist");
}

/// Test inserting generic relationship records with foreign key constraint
///
/// **Test Intent**: Verify we can insert records into GFK relationship tables
/// and that content_type_id foreign key constraint is enforced
///
/// **Integration Point**: GFK pattern INSERT with FK constraint validation
///
/// **Not Intent**: Query operations, relationship traversal
#[rstest]
#[tokio::test]
async fn test_insert_generic_relationship_with_constraint(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create ContentType table
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

	// Create comments table with FK constraint
	sqlx::query(
		r#"
		CREATE TABLE comments (
			id SERIAL PRIMARY KEY,
			content_type_id INTEGER NOT NULL,
			object_id BIGINT NOT NULL,
			text TEXT NOT NULL,
			FOREIGN KEY (content_type_id) REFERENCES contenttypes_contenttype(id)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert ContentType for "blog.Post"
	let ct_id: i32 = sqlx::query_scalar(
		"INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2) RETURNING id",
	)
	.bind("blog")
	.bind("Post")
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to insert ContentType");

	// Insert comment linked to blog.Post (object_id=1)
	sqlx::query("INSERT INTO comments (content_type_id, object_id, text) VALUES ($1, $2, $3)")
		.bind(ct_id)
		.bind(1_i64)
		.bind("Great post!")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert comment");

	// Verify comment was inserted
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM comments")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count comments");
	assert_eq!(count, 1);

	// Attempt to insert comment with non-existent content_type_id (should fail)
	let result = sqlx::query("INSERT INTO comments (content_type_id, object_id, text) VALUES ($1, $2, $3)")
		.bind(9999) // Non-existent content type
		.bind(2_i64)
		.bind("Another comment")
		.execute(pool.as_ref())
		.await;

	assert!(
		result.is_err(),
		"Insert with invalid content_type_id should fail due to FK constraint"
	);
}

/// Test querying generic relationships by content type
///
/// **Test Intent**: Verify we can query GFK relationship tables filtered by
/// content_type_id to get all objects of a specific type
///
/// **Integration Point**: GFK pattern SELECT with content_type_id filter
///
/// **Not Intent**: Complex joins, reverse relationship queries
#[rstest]
#[tokio::test]
async fn test_query_generic_relationships_by_content_type(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create ContentType table
	sqlx::query(
		r#"
		CREATE TABLE contenttypes_contenttype (
			id SERIAL PRIMARY KEY,
			app_label VARCHAR(100) NOT NULL,
			model VARCHAR(100) NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Create comments table
	sqlx::query(
		r#"
		CREATE TABLE comments (
			id SERIAL PRIMARY KEY,
			content_type_id INTEGER NOT NULL,
			object_id BIGINT NOT NULL,
			text TEXT NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert ContentTypes
	let post_ct_id: i32 = sqlx::query_scalar(
		"INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2) RETURNING id",
	)
	.bind("blog")
	.bind("Post")
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	let article_ct_id: i32 = sqlx::query_scalar(
		"INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2) RETURNING id",
	)
	.bind("news")
	.bind("Article")
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Insert comments for both types
	sqlx::query("INSERT INTO comments (content_type_id, object_id, text) VALUES ($1, $2, $3)")
		.bind(post_ct_id)
		.bind(1_i64)
		.bind("Comment on post 1")
		.execute(pool.as_ref())
		.await
		.unwrap();

	sqlx::query("INSERT INTO comments (content_type_id, object_id, text) VALUES ($1, $2, $3)")
		.bind(post_ct_id)
		.bind(2_i64)
		.bind("Comment on post 2")
		.execute(pool.as_ref())
		.await
		.unwrap();

	sqlx::query("INSERT INTO comments (content_type_id, object_id, text) VALUES ($1, $2, $3)")
		.bind(article_ct_id)
		.bind(1_i64)
		.bind("Comment on article 1")
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Query comments for blog.Post only
	let post_comments: Vec<String> =
		sqlx::query_scalar("SELECT text FROM comments WHERE content_type_id = $1 ORDER BY id")
			.bind(post_ct_id)
			.fetch_all(pool.as_ref())
			.await
			.expect("Failed to query post comments");

	assert_eq!(post_comments.len(), 2);
	assert_eq!(post_comments[0], "Comment on post 1");
	assert_eq!(post_comments[1], "Comment on post 2");

	// Query comments for news.Article only
	let article_comments: Vec<String> =
		sqlx::query_scalar("SELECT text FROM comments WHERE content_type_id = $1")
			.bind(article_ct_id)
			.fetch_all(pool.as_ref())
			.await
			.expect("Failed to query article comments");

	assert_eq!(article_comments.len(), 1);
	assert_eq!(article_comments[0], "Comment on article 1");
}

/// Test querying generic relationships by both content type and object ID
///
/// **Test Intent**: Verify we can query GFK relationship tables to get all
/// relationships for a specific object instance (content_type_id + object_id)
///
/// **Integration Point**: GFK pattern SELECT with both content_type_id and object_id filters
///
/// **Not Intent**: Cross-type queries, aggregation
#[rstest]
#[tokio::test]
async fn test_query_generic_relationships_by_object(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create ContentType table
	sqlx::query(
		r#"
		CREATE TABLE contenttypes_contenttype (
			id SERIAL PRIMARY KEY,
			app_label VARCHAR(100) NOT NULL,
			model VARCHAR(100) NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Create likes table (another GFK example)
	sqlx::query(
		r#"
		CREATE TABLE likes (
			id SERIAL PRIMARY KEY,
			content_type_id INTEGER NOT NULL,
			object_id BIGINT NOT NULL,
			user_id INTEGER NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert ContentType for "blog.Post"
	let post_ct_id: i32 = sqlx::query_scalar(
		"INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2) RETURNING id",
	)
	.bind("blog")
	.bind("Post")
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Insert likes for different posts
	sqlx::query("INSERT INTO likes (content_type_id, object_id, user_id) VALUES ($1, $2, $3)")
		.bind(post_ct_id)
		.bind(1_i64) // Post ID = 1
		.bind(100)
		.execute(pool.as_ref())
		.await
		.unwrap();

	sqlx::query("INSERT INTO likes (content_type_id, object_id, user_id) VALUES ($1, $2, $3)")
		.bind(post_ct_id)
		.bind(1_i64) // Post ID = 1
		.bind(200)
		.execute(pool.as_ref())
		.await
		.unwrap();

	sqlx::query("INSERT INTO likes (content_type_id, object_id, user_id) VALUES ($1, $2, $3)")
		.bind(post_ct_id)
		.bind(2_i64) // Post ID = 2
		.bind(100)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Query likes for specific post (content_type_id=post_ct_id, object_id=1)
	let likes: Vec<i32> = sqlx::query_scalar(
		"SELECT user_id FROM likes WHERE content_type_id = $1 AND object_id = $2 ORDER BY user_id",
	)
	.bind(post_ct_id)
	.bind(1_i64)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to query likes");

	assert_eq!(likes.len(), 2);
	assert_eq!(likes[0], 100);
	assert_eq!(likes[1], 200);
}

// ============================================================================
// Multiple Content Type Tests
// ============================================================================

/// Test GenericForeignKey with multiple different content types
///
/// **Test Intent**: Verify GenericForeignKey can correctly handle references
/// to multiple different content types in the same table
///
/// **Integration Point**: GFK pattern with heterogeneous content types
///
/// **Not Intent**: Same content type queries, type-specific operations
#[rstest]
#[tokio::test]
async fn test_gfk_multiple_content_types(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create ContentType table
	sqlx::query(
		r#"
		CREATE TABLE contenttypes_contenttype (
			id SERIAL PRIMARY KEY,
			app_label VARCHAR(100) NOT NULL,
			model VARCHAR(100) NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Create tags table (can tag any object type)
	sqlx::query(
		r#"
		CREATE TABLE tags (
			id SERIAL PRIMARY KEY,
			content_type_id INTEGER NOT NULL,
			object_id BIGINT NOT NULL,
			tag_name VARCHAR(50) NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert different ContentTypes
	let post_ct_id: i32 = sqlx::query_scalar(
		"INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2) RETURNING id",
	)
	.bind("blog")
	.bind("Post")
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	let product_ct_id: i32 = sqlx::query_scalar(
		"INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2) RETURNING id",
	)
	.bind("shop")
	.bind("Product")
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	let user_ct_id: i32 = sqlx::query_scalar(
		"INSERT INTO contenttypes_contenttype (app_label, model) VALUES ($1, $2) RETURNING id",
	)
	.bind("auth")
	.bind("User")
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	// Insert tags for different content types
	sqlx::query("INSERT INTO tags (content_type_id, object_id, tag_name) VALUES ($1, $2, $3)")
		.bind(post_ct_id)
		.bind(1_i64)
		.bind("tech")
		.execute(pool.as_ref())
		.await
		.unwrap();

	sqlx::query("INSERT INTO tags (content_type_id, object_id, tag_name) VALUES ($1, $2, $3)")
		.bind(product_ct_id)
		.bind(5_i64)
		.bind("electronics")
		.execute(pool.as_ref())
		.await
		.unwrap();

	sqlx::query("INSERT INTO tags (content_type_id, object_id, tag_name) VALUES ($1, $2, $3)")
		.bind(user_ct_id)
		.bind(10_i64)
		.bind("admin")
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Verify all tags were inserted with correct content types
	let tag_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tags")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();
	assert_eq!(tag_count, 3);

	// Query tags grouped by content type
	let post_tags: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tags WHERE content_type_id = $1")
		.bind(post_ct_id)
		.fetch_one(pool.as_ref())
		.await
		.unwrap();
	assert_eq!(post_tags, 1);

	let product_tags: i64 =
		sqlx::query_scalar("SELECT COUNT(*) FROM tags WHERE content_type_id = $1")
			.bind(product_ct_id)
			.fetch_one(pool.as_ref())
			.await
			.unwrap();
	assert_eq!(product_tags, 1);

	let user_tags: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tags WHERE content_type_id = $1")
		.bind(user_ct_id)
		.fetch_one(pool.as_ref())
		.await
		.unwrap();
	assert_eq!(user_tags, 1);
}

// ============================================================================
// GenericForeignKey Serialization Tests
// ============================================================================

/// Test GenericForeignKey JSON serialization
///
/// **Test Intent**: Verify GenericForeignKey can be serialized to JSON for
/// API responses or storage
///
/// **Integration Point**: GenericForeignKeyField → serde JSON
///
/// **Not Intent**: Database persistence, other serialization formats
#[rstest]
#[tokio::test]
async fn test_gfk_serialization(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	// Create GFK with values
	let gfk = GenericForeignKeyField::with_values(Some(7), Some(42));

	// Serialize to JSON
	let json = serde_json::to_string(&gfk).expect("Failed to serialize GFK");

	assert!(json.contains("7"));
	assert!(json.contains("42"));

	// Deserialize back
	let deserialized: GenericForeignKeyField =
		serde_json::from_str(&json).expect("Failed to deserialize GFK");

	assert_eq!(deserialized.content_type_id(), Some(7));
	assert_eq!(deserialized.object_id(), Some(42));
}

/// Test GenericForeignKey equality comparison
///
/// **Test Intent**: Verify GenericForeignKey implements PartialEq correctly
/// for comparing GFK instances
///
/// **Integration Point**: GenericForeignKeyField PartialEq implementation
///
/// **Not Intent**: Database queries, deep object comparison
#[rstest]
#[tokio::test]
async fn test_gfk_equality(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let gfk1 = GenericForeignKeyField::with_values(Some(1), Some(100));
	let gfk2 = GenericForeignKeyField::with_values(Some(1), Some(100));
	let gfk3 = GenericForeignKeyField::with_values(Some(2), Some(100));
	let gfk4 = GenericForeignKeyField::with_values(Some(1), Some(200));

	// Same values should be equal
	assert_eq!(gfk1, gfk2);

	// Different content_type_id should not be equal
	assert_ne!(gfk1, gfk3);

	// Different object_id should not be equal
	assert_ne!(gfk1, gfk4);
}
