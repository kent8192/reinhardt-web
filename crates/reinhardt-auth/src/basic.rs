//! HTTP Basic Authentication
//!
//! Passwords are hashed with Argon2id by default on storage and verified using
//! constant-time comparison provided by the configured hasher.
//!
//! Implements [`AuthBackend`] returning [`AuthIdentity`] trait objects.

use crate::core::AuthIdentity;
use crate::core::hasher::{PasswordHashPolicy, PasswordHasher, PasswordVerification};
use crate::internal_user::InternalUser;
use crate::rest_authentication::RestAuthentication;
use crate::{AuthBackend, AuthenticationError};
use base64::{Engine, engine::general_purpose::STANDARD};
use reinhardt_http::Request;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

/// Argon2-based password hasher used internally by `BasicAuthentication`.
///
/// This is intentionally a thin wrapper so the module stays self-contained
/// without requiring the `argon2-hasher` feature flag.
struct InternalArgon2Hasher;

impl PasswordHasher for InternalArgon2Hasher {
	fn hash(&self, password: &str) -> Result<String, reinhardt_core::exception::Error> {
		use argon2::Argon2;
		use password_hash::{PasswordHasher as _, SaltString, rand_core::OsRng};

		let salt = SaltString::generate(&mut OsRng);
		let argon2 = Argon2::default();

		argon2
			.hash_password(password.as_bytes(), &salt)
			.map(|hash| hash.to_string())
			.map_err(|e| reinhardt_core::exception::Error::Authentication(e.to_string()))
	}

	fn verify(&self, password: &str, hash: &str) -> Result<bool, reinhardt_core::exception::Error> {
		use argon2::Argon2;
		use password_hash::{PasswordHash, PasswordVerifier};

		let parsed_hash = PasswordHash::new(hash)
			.map_err(|e| reinhardt_core::exception::Error::Authentication(e.to_string()))?;

		// Argon2 verify_password uses constant-time comparison internally
		Ok(Argon2::default()
			.verify_password(password.as_bytes(), &parsed_hash)
			.is_ok())
	}

	fn algorithm(&self) -> Option<&'static str> {
		Some("argon2id")
	}

	fn identify(&self, hash: &str) -> bool {
		use password_hash::PasswordHash;

		PasswordHash::new(hash)
			.map(|parsed| parsed.algorithm.as_str() == "argon2id")
			.unwrap_or(false)
	}

	fn must_update(&self, hash: &str) -> Result<bool, reinhardt_core::exception::Error> {
		use argon2::{Argon2, Params, Version};
		use password_hash::PasswordHash;

		let parsed_hash = PasswordHash::new(hash)
			.map_err(|e| reinhardt_core::exception::Error::Authentication(e.to_string()))?;
		let stored_params = Params::try_from(&parsed_hash)
			.map_err(|e| reinhardt_core::exception::Error::Authentication(e.to_string()))?;
		let current = Argon2::default();
		let current_params = current.params();
		let stored_output_len = stored_params
			.output_len()
			.unwrap_or(Params::DEFAULT_OUTPUT_LEN);
		let current_output_len = current_params
			.output_len()
			.unwrap_or(Params::DEFAULT_OUTPUT_LEN);

		Ok(parsed_hash.version != Some(u32::from(Version::default()))
			|| stored_output_len != current_output_len
			|| stored_params.m_cost() != current_params.m_cost()
			|| stored_params.t_cost() != current_params.t_cost()
			|| stored_params.p_cost() != current_params.p_cost())
	}
}

/// Basic Authentication backend
///
/// Passwords are hashed with the configured policy before storage.
/// The default policy uses Argon2id.
pub struct BasicAuthentication {
	/// username -> password hash
	users: RwLock<HashMap<String, String>>,
	policy: PasswordHashPolicy,
}

impl BasicAuthentication {
	/// Creates a new BasicAuthentication backend with no users.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::{HttpBasicAuth, AuthBackend};
	/// use bytes::Bytes;
	/// use hyper::{HeaderMap, Method, Uri, Version};
	/// use reinhardt_http::Request;
	///
	/// # async fn example() {
	/// let auth = HttpBasicAuth::new();
	///
	/// // Create a request without authentication header
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/")
	///     .body(Bytes::new())
	///     .build()
	///     .unwrap();
	///
	/// // Since no users are registered, authentication should return None
	/// let result = auth.authenticate(&request).await.unwrap();
	/// assert!(result.is_none());
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub fn new() -> Self {
		Self {
			users: RwLock::new(HashMap::new()),
			policy: PasswordHashPolicy::new(InternalArgon2Hasher),
		}
	}

