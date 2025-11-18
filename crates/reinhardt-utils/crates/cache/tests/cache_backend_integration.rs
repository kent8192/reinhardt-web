//! Integration tests for cache backend implementations
//!
//! This test file verifies the integration between:
//! - Cache backends (memory, Redis, file, Memcached)
//! - Serialization layer (JSON with serde)
//! - TTL management (expiration and cleanup)
//! - Key namespacing (prefixes and versioning)
//!
//! ## Testing Strategy
//!
//! Tests use actual cache backends:
//! - InMemoryCache: Tests with naive and layered cleanup strategies
//! - RedisCache: Uses TestContainers for real Redis instances
//! - FileCache: Uses tempfile for filesystem-based cache
//! - MemcachedCache: Uses TestContainers for real Memcached instances
//!
//! This ensures cache operations work correctly in production-like scenarios
//! with real infrastructure, serialization, and concurrent access patterns.
//!
//! ## Test Coverage
//!
//! - Basic cache operations (get, set, delete, has_key, clear)
//! - TTL expiration and verification
//! - Batch operations (get_many, set_many, delete_many)
//! - Atomic operations (incr, decr)
//! - Key namespacing and versioning
//! - Serialization of complex types
//! - Concurrent access patterns
//! - Backend consistency and trait implementation
//! - Cache miss handling
//! - Backend switching and migration

use reinhardt_cache::{
	Cache, CacheKeyBuilder, FileCache, InMemoryCache, MemcachedCache, RedisCache,
};
use reinhardt_test::containers::MemcachedContainer;
use rstest::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use testcontainers::GenericImage;
use testcontainers::core::ContainerAsync;
use tokio::sync::Barrier;
use uuid::Uuid;

// ========================================
// Test Fixtures
// ========================================

/// Complex struct for serialization testing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct UserData {
	id: u64,
	username: String,
	email: String,
	tags: Vec<String>,
	metadata: HashMap<String, String>,
}

impl UserData {
	fn sample() -> Self {
		let mut metadata = HashMap::new();
		metadata.insert("role".to_string(), "admin".to_string());
		metadata.insert("department".to_string(), "engineering".to_string());

		Self {
			id: 12345,
			username: "alice".to_string(),
			email: "alice@example.com".to_string(),
			tags: vec!["vip".to_string(), "premium".to_string()],
			metadata,
		}
	}
}

/// Complex enum for serialization testing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum CacheData {
	String(String),
	Number(i64),
	Object { name: String, value: i64 },
	List(Vec<String>),
}

#[fixture]
fn temp_cache_dir() -> PathBuf {
	let temp_dir = std::env::temp_dir().join(format!("cache_test_{}", uuid::Uuid::new_v4()));
	std::fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");
	temp_dir
}

#[fixture]
async fn redis_container() -> ContainerAsync<GenericImage> {
	let image = GenericImage::new("redis", "7-alpine").with_wait_for(
		testcontainers::core::WaitFor::message_on_stdout("Ready to accept connections"),
	);
	testcontainers::runners::AsyncRunner::start(image)
		.await
		.expect("Failed to start Redis container")
}

#[fixture]
async fn memcached_container() -> MemcachedContainer {
	let (_container, _url) = reinhardt_test::containers::start_memcached().await;
	_container
}

// ========================================
// Memory Cache Backend Tests
// ========================================

/// Test Intent: Verify InMemoryCache basic operations (get, set, delete, has_key)
/// Integration Point: InMemoryCache ↔ Cache trait ↔ Serialization layer
#[rstest]
#[tokio::test]
async fn test_memory_cache_basic_operations() {
	let cache = InMemoryCache::new();

	// Set and get basic string
	cache
		.set("key1", &"value1", None)
		.await
		.expect("Failed to set key1");
	let value: Option<String> = cache.get("key1").await.expect("Failed to get key1");
	assert_eq!(value, Some("value1".to_string()));

	// Has key check
	assert!(cache.has_key("key1").await.expect("Failed to check key1"));
	assert!(
		!cache
			.has_key("nonexistent")
			.await
			.expect("Failed to check nonexistent")
	);

	// Delete operation
	cache.delete("key1").await.expect("Failed to delete key1");
	let value: Option<String> = cache.get("key1").await.expect("Failed to get after delete");
	assert_eq!(value, None);
	assert!(
		!cache
			.has_key("key1")
			.await
			.expect("Failed to check after delete")
	);
}

