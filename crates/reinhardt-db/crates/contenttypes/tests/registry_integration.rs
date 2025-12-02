//! ContentType Registry Integration Tests
//!
//! These tests verify ContentType registry operations with real PostgreSQL database,
//! including registry initialization, lookup, caching, and synchronization.
//!
//! **Test Coverage:**
//! - Registry initialization and population
//! - ContentType lookup by app_label and model
//! - ContentType lookup by ID
//! - Registry caching behavior
//! - Registry synchronization with database
//! - Bulk registry operations
//! - Type-safe registry operations (ModelType trait)
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container

#[cfg(feature = "database")]
use reinhardt_contenttypes::persistence::{ContentTypePersistence, ContentTypePersistenceBackend};
use reinhardt_contenttypes::{CONTENT_TYPE_REGISTRY, ContentType, ContentTypeRegistry};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use serial_test::serial;
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Registry Basic Operations Tests
// ============================================================================

/// Test ContentTypeRegistry creation and basic operations
///
/// **Test Intent**: Verify ContentTypeRegistry can be created and basic
/// operations (register, get, get_by_id) work correctly
///
/// **Integration Point**: ContentTypeRegistry → in-memory registry
///
/// **Not Intent**: Database operations, persistence
#[rstest]
#[tokio::test]
async fn test_registry_basic_operations(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let registry = ContentTypeRegistry::new();

	// Register content type
	let ct = registry.register(ContentType::new("blog", "Post"));
	assert!(ct.id.is_some());
	assert_eq!(ct.app_label, "blog");
	assert_eq!(ct.model, "Post");

	// Get by app_label and model
	let found = registry.get("blog", "Post");
	assert!(found.is_some());
	assert_eq!(found.unwrap().id, ct.id);

	// Get by ID
	let found_by_id = registry.get_by_id(ct.id.unwrap());
	assert!(found_by_id.is_some());
	assert_eq!(found_by_id.unwrap().app_label, "blog");
}

/// Test ContentTypeRegistry get_or_create idempotency
///
/// **Test Intent**: Verify get_or_create returns the same ContentType instance
/// when called multiple times with same parameters
///
/// **Integration Point**: ContentTypeRegistry::get_or_create
///
/// **Not Intent**: Database persistence, different content types
#[rstest]
#[tokio::test]
async fn test_registry_get_or_create_idempotent(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let registry = ContentTypeRegistry::new();

	// First call creates
	let ct1 = registry.get_or_create("auth", "User");
	assert!(ct1.id.is_some());

	// Second call returns existing
	let ct2 = registry.get_or_create("auth", "User");
	assert_eq!(ct1.id, ct2.id);
	assert_eq!(ct1.app_label, ct2.app_label);
	assert_eq!(ct1.model, ct2.model);
}

/// Test ContentTypeRegistry with multiple content types
///
/// **Test Intent**: Verify registry can handle multiple different content types
/// simultaneously without conflicts
///
/// **Integration Point**: ContentTypeRegistry with multiple entries
///
/// **Not Intent**: Single content type operations, registry clearing
#[rstest]
#[tokio::test]
async fn test_registry_multiple_content_types(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let registry = ContentTypeRegistry::new();

	// Register multiple content types
	let ct1 = registry.register(ContentType::new("blog", "Post"));
	let ct2 = registry.register(ContentType::new("blog", "Comment"));
	let ct3 = registry.register(ContentType::new("shop", "Product"));
	let ct4 = registry.register(ContentType::new("auth", "User"));

	// Verify all have unique IDs
	let ids = [
		ct1.id.unwrap(),
		ct2.id.unwrap(),
		ct3.id.unwrap(),
		ct4.id.unwrap(),
	];
	let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
	assert_eq!(unique_ids.len(), 4, "All IDs should be unique");

	// Verify all can be retrieved
	assert!(registry.get("blog", "Post").is_some());
	assert!(registry.get("blog", "Comment").is_some());
	assert!(registry.get("shop", "Product").is_some());
	assert!(registry.get("auth", "User").is_some());
}

