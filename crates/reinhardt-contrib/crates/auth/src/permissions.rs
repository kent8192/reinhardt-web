use async_trait::async_trait;
use reinhardt_types::Request;

use crate::user::User;

/// Context for permission checks
pub struct PermissionContext<'a> {
	pub request: &'a Request,
	pub is_authenticated: bool,
	pub is_admin: bool,
	pub is_active: bool,
	pub user: Option<&'a dyn User>,
}

/// Permission trait for authorization checks
#[async_trait]
pub trait Permission: Send + Sync {
	async fn has_permission(&self, context: &PermissionContext<'_>) -> bool;
}

/// Allow any request
pub struct AllowAny;

#[async_trait]
impl Permission for AllowAny {
	async fn has_permission(&self, _context: &PermissionContext<'_>) -> bool {
		true
	}
}

/// Require authenticated user
pub struct IsAuthenticated;

#[async_trait]
impl Permission for IsAuthenticated {
	async fn has_permission(&self, context: &PermissionContext<'_>) -> bool {
		context.is_authenticated
	}
}

/// Require admin user
pub struct IsAdminUser;

#[async_trait]
impl Permission for IsAdminUser {
	async fn has_permission(&self, context: &PermissionContext<'_>) -> bool {
		context.is_authenticated && context.is_admin
	}
}

/// Require active user
pub struct IsActiveUser;

#[async_trait]
impl Permission for IsActiveUser {
	async fn has_permission(&self, context: &PermissionContext<'_>) -> bool {
		context.is_authenticated && context.is_active
	}
}

/// Authenticated for write, read-only for unauthenticated
pub struct IsAuthenticatedOrReadOnly;

#[async_trait]
impl Permission for IsAuthenticatedOrReadOnly {
	async fn has_permission(&self, context: &PermissionContext<'_>) -> bool {
		if context.is_authenticated {
			return true;
		}

		// Allow GET, HEAD, OPTIONS for unauthenticated users
		matches!(context.request.method.as_str(), "GET" | "HEAD" | "OPTIONS")
	}
}
