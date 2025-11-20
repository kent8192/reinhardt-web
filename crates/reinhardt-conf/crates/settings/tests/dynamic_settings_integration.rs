//! Integration tests for dynamic settings with Memory backend
//!
//! This test module validates the integration of DynamicSettings with MemoryBackend,
//! including basic get/set operations, observer patterns, and concurrent access.

use reinhardt_settings::backends::memory::MemoryBackend;
use reinhardt_settings::dynamic::DynamicSettings;
use rstest::*;
use serde_json::json;
use serial_test::serial;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::time::{sleep, Duration};

/// Test: Memory backend basic get/set operations
#[rstest]
#[serial(dynamic_memory)]
#[tokio::test]
async fn test_memory_backend_get_set() {
	let backend = MemoryBackend::new();
	let dynamic = DynamicSettings::new(Arc::new(backend));

	// Set value
	let value = "test_app";
	dynamic
		.set("app.name", &value, None)
		.await
		.expect("Failed to set value");

	// Get value
	let retrieved: Option<String> = dynamic.get("app.name").await.expect("Failed to get value");
	assert_eq!(retrieved, Some("test_app".to_string()));
}

/// Test: Memory backend with different value types
#[rstest]
#[serial(dynamic_memory)]
#[tokio::test]
async fn test_memory_backend_value_types() {
	let backend = MemoryBackend::new();
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
}

/// Test: Memory backend value updates
#[rstest]
#[serial(dynamic_memory)]
#[tokio::test]
async fn test_memory_backend_value_updates() {
	let backend = MemoryBackend::new();
	let dynamic = DynamicSettings::new(Arc::new(backend));

	// Set initial value
	dynamic.set("counter", &1, None).await.unwrap();
	let v: Option<i64> = dynamic.get("counter").await.unwrap();
	assert_eq!(v, Some(1));

	// Update value
	dynamic.set("counter", &2, None).await.unwrap();
	let v: Option<i64> = dynamic.get("counter").await.unwrap();
	assert_eq!(v, Some(2));

	// Update again
	dynamic.set("counter", &100, None).await.unwrap();
	let v: Option<i64> = dynamic.get("counter").await.unwrap();
	assert_eq!(v, Some(100));
}

/// Test: Get non-existent key returns None
#[rstest]
#[serial(dynamic_memory)]
#[tokio::test]
async fn test_memory_get_nonexistent_key() {
	let backend = MemoryBackend::new();
	let dynamic = DynamicSettings::new(Arc::new(backend));

	// Try to get non-existent key
	let result: Option<String> = dynamic.get("non.existent.key").await.unwrap();

	// Should return None
	assert_eq!(result, None);
}

/// Test: Observer pattern with value changes
#[rstest]
#[serial(dynamic_memory)]
#[tokio::test]
async fn test_memory_observer_pattern() {
	let backend = MemoryBackend::new();
	let dynamic = DynamicSettings::new(Arc::new(backend));

	// Shared counter to track observer calls
	let call_count = Arc::new(AtomicU32::new(0));
	let call_count_clone = call_count.clone();

	// Subscribe to all changes
	let _subscription_id = dynamic.subscribe(move |_key, _value| {
		call_count_clone.fetch_add(1, Ordering::SeqCst);
	});

	// Set value (should trigger observer)
	dynamic.set("settings.version", &1, None).await.unwrap();

	// Small delay to allow observer callback
	sleep(Duration::from_millis(50)).await;

	// Verify observer was called
	assert_eq!(call_count.load(Ordering::SeqCst), 1);

	// Update value again
	dynamic.set("settings.version", &2, None).await.unwrap();
	sleep(Duration::from_millis(50)).await;

	// Observer should have been called twice
	assert_eq!(call_count.load(Ordering::SeqCst), 2);
}

/// Test: Multiple observers
#[rstest]
#[serial(dynamic_memory)]
#[tokio::test]
async fn test_memory_multiple_observers() {
	let backend = MemoryBackend::new();
	let dynamic = DynamicSettings::new(Arc::new(backend));

	let count1 = Arc::new(AtomicU32::new(0));
	let count2 = Arc::new(AtomicU32::new(0));

	let count1_clone = count1.clone();
	let count2_clone = count2.clone();

	// Subscribe with two observers
	let _sub1 = dynamic.subscribe(move |_key, _val| {
		count1_clone.fetch_add(1, Ordering::SeqCst);
	});

	let _sub2 = dynamic.subscribe(move |_key, _val| {
		count2_clone.fetch_add(1, Ordering::SeqCst);
	});

	// Set value
	dynamic.set("feature.enabled", &true, None).await.unwrap();
	sleep(Duration::from_millis(50)).await;

	// Both observers should be called
	assert_eq!(count1.load(Ordering::SeqCst), 1);
	assert_eq!(count2.load(Ordering::SeqCst), 1);
}