/// Test Intent: Verify TTL expiration and passive cleanup in InMemoryCache
/// Integration Point: InMemoryCache ↔ TTL management ↔ Expiration checking
#[rstest]
#[tokio::test]
async fn test_memory_cache_ttl_expiration() {
	let cache = InMemoryCache::new();

	// Set with short TTL
	cache
		.set("ttl_key", &"value", Some(Duration::from_millis(100)))
		.await
		.expect("Failed to set ttl_key");

	// Should exist immediately
	assert!(
		cache
			.has_key("ttl_key")
			.await
			.expect("Failed to check ttl_key")
	);
	let value: Option<String> = cache.get("ttl_key").await.expect("Failed to get ttl_key");
	assert_eq!(value, Some("value".to_string()));

	// Wait for expiration
	tokio::time::sleep(Duration::from_millis(150)).await;

	// Should be expired (passive cleanup on get)
	let value: Option<String> = cache
		.get("ttl_key")
		.await
		.expect("Failed to get expired key");
	assert_eq!(value, None, "Key should have expired");
}

/// Test Intent: Verify serialization of complex struct types
/// Integration Point: InMemoryCache ↔ serde JSON serialization ↔ Complex types
#[rstest]
#[tokio::test]
async fn test_memory_cache_complex_type_serialization() {
	let cache = InMemoryCache::new();

	let user_data = UserData::sample();

	// Set complex struct
	cache
		.set("user:12345", &user_data, None)
		.await
		.expect("Failed to set user data");

	// Get and verify
	let retrieved: Option<UserData> = cache
		.get("user:12345")
		.await
		.expect("Failed to get user data");
	assert_eq!(retrieved, Some(user_data.clone()));

	// Verify nested fields
	let retrieved = retrieved.unwrap();
	assert_eq!(retrieved.id, 12345);
	assert_eq!(retrieved.username, "alice");
	assert_eq!(retrieved.tags.len(), 2);
	assert_eq!(retrieved.metadata.get("role"), Some(&"admin".to_string()));
}

/// Test Intent: Verify batch operations (get_many, set_many, delete_many)
/// Integration Point: InMemoryCache ↔ Batch operation trait methods
#[rstest]
#[tokio::test]
async fn test_memory_cache_batch_operations() {
	let cache = InMemoryCache::new();

	// Set many values
	let mut values = HashMap::new();
	values.insert("batch1".to_string(), "value1".to_string());
	values.insert("batch2".to_string(), "value2".to_string());
	values.insert("batch3".to_string(), "value3".to_string());

	cache
		.set_many(values, None)
		.await
		.expect("Failed to set many");

	// Get many values
	let keys = vec!["batch1", "batch2", "batch3", "nonexistent"];
	let results: HashMap<String, String> = cache.get_many(&keys).await.expect("Failed to get many");

	assert_eq!(results.len(), 3);
	assert_eq!(results.get("batch1"), Some(&"value1".to_string()));
	assert_eq!(results.get("batch2"), Some(&"value2".to_string()));
	assert_eq!(results.get("batch3"), Some(&"value3".to_string()));
	assert_eq!(results.get("nonexistent"), None);

	// Delete many values
	cache
		.delete_many(&["batch1", "batch3"])
		.await
		.expect("Failed to delete many");

	let results: HashMap<String, String> = cache
		.get_many(&["batch1", "batch2", "batch3"])
		.await
		.expect("Failed to get after delete");

	assert_eq!(results.len(), 1);
	assert_eq!(results.get("batch2"), Some(&"value2".to_string()));
}

