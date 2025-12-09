//! Reset password endpoint tests
//!
//! Tests for password reset token generation including:
//! - Success cases (existing user, non-existent user for security)
//! - Validation errors (invalid email format, empty email)

#[cfg(test)]
mod reset_password_tests {
	use reinhardt::core::serde::json::json;
	use reinhardt::StatusCode;
	use rstest::rstest;

	use crate::test_utils::fixtures::*;

	/// Test 1: Success - Request reset for existing user
	///
	/// POST /accounts/auth/reset-password/
	/// Expected: 200 OK with reset token
	#[rstest]
	#[tokio::test]
	async fn test_success_reset_password_request(#[future] test_context: TestContext) {
		let context = test_context.await;

		// Create test user with known email
		let user = create_test_user(
			&context.db,
			TestUserParams::default()
				.with_email("reset_test@example.com")
				.with_username("reset_user"),
		)
		.await;

		// Build request body
		let body = json!({
			"email": user.email
		});

		// Get URL from router using reverse
		let url = context
			.router
			.reverse("auth:reset_password", &[])
			.expect("auth:reset_password route should exist");

		// Send request via APIClient
		let response = context
			.client
			.post(&url, &body, "json")
			.await
			.expect("Request should succeed");

		// Assert response status
		assert_eq!(
			response.status(),
			StatusCode::OK,
			"Status should be 200 OK"
		);

		// Parse response to verify reset token format
		let response_text = String::from_utf8(response.body().to_vec())
			.expect("Response should be valid UTF-8");
		let response_json: serde_json::Value = serde_json::from_str(&response_text)
			.expect("Response should be valid JSON");

		// Verify reset_token field exists and is not empty
		let reset_token = response_json
			.get("reset_token")
			.expect("Response should contain reset_token field")
			.as_str()
			.expect("reset_token should be a string");

		assert!(
			!reset_token.is_empty(),
			"Reset token should not be empty"
		);
	}

	/// Test 2: Success - Request reset for non-existent user (security)
	///
	/// For security reasons, the endpoint should return success
	/// even if the email doesn't exist (prevents email enumeration)
	#[rstest]
	#[tokio::test]
	async fn test_success_reset_for_nonexistent_user(#[future] test_context: TestContext) {
		let context = test_context.await;

		// Build request body with email that doesn't exist
		let body = json!({
			"email": "nonexistent@example.com"
		});

		// Get URL from router
		let url = context
			.router
			.reverse("auth:reset_password", &[])
			.expect("auth:reset_password route should exist");

		// Send request
		let response = context
			.client
			.post(&url, &body, "json")
			.await
			.expect("Request should complete");

		// Assert success (not 404!) for security
		assert_eq!(
			response.status(),
			StatusCode::OK,
			"Should return 200 OK for security (prevents email enumeration)"
		);

		// Parse response to verify reset token format
		let response_text = String::from_utf8(response.body().to_vec())
			.expect("Response should be valid UTF-8");
		let response_json: serde_json::Value = serde_json::from_str(&response_text)
			.expect("Response should be valid JSON");

		// Verify reset_token field exists and is not empty
		let reset_token = response_json
			.get("reset_token")
			.expect("Response should contain reset_token field")
			.as_str()
			.expect("reset_token should be a string");

		assert!(
			!reset_token.is_empty(),
			"Reset token should not be empty even for non-existent user"
		);
	}

	/// Test 3: Failure - Request reset with invalid email format
	#[rstest]
	#[tokio::test]
	async fn test_failure_reset_with_invalid_email(#[future] test_context: TestContext) {
		let context = test_context.await;

		// Build request body with invalid email format
		let body = json!({
			"email": "notanemail"
		});

		// Get URL from router
		let url = context.router.reverse("auth:reset_password", &[]).unwrap();

		// Send request
		let response = context.client.post(&url, &body, "json").await.unwrap();

		// Assert validation error
		assert!(
			response.status() == StatusCode::BAD_REQUEST
				|| response.status() == StatusCode::UNPROCESSABLE_ENTITY,
			"Should return validation error, got: {}",
			response.status()
		);
	}

	/// Test 4: Failure - Request reset with empty email
	#[rstest]
	#[tokio::test]
	async fn test_failure_reset_with_empty_email(#[future] test_context: TestContext) {
		let context = test_context.await;

		// Build request body with empty email
		let body = json!({
			"email": ""
		});

		// Get URL from router
		let url = context.router.reverse("auth:reset_password", &[]).unwrap();

		// Send request
		let response = context.client.post(&url, &body, "json").await.unwrap();

		// Assert validation error
		assert!(
			response.status() == StatusCode::BAD_REQUEST
				|| response.status() == StatusCode::UNPROCESSABLE_ENTITY,
			"Should return validation error"
		);
	}
}