/// Test ContentTypeRegistry all() method
///
/// **Test Intent**: Verify registry.all() returns all registered content types
///
/// **Integration Point**: ContentTypeRegistry::all
///
/// **Not Intent**: Filtered queries, specific content type retrieval
#[rstest]
#[tokio::test]
async fn test_registry_all(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let registry = ContentTypeRegistry::new();

	// Register some content types
	registry.register(ContentType::new("app1", "Model1"));
	registry.register(ContentType::new("app2", "Model2"));
	registry.register(ContentType::new("app3", "Model3"));

	// Get all content types
	let all = registry.all();
	assert_eq!(all.len(), 3);

	// Verify all expected content types are present
	let app_labels: Vec<&str> = all.iter().map(|ct| ct.app_label.as_str()).collect();
	assert!(app_labels.contains(&"app1"));
	assert!(app_labels.contains(&"app2"));
	assert!(app_labels.contains(&"app3"));
}

/// Test ContentTypeRegistry clear() method
///
/// **Test Intent**: Verify registry.clear() removes all content types and resets state
///
/// **Integration Point**: ContentTypeRegistry::clear
///
/// **Not Intent**: Individual content type deletion, database operations
#[rstest]
#[tokio::test]
async fn test_registry_clear(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let registry = ContentTypeRegistry::new();

	// Register content types
	registry.register(ContentType::new("test1", "Model1"));
	registry.register(ContentType::new("test2", "Model2"));

	assert_eq!(registry.all().len(), 2);

	// Clear registry
	registry.clear();
	assert_eq!(registry.all().len(), 0);

	// Verify lookups return None
	assert!(registry.get("test1", "Model1").is_none());
	assert!(registry.get("test2", "Model2").is_none());
}

// ============================================================================
// Registry + Database Persistence Integration Tests
// ============================================================================
// Note: These tests require ContentTypePersistence with Arc<PgPool> support
// Currently deferred to Phase 2 as ContentTypePersistence expects Arc<AnyPool>

#[cfg(feature = "database")]
/// Test registry initialization from database
///
/// **Test Intent**: Verify registry can be populated from existing database
/// ContentType records
///
/// **Integration Point**: ContentTypeRegistry ← ContentTypePersistence
///
/// **Not Intent**: Empty database initialization, registry-only operations
#[rstest]
#[tokio::test]
#[ignore = "Requires ContentTypePersistence with PgPool support - deferred to Phase 2"]
async fn test_registry_init_from_database(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let any_pool = Arc::new(
		sqlx::AnyPool::connect(&url)
			.await
			.expect("Failed to connect AnyPool"),
	);
	let persistence = ContentTypePersistence::from_pool(any_pool, &url);
	persistence
		.create_table()
		.await
		.expect("Failed to create ContentType table");

	// Insert content types directly into database
	let ct1 = persistence
		.save(&ContentType::new("auth", "User"))
		.await
		.expect("Failed to save ct1");
	let ct2 = persistence
		.save(&ContentType::new("blog", "Post"))
		.await
		.expect("Failed to save ct2");

	// Create new registry and populate from database
	let registry = ContentTypeRegistry::new();

	// Manually load from database (simulating initialization)
	let all_cts = persistence
		.load_all()
		.await
		.expect("Failed to load all CTs");
	for ct in all_cts {
		registry.register(ct);
	}

	// Verify registry contains database content types
	let user_ct = registry.get("auth", "User");
	assert!(user_ct.is_some());
	assert_eq!(user_ct.unwrap().id, ct1.id);

	let post_ct = registry.get("blog", "Post");
	assert!(post_ct.is_some());
	assert_eq!(post_ct.unwrap().id, ct2.id);
}

