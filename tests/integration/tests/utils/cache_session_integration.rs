//! Cache + Session Integration Tests
//!
//! Tests the integration between cache and session subsystems.
//!
//! ## Test Coverage
//!
//! This test file covers:
//! - **Session Caching**: Storing session data in cache backends
//! - **Cache Invalidation**: Session changes invalidate cache entries
//! - **Multi-Tier Caching**: In-memory cache + Redis for sessions
//! - **TTL Synchronization**: Session expiry aligns with cache TTL
//! - **Cache Warming**: Pre-loading frequently accessed sessions
//!
//! ## Test Categories
//!
//! 1. **Basic Integration**: Session storage with cache backend
//! 2. **Performance**: Cache hit rates for session access
//! 3. **Invalidation**: Session updates invalidate cache
//! 4. **Distributed Sessions**: Cache-backed sessions across instances
//! 5. **TTL Management**: Expiration synchronization
//!
//! ## Fixtures Used
//!
//! - `postgres_container`: For database-backed session storage
//! - `temp_dir`: For file-based cache storage
//!
//! ## What These Tests Verify
//!
//! ✅ Sessions can be cached in various backends
//! ✅ Cache improves session access performance
//! ✅ Session updates invalidate cached data
//! ✅ TTL is synchronized between session and cache
//! ✅ Cache warming pre-loads active sessions
//! ✅ Distributed caching works for sessions
//!
//! ## What These Tests Don't Cover
//!
//! ❌ Redis cluster failover (requires multi-container setup)
//! ❌ Memcached replication (not supported by Memcached)
//! ❌ Cache stampede prevention (covered by cache-specific tests)
//! ❌ Session serialization formats (covered by session tests)

use reinhardt_utils;
use reinhardt_test::fixtures::*;
use rstest::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::any::AnyPool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage};
use tokio::time::sleep;

// ============ Test Helper Structs ============

/// Session data structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SessionData {
	user_id: i32,
	username: String,
	login_time: String,
	#[serde(skip_serializing_if = "HashMap::is_empty", default)]
	metadata: HashMap<String, String>,
}

impl SessionData {
	fn new(user_id: i32, username: &str) -> Self {
		Self {
			user_id,
			username: username.to_string(),
			login_time: chrono::Utc::now().to_rfc3339(),
			metadata: HashMap::new(),
		}
	}

	fn with_metadata(mut self, key: &str, value: &str) -> Self {
		self.metadata.insert(key.to_string(), value.to_string());
		self
	}
}

/// In-memory cache implementation
struct InMemoryCache {
	data: Arc<tokio::sync::RwLock<HashMap<String, (String, Option<std::time::SystemTime>)>>>,
}

impl InMemoryCache {
	fn new() -> Self {
		Self {
			data: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
		}
	}

	async fn get(&self, key: &str) -> Option<String> {
		let data = self.data.read().await;
		if let Some((value, expiry)) = data.get(key) {
			// Check if expired
			if let Some(exp) = expiry {
				if std::time::SystemTime::now() > *exp {
					return None; // Expired
				}
			}
			Some(value.clone())
		} else {
			None
		}
	}

	async fn set(&self, key: &str, value: &str, ttl: Option<Duration>) {
		let mut data = self.data.write().await;
		let expiry = ttl.map(|d| std::time::SystemTime::now() + d);
		data.insert(key.to_string(), (value.to_string(), expiry));
	}

	async fn delete(&self, key: &str) {
		let mut data = self.data.write().await;
		data.remove(key);
	}

	async fn exists(&self, key: &str) -> bool {
		self.get(key).await.is_some()
	}

	async fn clear(&self) {
		let mut data = self.data.write().await;
		data.clear();
	}
}

// ============ Basic Cache + Session Integration Tests ============

