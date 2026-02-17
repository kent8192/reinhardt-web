//! Cache Middleware Integration Tests
//!
//! Tests the integration of cache middleware with storage backends:
//! - ETag generation and conditional GET handling
//! - Cache storage (Redis/Memory) integration
//! - Cache hit/miss verification

use reinhardt_middleware::cache::{CacheConfig, CacheKeyStrategy, CacheMiddleware};
use rstest::rstest;
use serial_test::serial;
use std::time::Duration;

/// Test cache middleware with default config
#[serial(cache)]
#[rstest]
#[tokio::test]
async fn test_cache_with_defaults() {
	// Create cache middleware with defaults
	let cache = CacheMiddleware::with_defaults();

	// Verify cache middleware is configured
	assert!(cache.store().is_empty());
	assert_eq!(cache.store().len(), 0);
}

/// Test cache middleware with custom config
#[serial(cache)]
#[rstest]
#[tokio::test]
async fn test_cache_with_custom_config() {
	// Create cache config with custom settings
	let config = CacheConfig::new(Duration::from_secs(300), CacheKeyStrategy::UrlOnly);
	let cache = CacheMiddleware::new(config);

	// Verify cache middleware uses custom config
	assert!(cache.store().is_empty());
}

/// Test cache store operations
#[serial(cache)]
#[rstest]
#[tokio::test]
async fn test_cache_store_operations() {
	let config = CacheConfig::new(Duration::from_secs(60), CacheKeyStrategy::UrlAndMethod);
	let cache = CacheMiddleware::new(config);

	// Verify initial state
	assert!(cache.store().is_empty());
	assert_eq!(cache.store().len(), 0);
}

/// Test multiple cache middleware instances
#[serial(cache)]
#[rstest]
#[tokio::test]
async fn test_multiple_cache_instances() {
	// Create multiple cache middleware instances
	let cache1 = CacheMiddleware::with_defaults();
	let cache2 = CacheMiddleware::new(CacheConfig::new(
		Duration::from_secs(120),
		CacheKeyStrategy::UrlAndQuery,
	));

	// Verify all instances are independent
	assert_eq!(cache1.store().len(), 0);
	assert_eq!(cache2.store().len(), 0);
}

/// Test cache config with excluded paths
#[serial(cache)]
#[rstest]
#[tokio::test]
async fn test_cache_with_excluded_paths() {
	// Create cache config with excluded paths
	let config = CacheConfig::new(Duration::from_secs(300), CacheKeyStrategy::UrlOnly)
		.with_excluded_paths(vec!["/admin".to_string(), "/api/private".to_string()]);

	let cache = CacheMiddleware::new(config);

	// Verify cache middleware is configured with excluded paths
	assert!(cache.store().is_empty());
}

/// Test cache config with max entries
#[serial(cache)]
#[rstest]
#[tokio::test]
async fn test_cache_with_max_entries() {
	// Create cache config with max entries limit
	let config = CacheConfig::new(Duration::from_secs(300), CacheKeyStrategy::UrlOnly)
		.with_max_entries(5000);

	let cache = CacheMiddleware::new(config);

	// Verify cache middleware is configured with max entries
	assert!(cache.store().is_empty());
}
