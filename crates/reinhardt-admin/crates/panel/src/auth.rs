//! Authentication and authorization integration for admin
//!
//! This module integrates with reinhardt-auth to provide permission checking
//! for admin operations.

use crate::{AdminError, AdminResult};
use reinhardt_auth::{DjangoModelPermissions, SimpleUser, User};
use std::any::Any;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Permission action types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionAction {
	/// View permission
	View,
	/// Add permission
	Add,
	/// Change permission
	Change,
	/// Delete permission
	Delete,
}

impl PermissionAction {
	/// Get the action string for Django-style permissions
	pub fn as_str(&self) -> &'static str {
		match self {
			PermissionAction::View => "view",
			PermissionAction::Add => "add",
			PermissionAction::Change => "change",
			PermissionAction::Delete => "delete",
		}
	}
}

/// Admin authentication backend
///
/// Provides permission checking integrated with reinhardt-auth.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_panel::{AdminAuthBackend, PermissionAction};
/// use reinhardt_auth::SimpleUser;
///
/// # async fn example() {
/// let auth = AdminAuthBackend::new();
/// // Create a user (requires uuid crate in your dependencies)
/// // let user = SimpleUser { ... };
/// // let can_view = auth.check_permission(&user, "User", PermissionAction::View).await;
/// # }
/// ```
pub struct AdminAuthBackend {
	model_permissions: Arc<RwLock<DjangoModelPermissions>>,
}

impl AdminAuthBackend {
	/// Create a new admin auth backend
	pub fn new() -> Self {
		Self {
			model_permissions: Arc::new(RwLock::new(DjangoModelPermissions::new())),
		}
	}

	/// Check if user has permission for a model action
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_panel::{AdminAuthBackend, PermissionAction};
	/// # use reinhardt_auth::SimpleUser;
	/// # async fn example() {
	/// let auth = AdminAuthBackend::new();
	/// // Create a user (requires uuid crate in your dependencies)
	/// // let user = SimpleUser { ... };
	/// // let can_change = auth.check_permission(&user, "Article", PermissionAction::Change).await;
	/// # }
	/// ```
	pub async fn check_permission(
		&self,
		user: &SimpleUser,
		model: &str,
		action: PermissionAction,
	) -> bool {
		// Only authenticated users can have permissions
		if !user.is_authenticated() {
			return false;
		}

		// Inactive users have no permissions
		if !user.is_active {
			return false;
		}

		// Superusers have all permissions
		if user.is_superuser {
			return true;
		}

		// Staff users need specific model permissions
		if !user.is_staff {
			return false;
		}

		// Check Django-style model permission: "admin.action_model"
		let permission = format!("admin.{}_{}", action.as_str(), model.to_lowercase());

		// Check permission using DjangoModelPermissions
		self.model_permissions
			.read()
			.await
			.user_has_permission(&user.username, &permission)
			.await
	}

	/// Check if user is admin (staff member)
	pub async fn is_admin(&self, user: &dyn User) -> bool {
		// Use the trait's is_admin method
		user.is_admin()
	}

	/// Check if user is superuser
	pub fn is_superuser(&self, user: &dyn User) -> bool {
		user.is_superuser()
	}

	/// Add permission to a user
	///
	/// Grants a specific permission to the user. The permission should be in
	/// Django-style format: "admin.action_model" (e.g., "admin.view_user", "admin.change_article").
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_panel::{AdminAuthBackend, PermissionAction};
	/// use reinhardt_auth::SimpleUser;
	///
	/// # async fn example() {
	/// let mut auth = AdminAuthBackend::new();
	/// auth.add_user_permission("alice", "admin.view_user").await;
	/// auth.add_user_permission("alice", "admin.change_article").await;
	/// # }
	/// ```
	pub async fn add_user_permission(&self, username: &str, permission: &str) {
		let mut perms = self.model_permissions.write().await;
		perms.add_user_permission(username, permission);
	}

