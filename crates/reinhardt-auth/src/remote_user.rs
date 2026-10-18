//! Remote User Authentication
//!
//! Authentication backend that trusts HTTP headers set by upstream
//! authentication systems (e.g., Apache mod_auth, nginx auth_request).

use crate::core::AuthIdentity;
use crate::internal_user::InternalUser;
use crate::{AuthBackend, AuthenticationError};
use reinhardt_http::Request;
use uuid::Uuid;

/// Remote user authentication backend
///
/// Authenticates users based on a trusted HTTP header (typically REMOTE_USER)
/// set by an upstream authentication layer.
///
/// # Security Warning
///
/// This backend trusts the specified header completely. Only use this when
/// your application is behind a properly configured authentication proxy
/// that prevents clients from spoofing this header.
///
/// # Forced Logout
///
/// By default, a request whose header is missing, empty, or not visible ASCII
/// is rejected with [`AuthenticationError::NotAuthenticated`], which composite
/// backends treat as terminal. See [`RemoteUserAuthentication::force_logout`].
///
/// # Examples
///
/// ```no_run
/// use reinhardt_auth::{AuthBackend, AuthIdentity};
/// use bytes::Bytes;
/// use hyper::{HeaderMap, Method};
/// use reinhardt_http::Request;
///
/// # async fn example() {
/// // Create auth backend
/// let auth = reinhardt_auth::RemoteUserAuth::new();
///
/// // Create request with REMOTE_USER header
/// let mut headers = HeaderMap::new();
/// headers.insert("REMOTE_USER", "alice".parse().unwrap());
/// let request = Request::builder()
///     .method(Method::GET)
///     .uri("/")
///     .headers(headers)
///     .body(Bytes::new())
///     .build()
///     .unwrap();
///
/// let result = auth.authenticate(&request).await.unwrap();
/// assert!(result.is_some());
/// assert!(result.unwrap().is_authenticated());
/// # }
/// ```
pub struct RemoteUserAuthentication {
	/// Header name to check (default: "REMOTE_USER")
	header_name: String,
	/// Whether a request without an asserted identity is rejected terminally
	force_logout: bool,
}

impl RemoteUserAuthentication {
	/// Create a new remote user authentication backend
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::RemoteUserAuth;
	///
	/// let auth = RemoteUserAuth::new();
	/// ```
	pub fn new() -> Self {
		Self {
			header_name: "REMOTE_USER".to_string(),
			force_logout: true,
		}
	}

	/// Set custom header name
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::RemoteUserAuth;
	///
	/// let auth = RemoteUserAuth::new()
	///     .with_header("X-Auth-User");
	/// ```
	pub fn with_header(mut self, header: impl Into<String>) -> Self {
		self.header_name = header.into();
		self
	}

	/// Set whether to force logout when the header is missing
	///
	/// When enabled (the default), a request whose header is missing, empty,
	/// or not visible ASCII is rejected with
	/// [`AuthenticationError::NotAuthenticated`].
	/// [`CompositeAuthentication`](crate::CompositeAuthentication) and
	/// [`CompositeAuthBackend`](crate::CompositeAuthBackend) stop at that
	/// error, so a still-valid fallback credential (session cookie, token, ...)
	/// accepted by a later backend cannot keep the client signed in after the
	/// upstream proxy stops asserting the identity. Backends registered before
	/// this one are still tried first, so register it ahead of any fallback.
	///
	/// When disabled, such a request returns `Ok(None)` and composite backends
	/// fall back to the next backend.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::RemoteUserAuth;
	///
	/// // Let fallback backends authenticate requests without the header
	/// let auth = RemoteUserAuth::new().force_logout(false);
	/// ```
	pub fn force_logout(mut self, force: bool) -> Self {
		self.force_logout = force;
		self
	}
}

impl Default for RemoteUserAuthentication {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait::async_trait]
impl AuthBackend for RemoteUserAuthentication {
	async fn authenticate(
		&self,
		request: &Request,
	) -> Result<Option<Box<dyn AuthIdentity>>, AuthenticationError> {
		// Get header value
		let header_value = request
			.headers
			.get(&self.header_name)
			.and_then(|v| v.to_str().ok());

		match header_value {
			Some(username) if !username.is_empty() => {
				// Create identity from header
				Ok(Some(Box::new(InternalUser {
					id: Uuid::new_v5(&crate::USER_ID_NAMESPACE, username.as_bytes()),
					username: username.to_string(),
					email: String::new(),
					is_active: true,
					is_admin: false,
					is_staff: false,
					is_superuser: false,
				})))
			}
			_ if self.force_logout => {
				// The proxy no longer asserts an identity. Reject terminally so
				// composite backends cannot fall back to a still-valid secondary
				// credential and keep the client signed in.
				Err(AuthenticationError::NotAuthenticated)
			}
			_ => {
				// No usable header: let the next backend try
				Ok(None)
			}
		}
	}