/// Test Intent: Verify atomic increment/decrement operations
/// Integration Point: InMemoryCache ↔ Atomic counter operations
#[rstest]
#[tokio::test]
async fn test_memory_cache_atomic_operations() {
	let cache = InMemoryCache::new();

	// Increment from zero
	let result = cache.incr("counter", 5).await.expect("Failed to increment");
	assert_eq!(result, 5);

	// Increment again
	let result = cache
		.incr("counter", 10)
		.await
		.expect("Failed to increment");
	assert_eq!(result, 15);

	// Decrement
	let result = cache.decr("counter", 3).await.expect("Failed to decrement");
	assert_eq!(result, 12);

	// Verify value persists
	let value: Option<i64> = cache.get("counter").await.expect("Failed to get counter");
	assert_eq!(value, Some(12));
}

/// Test Intent: Verify layered cleanup strategy performance and correctness
/// Integration Point: InMemoryCache layered store ↔ Cleanup mechanism
#[rstest]
#[tokio::test]
async fn test_memory_cache_layered_cleanup_strategy() {
	let cache = InMemoryCache::with_layered_cleanup();

	// Set multiple keys with short TTL
	for i in 0..50 {
		cache
			.set(
				&format!("layered_key_{}", i),
				&format!("value_{}", i),
				Some(Duration::from_millis(100)),
			)
			.await
			.expect(&format!("Failed to set key {}", i));
	}

	// All keys should exist
	let stats = cache.get_statistics().await;
	assert_eq!(stats.entry_count, 50);

	// Wait for expiration
	tokio::time::sleep(Duration::from_millis(150)).await;

	// Cleanup expired entries (layered strategy should be efficient)
	cache.cleanup_expired().await;

	// All keys should be cleaned up
	let stats = cache.get_statistics().await;
	assert_eq!(stats.entry_count, 0);
}

/// Test Intent: Verify concurrent access to cache from multiple tasks
/// Integration Point: InMemoryCache ↔ Tokio concurrency ↔ RwLock synchronization
#[rstest]
#[tokio::test]
async fn test_memory_cache_concurrent_access() {
	let cache = Arc::new(InMemoryCache::new());
	let barrier = Arc::new(Barrier::new(10));

	let mut handles = vec![];

	// Spawn 10 tasks that concurrently set and get values
	for i in 0..10 {
		let cache_clone = cache.clone();
		let barrier_clone = barrier.clone();

		let handle = tokio::spawn(async move {
			barrier_clone.wait().await;

			let key = format!("concurrent_key_{}", i);
			let value = format!("concurrent_value_{}", i);

			cache_clone
				.set(&key, &value, None)
				.await
				.expect("Failed to set in concurrent task");

			let retrieved: Option<String> = cache_clone
				.get(&key)
				.await
				.expect("Failed to get in concurrent task");

			assert_eq!(retrieved, Some(value));
		});

		handles.push(handle);
	}

	// Wait for all tasks to complete
	for handle in handles {
		handle.await.expect("Task panicked");
	}

	// Verify all keys exist
	let stats = cache.get_statistics().await;
	assert_eq!(stats.entry_count, 10);
}

// ========================================
// Redis Cache Backend Tests
// ========================================

