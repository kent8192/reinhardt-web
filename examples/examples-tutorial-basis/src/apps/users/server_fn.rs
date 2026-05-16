//! User authentication server functions
//!
//! Provides session-cookie-based login/logout and current-user lookup.
//! Follows the examples-twitter pattern: `SessionData` + `SessionStoreRef`
//! are injected, the session ID is regenerated on successful login
//! (fixation prevention), and `user_id` is persisted in the session map.

use crate::shared::types::UserInfo;
#[cfg(native)]
use crate::shared::types::{LoginRequest, RegisterRequest};
use reinhardt::pages::server_fn::{ServerFnError, server_fn};

#[cfg(native)]
use {
	crate::apps::users::models::{User, UserManager},
	reinhardt::BaseUser,
	reinhardt::DatabaseConnection,
	reinhardt::Validate,
	reinhardt::db::orm::{FilterOperator, FilterValue, Model},
	reinhardt::di::Depends,
	reinhardt::middleware::session::{SessionData, SessionStoreRef},
	reinhardt::reinhardt_auth::BaseUserManager,
	std::collections::HashMap,
};

/// Authenticate a user by username/password and persist the session.
///
/// `_csrf_token` is appended by `form!` for non-GET forms (reinhardt-web#3337);
/// CSRF is verified by middleware before this handler runs.
#[server_fn]
pub async fn login(
	username: String,
	password: String,
	_csrf_token: String,
	#[inject] _db: DatabaseConnection,
	#[inject] session: SessionData,
	#[inject] store: SessionStoreRef,
) -> std::result::Result<UserInfo, ServerFnError> {
	let mut session = session;

	let request = LoginRequest { username, password };

	// Run the field-level validators declared on `LoginRequest` before
	// touching the database — empty/oversized credentials should reject
	// at the request boundary rather than slip through to the password
	// comparison below.
	request
		.validate()
		.map_err(|e| ServerFnError::application(format!("Validation failed: {}", e)))?;

	let manager = User::objects();
	let user = manager
		.filter(
			User::field_username(),
			FilterOperator::Eq,
			FilterValue::String(request.username.trim().to_string()),
		)
		.first()
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?
		.ok_or_else(|| ServerFnError::server(401, "Invalid credentials"))?;

	let valid = user
		.check_password(&request.password)
		.map_err(|e| ServerFnError::application(format!("Password check failed: {}", e)))?;

	if !valid {
		return Err(ServerFnError::server(401, "Invalid credentials"));
	}

	if !user.is_active() {
		return Err(ServerFnError::server(403, "User account is inactive"));
	}

	// Session fixation prevention: rotate the session ID before we associate
	// it with the authenticated user. See examples-twitter login for context.
	let old_id = session.regenerate_id();

	session
		.set("user_id".to_string(), user.id())
		.map_err(|e| ServerFnError::application(format!("Session error: {}", e)))?;

	store.inner().delete(&old_id);
	store.inner().save(session);

	Ok(UserInfo::from(user))
}

