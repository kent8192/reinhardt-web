//! CSRF + Session Integration Tests
//!
//! Tests the integration between CSRF protection and session storage:
//! - CSRF token generation and storage in session
//! - Token validation from session data
//! - CSRF attack prevention
//! - Token issuance after session ID rotation

use async_trait::async_trait;
use bytes::Bytes;
use hyper::header::SET_COOKIE;
use hyper::{HeaderMap, Method, StatusCode, Version};
use reinhardt_http::{Handler, Middleware, MiddlewareChain, Request, Response, Result};
use reinhardt_middleware::csrf::{CsrfMiddleware, CsrfMiddlewareConfig};
use reinhardt_middleware::session::{
	ActiveSessionId, SessionConfig, SessionMiddleware, SessionStore,
};
use rstest::rstest;
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;

/// Test CSRF and session middleware integration
#[serial(csrf_session)]
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

/// Handler that rotates the session ID on login for session-fixation prevention.
struct RotatingLoginHandler {
	store: Arc<SessionStore>,
}

#[async_trait]
impl Handler for RotatingLoginHandler {
	async fn handle(&self, request: Request) -> Result<Response> {
		let active_id = request
			.extensions
			.get::<ActiveSessionId>()
			.expect("ActiveSessionId should be present");
		let mut session = self
			.store
			.get(&active_id.get())
			.expect("session created by middleware should be present");
		session.id_holder = Some(active_id);
		let old_id = session.regenerate_id();
		self.store.delete(&old_id);
		self.store.save(session);
		Ok(Response::ok())
	}
}

struct OkHandler;

#[async_trait]
impl Handler for OkHandler {
	async fn handle(&self, _request: Request) -> Result<Response> {
		Ok(Response::ok())
	}
}

/// Session middleware wrapping CSRF middleware wrapping `handler`.
async fn send(
	store: &Arc<SessionStore>,
	handler: Arc<dyn Handler>,
	method: Method,
	headers: HeaderMap,
) -> Response {
	let mut csrf_config = CsrfMiddlewareConfig::default();
	csrf_config.check_referer_header = false;
	let csrf: Arc<dyn Middleware> = Arc::new(CsrfMiddleware::with_config(csrf_config));
	let inner = Arc::new(MiddlewareChain::new(handler).with_middleware(csrf));
	let session = SessionMiddleware::from_arc(
		SessionConfig::new("sessionid".to_string(), Duration::from_secs(3600)),
		Arc::clone(store),
	);
	let request = Request::builder()
		.method(method)
		.uri("/")
		.version(Version::HTTP_11)
		.headers(headers)
		.body(Bytes::new())
		.build()
		.unwrap();
	session.process(request, inner).await.unwrap()
}

fn set_cookie_value(response: &Response, name: &str) -> String {
	let prefix = format!("{name}=");
	response
		.headers
		.get_all(SET_COOKIE)
		.iter()
		.filter_map(|value| value.to_str().ok())
		.find_map(|cookie| cookie.split(';').next()?.strip_prefix(&prefix))
		.unwrap_or_else(|| panic!("{name} cookie should be set"))
		.to_string()
}

fn csrf_headers(session_id: &str, token: &str) -> HeaderMap {
	let mut headers = HeaderMap::new();
	headers.insert(
		"Cookie",
		format!("sessionid={session_id}; csrftoken={token}")
			.parse()
			.unwrap(),
	);
	headers.insert("X-CSRFToken", token.parse().unwrap());
	headers
}

/// A CSRF token issued by a response that rotates the session ID must be
/// valid for the rotated session cookie returned in that same response.
#[rstest]
#[tokio::test]
async fn test_csrf_token_issued_for_rotated_session_id() {
	// Arrange: obtain a session and a CSRF token for it.
	let store = Arc::new(SessionStore::new());
	let form = send(&store, Arc::new(OkHandler), Method::GET, HeaderMap::new()).await;
	let initial_session_id = set_cookie_value(&form, "sessionid");
	let initial_token = set_cookie_value(&form, "csrftoken");

	// Act: log in (rotating the session ID), then immediately POST with the
	// cookies and token returned by the login response.
	let login = send(
		&store,
		Arc::new(RotatingLoginHandler {
			store: Arc::clone(&store),
		}),
		Method::POST,
		csrf_headers(&initial_session_id, &initial_token),
	)
	.await;
	let rotated_session_id = set_cookie_value(&login, "sessionid");
	let rotated_token = set_cookie_value(&login, "csrftoken");
	let follow_up = send(
		&store,
		Arc::new(OkHandler),
		Method::POST,
		csrf_headers(&rotated_session_id, &rotated_token),
	)
	.await;

	// Assert
	assert_eq!(login.status, StatusCode::OK);
	assert_ne!(rotated_session_id, initial_session_id);
	assert_eq!(follow_up.status, StatusCode::OK);
}
