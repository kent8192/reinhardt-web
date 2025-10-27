//! Authentication and authorization integration for admin
//!
//! This module integrates with reinhardt-auth to provide permission checking
//! for admin operations.

use crate::{AdminError, AdminResult};
use reinhardt_auth::{DjangoModelPermissions, IsAdminUser, Permission, User};
use std::any::Any;
use std::sync::Arc;

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
/// ```
/// use reinhardt_admin::{AdminAuthBackend, PermissionAction};
/// use reinhardt_auth::User;
///
/// # async fn example() {
/// let auth = AdminAuthBackend::new();
/// let user = User {
///     id: 1,
///     username: "admin".to_string(),
///     is_staff: true,
///     is_superuser: true,
///     ..Default::default()
/// };
///
/// let can_view = auth.check_permission(&user, "User", PermissionAction::View).await;
/// # }
/// ```
pub struct AdminAuthBackend {
    model_permissions: Arc<DjangoModelPermissions>,
    admin_checker: Arc<IsAdminUser>,
}

impl AdminAuthBackend {
    /// Create a new admin auth backend
    pub fn new() -> Self {
        Self {
            model_permissions: Arc::new(DjangoModelPermissions::new()),
            admin_checker: Arc::new(IsAdminUser),
        }
    }

    /// Check if user has permission for a model action
    ///
    /// # Examples
    ///
    /// ```
    /// # use reinhardt_admin::{AdminAuthBackend, PermissionAction};
    /// # use reinhardt_auth::User;
    /// # async fn example() {
    /// let auth = AdminAuthBackend::new();
    /// let user = User {
    ///     id: 1,
    ///     username: "admin".to_string(),
    ///     is_staff: true,
    ///     is_superuser: false,
    ///     ..Default::default()
    /// };
    ///
    /// let can_change = auth.check_permission(&user, "Article", PermissionAction::Change).await;
    /// # }
    /// ```
    pub async fn check_permission(
        &self,
        user: &User,
        model: &str,
        action: PermissionAction,
    ) -> bool {
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

        // TODO: Implement actual permission check with PermissionContext
        // For now, grant all permissions to staff users
        true
    }

    /// Check if user is admin (staff member)
    pub async fn is_admin(&self, user: &User) -> bool {
        user.is_staff || user.is_superuser
    }

    /// Check if user is superuser
    pub fn is_superuser(&self, user: &User) -> bool {
        user.is_superuser
    }

    /// Extract User from Any type
    pub fn extract_user(user: &dyn Any) -> Option<&User> {
        user.downcast_ref::<User>()
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
    user: User,
}

impl AdminPermissionChecker {
    /// Create a new permission checker for a user
    pub fn new(user: User) -> Self {
        Self {
            backend: Arc::new(AdminAuthBackend::new()),
            user,
        }
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
        self.backend.is_admin(&self.user).await
    }

    /// Check if user is superuser
    pub fn is_superuser(&self) -> bool {
        self.backend.is_superuser(&self.user)
    }

    /// Get the user
    pub fn user(&self) -> &User {
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
    pub async fn check_access(&self, user: &User) -> AdminResult<()> {
        if !self.backend.is_admin(user).await {
            return Err(AdminError::PermissionDenied(
                "User is not a staff member".to_string(),
            ));
        }
        Ok(())
    }

    /// Verify user for admin access
    pub async fn verify_admin_user(&self, user: &dyn Any) -> AdminResult<&User> {
        let user = AdminAuthBackend::extract_user(user).ok_or_else(|| {
            AdminError::PermissionDenied("Invalid user type".to_string())
        })?;

        self.check_access(user).await?;
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

    fn create_staff_user() -> User {
        User {
            id: 1,
            username: "staff".to_string(),
            email: Some("staff@example.com".to_string()),
            is_staff: true,
            is_superuser: false,
            is_active: true,
            ..Default::default()
        }
    }

    fn create_superuser() -> User {
        User {
            id: 2,
            username: "admin".to_string(),
            email: Some("admin@example.com".to_string()),
            is_staff: true,
            is_superuser: true,
            is_active: true,
            ..Default::default()
        }
    }

    fn create_regular_user() -> User {
        User {
            id: 3,
            username: "user".to_string(),
            email: Some("user@example.com".to_string()),
            is_staff: false,
            is_superuser: false,
            is_active: true,
            ..Default::default()
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

        assert!(auth.check_permission(&user, "User", PermissionAction::View).await);
        assert!(auth.check_permission(&user, "User", PermissionAction::Add).await);
        assert!(auth.check_permission(&user, "User", PermissionAction::Change).await);
        assert!(auth.check_permission(&user, "User", PermissionAction::Delete).await);
    }

    #[tokio::test]
    async fn test_regular_user_no_admin_access() {
        let auth = AdminAuthBackend::new();
        let user = create_regular_user();

        assert!(!auth.check_permission(&user, "User", PermissionAction::View).await);
        assert!(!auth.is_admin(&user).await);
    }

    #[tokio::test]
    async fn test_staff_user_admin_access() {
        let auth = AdminAuthBackend::new();
        let user = create_staff_user();

        assert!(auth.is_admin(&user).await);
        assert!(!auth.is_superuser(&user));
    }

    #[tokio::test]
    async fn test_permission_checker() {
        let user = create_staff_user();
        let checker = AdminPermissionChecker::new(user);

        assert!(checker.can_view("Article").await);
        assert!(checker.can_add("Article").await);
        assert!(checker.is_admin().await);
        assert!(!checker.is_superuser());
    }

    #[tokio::test]
    async fn test_admin_middleware_staff_access() {
        let middleware = AdminAuthMiddleware::new();
        let user = create_staff_user();

        let result = middleware.check_access(&user).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_admin_middleware_regular_user_denied() {
        let middleware = AdminAuthMiddleware::new();
        let user = create_regular_user();

        let result = middleware.check_access(&user).await;
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
}