/// Register a new account and start an authenticated session.
///
/// Mirrors `login`'s session-handling: on success the session id is rotated
/// (fixation prevention) and `user_id` is persisted so the caller is logged
/// in immediately — typical "sign-up then continue" UX for tutorials. The
/// trailing `_csrf_token: String` is supplied by `form!`'s `strip_arguments`
/// (reinhardt-web#3971); CSRF is verified by middleware before this runs.
///
/// We invoke `request.validate()` manually rather than using
/// `#[server_fn(pre_validate = true)]` because that flag only triggers when
/// each parameter is an extractor type whose inner DTO derives `Validate`
/// (e.g. `body: Json<RegisterRequest>` — see
/// `tests/integration/src/pre_validate.rs`). The `form!` macro sends the
/// HTML form's fields as individual `String` params to keep its
/// `strip_arguments` flow working, so the macro-generated synthetic
/// `Args` struct only derives `Deserialize` — there is nothing on the
/// auto-path that knows the field-level `#[validate(...)]` attributes on
/// `RegisterRequest`. Building the DTO by hand and validating it
/// recovers the same guarantees without giving up the `form!` ergonomics.
#[server_fn]
pub async fn register(
	username: String,
	password: String,
	password_confirmation: String,
	_csrf_token: String,
	#[inject] user_manager: Depends<UserManager>,
	#[inject] session: SessionData,
	#[inject] store: SessionStoreRef,
) -> std::result::Result<UserInfo, ServerFnError> {
	let mut session = session;

	let request = RegisterRequest {
		username,
		password,
		password_confirmation,
	};

	// Field-level validators declared on `RegisterRequest` (length on
	// username + password, non-empty password_confirmation). Run them
	// before the password-equality check so a too-short password does
	// not silently match a too-short confirmation.
	request
		.validate()
		.map_err(|e| ServerFnError::application(format!("Validation failed: {}", e)))?;

	// Password confirmation lives outside the derived `Validate` —
	// see the `validate_passwords_match` rationale on `RegisterRequest`.
	request
		.validate_passwords_match()
		.map_err(ServerFnError::application)?;

	// Delegate to `UserManager` — it owns the "validate + hash + persist"
	// pipeline so this server function stays focused on session handling.
	// Username length, uniqueness, and password strength are all enforced
	// inside `create_user`; any failure surfaces as a `reinhardt::Error`
	// that maps to a 400 via `ServerFnError::application`.
	//
	// `BaseUserManager::create_user` takes `&mut self`, but DI hands us a
	// shared `Depends<UserManager>` (an `Arc` under the hood). Clone the
	// inner manager — its only field is another `Depends<DatabaseConnection>`,
	// which is itself an `Arc` clone — so this is cheap and gives us the
	// `&mut` access the trait method needs.
	let mut user_manager: UserManager = (*user_manager).clone();
	let saved = user_manager
		.create_user(
			request.username.trim(),
			Some(&request.password),
			HashMap::new(),
		)
		.await
		.map_err(|e| ServerFnError::application(e.to_string()))?;

	// Match `login`: rotate the session id and persist `user_id` so the
	// account is signed in immediately on successful registration.
	let old_id = session.regenerate_id();
	session
		.set("user_id".to_string(), saved.id())
		.map_err(|e| ServerFnError::application(format!("Session error: {}", e)))?;
	store.inner().delete(&old_id);
	store.inner().save(session);

	Ok(UserInfo::from(saved))
}

/// Clear the active session.
///
/// `_csrf_token` is appended by `form!` for non-GET forms; see [`login`].
#[server_fn]
pub async fn logout(
	_csrf_token: String,
	#[inject] session: SessionData,
	#[inject] store: SessionStoreRef,
) -> std::result::Result<(), ServerFnError> {
	let mut session = session;

	// Only honor logout for sessions that actually carry an authenticated
	// user; unauthenticated callers with a fresh cookie should not be able
	// to drive session-store deletes.
	if session.get::<i64>("user_id").is_none() {
		return Err(ServerFnError::server(401, "Not authenticated"));
	}

	// Rotate the session id before deleting the old entry so the previous
	// id cannot be reused by a downstream component that re-saves the
	// session struct, mirroring the fixation-prevention rotation in
	// `login`.
	let old_id = session.regenerate_id();
	store.inner().delete(&old_id);
	store.inner().delete(&session.id);
	Ok(())
}

/// Return the currently authenticated user, if any.
#[server_fn]
pub async fn current_user(
	#[inject] _db: DatabaseConnection,
	#[inject] session: SessionData,
) -> std::result::Result<Option<UserInfo>, ServerFnError> {
	let user_id = match session.get::<i64>("user_id") {
		Some(id) => id,
		None => return Ok(None),
	};

	let user = User::objects()
		.filter(
			User::field_id(),
			FilterOperator::Eq,
			FilterValue::Int(user_id),
		)
		.first()
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?;

	Ok(user.map(UserInfo::from))
}
