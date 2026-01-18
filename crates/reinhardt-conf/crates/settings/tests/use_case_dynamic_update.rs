//! Integration tests for Dynamic Configuration Update Without Restart Use Case.
//!
//! This test module validates that running applications can update feature flags
//! and configuration values dynamically without restart, using the observer pattern
//! to notify subscribers of changes.
//!
//! NOTE: These tests are feature-gated with "async" feature.

#![cfg(feature = "async")]

use reinhardt_conf::settings::backends::MemoryBackend;
use reinhardt_conf::settings::dynamic::DynamicSettings;
use rstest::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::time::{Duration, sleep};

/// Test: Dynamic feature flag update without restart
///
/// Why: Validates that applications can update feature flags at runtime
/// and subscribers are notified of changes, enabling dynamic behavior updates
/// without restarting the application.
#[rstest]
#[tokio::test]
async fn test_dynamic_feature_flag_update() {
	// Step 1: Start application with DynamicSettings + MemoryBackend
	let backend = Arc::new(MemoryBackend::new());
	let settings = DynamicSettings::new(backend);

	// Step 2: Load initial feature flags
	settings
		.set("feature.auth", &true, None)
		.await
		.expect("Setting feature.auth should succeed");
	settings
		.set("feature.cache", &false, None)
		.await
		.expect("Setting feature.cache should succeed");

	// Verify initial state
	let auth_enabled: bool = settings
		.get("feature.auth")
		.await
		.expect("Get should succeed")
		.expect("feature.auth should exist");
	let cache_enabled: bool = settings
		.get("feature.cache")
		.await
		.expect("Get should succeed")
		.expect("feature.cache should exist");

	assert_eq!(
		auth_enabled, true,
		"Auth feature should be enabled initially"
	);
	assert_eq!(
		cache_enabled, false,
		"Cache feature should be disabled initially"
	);

	// Step 3: Application subscribes to changes
	let observer_called = Arc::new(AtomicBool::new(false));
	let observed_key = Arc::new(parking_lot::Mutex::new(String::new()));
	let observed_value = Arc::new(parking_lot::Mutex::new(serde_json::Value::Null));

	let observer_called_clone = observer_called.clone();
	let observed_key_clone = observed_key.clone();
	let observed_value_clone = observed_value.clone();

	let subscription_id = settings.subscribe(move |key, value| {
		observer_called_clone.store(true, Ordering::SeqCst);
		*observed_key_clone.lock() = key.to_string();
		if let Some(v) = value {
			*observed_value_clone.lock() = v.clone();
		}
	});

	// Step 4: Admin updates feature.cache=true via API (simulated)
	settings
		.set("feature.cache", &true, None)
		.await
		.expect("Updating feature.cache should succeed");

	// Allow time for observer to be called (in-memory, should be immediate)
	sleep(Duration::from_millis(10)).await;

	// Step 5: Verify DynamicSettings notified observers
	assert!(
		observer_called.load(Ordering::SeqCst),
		"Observer should be called when setting is updated"
	);

	let notified_key = observed_key.lock().clone();
	let notified_value = observed_value.lock().clone();

	assert_eq!(
		notified_key, "feature.cache",
		"Observer should be notified of feature.cache change"
	);
	assert_eq!(
		notified_value.as_bool().unwrap(),
		true,
		"Observer should receive new value: true"
	);

	// Step 6: Verify application behavior changes (cache now enabled)
	let cache_enabled_after: bool = settings
		.get("feature.cache")
		.await
		.expect("Get should succeed")
		.expect("feature.cache should exist");

	assert_eq!(
		cache_enabled_after, true,
		"Cache feature should be enabled after update"
	);

	// Unsubscribe
	settings.unsubscribe(subscription_id);
}

