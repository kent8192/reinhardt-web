//! Redis Backend Infrastructure Integration Tests
//!
//! This test module validates the integration of DynamicSettings with Redis backend
//! using TestContainers to spin up a real Redis instance.
//!
//! ## Test Categories
//!
//! 1. **Redis Connection**: Basic connectivity with real Redis container
//! 2. **Get/Set Operations**: Read and write operations with Redis backend
//! 3. **Key Expiration**: TTL and expiration functionality
//! 4. **Concurrent Access**: Multiple connections accessing same keys
//! 5. **Error Handling**: Network failures, connection issues
//!
//! ## Infrastructure
//!
//! - Uses TestContainers to start Redis 7.x in Docker
//! - Each test gets isolated Redis instance
//! - Automatic cleanup after test completion

#[cfg(all(feature = "async", feature = "dynamic-redis"))]
mod redis_integration_tests {
	use reinhardt_settings::backends::redis_backend::RedisSettingsBackend;
	use reinhardt_settings::dynamic::DynamicSettings;
	use rstest::*;
	use serial_test::serial;
	use std::sync::Arc;
	use std::time::Duration;
	use testcontainers::{ContainerAsync, GenericImage};
	use tokio::time::sleep;

	/// Fixture: Start Redis container for testing
	///
	/// Returns: (container, redis_url)
	#[fixture]
	async fn redis_container() -> (ContainerAsync<GenericImage>, String) {
		use testcontainers::runners::AsyncRunner;

		let redis_image = GenericImage::new("redis", "7-alpine").with_exposed_port(6379.into());

		let container = AsyncRunner::start(redis_image)
			.await
			.expect("Failed to start Redis container");

		let host_port = container
			.get_host_port_ipv4(6379)
			.await
			.expect("Failed to get Redis port");

		let redis_url = format!("redis://127.0.0.1:{}", host_port);

		// Wait for Redis to be ready
		sleep(Duration::from_secs(1)).await;

		(container, redis_url)
	}

	/// Test: Redis backend basic connectivity
	///
	/// Why: Verifies that RedisSettingsBackend can connect to real Redis instance.
	#[rstest]
	#[serial(redis)]
	#[tokio::test]
	async fn test_redis_backend_connectivity(
		#[future] redis_container: (ContainerAsync<GenericImage>, String),
	) {
		let (_container, redis_url) = redis_container.await;

		let backend = RedisSettingsBackend::new(&redis_url)
			.await
			.expect("Failed to create Redis backend");

		let dynamic = DynamicSettings::new(Arc::new(backend));

		// Just verify it can be created without errors
		let result = dynamic.get::<String>("nonexistent_key").await;
		assert!(result.is_ok(), "Redis backend should be operational");
	}

	/// Test: Redis backend set and get operations
	///
	/// Why: Verifies that values can be stored and retrieved from Redis.
	#[rstest]
	#[serial(redis)]
	#[tokio::test]
	async fn test_redis_backend_set_get(
		#[future] redis_container: (ContainerAsync<GenericImage>, String),
	) {
		let (_container, redis_url) = redis_container.await;

		let backend = RedisSettingsBackend::new(&redis_url)
			.await
			.expect("Failed to create Redis backend");

		let dynamic = DynamicSettings::new(Arc::new(backend));

		// Set value
		dynamic
			.set("test.key", &"test_value", None)
			.await
			.expect("Failed to set value");

		// Get value
		let value: Option<String> = dynamic.get("test.key").await.expect("Failed to get value");

		assert_eq!(
			value,
			Some("test_value".to_string()),
			"Redis backend should persist values"
		);
	}

	/// Test: Redis backend with TTL (time-to-live)
	///
	/// Why: Verifies that key expiration works correctly with Redis.
	#[rstest]
	#[serial(redis)]
	#[tokio::test]
	async fn test_redis_backend_ttl(
		#[future] redis_container: (ContainerAsync<GenericImage>, String),
	) {
		let (_container, redis_url) = redis_container.await;

		let backend = RedisSettingsBackend::new(&redis_url)
			.await
			.expect("Failed to create Redis backend");

		let dynamic = DynamicSettings::new(Arc::new(backend));

		// Set value with 2 second TTL (in seconds as u64)
		dynamic
			.set("expiring.key", &"temporary_value", Some(2))
			.await
			.expect("Failed to set value with TTL");

		// Immediately get value (should exist)
		let value1: Option<String> = dynamic
			.get("expiring.key")
			.await
			.expect("Failed to get value");
		assert_eq!(
			value1,
			Some("temporary_value".to_string()),
			"Value should exist immediately"
		);

		// Wait for expiration (3 seconds to be safe)
		sleep(Duration::from_secs(3)).await;

		// Get value after expiration (should be None)
		let value2: Option<String> = dynamic
			.get("expiring.key")
			.await
			.expect("Failed to get value");
		assert_eq!(value2, None, "Value should have expired");
	}

