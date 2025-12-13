//! Authentication fixtures for tests.
//!
//! Provides JWT token generation helpers.

use crate::apps::auth::models::User;
use chrono::Duration;
use reinhardt::{Claims, JwtAuth};

/// Generate a test JWT token for a user.
///
/// Uses the same JWT configuration as the application.
/// Secret key is hardcoded for tests.
///
/// # Example
///
/// ```rust,no_run
/// let token = generate_test_token(&user);
/// // Use token in Authorization header: "Bearer {token}"
/// ```
pub fn generate_test_token(user: &User) -> String {
	let jwt_auth = JwtAuth::new(b"test-secret-key-for-testing-only");
	jwt_auth
		.generate_token(user.id.to_string(), user.username.clone())
		.expect("Failed to create test JWT token")
}

/// Generate an expired test JWT token.
///
/// Note: JwtAuth::generate_token uses a fixed 24-hour expiry.
/// For testing expired tokens, we use the encode method directly with custom claims.
///
/// # Example
///
/// ```rust,no_run
/// let expired_token = generate_expired_token(&user);
/// // This token will fail verification due to expiration
/// ```
pub fn generate_expired_token(user: &User) -> String {
	let jwt_auth = JwtAuth::new(b"test-secret-key-for-testing-only");

	// Create claims that are already expired (1 hour ago)
	let claims = Claims::new(
		user.id.to_string(),
		user.username.clone(),
		Duration::hours(-1), // Already expired
	);
	jwt_auth
		.encode(&claims)
		.expect("Failed to create expired JWT token")
}

/// Generate an invalid JWT token (signed with wrong key).
///
/// # Example
///
/// ```rust,no_run
/// let invalid_token = generate_invalid_token(&user);
/// // This token will fail verification due to invalid signature
/// ```
pub fn generate_invalid_token(user: &User) -> String {
	let jwt_auth = JwtAuth::new(b"wrong-secret-key");
	jwt_auth
		.generate_token(user.id.to_string(), user.username.clone())
		.expect("Failed to create invalid JWT token")
}
