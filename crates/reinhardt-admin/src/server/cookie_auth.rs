//! Admin Cookie-based JWT Authentication Middleware
//!
//! Extracts JWT tokens from the `reinhardt_admin_token` HTTP-Only cookie
//! and populates the request's [`reinhardt_http::AuthState`] extension. This replaces the
//! `Authorization: Bearer` header approach for the admin panel, providing
//! XSS protection since JavaScript cannot access HTTP-Only cookies.
//!
//! # Extraction Order
//!
//! 1. `reinhardt_admin_token` cookie (primary — set by admin login)
//! 2. `Authorization: Bearer` header (fallback — for API testing / migration)
//!
//! # Security Properties
//!
//! - The cookie is `HttpOnly`, so XSS attacks cannot steal the token.
//! - `SameSite=Strict` prevents cross-origin cookie sending (CSRF protection).
//! - `Path=/admin` limits the cookie scope to admin routes.
//! - `Secure` flag ensures HTTPS-only transmission in production.

use async_trait::async_trait;
use reinhardt_auth::JwtAuth;
use reinhardt_http::{
	AuthState, Extensions, Handler, IsActive, IsAdmin, IsAuthenticated, Middleware, Request,
	Response, Result,
};
use std::sync::Arc;

use super::security::extract_admin_auth_cookie;

/// Admin-specific JWT authentication middleware.
///
/// Unlike the general `JwtAuthMiddleware` from reinhardt-middleware, this
/// middleware extracts JWT tokens from the `reinhardt_admin_token` cookie
/// first, falling back to the `Authorization: Bearer` header.
///
/// # Example
///
/// ```ignore
/// use reinhardt_admin::server::cookie_auth::AdminCookieAuthMiddleware;
///
/// let middleware = AdminCookieAuthMiddleware::new(b"jwt-secret");
/// let router = ServerRouter::new()
///     .with_namespace("admin")
///     .with_middleware(middleware);
/// ```
pub struct AdminCookieAuthMiddleware {
	jwt_auth: JwtAuth,
}

impl AdminCookieAuthMiddleware {
	/// Creates a new admin cookie auth middleware from a JWT secret.
	pub fn new(secret: &[u8]) -> Self {
		Self {
			jwt_auth: JwtAuth::new(secret),
		}
	}

	/// Creates a new admin cookie auth middleware from a pre-built `JwtAuth`.
	pub fn from_jwt_auth(jwt_auth: JwtAuth) -> Self {
		Self { jwt_auth }
	}

	/// Extracts the JWT token from the admin auth cookie or Authorization header.
	fn extract_token(request: &Request) -> Option<String> {
		// 1. Try cookie first (primary for admin panel)
		if let Some(token) = extract_admin_auth_cookie(&request.headers) {
			return Some(token);
		}

		// 2. Fall back to Authorization: Bearer header (API testing / migration)
		request
			.headers
			.get("Authorization")
			.and_then(|v| v.to_str().ok())
			.and_then(|s| s.strip_prefix("Bearer "))
			.map(|s| s.to_string())
	}
}

#[async_trait]
impl Middleware for AdminCookieAuthMiddleware {
	async fn process(&self, request: Request, next: Arc<dyn Handler>) -> Result<Response> {
		let auth_state = if let Some(token) = Self::extract_token(&request)
			&& let Ok(claims) = self.jwt_auth.verify_token(&token)
		{
			let is_admin = claims.is_staff || claims.is_superuser;
			AuthState::authenticated(claims.sub, is_admin, true)
		} else {
			AuthState::anonymous()
		};

		install_auth_state(&request.extensions, auth_state);
		next.handle(request).await
	}
}

/// Installs `state` together with the legacy compatibility entries
/// (`String` user ID, [`IsAuthenticated`], [`IsAdmin`], [`IsActive`]) that
/// existing code reads from extensions directly.
///
/// The legacy flags are always overwritten together with the canonical
/// state, so an anonymous state writes `false` flags and discards any
/// principal left behind by an earlier authentication layer instead of
/// letting legacy consumers observe it after admin authentication failed.
///
/// An anonymous state blanks an existing user ID but never adds one: the
/// untyped `String` entry is also read as the CSRF session identifier and
/// the rate-limit user key, and an empty ID added to every anonymous request
/// would give them all the same CSRF secret. This mirrors the helper used by
/// the authentication middleware in `reinhardt-middleware`.
fn install_auth_state(extensions: &Extensions, state: AuthState) {
	if state.is_authenticated() {
		extensions.insert(state.user_id().to_owned());
	} else if extensions.contains::<String>() {
		extensions.insert(String::new());
	}
	extensions.insert(IsAuthenticated(state.is_authenticated()));
	extensions.insert(IsAdmin(state.is_admin()));
	extensions.insert(IsActive(state.is_active()));
	extensions.insert(state);
}

