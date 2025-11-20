//! Authentication integration test fixtures
//!
//! This module provides rstest fixtures for authentication integration tests,
//! including pre-configured session backends, mock users, and test data.

use rstest::*;
use uuid::Uuid;

/// Test user fixture
///
/// Provides a consistent test user for authentication tests.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::auth::test_user;
/// use rstest::*;
///
/// #[rstest]
/// fn test_with_user(test_user: TestUser) {
///     assert_eq!(test_user.username, "testuser");
/// }
/// ```
#[derive(Clone, Debug)]
pub struct TestUser {
	pub id: Uuid,
	pub username: String,
	pub email: String,
	pub is_active: bool,
	pub is_admin: bool,
	pub is_staff: bool,
	pub is_superuser: bool,
}

#[fixture]
pub fn test_user() -> TestUser {
	TestUser {
		id: Uuid::new_v4(),
		username: "testuser".to_string(),
		email: "test@example.com".to_string(),
		is_active: true,
		is_admin: false,
		is_staff: false,
		is_superuser: false,
	}
}

/// Admin user fixture
///
/// Provides a test user with admin privileges.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::auth::admin_user;
/// use rstest::*;
///
/// #[rstest]
/// fn test_with_admin(admin_user: TestUser) {
///     assert!(admin_user.is_admin);
///     assert!(admin_user.is_staff);
///     assert!(admin_user.is_superuser);
/// }
/// ```
#[fixture]
pub fn admin_user() -> TestUser {
	TestUser {
		id: Uuid::new_v4(),
		username: "admin".to_string(),
		email: "admin@example.com".to_string(),
		is_active: true,
		is_admin: true,
		is_staff: true,
		is_superuser: true,
	}
}

/// Multiple test users fixture
///
/// Provides a collection of test users for testing authentication scenarios
/// with multiple users.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::auth::test_users;
/// use rstest::*;
///
/// #[rstest]
/// fn test_with_multiple_users(test_users: Vec<TestUser>) {
///     assert_eq!(test_users.len(), 5);
/// }
/// ```
#[fixture]
pub fn test_users() -> Vec<TestUser> {
	vec![
		TestUser {
			id: Uuid::new_v4(),
			username: "user1".to_string(),
			email: "user1@example.com".to_string(),
			is_active: true,
			is_admin: false,
			is_staff: false,
			is_superuser: false,
		},
		TestUser {
			id: Uuid::new_v4(),
			username: "user2".to_string(),
			email: "user2@example.com".to_string(),
			is_active: true,
			is_admin: false,
			is_staff: false,
			is_superuser: false,
		},
		TestUser {
			id: Uuid::new_v4(),
			username: "user3".to_string(),
			email: "user3@example.com".to_string(),
			is_active: false, // Inactive user
			is_admin: false,
			is_staff: false,
			is_superuser: false,
		},
		TestUser {
			id: Uuid::new_v4(),
			username: "staff".to_string(),
			email: "staff@example.com".to_string(),
			is_active: true,
			is_admin: false,
			is_staff: true,
			is_superuser: false,
		},
		TestUser {
			id: Uuid::new_v4(),
			username: "superuser".to_string(),
			email: "superuser@example.com".to_string(),
			is_active: true,
			is_admin: true,
			is_staff: true,
			is_superuser: true,
		},
	]
}