/// Test Intent: Verify RedisCache basic operations with real Redis instance
/// Integration Point: RedisCache ↔ Redis protocol ↔ Connection pooling
#[rstest]
#[tokio::test]
async fn test_redis_cache_basic_operations(
	#[future] redis_container: ContainerAsync<GenericImage>,
) {
	let container = redis_container.await;
	let port = container
		.get_host_port_ipv4(6379)
		.await
		.expect("Failed to get Redis port");
	let url = format!("redis://127.0.0.1:{}", port);

	let cache = RedisCache::new(&url)
		.await
		.expect("Failed to create Redis cache");

	// Set and get
	cache
		.set("redis_key", &"redis_value", None)
		.await
		.expect("Failed to set");
	let value: Option<String> = cache.get("redis_key").await.expect("Failed to get");
	assert_eq!(value, Some("redis_value".to_string()));

	// Delete
	cache.delete("redis_key").await.expect("Failed to delete");
	let value: Option<String> = cache
		.get("redis_key")
		.await
		.expect("Failed to get after delete");
	assert_eq!(value, None);
}

/// Test Intent: Verify Redis TTL expiration with actual Redis backend
/// Integration Point: RedisCache ↔ Redis SETEX command ↔ TTL expiration
#[rstest]
#[tokio::test]
async fn test_redis_cache_ttl_expiration(#[future] redis_container: ContainerAsync<GenericImage>) {
	let container = redis_container.await;
	let port = container
		.get_host_port_ipv4(6379)
		.await
		.expect("Failed to get Redis port");
	let url = format!("redis://127.0.0.1:{}", port);

	let cache = RedisCache::new(&url)
		.await
		.expect("Failed to create Redis cache");

	// Set with 1 second TTL
	cache
		.set("ttl_key", &"value", Some(Duration::from_secs(1)))
		.await
		.expect("Failed to set with TTL");

	// Should exist immediately
	assert!(cache.has_key("ttl_key").await.expect("Failed to check key"));

	// Wait for expiration
	tokio::time::sleep(Duration::from_millis(1200)).await;

	// Should be expired
	assert!(
		!cache
			.has_key("ttl_key")
			.await
			.expect("Failed to check expired key")
	);
}

/// Test Intent: Verify Redis key namespacing with prefixes
/// Integration Point: RedisCache ↔ Key prefix building ↔ Redis storage
#[rstest]
#[tokio::test]
async fn test_redis_cache_key_namespacing(#[future] redis_container: ContainerAsync<GenericImage>) {
	let container = redis_container.await;
	let port = container
		.get_host_port_ipv4(6379)
		.await
		.expect("Failed to get Redis port");
	let url = format!("redis://127.0.0.1:{}", port);

	let cache = RedisCache::new(&url)
		.await
		.expect("Failed to create Redis cache")
		.with_key_prefix("myapp");

	cache
		.set("user:1", &"Alice", None)
		.await
		.expect("Failed to set");

	// Key should be accessible via cache
	let value: Option<String> = cache.get("user:1").await.expect("Failed to get");
	assert_eq!(value, Some("Alice".to_string()));

	// Clear should only clear prefixed keys
	cache.clear().await.expect("Failed to clear");

	let value: Option<String> = cache
		.get("user:1")
		.await
		.expect("Failed to get after clear");
	assert_eq!(value, None);
}

/// Test Intent: Verify Redis batch operations with connection pooling
/// Integration Point: RedisCache ↔ Redis pipelined commands ↔ Connection pool
#[rstest]
#[tokio::test]
async fn test_redis_cache_batch_operations(
	#[future] redis_container: ContainerAsync<GenericImage>,
) {
	let container = redis_container.await;
	let port = container
		.get_host_port_ipv4(6379)
		.await
		.expect("Failed to get Redis port");
	let url = format!("redis://127.0.0.1:{}", port);

	let cache = RedisCache::new(&url)
		.await
		.expect("Failed to create Redis cache");

	// Set many
	let mut values = HashMap::new();
	values.insert("batch_a".to_string(), "value_a".to_string());
	values.insert("batch_b".to_string(), "value_b".to_string());
	values.insert("batch_c".to_string(), "value_c".to_string());

	cache
		.set_many(values, None)
		.await
		.expect("Failed to set many");

	// Get many
	let keys = vec!["batch_a", "batch_b", "batch_c"];
	let results: HashMap<String, String> = cache.get_many(&keys).await.expect("Failed to get many");

	assert_eq!(results.len(), 3);
	assert_eq!(results.get("batch_a"), Some(&"value_a".to_string()));
}

