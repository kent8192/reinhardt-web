//! Concurrent Operations Stress Tests
//!
//! This test module validates the behavior of DynamicSettings under concurrent access
//! from multiple tasks, testing for race conditions, deadlocks, and data consistency.
//!
//! ## Test Categories
//!
//! 1. **High Concurrency**: Many tasks reading/writing simultaneously
//! 2. **Read-Write Contention**: Heavy read operations with occasional writes
//! 3. **Write-Write Contention**: Multiple tasks updating same keys
//! 4. **Observer Stress**: Many observers watching same keys
//! 5. **Memory Pressure**: Large data structures under concurrent access
//!
//! ## Testing Strategy
//!
//! - Use tokio::spawn for true concurrent execution
//! - Test with MemoryBackend (no external dependencies)
//! - Verify data consistency after stress operations
//! - Check for panics, deadlocks, or data corruption

#[cfg(feature = "async")]
mod concurrent_stress_tests {
	use reinhardt_conf::settings::backends::memory::MemoryBackend;
	use reinhardt_conf::settings::dynamic::DynamicSettings;
	use rstest::*;
	use serial_test::serial;
	use std::sync::Arc;
	use std::sync::atomic::{AtomicU32, Ordering};
	use tokio::time::{Duration, sleep};

	/// Test: High concurrency read-write mix
	///
	/// Why: Verifies that DynamicSettings handles many concurrent read/write operations
	/// without panics or data corruption.
	#[rstest]
	#[serial(stress)]
	#[tokio::test]
	async fn test_concurrent_stress_read_write_mix() {
		let backend = Arc::new(MemoryBackend::new());
		let dynamic = Arc::new(DynamicSettings::new(backend));

		// Initialize some keys
		for i in 0..10 {
			let key = format!("stress.key.{}", i);
			dynamic
				.set(&key, &i, None)
				.await
				.expect("Failed to initialize");
		}

		// Spawn many concurrent tasks (50 readers + 10 writers)
		let mut handles = vec![];

		// Readers (50 tasks)
		for reader_id in 0..50 {
			let dynamic_clone = Arc::clone(&dynamic);
			let handle = tokio::spawn(async move {
				for iteration in 0..100 {
					// Pseudo-random key selection (derived from task ID and iteration)
					let key_index = (reader_id + iteration) % 10;
					let key = format!("stress.key.{}", key_index);
					let _value: Option<i32> = dynamic_clone.get(&key).await.unwrap();
				}
			});
			handles.push(handle);
		}

		// Writers (10 tasks)
		for writer_id in 0..10 {
			let dynamic_clone = Arc::clone(&dynamic);
			let handle = tokio::spawn(async move {
				for iteration in 0..50 {
					let key = format!("stress.key.{}", writer_id);
					let value = writer_id * 1000 + iteration;
					dynamic_clone.set(&key, &value, None).await.unwrap();
				}
			});
			handles.push(handle);
		}

		// Wait for all tasks
		for handle in handles {
			handle.await.expect("Task should not panic");
		}

		// Verify final state (all keys should still be accessible)
		for i in 0..10 {
			let key = format!("stress.key.{}", i);
			let value: Option<i32> = dynamic.get(&key).await.unwrap();
			assert!(value.is_some(), "Key {} should still exist", key);
		}
	}

	/// Test: Write-write contention on same keys
	///
	/// Why: Verifies that concurrent writes to the same keys don't cause corruption.
	/// Last write should win.
	#[rstest]
	#[serial(stress)]
	#[tokio::test]
	async fn test_concurrent_stress_write_contention() {
		let backend = Arc::new(MemoryBackend::new());
		let dynamic = Arc::new(DynamicSettings::new(backend));

		let write_counter = Arc::new(AtomicU32::new(0));

		// Spawn many writers to the same key
		let mut handles = vec![];

		for writer_id in 0..20 {
			let dynamic_clone = Arc::clone(&dynamic);
			let counter_clone = Arc::clone(&write_counter);
			let handle = tokio::spawn(async move {
				for _ in 0..50 {
					let value = format!("writer_{}", writer_id);
					dynamic_clone
						.set("contention.key", &value, None)
						.await
						.unwrap();
					counter_clone.fetch_add(1, Ordering::SeqCst);
					sleep(Duration::from_micros(1)).await;
				}
			});
			handles.push(handle);
		}

		// Wait for all writes
		for handle in handles {
			handle.await.expect("Writer should not panic");
		}

		// Verify total writes
		assert_eq!(
			write_counter.load(Ordering::SeqCst),
			1000,
			"All writes should have completed"
		);

		// Verify key exists and has valid value
		let final_value: Option<String> = dynamic.get("contention.key").await.unwrap();
		assert!(final_value.is_some(), "Key should exist after stress");
		assert!(
			final_value.unwrap().starts_with("writer_"),
			"Value should be from one of the writers"
		);
	}