/// Test: Unsubscribe from observer
#[rstest]
#[serial(dynamic_memory)]
#[tokio::test]
async fn test_memory_unsubscribe() {
	let backend = MemoryBackend::new();
	let dynamic = DynamicSettings::new(Arc::new(backend));

	let call_count = Arc::new(AtomicU32::new(0));
	let call_count_clone = call_count.clone();

	// Subscribe
	let subscription_id = dynamic.subscribe(move |_key, _val| {
		call_count_clone.fetch_add(1, Ordering::SeqCst);
	});

	// Set value (should trigger observer)
	dynamic.set("config.value", &"test", None).await.unwrap();
	sleep(Duration::from_millis(50)).await;
	assert_eq!(call_count.load(Ordering::SeqCst), 1);

	// Unsubscribe
	dynamic.unsubscribe(subscription_id);

	// Set value again (should NOT trigger observer)
	dynamic.set("config.value", &"updated", None).await.unwrap();
	sleep(Duration::from_millis(50)).await;

	// Call count should still be 1
	assert_eq!(call_count.load(Ordering::SeqCst), 1);
}

/// Test: Complex JSON values
#[rstest]
#[serial(dynamic_memory)]
#[tokio::test]
async fn test_memory_complex_json_values() {
	let backend = MemoryBackend::new();
	let dynamic = DynamicSettings::new(Arc::new(backend));

	// Set complex nested object
	let complex_value = json!({
		"database": {
			"connections": [
				{"host": "db1.example.com", "port": 5432},
				{"host": "db2.example.com", "port": 5432}
			],
			"pool_size": 10,
			"timeout": 30
		},
		"features": {
			"auth": true,
			"cache": false,
			"logging": {
				"level": "info",
				"format": "json"
			}
		}
	});

	dynamic
		.set("app.config", &complex_value, None)
		.await
		.unwrap();

	// Get and verify
	let retrieved: Option<serde_json::Value> = dynamic.get("app.config").await.unwrap();
	assert_eq!(retrieved, Some(complex_value));
}

/// Test: Concurrent get/set operations
#[rstest]
#[serial(dynamic_memory)]
#[tokio::test]
async fn test_memory_concurrent_operations() {
	let backend = Arc::new(MemoryBackend::new());
	let dynamic = Arc::new(DynamicSettings::new(backend));

	// Spawn multiple concurrent tasks
	let mut handles = vec![];

	for i in 0..10 {
		let dynamic_clone = dynamic.clone();
		let handle = tokio::spawn(async move {
			let key = format!("concurrent.key{}", i);
			dynamic_clone.set(&key, &i, None).await.unwrap();

			let value: Option<i32> = dynamic_clone.get(&key).await.unwrap();
			assert_eq!(value, Some(i));
		});
		handles.push(handle);
	}

	// Wait for all tasks
	for handle in handles {
		handle.await.expect("Task panicked");
	}
}

/// Test: Delete operation
#[rstest]
#[serial(dynamic_memory)]
#[tokio::test]
async fn test_memory_delete() {
	let backend = MemoryBackend::new();
	let dynamic = DynamicSettings::new(Arc::new(backend));

	// Set value
	dynamic.set("temp.value", &"temporary", None).await.unwrap();
	let v: Option<String> = dynamic.get("temp.value").await.unwrap();
	assert_eq!(v, Some("temporary".to_string()));

	// Delete value
	dynamic.delete("temp.value").await.unwrap();

	// Get should return None
	let v: Option<String> = dynamic.get("temp.value").await.unwrap();
	assert_eq!(v, None);
}

/// Test: Set with TTL
#[rstest]
#[serial(dynamic_memory)]
#[tokio::test]
async fn test_memory_set_with_ttl() {
	let backend = MemoryBackend::new();
	let dynamic = DynamicSettings::new(Arc::new(backend));

	// Set value with 1 second TTL
	dynamic.set("ttl.key", &"expires", Some(1)).await.unwrap();

	// Immediately get value (should exist)
	let v: Option<String> = dynamic.get("ttl.key").await.unwrap();
	assert_eq!(v, Some("expires".to_string()));

	// Wait for TTL to expire
	sleep(Duration::from_secs(2)).await;

	// Get should return None (expired)
	let v: Option<String> = dynamic.get("ttl.key").await.unwrap();
	assert_eq!(v, None);
}