	/// Creates a new BasicAuthentication backend with a custom password policy.
	pub fn with_policy(policy: PasswordHashPolicy) -> Self {
		Self {
			users: RwLock::new(HashMap::new()),
			policy,
		}
	}

	/// Adds a user with the given username and password.
	///
	/// The password is hashed with the configured policy before storage.
	///
	/// # Panics
	///
	/// Panics if password hashing fails (should not happen in practice).
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::{HttpBasicAuth, AuthBackend};
	/// use bytes::Bytes;
	/// use hyper::{HeaderMap, Method, Uri, Version};
	/// use reinhardt_http::Request;
	///
	/// # async fn example() {
	/// let mut auth = HttpBasicAuth::new();
	/// auth.add_user("alice", "secret123");
	/// auth.add_user("bob", "password456");
	///
	/// // Create a request with valid Basic auth credentials
	/// // "alice:secret123" in base64 is "YWxpY2U6c2VjcmV0MTIz"
	/// let mut headers = HeaderMap::new();
	/// headers.insert("Authorization", "Basic YWxpY2U6c2VjcmV0MTIz".parse().unwrap());
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/")
	///     .headers(headers)
	///     .body(Bytes::new())
	///     .build()
	///     .unwrap();
	///
	/// // Authentication should succeed
	/// let result = auth.authenticate(&request).await.unwrap();
	/// assert!(result.is_some());
	/// assert!(result.unwrap().is_authenticated());
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub fn add_user(&self, username: impl Into<String>, password: impl Into<String>) {
		let hash = self
			.policy
			.hash(&password.into())
			.expect("password hashing should not fail");
		self.users
			.write()
			.expect("basic auth users lock should not be poisoned")
			.insert(username.into(), hash);
	}

	/// Parse Authorization header
	fn parse_auth_header(&self, header: &str) -> Option<(String, String)> {
		if !header.starts_with("Basic ") {
			return None;
		}

		let encoded = header.strip_prefix("Basic ")?;
		let decoded = STANDARD.decode(encoded).ok()?;
		let decoded_str = String::from_utf8(decoded).ok()?;

		let parts: Vec<&str> = decoded_str.splitn(2, ':').collect();
		if parts.len() != 2 {
			return None;
		}

		Some((parts[0].to_string(), parts[1].to_string()))
	}

	fn internal_user(username: &str) -> InternalUser {
		InternalUser {
			id: Uuid::new_v5(&crate::USER_ID_NAMESPACE, username.as_bytes()),
			username: username.to_string(),
			email: String::new(),
			is_active: true,
			is_admin: false,
			is_staff: false,
			is_superuser: false,
		}
	}

	fn replace_hash_if_current(
		&self,
		username: &str,
		expected_hash: &str,
		updated_hash: String,
	) -> bool {
		let mut users = self
			.users
			.write()
			.expect("basic auth users lock should not be poisoned");
		match users.get_mut(username) {
			Some(current_hash) if current_hash == expected_hash => {
				*current_hash = updated_hash;
				true
			}
			_ => false,
		}
	}

	fn current_hash_matches_password(&self, username: &str, password: &str) -> bool {
		let current_hash = self
			.users
			.read()
			.expect("basic auth users lock should not be poisoned")
			.get(username)
			.cloned();

		let Some(current_hash) = current_hash else {
			return false;
		};

		!matches!(
			self.policy
				.verify_with_update(password, &current_hash)
				.unwrap_or(PasswordVerification::Invalid),
			PasswordVerification::Invalid
		)
	}

	fn replace_hash_if_current_or_current_matches(
		&self,
		username: &str,
		password: &str,
		expected_hash: &str,
		updated_hash: String,
	) -> bool {
		self.replace_hash_if_current(username, expected_hash, updated_hash)
			|| self.current_hash_matches_password(username, password)
	}
}

