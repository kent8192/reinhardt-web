//! Internal user type for testing and development.
//!
//! Provides a concrete [`AuthIdentity`] implementation with common user fields.
//! For production, implement [`AuthIdentity`] on your own user model.

use crate::core::AuthIdentity;
use uuid::Uuid;

/// A simple user type implementing [`AuthIdentity`] for testing and development.
///
/// This type replaces the removed `SimpleUser` struct. It carries common user
/// fields (username, email, active/admin/staff/superuser flags) but only
/// exposes `id()`, `is_authenticated()`, and `is_admin()` through the
/// [`AuthIdentity`] trait.
#[derive(Debug, Clone)]
pub struct InternalUser {
	/// The unique user identifier.
	pub id: Uuid,
	/// The user's login name.
	pub username: String,
	/// The user's email address.
	pub email: String,
	/// Whether the user account is active.
	pub is_active: bool,
	/// Whether the user has admin privileges.
	pub is_admin: bool,
	/// Whether the user is a staff member.
	pub is_staff: bool,
	/// Whether the user is a superuser.
	pub is_superuser: bool,
}

impl AuthIdentity for InternalUser {
	fn id(&self) -> String {
		self.id.to_string()
	}

	fn is_authenticated(&self) -> bool {
		true
	}

	fn is_admin(&self) -> bool {
		self.is_admin
	}
}
