//! Integration tests for Dynamic Backend Independence.
//!
//! This test module validates that multiple DynamicSettings instances with different
//! backends can coexist and operate independently without interfering with each other.
//!
//! ## Testing Strategy
//!
//! Since `switch_backend()` is not implemented (backend field is immutable),
//! this test validates the practical alternative: using multiple DynamicSettings
//! instances with different backends simultaneously.
//!
//! NOTE: These tests are feature-gated with "async" feature.

#![cfg(feature = "async")]

use reinhardt_conf::settings::backends::MemoryBackend;
use reinhardt_conf::settings::dynamic::DynamicSettings;
use rstest::*;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

/// Test: Multiple DynamicSettings instances with MemoryBackend operate independently
///
/// Why: Validates that multiple instances don't share state or interfere with each other.
#[rstest]
#[tokio::test]
async fn test_multiple_memory_backends_independence() {
	let backend1 = Arc::new(MemoryBackend::new());
	let backend2 = Arc::new(MemoryBackend::new());

	let settings1 = DynamicSettings::new(backend1);
	let settings2 = DynamicSettings::new(backend2);

	// Set different values in each instance
	settings1
		.set("key", &"value1", None)
		.await
		.expect("Set in settings1 should succeed");

	settings2
		.set("key", &"value2", None)
		.await
		.expect("Set in settings2 should succeed");

	// Verify independence
	let result1: String = settings1
		.get("key")
		.await
		.expect("Get should succeed")
		.expect("Value should exist");

	let result2: String = settings2
		.get("key")
		.await
		.expect("Get should succeed")
		.expect("Value should exist");

	assert_eq!(result1, "value1", "settings1 should have value1");
	assert_eq!(result2, "value2", "settings2 should have value2");
}

/// Test: Different backends can have different TTLs
///
/// Why: Validates that TTL configurations are independent per instance.
#[rstest]
#[tokio::test]
async fn test_different_ttls_per_backend() {
	let backend1 = Arc::new(MemoryBackend::new());
	let backend2 = Arc::new(MemoryBackend::new());

	let settings1 = DynamicSettings::new(backend1);
	let settings2 = DynamicSettings::new(backend2);

	// settings1: 1 second TTL
	settings1
		.set("key", &"value1", Some(1))
		.await
		.expect("Set with TTL 1 should succeed");

	// settings2: 3 seconds TTL
	settings2
		.set("key", &"value2", Some(3))
		.await
		.expect("Set with TTL 3 should succeed");

	// Wait 2 seconds
	sleep(Duration::from_secs(2)).await;

	// settings1 should be expired
	let result1: Option<String> = settings1.get("key").await.expect("Get should succeed");
	assert_eq!(result1, None, "settings1 value should be expired");

	// settings2 should still exist
	let result2: Option<String> = settings2.get("key").await.expect("Get should succeed");
	assert_eq!(
		result2,
		Some("value2".to_string()),
		"settings2 value should still exist"
	);
}

/// Test: Observers are independent per instance
///
/// Why: Validates that observers in one instance don't receive notifications
/// from other instances.
#[rstest]
#[tokio::test]
async fn test_observers_independent_per_backend() {
	use std::sync::atomic::{AtomicUsize, Ordering};

	let backend1 = Arc::new(MemoryBackend::new());
	let backend2 = Arc::new(MemoryBackend::new());

	let settings1 = DynamicSettings::new(backend1);
	let settings2 = DynamicSettings::new(backend2);

	let observer1_count = Arc::new(AtomicUsize::new(0));
	let observer2_count = Arc::new(AtomicUsize::new(0));

	let obs1_clone = observer1_count.clone();
	let obs2_clone = observer2_count.clone();

	// Subscribe observers
	let _sub1 = settings1.subscribe(move |_key, _value| {
		obs1_clone.fetch_add(1, Ordering::SeqCst);
	});

	let _sub2 = settings2.subscribe(move |_key, _value| {
		obs2_clone.fetch_add(1, Ordering::SeqCst);
	});

	// Update only settings1
	settings1
		.set("key", &"value1", None)
		.await
		.expect("Set should succeed");

	sleep(Duration::from_millis(50)).await;

	// Verify only observer1 was notified
	assert_eq!(
		observer1_count.load(Ordering::SeqCst),
		1,
		"observer1 should be notified"
	);
	assert_eq!(
		observer2_count.load(Ordering::SeqCst),
		0,
		"observer2 should not be notified"
	);

	// Update only settings2
	settings2
		.set("key", &"value2", None)
		.await
		.expect("Set should succeed");

	sleep(Duration::from_millis(50)).await;

	// Verify only observer2 was notified
	assert_eq!(
		observer1_count.load(Ordering::SeqCst),
		1,
		"observer1 should still have 1 notification"
	);
	assert_eq!(
		observer2_count.load(Ordering::SeqCst),
		1,
		"observer2 should be notified"
	);
}

