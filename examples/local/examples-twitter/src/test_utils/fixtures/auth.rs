//! Authentication fixtures for tests.
//!
//! Provides JWT token generation helpers and session fixtures.

use crate::apps::auth::models::User;
use crate::test_utils::fixtures::{TestDatabase, test_user};
use chrono::Duration;
use reinhardt::middleware::session::{SessionData, SessionStore};
use reinhardt::{Claims, JwtAuth};
use rstest::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration as StdDuration, SystemTime};
use uuid::Uuid;

// ============================================================================
// Session Fixtures
// ============================================================================

/// Create a new session with default TTL (1 hour)
fn create_session_data() -> SessionData {
	let now = SystemTime::now();
	SessionData {
		id: Uuid::new_v4().to_string(),
		data: HashMap::new(),
		created_at: now,
		last_accessed: now,
		expires_at: now + StdDuration::from_secs(3600),
	}
}

/// Empty session fixture for unauthenticated tests
#[fixture]
pub fn session() -> SessionData {
	create_session_data()
}

/// Session store fixture
#[fixture]
pub fn session_store() -> Arc<SessionStore> {
	Arc::new(SessionStore::default())
}

/// Authenticated session fixture with user_id set
#[fixture]
pub async fn authenticated_session(#[future] test_user: (User, TestDatabase)) -> SessionData {
	let (user, _db) = test_user.await;
	let mut session = create_session_data();
	session
		.data
		.insert("user_id".to_string(), serde_json::json!(user.id()));
	session
}

// ============================================================================
// JWT Token Helpers
// ============================================================================

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
		.generate_token(user.id().to_string(), user.username().to_string())
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
		user.id().to_string(),
		user.username().to_string(),
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
		.generate_token(user.id().to_string(), user.username().to_string())
		.expect("Failed to create invalid JWT token")
}
