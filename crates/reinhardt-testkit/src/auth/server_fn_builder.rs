use super::error::TestAuthError;
use super::identity::SessionIdentity;
use super::secondary::SecondaryAuth;
use super::traits::ForceLoginUser;
use crate::server_fn::MockSession;
use crate::server_fn::ServerFnTestContext;

/// Builder for auth configuration in server_fn test contexts.
///
/// Mirrors the [`crate::client::APIClient`] auth builder API. Uses [`MockSession`] instead of
/// a real `AsyncSessionBackend`.
pub struct ServerFnAuthBuilder {
	ctx: ServerFnTestContext,
	identity: Option<SessionIdentity>,
	secondary: Vec<Box<dyn SecondaryAuth>>,
}

impl ServerFnAuthBuilder {
	pub(crate) fn new(ctx: ServerFnTestContext) -> Self {
		Self {
			ctx,
			identity: None,
			secondary: vec![],
		}
	}

	/// Authenticate as the given user via session.
	///
	/// No `AsyncSessionBackend` is required — uses [`MockSession`] internally.
	pub fn session(mut self, user: &impl ForceLoginUser) -> Self {
		self.identity = Some(SessionIdentity::from_user(user));
		self
	}

	/// Authenticate via JWT (sets identity for mock session).
	pub fn jwt(
		mut self,
		user: &impl ForceLoginUser,
		_config: &super::builder::JwtTestConfig,
	) -> Self {
		self.identity = Some(SessionIdentity::from_user(user));
		self
	}

	/// Override the `is_staff` flag.
	pub fn with_staff(mut self, is_staff: bool) -> Self {
		if let Some(ref mut id) = self.identity {
			id.is_staff = is_staff;
		}
		self
	}

	/// Override the `is_superuser` flag.
	pub fn with_superuser(mut self, is_superuser: bool) -> Self {
		if let Some(ref mut id) = self.identity {
			id.is_superuser = is_superuser;
		}
		self
	}

	/// Add TOTP MFA as a secondary auth layer with a pre-generated code.
	pub fn with_mfa_code(self, code: impl Into<String>) -> Self {
		self.with_secondary(super::secondary::TotpSecondaryAuth::with_code_only(code))
	}

	/// Add a custom secondary auth layer.
	pub fn with_secondary(mut self, auth: impl SecondaryAuth + 'static) -> Self {
		self.secondary.push(Box::new(auth));
		self
	}

	/// Finalize auth configuration and return the configured [`ServerFnTestContext`].
	///
	/// Call `.build()` or `.build_context()` on the result to get the test environment.
	pub fn done(mut self) -> ServerFnTestContext {
		if let Some(identity) = &self.identity {
			let mock_session = MockSession::from_identity(identity);
			self.ctx = self.ctx.with_session(mock_session);
		}
		self.ctx
	}
}
