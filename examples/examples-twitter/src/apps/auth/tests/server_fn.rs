//! Auth server function E2E integration tests
//!
//! Tests that exercise the full DI resolution pipeline by routing HTTP requests
//! through `ServerRouter::handle()`. This ensures `#[inject]` parameters
//! (`DatabaseConnection`, `SessionData`, `SessionStoreRef`) are resolved via
//! `InjectionContext` rather than passed directly as function arguments.
//!
//! Covers: Issue #3525 — DI bypass in auth server_fn tests
use crate::apps::auth::shared::server_fn::{current_user, login, logout};
use crate::apps::auth::shared::types::UserInfo;
use crate::test_utils::factories::user::UserFactory;
use crate::test_utils::fixtures::database::twitter_db_pool;
use crate::test_utils::fixtures::users::TestTwitterUser;
use bytes::Bytes;
use reinhardt::db::orm::reinitialize_database;
use reinhardt::di::{InjectionContext, SingletonScope};
use reinhardt::middleware::session::{SessionConfig, SessionData, SessionStore};
use reinhardt::pages::server_fn::ServerFnRouterExt;
use reinhardt::{
	BaseUser, DatabaseConnection, Handler, Method, Request, StatusCode, UnifiedRouter,
};
use rstest::*;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
const SESSION_COOKIE_NAME: &str = "sessionid";
/// Build a `ServerRouter` wired with auth server function routes, DI context,
/// and `SessionMiddleware`. Returns the router and a shared `Arc<SessionStore>`
/// for pre-populating / inspecting sessions in tests.
async fn build_auth_router(url: &str) -> (impl Handler, Arc<SessionStore>) {
	let db = DatabaseConnection::connect_postgres(url)
		.await
		.expect("DB connection should succeed");
	let store = Arc::new(SessionStore::new());
	let singleton = Arc::new(SingletonScope::new());
	singleton.set(db);
	singleton.set(Arc::clone(&store));
	let di_ctx = Arc::new(InjectionContext::builder(singleton).build());
	let session_config =
		SessionConfig::new(SESSION_COOKIE_NAME.to_string(), Duration::from_secs(3600));
	let session_mw = reinhardt::middleware::session::SessionMiddleware::from_arc(
		session_config,
		Arc::clone(&store),
	);
	let router = UnifiedRouter::new()
		.server(|s| {
			s.server_fn(login::marker)
				.server_fn(logout::marker)
				.server_fn(current_user::marker)
		})
		.with_di_context(di_ctx)
		.with_middleware(session_mw)
		.into_server();
	(router, store)
}
/// Build an HTTP POST request with JSON body and an optional session cookie.
fn make_request(path: &str, body: serde_json::Value, session_id: Option<&str>) -> Request {
	let body_bytes = serde_json::to_vec(&body).expect("Failed to serialize body");
	let mut builder = Request::builder()
		.method(Method::POST)
		.uri(path)
		.header("content-type", "application/json");
	if let Some(sid) = session_id {
		builder = builder.header("cookie", format!("{}={}", SESSION_COOKIE_NAME, sid));
	}
	builder
		.body(Bytes::from(body_bytes))
		.build()
		.expect("Failed to build request")
}
/// Extract session cookie value from response Set-Cookie headers.
fn extract_session_cookie(response: &reinhardt::Response) -> Option<String> {
	response
		.headers
		.get_all("set-cookie")
		.iter()
		.find_map(|val| {
			let s = val.to_str().ok()?;
			let prefix = format!("{}=", SESSION_COOKIE_NAME);
			if s.starts_with(&prefix) {
				let after = &s[prefix.len()..];
				Some(after.split(';').next()?.to_string())
			} else {
				None
			}
		})
}
/// Parse a successful response body as `T`.
fn parse_ok_body<T: serde::de::DeserializeOwned>(response: &reinhardt::Response) -> T {
	serde_json::from_slice(&response.body).unwrap_or_else(|e| {
		panic!(
			"Failed to parse response body: {}. Body: {:?}",
			e,
			String::from_utf8_lossy(&response.body)
		)
	})
}
#[rstest]
#[tokio::test]
async fn test_login_server_fn_success(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, url) = twitter_db_pool.await;
	reinitialize_database(&url)
		.await
		.expect("Database initialization should succeed");
	let factory = UserFactory::new();
	let test_user = TestTwitterUser::new("loginfnuser").with_password("ValidPassword123");
	let user = factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	let (router, _store) = build_auth_router(&url).await;
	let request = make_request(
		"/api/server_fn/login",
		json!(
			{ "email" : test_user.email, "password" : test_user.password, "_csrf_token" :
			"" }
		),
		None,
	);
	let response = router
		.handle(request)
		.await
		.expect("Router should handle request");
	assert_eq!(response.status, StatusCode::OK, "Login should succeed");
	let user_info: UserInfo = parse_ok_body(&response);
	assert_eq!(user_info.id, user.id);
	assert_eq!(user_info.username, test_user.username);
	assert_eq!(user_info.email, test_user.email);
	assert!(user_info.is_active);
}
#[rstest]
#[tokio::test]
async fn test_login_server_fn_invalid_credentials(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, url) = twitter_db_pool.await;
	reinitialize_database(&url)
		.await
		.expect("Database initialization should succeed");
	let factory = UserFactory::new();
	let test_user = TestTwitterUser::new("badpwdfnuser").with_password("CorrectPassword123");
	factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	let (router, _store) = build_auth_router(&url).await;
	let request = make_request(
		"/api/server_fn/login",
		json!(
			{ "email" : test_user.email, "password" : "WrongPassword456", "_csrf_token" :
			"" }
		),
		None,
	);
	let response = router
		.handle(request)
		.await
		.expect("Router should handle request");
	assert_ne!(
		response.status,
		StatusCode::OK,
		"Login with wrong password should fail"
	);
}
#[rstest]
#[tokio::test]
async fn test_login_server_fn_nonexistent_user(#[future] twitter_db_pool: (PgPool, String)) {
	let (_pool, url) = twitter_db_pool.await;
	reinitialize_database(&url)
		.await
		.expect("Database initialization should succeed");
	let (router, _store) = build_auth_router(&url).await;
	let request = make_request(
		"/api/server_fn/login",
		json!(
			{ "email" : "nonexistent@example.com", "password" : "SomePassword123",
			"_csrf_token" : "" }
		),
		None,
	);
	let response = router
		.handle(request)
		.await
		.expect("Router should handle request");
	assert_ne!(
		response.status,
		StatusCode::OK,
		"Login with nonexistent user should fail"
	);
}
#[rstest]
#[tokio::test]
async fn test_login_server_fn_inactive_user(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, url) = twitter_db_pool.await;
	reinitialize_database(&url)
		.await
		.expect("Database initialization should succeed");
	let factory = UserFactory::new();
	let test_user = TestTwitterUser::new("inactivefnuser")
		.with_password("ValidPassword123")
		.with_active(false);
	factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	let (router, _store) = build_auth_router(&url).await;
	let request = make_request(
		"/api/server_fn/login",
		json!(
			{ "email" : test_user.email, "password" : test_user.password, "_csrf_token" :
			"" }
		),
		None,
	);
	let response = router
		.handle(request)
		.await
		.expect("Router should handle request");
	assert_ne!(
		response.status,
		StatusCode::OK,
		"Login with inactive user should fail"
	);
}
#[rstest]
#[tokio::test]
async fn test_current_user_authenticated(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, url) = twitter_db_pool.await;
	reinitialize_database(&url)
		.await
		.expect("Database initialization should succeed");
	let factory = UserFactory::new();
	let test_user = TestTwitterUser::new("currentfnuser").with_password("ValidPassword123");
	let created_user = factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	let (router, store) = build_auth_router(&url).await;
	let mut session = SessionData::new(Duration::from_secs(3600));
	session
		.set("user_id".to_string(), created_user.id)
		.expect("Session set should succeed");
	let session_id = session.id.clone();
	store.save(session);
	let request = make_request("/api/server_fn/current_user", json!({}), Some(&session_id));
	let response = router
		.handle(request)
		.await
		.expect("Router should handle request");
	assert_eq!(
		response.status,
		StatusCode::OK,
		"current_user should succeed"
	);
	let user_info: Option<UserInfo> = parse_ok_body(&response);
	assert!(user_info.is_some(), "Should return user info");
	let user_info = user_info.unwrap();
	assert_eq!(user_info.id, created_user.id);
	assert_eq!(user_info.username, test_user.username);
	assert_eq!(user_info.email, test_user.email);
}
#[rstest]
#[tokio::test]
async fn test_current_user_unauthenticated(#[future] twitter_db_pool: (PgPool, String)) {
	let (_pool, url) = twitter_db_pool.await;
	reinitialize_database(&url)
		.await
		.expect("Database initialization should succeed");
	let (router, store) = build_auth_router(&url).await;
	let session = SessionData::new(Duration::from_secs(3600));
	let session_id = session.id.clone();
	store.save(session);
	let request = make_request("/api/server_fn/current_user", json!({}), Some(&session_id));
	let response = router
		.handle(request)
		.await
		.expect("Router should handle request");
	assert_eq!(
		response.status,
		StatusCode::OK,
		"current_user should succeed"
	);
	let user_info: Option<UserInfo> = parse_ok_body(&response);
	assert!(
		user_info.is_none(),
		"Should return None for unauthenticated session"
	);
}
#[rstest]
#[tokio::test]
async fn test_logout_server_fn(#[future] twitter_db_pool: (PgPool, String)) {
	let (_pool, url) = twitter_db_pool.await;
	reinitialize_database(&url)
		.await
		.expect("Database initialization should succeed");
	let (router, store) = build_auth_router(&url).await;
	let mut session = SessionData::new(Duration::from_secs(3600));
	session
		.set("user_id".to_string(), uuid::Uuid::now_v7())
		.expect("Session set should succeed");
	let session_id = session.id.clone();
	store.save(session);
	assert!(
		store.get(&session_id).is_some(),
		"Session should exist before logout"
	);
	let request = make_request("/api/server_fn/logout", json!({}), Some(&session_id));
	let response = router
		.handle(request)
		.await
		.expect("Router should handle request");
	assert_eq!(response.status, StatusCode::OK, "Logout should succeed");
	assert!(
		store.get(&session_id).is_none(),
		"Session should be deleted after logout"
	);
}
#[rstest]
#[tokio::test]
async fn test_login_persists_session_data(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, url) = twitter_db_pool.await;
	reinitialize_database(&url)
		.await
		.expect("Database initialization should succeed");
	let factory = UserFactory::new();
	let test_user = TestTwitterUser::new("persistfnuser").with_password("ValidPassword123");
	factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	let (router, store) = build_auth_router(&url).await;
	let old_session = SessionData::new(Duration::from_secs(3600));
	let old_session_id = old_session.id.clone();
	store.save(old_session);
	let request = make_request(
		"/api/server_fn/login",
		json!(
			{ "email" : test_user.email, "password" : test_user.password, "_csrf_token" :
			"" }
		),
		Some(&old_session_id),
	);
	let response = router
		.handle(request)
		.await
		.expect("Router should handle request");
	assert_eq!(response.status, StatusCode::OK, "Login should succeed");
	assert!(
		store.get(&old_session_id).is_none(),
		"Old session should be deleted after login"
	);
}
#[rstest]
#[tokio::test]
async fn test_auth_flow_login_then_current_user(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, url) = twitter_db_pool.await;
	reinitialize_database(&url)
		.await
		.expect("Database initialization should succeed");
	let factory = UserFactory::new();
	let test_user = TestTwitterUser::new("flowfnuser").with_password("ValidPassword123");
	let created_user = factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	let (router, store) = build_auth_router(&url).await;
	let request = make_request(
		"/api/server_fn/login",
		json!(
			{ "email" : test_user.email, "password" : test_user.password, "_csrf_token" :
			"" }
		),
		None,
	);
	let login_response = router
		.handle(request)
		.await
		.expect("Router should handle request");
	assert_eq!(
		login_response.status,
		StatusCode::OK,
		"Login should succeed"
	);
	let login_user_info: UserInfo = parse_ok_body(&login_response);
	assert_eq!(login_user_info.id, created_user.id);
	let current_session_id =
		extract_session_cookie(&login_response).expect("Login response should set session cookie");
	let post_login_session = store
		.get(&current_session_id)
		.expect("Post-login session should exist in store");
	assert!(
		post_login_session.get::<uuid::Uuid>("user_id").is_some(),
		"Post-login session should contain user_id"
	);
	let request = make_request(
		"/api/server_fn/current_user",
		json!({}),
		Some(&current_session_id),
	);
	let current_response = router
		.handle(request)
		.await
		.expect("Router should handle request");
	assert_eq!(
		current_response.status,
		StatusCode::OK,
		"current_user should succeed"
	);
	let current_user_info: Option<UserInfo> = parse_ok_body(&current_response);
	assert!(
		current_user_info.is_some(),
		"Should return user after login"
	);
	let current_user_info = current_user_info.unwrap();
	assert_eq!(current_user_info.id, created_user.id);
	assert_eq!(current_user_info.username, test_user.username);
	assert_eq!(current_user_info.email, test_user.email);
	let request = make_request(
		"/api/server_fn/logout",
		json!({}),
		Some(&current_session_id),
	);
	let logout_response = router
		.handle(request)
		.await
		.expect("Router should handle request");
	assert_eq!(
		logout_response.status,
		StatusCode::OK,
		"Logout should succeed"
	);
	let fresh_session = SessionData::new(Duration::from_secs(3600));
	let fresh_session_id = fresh_session.id.clone();
	store.save(fresh_session);
	let request = make_request(
		"/api/server_fn/current_user",
		json!({}),
		Some(&fresh_session_id),
	);
	let after_logout_response = router
		.handle(request)
		.await
		.expect("Router should handle request");
	assert_eq!(
		after_logout_response.status,
		StatusCode::OK,
		"current_user after logout should succeed"
	);
	let after_logout_info: Option<UserInfo> = parse_ok_body(&after_logout_response);
	assert!(
		after_logout_info.is_none(),
		"Should return None after logout"
	);
}
#[rstest]
#[tokio::test]
async fn test_login_success(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let factory = UserFactory::new();
	let test_user = TestTwitterUser::new("loginuser").with_password("ValidPassword123");
	let user = factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	assert_eq!(&user.email, &test_user.email);
	assert!(user.is_active);
	let password_valid = user
		.check_password(&test_user.password)
		.expect("Password check should succeed");
	assert!(password_valid, "Password should be valid");
}
#[rstest]
#[tokio::test]
async fn test_login_invalid_password(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let factory = UserFactory::new();
	let test_user = TestTwitterUser::new("wrongpwduser").with_password("CorrectPassword123");
	let user = factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	let password_valid = user
		.check_password("WrongPassword456")
		.expect("Password check should succeed");
	assert!(!password_valid, "Wrong password should fail");
}
#[rstest]
#[tokio::test]
async fn test_login_inactive_user(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let factory = UserFactory::new();
	let test_user = TestTwitterUser::new("inactiveuser").with_active(false);
	let user = factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	assert!(!user.is_active, "User should be inactive");
}
#[rstest]
#[tokio::test]
async fn test_register_success(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let existing = sqlx::query("SELECT id FROM auth_user WHERE email = $1")
		.bind("newuser@example.com")
		.fetch_optional(&pool)
		.await
		.expect("Query should succeed");
	assert!(existing.is_none(), "No user should exist with this email");
	let factory = UserFactory::new();
	let test_user = TestTwitterUser::new("newuser")
		.with_email("newuser@example.com")
		.with_password("SecurePassword123");
	let user = factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	assert_eq!(user.username, "newuser");
	assert_eq!(user.email, "newuser@example.com");
	assert!(user.is_active);
	let password_valid = user
		.check_password("SecurePassword123")
		.expect("Password check should succeed");
	assert!(password_valid);
}
#[rstest]
#[tokio::test]
async fn test_register_duplicate_email(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let factory = UserFactory::new();
	let test_user = TestTwitterUser::new("firstuser").with_email("duplicate@example.com");
	factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("First user creation should succeed");
	let test_user2 = TestTwitterUser::new("seconduser").with_email("duplicate@example.com");
	let result = factory.create_from_test_user(&pool, &test_user2).await;
	assert!(result.is_err(), "Duplicate email should fail");
}
#[rstest]
#[tokio::test]
async fn test_register_password_validation() {
	use crate::apps::auth::shared::types::RegisterRequest;
	use reinhardt::Validate;
	let request = RegisterRequest {
		username: "testuser".to_string(),
		email: "test@example.com".to_string(),
		password: "short".to_string(),
		password_confirmation: "short".to_string(),
	};
	let result = request.validate();
	assert!(result.is_err(), "Short password should fail validation");
}
#[rstest]
#[tokio::test]
async fn test_register_password_mismatch() {
	use crate::apps::auth::shared::types::RegisterRequest;
	let request = RegisterRequest {
		username: "testuser".to_string(),
		email: "test@example.com".to_string(),
		password: "SecurePassword123".to_string(),
		password_confirmation: "DifferentPassword456".to_string(),
	};
	let result = request.validate_passwords_match();
	assert!(result.is_err(), "Password mismatch should fail");
	assert!(result.unwrap_err().contains("do not match"));
}
#[rstest]
#[tokio::test]
async fn test_register_invalid_email() {
	use crate::apps::auth::shared::types::RegisterRequest;
	use reinhardt::Validate;
	let request = RegisterRequest {
		username: "testuser".to_string(),
		email: "not-an-email".to_string(),
		password: "SecurePassword123".to_string(),
		password_confirmation: "SecurePassword123".to_string(),
	};
	let result = request.validate();
	assert!(result.is_err(), "Invalid email should fail validation");
}
#[rstest]
#[tokio::test]
async fn test_register_short_username() {
	use crate::apps::auth::shared::types::RegisterRequest;
	use reinhardt::Validate;
	let request = RegisterRequest {
		username: "ab".to_string(),
		email: "test@example.com".to_string(),
		password: "SecurePassword123".to_string(),
		password_confirmation: "SecurePassword123".to_string(),
	};
	let result = request.validate();
	assert!(result.is_err(), "Short username should fail validation");
}
#[rstest]
#[tokio::test]
async fn test_login_request_validation_empty_password() {
	use crate::apps::auth::shared::types::LoginRequest;
	use reinhardt::Validate;
	let request = LoginRequest {
		email: "test@example.com".to_string(),
		password: "".to_string(),
	};
	let result = request.validate();
	assert!(result.is_err(), "Empty password should fail validation");
}
#[rstest]
#[tokio::test]
async fn test_login_request_validation_invalid_email() {
	use crate::apps::auth::shared::types::LoginRequest;
	use reinhardt::Validate;
	let request = LoginRequest {
		email: "invalid-email".to_string(),
		password: "ValidPassword123".to_string(),
	};
	let result = request.validate();
	assert!(result.is_err(), "Invalid email should fail validation");
}
#[rstest]
#[tokio::test]
async fn test_user_info_conversion(#[future] twitter_db_pool: (PgPool, String)) {
	let (pool, _url) = twitter_db_pool.await;
	let factory = UserFactory::new();
	let test_user = TestTwitterUser::new("infouser");
	let user = factory
		.create_from_test_user(&pool, &test_user)
		.await
		.expect("User creation should succeed");
	let user_info = UserInfo::from(user.clone());
	assert_eq!(user_info.id, user.id);
	assert_eq!(&user_info.username, &user.username);
	assert_eq!(&user_info.email, &user.email);
	assert_eq!(user_info.is_active, user.is_active);
}