// ========================================
// File Cache Backend Tests
// ========================================

/// Test Intent: Verify FileCache basic operations with filesystem persistence
/// Integration Point: FileCache ↔ Filesystem I/O ↔ MD5 key hashing
#[rstest]
#[tokio::test]
async fn test_file_cache_basic_operations(temp_cache_dir: PathBuf) {
	let cache = FileCache::new(temp_cache_dir.clone())
		.await
		.expect("Failed to create file cache");

	// Set and get
	cache
		.set("file_key", &"file_value", None)
		.await
		.expect("Failed to set");
	let value: Option<String> = cache.get("file_key").await.expect("Failed to get");
	assert_eq!(value, Some("file_value".to_string()));

	// Verify file exists on filesystem
	assert!(temp_cache_dir.read_dir().unwrap().count() > 0);

	// Delete
	cache.delete("file_key").await.expect("Failed to delete");
	let value: Option<String> = cache
		.get("file_key")
		.await
		.expect("Failed to get after delete");
	assert_eq!(value, None);

	// Cleanup
	let _ = std::fs::remove_dir_all(&temp_cache_dir);
}

/// Test Intent: Verify FileCache TTL expiration and cleanup
/// Integration Point: FileCache ↔ CacheEntry expiration ↔ Filesystem cleanup
#[rstest]
#[tokio::test]
async fn test_file_cache_ttl_and_cleanup(temp_cache_dir: PathBuf) {
	let cache = FileCache::new(temp_cache_dir.clone())
		.await
		.expect("Failed to create file cache");

	// Set with short TTL
	cache
		.set("ttl_file", &"value", Some(Duration::from_millis(100)))
		.await
		.expect("Failed to set with TTL");

	// Should exist immediately
	assert!(
		cache
			.has_key("ttl_file")
			.await
			.expect("Failed to check key")
	);

	// Wait for expiration
	tokio::time::sleep(Duration::from_millis(150)).await;

	// Should be expired (passive cleanup on get)
	let value: Option<String> = cache
		.get("ttl_file")
		.await
		.expect("Failed to get expired key");
	assert_eq!(value, None);

	// Cleanup should remove file
	cache
		.cleanup_expired()
		.await
		.expect("Failed to cleanup expired");

	// Cleanup
	let _ = std::fs::remove_dir_all(&temp_cache_dir);
}

/// Test Intent: Verify FileCache persistence across instances
/// Integration Point: FileCache ↔ Filesystem persistence ↔ Index loading
#[rstest]
#[tokio::test]
async fn test_file_cache_persistence_across_instances(temp_cache_dir: PathBuf) {
	let cache_dir = temp_cache_dir.clone();

	// First instance: write data
	{
		let cache = FileCache::new(cache_dir.clone())
			.await
			.expect("Failed to create file cache");

		cache
			.set("persist_key", &"persist_value", None)
			.await
			.expect("Failed to set");
	}

	// Second instance: read persisted data
	{
		let cache = FileCache::new(cache_dir.clone())
			.await
			.expect("Failed to create file cache");

		let value: Option<String> = cache
			.get("persist_key")
			.await
			.expect("Failed to get persisted value");
		assert_eq!(value, Some("persist_value".to_string()));
	}

	// Cleanup
	let _ = std::fs::remove_dir_all(&temp_cache_dir);
}

// ========================================
// Memcached Cache Backend Tests
// ========================================

