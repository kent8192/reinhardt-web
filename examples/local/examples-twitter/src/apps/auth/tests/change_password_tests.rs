//! Change password endpoint tests
//!
//! Tests for password change functionality including:
//! - Success cases (valid password change)
//! - Authentication errors (wrong current password, no auth)
//! - Validation errors (mismatched passwords, short password, whitespace password)

#[cfg(test)]
mod change_password_tests {
	use reinhardt::StatusCode;
	use reinhardt::core::serde::json::json;
	use reinhardt::db::orm::{FilterOperator, FilterValue, Model};
	use rstest::rstest;

	use crate::apps::auth::models::User;
	use crate::test_utils::fixtures::*;

	/// Test 1: Success - Change password with valid data
	///
	/// POST /accounts/auth/change-password/
	/// Expected: 204 No Content
	#[rstest]
	#[tokio::test]
	async fn test_success_change_password(#[future] authenticated_context: (TestContext, User)) {
		let (context, user) = authenticated_context.await;

		// Define old and new passwords
		let old_password = "password123"; // Default password from TestUserParams
		let new_password = "newpassword456";

		// Build request body
		let body = json!({
			"current_password": old_password,
			"new_password": new_password,
			"new_password_confirmation": new_password
		});

		// Get URL from router using reverse
		let url = context
			.router
			.reverse("auth:change_password", &[])
			.expect("auth:change_password route should exist");

		// Send request via APIClient (Authorization header already set)
		let response = context
			.client
			.post(&url, &body, "json")
			.await
			.expect("Request should succeed");

		// Assert response status
		assert_eq!(
			response.status(),
			StatusCode::NO_CONTENT,
			"Status should be 204 No Content"
		);

		// Verify new password works
		let updated_user = User::objects()
			.filter(
				User::field_id(),
				FilterOperator::Eq,
				FilterValue::String(user.id.to_string()),
			)
			.first_with_conn(&context.db)
			.await
			.expect("Database query should succeed")
			.expect("User should exist");

		assert!(
			updated_user.check_password(new_password).unwrap_or(false),
			"New password should work"
		);
		assert!(
			!updated_user.check_password(old_password).unwrap_or(true),
			"Old password should not work"
		);
	}

	/// Test 2: Failure - Change with wrong current password
	#[rstest]
	#[tokio::test]
	async fn test_failure_change_with_wrong_current_password(
		#[future] authenticated_context: (TestContext, User),
	) {
		let (context, _user) = authenticated_context.await;

		// Build request with wrong current password
		let body = json!({
			"current_password": "wrongpassword",
			"new_password": "newpassword123",
			"new_password_confirmation": "newpassword123"
		});

		// Get URL from router
		let url = context
			.router
			.reverse("auth:change_password", &[])
			.expect("auth:change_password route should exist");

		// Send request
		let response = context
			.client
			.post(&url, &body, "json")
			.await
			.expect("Request should complete");

		// Assert authentication error
		assert!(
			response.status() == StatusCode::UNAUTHORIZED
				|| response.status() == StatusCode::BAD_REQUEST,
			"Should return authentication error, got: {}",
			response.status()
		);
	}

	/// Test 3: Failure - Change with mismatched new passwords
	#[rstest]
	#[tokio::test]
	async fn test_failure_change_with_mismatched_passwords(
		#[future] authenticated_context: (TestContext, User),
	) {
		let (context, _user) = authenticated_context.await;

		// Build request with mismatched new passwords
		let body = json!({
			"current_password": "password123",
			"new_password": "newpassword123",
			"new_password_confirmation": "different456"
		});

		// Get URL from router
		let url = context.router.reverse("auth:change_password", &[]).unwrap();

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

	/// Test 4: Failure - Change with too short new password
	#[rstest]
	#[tokio::test]
	async fn test_failure_change_with_short_password(
		#[future] authenticated_context: (TestContext, User),
	) {
		let (context, _user) = authenticated_context.await;

		// Build request with short password (less than 8 chars)
		let body = json!({
			"current_password": "password123",
			"new_password": "short",
			"new_password_confirmation": "short"
		});

		// Get URL from router
		let url = context.router.reverse("auth:change_password", &[]).unwrap();

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

	/// Test 5: Failure - Change without authentication
	#[rstest]
	#[tokio::test]
	async fn test_failure_change_without_auth(#[future] test_context: TestContext) {
		let context = test_context.await;

		// Build valid request body
		let body = json!({
			"current_password": "currentpass",
			"new_password": "newpassword123",
			"new_password_confirmation": "newpassword123"
		});

		// Get URL from router
		let url = context.router.reverse("auth:change_password", &[]).unwrap();

		// Create a new client without Authorization header
		let client = reinhardt::test::client::APIClient::with_base_url(&context._guard.url());

		// Send request
		let response = client.post(&url, &body, "json").await.unwrap();

		// Assert authentication error
		assert!(
			response.status() == StatusCode::UNAUTHORIZED
				|| response.status() == StatusCode::FORBIDDEN,
			"Should return authentication error, got: {}",
			response.status()
		);
	}

	/// Test 6: Failure - Change with whitespace-only new password
	#[rstest]
	#[tokio::test]
	async fn test_failure_change_with_whitespace_password(
		#[future] authenticated_context: (TestContext, User),
	) {
		let (context, _user) = authenticated_context.await;

		// Build request with whitespace-only password
		let body = json!({
			"current_password": "password123",
			"new_password": "        ",
			"new_password_confirmation": "        "
		});

		// Get URL from router
		let url = context.router.reverse("auth:change_password", &[]).unwrap();

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
}
