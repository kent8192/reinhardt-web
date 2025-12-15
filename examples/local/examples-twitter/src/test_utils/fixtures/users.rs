//! User fixtures for tests.
//!
//! Provides user creation fixtures and helpers.

use crate::apps::auth::models::User;
use crate::test_utils::fixtures::{TestDatabase, test_database};
use reinhardt::db::DatabaseConnection;
use reinhardt::{BaseUser, Model};
use rstest::*;
use uuid::Uuid;

/// Parameters for creating a test user.
#[derive(Debug, Clone)]
pub struct TestUserParams {
	pub email: String,
	pub username: String,
	pub password: String,
	pub is_active: bool,
}

impl Default for TestUserParams {
	fn default() -> Self {
		let unique_id = Uuid::new_v4().to_string()[..8].to_string();
		Self {
			email: format!("test_{}@example.com", unique_id),
			username: format!("testuser_{}", unique_id),
			password: "password123".to_string(),
			is_active: true,
		}
	}
}

impl TestUserParams {
	/// Create params with a specific email.
	pub fn with_email(mut self, email: impl Into<String>) -> Self {
		self.email = email.into();
		self
	}

	/// Create params with a specific username.
	pub fn with_username(mut self, username: impl Into<String>) -> Self {
		self.username = username.into();
		self
	}

	/// Create params with a specific password.
	pub fn with_password(mut self, password: impl Into<String>) -> Self {
		self.password = password.into();
		self
	}

	/// Create params with inactive status.
	pub fn inactive(mut self) -> Self {
		self.is_active = false;
		self
	}
}

/// Create a test user in the database.
///
/// Uses reinhardt ORM for type-safe insertion with SQL injection protection.
///
/// # Arguments
///
/// * `db` - Database connection
/// * `params` - User creation parameters
///
/// # Returns
///
/// The created User with its ID set.
///
/// # Example
///
/// ```rust,no_run
/// # #[tokio::main]
/// # async fn main() {
/// let db = Arc::new(connection);
/// let user = create_test_user(&db, TestUserParams::default()).await;
/// assert_eq!(user.is_active, true);
///
/// # }
/// ```
pub async fn create_test_user(db: &DatabaseConnection, params: TestUserParams) -> User {
	// Create user using User::new() which auto-generates id, created_at, and ManyToManyFields
	let mut user = User::new(
		params.username,
		params.email,
		None, // password_hash will be set after hashing
		params.is_active,
		None, // bio (optional)
	);

	// Hash password using BaseUser trait
	user.set_password(&params.password)
		.expect("Failed to hash password");

	// Create user in database using ORM
	User::objects()
		.create_with_conn(db, &user)
		.await
		.expect("Failed to create test user")
}

/// Create multiple test users.
///
/// # Example
///
/// ```rust,no_run
/// # #[tokio::main]
/// # async fn main() {
/// let users = create_test_users(&db, 5).await;
/// assert_eq!(users.len(), 5);
///
/// # }
/// ```
pub async fn create_test_users(db: &DatabaseConnection, count: usize) -> Vec<User> {
	let mut users = Vec::with_capacity(count);
	for i in 0..count {
		let params = TestUserParams::default()
			.with_username(format!("testuser_{}", i))
			.with_email(format!("test_{}@example.com", i));
		users.push(create_test_user(db, params).await);
	}
	users
}

/// Default test user fixture.
///
/// Creates a single test user with default parameters.
///
/// # Example
///
/// ```rust,no_run
/// # #[tokio::main]
/// # async fn main() {
/// #[rstest]
/// #[tokio::test]
/// async fn my_test(#[future] test_user: (User, TestDatabase)) {
///     let (user, (_container, db)) = test_user.await;
///     assert!(user.is_active);
/// }
/// ```
#[fixture]
pub async fn test_user(#[future] test_database: TestDatabase) -> (User, TestDatabase) {
	let db_tuple = test_database.await;
	let user = create_test_user(&db_tuple.1, TestUserParams::default()).await;

	(user, db_tuple)
}