	/// Create AdminAuthBackend with pre-configured permissions
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_panel::AdminAuthBackend;
	/// use std::collections::HashMap;
	///
	/// # async fn example() {
	/// let mut permissions = HashMap::new();
	/// permissions.insert("alice".to_string(), vec!["admin.view_user".to_string()]);
	/// permissions.insert("bob".to_string(), vec!["admin.change_article".to_string()]);
	///
	/// let auth = AdminAuthBackend::with_permissions(permissions).await;
	/// # }
	/// ```
	pub async fn with_permissions(
		permissions: std::collections::HashMap<String, Vec<String>>,
	) -> Self {
		let backend = Self::new();

		for (username, perms) in permissions {
			for permission in perms {
				backend.add_user_permission(&username, &permission).await;
			}
		}

		backend
	}

	/// Extract SimpleUser from Any type
	///
	/// This is a helper method used by AdminAuthMiddleware to safely downcast
	/// a user object to SimpleUser type.
	pub fn extract_user(user: &dyn Any) -> Option<&SimpleUser> {
		user.downcast_ref::<SimpleUser>()
	}
}

impl Default for AdminAuthBackend {
	fn default() -> Self {
		Self::new()
	}
}

/// Admin permission checker
///
/// Provides convenient methods for checking permissions in admin views.
pub struct AdminPermissionChecker {
	backend: Arc<AdminAuthBackend>,
	user: SimpleUser,
}

impl AdminPermissionChecker {
	/// Create a new permission checker for a user
	pub fn new(user: SimpleUser) -> Self {
		Self {
			backend: Arc::new(AdminAuthBackend::new()),
			user,
		}
	}

	/// Create a permission checker with a custom backend
	pub fn with_backend(user: SimpleUser, backend: Arc<AdminAuthBackend>) -> Self {
		Self { backend, user }
	}

	/// Check view permission
	pub async fn can_view(&self, model: &str) -> bool {
		self.backend
			.check_permission(&self.user, model, PermissionAction::View)
			.await
	}

	/// Check add permission
	pub async fn can_add(&self, model: &str) -> bool {
		self.backend
			.check_permission(&self.user, model, PermissionAction::Add)
			.await
	}

	/// Check change permission
	pub async fn can_change(&self, model: &str) -> bool {
		self.backend
			.check_permission(&self.user, model, PermissionAction::Change)
			.await
	}

	/// Check delete permission
	pub async fn can_delete(&self, model: &str) -> bool {
		self.backend
			.check_permission(&self.user, model, PermissionAction::Delete)
			.await
	}

	/// Check if user is admin
	pub async fn is_admin(&self) -> bool {
		self.backend.is_admin(&self.user as &dyn User).await
	}

	/// Check if user is superuser
	pub fn is_superuser(&self) -> bool {
		self.backend.is_superuser(&self.user as &dyn User)
	}

	/// Get the user
	pub fn user(&self) -> &SimpleUser {
		&self.user
	}
}

/// Admin authentication middleware
///
/// Ensures that only authenticated staff users can access admin views.
pub struct AdminAuthMiddleware {
	backend: Arc<AdminAuthBackend>,
}

impl AdminAuthMiddleware {
	/// Create a new admin auth middleware
	pub fn new() -> Self {
		Self {
			backend: Arc::new(AdminAuthBackend::new()),
		}
	}

	/// Check if user can access admin
	pub async fn check_access(&self, user: &dyn User) -> AdminResult<()> {
		if !self.backend.is_admin(user).await {
			return Err(AdminError::PermissionDenied(
				"User is not a staff member".to_string(),
			));
		}
		Ok(())
	}

	/// Verify user for admin access
	pub async fn verify_admin_user<'a>(&self, user: &'a dyn Any) -> AdminResult<&'a SimpleUser> {
		let user = AdminAuthBackend::extract_user(user)
			.ok_or_else(|| AdminError::PermissionDenied("Invalid user type".to_string()))?;

		self.check_access(user as &dyn User).await?;
		Ok(user)
	}
}

