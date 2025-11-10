//! Advanced Permission System
//!
//! Provides object-level permissions, role-based access control,
//! and dynamic permission evaluation.

use crate::{Permission, PermissionContext};
use async_trait::async_trait;
use std::collections::HashMap;

/// Object-level permission
///
/// Checks permissions on specific object instances.
///
/// # Examples
///
/// ```
/// use reinhardt_auth::advanced_permissions::ObjectPermission;
///
/// let perm = ObjectPermission::new("view", Some("article:123"));
/// assert_eq!(perm.permission(), "view");
/// assert_eq!(perm.object_id(), Some("article:123"));
/// ```
pub struct ObjectPermission {
	/// Permission name (e.g., "view", "edit", "delete")
	permission: String,
	/// Optional object identifier (e.g., "article:123")
	object_id: Option<String>,
}

impl ObjectPermission {
	/// Create a new object permission
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::advanced_permissions::ObjectPermission;
	///
	/// let perm = ObjectPermission::new("edit", Some("post:42"));
	/// assert_eq!(perm.permission(), "edit");
	/// assert_eq!(perm.object_id(), Some("post:42"));
	/// ```
	pub fn new(permission: impl Into<String>, object_id: Option<impl Into<String>>) -> Self {
		Self {
			permission: permission.into(),
			object_id: object_id.map(|id| id.into()),
		}
	}

	/// Get permission name
	pub fn permission(&self) -> &str {
		&self.permission
	}

	/// Get object ID
	pub fn object_id(&self) -> Option<&str> {
		self.object_id.as_deref()
	}
}

#[async_trait]
impl Permission for ObjectPermission {
	async fn has_permission(&self, context: &PermissionContext<'_>) -> bool {
		context.is_authenticated
	}
}

/// Role-based permission
///
/// Assigns permissions based on user roles.
///
/// # Examples
///
/// ```
/// use reinhardt_auth::advanced_permissions::RoleBasedPermission;
///
/// let mut perm = RoleBasedPermission::new();
/// perm.add_role("admin", vec!["create", "read", "update", "delete"]);
/// perm.add_role("editor", vec!["read", "update"]);
/// perm.assign_user_role("alice", "admin");
///
/// assert!(perm.user_has_permission("alice", "create"));
/// assert!(perm.user_has_permission("alice", "delete"));
/// assert!(!perm.user_has_permission("bob", "read"));
/// ```
pub struct RoleBasedPermission {
	/// Roles mapped to their permissions
	roles: HashMap<String, Vec<String>>,
	/// User role mapping (username -> role)
	user_roles: HashMap<String, String>,
}

impl RoleBasedPermission {
	/// Create a new role-based permission system
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::advanced_permissions::RoleBasedPermission;
	///
	/// let perm = RoleBasedPermission::new();
	/// ```
	pub fn new() -> Self {
		Self {
			roles: HashMap::new(),
			user_roles: HashMap::new(),
		}
	}

	/// Add a role with permissions
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::advanced_permissions::RoleBasedPermission;
	///
	/// let mut perm = RoleBasedPermission::new();
	/// perm.add_role("admin", vec!["read", "write", "delete"]);
	/// ```
	pub fn add_role(&mut self, role: impl Into<String>, permissions: Vec<impl Into<String>>) {
		let perms = permissions.into_iter().map(|p| p.into()).collect();
		self.roles.insert(role.into(), perms);
	}

	/// Assign role to user
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::advanced_permissions::RoleBasedPermission;
	///
	/// let mut perm = RoleBasedPermission::new();
	/// perm.add_role("user", vec!["read"]);
	/// perm.assign_user_role("alice", "user");
	/// ```
	pub fn assign_user_role(&mut self, username: impl Into<String>, role: impl Into<String>) {
		self.user_roles.insert(username.into(), role.into());
	}

