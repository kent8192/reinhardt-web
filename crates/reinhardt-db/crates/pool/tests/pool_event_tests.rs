//! Pool event system tests
//! Based on SQLAlchemy's pool event tests

use async_trait::async_trait;
use reinhardt_pool::{ConnectionPool, PoolConfig, PoolEvent, PoolEventListener};
use rstest::*;
use sqlx::Sqlite;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Test event listener that records events
struct TestEventListener {
	events: Arc<Mutex<Vec<PoolEvent>>>,
}

impl TestEventListener {
	fn new() -> (Self, Arc<Mutex<Vec<PoolEvent>>>) {
		let events = Arc::new(Mutex::new(Vec::new()));
		let listener = Self {
			events: events.clone(),
		};
		(listener, events)
	}
}

/// Helper struct for event polling fixtures
struct EventPoller;

impl EventPoller {
	/// Poll until a specific event type is received
	async fn wait_for_event<F>(
		events: Arc<Mutex<Vec<PoolEvent>>>,
		timeout: Duration,
		predicate: F,
	) -> Result<(), String>
	where
		F: Fn(&PoolEvent) -> bool + Send,
	{
		reinhardt_test::poll_until(timeout, Duration::from_millis(10), || async {
			let events = events.lock().await;
			events.iter().any(&predicate)
		})
		.await
	}

	/// Poll until event list is non-empty
	async fn wait_for_any_event(
		events: Arc<Mutex<Vec<PoolEvent>>>,
		timeout: Duration,
	) -> Result<(), String> {
		reinhardt_test::poll_until(timeout, Duration::from_millis(10), || async {
			let events = events.lock().await;
			!events.is_empty()
		})
		.await
	}

	/// Poll until event count reaches expected value
	async fn wait_for_event_count(
		events: Arc<Mutex<Vec<PoolEvent>>>,
		expected: usize,
		timeout: Duration,
	) -> Result<(), String> {
		reinhardt_test::poll_until(timeout, Duration::from_millis(10), || async {
			let events = events.lock().await;
			events.len() >= expected
		})
		.await
	}
}

/// Fixture for event poller helper
#[fixture]
fn event_poller() -> EventPoller {
	EventPoller
}

#[async_trait]
impl PoolEventListener for TestEventListener {
	async fn on_event(&self, event: PoolEvent) {
		let mut events = self.events.lock().await;
		events.push(event);
	}
}

#[tokio::test]
async fn test_add_listener() {
	// Test adding event listeners to pool
	let url = "sqlite::memory:";
	let config = PoolConfig::default();

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	let (listener, _events) = TestEventListener::new();
	pool.add_listener(Arc::new(listener)).await;
}

#[tokio::test]
async fn test_pool_events_multiple_listeners() {
	// Test adding multiple event listeners
	let url = "sqlite::memory:";
	let config = PoolConfig::default();

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	let (listener1, _events1) = TestEventListener::new();
	let (listener2, _events2) = TestEventListener::new();

	pool.add_listener(Arc::new(listener1)).await;
	pool.add_listener(Arc::new(listener2)).await;
}

#[tokio::test]
async fn test_pool_event_types() {
	// Test different types of pool events
	let event1 = PoolEvent::connection_acquired("conn-1".to_string());
	let event2 = PoolEvent::connection_returned("conn-1".to_string());
	let event3 = PoolEvent::connection_created("conn-2".to_string());
	let event4 = PoolEvent::connection_closed("conn-2".to_string(), "timeout".to_string());
	let event5 =
		PoolEvent::connection_test_failed("conn-3".to_string(), "connection refused".to_string());

	// Verify event types
	match event1 {
		PoolEvent::ConnectionAcquired { connection_id, .. } => {
			assert_eq!(connection_id, "conn-1");
		}
		_ => panic!("Wrong event type"),
	}

	match event2 {
		PoolEvent::ConnectionReturned { connection_id, .. } => {
			assert_eq!(connection_id, "conn-1");
		}
		_ => panic!("Wrong event type"),
	}

	match event3 {
		PoolEvent::ConnectionCreated { connection_id, .. } => {
			assert_eq!(connection_id, "conn-2");
		}
		_ => panic!("Wrong event type"),
	}

	match event4 {
		PoolEvent::ConnectionClosed {
			connection_id,
			reason,
			..
		} => {
			assert_eq!(connection_id, "conn-2");
			assert_eq!(reason, "timeout");
		}
		_ => panic!("Wrong event type"),
	}

	match event5 {
		PoolEvent::ConnectionTestFailed {
			connection_id,
			error,
			..
		} => {
			assert_eq!(connection_id, "conn-3");
			assert_eq!(error, "connection refused");
		}
		_ => panic!("Wrong event type"),
	}
}

