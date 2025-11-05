use bytes::Bytes;
use hyper::{HeaderMap, Method, Uri, Version};
use reinhardt_apps::Request;
use reinhardt_auth::{Authentication, AuthenticationBackend, SessionAuthentication};
use reinhardt_sessions::{Session, backends::InMemorySessionBackend};
use uuid::Uuid;

/// Test SessionAuthentication with InMemorySessionBackend
#[tokio::test]
async fn test_session_authentication_with_inmemory_backend() {
	let session_backend = InMemorySessionBackend::new();
	let auth = SessionAuthentication::new(session_backend.clone());

	// Create a test session with user data
	let user_id = Uuid::new_v4();
	let mut session = Session::new(session_backend.clone());
	session.set("_auth_user_id", &user_id.to_string()).unwrap();
	session
		.set("_auth_user_name", &"alice".to_string())
		.unwrap();
	session
		.set("_auth_user_email", &"alice@example.com".to_string())
		.unwrap();
	session.set("_auth_user_is_active", &true).unwrap();
	session.set("_auth_user_is_admin", &false).unwrap();
	session.set("_auth_user_is_staff", &false).unwrap();
	session.set("_auth_user_is_superuser", &false).unwrap();

	// Save session to backend
	session.save().await.unwrap();
	let session_key = session.session_key().unwrap();

	// Create request with session cookie
	let mut headers = HeaderMap::new();
	headers.insert(
		"Cookie",
		format!("sessionid={}", session_key).parse().unwrap(),
	);

	let request = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);

	// Test authentication
	let result = Authentication::authenticate(&auth, &request).await.unwrap();
	assert!(result.is_some());

	let user = result.unwrap();
	assert_eq!(user.id(), user_id.to_string());
	assert_eq!(user.get_username(), "alice");
	assert!(user.is_active());
	assert!(!user.is_admin());
}

/// Test SessionAuthentication without session cookie
#[tokio::test]
async fn test_session_authentication_no_cookie() {
	let session_backend = InMemorySessionBackend::new();
	let auth = SessionAuthentication::new(session_backend);

	// Create request without session cookie
	let headers = HeaderMap::new();
	let request = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);

	// Should return None (no authentication)
	let result = Authentication::authenticate(&auth, &request).await.unwrap();
	assert!(result.is_none());
}

/// Test SessionAuthentication with invalid session key
#[tokio::test]
async fn test_session_authentication_invalid_session_key() {
	let session_backend = InMemorySessionBackend::new();
	let auth = SessionAuthentication::new(session_backend);

	// Create request with invalid session cookie
	let mut headers = HeaderMap::new();
	headers.insert("Cookie", "sessionid=invalid_key".parse().unwrap());

	let request = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);

	// Should return None (invalid session)
	let result = Authentication::authenticate(&auth, &request).await.unwrap();
	assert!(result.is_none());
}

/// Test SessionAuthentication with session without user data
#[tokio::test]
async fn test_session_authentication_no_user_data() {
	let session_backend = InMemorySessionBackend::new();
	let auth = SessionAuthentication::new(session_backend.clone());

	// Create a session without user data (but with some other data to make it valid)
	let mut session = Session::new(session_backend.clone());
	session.set("_some_data", &"value".to_string()).unwrap();
	session.save().await.unwrap();
	let session_key = session.session_key().unwrap();

	// Create request with session cookie
	let mut headers = HeaderMap::new();
	headers.insert(
		"Cookie",
		format!("sessionid={}", session_key).parse().unwrap(),
	);

	let request = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);

	// Should return None (no user data in session)
	let result = Authentication::authenticate(&auth, &request).await.unwrap();
	assert!(result.is_none());
}

/// Test SessionAuthentication get_user() method
#[tokio::test]
async fn test_session_authentication_get_user() {
	let session_backend = InMemorySessionBackend::new();
	let auth = SessionAuthentication::new(session_backend);

	// Test get_user (returns minimal SimpleUser)
	let user_id = Uuid::new_v4();
	let result = auth.get_user(&user_id.to_string()).await.unwrap();
	assert!(result.is_some());

	let user = result.unwrap();
	assert_eq!(user.id(), user_id.to_string());
	assert!(user.is_active());
}

/// Test SessionAuthentication with custom cookie name
#[tokio::test]
async fn test_session_authentication_custom_cookie_name() {
	use reinhardt_auth::SessionAuthConfig;

	let session_backend = InMemorySessionBackend::new();
	let config = SessionAuthConfig {
		cookie_name: "custom_session".to_string(),
		enforce_csrf: true,
	};
	let auth = SessionAuthentication::with_config(config, session_backend.clone());

	// Create a test session with user data
	let user_id = Uuid::new_v4();
	let mut session = Session::new(session_backend.clone());
	session.set("_auth_user_id", &user_id.to_string()).unwrap();
	session.save().await.unwrap();
	let session_key = session.session_key().unwrap();

	// Create request with custom cookie name
	let mut headers = HeaderMap::new();
	headers.insert(
		"Cookie",
		format!("custom_session={}", session_key).parse().unwrap(),
	);

	let request = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);

	// Test authentication
	let result = Authentication::authenticate(&auth, &request).await.unwrap();
	assert!(result.is_some());

	let user = result.unwrap();
	assert_eq!(user.id(), user_id.to_string());
}
