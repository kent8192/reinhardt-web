//! Register endpoint tests
//!
//! Tests for user registration functionality including:
//! - Success cases (valid data)
//! - Validation errors (empty, whitespace, invalid format)
//! - Duplicate email handling

#[cfg(test)]
mod register_tests {
	use reinhardt::StatusCode;
	use reinhardt::core::serde::json::json;
	use reinhardt::db::orm::{FilterOperator, FilterValue, Model};
	use rstest::rstest;

	use crate::apps::auth::models::User;
	use crate::test_utils::fixtures::*;

	/// Test 1: Success - Register with valid data
	///
	/// POST /accounts/auth/register/
	/// Expected: 204 No Content, user created in DB
	#[rstest]
	#[tokio::test]
	async fn test_success_register(#[future] test_context: TestContext) {
		let context = test_context.await;

		// Build request body
		let body = json!({
			"email": "newuser@example.com",
			"username": "newuser",
			"password": "password123",
			"password_confirmation": "password123"
		});

		// Get URL from router using reverse
		let url = context
			.router
			.reverse("auth:register", &[])
			.expect("auth:register route should exist");

		// Send request via APIClient
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

		// Assert user was created in database
		let user = User::objects()
			.filter(
				User::field_email(),
				FilterOperator::Eq,
				FilterValue::String("newuser@example.com".to_string()),
			)
			.first_with_conn(&context.db)
			.await
			.expect("Database query should succeed")
			.expect("User should exist");

		assert_eq!(user.email, "newuser@example.com");
		assert_eq!(user.username, "newuser");
	}

	/// Test 2: Failure - Register with empty form
	/// Expected: 400 Bad Request, no user created
	#[rstest]
	#[tokio::test]
	async fn test_failure_register_with_empty_form(#[future] test_context: TestContext) {
		let context = test_context.await;

		// Send empty JSON body
		let body = json!({});

		// Get URL from router
		let url = context
			.router
			.reverse("auth:register", &[])
			.expect("auth:register route should exist");

		// Send request
		let response = context
			.client
			.post(&url, &body, "json")
			.await
			.expect("Request should complete");

		// Assert validation error (400 or 422)
		assert!(
			response.status() == StatusCode::BAD_REQUEST
				|| response.status() == StatusCode::UNPROCESSABLE_ENTITY,
			"Should return validation error, got: {}",
			response.status()
		);

		// Assert no user created
		let user_count = User::objects()
			.all_with_db(&context.db)
			.await
			.expect("Query should succeed")
			.len();
		assert_eq!(user_count, 0, "No user should be created");
	}

	/// Test 3: Failure - Register with whitespace strings
	#[rstest]
	#[tokio::test]
	async fn test_failure_register_with_form_including_whitespaces(
		#[future] test_context: TestContext,
	) {
		let context = test_context.await;

		// Send form with whitespace-only email
		let body = json!({
			"email": "   ",
			"username": "testuser",
			"password": "password123",
			"password_confirmation": "password123"
		});

		let url = context.router.reverse("auth:register", &[]).unwrap();
		let response = context.client.post(&url, &body, "json").await.unwrap();

		// Assert validation error
		assert!(
			response.status() == StatusCode::BAD_REQUEST
				|| response.status() == StatusCode::UNPROCESSABLE_ENTITY,
			"Should return validation error"
		);

		// Assert no user created
		let user_count = User::objects()
			.all_with_db(&context.db)
			.await
			.unwrap()
			.len();
		assert_eq!(user_count, 0);
	}

	/// Test 4: Failure - Register with whitespace password
	#[rstest]
	#[tokio::test]
	async fn test_failure_register_with_password_including_whitespaces(
		#[future] test_context: TestContext,
	) {
		let context = test_context.await;

		let body = json!({
			"email": "test@example.com",
			"username": "testuser",
			"password": "        ",
			"password_confirmation": "        "
		});

		let url = context.router.reverse("auth:register", &[]).unwrap();
		let response = context.client.post(&url, &body, "json").await.unwrap();

		assert!(
			response.status() == StatusCode::BAD_REQUEST
				|| response.status() == StatusCode::UNPROCESSABLE_ENTITY
		);

		let user_count = User::objects()
			.all_with_db(&context.db)
			.await
			.unwrap()
			.len();
		assert_eq!(user_count, 0);
	}