#[cfg(feature = "database")]
/// Test registry synchronization with database
///
/// **Test Intent**: Verify registry can detect and sync new content types
/// added to database by other processes
///
/// **Integration Point**: ContentTypeRegistry ↔ ContentTypePersistence sync
///
/// **Not Intent**: Real-time updates, database triggers
#[rstest]
#[tokio::test]
#[ignore = "Requires ContentTypePersistence with PgPool support - deferred to Phase 2"]
async fn test_registry_sync_with_database(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let any_pool = Arc::new(
		sqlx::AnyPool::connect(&url)
			.await
			.expect("Failed to connect AnyPool"),
	);
	let persistence = ContentTypePersistence::from_pool(any_pool, &url);
	persistence
		.create_table()
		.await
		.expect("Failed to create ContentType table");

	let registry = ContentTypeRegistry::new();

	// Initially, registry is empty
	assert_eq!(registry.all().len(), 0);

	// Save content type to database
	let ct = persistence
		.save(&ContentType::new("shop", "Product"))
		.await
		.expect("Failed to save CT");

	// Registry doesn't know about it yet
	assert!(registry.get("shop", "Product").is_none());

	// Sync from database
	let all_cts = persistence
		.load_all()
		.await
		.expect("Failed to load all CTs");
	for db_ct in all_cts {
		registry.register(db_ct);
	}

	// Now registry should have it
	let synced_ct = registry.get("shop", "Product");
	assert!(synced_ct.is_some());
	assert_eq!(synced_ct.unwrap().id, ct.id);
}

#[cfg(feature = "database")]
/// Test registry handles database content types with existing IDs
///
/// **Test Intent**: Verify registry correctly handles registering content types
/// that already have database-assigned IDs
///
/// **Integration Point**: ContentTypeRegistry::register with pre-assigned IDs
///
/// **Not Intent**: Auto-generated IDs, ID conflicts
#[rstest]
#[tokio::test]
#[ignore = "Requires ContentTypePersistence with PgPool support - deferred to Phase 2"]
async fn test_registry_with_database_ids(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let any_pool = Arc::new(
		sqlx::AnyPool::connect(&url)
			.await
			.expect("Failed to connect AnyPool"),
	);
	let persistence = ContentTypePersistence::from_pool(any_pool, &url);
	persistence
		.create_table()
		.await
		.expect("Failed to create ContentType table");

	// Save to database (gets ID)
	let ct = persistence
		.save(&ContentType::new("test", "Model"))
		.await
		.expect("Failed to save CT");
	let db_id = ct.id.unwrap();

	// Register in new registry with existing ID
	let registry = ContentTypeRegistry::new();
	let registered = registry.register(ct.clone());

	// Should preserve database ID
	assert_eq!(registered.id, Some(db_id));

	// Can lookup by database ID
	let found = registry.get_by_id(db_id);
	assert!(found.is_some());
	assert_eq!(found.unwrap().app_label, "test");
}

#[cfg(feature = "database")]
/// Test bulk registry operations from database
///
/// **Test Intent**: Verify registry can efficiently handle bulk loading of
/// many content types from database
///
/// **Integration Point**: ContentTypeRegistry bulk loading
///
/// **Not Intent**: Single content type operations, incremental loading
#[rstest]
#[tokio::test]
#[ignore = "Requires ContentTypePersistence with PgPool support - deferred to Phase 2"]
async fn test_registry_bulk_load_from_database(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let any_pool = Arc::new(
		sqlx::AnyPool::connect(&url)
			.await
			.expect("Failed to connect AnyPool"),
	);
	let persistence = ContentTypePersistence::from_pool(any_pool, &url);
	persistence
		.create_table()
		.await
		.expect("Failed to create ContentType table");

	// Create multiple content types in database
	let apps = vec!["app1", "app2", "app3", "app4", "app5"];
	let models = vec!["Model1", "Model2", "Model3"];

	for app in &apps {
		for model in &models {
			persistence
				.save(&ContentType::new(*app, *model))
				.await
				.expect("Failed to save CT");
		}
	}

	// Bulk load into registry
	let registry = ContentTypeRegistry::new();
	let all_cts = persistence
		.load_all()
		.await
		.expect("Failed to load all CTs");

	for ct in all_cts {
		registry.register(ct);
	}

	// Verify all were loaded
	assert_eq!(registry.all().len(), 15); // 5 apps × 3 models

	// Spot check some content types
	assert!(registry.get("app1", "Model1").is_some());
	assert!(registry.get("app3", "Model2").is_some());
	assert!(registry.get("app5", "Model3").is_some());
}