	async fn get_user(
		&self,
		_user_id: &str,
	) -> Result<Option<Box<dyn AuthIdentity>>, AuthenticationError> {
		// For remote user auth, we can't retrieve users by ID
		// since we only have the username from the header
		Ok(None)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{CompositeAuthBackend, CompositeAuthentication, TokenAuthentication};
	use bytes::Bytes;
	use hyper::header::HeaderValue;
	use hyper::{HeaderMap, Method};
	use rstest::rstest;

	const FALLBACK_TOKEN: &str = "still-valid-fallback-token";

	fn build_request(headers: HeaderMap) -> Request {
		Request::builder()
			.method(Method::GET)
			.uri("/")
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap()
	}

	/// Headers carrying a still-valid fallback token plus the given
	/// `REMOTE_USER` value (`None` = header removed by the proxy).
	fn headers_with_fallback_token(remote_user: Option<HeaderValue>) -> HeaderMap {
		let mut headers = HeaderMap::new();
		headers.insert(
			"Authorization",
			HeaderValue::from_str(&format!("Token {FALLBACK_TOKEN}")).unwrap(),
		);
		if let Some(value) = remote_user {
			headers.insert("REMOTE_USER", value);
		}
		headers
	}

	fn token_fallback() -> TokenAuthentication {
		let mut token_auth = TokenAuthentication::new();
		token_auth.add_token(FALLBACK_TOKEN, "bob");
		token_auth
	}

	#[tokio::test]
	async fn test_remote_user_with_header() {
		let auth = RemoteUserAuthentication::new();
		let mut headers = HeaderMap::new();
		headers.insert("REMOTE_USER", "testuser".parse().unwrap());

		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap();

		let result = auth.authenticate(&request).await.unwrap();
		let user = result.expect("remote user authentication should succeed");
		let expected_id = Uuid::new_v5(&crate::USER_ID_NAMESPACE, b"testuser").to_string();
		assert_eq!(user.id(), expected_id);
		assert!(user.is_authenticated());
	}

	#[rstest]
	#[case::missing(None)]
	#[case::empty(Some(HeaderValue::from_static("")))]
	#[case::not_visible_ascii(Some(HeaderValue::from_bytes(b"caf\xc3\xa9").unwrap()))]
	#[tokio::test]
	async fn test_force_logout_rejects_request_without_identity(
		#[case] remote_user: Option<HeaderValue>,
	) {
		// Arrange
		let auth = RemoteUserAuthentication::new();
		let mut headers = HeaderMap::new();
		if let Some(value) = remote_user {
			headers.insert("REMOTE_USER", value);
		}
		let request = build_request(headers);

		// Act
		let result = auth.authenticate(&request).await;

		// Assert
		assert_eq!(result.err(), Some(AuthenticationError::NotAuthenticated));
	}

	#[rstest]
	#[case::missing(None)]
	#[case::empty(Some(HeaderValue::from_static("")))]
	#[case::not_visible_ascii(Some(HeaderValue::from_bytes(b"caf\xc3\xa9").unwrap()))]
	#[tokio::test]
	async fn test_disabled_force_logout_defers_request_without_identity(
		#[case] remote_user: Option<HeaderValue>,
	) {
		// Arrange
		let auth = RemoteUserAuthentication::new().force_logout(false);
		let mut headers = HeaderMap::new();
		if let Some(value) = remote_user {
			headers.insert("REMOTE_USER", value);
		}
		let request = build_request(headers);

		// Act
		let result = auth.authenticate(&request).await.unwrap();

		// Assert
		assert!(result.is_none());
	}

	#[rstest]
	#[case::missing(None)]
	#[case::empty(Some(HeaderValue::from_static("")))]
	#[case::not_visible_ascii(Some(HeaderValue::from_bytes(b"caf\xc3\xa9").unwrap()))]
	#[tokio::test]
	async fn test_force_logout_blocks_fallback_credential_in_composite_authentication(
		#[case] remote_user: Option<HeaderValue>,
	) {
		// Arrange
		let composite = CompositeAuthentication::new()
			.with_backend(RemoteUserAuthentication::new())
			.with_backend(token_fallback());
		let request = build_request(headers_with_fallback_token(remote_user));

		// Act
		let result = AuthBackend::authenticate(&composite, &request).await;

		// Assert
		assert_eq!(result.err(), Some(AuthenticationError::NotAuthenticated));
	}

	#[rstest]
	#[tokio::test]
	async fn test_force_logout_blocks_fallback_credential_in_composite_auth_backend() {
		// Arrange
		let mut composite = CompositeAuthBackend::new();
		composite.add_backend(Box::new(RemoteUserAuthentication::new()));
		composite.add_backend(Box::new(token_fallback()));
		let request = build_request(headers_with_fallback_token(None));

		// Act
		let result = composite.authenticate(&request).await;

		// Assert
		assert_eq!(result.err(), Some(AuthenticationError::NotAuthenticated));
	}

	#[rstest]
	#[tokio::test]
	async fn test_disabled_force_logout_allows_fallback_credential_in_composite() {
		// Arrange
		let composite = CompositeAuthentication::new()
			.with_backend(RemoteUserAuthentication::new().force_logout(false))
			.with_backend(token_fallback());
		let request = build_request(headers_with_fallback_token(None));

		// Act
		let user = AuthBackend::authenticate(&composite, &request)
			.await
			.unwrap()
			.expect("fallback token should authenticate when force_logout is disabled");

		// Assert
		let expected_id = Uuid::new_v5(&crate::USER_ID_NAMESPACE, b"bob").to_string();
		assert_eq!(user.id(), expected_id);
	}

	#[rstest]
	#[tokio::test]
	async fn test_asserted_identity_wins_over_fallback_credential_in_composite() {
		// Arrange
		let composite = CompositeAuthentication::new()
			.with_backend(RemoteUserAuthentication::new())
			.with_backend(token_fallback());
		let request = build_request(headers_with_fallback_token(Some(HeaderValue::from_static(
			"alice",
		))));

		// Act
		let user = AuthBackend::authenticate(&composite, &request)
			.await
			.unwrap()
			.expect("asserted remote user should authenticate");

		// Assert
		let expected_id = Uuid::new_v5(&crate::USER_ID_NAMESPACE, b"alice").to_string();
		assert_eq!(user.id(), expected_id);
	}

	#[tokio::test]
	async fn test_custom_header() {
		let auth = RemoteUserAuthentication::new().with_header("X-Auth-User");
		let mut headers = HeaderMap::new();
		headers.insert("X-Auth-User", "alice".parse().unwrap());

		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap();

		let result = auth.authenticate(&request).await.unwrap();
		let user = result.expect("custom header authentication should succeed");
		let expected_id = Uuid::new_v5(&crate::USER_ID_NAMESPACE, b"alice").to_string();
		assert_eq!(user.id(), expected_id);
		assert!(user.is_authenticated());
	}

	#[rstest]
	#[tokio::test]
	async fn test_same_username_produces_same_id() {
		// Arrange
		let auth = RemoteUserAuthentication::new();

		let mut headers1 = HeaderMap::new();
		headers1.insert("REMOTE_USER", "testuser".parse().unwrap());
		let request1 = Request::builder()
			.method(Method::GET)
			.uri("/")
			.headers(headers1)
			.body(Bytes::new())
			.build()
			.unwrap();

		let mut headers2 = HeaderMap::new();
		headers2.insert("REMOTE_USER", "testuser".parse().unwrap());
		let request2 = Request::builder()
			.method(Method::GET)
			.uri("/")
			.headers(headers2)
			.body(Bytes::new())
			.build()
			.unwrap();

		// Act
		let user1 = auth.authenticate(&request1).await.unwrap().unwrap();
		let user2 = auth.authenticate(&request2).await.unwrap().unwrap();

		// Assert
		assert_eq!(
			user1.id(),
			user2.id(),
			"same username must produce the same UUID"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_authenticated_user_has_default_privilege_flags() {
		// Arrange
		let auth = RemoteUserAuthentication::new();
		let mut headers = HeaderMap::new();
		headers.insert("REMOTE_USER", "testuser".parse().unwrap());
		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap();

		// Act
		let user = auth.authenticate(&request).await.unwrap().unwrap();

		// Assert
		assert!(user.is_authenticated());
		assert!(!user.is_admin());
	}

	#[rstest]
	#[tokio::test]
	async fn test_get_user_always_returns_none() {
		// Arrange
		let auth = RemoteUserAuthentication::new();

		// Act
		let result = auth.get_user("any_user_id").await.unwrap();

		// Assert
		assert!(result.is_none());
	}
}
