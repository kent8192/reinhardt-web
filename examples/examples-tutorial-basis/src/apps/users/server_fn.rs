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
	reinhardt::middleware::session::{
		SessionAuthExt, SessionData, SessionStoreRef, USER_ID_SESSION_KEY,
	},
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
	session
		.login(&store, user.id())
		.map_err(|e| ServerFnError::application(format!("Session error: {}", e)))?;
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
	request
		.validate()
		.map_err(|e| ServerFnError::application(format!("Validation failed: {}", e)))?;
	request
		.validate_passwords_match()
		.map_err(ServerFnError::application)?;
	let mut user_manager: UserManager = (*user_manager).clone();
	let saved = user_manager
		.create_user(
			request.username.trim(),
			Some(&request.password),
			HashMap::new(),
		)
		.await
		.map_err(|e| ServerFnError::application(e.to_string()))?;
	session
		.login(&store, saved.id())
		.map_err(|e| ServerFnError::application(format!("Session error: {}", e)))?;
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
	if session.get::<i64>(USER_ID_SESSION_KEY).is_none() {
		return Err(ServerFnError::server(401, "Not authenticated"));
	}
	session.logout(&store);
	Ok(())
}
/// Return the currently authenticated user, if any.
#[server_fn]
pub async fn current_user(
	#[inject] _db: DatabaseConnection,
	#[inject] session: SessionData,
) -> std::result::Result<Option<UserInfo>, ServerFnError> {
	let user_id = match session.get::<i64>(USER_ID_SESSION_KEY) {
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