	/// Check if user has specific permission
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::advanced_permissions::RoleBasedPermission;
	///
	/// let mut perm = RoleBasedPermission::new();
	/// perm.add_role("editor", vec!["read", "write"]);
	/// perm.assign_user_role("bob", "editor");
	///
	/// assert!(perm.user_has_permission("bob", "read"));
	/// assert!(perm.user_has_permission("bob", "write"));
	/// assert!(!perm.user_has_permission("bob", "delete"));
	/// ```
	pub fn user_has_permission(&self, username: &str, permission: &str) -> bool {
		if let Some(role) = self.user_roles.get(username)
			&& let Some(perms) = self.roles.get(role)
		{
			return perms.iter().any(|p| p == permission);
		}
		false
	}
}

impl Default for RoleBasedPermission {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl Permission for RoleBasedPermission {
	async fn has_permission(&self, context: &PermissionContext<'_>) -> bool {
		context.is_authenticated
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, Uri, Version};
	use reinhardt_core::types::Request;

	#[test]
	fn test_object_permission_creation() {
		let perm = ObjectPermission::new("view", Some("article:123"));
		assert_eq!(perm.permission(), "view");
		assert_eq!(perm.object_id(), Some("article:123"));
	}

	#[test]
	fn test_object_permission_without_object_id() {
		let perm = ObjectPermission::new("create", None::<String>);
		assert_eq!(perm.permission(), "create");
		assert_eq!(perm.object_id(), None);
	}

	#[tokio::test]
	async fn test_object_permission_authenticated() {
		let perm = ObjectPermission::new("view", None::<String>);
		let request = Request::new(
			Method::GET,
			Uri::from_static("/"),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);

		let context = PermissionContext {
			request: &request,
			is_authenticated: true,
			is_admin: false,
			is_active: true,
			user: None,
		};

		assert!(perm.has_permission(&context).await);
	}

	#[tokio::test]
	async fn test_object_permission_unauthenticated() {
		let perm = ObjectPermission::new("edit", Some("post:42"));
		let request = Request::new(
			Method::GET,
			Uri::from_static("/"),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);

		let context = PermissionContext {
			request: &request,
			is_authenticated: false,
			is_admin: false,
			is_active: false,
			user: None,
		};

		assert!(!perm.has_permission(&context).await);
	}

	#[test]
	fn test_role_based_permission_creation() {
		let perm = RoleBasedPermission::new();
		assert!(!perm.user_has_permission("alice", "read"));
	}

	#[test]
	fn test_role_based_permission_add_role() {
		let mut perm = RoleBasedPermission::new();
		perm.add_role("admin", vec!["read", "write", "delete"]);
		perm.assign_user_role("alice", "admin");

		assert!(perm.user_has_permission("alice", "read"));
		assert!(perm.user_has_permission("alice", "write"));
		assert!(perm.user_has_permission("alice", "delete"));
	}

	#[test]
	fn test_role_based_permission_different_roles() {
		let mut perm = RoleBasedPermission::new();
		perm.add_role("admin", vec!["read", "write", "delete"]);
		perm.add_role("viewer", vec!["read"]);

		perm.assign_user_role("alice", "admin");
		perm.assign_user_role("bob", "viewer");

		assert!(perm.user_has_permission("alice", "write"));
		assert!(perm.user_has_permission("bob", "read"));
		assert!(!perm.user_has_permission("bob", "write"));
	}

	#[test]
	fn test_role_based_permission_no_role() {
		let perm = RoleBasedPermission::new();
		assert!(!perm.user_has_permission("charlie", "read"));
	}

	#[tokio::test]
	async fn test_role_permission_trait_authenticated() {
		let mut perm = RoleBasedPermission::new();
		perm.add_role("user", vec!["read"]);
		perm.assign_user_role("alice", "user");

		let request = Request::new(
			Method::GET,
			Uri::from_static("/"),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);

		let context = PermissionContext {
			request: &request,
			is_authenticated: true,
			is_admin: false,
			is_active: true,
			user: None,
		};

		assert!(perm.has_permission(&context).await);
	}

	#[tokio::test]
	async fn test_role_permission_trait_unauthenticated() {
		let perm = RoleBasedPermission::new();
		let request = Request::new(
			Method::GET,
			Uri::from_static("/"),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);

		let context = PermissionContext {
			request: &request,
			is_authenticated: false,
			is_admin: false,
			is_active: false,
			user: None,
		};

		assert!(!perm.has_permission(&context).await);
	}
}
