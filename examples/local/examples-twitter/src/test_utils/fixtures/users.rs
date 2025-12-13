//! User fixtures for tests.
//!
//! Provides user creation fixtures and helpers.

use crate::apps::auth::models::User;
use crate::test_utils::fixtures::TestDatabase;
use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHasher};
use chrono::Utc;
use reinhardt::db::DatabaseConnection;
use rstest::*;
use std::sync::Arc;
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

/// Hash a password using Argon2.
fn hash_password(password: &str) -> String {
	let salt = SaltString::generate(&mut OsRng);
	let argon2 = Argon2::default();
	argon2
		.hash_password(password.as_bytes(), &salt)
		.expect("Failed to hash password")
		.to_string()
}

/// Create a test user in the database.
///
/// Uses raw SQL for insertion to avoid ORM complexity in tests.
/// This is acceptable for test fixtures.
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
	let id = Uuid::new_v4();
	let password_hash = hash_password(&params.password);
	let now = Utc::now();

	// Insert user into database using raw SQL
	// This is acceptable for test fixtures to avoid ORM complexity
	let sql = format!(
		r#"INSERT INTO auth_user (id, username, email, password_hash, is_active, created_at)
		VALUES ('{}', '{}', '{}', '{}', {}, '{}')"#,
		id,
		params.username,
		params.email,
		password_hash,
		params.is_active,
		now.format("%Y-%m-%d %H:%M:%S%.6f")
	);

	db.execute(&sql, vec![])
		.await
		.expect("Failed to create test user");

	User {
		id,
		username: params.username,
		email: params.email,
		password_hash: Some(password_hash),
		is_active: params.is_active,
		last_login: None,
		created_at: now,
		following: Default::default(),
		blocked_users: Default::default(),
	}
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