impl Default for BasicAuthentication {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait::async_trait]
impl AuthBackend for BasicAuthentication {
	async fn authenticate(
		&self,
		request: &Request,
	) -> Result<Option<Box<dyn AuthIdentity>>, AuthenticationError> {
		let auth_header = request
			.headers
			.get("Authorization")
			.and_then(|h| h.to_str().ok());

		if let Some(header) = auth_header
			&& let Some((username, password)) = self.parse_auth_header(header)
		{
			let stored_hash = self
				.users
				.read()
				.expect("basic auth users lock should not be poisoned")
				.get(&username)
				.cloned();

			if let Some(stored_hash) = stored_hash {
				let verification = self
					.policy
					.verify_with_update(&password, &stored_hash)
					.unwrap_or(PasswordVerification::Invalid);

				match verification {
					PasswordVerification::Valid => {
						return Ok(Some(Box::new(Self::internal_user(&username))));
					}
					PasswordVerification::ValidNeedsRehash { updated_hash } => {
						if self.replace_hash_if_current_or_current_matches(
							&username,
							&password,
							&stored_hash,
							updated_hash,
						) {
							return Ok(Some(Box::new(Self::internal_user(&username))));
						}
					}
					PasswordVerification::Invalid => {}
				}
			}
			return Err(AuthenticationError::InvalidCredentials);
		}

		Ok(None)
	}

	async fn get_user(
		&self,
		user_id: &str,
	) -> Result<Option<Box<dyn AuthIdentity>>, AuthenticationError> {
		if self
			.users
			.read()
			.expect("basic auth users lock should not be poisoned")
			.contains_key(user_id)
		{
			Ok(Some(Box::new(Self::internal_user(user_id))))
		} else {
			Ok(None)
		}
	}
}

// Implement REST API Authentication trait by forwarding to AuthBackend
#[async_trait::async_trait]
impl RestAuthentication for BasicAuthentication {
	async fn authenticate(
		&self,
		request: &Request,
	) -> Result<Option<Box<dyn AuthIdentity>>, AuthenticationError> {
		// Forward to AuthBackend implementation
		AuthBackend::authenticate(self, request).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method};
	use rstest::rstest;

	fn create_request_with_auth(auth: &str) -> Request {
		let mut headers = HeaderMap::new();
		headers.insert("Authorization", auth.parse().unwrap());
		Request::builder()
			.method(Method::GET)
			.uri("/")
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap()
	}

	#[rstest]
	#[tokio::test]
	async fn test_basic_auth_success() {
		// Arrange
		let backend = BasicAuthentication::new();
		backend.add_user("testuser", "testpass");

		// Base64 encode "testuser:testpass"
		let auth = "Basic dGVzdHVzZXI6dGVzdHBhc3M=";
		let request = create_request_with_auth(auth);

		// Act
		let result = AuthBackend::authenticate(&backend, &request).await.unwrap();

		// Assert
		let user = result.expect("authentication should succeed");
		let expected_id = Uuid::new_v5(&crate::USER_ID_NAMESPACE, b"testuser").to_string();
		assert_eq!(user.id(), expected_id);
		assert!(user.is_authenticated());
	}

	#[rstest]
	#[tokio::test]
	async fn test_basic_auth_invalid_password() {
		// Arrange
		let backend = BasicAuthentication::new();
		backend.add_user("testuser", "correctpass");

		// Base64 encode "testuser:wrongpass"
		let auth = "Basic dGVzdHVzZXI6d3JvbmdwYXNz";
		let request = create_request_with_auth(auth);

		// Act
		let result = AuthBackend::authenticate(&backend, &request).await;

		// Assert
		assert!(result.is_err());
	}

	#[rstest]
	#[tokio::test]
	async fn test_basic_auth_no_header() {
		// Arrange
		let backend = BasicAuthentication::new();
		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.body(Bytes::new())
			.build()
			.unwrap();

		// Act
		let result = AuthBackend::authenticate(&backend, &request).await.unwrap();

		// Assert
		assert!(result.is_none());
	}

	#[rstest]
	fn test_parse_auth_header() {
		// Arrange
		let backend = BasicAuthentication::new();

		// Act
		let (user, pass) = backend.parse_auth_header("Basic dGVzdDpwYXNz").unwrap();

		// Assert
		assert_eq!(user, "test");
		assert_eq!(pass, "pass");
	}