/// Test: Multiple observers receive notifications
///
/// Why: Validates that multiple components can subscribe to configuration changes
/// independently and all receive notifications.
#[rstest]
#[tokio::test]
async fn test_multiple_observers() {
	let backend = Arc::new(MemoryBackend::new());
	let settings = DynamicSettings::new(backend);

	settings
		.set("app.timeout", &30, None)
		.await
		.expect("Setting timeout should succeed");

	// Subscribe multiple observers
	let observer1_called = Arc::new(AtomicUsize::new(0));
	let observer2_called = Arc::new(AtomicUsize::new(0));
	let observer3_called = Arc::new(AtomicUsize::new(0));

	let obs1_clone = observer1_called.clone();
	let obs2_clone = observer2_called.clone();
	let obs3_clone = observer3_called.clone();

	let sub1 = settings.subscribe(move |_key, _value| {
		obs1_clone.fetch_add(1, Ordering::SeqCst);
	});

	let sub2 = settings.subscribe(move |_key, _value| {
		obs2_clone.fetch_add(1, Ordering::SeqCst);
	});

	let sub3 = settings.subscribe(move |_key, _value| {
		obs3_clone.fetch_add(1, Ordering::SeqCst);
	});

	// Update setting
	settings
		.set("app.timeout", &60, None)
		.await
		.expect("Updating timeout should succeed");

	sleep(Duration::from_millis(10)).await;

	// Verify all observers were called
	assert_eq!(
		observer1_called.load(Ordering::SeqCst),
		1,
		"Observer 1 should be called once"
	);
	assert_eq!(
		observer2_called.load(Ordering::SeqCst),
		1,
		"Observer 2 should be called once"
	);
	assert_eq!(
		observer3_called.load(Ordering::SeqCst),
		1,
		"Observer 3 should be called once"
	);

	// Cleanup
	settings.unsubscribe(sub1);
	settings.unsubscribe(sub2);
	settings.unsubscribe(sub3);
}

/// Test: Observer unsubscribe stops notifications
///
/// Why: Validates that after unsubscribing, components no longer receive
/// change notifications, preventing resource leaks and unwanted updates.
#[rstest]
#[tokio::test]
async fn test_observer_unsubscribe() {
	let backend = Arc::new(MemoryBackend::new());
	let settings = DynamicSettings::new(backend);

	settings
		.set("counter", &0, None)
		.await
		.expect("Setting counter should succeed");

	let call_count = Arc::new(AtomicUsize::new(0));
	let call_count_clone = call_count.clone();

	// Subscribe
	let subscription_id = settings.subscribe(move |_key, _value| {
		call_count_clone.fetch_add(1, Ordering::SeqCst);
	});

	// First update (observer should be called)
	settings
		.set("counter", &1, None)
		.await
		.expect("Update should succeed");
	sleep(Duration::from_millis(10)).await;

	assert_eq!(
		call_count.load(Ordering::SeqCst),
		1,
		"Observer should be called for first update"
	);

	// Unsubscribe
	settings.unsubscribe(subscription_id);

	// Second update (observer should NOT be called)
	settings
		.set("counter", &2, None)
		.await
		.expect("Update should succeed");
	sleep(Duration::from_millis(10)).await;

	assert_eq!(
		call_count.load(Ordering::SeqCst),
		1,
		"Observer should not be called after unsubscribe"
	);
}

/// Test: Dynamic settings with TTL expiration
///
/// Why: Validates that settings with TTL automatically expire and are removed,
/// useful for temporary feature flags or time-limited configurations.
#[rstest]
#[tokio::test]
async fn test_dynamic_settings_ttl_expiration() {
	let backend = Arc::new(MemoryBackend::new());
	let settings = DynamicSettings::new(backend);

	// Set value with 1 second TTL
	settings
		.set("temporary_flag", &"active", Some(1))
		.await
		.expect("Setting with TTL should succeed");

	// Immediately after set, value should exist
	let value: Option<String> = settings
		.get("temporary_flag")
		.await
		.expect("Get should succeed");
	assert_eq!(
		value,
		Some("active".to_string()),
		"Value should exist immediately after set"
	);

	// Wait for TTL to expire (1 second + buffer)
	sleep(Duration::from_secs(2)).await;

	// After expiration, value should be None
	let value_after: Option<String> = settings
		.get("temporary_flag")
		.await
		.expect("Get should succeed");
	assert_eq!(value_after, None, "Value should be expired and return None");
}

