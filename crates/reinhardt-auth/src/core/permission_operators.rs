//! Permission Operators
//!
//! Provides logical operators (AND, OR, NOT) for composing permissions.
//! Supports both builder-style composition (`AndPermission::new()`) and
//! operator-based composition (`&`, `|`, `!`).

use crate::core::{Permission, PermissionContext};
use async_trait::async_trait;
use std::ops::{BitAnd, BitOr, Not};

/// AND permission operator
///
/// Combines two permissions with logical AND. Both permissions must be satisfied.
///
/// # Examples
///
/// ```
/// use reinhardt_core_auth::permission_operators::AndPermission;
/// use reinhardt_core_auth::permission::{IsAuthenticated, IsAdminUser, Permission, PermissionContext};
/// use bytes::Bytes;
/// use hyper::{HeaderMap, Method, Version};
/// use reinhardt_types::Request;
///
/// #[tokio::main]
/// async fn main() {
///     let permission = AndPermission::new(IsAuthenticated, IsAdminUser);
/// let request = Request::builder()
///     .method(Method::GET)
///     .uri("/")
///     .version(Version::HTTP_11)
///     .headers(HeaderMap::new())
///     .body(Bytes::new())
///     .build()
///     .unwrap();
///
///     // Both authenticated AND admin required
///     let context = PermissionContext {
///         request: &request,
///         is_authenticated: true,
///         is_admin: true,
///         is_active: true,
///         user: None,
///     };
///     assert!(permission.has_permission(&context).await);
///
///     // Not admin - fails
///     let context = PermissionContext {
///         request: &request,
///         is_authenticated: true,
///         is_admin: false,
///         is_active: true,
///         user: None,
///     };
///     assert!(!permission.has_permission(&context).await);
/// }
/// ```
pub struct AndPermission<A, B> {
	left: A,
	right: B,
}

impl<A, B> AndPermission<A, B> {
	/// Create a new AND permission
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core_auth::permission_operators::AndPermission;
	/// use reinhardt_core_auth::permission::{IsAuthenticated, IsActiveUser};
	///
	/// let permission = AndPermission::new(IsAuthenticated, IsActiveUser);
	/// ```
	pub fn new(left: A, right: B) -> Self {
		Self { left, right }
	}
}

#[async_trait]
impl<A, B> Permission for AndPermission<A, B>
where
	A: Permission + Send + Sync,
	B: Permission + Send + Sync,
{
	async fn has_permission(&self, context: &PermissionContext<'_>) -> bool {
		self.left.has_permission(context).await && self.right.has_permission(context).await
	}
}

/// OR permission operator
///
/// Combines two permissions with logical OR. Either permission can be satisfied.
///
/// # Examples
///
/// ```
/// use reinhardt_core_auth::permission_operators::OrPermission;
/// use reinhardt_core_auth::permission::{IsAuthenticated, AllowAny, Permission, PermissionContext};
/// use bytes::Bytes;
/// use hyper::{HeaderMap, Method, Version};
/// use reinhardt_types::Request;
///
/// #[tokio::main]
/// async fn main() {
///     let permission = OrPermission::new(IsAuthenticated, AllowAny);
/// let request = Request::builder()
///     .method(Method::GET)
///     .uri("/")
///     .version(Version::HTTP_11)
///     .headers(HeaderMap::new())
///     .body(Bytes::new())
///     .build()
///     .unwrap();
///
///     // Either authenticated OR allow any
///     let context = PermissionContext {
///         request: &request,
///         is_authenticated: false,
///         is_admin: false,
///         is_active: false,
///         user: None,
///     };
///     assert!(permission.has_permission(&context).await);
/// }
/// ```
pub struct OrPermission<A, B> {
	left: A,
	right: B,
}

impl<A, B> OrPermission<A, B> {
	/// Create a new OR permission
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core_auth::permission_operators::OrPermission;
	/// use reinhardt_core_auth::permission::{IsAdminUser, IsActiveUser};
	///
	/// let permission = OrPermission::new(IsAdminUser, IsActiveUser);
	/// ```
	pub fn new(left: A, right: B) -> Self {
		Self { left, right }
	}
}