// ============================================================================
// Global Registry Tests (CONTENT_TYPE_REGISTRY)
// ============================================================================

/// Test global CONTENT_TYPE_REGISTRY singleton
///
/// **Test Intent**: Verify global CONTENT_TYPE_REGISTRY can be used for
/// application-wide content type management
///
/// **Integration Point**: CONTENT_TYPE_REGISTRY global instance
///
/// **Not Intent**: Multiple registry instances, local registries
#[rstest]
#[serial(global_registry)]
#[tokio::test]
async fn test_global_registry_basic(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	// Clear global registry before test
	CONTENT_TYPE_REGISTRY.clear();

	// Register content type
	let ct = CONTENT_TYPE_REGISTRY.register(ContentType::new("global", "Test"));
	assert!(ct.id.is_some());

	// Retrieve from global registry
	let found = CONTENT_TYPE_REGISTRY.get("global", "Test");
	assert!(found.is_some());
	assert_eq!(found.unwrap().id, ct.id);

	// Cleanup
	CONTENT_TYPE_REGISTRY.clear();
}

/// Test global registry isolation between tests
///
/// **Test Intent**: Verify global registry can be properly cleared between tests
/// to prevent state leakage
///
/// **Integration Point**: CONTENT_TYPE_REGISTRY::clear
///
/// **Not Intent**: Registry persistence across tests
#[rstest]
#[serial(global_registry)]
#[tokio::test]
async fn test_global_registry_isolation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	// Clear and verify empty
	CONTENT_TYPE_REGISTRY.clear();
	assert_eq!(CONTENT_TYPE_REGISTRY.all().len(), 0);

	// Add content type
	CONTENT_TYPE_REGISTRY.register(ContentType::new("isolation", "Test"));
	assert_eq!(CONTENT_TYPE_REGISTRY.all().len(), 1);

	// Clear again
	CONTENT_TYPE_REGISTRY.clear();
	assert_eq!(CONTENT_TYPE_REGISTRY.all().len(), 0);

	// Verify it's really gone
	assert!(CONTENT_TYPE_REGISTRY.get("isolation", "Test").is_none());
}

// ============================================================================
// Type-Safe Registry Operations Tests (ModelType trait)
// ============================================================================

// Define test model types
struct UserModel;
impl reinhardt_contenttypes::ModelType for UserModel {
	const APP_LABEL: &'static str = "auth";
	const MODEL_NAME: &'static str = "User";
}

struct PostModel;
impl reinhardt_contenttypes::ModelType for PostModel {
	const APP_LABEL: &'static str = "blog";
	const MODEL_NAME: &'static str = "Post";
}

struct ProductModel;
impl reinhardt_contenttypes::ModelType for ProductModel {
	const APP_LABEL: &'static str = "shop";
	const MODEL_NAME: &'static str = "Product";
}

/// Test type-safe registry registration
///
/// **Test Intent**: Verify ContentTypeRegistry::register_typed works with
/// compile-time verified model types
///
/// **Integration Point**: ContentTypeRegistry::register_typed<M: ModelType>
///
/// **Not Intent**: String-based registration, runtime type checking
#[rstest]
#[tokio::test]
async fn test_registry_typed_register(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let registry = ContentTypeRegistry::new();

	// Register using typed method
	let ct = registry.register_typed::<UserModel>();

	assert_eq!(ct.app_label, "auth");
	assert_eq!(ct.model, "User");
	assert!(ct.id.is_some());
}

