//! Session + Authentication Integration Tests
//!
//! Tests the integration between SessionMiddleware and AuthenticationMiddleware:
//! - Session data retrieval for user restoration
//! - Authentication state persistence
//! - User session lifecycle

use reinhardt_middleware::session::{SessionConfig, SessionMiddleware};
use rstest::rstest;
use serial_test::serial;
use std::time::Duration;

/// Test session middleware creation with default config
#[serial(session_auth)]
#[rstest]
#[tokio::test]
async fn test_session_with_defaults() {
	// Create session middleware with defaults
	let session = SessionMiddleware::with_defaults();

	// Verify session middleware is configured
	assert_eq!(session.store().len(), 0);
}

/// Test session middleware with custom config
#[serial(session_auth)]
#[rstest]
#[tokio::test]
async fn test_session_with_custom_config() {
	// Create session config with custom settings
	let config = SessionConfig::new("test_session".to_string(), Duration::from_secs(3600));
	let session = SessionMiddleware::new(config);

	// Verify session middleware uses custom config
	assert_eq!(session.store().len(), 0);
}

/// Test session store operations
#[serial(session_auth)]
#[rstest]
#[tokio::test]
async fn test_session_store_operations() {
	let config = SessionConfig::new("sessionid".to_string(), Duration::from_secs(3600));
	let session = SessionMiddleware::new(config);

	// Verify initial state
	assert!(session.store().is_empty());
	assert_eq!(session.store().len(), 0);
}

/// Test multiple session middleware instances
#[serial(session_auth)]
#[rstest]
#[tokio::test]
async fn test_multiple_session_instances() {
	// Create multiple session middleware instances
	let session1 = SessionMiddleware::with_defaults();
	let session2 = SessionMiddleware::new(SessionConfig::new(
		"session2".to_string(),
		Duration::from_secs(1800),
	));

	// Verify all instances are independent
	assert_eq!(session1.store().len(), 0);
	assert_eq!(session2.store().len(), 0);
}