	/// Test 5: Failure - Register with duplicated email
	/// Expected: 400 Bad Request, no duplicate user created
	#[rstest]
	#[tokio::test]
	async fn test_failure_register_with_duplicated_email(#[future] test_context: TestContext) {
		let context = test_context.await;

		// Create existing user using test helper
		let _existing = create_test_user(
			&context.db,
			TestUserParams::default()
				.with_email("existing@example.com")
				.with_username("existing"),
		)
		.await;

		// Try to register with same email
		let body = json!({
			"email": "existing@example.com",
			"username": "newuser",
			"password": "password123",
			"password_confirmation": "password123"
		});

		let url = context.router.reverse("auth:register", &[]).unwrap();
		let response = context.client.post(&url, &body, "json").await.unwrap();

		// Assert error response
		assert!(
			response.status() == StatusCode::BAD_REQUEST
				|| response.status() == StatusCode::CONFLICT,
			"Should return error for duplicate email"
		);

		// Assert only one user exists
		let user_count = User::objects()
			.all_with_db(&context.db)
			.await
			.unwrap()
			.len();
		assert_eq!(user_count, 1, "Only original user should exist");
	}

	/// Test 6: Failure - Register with invalid email format
	#[rstest]
	#[tokio::test]
	async fn test_failure_register_with_invalid_email(#[future] test_context: TestContext) {
		let context = test_context.await;

		let body = json!({
			"email": "notanemail",
			"username": "testuser",
			"password": "password123",
			"password_confirmation": "password123"
		});

		let url = context.router.reverse("auth:register", &[]).unwrap();
		let response = context.client.post(&url, &body, "json").await.unwrap();

		assert!(
			response.status() == StatusCode::BAD_REQUEST
				|| response.status() == StatusCode::UNPROCESSABLE_ENTITY
		);

		let user_count = User::objects()
			.all_with_db(&context.db)
			.await
			.unwrap()
			.len();
		assert_eq!(user_count, 0);
	}

	/// Test 7: Failure - Register with too short password
	#[rstest]
	#[tokio::test]
	async fn test_failure_register_with_too_short_password(#[future] test_context: TestContext) {
		let context = test_context.await;

		let body = json!({
			"email": "test@example.com",
			"username": "testuser",
			"password": "pass",
			"password_confirmation": "pass"
		});

		let url = context.router.reverse("auth:register", &[]).unwrap();
		let response = context.client.post(&url, &body, "json").await.unwrap();

		assert!(
			response.status() == StatusCode::BAD_REQUEST
				|| response.status() == StatusCode::UNPROCESSABLE_ENTITY
		);

		let user_count = User::objects()
			.all_with_db(&context.db)
			.await
			.unwrap()
			.len();
		assert_eq!(user_count, 0);
	}

	/// Test 8: Failure - Register with mismatched passwords
	#[rstest]
	#[tokio::test]
	async fn test_failure_register_with_password_mismatch(#[future] test_context: TestContext) {
		let context = test_context.await;

		let body = json!({
			"email": "test@example.com",
			"username": "testuser",
			"password": "password123",
			"password_confirmation": "different456"
		});

		let url = context.router.reverse("auth:register", &[]).unwrap();
		let response = context.client.post(&url, &body, "json").await.unwrap();

		assert!(
			response.status() == StatusCode::BAD_REQUEST
				|| response.status() == StatusCode::UNPROCESSABLE_ENTITY
		);

		let user_count = User::objects()
			.all_with_db(&context.db)
			.await
			.unwrap()
			.len();
		assert_eq!(user_count, 0);
	}
}
