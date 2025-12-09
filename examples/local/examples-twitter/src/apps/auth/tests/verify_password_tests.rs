//! Verify password endpoint tests
//!
//! Tests for password verification functionality including:
//! - Success cases (correct password, wrong password)
//! - Authentication errors (no auth)
//! - Validation errors (empty password)

#[cfg(test)]
mod verify_password_tests {
	use reinhardt::core::serde::json::json;
	use reinhardt::StatusCode;
	use rstest::rstest;

	use crate::test_utils::fixtures::*;

	/// Test 1: Success - Verify correct password
	///
	/// POST /accounts/auth/verify-password/
	/// Expected: 200 OK with { "valid": true }
	#[rstest]
	#[tokio::test]
	async fn test_success_verify_password(#[future] authenticated_context: (TestContext, crate::apps::auth::models::User)) {
		let (context, user) = authenticated_context.await;

		// Build request body with the correct password
		let body = json!({
			"password": "testpassword123"  // Default password from create_test_user
		});

		// Get URL from router using reverse
		let url = context
			.router
			.reverse("auth:verify_password", &[])
			.expect("auth:verify_password route should exist");

		// Send authenticated request (Authorization header already set by fixture)
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

		// Assert response body
		let response_body: serde_json::Value = serde_json::from_slice(response.body())
			.expect("Response should be valid JSON");

		assert_eq!(
			response_body.get("valid").and_then(|v| v.as_bool()),
			Some(true),
			"Password should be valid"
		);
	}

	/// Test 2: Success - Verify wrong password returns false
	///
	/// POST /accounts/auth/verify-password/
	/// Expected: 200 OK with { "valid": false }
	#[rstest]
	#[tokio::test]
	async fn test_success_verify_wrong_password(#[future] authenticated_context: (TestContext, crate::apps::auth::models::User)) {
		let (context, _user) = authenticated_context.await;

		// Build request body with wrong password
		let body = json!({
			"password": "wrongpassword123"
		});

		// Get URL from router
		let url = context
			.router
			.reverse("auth:verify_password", &[])
			.expect("auth:verify_password route should exist");

		// Send authenticated request
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

		// Assert response body
		let response_body: serde_json::Value = serde_json::from_slice(response.body())
			.expect("Response should be valid JSON");

		assert_eq!(
			response_body.get("valid").and_then(|v| v.as_bool()),
			Some(false),
			"Password should be invalid"
		);
	}

	/// Test 3: Failure - Verify without authentication
	///
	/// POST /accounts/auth/verify-password/
	/// Expected: 401 Unauthorized
	#[rstest]
	#[tokio::test]
	async fn test_failure_verify_without_auth(#[future] test_context: TestContext) {
		let context = test_context.await;

		// Build request body
		let body = json!({
			"password": "anypassword123"
		});

		// Get URL from router
		let url = context
			.router
			.reverse("auth:verify_password", &[])
			.expect("auth:verify_password route should exist");

		// Send request without authentication
		let response = context
			.client
			.post(&url, &body, "json")
			.await
			.expect("Request should complete");

		// Assert authentication error
		assert_eq!(
			response.status(),
			StatusCode::UNAUTHORIZED,
			"Should return 401 Unauthorized without auth"
		);
	}

	/// Test 4: Failure - Verify with empty password
	///
	/// POST /accounts/auth/verify-password/
	/// Expected: 400 Bad Request or 422 Unprocessable Entity
	#[rstest]
	#[tokio::test]
	async fn test_failure_verify_with_empty_password(#[future] authenticated_context: (TestContext, crate::apps::auth::models::User)) {
		let (context, _user) = authenticated_context.await;

		// Build request body with empty password
		let body = json!({
			"password": ""
		});

		// Get URL from router
		let url = context
			.router
			.reverse("auth:verify_password", &[])
			.expect("auth:verify_password route should exist");

		// Send authenticated request
		let response = context
			.client
			.post(&url, &body, "json")
			.await
			.expect("Request should complete");

		// Assert validation error
		assert!(
			response.status() == StatusCode::BAD_REQUEST
				|| response.status() == StatusCode::UNPROCESSABLE_ENTITY,
			"Should return validation error for empty password, got: {}",
			response.status()
		);
	}
}