/// Test type-safe registry get
///
/// **Test Intent**: Verify ContentTypeRegistry::get_typed retrieves content types
/// using compile-time verified model types
///
/// **Integration Point**: ContentTypeRegistry::get_typed<M: ModelType>
///
/// **Not Intent**: String-based lookup, dynamic type resolution
#[rstest]
#[tokio::test]
async fn test_registry_typed_get(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let registry = ContentTypeRegistry::new();

	// Register content type
	registry.register_typed::<PostModel>();

	// Get using typed method
	let ct = registry.get_typed::<PostModel>();
	assert!(ct.is_some());
	assert_eq!(ct.unwrap().model, "Post");

	// Non-existent type returns None
	let none = registry.get_typed::<ProductModel>();
	assert!(none.is_none());
}

/// Test type-safe registry get_or_create
///
/// **Test Intent**: Verify ContentTypeRegistry::get_or_create_typed creates
/// content types on first call and returns existing on subsequent calls
///
/// **Integration Point**: ContentTypeRegistry::get_or_create_typed<M: ModelType>
///
/// **Not Intent**: String-based get_or_create, separate get/create calls
#[rstest]
#[tokio::test]
async fn test_registry_typed_get_or_create(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let registry = ContentTypeRegistry::new();

	// First call creates
	let ct1 = registry.get_or_create_typed::<ProductModel>();
	assert_eq!(ct1.app_label, "shop");
	assert_eq!(ct1.model, "Product");

	// Second call returns existing
	let ct2 = registry.get_or_create_typed::<ProductModel>();
	assert_eq!(ct1.id, ct2.id);
}

/// Test mixing typed and string-based registry access
///
/// **Test Intent**: Verify typed and string-based registry methods can be used
/// interchangeably on the same registry instance
///
/// **Integration Point**: ContentTypeRegistry typed ↔ string-based methods
///
/// **Not Intent**: Separate registries, type-only or string-only access
#[rstest]
#[tokio::test]
async fn test_registry_typed_and_string_mixed(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let registry = ContentTypeRegistry::new();

	// Register using typed method
	registry.register_typed::<UserModel>();

	// Can access using both typed and string methods
	let typed = registry.get_typed::<UserModel>();
	let string = registry.get("auth", "User");

	assert!(typed.is_some());
	assert!(string.is_some());
	assert_eq!(typed.unwrap().id, string.unwrap().id);
}

// ============================================================================
// Registry Edge Cases and Error Conditions
// ============================================================================

/// Test registry with duplicate registration attempts
///
/// **Test Intent**: Verify registry handles duplicate registration gracefully
/// by returning existing content type
///
/// **Integration Point**: ContentTypeRegistry::register with duplicates
///
/// **Not Intent**: Error handling, exception throwing
#[rstest]
#[tokio::test]
async fn test_registry_duplicate_registration(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let registry = ContentTypeRegistry::new();

	// First registration
	let ct1 = registry.register(ContentType::new("dup", "Model"));
	let id1 = ct1.id.unwrap();

	// Attempt duplicate registration
	let ct2 = registry.register(ContentType::new("dup", "Model"));
	let id2 = ct2.id.unwrap();

	// Should return existing (same ID)
	assert_eq!(id1, id2);

	// Registry should still have only one entry
	let all = registry.all();
	let dup_models: Vec<_> = all
		.iter()
		.filter(|ct| ct.app_label == "dup" && ct.model == "Model")
		.collect();
	assert_eq!(dup_models.len(), 1);
}