/// Test: Multiple instances can use the same backend (shared state)
///
/// Why: Validates that multiple DynamicSettings instances can share the same
/// backend, enabling shared configuration across instances.
#[rstest]
#[tokio::test]
async fn test_multiple_instances_shared_backend() {
	let shared_backend = Arc::new(MemoryBackend::new());

	let settings1 = DynamicSettings::new(shared_backend.clone());
	let settings2 = DynamicSettings::new(shared_backend.clone());

	// Set value in settings1
	settings1
		.set("shared_key", &"shared_value", None)
		.await
		.expect("Set should succeed");

	// Retrieve from settings2 (should see same value)
	let result: String = settings2
		.get("shared_key")
		.await
		.expect("Get should succeed")
		.expect("Value should exist");

	assert_eq!(
		result, "shared_value",
		"settings2 should see value set by settings1"
	);
}

/// Test: Concurrent operations on different backends
///
/// Why: Validates that concurrent operations on different backends
/// don't interfere with each other.
#[rstest]
#[tokio::test]
async fn test_concurrent_operations_different_backends() {
	let backend1 = Arc::new(MemoryBackend::new());
	let backend2 = Arc::new(MemoryBackend::new());

	let settings1 = Arc::new(DynamicSettings::new(backend1));
	let settings2 = Arc::new(DynamicSettings::new(backend2));

	let s1_clone = settings1.clone();
	let s2_clone = settings2.clone();

	// Concurrent writes
	let handle1 = tokio::spawn(async move {
		for i in 0..10 {
			s1_clone
				.set(&format!("key{}", i), &format!("value1_{}", i), None)
				.await
				.unwrap();
		}
	});

	let handle2 = tokio::spawn(async move {
		for i in 0..10 {
			s2_clone
				.set(&format!("key{}", i), &format!("value2_{}", i), None)
				.await
				.unwrap();
		}
	});

	handle1.await.expect("Task 1 should complete");
	handle2.await.expect("Task 2 should complete");

	// Verify data integrity
	for i in 0..10 {
		let key = format!("key{}", i);

		let val1: String = settings1
			.get(&key)
			.await
			.unwrap()
			.expect("Value should exist");
		assert_eq!(val1, format!("value1_{}", i));

		let val2: String = settings2
			.get(&key)
			.await
			.unwrap()
			.expect("Value should exist");
		assert_eq!(val2, format!("value2_{}", i));
	}
}

/// Test: Delete operations are independent
///
/// Why: Validates that deleting from one instance doesn't affect another.
#[rstest]
#[tokio::test]
async fn test_delete_operations_independent() {
	let backend1 = Arc::new(MemoryBackend::new());
	let backend2 = Arc::new(MemoryBackend::new());

	let settings1 = DynamicSettings::new(backend1);
	let settings2 = DynamicSettings::new(backend2);

	// Set same key in both instances
	settings1
		.set("key", &"value1", None)
		.await
		.expect("Set should succeed");
	settings2
		.set("key", &"value2", None)
		.await
		.expect("Set should succeed");

	// Delete from settings1
	settings1
		.delete("key")
		.await
		.expect("Delete should succeed");

	// Verify settings1 no longer has the key
	let result1: Option<String> = settings1.get("key").await.expect("Get should succeed");
	assert_eq!(result1, None, "settings1 should not have the key");

	// Verify settings2 still has the key
	let result2: Option<String> = settings2.get("key").await.expect("Get should succeed");
	assert_eq!(
		result2,
		Some("value2".to_string()),
		"settings2 should still have the key"
	);
}