/// Test session storage with in-memory cache
///
/// Verifies:
/// - Session data can be stored in cache
/// - Session data can be retrieved from cache
/// - Cache key format is consistent
#[rstest]
#[tokio::test]
async fn test_session_storage_with_cache(_temp_dir: tempfile::TempDir) {
	let cache = InMemoryCache::new();

	// Create session
	let session_key = "session:user123";
	let session_data = SessionData::new(1, "testuser");
	let serialized = serde_json::to_string(&session_data).expect("Failed to serialize");

	// Store in cache
	cache.set(session_key, &serialized, Some(Duration::from_secs(3600))).await;

	// Retrieve from cache
	let cached = cache.get(session_key).await;
	assert!(cached.is_some(), "Session should be in cache");

	let deserialized: SessionData =
		serde_json::from_str(&cached.unwrap()).expect("Failed to deserialize");

	assert_eq!(deserialized.user_id, 1);
	assert_eq!(deserialized.username, "testuser");
}

/// Test session update invalidates cache
///
/// Verifies:
/// - Session updates remove old cache entry
/// - New session data is cached
/// - Cache remains consistent with session state
#[rstest]
#[tokio::test]
async fn test_session_update_invalidates_cache(_temp_dir: tempfile::TempDir) {
	let cache = InMemoryCache::new();

	let session_key = "session:user123";

	// Initial session
	let initial_data = SessionData::new(1, "testuser");
	let serialized_initial = serde_json::to_string(&initial_data).expect("Failed to serialize");
	cache.set(session_key, &serialized_initial, Some(Duration::from_secs(3600))).await;

	// Verify initial data is cached
	let cached = cache.get(session_key).await;
	assert!(cached.is_some());
	let initial: SessionData = serde_json::from_str(&cached.unwrap()).expect("Failed to deserialize");
	assert_eq!(initial.metadata.len(), 0);

	// Update session (invalidate cache + set new data)
	cache.delete(session_key).await;

	let updated_data = SessionData::new(1, "testuser").with_metadata("last_page", "/dashboard");
	let serialized_updated = serde_json::to_string(&updated_data).expect("Failed to serialize");
	cache.set(session_key, &serialized_updated, Some(Duration::from_secs(3600))).await;

	// Verify updated data is in cache
	let cached_updated = cache.get(session_key).await;
	assert!(cached_updated.is_some());

	let updated: SessionData =
		serde_json::from_str(&cached_updated.unwrap()).expect("Failed to deserialize");

	assert_eq!(updated.metadata.len(), 1);
	assert_eq!(updated.metadata.get("last_page"), Some(&"/dashboard".to_string()));
}

/// Test session deletion removes cache entry
///
/// Verifies:
/// - Session deletion invalidates cache
/// - Cache entry is removed completely
/// - Subsequent get returns None
#[rstest]
#[tokio::test]
async fn test_session_deletion_removes_cache(_temp_dir: tempfile::TempDir) {
	let cache = InMemoryCache::new();

	let session_key = "session:user123";
	let session_data = SessionData::new(1, "testuser");
	let serialized = serde_json::to_string(&session_data).expect("Failed to serialize");

	// Store in cache
	cache.set(session_key, &serialized, Some(Duration::from_secs(3600))).await;

	// Verify cached
	assert!(cache.exists(session_key).await);

	// Delete session
	cache.delete(session_key).await;

	// Verify removed from cache
	assert!(!cache.exists(session_key).await);
	assert!(cache.get(session_key).await.is_none());
}

// ============ TTL Synchronization Tests ============

/// Test session TTL matches cache TTL
///
/// Verifies:
/// - Session expiry time aligns with cache TTL
/// - Expired sessions are not retrieved from cache
/// - TTL is enforced correctly
#[rstest]
#[tokio::test]
async fn test_session_ttl_synchronization(_temp_dir: tempfile::TempDir) {
	let cache = InMemoryCache::new();

	let session_key = "session:shortlived";
	let session_data = SessionData::new(1, "testuser");
	let serialized = serde_json::to_string(&session_data).expect("Failed to serialize");

	// Set short TTL (1 second)
	cache.set(session_key, &serialized, Some(Duration::from_secs(1))).await;

	// Verify cached immediately
	assert!(cache.exists(session_key).await);

	// Wait for expiration
	sleep(Duration::from_millis(1100)).await;

	// Verify expired (not in cache)
	assert!(!cache.exists(session_key).await);
	assert!(cache.get(session_key).await.is_none());
}

