//! Custom assertion helpers for tests.
//!
//! Provides trait extensions for common test assertions.

use crate::apps::auth::models::User;
use reinhardt::Model;
use reinhardt::db::DatabaseConnection;
use reinhardt::db::orm::{FilterOperator, FilterValue};
use serde_json::Value;
use uuid::Uuid;

/// Database query helpers for assertions.
pub struct DbAssertions;

impl DbAssertions {
	/// Check if a user exists in the database by email.
	pub async fn user_exists_by_email(db: &DatabaseConnection, email: &str) -> bool {
		User::objects()
			.filter(
				User::field_email(),
				FilterOperator::Eq,
				FilterValue::String(email.to_string()),
			)
			.first_with_db(db)
			.await
			.is_ok()
	}

	/// Get a user from the database by ID.
	pub async fn get_user_by_id(db: &DatabaseConnection, id: Uuid) -> Option<User> {
		User::objects()
			.get(id)
			.first_with_db(db)
			.await
			.ok()
			.flatten()
	}

	/// Count users in the database.
	pub async fn count_users(db: &DatabaseConnection) -> i64 {
		User::objects().count_with_conn(db).await.unwrap_or(0)
	}

	/// Delete all users from the database (for cleanup).
	pub async fn cleanup_users(db: &DatabaseConnection) {
		let _ = db.execute("DELETE FROM auth_user_following", vec![]).await;
		let _ = db
			.execute("DELETE FROM auth_user_blocked_users", vec![])
			.await;
		let _ = db.execute("DELETE FROM auth_user", vec![]).await;
	}

	/// Delete a specific user by ID.
	pub async fn delete_user(db: &DatabaseConnection, id: Uuid) {
		let _ = User::objects().delete_with_conn(db, id).await;
	}
}

/// JSON assertion helpers.
pub trait JsonAssertions {
	/// Assert that JSON contains a specific key with a value.
	fn assert_json_contains(&self, key: &str, expected_value: &str);

	/// Assert that JSON has a specific key.
	fn assert_json_has_key(&self, key: &str);
}

impl JsonAssertions for Value {
	fn assert_json_contains(&self, key: &str, expected_value: &str) {
		let actual = self.get(key).and_then(|v| v.as_str());
		assert_eq!(
			actual,
			Some(expected_value),
			"Expected key '{}' to have value '{}', but got {:?}",
			key,
			expected_value,
			actual
		);
	}

	fn assert_json_has_key(&self, key: &str) {
		assert!(
			self.get(key).is_some(),
			"Expected JSON to have key '{}', but it was missing",
			key
		);
	}
}
