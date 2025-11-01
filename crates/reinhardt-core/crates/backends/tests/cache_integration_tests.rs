//! Integration tests for cache backends using TestContainers
//!
//! These tests verify the cache backend implementations against real
//! infrastructure using Docker containers.

#![cfg(all(feature = "redis-cache", test))]

use reinhardt_backends::cache::{CacheBackend, redis::RedisCache};
use std::time::Duration;
use testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner};
use testcontainers_modules::redis::Redis;

/// Test fixture for Redis cache with TestContainer
struct RedisCacheFixture {
	_container: ContainerAsync<Redis>,
	cache: RedisCache,
}

impl RedisCacheFixture {
	async fn new() -> Self {
		let container = Redis::default()
			.start()
			.await
			.expect("Failed to start Redis container");

		let host = container.get_host().await.expect("Failed to get host");
		let port = container
			.get_host_port_ipv4(6379)
			.await
			.expect("Failed to get port");

		let redis_url = format!("redis://{}:{}", host, port);
		let cache = RedisCache::new(&redis_url)
			.await
			.expect("Failed to create Redis cache");

		Self {
			_container: container,
			cache,
		}
	}
}

#[tokio::test]
async fn test_redis_cache_basic_operations() {
	let fixture = RedisCacheFixture::new().await;
	let cache = &fixture.cache;

	// Test set and get
	cache
		.set("test_key", b"test_value", Some(Duration::from_secs(60)))
		.await
		.expect("Failed to set value");

	let value = cache.get("test_key").await.expect("Failed to get value");
	assert_eq!(value, Some(b"test_value".to_vec()));

	// Test exists
	let exists = cache
		.exists("test_key")
		.await
		.expect("Failed to check exists");
	assert!(exists);

	// Test delete
	let deleted = cache.delete("test_key").await.expect("Failed to delete");
	assert!(deleted);

	let value = cache.get("test_key").await.expect("Failed to get value");
	assert_eq!(value, None);
}

#[tokio::test]
async fn test_redis_cache_ttl_expiration() {
	let fixture = RedisCacheFixture::new().await;
	let cache = &fixture.cache;

	// Set value with 1 second TTL
	cache
		.set("ttl_key", b"value", Some(Duration::from_secs(1)))
		.await
		.expect("Failed to set value");

	// Value should exist immediately
	let exists = cache
		.exists("ttl_key")
		.await
		.expect("Failed to check exists");
	assert!(exists);

	// Wait for expiration
	tokio::time::sleep(Duration::from_secs(2)).await;

	// Value should be expired
	let exists = cache
		.exists("ttl_key")
		.await
		.expect("Failed to check exists");
	assert!(!exists);
}

#[tokio::test]
async fn test_redis_cache_batch_operations() {
	let fixture = RedisCacheFixture::new().await;
	let cache = &fixture.cache;

	// Test set_many
	let items = vec![
		("batch_1".to_string(), b"value_1".to_vec()),
		("batch_2".to_string(), b"value_2".to_vec()),
		("batch_3".to_string(), b"value_3".to_vec()),
		("batch_4".to_string(), b"value_4".to_vec()),
		("batch_5".to_string(), b"value_5".to_vec()),
	];

	cache
		.set_many(&items, Some(Duration::from_secs(60)))
		.await
		.expect("Failed to set many");

	// Test get_many
	let keys = vec![
		"batch_1".to_string(),
		"batch_2".to_string(),
		"batch_3".to_string(),
		"batch_4".to_string(),
		"batch_5".to_string(),
	];

	let values = cache.get_many(&keys).await.expect("Failed to get many");
	assert_eq!(values.len(), 5);
	assert_eq!(values[0], Some(b"value_1".to_vec()));
	assert_eq!(values[1], Some(b"value_2".to_vec()));
	assert_eq!(values[2], Some(b"value_3".to_vec()));
	assert_eq!(values[3], Some(b"value_4".to_vec()));
	assert_eq!(values[4], Some(b"value_5".to_vec()));

	// Test delete_many
	let deleted = cache
		.delete_many(&keys)
		.await
		.expect("Failed to delete many");
	assert_eq!(deleted, 5);

	// Verify all keys are deleted
	let values = cache.get_many(&keys).await.expect("Failed to get many");
	assert!(values.iter().all(|v| v.is_none()));
}

#[tokio::test]
async fn test_redis_cache_get_many_mixed_keys() {
	let fixture = RedisCacheFixture::new().await;
	let cache = &fixture.cache;

	// Set only some keys
	cache
		.set("exists_1", b"value_1", None)
		.await
		.expect("Failed to set");
	cache
		.set("exists_3", b"value_3", None)
		.await
		.expect("Failed to set");

	// Get mixed keys (some exist, some don't)
	let keys = vec![
		"exists_1".to_string(),
		"missing_2".to_string(),
		"exists_3".to_string(),
		"missing_4".to_string(),
	];

	let values = cache.get_many(&keys).await.expect("Failed to get many");
	assert_eq!(values.len(), 4);
	assert_eq!(values[0], Some(b"value_1".to_vec()));
	assert_eq!(values[1], None);
	assert_eq!(values[2], Some(b"value_3".to_vec()));
	assert_eq!(values[3], None);

	// Cleanup
	cache.delete("exists_1").await.expect("Failed to delete");
	cache.delete("exists_3").await.expect("Failed to delete");
}

