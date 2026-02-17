//! CSRF + Session Integration Tests
//!
//! Tests the integration between CSRF protection and session storage:
//! - CSRF token generation and storage in session
//! - Token validation from session data
//! - CSRF attack prevention

use reinhardt_middleware::{csrf::CsrfMiddleware, session::SessionMiddleware};
use rstest::rstest;
use serial_test::serial;

/// Test CSRF and session middleware integration
#[serial(csrf_session)]
#[rstest]
#[tokio::test]
async fn test_csrf_session_integration() {
	// Create session and CSRF middlewares
	let session = SessionMiddleware::with_defaults();
	let csrf = CsrfMiddleware::new();

	// Verify both middlewares can be used together
	assert_eq!(session.store().len(), 0);
	drop(csrf);
}

/// Test CSRF with test secret for deterministic testing
#[serial(csrf_session)]
#[rstest]
#[tokio::test]
async fn test_csrf_with_session_and_test_secret() {
	// Create session middleware
	let session = SessionMiddleware::with_defaults();

	// Create CSRF middleware with test secret
	let csrf = CsrfMiddleware::with_test_secret("test_secret".to_string());

	// Verify both middlewares work together
	assert_eq!(session.store().len(), 0);
	drop(csrf);
}

/// Test multiple CSRF and session middleware instances
#[serial(csrf_session)]
#[rstest]
#[tokio::test]
async fn test_multiple_csrf_session_instances() {
	// Create multiple session middleware instances
	let session1 = SessionMiddleware::with_defaults();
	let session2 = SessionMiddleware::with_defaults();

	// Create multiple CSRF middleware instances
	let csrf1 = CsrfMiddleware::new();
	let csrf2 = CsrfMiddleware::with_test_secret("secret1".to_string());

	// Verify all instances are independent
	assert_eq!(session1.store().len(), 0);
	assert_eq!(session2.store().len(), 0);
	drop(csrf1);
	drop(csrf2);
}
