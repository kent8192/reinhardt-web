//! CurrentUser Injectable for dependency injection
//!
//! Provides access to the authenticated user in endpoint handlers.

use crate::AuthenticationError;
use crate::BaseUser;
use async_trait::async_trait;
use reinhardt_di::{DiResult, Injectable, InjectionContext};
use uuid::Uuid;

/// Wrapper type representing the currently authenticated user for DI.
///
/// This type provides access to the authenticated user within endpoint handlers
/// through dependency injection. It wraps an optional user instance and user ID,
/// allowing handlers to check authentication status and access user data.
///
/// # Example
///
/// ```rust,ignore
/// use reinhardt::prelude::*;
/// use reinhardt::CurrentUser;
///
/// #[endpoint]
/// async fn my_handler(
///     #[inject] current_user: CurrentUser<User>,
/// ) -> ViewResult<Response> {
///     let user = current_user.user()?;
///     let user_id = current_user.id()?;
///     // Use user data...
///     Ok(Response::ok())
/// }
/// ```
pub struct CurrentUser<U: BaseUser + Clone> {
	user: Option<U>,
	user_id: Option<Uuid>,
}

impl<U: BaseUser + Clone> Clone for CurrentUser<U> {
	fn clone(&self) -> Self {
		Self {
			user: self.user.clone(),
			user_id: self.user_id,
		}
	}
}

impl<U: BaseUser + Clone> CurrentUser<U> {
	/// Creates a new authenticated CurrentUser.
	///
	/// # Arguments
	///
	/// * `user` - The authenticated user instance
	/// * `user_id` - The user's unique identifier
	pub fn authenticated(user: U, user_id: Uuid) -> Self {
		Self {
			user: Some(user),
			user_id: Some(user_id),
		}
	}

	/// Creates an anonymous (unauthenticated) CurrentUser.
	pub fn anonymous() -> Self {
		Self {
			user: None,
			user_id: None,
		}
	}

	/// Returns whether the current user is authenticated.
	pub fn is_authenticated(&self) -> bool {
		self.user.is_some()
	}

	/// Returns a reference to the user if authenticated.
	///
	/// # Errors
	///
	/// Returns `AuthenticationError::NotAuthenticated` if the user is not authenticated.
	pub fn user(&self) -> Result<&U, AuthenticationError> {
		self.user
			.as_ref()
			.ok_or(AuthenticationError::NotAuthenticated)
	}

	/// Returns the user ID if authenticated.
	///
	/// # Errors
	///
	/// Returns `AuthenticationError::NotAuthenticated` if the user is not authenticated.
	pub fn id(&self) -> Result<Uuid, AuthenticationError> {
		self.user_id.ok_or(AuthenticationError::NotAuthenticated)
	}

	/// Consumes this wrapper and returns the user if authenticated.
	///
	/// # Errors
	///
	/// Returns `AuthenticationError::NotAuthenticated` if the user is not authenticated.
	pub fn into_user(self) -> Result<U, AuthenticationError> {
		self.user.ok_or(AuthenticationError::NotAuthenticated)
	}
}

#[async_trait]
impl<U: BaseUser + Clone + Send + Sync + 'static> Injectable for CurrentUser<U> {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		// Current implementation returns anonymous user.
		// Session integration will be added in future phases:
		// 1. Extract session from Request
		// 2. Get user_id from session
		// 3. Load user from database
		Ok(Self::anonymous())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use chrono::{DateTime, Utc};

	// Test user implementation for unit tests
	#[derive(Clone)]
	struct TestUser {
		id: Uuid,
		username: String,
		is_active: bool,
	}

	impl BaseUser for TestUser {
		type PrimaryKey = Uuid;

		fn get_username_field() -> &'static str {
			"username"
		}

		fn get_username(&self) -> &str {
			&self.username
		}

		fn password_hash(&self) -> Option<&str> {
			None
		}

		fn set_password_hash(&mut self, _hash: String) {}

		fn last_login(&self) -> Option<DateTime<Utc>> {
			None
		}

		fn set_last_login(&mut self, _time: DateTime<Utc>) {}

		fn is_active(&self) -> bool {
			self.is_active
		}
	}

	#[test]
	fn test_authenticated_user() {
		let user_id = Uuid::new_v4();
		let user = TestUser {
			id: user_id,
			username: "testuser".to_string(),
			is_active: true,
		};

		let current_user = CurrentUser::authenticated(user, user_id);

		assert!(current_user.is_authenticated());
		assert_eq!(current_user.id().unwrap(), user_id);
		assert_eq!(current_user.user().unwrap().get_username(), "testuser");
	}

	#[test]
	fn test_anonymous_user() {
		let current_user: CurrentUser<TestUser> = CurrentUser::anonymous();

		assert!(!current_user.is_authenticated());
		assert!(current_user.id().is_err());
		assert!(current_user.user().is_err());
	}

	#[test]
	fn test_into_user_authenticated() {
		let user_id = Uuid::new_v4();
		let user = TestUser {
			id: user_id,
			username: "testuser".to_string(),
			is_active: true,
		};

		let current_user = CurrentUser::authenticated(user, user_id);
		let extracted = current_user.into_user().unwrap();

		assert_eq!(extracted.get_username(), "testuser");
	}

	#[test]
	fn test_into_user_anonymous() {
		let current_user: CurrentUser<TestUser> = CurrentUser::anonymous();
		let result = current_user.into_user();

		assert!(result.is_err());
	}
}