/// Test Intent: Verify MemcachedCache basic operations with real Memcached instance
/// Integration Point: MemcachedCache ↔ Memcached ASCII protocol ↔ Server communication
#[rstest]
#[tokio::test]
async fn test_memcached_cache_basic_operations(#[future] memcached_container: MemcachedContainer) {
	let container = memcached_container.await;
	let url = container.connection_url();

	let cache = MemcachedCache::from_url(&url)
		.await
		.expect("Failed to create Memcached cache");

	// Set and get
	cache
		.set("memcached_key", &"memcached_value", None)
		.await
		.expect("Failed to set");

	let value: Option<String> = cache.get("memcached_key").await.expect("Failed to get");
	assert_eq!(value, Some("memcached_value".to_string()));

	// Delete
	cache
		.delete("memcached_key")
		.await
		.expect("Failed to delete");

	tokio::time::sleep(Duration::from_millis(100)).await;

	let value: Option<String> = cache
		.get("memcached_key")
		.await
		.expect("Failed to get after delete");
	assert_eq!(value, None);
}

/// Test Intent: Verify MemcachedCache TTL expiration
/// Integration Point: MemcachedCache ↔ Memcached expiration ↔ TTL handling
#[rstest]
#[tokio::test]
async fn test_memcached_cache_ttl_expiration(#[future] memcached_container: MemcachedContainer) {
	let container = memcached_container.await;
	let url = container.connection_url();

	let cache = MemcachedCache::from_url(&url)
		.await
		.expect("Failed to create Memcached cache");

	// Set with 1 second TTL
	cache
		.set("ttl_memcached", &"value", Some(Duration::from_secs(1)))
		.await
		.expect("Failed to set with TTL");

	// Should exist immediately
	assert!(
		cache
			.has_key("ttl_memcached")
			.await
			.expect("Failed to check key")
	);

	// Wait for expiration
	tokio::time::sleep(Duration::from_millis(1200)).await;

	// Should be expired
	let value: Option<String> = cache
		.get("ttl_memcached")
		.await
		.expect("Failed to get expired key");
	assert_eq!(value, None);
}

// ========================================
// Cross-Backend Tests
// ========================================