#[tokio::test]
async fn test_event_serialization() {
	// Test that events can be serialized/deserialized
	let event = PoolEvent::connection_acquired("test-conn".to_string());

	let json = serde_json::to_string(&event).expect("Failed to serialize event");
	assert!(json.contains("ConnectionAcquired"));
	assert!(json.contains("test-conn"));

	let deserialized: PoolEvent = serde_json::from_str(&json).expect("Failed to deserialize event");

	match deserialized {
		PoolEvent::ConnectionAcquired { connection_id, .. } => {
			assert_eq!(connection_id, "test-conn");
		}
		_ => panic!("Wrong event type after deserialization"),
	}
}

#[tokio::test]
async fn test_event_timestamps() {
	// Test that events include timestamps
	let event1 = PoolEvent::connection_acquired("conn-1".to_string());
	let event2 = PoolEvent::connection_acquired("conn-2".to_string());

	match (&event1, &event2) {
		(
			PoolEvent::ConnectionAcquired { timestamp: ts1, .. },
			PoolEvent::ConnectionAcquired { timestamp: ts2, .. },
		) => {
			assert!(ts2 >= ts1, "Second event should have later timestamp");
		}
		_ => panic!("Wrong event types"),
	}
}

// NOTE: The following tests are based on SQLAlchemy's event system tests.
// These tests verify that pool events are properly emitted during connection lifecycle operations.
// The event emission system is functional but may require additional hooks for advanced use cases.

#[rstest]
#[tokio::test]
async fn test_checkout_event(_event_poller: EventPoller) {
	// Test that checkout event fires when connection is checked out
	// Based on SQLAlchemy test_checkout_event
	let url = "sqlite::memory:";
	let config = PoolConfig::default();

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	let (listener, events) = TestEventListener::new();
	pool.add_listener(Arc::new(listener)).await;

	// Acquire connection (should trigger checkout event)
	let _conn = pool.acquire().await.expect("Failed to acquire connection");

	// Wait for event to be recorded
	EventPoller::wait_for_any_event(events.clone(), Duration::from_millis(100))
		.await
		.expect("Should receive checkout event within 100ms");

	let events = events.lock().await;
	assert!(!events.is_empty(), "Should have received checkout event");

	// Verify it's a ConnectionAcquired event
	let has_acquired = events
		.iter()
		.any(|e| matches!(e, PoolEvent::ConnectionAcquired { .. }));
	assert!(
		has_acquired,
		"Should have received ConnectionAcquired event"
	);
}

#[rstest]
#[tokio::test]
async fn test_checkin_event(_event_poller: EventPoller) {
	// Test that checkin event fires when connection is returned
	// Based on SQLAlchemy test_checkin_event
	let url = "sqlite::memory:";
	let config = PoolConfig::default();

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	let (listener, events) = TestEventListener::new();
	pool.add_listener(Arc::new(listener)).await;

	{
		let _conn = pool.acquire().await.expect("Failed to acquire connection");
		// Connection returned on drop
	}

	// Wait for ConnectionReturned event
	EventPoller::wait_for_event(events.clone(), Duration::from_millis(100), |e| {
		matches!(e, PoolEvent::ConnectionReturned { .. })
	})
	.await
	.expect("Should receive ConnectionReturned event within 100ms");

	let events = events.lock().await;
	let returned_events: Vec<_> = events
		.iter()
		.filter(|e| matches!(e, PoolEvent::ConnectionReturned { .. }))
		.collect();

	assert!(
		!returned_events.is_empty(),
		"Should have received checkin event"
	);
}

#[rstest]
#[tokio::test]
async fn test_connect_event(_event_poller: EventPoller) {
	// Test that connect event fires on connection creation (first connect)
	// Based on SQLAlchemy test_connect_event
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(5);

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	let (listener, events) = TestEventListener::new();
	pool.add_listener(Arc::new(listener)).await;

	let _conn = pool.acquire().await.expect("Failed to acquire connection");

	// Wait for ConnectionCreated event
	EventPoller::wait_for_event(events.clone(), Duration::from_millis(100), |e| {
		matches!(e, PoolEvent::ConnectionCreated { .. })
	})
	.await
	.expect("Should receive ConnectionCreated event within 100ms");

	let events = events.lock().await;
	let created_events: Vec<_> = events
		.iter()
		.filter(|e| matches!(e, PoolEvent::ConnectionCreated { .. }))
		.collect();

	assert!(
		!created_events.is_empty(),
		"Should have received connect event (first connect)"
	);
}

