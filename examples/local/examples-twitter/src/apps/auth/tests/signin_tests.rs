//! Signin endpoint tests
//!
//! Tests for user signin functionality including:
//! - Success cases (valid credentials)
//! - Validation errors (invalid email, empty password)
//! - Authentication errors (wrong password, inactive user, non-existent user)

#[cfg(test)]
mod signin_tests {
	use reinhardt::StatusCode;
	use reinhardt::core::serde::json::json;
	use reinhardt::db::orm::{FilterOperator, FilterValue, Model};
	use rstest::rstest;

	use crate::apps::auth::models::User;
	use crate::apps::auth::serializers::SigninResponse;
	use crate::test_utils::fixtures::*;

	use super::super::helpers::valid_signin_request;

	/// Test 1: Success - Signin with valid credentials
	///
	/// POST /accounts/auth/signin/
	/// Expected: 200 OK with JWT token and user info
	#[rstest]
	#[tokio::test]
	async fn test_success_signin(#[future] test_context: TestContext) {
		let context = test_context.await;

		// Create test user with known password
		let test_password = "testpassword123";
		let user = create_test_user(
			&context.db,
			TestUserParams::default()
				.with_email("signin_test@example.com")
				.with_username("signin_user")
				.with_password(test_password),
		)
		.await;

		// Get URL from router
		let url = context
			.router
			.reverse("auth:signin", &[])
			.expect("auth:signin route should exist");

		// Send signin request
		let body = valid_signin_request(&user.email, test_password);
		let response = context
			.client
			.post(&url, &body, "json")
			.await
			.expect("Request should succeed");

		// Assert success status
		assert_eq!(
			response.status(),
			StatusCode::OK,
			"Signin should succeed with valid credentials"
		);

		// Parse response body
		let response_text = response.text().await.expect("Should get response text");
		let signin_response: SigninResponse =
			serde_json::from_str(&response_text).expect("Should parse SigninResponse");

		// Assert token is not empty
		assert!(
			!signin_response.token.is_empty(),
			"Token should not be empty"
		);
		assert!(
			signin_response.token.contains('.'),
			"Token should be a valid JWT format"
		);

		// Assert user info is correct
		assert_eq!(
			signin_response.user.id,
			user.id.to_string(),
			"User ID should match"
		);
		assert_eq!(
			signin_response.user.username, user.username,
			"Username should match"
		);
		assert_eq!(signin_response.user.email, user.email, "Email should match");
	}

	/// Test 2: Failure - Signin with invalid email format
	#[rstest]
	#[tokio::test]
	async fn test_failure_signin_with_invalid_email(#[future] test_context: TestContext) {
		let context = test_context.await;

		// Get URL from router
		let url = context.router.reverse("auth:signin", &[]).unwrap();

		// Send signin request with invalid email format
		let body = json!({
			"email": "notanemail",
			"password": "password123"
		});
		let response = context.client.post(&url, &body, "json").await.unwrap();

		// Assert validation error
		assert!(
			response.status() == StatusCode::BAD_REQUEST
				|| response.status() == StatusCode::UNPROCESSABLE_ENTITY,
			"Should return validation error for invalid email, got: {}",
			response.status()
		);
	}

	/// Test 3: Failure - Signin with wrong password
	#[rstest]
	#[tokio::test]
	async fn test_failure_signin_with_wrong_password(#[future] test_context: TestContext) {
		let context = test_context.await;

		// Create test user with known password
		let test_password = "correctpassword";
		let user = create_test_user(
			&context.db,
			TestUserParams::default()
				.with_email("wrongpass@example.com")
				.with_username("wrongpass_user")
				.with_password(test_password),
		)
		.await;

		// Get URL from router
		let url = context.router.reverse("auth:signin", &[]).unwrap();

		// Send signin request with wrong password
		let body = valid_signin_request(&user.email, "wrongpassword");
		let response = context.client.post(&url, &body, "json").await.unwrap();

		// Assert authentication error
		assert!(
			response.status() == StatusCode::UNAUTHORIZED
				|| response.status() == StatusCode::FORBIDDEN,
			"Should return authentication error for wrong password, got: {}",
			response.status()
		);
	}

	/// Test 4: Failure - Signin with inactive user
	#[rstest]
	#[tokio::test]
	async fn test_failure_signin_with_inactive_user(#[future] test_context: TestContext) {
		let context = test_context.await;

		// Create inactive user
		let test_password = "password123";
		let user = create_test_user(
			&context.db,
			TestUserParams::default()
				.with_email("inactive@example.com")
				.with_username("inactive_user")
				.with_password(test_password)
				.inactive(),
		)
		.await;

		// Get URL from router
		let url = context.router.reverse("auth:signin", &[]).unwrap();

		// Send signin request with valid credentials
		let body = valid_signin_request(&user.email, test_password);
		let response = context.client.post(&url, &body, "json").await.unwrap();

		// Assert authentication error
		assert!(
			response.status() == StatusCode::UNAUTHORIZED
				|| response.status() == StatusCode::FORBIDDEN,
			"Should return authentication error for inactive user, got: {}",
			response.status()
		);
	}

	/// Test 5: Failure - Signin with non-existent user
	#[rstest]
	#[tokio::test]
	async fn test_failure_signin_with_nonexistent_user(#[future] test_context: TestContext) {
		let context = test_context.await;

		// Get URL from router
		let url = context.router.reverse("auth:signin", &[]).unwrap();

		// Send signin request with email that doesn't exist
		let body = valid_signin_request("nonexistent@example.com", "password123");
		let response = context.client.post(&url, &body, "json").await.unwrap();

		// Assert authentication error
		assert!(
			response.status() == StatusCode::UNAUTHORIZED
				|| response.status() == StatusCode::FORBIDDEN,
			"Should return authentication error for nonexistent user, got: {}",
			response.status()
		);
	}

	/// Test 6: Failure - Signin with empty password
	#[rstest]
	#[tokio::test]
	async fn test_failure_signin_with_empty_password(#[future] test_context: TestContext) {
		let context = test_context.await;

		// Get URL from router
		let url = context.router.reverse("auth:signin", &[]).unwrap();

		// Send signin request with empty password
		let body = json!({
			"email": "test@example.com",
			"password": ""
		});
		let response = context.client.post(&url, &body, "json").await.unwrap();

		// Assert validation error
		assert!(
			response.status() == StatusCode::BAD_REQUEST
				|| response.status() == StatusCode::UNPROCESSABLE_ENTITY,
			"Should return validation error for empty password, got: {}",
			response.status()
		);
	}
}