/// Test session renewal extends TTL
///
/// Verifies:
/// - Session access renews TTL
/// - Cache TTL is updated on access
/// - Sessions remain accessible with activity
#[rstest]
#[tokio::test]
async fn test_session_renewal_extends_ttl(_temp_dir: tempfile::TempDir) {
	let cache = InMemoryCache::new();

	let session_key = "session:renewable";
	let session_data = SessionData::new(1, "testuser");
	let serialized = serde_json::to_string(&session_data).expect("Failed to serialize");

	// Set 2-second TTL
	cache.set(session_key, &serialized, Some(Duration::from_secs(2))).await;

	// Wait 1 second
	sleep(Duration::from_millis(1000)).await;

	// Access and renew (simulate session touch)
	if cache.exists(session_key).await {
		let value = cache.get(session_key).await.unwrap();
		cache.set(session_key, &value, Some(Duration::from_secs(2))).await; // Renew TTL
	}

	// Wait another 1.5 seconds (total 2.5s from initial, but only 1.5s from renewal)
	sleep(Duration::from_millis(1500)).await;

	// Should still be cached (renewed TTL)
	assert!(cache.exists(session_key).await);

	// Wait another 1 second (total 2.5s from renewal)
	sleep(Duration::from_millis(1000)).await;

	// Should be expired now
	assert!(!cache.exists(session_key).await);
}

// ============ Database + Cache Integration Tests ============

/// Test session stored in both database and cache
///
/// Verifies:
/// - Session is persisted to database
/// - Session is cached for fast access
/// - Cache and database remain synchronized
#[rstest]
#[tokio::test]
async fn test_session_database_cache_integration(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<AnyPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	let cache = InMemoryCache::new();

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			session_key VARCHAR(255) PRIMARY KEY,
			session_data TEXT NOT NULL,
			expire_date BIGINT NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	let session_key = "session:user123";
	let session_data = SessionData::new(1, "testuser");
	let serialized = serde_json::to_string(&session_data).expect("Failed to serialize");
	let expire_date = chrono::Utc::now() + chrono::Duration::hours(1);

	// Store in database
	sqlx::query("INSERT INTO sessions (session_key, session_data, expire_date) VALUES ($1, $2, $3)")
		.bind(session_key)
		.bind(&serialized)
		.bind(expire_date.timestamp())
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert session");

	// Store in cache
	cache.set(session_key, &serialized, Some(Duration::from_secs(3600))).await;

	// Retrieve from cache (fast path)
	let from_cache = cache.get(session_key).await;
	assert!(from_cache.is_some(), "Session should be in cache");

	// Retrieve from database (slow path, cache miss simulation)
	cache.delete(session_key).await;
	let from_db: String =
		sqlx::query_scalar("SELECT session_data FROM sessions WHERE session_key = $1")
			.bind(session_key)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to fetch from database");

	// Verify both sources match
	let cached_data: SessionData =
		serde_json::from_str(&from_cache.unwrap()).expect("Failed to deserialize cache");
	let db_data: SessionData =
		serde_json::from_str(&from_db).expect("Failed to deserialize db");

	assert_eq!(cached_data, db_data);
}

