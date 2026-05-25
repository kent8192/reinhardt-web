//! Authentication integration test fixtures
//!
//! This module provides rstest fixtures for authentication integration tests,
//! including pre-configured session backends, mock users, and test data.

use rstest::*;

// Re-exports for convenience
pub use reinhardt_auth::mfa::MFAAuthentication as MfaManager;
pub use reinhardt_auth::{
	Argon2Hasher, InMemoryTokenStorage, JwtAuth, PasswordHasher, StoredToken, TokenStorage,
};

// =============================================================================
// MFA Fixtures
// =============================================================================

/// MFA authentication fixture
///
/// Provides a pre-configured MFA authentication backend for testing.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::auth::mfa_authentication;
/// use rstest::*;
///
/// #[rstest]
/// fn test_with_mfa(mfa_authentication: MfaManager) {
///     mfa_authentication.register_user("alice", "JBSWY3DPEHPK3PXP");
/// }
/// ```
#[fixture]
pub fn mfa_authentication() -> MfaManager {
	MfaManager::new("ReinhardtTest").time_window(30)
}

// =============================================================================
// JWT Fixtures
// =============================================================================

/// JWT authentication fixture with default secret
///
/// Provides a pre-configured JWT authentication backend for testing.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::auth::jwt_auth;
/// use rstest::*;
///
/// #[rstest]
/// fn test_with_jwt(jwt_auth: JwtAuth) {
///     let token = jwt_auth.generate_token("user123".to_string(), "alice".to_string(), false, false).unwrap();
///     assert!(!token.is_empty());
/// }
/// ```
#[fixture]
pub fn jwt_auth() -> JwtAuth {
	// Use a secure test secret (32 bytes minimum)
	JwtAuth::new(b"reinhardt-test-secret-key-32bytes")
}

/// JWT authentication fixture with custom secret
///
/// Provides a JWT authentication backend with a specified secret.
#[fixture]
pub fn jwt_auth_with_secret(
	#[default(b"custom-secret-key-for-testing-32")] secret: &[u8],
) -> JwtAuth {
	JwtAuth::new(secret)
}

// =============================================================================
// Password Hasher Fixtures
// =============================================================================

/// Argon2 password hasher fixture
///
/// Provides a pre-configured Argon2 password hasher for testing.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::auth::argon2_hasher;
/// use rstest::*;
///
/// #[rstest]
/// fn test_with_hasher(argon2_hasher: Argon2Hasher) {
///     let hash = argon2_hasher.hash("password123").unwrap();
///     assert!(argon2_hasher.verify("password123", &hash).unwrap());
/// }
/// ```
#[fixture]
pub fn argon2_hasher() -> Argon2Hasher {
	Argon2Hasher
}

// =============================================================================
// Token Storage Fixtures
// =============================================================================

/// In-memory token storage fixture
///
/// Provides a pre-configured in-memory token storage for testing.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::auth::in_memory_token_storage;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_with_storage(in_memory_token_storage: InMemoryTokenStorage) {
///     assert!(in_memory_token_storage.is_empty());
/// }
/// ```
#[fixture]
pub fn in_memory_token_storage() -> InMemoryTokenStorage {
	InMemoryTokenStorage::new()
}

/// Token storage with test data fixture
///
/// Provides an in-memory token storage pre-populated with test tokens.
#[fixture]
pub fn token_storage_with_data() -> InMemoryTokenStorage {
	InMemoryTokenStorage::new()
}