#[async_trait]
impl<A, B> Permission for OrPermission<A, B>
where
	A: Permission + Send + Sync,
	B: Permission + Send + Sync,
{
	async fn has_permission(&self, context: &PermissionContext<'_>) -> bool {
		self.left.has_permission(context).await || self.right.has_permission(context).await
	}
}

/// NOT permission operator
///
/// Negates a permission. Returns true if the inner permission is false.
///
/// # Examples
///
/// ```
/// use reinhardt_core_auth::permission_operators::NotPermission;
/// use reinhardt_core_auth::permission::{IsAuthenticated, Permission, PermissionContext};
/// use bytes::Bytes;
/// use hyper::{HeaderMap, Method, Version};
/// use reinhardt_types::Request;
///
/// #[tokio::main]
/// async fn main() {
///     let permission = NotPermission::new(IsAuthenticated);
/// let request = Request::builder()
///     .method(Method::GET)
///     .uri("/")
///     .version(Version::HTTP_11)
///     .headers(HeaderMap::new())
///     .body(Bytes::new())
///     .build()
///     .unwrap();
///
///     // NOT authenticated - only allows unauthenticated users
///     let context = PermissionContext {
///         request: &request,
///         is_authenticated: false,
///         is_admin: false,
///         is_active: false,
///         user: None,
///     };
///     assert!(permission.has_permission(&context).await);
///
///     // Authenticated - denies
///     let context = PermissionContext {
///         request: &request,
///         is_authenticated: true,
///         is_admin: false,
///         is_active: true,
///         user: None,
///     };
///     assert!(!permission.has_permission(&context).await);
/// }
/// ```
pub struct NotPermission<P> {
	inner: P,
}

impl<P> NotPermission<P> {
	/// Create a new NOT permission
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core_auth::permission_operators::NotPermission;
	/// use reinhardt_core_auth::permission::IsAdminUser;
	///
	/// let permission = NotPermission::new(IsAdminUser);
	/// ```
	pub fn new(inner: P) -> Self {
		Self { inner }
	}
}

#[async_trait]
impl<P> Permission for NotPermission<P>
where
	P: Permission + Send + Sync,
{
	async fn has_permission(&self, context: &PermissionContext<'_>) -> bool {
		!self.inner.has_permission(context).await
	}
}

// Operator overloading implementations using macros
//
// Due to Rust's orphan rules, we need to implement operators for each concrete permission type.
// This macro makes it easy to add operator support for new permission types.

/// Macro to implement BitAnd, BitOr, and Not operators for permission types.
///
/// This allows natural composition syntax like:
/// ```ignore
/// let permission = IsAuthenticated & IsActiveUser;
/// let permission = IsAdminUser | IsActiveUser;
/// let permission = !IsAuthenticated;
/// ```
///
/// The macro generates implementations for `&`, `|`, and `!` operators
/// that create the appropriate `AndPermission`, `OrPermission`, and `NotPermission` types.
macro_rules! impl_permission_operators {
	($type:ty) => {
		impl<B: Permission> BitAnd<B> for $type {
			type Output = AndPermission<Self, B>;

			fn bitand(self, rhs: B) -> Self::Output {
				AndPermission::new(self, rhs)
			}
		}

		impl<B: Permission> BitOr<B> for $type {
			type Output = OrPermission<Self, B>;

			fn bitor(self, rhs: B) -> Self::Output {
				OrPermission::new(self, rhs)
			}
		}

		impl Not for $type {
			type Output = NotPermission<Self>;

			fn not(self) -> Self::Output {
				NotPermission::new(self)
			}
		}
	};
}

// Apply operators to all built-in permission types
impl_permission_operators!(crate::AllowAny);
impl_permission_operators!(crate::IsAuthenticated);
impl_permission_operators!(crate::IsAdminUser);
impl_permission_operators!(crate::IsActiveUser);
impl_permission_operators!(crate::IsAuthenticatedOrReadOnly);