impl Default for AdminAuthMiddleware {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn create_staff_user() -> SimpleUser {
		SimpleUser {
			id: uuid::Uuid::from_u128(1),
			username: "staff".to_string(),
			email: "staff@example.com".to_string(),
			is_staff: true,
			is_superuser: false,
			is_active: true,
			is_admin: true,
		}
	}

	fn create_superuser() -> SimpleUser {
		SimpleUser {
			id: uuid::Uuid::from_u128(2),
			username: "admin".to_string(),
			email: "admin@example.com".to_string(),
			is_staff: true,
			is_superuser: true,
			is_active: true,
			is_admin: true,
		}
	}

	fn create_regular_user() -> SimpleUser {
		SimpleUser {
			id: uuid::Uuid::from_u128(3),
			username: "user".to_string(),
			email: "user@example.com".to_string(),
			is_staff: false,
			is_superuser: false,
			is_active: true,
			is_admin: false,
		}
	}

	#[test]
	fn test_permission_action_as_str() {
		assert_eq!(PermissionAction::View.as_str(), "view");
		assert_eq!(PermissionAction::Add.as_str(), "add");
		assert_eq!(PermissionAction::Change.as_str(), "change");
		assert_eq!(PermissionAction::Delete.as_str(), "delete");
	}

	#[tokio::test]
	async fn test_superuser_has_all_permissions() {
		let auth = AdminAuthBackend::new();
		let user = create_superuser();

		assert!(
			auth.check_permission(&user, "User", PermissionAction::View)
				.await
		);
		assert!(
			auth.check_permission(&user, "User", PermissionAction::Add)
				.await
		);
		assert!(
			auth.check_permission(&user, "User", PermissionAction::Change)
				.await
		);
		assert!(
			auth.check_permission(&user, "User", PermissionAction::Delete)
				.await
		);
	}

	#[tokio::test]
	async fn test_regular_user_no_admin_access() {
		let auth = AdminAuthBackend::new();
		let user = create_regular_user();

		assert!(
			!auth
				.check_permission(&user, "User", PermissionAction::View)
				.await
		);
		assert!(!auth.is_admin(&user as &dyn User).await);
	}

	#[tokio::test]
	async fn test_staff_user_admin_access() {
		let auth = AdminAuthBackend::new();
		let user = create_staff_user();

		assert!(auth.is_admin(&user as &dyn User).await);
		assert!(!auth.is_superuser(&user as &dyn User));
	}

	#[tokio::test(flavor = "multi_thread")]
	async fn test_permission_checker() {
		let user = create_staff_user();
		let backend = Arc::new(AdminAuthBackend::new());

		// Grant permissions to staff user
		backend
			.add_user_permission("staff", "admin.view_article")
			.await;
		backend
			.add_user_permission("staff", "admin.add_article")
			.await;

		let checker = AdminPermissionChecker::with_backend(user, backend);

		assert!(checker.can_view("Article").await);
		assert!(checker.can_add("Article").await);
		assert!(checker.is_admin().await);
		assert!(!checker.is_superuser());
	}

