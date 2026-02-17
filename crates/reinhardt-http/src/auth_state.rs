//! Authentication state stored in request extensions.
//!
//! This module provides [`AuthState`], a helper struct that stores
//! authentication information in request extensions.

use crate::Extensions;

/// Helper struct to store authentication state in request extensions.
///
/// This struct is used by authentication middleware to communicate
/// the authenticated user's information to downstream handlers.
///
/// # Example
///
/// ```rust,no_run
/// # use reinhardt_http::AuthState;
/// # struct Request { extensions: Extensions }
/// # struct Extensions;
/// # impl Extensions {
/// #     fn insert<T>(&mut self, _value: T) {}
/// #     fn get<T>(&self) -> Option<T> { None }
/// # }
/// # let mut request = Request { extensions: Extensions };
/// // In middleware (after authentication)
/// request.extensions.insert(AuthState::authenticated("123", false, true));
///
/// // In handler (via CurrentUser or directly)
/// let auth_state: Option<AuthState> = request.extensions.get();
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthState {
	/// The authenticated user's ID as a string.
	///
	/// This is typically a UUID or database primary key serialized to string.
	pub user_id: String,

	/// Whether the user is authenticated.
	pub is_authenticated: bool,

	/// Whether the user has admin/superuser privileges.
	pub is_admin: bool,

	/// Whether the user's account is active.
	pub is_active: bool,
}

impl AuthState {
	/// Creates a new authenticated state.
	///
	/// # Arguments
	///
	/// * `user_id` - The authenticated user's ID
	/// * `is_admin` - Whether the user has admin privileges
	/// * `is_active` - Whether the user's account is active
	pub fn authenticated(user_id: impl Into<String>, is_admin: bool, is_active: bool) -> Self {
		Self {
			user_id: user_id.into(),
			is_authenticated: true,
			is_admin,
			is_active,
		}
	}

	/// Creates an anonymous (unauthenticated) state.
	pub fn anonymous() -> Self {
		Self {
			user_id: String::new(),
			is_authenticated: false,
			is_admin: false,
			is_active: false,
		}
	}

	/// Create auth state from request extensions.
	///
	/// This method extracts authentication-related data that was stored
	/// as individual values in extensions by the authentication middleware.
	///
	/// # Returns
	///
	/// Returns `Some(AuthState)` if user_id and is_authenticated are found,
	/// `None` otherwise.
	pub fn from_extensions(extensions: &Extensions) -> Option<Self> {
		Some(Self {
			user_id: extensions.get::<String>()?,
			is_authenticated: extensions.get::<bool>()?,
			is_admin: false,
			is_active: false,
		})
	}

	/// Check if user is anonymous (not authenticated).
	pub fn is_anonymous(&self) -> bool {
		!self.is_authenticated
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_authenticated() {
		let state = AuthState::authenticated("user-123", true, true);

		assert_eq!(state.user_id, "user-123");
		assert!(state.is_authenticated);
		assert!(state.is_admin);
		assert!(state.is_active);
	}

	#[rstest]
	fn test_anonymous() {
		let state = AuthState::anonymous();

		assert!(state.user_id.is_empty());
		assert!(!state.is_authenticated);
		assert!(!state.is_admin);
		assert!(!state.is_active);
	}
}