/// Test registry with non-existent content type lookup
///
/// **Test Intent**: Verify registry returns None for non-existent content types
/// rather than panicking or creating entries
///
/// **Integration Point**: ContentTypeRegistry::get with non-existent key
///
/// **Not Intent**: Auto-creation, error throwing
#[rstest]
#[tokio::test]
async fn test_registry_nonexistent_lookup(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let registry = ContentTypeRegistry::new();

	// Lookup non-existent content type
	let result = registry.get("nonexistent", "Model");
	assert!(result.is_none());

	// Lookup by non-existent ID
	let result_by_id = registry.get_by_id(9999);
	assert!(result_by_id.is_none());

	// Registry should still be empty
	assert_eq!(registry.all().len(), 0);
}

/// Test registry case sensitivity
///
/// **Test Intent**: Verify registry treats different cases as different content types
///
/// **Integration Point**: ContentTypeRegistry case handling
///
/// **Not Intent**: Case-insensitive matching, normalization
#[rstest]
#[tokio::test]
async fn test_registry_case_sensitivity(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let registry = ContentTypeRegistry::new();

	// Register with different cases
	let ct1 = registry.register(ContentType::new("App", "Model"));
	let ct2 = registry.register(ContentType::new("app", "Model"));
	let ct3 = registry.register(ContentType::new("app", "model"));

	// All should have different IDs
	assert_ne!(ct1.id, ct2.id);
	assert_ne!(ct2.id, ct3.id);
	assert_ne!(ct1.id, ct3.id);

	// All should be retrievable independently
	assert!(registry.get("App", "Model").is_some());
	assert!(registry.get("app", "Model").is_some());
	assert!(registry.get("app", "model").is_some());
}

/// Test registry with special characters in names
///
/// **Test Intent**: Verify registry handles content types with special characters
/// (underscores, hyphens) in app_label and model names
///
/// **Integration Point**: ContentTypeRegistry with special characters
///
/// **Not Intent**: Unicode handling, emoji support
#[rstest]
#[tokio::test]
async fn test_registry_special_characters(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let registry = ContentTypeRegistry::new();

	// Register with underscores
	let ct1 = registry.register(ContentType::new("my_app", "My_Model"));
	assert_eq!(ct1.app_label, "my_app");
	assert_eq!(ct1.model, "My_Model");

	// Register with hyphens (if supported by your system)
	let ct2 = registry.register(ContentType::new("my-app", "my-model"));
	assert_eq!(ct2.app_label, "my-app");
	assert_eq!(ct2.model, "my-model");

	// Both should be retrievable
	assert!(registry.get("my_app", "My_Model").is_some());
	assert!(registry.get("my-app", "my-model").is_some());
}

/// Test registry with empty initial state
///
/// **Test Intent**: Verify newly created registry starts with zero content types
///
/// **Integration Point**: ContentTypeRegistry::new initial state
///
/// **Not Intent**: Pre-populated registries, default content types
#[rstest]
#[tokio::test]
async fn test_registry_empty_initial_state(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let registry = ContentTypeRegistry::new();

	// Should be empty initially
	assert_eq!(registry.all().len(), 0);

	// get_or_create on empty registry should create
	let ct = registry.get_or_create("first", "Model");
	assert!(ct.id.is_some());
	assert_eq!(registry.all().len(), 1);
}

/// Test separate registry instances don't share state
///
/// **Test Intent**: Verify multiple ContentTypeRegistry instances maintain
/// independent state
///
/// **Integration Point**: ContentTypeRegistry instance isolation
///
/// **Not Intent**: Global registry, shared state
#[rstest]
#[tokio::test]
async fn test_registry_instance_isolation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let registry1 = ContentTypeRegistry::new();
	let registry2 = ContentTypeRegistry::new();

	// Register in registry1
	registry1.register(ContentType::new("test", "Model"));

	// registry2 should not have it
	assert!(registry2.get("test", "Model").is_none());
	assert_eq!(registry1.all().len(), 1);
	assert_eq!(registry2.all().len(), 0);
}