	#[tokio::test]
	async fn test_admin_middleware_staff_access() {
		let middleware = AdminAuthMiddleware::new();
		let user = create_staff_user();

		let result = middleware.check_access(&user as &dyn User).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_admin_middleware_regular_user_denied() {
		let middleware = AdminAuthMiddleware::new();
		let user = create_regular_user();

		let result = middleware.check_access(&user as &dyn User).await;
		assert!(result.is_err());

		if let Err(AdminError::PermissionDenied(msg)) = result {
			assert!(msg.contains("not a staff member"));
		} else {
			panic!("Expected PermissionDenied error");
		}
	}

	#[test]
	fn test_extract_user() {
		let user = create_staff_user();
		let any_user: &dyn Any = &user;

		let extracted = AdminAuthBackend::extract_user(any_user);
		assert!(extracted.is_some());
		assert_eq!(extracted.unwrap().username, "staff");
	}

	#[tokio::test(flavor = "multi_thread")]
	async fn test_staff_user_with_permission_allowed() {
		let auth = AdminAuthBackend::new();
		let user = create_staff_user();

		// Grant specific permissions to staff user
		auth.add_user_permission("staff", "admin.view_user").await;
		auth.add_user_permission("staff", "admin.change_article")
			.await;

		// Should have granted permissions
		assert!(
			auth.check_permission(&user, "User", PermissionAction::View)
				.await
		);
		assert!(
			auth.check_permission(&user, "Article", PermissionAction::Change)
				.await
		);

		// Should not have un-granted permissions
		assert!(
			!auth
				.check_permission(&user, "User", PermissionAction::Delete)
				.await
		);
	}

	#[tokio::test]
	async fn test_staff_user_without_permission_denied() {
		let auth = AdminAuthBackend::new();
		let user = create_staff_user();

		// No permissions granted

		// Should be denied for all actions
		assert!(
			!auth
				.check_permission(&user, "User", PermissionAction::View)
				.await
		);
		assert!(
			!auth
				.check_permission(&user, "User", PermissionAction::Add)
				.await
		);
		assert!(
			!auth
				.check_permission(&user, "User", PermissionAction::Change)
				.await
		);
		assert!(
			!auth
				.check_permission(&user, "User", PermissionAction::Delete)
				.await
		);
	}

	#[tokio::test(flavor = "multi_thread")]
	async fn test_staff_user_partial_permissions() {
		let auth = AdminAuthBackend::new();
		let user = create_staff_user();

		// Grant only view and change permissions
		auth.add_user_permission("staff", "admin.view_article")
			.await;
		auth.add_user_permission("staff", "admin.change_article")
			.await;

		// Should have view and change
		assert!(
			auth.check_permission(&user, "Article", PermissionAction::View)
				.await
		);
		assert!(
			auth.check_permission(&user, "Article", PermissionAction::Change)
				.await
		);

		// Should not have add and delete
		assert!(
			!auth
				.check_permission(&user, "Article", PermissionAction::Add)
				.await
		);
		assert!(
			!auth
				.check_permission(&user, "Article", PermissionAction::Delete)
				.await
		);
	}

	#[tokio::test(flavor = "multi_thread")]
	async fn test_with_permissions_constructor() {
		use std::collections::HashMap;

		let mut permissions = HashMap::new();
		permissions.insert(
			"alice".to_string(),
			vec![
				"admin.view_user".to_string(),
				"admin.change_user".to_string(),
			],
		);
		permissions.insert("bob".to_string(), vec!["admin.view_article".to_string()]);

		let auth = AdminAuthBackend::with_permissions(permissions).await;

		// Create users
		let alice = SimpleUser {
			id: uuid::Uuid::from_u128(10),
			username: "alice".to_string(),
			email: "alice@example.com".to_string(),
			is_staff: true,
			is_superuser: false,
			is_active: true,
			is_admin: true,
		};

		let bob = SimpleUser {
			id: uuid::Uuid::from_u128(11),
			username: "bob".to_string(),
			email: "bob@example.com".to_string(),
			is_staff: true,
			is_superuser: false,
			is_active: true,
			is_admin: true,
		};

		// Alice should have user permissions
		assert!(
			auth.check_permission(&alice, "User", PermissionAction::View)
				.await
		);
		assert!(
			auth.check_permission(&alice, "User", PermissionAction::Change)
				.await
		);
		assert!(
			!auth
				.check_permission(&alice, "User", PermissionAction::Delete)
				.await
		);

		// Bob should have article view permission
		assert!(
			auth.check_permission(&bob, "Article", PermissionAction::View)
				.await
		);
		assert!(
			!auth
				.check_permission(&bob, "Article", PermissionAction::Change)
				.await
		);
	}
}