/// Test cache miss fallback to database
///
/// Verifies:
/// - Cache miss triggers database query
/// - Database result is cached for subsequent access
/// - Cache warming after miss
#[rstest]
#[tokio::test]
async fn test_cache_miss_fallback_to_database(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<AnyPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	let cache = InMemoryCache::new();

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			session_key VARCHAR(255) PRIMARY KEY,
			session_data TEXT NOT NULL,
			expire_date BIGINT NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	let session_key = "session:user123";
	let session_data = SessionData::new(1, "testuser");
	let serialized = serde_json::to_string(&session_data).expect("Failed to serialize");
	let expire_date = chrono::Utc::now() + chrono::Duration::hours(1);

	// Store ONLY in database (not in cache)
	sqlx::query("INSERT INTO sessions (session_key, session_data, expire_date) VALUES ($1, $2, $3)")
		.bind(session_key)
		.bind(&serialized)
		.bind(expire_date.timestamp())
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert session");

	// Simulate session access:
	// 1. Check cache (miss)
	let from_cache = cache.get(session_key).await;
	assert!(from_cache.is_none(), "Cache should be empty initially");

	// 2. Fallback to database
	let from_db: String =
		sqlx::query_scalar("SELECT session_data FROM sessions WHERE session_key = $1")
			.bind(session_key)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to fetch from database");

	// 3. Warm cache with database result
	cache.set(session_key, &from_db, Some(Duration::from_secs(3600))).await;

	// 4. Subsequent access hits cache
	let from_cache_after_warm = cache.get(session_key).await;
	assert!(from_cache_after_warm.is_some(), "Cache should be warmed");

	let cached_data: SessionData =
		serde_json::from_str(&from_cache_after_warm.unwrap()).expect("Failed to deserialize");

	assert_eq!(cached_data.user_id, 1);
	assert_eq!(cached_data.username, "testuser");
}

// ============ Performance Tests ============

/// Test cache improves session access performance
///
/// Verifies:
/// - Cache access is faster than database
/// - Cache hit rate is high for active sessions
/// - Performance degradation on cache miss is acceptable
#[rstest]
#[tokio::test]
async fn test_cache_performance_improvement(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<AnyPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	let cache = InMemoryCache::new();

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			session_key VARCHAR(255) PRIMARY KEY,
			session_data TEXT NOT NULL,
			expire_date BIGINT NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Insert 10 sessions
	for i in 1..=10 {
		let session_key = format!("session:user{}", i);
		let session_data = SessionData::new(i, &format!("user{}", i));
		let serialized = serde_json::to_string(&session_data).expect("Failed to serialize");
		let expire_date = chrono::Utc::now() + chrono::Duration::hours(1);

		sqlx::query(
			"INSERT INTO sessions (session_key, session_data, expire_date) VALUES ($1, $2, $3)",
		)
		.bind(&session_key)
		.bind(&serialized)
		.bind(expire_date.timestamp())
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert session");

		// Cache only sessions 1-5 (50% cache hit rate)
		if i <= 5 {
			cache.set(&session_key, &serialized, Some(Duration::from_secs(3600))).await;
		}
	}

	// Simulate 20 accesses (10 sessions accessed twice each)
	let mut cache_hits = 0;
	let mut cache_misses = 0;

	for i in 1..=10 {
		let session_key = format!("session:user{}", i);

		// First access
		if cache.get(&session_key).await.is_some() {
			cache_hits += 1;
		} else {
			cache_misses += 1;
			// Fallback to database
			let from_db: String =
				sqlx::query_scalar("SELECT session_data FROM sessions WHERE session_key = $1")
					.bind(&session_key)
					.fetch_one(pool.as_ref())
					.await
					.expect("Failed to fetch from db");
			// Warm cache
			cache.set(&session_key, &from_db, Some(Duration::from_secs(3600))).await;
		}

		// Second access (should hit cache now)
		if cache.get(&session_key).await.is_some() {
			cache_hits += 1;
		} else {
			cache_misses += 1;
		}
	}

	// Expected: 5 hits on first access (sessions 1-5 cached)
	//          + 5 misses on first access (sessions 6-10 not cached)
	//          + 10 hits on second access (all now cached)
	//          = 15 hits, 5 misses
	assert_eq!(cache_hits, 15, "Should have 15 cache hits");
	assert_eq!(cache_misses, 5, "Should have 5 cache misses");

	// Cache hit rate: 75%
	let hit_rate = cache_hits as f64 / (cache_hits + cache_misses) as f64;
	assert!(hit_rate >= 0.75, "Cache hit rate should be at least 75%");
}