	#[rstest]
	#[tokio::test]
	async fn test_get_user() {
		// Arrange
		let backend = BasicAuthentication::new();
		backend.add_user("testuser", "testpass");

		// Act
		let user = backend.get_user("testuser").await.unwrap();
		let no_user = backend.get_user("nonexistent").await.unwrap();

		// Assert
		let user = user.expect("registered user should be found");
		let expected_id = Uuid::new_v5(&crate::USER_ID_NAMESPACE, b"testuser").to_string();
		assert_eq!(user.id(), expected_id);
		assert!(user.is_authenticated());
		assert!(no_user.is_none());
	}

	#[rstest]
	fn test_password_is_hashed_on_storage() {
		// Arrange
		let backend = BasicAuthentication::new();

		// Act
		backend.add_user("testuser", "plaintext_password");

		// Assert
		let users = backend
			.users
			.read()
			.expect("basic auth users lock should not be poisoned");
		let stored = users.get("testuser").unwrap();
		// Argon2 hashes start with "$argon2"
		assert!(
			stored.starts_with("$argon2"),
			"Password should be stored as Argon2 hash, got: {}",
			stored
		);
		assert_ne!(stored, "plaintext_password");
	}

	#[cfg(all(feature = "argon2-hasher", feature = "bcrypt-hasher"))]
	#[test]
	fn test_basic_auth_rehashes_legacy_argon2_to_bcrypt_policy() {
		use crate::{Argon2Hasher, BcryptHasher, PasswordHashPolicy};

		// Arrange
		let backend = BasicAuthentication::with_policy(
			PasswordHashPolicy::new(BcryptHasher::default()).with_legacy(Argon2Hasher::new()),
		);
		let legacy_hash = Argon2Hasher::new().hash("secret").unwrap();
		backend
			.users
			.write()
			.expect("basic auth users lock should not be poisoned")
			.insert("alice".to_string(), legacy_hash);

		let request = create_request_with_auth("Basic YWxpY2U6c2VjcmV0");

		// Act
		let result = tokio::runtime::Runtime::new()
			.unwrap()
			.block_on(async { AuthBackend::authenticate(&backend, &request).await.unwrap() });

		// Assert
		assert!(result.is_some());
		let users = backend
			.users
			.read()
			.expect("basic auth users lock should not be poisoned");
		let stored = users.get("alice").unwrap();
		assert!(BcryptHasher::default().identify(stored));
	}

	#[rstest]
	fn test_replace_hash_if_current_rejects_stale_rehash() {
		// Arrange
		let backend = BasicAuthentication::new();
		backend
			.users
			.write()
			.expect("basic auth users lock should not be poisoned")
			.insert("alice".to_string(), "old$secret".to_string());
		backend
			.users
			.write()
			.expect("basic auth users lock should not be poisoned")
			.insert("alice".to_string(), "new$secret".to_string());

		// Act
		let replaced =
			backend.replace_hash_if_current("alice", "old$secret", "updated$secret".to_string());

		// Assert
		assert!(!replaced);
		let users = backend
			.users
			.read()
			.expect("basic auth users lock should not be poisoned");
		assert_eq!(users.get("alice").map(String::as_str), Some("new$secret"));
	}

	#[cfg(all(feature = "argon2-hasher", feature = "bcrypt-hasher"))]
	#[test]
	fn test_rehash_race_accepts_already_updated_current_password() {
		use crate::{Argon2Hasher, BcryptHasher, PasswordHashPolicy};

		// Arrange
		let bcrypt = BcryptHasher::with_cost(4);
		let backend = BasicAuthentication::with_policy(
			PasswordHashPolicy::new(bcrypt.clone()).with_legacy(Argon2Hasher::new()),
		);
		let current_hash = bcrypt.hash("secret").unwrap();
		let stale_hash = Argon2Hasher::new().hash("secret").unwrap();
		let stale_update = bcrypt.hash("secret").unwrap();
		backend
			.users
			.write()
			.expect("basic auth users lock should not be poisoned")
			.insert("alice".to_string(), current_hash.clone());

		// Act
		let accepted = backend.replace_hash_if_current_or_current_matches(
			"alice",
			"secret",
			&stale_hash,
			stale_update,
		);

		// Assert
		assert!(accepted);
		let users = backend
			.users
			.read()
			.expect("basic auth users lock should not be poisoned");
		assert_eq!(users.get("alice"), Some(&current_hash));
	}