/// Test: Different instances can have different key sets
///
/// Why: Validates that key sets are independent per instance.
#[rstest]
#[tokio::test]
async fn test_different_key_sets_per_instance() {
	let backend1 = Arc::new(MemoryBackend::new());
	let backend2 = Arc::new(MemoryBackend::new());

	let settings1 = DynamicSettings::new(backend1);
	let settings2 = DynamicSettings::new(backend2);

	// Set different keys in each instance
	settings1
		.set("key1", &"value1", None)
		.await
		.expect("Set should succeed");
	settings1
		.set("key2", &"value2", None)
		.await
		.expect("Set should succeed");

	settings2
		.set("key3", &"value3", None)
		.await
		.expect("Set should succeed");
	settings2
		.set("key4", &"value4", None)
		.await
		.expect("Set should succeed");

	// Verify settings1 has key1, key2 but not key3, key4
	assert!(settings1.get::<String>("key1").await.unwrap().is_some());
	assert!(settings1.get::<String>("key2").await.unwrap().is_some());
	assert!(settings1.get::<String>("key3").await.unwrap().is_none());
	assert!(settings1.get::<String>("key4").await.unwrap().is_none());

	// Verify settings2 has key3, key4 but not key1, key2
	assert!(settings2.get::<String>("key1").await.unwrap().is_none());
	assert!(settings2.get::<String>("key2").await.unwrap().is_none());
	assert!(settings2.get::<String>("key3").await.unwrap().is_some());
	assert!(settings2.get::<String>("key4").await.unwrap().is_some());
}

/// Test: Complex data types are independent per instance
///
/// Why: Validates that complex JSON structures are independent per instance.
#[rstest]
#[tokio::test]
async fn test_complex_types_independent() {
	use serde_json::json;

	let backend1 = Arc::new(MemoryBackend::new());
	let backend2 = Arc::new(MemoryBackend::new());

	let settings1 = DynamicSettings::new(backend1);
	let settings2 = DynamicSettings::new(backend2);

	let config1 = json!({
		"database": {
			"host": "localhost",
			"port": 5432
		}
	});

	let config2 = json!({
		"database": {
			"host": "production.example.com",
			"port": 3306
		}
	});

	settings1
		.set("config", &config1, None)
		.await
		.expect("Set should succeed");
	settings2
		.set("config", &config2, None)
		.await
		.expect("Set should succeed");

	// Verify independence
	let result1: serde_json::Value = settings1
		.get("config")
		.await
		.unwrap()
		.expect("Value should exist");
	let result2: serde_json::Value = settings2
		.get("config")
		.await
		.unwrap()
		.expect("Value should exist");

	assert_eq!(result1, config1, "settings1 should have config1");
	assert_eq!(result2, config2, "settings2 should have config2");
	assert_ne!(result1, result2, "Configs should be different");
}

/// Test: Instance lifecycle independence
///
/// Why: Validates that dropping one instance doesn't affect another.
#[rstest]
#[tokio::test]
async fn test_instance_lifecycle_independence() {
	let backend1 = Arc::new(MemoryBackend::new());
	let backend2 = Arc::new(MemoryBackend::new());

	{
		let settings1 = DynamicSettings::new(backend1.clone());
		settings1
			.set("key", &"value1", None)
			.await
			.expect("Set should succeed");
		// settings1 dropped here
	}

	let settings2 = DynamicSettings::new(backend2);
	settings2
		.set("key", &"value2", None)
		.await
		.expect("Set should succeed");

	// Verify settings2 still works after settings1 is dropped
	let result: String = settings2
		.get("key")
		.await
		.expect("Get should succeed")
		.expect("Value should exist");

	assert_eq!(result, "value2", "settings2 should still work");

	// Verify backend1 still retains data after settings1 is dropped
	let settings1_new = DynamicSettings::new(backend1);
	let result1: String = settings1_new
		.get("key")
		.await
		.expect("Get should succeed")
		.expect("Value should exist");

	assert_eq!(
		result1, "value1",
		"Backend should retain data after instance drop"
	);
}

/// Test: Multiple instances with type safety
///
/// Why: Validates that type safety is maintained independently per instance.
#[rstest]
#[tokio::test]
async fn test_type_safety_per_instance() {
	let backend1 = Arc::new(MemoryBackend::new());
	let backend2 = Arc::new(MemoryBackend::new());

	let settings1 = DynamicSettings::new(backend1);
	let settings2 = DynamicSettings::new(backend2);

	// Same key, different types
	settings1
		.set("key", &"string_value", None)
		.await
		.expect("Set string should succeed");
	settings2
		.set("key", &42, None)
		.await
		.expect("Set integer should succeed");

	// Verify correct types per instance
	let str_result: String = settings1
		.get("key")
		.await
		.unwrap()
		.expect("Value should exist");
	assert_eq!(str_result, "string_value");

	let int_result: i32 = settings2
		.get("key")
		.await
		.unwrap()
		.expect("Value should exist");
	assert_eq!(int_result, 42);

	// Verify type mismatch errors are independent
	let wrong_type1 = settings1.get::<i32>("key").await;
	assert!(
		wrong_type1.is_err(),
		"Getting string as int should fail in settings1"
	);

	let wrong_type2 = settings2.get::<String>("key").await;
	assert!(
		wrong_type2.is_err(),
		"Getting int as string should fail in settings2"
	);
}