#[rstest]
#[tokio::test]
async fn test_first_connect_event(_event_poller: EventPoller) {
	// Test that first_connect event fires only once
	// Based on SQLAlchemy test_first_connect_event
	let url = "sqlite::memory:";
	let config = PoolConfig::new().with_min_connections(0);

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	let (listener, events) = TestEventListener::new();
	pool.add_listener(Arc::new(listener)).await;

	// First connection
	let _conn1 = pool.acquire().await.expect("Failed to acquire conn1");

	// Wait for first ConnectionCreated event
	EventPoller::wait_for_event(events.clone(), Duration::from_millis(100), |e| {
		matches!(e, PoolEvent::ConnectionCreated { .. })
	})
	.await
	.expect("Should receive first ConnectionCreated event within 100ms");

	// Second connection
	let _conn2 = pool.acquire().await.expect("Failed to acquire conn2");

	// Small delay to ensure no additional events
	tokio::time::sleep(Duration::from_millis(50)).await;

	// Verify that ConnectionCreated event fired only once (for first connect)
	let events = events.lock().await;
	let created_count = events
		.iter()
		.filter(|e| matches!(e, PoolEvent::ConnectionCreated { .. }))
		.count();

	assert_eq!(
		created_count, 1,
		"ConnectionCreated (first_connect) should fire only once"
	);
}

#[rstest]
#[tokio::test]
async fn test_checkout_event_fires_subsequent(_event_poller: EventPoller) {
	// Test that checkout event fires for each checkout
	// Based on SQLAlchemy test_checkout_event_fires_subsequent
	let url = "sqlite::memory:";
	let config = PoolConfig::default();

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	let (listener, events) = TestEventListener::new();
	pool.add_listener(Arc::new(listener)).await;

	// Multiple checkouts
	for _ in 0..3 {
		let _conn = pool.acquire().await.expect("Failed to acquire connection");
	}

	// Wait for at least 3 ConnectionAcquired events
	EventPoller::wait_for_event_count(events.clone(), 3, Duration::from_millis(200))
		.await
		.expect("Should receive at least 3 ConnectionAcquired events within 200ms");

	let events = events.lock().await;
	let acquired_count = events
		.iter()
		.filter(|e| matches!(e, PoolEvent::ConnectionAcquired { .. }))
		.count();

	assert!(
		acquired_count >= 3,
		"Should have received at least 3 ConnectionAcquired events, got {}",
		acquired_count
	);
}

#[rstest]
#[tokio::test]
async fn test_soft_invalidate_event(_event_poller: EventPoller) {
	// Test soft_invalidate event
	// Based on SQLAlchemy test_soft_invalidate_event_no_exception
	let url = "sqlite::memory:";
	let config = PoolConfig::default();

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	let (listener, events) = TestEventListener::new();
	pool.add_listener(Arc::new(listener)).await;

	{
		let mut conn = pool.acquire().await.expect("Failed to acquire connection");
		conn.soft_invalidate().await;
	}

	// Wait for ConnectionSoftInvalidated event
	EventPoller::wait_for_event(events.clone(), Duration::from_millis(100), |e| {
		matches!(e, PoolEvent::ConnectionSoftInvalidated { .. })
	})
	.await
	.expect("Should receive ConnectionSoftInvalidated event within 100ms");

	let events = events.lock().await;
	let soft_invalidated = events
		.iter()
		.any(|e| matches!(e, PoolEvent::ConnectionSoftInvalidated { .. }));

	assert!(
		soft_invalidated,
		"Should have received ConnectionSoftInvalidated event"
	);
}

#[rstest]
#[tokio::test]
async fn test_invalidate_event(_event_poller: EventPoller) {
	// Test invalidate event
	// Based on SQLAlchemy test_invalidate_event_no_exception
	let url = "sqlite::memory:";
	let config = PoolConfig::default();

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	let (listener, events) = TestEventListener::new();
	pool.add_listener(Arc::new(listener)).await;

	{
		let conn = pool.acquire().await.expect("Failed to acquire connection");
		conn.invalidate("Test invalidation".to_string()).await;
	}

	// Wait for ConnectionInvalidated event
	EventPoller::wait_for_event(events.clone(), Duration::from_millis(100), |e| {
		matches!(e, PoolEvent::ConnectionInvalidated { .. })
	})
	.await
	.expect("Should receive ConnectionInvalidated event within 100ms");

	let events = events.lock().await;
	let invalidated = events
		.iter()
		.any(|e| matches!(e, PoolEvent::ConnectionInvalidated { .. }));

	assert!(
		invalidated,
		"Should have received ConnectionInvalidated event"
	);
}

#[rstest]
#[tokio::test]
async fn test_reset_event(_event_poller: EventPoller) {
	// Test reset event on connection return
	// Based on SQLAlchemy test_reset_event
	let url = "sqlite::memory:";
	let config = PoolConfig::default();

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	let (listener, events) = TestEventListener::new();
	pool.add_listener(Arc::new(listener)).await;

	{
		let mut conn = pool.acquire().await.expect("Failed to acquire connection");
		conn.reset().await;
	}

	// Wait for ConnectionReset event
	EventPoller::wait_for_event(events.clone(), Duration::from_millis(100), |e| {
		matches!(e, PoolEvent::ConnectionReset { .. })
	})
	.await
	.expect("Should receive ConnectionReset event within 100ms");

	let events = events.lock().await;
	let reset = events
		.iter()
		.any(|e| matches!(e, PoolEvent::ConnectionReset { .. }));

	assert!(reset, "Should have received ConnectionReset event");
}