	#[cfg(all(feature = "argon2-hasher", feature = "bcrypt-hasher"))]
	#[test]
	fn test_rehash_race_rejects_changed_current_password() {
		use crate::{Argon2Hasher, BcryptHasher, PasswordHashPolicy};

		// Arrange
		let bcrypt = BcryptHasher::with_cost(4);
		let backend = BasicAuthentication::with_policy(
			PasswordHashPolicy::new(bcrypt.clone()).with_legacy(Argon2Hasher::new()),
		);
		let current_hash = bcrypt.hash("changed").unwrap();
		let stale_hash = Argon2Hasher::new().hash("secret").unwrap();
		let stale_update = bcrypt.hash("secret").unwrap();
		backend
			.users
			.write()
			.expect("basic auth users lock should not be poisoned")
			.insert("alice".to_string(), current_hash.clone());

		// Act
		let accepted = backend.replace_hash_if_current_or_current_matches(
			"alice",
			"secret",
			&stale_hash,
			stale_update,
		);

		// Assert
		assert!(!accepted);
		let users = backend
			.users
			.read()
			.expect("basic auth users lock should not be poisoned");
		assert_eq!(users.get("alice"), Some(&current_hash));
	}

	#[rstest]
	#[tokio::test]
	async fn test_authenticate_same_username_produces_same_id() {
		// Arrange
		let backend = BasicAuthentication::new();
		backend.add_user("testuser", "testpass");

		let auth = "Basic dGVzdHVzZXI6dGVzdHBhc3M=";
		let request1 = create_request_with_auth(auth);
		let request2 = create_request_with_auth(auth);

		// Act
		let user1 = AuthBackend::authenticate(&backend, &request1)
			.await
			.unwrap()
			.unwrap();
		let user2 = AuthBackend::authenticate(&backend, &request2)
			.await
			.unwrap()
			.unwrap();

		// Assert
		assert_eq!(
			user1.id(),
			user2.id(),
			"same username must produce the same UUID"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_authenticated_user_id_is_deterministic_uuidv5() {
		// Arrange
		let backend = BasicAuthentication::new();
		backend.add_user("testuser", "testpass");

		let auth = "Basic dGVzdHVzZXI6dGVzdHBhc3M=";
		let request = create_request_with_auth(auth);

		// Act
		let user = AuthBackend::authenticate(&backend, &request)
			.await
			.unwrap()
			.unwrap();
		let id = Uuid::parse_str(&user.id()).unwrap();

		// Assert
		assert_eq!(id.get_version_num(), 5, "user ID must be UUIDv5");
		assert_eq!(
			id.get_variant(),
			uuid::Variant::RFC4122,
			"user ID must use RFC 4122 variant"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_authenticated_user_has_default_privilege_flags() {
		// Arrange
		let backend = BasicAuthentication::new();
		backend.add_user("testuser", "testpass");

		let auth = "Basic dGVzdHVzZXI6dGVzdHBhc3M=";
		let request = create_request_with_auth(auth);

		// Act
		let user = AuthBackend::authenticate(&backend, &request)
			.await
			.unwrap()
			.unwrap();

		// Assert
		assert!(user.is_authenticated());
		assert!(!user.is_admin());
	}

	#[rstest]
	#[tokio::test]
	async fn test_get_user_same_username_produces_same_id() {
		// Arrange
		let backend = BasicAuthentication::new();
		backend.add_user("testuser", "testpass");

		// Act
		let user1 = backend.get_user("testuser").await.unwrap().unwrap();
		let user2 = backend.get_user("testuser").await.unwrap().unwrap();

		// Assert
		assert_eq!(
			user1.id(),
			user2.id(),
			"same username must produce the same UUID"
		);
	}

	#[rstest]
	fn test_argon2_verification_works() {
		// Arrange
		let hasher = InternalArgon2Hasher;
		let password = "test_password_123";

		// Act
		let hash = hasher.hash(password).unwrap();
		let valid = hasher.verify(password, &hash).unwrap();
		let invalid = hasher.verify("wrong_password", &hash).unwrap();

		// Assert
		assert!(valid);
		assert!(!invalid);
	}
}
