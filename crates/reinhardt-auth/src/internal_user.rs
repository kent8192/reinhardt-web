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
#[allow(dead_code, reason = "fields are used by InternalUser constructors in tests")]
pub(crate) struct InternalUser {
	pub(crate) id: Uuid,
	pub(crate) username: String,
	pub(crate) email: String,
	pub(crate) is_active: bool,
	pub(crate) is_admin: bool,
	pub(crate) is_staff: bool,
	pub(crate) is_superuser: bool,
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