/// Test: Concurrent updates with observers
///
/// Why: Validates that concurrent updates to different keys don't interfere
/// and all observers are correctly notified.
#[rstest]
#[tokio::test]
async fn test_concurrent_updates_with_observers() {
	let backend = Arc::new(MemoryBackend::new());
	let settings = Arc::new(DynamicSettings::new(backend));

	let notification_count = Arc::new(AtomicUsize::new(0));
	let notif_clone = notification_count.clone();

	settings.subscribe(move |_key, _value| {
		notif_clone.fetch_add(1, Ordering::SeqCst);
	});

	// Spawn concurrent updates
	let settings_clone1 = settings.clone();
	let settings_clone2 = settings.clone();
	let settings_clone3 = settings.clone();

	let handle1 = tokio::spawn(async move {
		settings_clone1.set("key1", &"value1", None).await.unwrap();
	});

	let handle2 = tokio::spawn(async move {
		settings_clone2.set("key2", &"value2", None).await.unwrap();
	});

	let handle3 = tokio::spawn(async move {
		settings_clone3.set("key3", &"value3", None).await.unwrap();
	});

	// Wait for all updates
	handle1.await.unwrap();
	handle2.await.unwrap();
	handle3.await.unwrap();

	sleep(Duration::from_millis(50)).await;

	// Verify all notifications were sent
	let notifications = notification_count.load(Ordering::SeqCst);
	assert_eq!(
		notifications, 3,
		"Should receive 3 notifications for 3 concurrent updates"
	);

	// Verify all values are set
	let val1: Option<String> = settings.get("key1").await.unwrap();
	let val2: Option<String> = settings.get("key2").await.unwrap();
	let val3: Option<String> = settings.get("key3").await.unwrap();

	assert_eq!(val1, Some("value1".to_string()));
	assert_eq!(val2, Some("value2".to_string()));
	assert_eq!(val3, Some("value3".to_string()));
}

/// Test: Observer receives old and new values
///
/// Why: Validates that observers can access both the key and new value,
/// allowing them to react appropriately to configuration changes.
#[rstest]
#[tokio::test]
async fn test_observer_receives_key_and_value() {
	let backend = Arc::new(MemoryBackend::new());
	let settings = DynamicSettings::new(backend);

	let received_key = Arc::new(parking_lot::Mutex::new(String::new()));
	let received_value = Arc::new(parking_lot::Mutex::new(serde_json::Value::Null));

	let key_clone = received_key.clone();
	let value_clone = received_value.clone();

	let sub_id = settings.subscribe(move |key, value| {
		*key_clone.lock() = key.to_string();
		if let Some(v) = value {
			*value_clone.lock() = v.clone();
		}
	});

	// Set a complex value
	let config_value = serde_json::json!({
		"enabled": true,
		"timeout": 300,
		"endpoints": ["api1", "api2"]
	});

	settings
		.set("service.config", &config_value, None)
		.await
		.expect("Setting complex value should succeed");

	sleep(Duration::from_millis(10)).await;

	// Verify observer received correct key and value
	let key = received_key.lock().clone();
	let value = received_value.lock().clone();

	assert_eq!(key, "service.config", "Observer should receive correct key");
	assert_eq!(
		value, config_value,
		"Observer should receive correct complex value"
	);

	settings.unsubscribe(sub_id);
}

/// Test: Feature flag cascading updates
///
/// Why: Validates a common pattern where updating one feature flag triggers
/// dependent feature flag updates through observers.
#[rstest]
#[tokio::test]
async fn test_feature_flag_cascading() {
	let backend = Arc::new(MemoryBackend::new());
	let settings = Arc::new(DynamicSettings::new(backend));

	// Initial state: both features disabled
	settings.set("feature.premium", &false, None).await.unwrap();
	settings
		.set("feature.analytics", &false, None)
		.await
		.unwrap();

	// Observer: when premium is enabled, automatically enable analytics
	let settings_clone = settings.clone();
	let sub_id = settings.subscribe(move |key, value| {
		if key == "feature.premium" && value.and_then(|v| v.as_bool()) == Some(true) {
			// Simulate cascading update (in real app, would be in async context)
			let settings_inner = settings_clone.clone();
			tokio::spawn(async move {
				settings_inner
					.set("feature.analytics", &true, None)
					.await
					.ok();
			});
		}
	});

	// Enable premium feature
	settings.set("feature.premium", &true, None).await.unwrap();

	// Allow time for cascading update
	sleep(Duration::from_millis(50)).await;

	// Verify analytics was automatically enabled
	let analytics_enabled: bool = settings.get("feature.analytics").await.unwrap().unwrap();

	assert_eq!(
		analytics_enabled, true,
		"Analytics should be automatically enabled when premium is enabled"
	);

	settings.unsubscribe(sub_id);
}