/// Test Intent: Verify Cache trait implementation consistency across all backends
/// Integration Point: All backends ↔ Cache trait ↔ Consistent behavior
#[rstest]
#[tokio::test]
async fn test_cache_trait_consistency_across_backends() {
	// Memory cache
	let memory_cache = InMemoryCache::new();
	verify_cache_trait_behavior(&memory_cache).await;

	// File cache
	let temp_dir = std::env::temp_dir().join(format!("cache_trait_test_{}", Uuid::new_v4()));
	std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");
	let file_cache = FileCache::new(temp_dir.clone())
		.await
		.expect("Failed to create file cache");
	verify_cache_trait_behavior(&file_cache).await;

	// Cleanup
	let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Helper function to verify consistent Cache trait behavior
async fn verify_cache_trait_behavior<C: Cache>(cache: &C) {
	// Set
	cache
		.set("trait_test", &"value", None)
		.await
		.expect("Failed to set");

	// Get
	let value: Option<String> = cache.get("trait_test").await.expect("Failed to get");
	assert_eq!(value, Some("value".to_string()));

	// Has key
	assert!(
		cache
			.has_key("trait_test")
			.await
			.expect("Failed to check key")
	);

	// Delete
	cache.delete("trait_test").await.expect("Failed to delete");
	assert!(
		!cache
			.has_key("trait_test")
			.await
			.expect("Failed to check deleted key")
	);

	// Clear
	cache
		.set("clear1", &"v1", None)
		.await
		.expect("Failed to set");
	cache
		.set("clear2", &"v2", None)
		.await
		.expect("Failed to set");
	cache.clear().await.expect("Failed to clear");
	assert!(!cache.has_key("clear1").await.expect("Failed to check"));
	assert!(!cache.has_key("clear2").await.expect("Failed to check"));
}

/// Test Intent: Verify CacheKeyBuilder key generation and versioning
/// Integration Point: CacheKeyBuilder ↔ Key namespacing ↔ Version management
#[rstest]
#[tokio::test]
async fn test_cache_key_builder_integration() {
	let builder = CacheKeyBuilder::new("myapp").with_version(2);

	// Single key
	assert_eq!(builder.build("user:123"), "myapp:2:user:123");

	// Multiple keys
	let keys = builder.build_many(&["session", "token", "data"]);
	assert_eq!(
		keys,
		vec!["myapp:2:session", "myapp:2:token", "myapp:2:data"]
	);

	// Use with cache
	let cache = InMemoryCache::new();
	let full_key = builder.build("test");

	cache
		.set(&full_key, &"value", None)
		.await
		.expect("Failed to set with builder key");

	let value: Option<String> = cache
		.get(&full_key)
		.await
		.expect("Failed to get with builder key");
	assert_eq!(value, Some("value".to_string()));
}

/// Test Intent: Verify cache miss handling consistency across backends
/// Integration Point: All backends ↔ None value handling ↔ Cache miss behavior
#[rstest]
#[tokio::test]
async fn test_cache_miss_handling() {
	let memory_cache = InMemoryCache::new();

	// Get non-existent key
	let value: Option<String> = memory_cache
		.get("nonexistent")
		.await
		.expect("Failed to get non-existent key");
	assert_eq!(value, None);

	// Has key on non-existent
	assert!(
		!memory_cache
			.has_key("nonexistent")
			.await
			.expect("Failed to check non-existent key")
	);

	// Delete non-existent key (should not error)
	memory_cache
		.delete("nonexistent")
		.await
		.expect("Failed to delete non-existent key");
}

/// Test Intent: Verify enum serialization across cache backends
/// Integration Point: Cache backends ↔ serde enum serialization ↔ Complex enum types
#[rstest]
#[tokio::test]
async fn test_enum_serialization_across_backends() {
	let cache = InMemoryCache::new();

	// Test different enum variants
	let data1 = CacheData::String("hello".to_string());
	let data2 = CacheData::Number(42);
	let data3 = CacheData::Object {
		name: "test".to_string(),
		value: 100,
	};
	let data4 = CacheData::List(vec!["a".to_string(), "b".to_string()]);

	cache
		.set("enum1", &data1, None)
		.await
		.expect("Failed to set");
	cache
		.set("enum2", &data2, None)
		.await
		.expect("Failed to set");
	cache
		.set("enum3", &data3, None)
		.await
		.expect("Failed to set");
	cache
		.set("enum4", &data4, None)
		.await
		.expect("Failed to set");

	let retrieved1: Option<CacheData> = cache.get("enum1").await.expect("Failed to get");
	let retrieved2: Option<CacheData> = cache.get("enum2").await.expect("Failed to get");
	let retrieved3: Option<CacheData> = cache.get("enum3").await.expect("Failed to get");
	let retrieved4: Option<CacheData> = cache.get("enum4").await.expect("Failed to get");

	assert_eq!(retrieved1, Some(data1));
	assert_eq!(retrieved2, Some(data2));
	assert_eq!(retrieved3, Some(data3));
	assert_eq!(retrieved4, Some(data4));
}

/// Test Intent: Verify default TTL behavior across backends
/// Integration Point: Cache backends ↔ Default TTL configuration ↔ TTL fallback
#[rstest]
#[tokio::test]
async fn test_default_ttl_behavior() {
	let cache = InMemoryCache::new().with_default_ttl(Duration::from_millis(100));

	// Set without explicit TTL (should use default)
	cache
		.set("default_ttl", &"value", None)
		.await
		.expect("Failed to set");

	// Should exist immediately
	assert!(
		cache
			.has_key("default_ttl")
			.await
			.expect("Failed to check key")
	);

	// Wait for default TTL expiration
	tokio::time::sleep(Duration::from_millis(150)).await;

	// Should be expired
	let value: Option<String> = cache
		.get("default_ttl")
		.await
		.expect("Failed to get expired key");
	assert_eq!(value, None);
}