// Apply operators to composite permission types to allow chaining
impl<A, B, C> BitAnd<C> for AndPermission<A, B>
where
	A: Permission,
	B: Permission,
	C: Permission,
{
	type Output = AndPermission<Self, C>;

	fn bitand(self, rhs: C) -> Self::Output {
		AndPermission::new(self, rhs)
	}
}

impl<A, B, C> BitOr<C> for AndPermission<A, B>
where
	A: Permission,
	B: Permission,
	C: Permission,
{
	type Output = OrPermission<Self, C>;

	fn bitor(self, rhs: C) -> Self::Output {
		OrPermission::new(self, rhs)
	}
}

impl<A, B> Not for AndPermission<A, B>
where
	A: Permission,
	B: Permission,
{
	type Output = NotPermission<Self>;

	fn not(self) -> Self::Output {
		NotPermission::new(self)
	}
}

impl<A, B, C> BitAnd<C> for OrPermission<A, B>
where
	A: Permission,
	B: Permission,
	C: Permission,
{
	type Output = AndPermission<Self, C>;

	fn bitand(self, rhs: C) -> Self::Output {
		AndPermission::new(self, rhs)
	}
}

impl<A, B, C> BitOr<C> for OrPermission<A, B>
where
	A: Permission,
	B: Permission,
	C: Permission,
{
	type Output = OrPermission<Self, C>;

	fn bitor(self, rhs: C) -> Self::Output {
		OrPermission::new(self, rhs)
	}
}

impl<A, B> Not for OrPermission<A, B>
where
	A: Permission,
	B: Permission,
{
	type Output = NotPermission<Self>;

	fn not(self) -> Self::Output {
		NotPermission::new(self)
	}
}

impl<P, B> BitAnd<B> for NotPermission<P>
where
	P: Permission,
	B: Permission,
{
	type Output = AndPermission<Self, B>;

	fn bitand(self, rhs: B) -> Self::Output {
		AndPermission::new(self, rhs)
	}
}

impl<P, B> BitOr<B> for NotPermission<P>
where
	P: Permission,
	B: Permission,
{
	type Output = OrPermission<Self, B>;

	fn bitor(self, rhs: B) -> Self::Output {
		OrPermission::new(self, rhs)
	}
}