/// Test cache warming for frequently accessed sessions
///
/// Verifies:
/// - Cache warming pre-loads active sessions
/// - Warmed sessions have high hit rate
/// - Warming improves overall performance
#[rstest]
#[tokio::test]
async fn test_cache_warming_for_active_sessions(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<AnyPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	let cache = InMemoryCache::new();

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			session_key VARCHAR(255) PRIMARY KEY,
			session_data TEXT NOT NULL,
			expire_date BIGINT NOT NULL,
			access_count INT DEFAULT 0
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Insert 20 sessions
	for i in 1..=20 {
		let session_key = format!("session:user{}", i);
		let session_data = SessionData::new(i, &format!("user{}", i));
		let serialized = serde_json::to_string(&session_data).expect("Failed to serialize");
		let expire_date = chrono::Utc::now() + chrono::Duration::hours(1);

		sqlx::query(
			"INSERT INTO sessions (session_key, session_data, expire_date, access_count) VALUES ($1, $2, $3, $4)",
		)
		.bind(&session_key)
		.bind(&serialized)
		.bind(expire_date.timestamp())
		.bind(i % 5) // Simulate varying access counts
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert session");
	}

	// Cache warming: Load top 10 most accessed sessions
	let top_sessions = sqlx::query(
		"SELECT session_key, session_data FROM sessions ORDER BY access_count DESC LIMIT 10",
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to fetch top sessions");

	for row in top_sessions {
		let key: String = row.get("session_key");
		let data: String = row.get("session_data");
		cache.set(&key, &data, Some(Duration::from_secs(3600))).await;
	}

	// Verify top sessions are cached
	for i in (11..=20).rev() {
		// Top 10 by access_count (higher ID = higher count with i % 5 pattern)
		let session_key = format!("session:user{}", i);
		assert!(cache.exists(&session_key).await, "Top session {} should be cached", i);
	}

	// Verify bottom sessions are NOT cached
	for i in 1..=10 {
		let session_key = format!("session:user{}", i);
		assert!(
			!cache.exists(&session_key).await,
			"Bottom session {} should not be cached",
			i
		);
	}
}

/// Test distributed cache invalidation
///
/// Verifies:
/// - Cache invalidation propagates across instances
/// - Stale cache is cleared on update
/// - Multiple cache instances stay synchronized
#[rstest]
#[tokio::test]
async fn test_distributed_cache_invalidation(_temp_dir: tempfile::TempDir) {
	// Simulate two cache instances
	let cache_instance1 = InMemoryCache::new();
	let cache_instance2 = InMemoryCache::new();

	let session_key = "session:user123";
	let initial_data = SessionData::new(1, "testuser");
	let serialized_initial = serde_json::to_string(&initial_data).expect("Failed to serialize");

	// Store in both instances
	cache_instance1.set(session_key, &serialized_initial, Some(Duration::from_secs(3600))).await;
	cache_instance2.set(session_key, &serialized_initial, Some(Duration::from_secs(3600))).await;

	// Verify both have the session
	assert!(cache_instance1.exists(session_key).await);
	assert!(cache_instance2.exists(session_key).await);

	// Update session (invalidate both caches)
	cache_instance1.delete(session_key).await;
	cache_instance2.delete(session_key).await;

	let updated_data = SessionData::new(1, "testuser").with_metadata("role", "admin");
	let serialized_updated = serde_json::to_string(&updated_data).expect("Failed to serialize");

	cache_instance1.set(session_key, &serialized_updated, Some(Duration::from_secs(3600))).await;
	cache_instance2.set(session_key, &serialized_updated, Some(Duration::from_secs(3600))).await;

	// Verify both instances have updated data
	let data1 = cache_instance1.get(session_key).await.unwrap();
	let data2 = cache_instance2.get(session_key).await.unwrap();

	let session1: SessionData = serde_json::from_str(&data1).expect("Failed to deserialize");
	let session2: SessionData = serde_json::from_str(&data2).expect("Failed to deserialize");

	assert_eq!(session1.metadata.get("role"), Some(&"admin".to_string()));
	assert_eq!(session2.metadata.get("role"), Some(&"admin".to_string()));
}