#[tokio::test]
async fn test_redis_cache_empty_batch_operations() {
	let fixture = RedisCacheFixture::new().await;
	let cache = &fixture.cache;

	// Test empty get_many
	let values = cache
		.get_many(&[])
		.await
		.expect("Failed to get many with empty keys");
	assert!(values.is_empty());

	// Test empty set_many
	cache
		.set_many(&[], None)
		.await
		.expect("Failed to set many with empty items");

	// Test empty delete_many
	let deleted = cache
		.delete_many(&[])
		.await
		.expect("Failed to delete many with empty keys");
	assert_eq!(deleted, 0);
}

#[tokio::test]
async fn test_redis_cache_large_values() {
	let fixture = RedisCacheFixture::new().await;
	let cache = &fixture.cache;

	// Create a large value (1MB)
	let large_value = vec![0u8; 1024 * 1024];

	cache
		.set("large_key", &large_value, Some(Duration::from_secs(60)))
		.await
		.expect("Failed to set large value");

	let retrieved = cache
		.get("large_key")
		.await
		.expect("Failed to get large value");
	assert_eq!(retrieved, Some(large_value));

	cache.delete("large_key").await.expect("Failed to delete");
}

#[tokio::test]
async fn test_redis_cache_concurrent_access() {
	let fixture = RedisCacheFixture::new().await;
	let cache = std::sync::Arc::new(fixture.cache);

	// Spawn multiple concurrent tasks
	let mut handles = vec![];

	for i in 0..10 {
		let cache_clone = cache.clone();
		let handle = tokio::spawn(async move {
			let key = format!("concurrent_{}", i);
			let value = format!("value_{}", i);

			cache_clone
				.set(&key, value.as_bytes(), Some(Duration::from_secs(60)))
				.await
				.expect("Failed to set");

			let retrieved = cache_clone.get(&key).await.expect("Failed to get");
			assert_eq!(retrieved, Some(value.as_bytes().to_vec()));

			cache_clone.delete(&key).await.expect("Failed to delete");
		});
		handles.push(handle);
	}

	// Wait for all tasks to complete
	for handle in handles {
		handle.await.expect("Task panicked");
	}
}

#[tokio::test]
async fn test_redis_cache_overwrite_value() {
	let fixture = RedisCacheFixture::new().await;
	let cache = &fixture.cache;

	// Set initial value
	cache
		.set("overwrite_key", b"initial", None)
		.await
		.expect("Failed to set initial value");

	let value = cache
		.get("overwrite_key")
		.await
		.expect("Failed to get value");
	assert_eq!(value, Some(b"initial".to_vec()));

	// Overwrite with new value
	cache
		.set("overwrite_key", b"updated", None)
		.await
		.expect("Failed to overwrite value");

	let value = cache
		.get("overwrite_key")
		.await
		.expect("Failed to get value");
	assert_eq!(value, Some(b"updated".to_vec()));

	cache
		.delete("overwrite_key")
		.await
		.expect("Failed to delete");
}

#[tokio::test]
async fn test_redis_cache_special_characters() {
	let fixture = RedisCacheFixture::new().await;
	let cache = &fixture.cache;

	// Test with special characters in key
	let special_key = "key:with:colons:and-dashes_underscores.dots";
	let special_value = b"Special \x00\x01\x02\xFF value with bytes";

	cache
		.set(special_key, special_value, Some(Duration::from_secs(60)))
		.await
		.expect("Failed to set special value");

	let value = cache.get(special_key).await.expect("Failed to get value");
	assert_eq!(value, Some(special_value.to_vec()));

	cache.delete(special_key).await.expect("Failed to delete");
}

#[tokio::test]
async fn test_redis_cache_clear() {
	let fixture = RedisCacheFixture::new().await;
	let cache = &fixture.cache;

	// Set multiple keys
	for i in 0..5 {
		cache
			.set(&format!("clear_test_{}", i), b"value", None)
			.await
			.expect("Failed to set");
	}

	// Verify keys exist
	for i in 0..5 {
		let exists = cache
			.exists(&format!("clear_test_{}", i))
			.await
			.expect("Failed to check exists");
		assert!(exists);
	}

	// Clear all keys
	cache.clear().await.expect("Failed to clear cache");

	// Verify all keys are gone
	for i in 0..5 {
		let exists = cache
			.exists(&format!("clear_test_{}", i))
			.await
			.expect("Failed to check exists");
		assert!(!exists);
	}
}