impl<P> Not for NotPermission<P>
where
	P: Permission,
{
	type Output = NotPermission<Self>;

	fn not(self) -> Self::Output {
		NotPermission::new(self)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::core::{AllowAny, IsAdminUser, IsAuthenticated};
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, Version};
	use reinhardt_types::Request;

	#[tokio::test]
	async fn test_and_permission_both_true() {
		let permission = AndPermission::new(IsAuthenticated, IsAdminUser);
		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let context = PermissionContext {
			request: &request,
			is_authenticated: true,
			is_admin: true,
			is_active: true,
			user: None,
		};

		assert!(permission.has_permission(&context).await);
	}

	#[tokio::test]
	async fn test_and_permission_left_false() {
		let permission = AndPermission::new(IsAuthenticated, IsAdminUser);
		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let context = PermissionContext {
			request: &request,
			is_authenticated: false,
			is_admin: true,
			is_active: false,
			user: None,
		};

		assert!(!permission.has_permission(&context).await);
	}

	#[tokio::test]
	async fn test_and_permission_right_false() {
		let permission = AndPermission::new(IsAuthenticated, IsAdminUser);
		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let context = PermissionContext {
			request: &request,
			is_authenticated: true,
			is_admin: false,
			is_active: true,
			user: None,
		};

		assert!(!permission.has_permission(&context).await);
	}

	#[tokio::test]
	async fn test_or_permission_both_true() {
		let permission = OrPermission::new(IsAuthenticated, AllowAny);
		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let context = PermissionContext {
			request: &request,
			is_authenticated: true,
			is_admin: false,
			is_active: true,
			user: None,
		};

		assert!(permission.has_permission(&context).await);
	}

	#[tokio::test]
	async fn test_or_permission_left_true() {
		let permission = OrPermission::new(IsAuthenticated, IsAdminUser);
		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let context = PermissionContext {
			request: &request,
			is_authenticated: true,
			is_admin: false,
			is_active: true,
			user: None,
		};

		assert!(permission.has_permission(&context).await);
	}

	#[tokio::test]
	async fn test_or_permission_right_true() {
		let permission = OrPermission::new(IsAuthenticated, AllowAny);
		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let context = PermissionContext {
			request: &request,
			is_authenticated: false,
			is_admin: false,
			is_active: false,
			user: None,
		};

		assert!(permission.has_permission(&context).await);
	}

	#[tokio::test]
	async fn test_or_permission_both_false() {
		let permission = OrPermission::new(IsAuthenticated, IsAdminUser);
		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let context = PermissionContext {
			request: &request,
			is_authenticated: false,
			is_admin: false,
			is_active: false,
			user: None,
		};

		assert!(!permission.has_permission(&context).await);
	}

	#[tokio::test]
	async fn test_not_permission_true() {
		let permission = NotPermission::new(IsAuthenticated);
		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let context = PermissionContext {
			request: &request,
			is_authenticated: false,
			is_admin: false,
			is_active: false,
			user: None,
		};

		assert!(permission.has_permission(&context).await);
	}

	#[tokio::test]
	async fn test_not_permission_false() {
		let permission = NotPermission::new(IsAuthenticated);
		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let context = PermissionContext {
			request: &request,
			is_authenticated: true,
			is_admin: false,
			is_active: true,
			user: None,
		};

		assert!(!permission.has_permission(&context).await);
	}

	#[tokio::test]
	async fn test_complex_permission_combination() {
		let permission =
			OrPermission::new(AndPermission::new(IsAuthenticated, IsAdminUser), AllowAny);

		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let context = PermissionContext {
			request: &request,
			is_authenticated: false,
			is_admin: false,
			is_active: false,
			user: None,
		};

		assert!(permission.has_permission(&context).await);
	}

	// Tests for operator overloading

	#[tokio::test]
	async fn test_bitand_operator() {
		let permission = IsAuthenticated & IsAdminUser;

		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		// Both conditions met
		let context = PermissionContext {
			request: &request,
			is_authenticated: true,
			is_admin: true,
			is_active: true,
			user: None,
		};
		assert!(permission.has_permission(&context).await);

		// Only authenticated, not admin
		let context = PermissionContext {
			request: &request,
			is_authenticated: true,
			is_admin: false,
			is_active: true,
			user: None,
		};
		assert!(!permission.has_permission(&context).await);
	}

	#[tokio::test]
	async fn test_bitor_operator() {
		let permission = IsAuthenticated | AllowAny;

		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		// Not authenticated, but AllowAny should allow
		let context = PermissionContext {
			request: &request,
			is_authenticated: false,
			is_admin: false,
			is_active: false,
			user: None,
		};
		assert!(permission.has_permission(&context).await);
	}

	#[tokio::test]
	async fn test_not_operator() {
		let permission = !IsAuthenticated;

		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		// Not authenticated - should be allowed
		let context = PermissionContext {
			request: &request,
			is_authenticated: false,
			is_admin: false,
			is_active: false,
			user: None,
		};
		assert!(permission.has_permission(&context).await);

		// Authenticated - should be denied
		let context = PermissionContext {
			request: &request,
			is_authenticated: true,
			is_admin: false,
			is_active: true,
			user: None,
		};
		assert!(!permission.has_permission(&context).await);
	}

	#[tokio::test]
	async fn test_complex_operator_combination() {
		// (IsAuthenticated & IsAdminUser) | AllowAny
		let permission = (IsAuthenticated & IsAdminUser) | AllowAny;

		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		// Not authenticated, but AllowAny allows
		let context = PermissionContext {
			request: &request,
			is_authenticated: false,
			is_admin: false,
			is_active: false,
			user: None,
		};
		assert!(permission.has_permission(&context).await);
	}
}
