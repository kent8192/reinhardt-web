//! User authentication server functions
//!
//! Provides session-cookie-based login/logout and current-user lookup.
//! Follows the examples-twitter pattern: `SessionData` + `SessionStoreRef`
//! are injected, the session ID is regenerated on successful login
//! (fixation prevention), and `user_id` is persisted in the session map.

use crate::shared::types::UserInfo;
#[cfg(native)]
use crate::shared::types::LoginRequest;
use reinhardt::pages::server_fn::{ServerFnError, server_fn};

#[cfg(native)]
use {
	crate::apps::users::models::User,
	reinhardt::BaseUser,
	reinhardt::DatabaseConnection,
	reinhardt::db::orm::{FilterOperator, FilterValue, Model},
	reinhardt::middleware::session::{SessionData, SessionStoreRef},
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

/// Clear the active session.
///
/// `_csrf_token` is appended by `form!` for non-GET forms; see [`login`].
#[server_fn]
pub async fn logout(
	_csrf_token: String,
	#[inject] session: SessionData,
	#[inject] store: SessionStoreRef,
) -> std::result::Result<(), ServerFnError> {
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