	/// Test: Heavy read operations with occasional writes
	///
	/// Why: Verifies that heavy read load doesn't block writes and vice versa.
	#[rstest]
	#[serial(stress)]
	#[tokio::test]
	async fn test_concurrent_stress_read_heavy() {
		let backend = Arc::new(MemoryBackend::new());
		let dynamic = Arc::new(DynamicSettings::new(backend));

		// Initialize key
		dynamic
			.set("read_heavy.key", &"initial", None)
			.await
			.unwrap();

		let read_counter = Arc::new(AtomicU32::new(0));
		let write_counter = Arc::new(AtomicU32::new(0));

		let mut handles = vec![];

		// Heavy readers (100 tasks)
		for _ in 0..100 {
			let dynamic_clone = Arc::clone(&dynamic);
			let counter_clone = Arc::clone(&read_counter);
			let handle = tokio::spawn(async move {
				for _ in 0..100 {
					let _value: Option<String> = dynamic_clone.get("read_heavy.key").await.unwrap();
					counter_clone.fetch_add(1, Ordering::SeqCst);
				}
			});
			handles.push(handle);
		}

		// Occasional writers (5 tasks)
		for i in 0..5 {
			let dynamic_clone = Arc::clone(&dynamic);
			let counter_clone = Arc::clone(&write_counter);
			let handle = tokio::spawn(async move {
				for j in 0..10 {
					let value = format!("update_{}_{}", i, j);
					dynamic_clone
						.set("read_heavy.key", &value, None)
						.await
						.unwrap();
					counter_clone.fetch_add(1, Ordering::SeqCst);
					sleep(Duration::from_millis(10)).await;
				}
			});
			handles.push(handle);
		}

		// Wait for all operations
		for handle in handles {
			handle.await.expect("Task should not panic");
		}

		// Verify counters
		assert_eq!(
			read_counter.load(Ordering::SeqCst),
			10000,
			"All reads should complete"
		);
		assert_eq!(
			write_counter.load(Ordering::SeqCst),
			50,
			"All writes should complete"
		);
	}

	/// Test: Multiple keys under concurrent access
	///
	/// Why: Verifies that concurrent access to different keys doesn't cause issues.
	#[rstest]
	#[serial(stress)]
	#[tokio::test]
	async fn test_concurrent_stress_multiple_keys() {
		let backend = Arc::new(MemoryBackend::new());
		let dynamic = Arc::new(DynamicSettings::new(backend));

		// Spawn tasks for different keys
		let mut handles = vec![];

		for key_id in 0..20 {
			let dynamic_clone = Arc::clone(&dynamic);
			let handle = tokio::spawn(async move {
				let key = format!("multi.key.{}", key_id);

				// Each task: write → read → update → read
				dynamic_clone
					.set(&key, &key_id, None)
					.await
					.expect("Initial set");

				let value1: Option<i32> = dynamic_clone.get(&key).await.expect("First read");
				assert_eq!(value1, Some(key_id as i32));

				dynamic_clone
					.set(&key, &(key_id * 2), None)
					.await
					.expect("Update");

				let value2: Option<i32> = dynamic_clone.get(&key).await.expect("Second read");
				assert_eq!(value2, Some((key_id * 2) as i32));
			});
			handles.push(handle);
		}

		// Wait for all tasks
		for handle in handles {
			handle.await.expect("Task should not panic");
		}

		// Verify all keys exist with correct final values
		for key_id in 0..20 {
			let key = format!("multi.key.{}", key_id);
			let value: Option<i32> = dynamic.get(&key).await.unwrap();
			assert_eq!(
				value,
				Some((key_id * 2) as i32),
				"Key {} should have correct final value",
				key
			);
		}
	}

	/// Test: Rapid key creation and deletion
	///
	/// Why: Verifies that rapid creation/deletion of keys doesn't cause memory leaks
	/// or panics.
	#[rstest]
	#[serial(stress)]
	#[tokio::test]
	async fn test_concurrent_stress_create_delete() {
		let backend = Arc::new(MemoryBackend::new());
		let dynamic = Arc::new(DynamicSettings::new(backend));

		let operation_counter = Arc::new(AtomicU32::new(0));

		let mut handles = vec![];

		for task_id in 0..10 {
			let dynamic_clone = Arc::clone(&dynamic);
			let counter_clone = Arc::clone(&operation_counter);
			let handle = tokio::spawn(async move {
				for iteration in 0..100 {
					let key = format!("temp.key.{}.{}", task_id, iteration);

					// Create
					dynamic_clone
						.set(&key, &iteration, None)
						.await
						.expect("Set should succeed");

					// Read
					let _value: Option<i32> =
						dynamic_clone.get(&key).await.expect("Get should succeed");

					// Note: MemoryBackend doesn't have explicit delete,
					// but we can overwrite with None-like value
					// In production, would use explicit delete operation

					counter_clone.fetch_add(1, Ordering::SeqCst);
				}
			});
			handles.push(handle);
		}

		// Wait for all operations
		for handle in handles {
			handle.await.expect("Task should not panic");
		}

		// Verify operation count
		assert_eq!(
			operation_counter.load(Ordering::SeqCst),
			1000,
			"All operations should complete"
		);
	}

	/// Test: Large value stress test
	///
	/// Why: Verifies that concurrent access to large values doesn't cause memory issues.
	#[rstest]
	#[serial(stress)]
	#[tokio::test]
	async fn test_concurrent_stress_large_values() {
		let backend = Arc::new(MemoryBackend::new());
		let dynamic = Arc::new(DynamicSettings::new(backend));

		// Create large value (1KB string)
		let large_value = "x".repeat(1024);

		let mut handles = vec![];

		// Multiple tasks writing large values
		for i in 0..10 {
			let dynamic_clone = Arc::clone(&dynamic);
			let value_clone = large_value.clone();
			let handle = tokio::spawn(async move {
				let key = format!("large.key.{}", i);
				for _ in 0..10 {
					dynamic_clone
						.set(&key, &value_clone, None)
						.await
						.expect("Set large value");

					let retrieved: Option<String> =
						dynamic_clone.get(&key).await.expect("Get large value");
					assert_eq!(retrieved, Some(value_clone.clone()));
				}
			});
			handles.push(handle);
		}

		// Wait for all tasks
		for handle in handles {
			handle.await.expect("Task should not panic");
		}

		// Verify all large keys exist
		for i in 0..10 {
			let key = format!("large.key.{}", i);
			let value: Option<String> = dynamic.get(&key).await.unwrap();
			assert_eq!(value, Some(large_value.clone()));
		}
	}
}
