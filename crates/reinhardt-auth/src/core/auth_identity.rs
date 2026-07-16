/// Authentication identity trait - replacement for the deprecated `User` trait.
///
/// Provides identity and authentication status methods. Use with
/// `BaseUser`/`FullUser` +
/// `PermissionsMixin` for full user functionality.
///
/// # Examples
///
/// ```
/// use reinhardt_auth::AuthIdentity;
///
/// struct MyUser {
///     id: i64,
///     is_superuser: bool,
/// }
///
/// impl AuthIdentity for MyUser {
///     fn id(&self) -> String { self.id.to_string() }
///     fn is_authenticated(&self) -> bool { true }
///     fn is_admin(&self) -> bool { self.is_superuser }
///     fn is_account_active(&self) -> bool { true }
/// }
///
/// let user = MyUser { id: 1, is_superuser: false };
/// assert!(user.is_authenticated());
/// assert!(!user.is_admin());
/// assert_eq!(user.id(), "1");
/// ```
pub trait AuthIdentity: Send + Sync {
	/// Returns the unique identifier for this user as a string.
	fn id(&self) -> String;

	/// Returns whether this user is authenticated.
	///
	/// For concrete user types, this should always return `true`.
	/// `AnonymousUser` should return `false`.
	fn is_authenticated(&self) -> bool;

	/// Returns whether this user is an administrator.
	fn is_admin(&self) -> bool;

	/// Returns whether this user account is active.
	///
	/// This intentionally differs from `BaseUser::is_active` so types that
	/// implement both traits retain unambiguous method calls. The default keeps
	/// existing identity implementations active unless they provide account
	/// status explicitly.
	fn is_account_active(&self) -> bool {
		true
	}
}

#[cfg(test)]
mod tests {
	use super::AuthIdentity;

	struct LegacyIdentity;

	impl AuthIdentity for LegacyIdentity {
		fn id(&self) -> String {
			"legacy".to_string()
		}

		fn is_authenticated(&self) -> bool {
			true
		}

		fn is_admin(&self) -> bool {
			false
		}
	}

	#[test]
	fn legacy_identity_defaults_to_active() {
		// Arrange
		let identity = LegacyIdentity;

		// Act / Assert
		assert!(identity.is_account_active());
	}
}
