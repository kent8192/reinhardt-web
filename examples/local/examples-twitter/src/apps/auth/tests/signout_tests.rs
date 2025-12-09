//! Signout endpoint tests
//!
//! Tests for user signout functionality including:
//! - Success cases (valid token)
//! - Authentication errors (no auth, invalid token, expired token)

#[cfg(test)]
mod signout_tests {
	use reinhardt::StatusCode;
	use rstest::rstest;

	use crate::test_utils::fixtures::*;

	/// Test 1: Success - Signout with valid session
	///
	/// POST /accounts/auth/signout/
	/// Expected: 204 No Content
	#[rstest]
	#[tokio::test]
	async fn test_success_signout(#[future] authenticated_context: (TestContext, User)) {
		let (context, _user) = authenticated_context.await;

		// Get URL from router using reverse
		let url = context
			.router
			.reverse("auth:signout", &[])
			.expect("auth:signout route should exist");

		// Send request via APIClient (Authorization header already set)
		let response = context
			.client
			.post(&url, &serde_json::json!({}), "json")
			.await
			.expect("Request should succeed");

		// Assert success
		assert_eq!(
			response.status(),
			StatusCode::NO_CONTENT,
			"Status should be 204 No Content"
		);
	}

	/// Test 2: Failure - Signout without authentication
	#[rstest]
	#[tokio::test]
	async fn test_failure_signout_without_auth(#[future] test_context: TestContext) {
		let context = test_context.await;

		// Get URL from router
		let url = context
			.router
			.reverse("auth:signout", &[])
			.expect("auth:signout route should exist");

		// Send request without Authorization header
		let response = context
			.client
			.post(&url, &serde_json::json!({}), "json")
			.await
			.expect("Request should complete");

		// Assert authentication error (401 Unauthorized)
		assert_eq!(
			response.status(),
			StatusCode::UNAUTHORIZED,
			"Should return 401 Unauthorized for missing auth"
		);
	}

	/// Test 3: Failure - Signout with invalid token
	#[rstest]
	#[tokio::test]
	async fn test_failure_signout_with_invalid_token(#[future] test_context: TestContext) {
		let context = test_context.await;

		// Create test user
		let user = create_test_user(
			&context.db,
			TestUserParams::default()
				.with_email("invalid_token_test@example.com")
				.with_username("invalid_token_user"),
		)
		.await;

		// Generate token signed with wrong key
		let token = generate_invalid_token(&user);

		// Get URL from router
		let url = context
			.router
			.reverse("auth:signout", &[])
			.expect("auth:signout route should exist");

		// Send request with invalid token
		let response = context
			.client
			.post_with_auth(&url, &serde_json::json!({}), "json", &token)
			.await
			.expect("Request should complete");

		// Assert authentication error (401 Unauthorized)
		assert_eq!(
			response.status(),
			StatusCode::UNAUTHORIZED,
			"Should return 401 Unauthorized for invalid token"
		);
	}

	/// Test 4: Failure - Signout with expired token
	#[rstest]
	#[tokio::test]
	async fn test_failure_signout_with_expired_token(#[future] test_context: TestContext) {
		let context = test_context.await;

		// Create test user
		let user = create_test_user(
			&context.db,
			TestUserParams::default()
				.with_email("expired_token_test@example.com")
				.with_username("expired_token_user"),
		)
		.await;

		// Generate expired token
		let token = generate_expired_token(&user);

		// Get URL from router
		let url = context
			.router
			.reverse("auth:signout", &[])
			.expect("auth:signout route should exist");

		// Send request with expired token
		let response = context
			.client
			.post_with_auth(&url, &serde_json::json!({}), "json", &token)
			.await
			.expect("Request should complete");

		// Assert authentication error (401 Unauthorized)
		assert_eq!(
			response.status(),
			StatusCode::UNAUTHORIZED,
			"Should return 401 Unauthorized for expired token"
		);
	}
}
