//! Dependency Injection Support for Permissions
//!
//! Provides integration with Reinhardt's dependency injection system
//! for permission instances.

use crate::permissions::{
	AllowAny, IsActiveUser, IsAdminUser, IsAuthenticated, IsAuthenticatedOrReadOnly,
};

/// Implement Clone for basic permissions to support DI
impl Clone for AllowAny {
	fn clone(&self) -> Self {
		Self
	}
}

impl Clone for IsAuthenticated {
	fn clone(&self) -> Self {
		Self
	}
}

impl Clone for IsAdminUser {
	fn clone(&self) -> Self {
		Self
	}
}

impl Clone for IsActiveUser {
	fn clone(&self) -> Self {
		Self
	}
}

impl Clone for IsAuthenticatedOrReadOnly {
	fn clone(&self) -> Self {
		Self
	}
}

/// Implement Default for basic permissions
impl Default for AllowAny {
	fn default() -> Self {
		Self
	}
}

impl Default for IsAuthenticated {
	fn default() -> Self {
		Self
	}
}

impl Default for IsAdminUser {
	fn default() -> Self {
		Self
	}
}

impl Default for IsActiveUser {
	fn default() -> Self {
		Self
	}
}

impl Default for IsAuthenticatedOrReadOnly {
	fn default() -> Self {
		Self
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_allow_any_clone() {
		let perm = AllowAny;
		let cloned = perm.clone();
		drop(cloned);
	}

	#[test]
	fn test_allow_any_default() {
		let perm = AllowAny::default();
		drop(perm);
	}

	#[test]
	fn test_is_authenticated_clone() {
		let perm = IsAuthenticated;
		let cloned = perm.clone();
		drop(cloned);
	}

	#[test]
	fn test_is_authenticated_default() {
		let perm = IsAuthenticated::default();
		drop(perm);
	}

	#[test]
	fn test_is_admin_user_clone() {
		let perm = IsAdminUser;
		let cloned = perm.clone();
		drop(cloned);
	}

	#[test]
	fn test_is_admin_user_default() {
		let perm = IsAdminUser::default();
		drop(perm);
	}

	#[test]
	fn test_is_active_user_clone() {
		let perm = IsActiveUser;
		let cloned = perm.clone();
		drop(cloned);
	}

	#[test]
	fn test_is_active_user_default() {
		let perm = IsActiveUser::default();
		drop(perm);
	}

	#[test]
	fn test_is_authenticated_or_read_only_clone() {
		let perm = IsAuthenticatedOrReadOnly;
		let cloned = perm.clone();
		drop(cloned);
	}

	#[test]
	fn test_is_authenticated_or_read_only_default() {
		let perm = IsAuthenticatedOrReadOnly::default();
		drop(perm);
	}
}
