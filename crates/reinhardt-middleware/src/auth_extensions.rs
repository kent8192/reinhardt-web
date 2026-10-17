//! Installation of authentication state into request extensions.
//!
//! Authentication middleware publishes the result of each attempt in two
//! forms: the canonical [`AuthState`] and the legacy compatibility entries
//! (`String` user ID, [`IsAuthenticated`], [`IsAdmin`], [`IsActive`]) that
//! older consumers read directly. The JWT, cookie-session and remote-user
//! middleware write both forms through [`install_auth_state`], so they
//! always describe the same principal. `AuthenticationMiddleware` writes
//! every entry itself on each request, including an empty user ID for
//! anonymous requests.

use reinhardt_http::{AuthState, Extensions, IsActive, IsAdmin, IsAuthenticated};

/// Installs `state` as the request's authentication state.
///
/// The legacy flags are always overwritten together with the canonical
/// [`AuthState`], so an anonymous state writes `false` flags and discards
/// any principal left behind by an earlier authentication layer instead of
/// letting legacy consumers observe it after this layer failed.
///
/// An anonymous state blanks an existing user ID (the same empty value that
/// `AuthenticationMiddleware` writes) but never adds one. The untyped
/// `String` entry is also read by other middleware: `CsrfMiddleware` uses
/// it as the session identifier and `RateLimitMiddleware` as the user key.
/// Adding an empty ID to requests that never carried one would give every
/// anonymous request the same CSRF secret.
pub(crate) fn install_auth_state(extensions: &Extensions, state: AuthState) {
	if state.is_authenticated() {
		extensions.insert(state.user_id().to_owned());
	} else if extensions.contains::<String>() {
		extensions.insert(String::new());
	}
	extensions.insert(IsAuthenticated(state.is_authenticated()));
	extensions.insert(IsAdmin(state.is_admin()));
	extensions.insert(IsActive(state.is_active()));
	extensions.insert(state);
}

/// Shared fixtures for authentication middleware regression tests.
#[cfg(test)]
pub(crate) mod test_support {
	use super::*;
	use reinhardt_http::{Handler, Request, Response, Result};
	use serde_json::{Value, json};

	/// User ID of the principal installed by [`seed_prior_principal`].
	pub(crate) const PRIOR_USER_ID: &str = "prior-admin";

	/// Simulates an earlier authentication layer that already installed an
	/// active admin principal in both the canonical and the legacy entries.
	pub(crate) fn seed_prior_principal(request: &Request) {
		let extensions = &request.extensions;
		extensions.insert(PRIOR_USER_ID.to_owned());
		extensions.insert(IsAuthenticated(true));
		extensions.insert(IsAdmin(true));
		extensions.insert(IsActive(true));
		extensions.insert(AuthState::authenticated(PRIOR_USER_ID, true, true));
	}

	/// Expected [`AuthEntriesHandler`] body when the canonical state and every
	/// legacy entry describe the given principal.
	pub(crate) fn expected_entries(
		user_id: &str,
		is_authenticated: bool,
		is_admin: bool,
		is_active: bool,
	) -> Value {
		json!({
			"auth_state": {
				"user_id": user_id,
				"is_authenticated": is_authenticated,
				"is_admin": is_admin,
				"is_active": is_active,
			},
			"user_id": user_id,
			"is_authenticated": is_authenticated,
			"is_admin": is_admin,
			"is_active": is_active,
		})
	}

	/// Expected [`AuthEntriesHandler`] body for an anonymous request.
	pub(crate) fn anonymous_entries() -> Value {
		expected_entries("", false, false, false)
	}

	/// Handler that echoes the canonical state and every legacy entry, the
	/// way a downstream legacy consumer would read them.
	pub(crate) struct AuthEntriesHandler;

	#[async_trait::async_trait]
	impl Handler for AuthEntriesHandler {
		async fn handle(&self, request: Request) -> Result<Response> {
			let extensions = &request.extensions;
			let auth_state = extensions.get::<AuthState>().map(|state| {
				json!({
					"user_id": state.user_id(),
					"is_authenticated": state.is_authenticated(),
					"is_admin": state.is_admin(),
					"is_active": state.is_active(),
				})
			});
			Ok(Response::ok().with_json(&json!({
				"auth_state": auth_state,
				"user_id": extensions.get::<String>(),
				"is_authenticated": extensions.get::<IsAuthenticated>().map(|v| v.0),
				"is_admin": extensions.get::<IsAdmin>().map(|v| v.0),
				"is_active": extensions.get::<IsActive>().map(|v| v.0),
			}))?)
		}
	}

	/// Parses the body produced by [`AuthEntriesHandler`].
	pub(crate) fn auth_entries(response: &Response) -> Value {
		serde_json::from_slice(response.body.as_ref()).unwrap()
	}
}

#[cfg(test)]
mod tests {
	use super::test_support::PRIOR_USER_ID;
	use super::*;
	use rstest::rstest;

	fn extensions_with_prior_principal() -> Extensions {
		let extensions = Extensions::new();
		install_auth_state(
			&extensions,
			AuthState::authenticated(PRIOR_USER_ID, true, true),
		);
		extensions
	}

	#[rstest]
	fn test_anonymous_state_overwrites_prior_legacy_markers() {
		// Arrange
		let extensions = extensions_with_prior_principal();

		// Act
		install_auth_state(&extensions, AuthState::anonymous());

		// Assert
		assert_eq!(extensions.get::<AuthState>(), Some(AuthState::anonymous()));
		assert_eq!(extensions.get::<String>(), Some(String::new()));
		assert_eq!(
			extensions.get::<IsAuthenticated>(),
			Some(IsAuthenticated(false))
		);
		assert_eq!(extensions.get::<IsAdmin>(), Some(IsAdmin(false)));
		assert_eq!(extensions.get::<IsActive>(), Some(IsActive(false)));
	}

	#[rstest]
	fn test_anonymous_state_does_not_add_user_id() {
		// Arrange
		let extensions = Extensions::new();

		// Act
		install_auth_state(&extensions, AuthState::anonymous());

		// Assert
		assert_eq!(extensions.get::<AuthState>(), Some(AuthState::anonymous()));
		assert_eq!(extensions.get::<String>(), None);
		assert_eq!(
			extensions.get::<IsAuthenticated>(),
			Some(IsAuthenticated(false))
		);
		assert_eq!(extensions.get::<IsAdmin>(), Some(IsAdmin(false)));
		assert_eq!(extensions.get::<IsActive>(), Some(IsActive(false)));
	}

	#[rstest]
	fn test_authenticated_state_replaces_prior_principal() {
		// Arrange
		let extensions = extensions_with_prior_principal();
		let state = AuthState::authenticated("next-user", false, true);

		// Act
		install_auth_state(&extensions, state.clone());

		// Assert
		assert_eq!(extensions.get::<AuthState>(), Some(state));
		assert_eq!(extensions.get::<String>(), Some("next-user".to_owned()));
		assert_eq!(
			extensions.get::<IsAuthenticated>(),
			Some(IsAuthenticated(true))
		);
		assert_eq!(extensions.get::<IsAdmin>(), Some(IsAdmin(false)));
		assert_eq!(extensions.get::<IsActive>(), Some(IsActive(true)));
	}
}