#[cfg(all(test, server))]
mod tests {
	use super::*;
	use crate::server::security::ADMIN_AUTH_COOKIE_NAME;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, StatusCode, Version};
	use reinhardt_auth::JwtAuth;
	use rstest::rstest;
	use serde_json::{Value, json};

	const PRIOR_USER_ID: &str = "prior-admin";

	struct AuthCheckHandler;

	/// Echoes the canonical state and every legacy entry, the way a
	/// downstream legacy consumer would read them.
	struct AuthEntriesHandler;

	#[async_trait]
	impl Handler for AuthEntriesHandler {
		async fn handle(&self, request: Request) -> Result<Response> {
			let extensions = &request.extensions;
			let auth_state = extensions.get::<AuthState>().map(|state| {
				json!({
					"user_id": state.user_id(),
					"is_authenticated": state.is_authenticated(),
					"is_admin": state.is_admin(),
					"is_active": state.is_active(),
				})
			});
			Ok(Response::ok().with_json(&json!({
				"auth_state": auth_state,
				"user_id": extensions.get::<String>(),
				"is_authenticated": extensions.get::<IsAuthenticated>().map(|v| v.0),
				"is_admin": extensions.get::<IsAdmin>().map(|v| v.0),
				"is_active": extensions.get::<IsActive>().map(|v| v.0),
			}))?)
		}
	}

	/// Simulates an earlier authentication layer that already installed an
	/// active admin principal in both the canonical and the legacy entries.
	fn seed_prior_principal(request: &Request) {
		let extensions = &request.extensions;
		extensions.insert(PRIOR_USER_ID.to_owned());
		extensions.insert(IsAuthenticated(true));
		extensions.insert(IsAdmin(true));
		extensions.insert(IsActive(true));
		extensions.insert(AuthState::authenticated(PRIOR_USER_ID, true, true));
	}

	fn expected_entries(
		user_id: &str,
		is_authenticated: bool,
		is_admin: bool,
		is_active: bool,
	) -> Value {
		json!({
			"auth_state": {
				"user_id": user_id,
				"is_authenticated": is_authenticated,
				"is_admin": is_admin,
				"is_active": is_active,
			},
			"user_id": user_id,
			"is_authenticated": is_authenticated,
			"is_admin": is_admin,
			"is_active": is_active,
		})
	}

	fn auth_entries(response: &Response) -> Value {
		serde_json::from_slice(response.body.as_ref()).unwrap()
	}

	#[async_trait]
	impl Handler for AuthCheckHandler {
		async fn handle(&self, request: Request) -> Result<Response> {
			let auth_state: AuthState = request
				.extensions
				.get()
				.unwrap_or_else(AuthState::anonymous);
			let body = if auth_state.is_authenticated() {
				format!("authenticated:{}", auth_state.user_id())
			} else {
				"anonymous".to_string()
			};
			Ok(Response::new(StatusCode::OK).with_body(body))
		}
	}

	fn make_request(headers: HeaderMap) -> Request {
		Request::builder()
			.method(Method::POST)
			.uri("/admin/api/server_fn/get_list")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap()
	}

	fn test_secret() -> &'static [u8] {
		b"test-secret-key-for-jwt-testing-purposes"
	}

	fn generate_test_token(user_id: &str) -> String {
		let jwt_auth = JwtAuth::new(test_secret());
		jwt_auth
			.generate_token(user_id.to_string(), "admin".to_string(), true, false)
			.unwrap()
	}

	#[tokio::test]
	async fn test_no_token_returns_anonymous() {
		let mw = AdminCookieAuthMiddleware::new(test_secret());
		let next = Arc::new(AuthCheckHandler);
		let req = make_request(HeaderMap::new());
		let resp = mw.process(req, next).await.unwrap();
		assert_eq!(resp.body, "anonymous");
	}

	#[tokio::test]
	async fn test_cookie_token_authenticates() {
		let token = generate_test_token("user-123");
		let mw = AdminCookieAuthMiddleware::new(test_secret());
		let next = Arc::new(AuthCheckHandler);
		let mut headers = HeaderMap::new();
		headers.insert(
			"cookie",
			format!("{}={}", ADMIN_AUTH_COOKIE_NAME, token)
				.parse()
				.unwrap(),
		);
		let req = make_request(headers);
		let resp = mw.process(req, next).await.unwrap();
		assert_eq!(resp.body, "authenticated:user-123");
	}

	#[tokio::test]
	async fn test_bearer_token_authenticates_as_fallback() {
		let token = generate_test_token("user-456");
		let mw = AdminCookieAuthMiddleware::new(test_secret());
		let next = Arc::new(AuthCheckHandler);
		let mut headers = HeaderMap::new();
		headers.insert(
			"Authorization",
			format!("Bearer {}", token).parse().unwrap(),
		);
		let req = make_request(headers);
		let resp = mw.process(req, next).await.unwrap();
		assert_eq!(resp.body, "authenticated:user-456");
	}

	#[tokio::test]
	async fn test_cookie_takes_precedence_over_bearer() {
		let cookie_token = generate_test_token("cookie-user");
		let bearer_token = generate_test_token("bearer-user");
		let mw = AdminCookieAuthMiddleware::new(test_secret());
		let next = Arc::new(AuthCheckHandler);
		let mut headers = HeaderMap::new();
		headers.insert(
			"cookie",
			format!("{}={}", ADMIN_AUTH_COOKIE_NAME, cookie_token)
				.parse()
				.unwrap(),
		);
		headers.insert(
			"Authorization",
			format!("Bearer {}", bearer_token).parse().unwrap(),
		);
		let req = make_request(headers);
		let resp = mw.process(req, next).await.unwrap();
		assert_eq!(resp.body, "authenticated:cookie-user");
	}

	#[tokio::test]
	async fn test_invalid_token_returns_anonymous() {
		let mw = AdminCookieAuthMiddleware::new(test_secret());
		let next = Arc::new(AuthCheckHandler);
		let mut headers = HeaderMap::new();
		headers.insert(
			"cookie",
			format!("{}=invalid.token.here", ADMIN_AUTH_COOKIE_NAME)
				.parse()
				.unwrap(),
		);
		let req = make_request(headers);
		let resp = mw.process(req, next).await.unwrap();
		assert_eq!(resp.body, "anonymous");
	}

	#[rstest]
	#[case::missing_token(None)]
	#[case::invalid_cookie_token(Some(format!("{}=invalid.token.here", ADMIN_AUTH_COOKIE_NAME)))]
	#[tokio::test]
	async fn test_auth_failure_clears_prior_legacy_principal(#[case] cookie: Option<String>) {
		// Arrange
		let mw = AdminCookieAuthMiddleware::new(test_secret());
		let mut headers = HeaderMap::new();
		if let Some(cookie) = cookie {
			headers.insert("cookie", cookie.parse().unwrap());
		}
		let req = make_request(headers);
		seed_prior_principal(&req);

		// Act
		let resp = mw.process(req, Arc::new(AuthEntriesHandler)).await.unwrap();

		// Assert
		assert_eq!(
			auth_entries(&resp),
			expected_entries("", false, false, false)
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_valid_cookie_token_replaces_prior_legacy_principal() {
		// Arrange
		let token = generate_test_token("user-123");
		let mw = AdminCookieAuthMiddleware::new(test_secret());
		let mut headers = HeaderMap::new();
		headers.insert(
			"cookie",
			format!("{}={}", ADMIN_AUTH_COOKIE_NAME, token)
				.parse()
				.unwrap(),
		);
		let req = make_request(headers);
		seed_prior_principal(&req);

		// Act
		let resp = mw.process(req, Arc::new(AuthEntriesHandler)).await.unwrap();

		// Assert
		assert_eq!(
			auth_entries(&resp),
			expected_entries("user-123", true, true, true)
		);
	}
}