	/// Test: Redis backend with different value types
	///
	/// Why: Verifies that Redis backend correctly serializes/deserializes different types.
	#[rstest]
	#[serial(redis)]
	#[tokio::test]
	async fn test_redis_backend_value_types(
		#[future] redis_container: (ContainerAsync<GenericImage>, String),
	) {
		let (_container, redis_url) = redis_container.await;

		let backend = RedisSettingsBackend::new(&redis_url)
			.await
			.expect("Failed to create Redis backend");

		let dynamic = DynamicSettings::new(Arc::new(backend));

		// String
		dynamic.set("string_key", &"hello", None).await.unwrap();
		let s: Option<String> = dynamic.get("string_key").await.unwrap();
		assert_eq!(s, Some("hello".to_string()));

		// Integer
		dynamic.set("int_key", &42, None).await.unwrap();
		let i: Option<i64> = dynamic.get("int_key").await.unwrap();
		assert_eq!(i, Some(42));

		// Boolean
		dynamic.set("bool_key", &true, None).await.unwrap();
		let b: Option<bool> = dynamic.get("bool_key").await.unwrap();
		assert_eq!(b, Some(true));

		// Array
		let arr = vec!["a", "b", "c"];
		dynamic.set("array_key", &arr, None).await.unwrap();
		let a: Option<Vec<String>> = dynamic.get("array_key").await.unwrap();
		assert_eq!(
			a,
			Some(vec!["a".to_string(), "b".to_string(), "c".to_string()])
		);
	}

	/// Test: Redis backend concurrent access
	///
	/// Why: Verifies that multiple connections can access Redis concurrently without issues.
	#[rstest]
	#[serial(redis)]
	#[tokio::test]
	async fn test_redis_backend_concurrent_access(
		#[future] redis_container: (ContainerAsync<GenericImage>, String),
	) {
		let (_container, redis_url) = redis_container.await;

		let backend = Arc::new(
			RedisSettingsBackend::new(&redis_url)
				.await
				.expect("Failed to create Redis backend"),
		);

		let dynamic = Arc::new(DynamicSettings::new(backend));

		// Spawn multiple concurrent tasks
		let mut handles = vec![];

		for i in 0..10 {
			let dynamic_clone = Arc::clone(&dynamic);
			let handle = tokio::spawn(async move {
				let key = format!("concurrent.key.{}", i);
				let value = format!("value_{}", i);

				// Set value
				dynamic_clone
					.set(&key, &value, None)
					.await
					.expect("Failed to set value");

				// Get value
				let retrieved: Option<String> =
					dynamic_clone.get(&key).await.expect("Failed to get value");

				assert_eq!(retrieved, Some(value));
			});
			handles.push(handle);
		}

		// Wait for all tasks to complete
		for handle in handles {
			handle.await.expect("Task panicked");
		}
	}

	/// Test: Redis backend update existing key
	///
	/// Why: Verifies that values can be updated in Redis.
	#[rstest]
	#[serial(redis)]
	#[tokio::test]
	async fn test_redis_backend_update(
		#[future] redis_container: (ContainerAsync<GenericImage>, String),
	) {
		let (_container, redis_url) = redis_container.await;

		let backend = RedisSettingsBackend::new(&redis_url)
			.await
			.expect("Failed to create Redis backend");

		let dynamic = DynamicSettings::new(Arc::new(backend));

		// Set initial value
		dynamic
			.set("update.key", &"initial", None)
			.await
			.expect("Failed to set initial value");

		let value1: Option<String> = dynamic.get("update.key").await.unwrap();
		assert_eq!(value1, Some("initial".to_string()));

		// Update value
		dynamic
			.set("update.key", &"updated", None)
			.await
			.expect("Failed to update value");

		let value2: Option<String> = dynamic.get("update.key").await.unwrap();
		assert_eq!(
			value2,
			Some("updated".to_string()),
			"Value should be updated"
		);
	}

	/// Test: Redis backend get nonexistent key
	///
	/// Why: Verifies that getting nonexistent key returns None gracefully.
	#[rstest]
	#[serial(redis)]
	#[tokio::test]
	async fn test_redis_backend_get_nonexistent(
		#[future] redis_container: (ContainerAsync<GenericImage>, String),
	) {
		let (_container, redis_url) = redis_container.await;

		let backend = RedisSettingsBackend::new(&redis_url)
			.await
			.expect("Failed to create Redis backend");

		let dynamic = DynamicSettings::new(Arc::new(backend));

		// Get nonexistent key
		let value: Option<String> = dynamic
			.get("nonexistent.key")
			.await
			.expect("Should not error on nonexistent key");

		assert_eq!(value, None, "Nonexistent key should return None");
	}
}
