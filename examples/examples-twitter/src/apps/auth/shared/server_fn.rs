//! Authentication server functions
//!
//! Server functions for user authentication and session management.

use crate::apps::auth::shared::types::UserInfo;
use reinhardt::pages::server_fn::{ServerFnError, server_fn};

// Server-only imports
#[cfg(native)]
use {
	crate::apps::auth::models::User,
	crate::apps::auth::shared::types::{LoginRequest, RegisterRequest},
	reinhardt::Validate,
	reinhardt::db::orm::{FilterOperator, FilterValue, Model},
	reinhardt::middleware::session::{
		SessionAuthExt, SessionData, SessionStoreRef, USER_ID_SESSION_KEY,
	},
	reinhardt::{BaseUser, DatabaseConnection},
	uuid::Uuid,
};

/// Login user, persist session, and return user info
///
/// `_csrf_token` is auto-appended by the `form!` macro for non-GET forms
/// (commit 0fd5bf1e1 / #3337). CSRF is enforced by middleware, so we accept
/// and ignore it here. See #3825.
#[server_fn]
pub async fn login(
	email: String,
	password: String,
	_csrf_token: String,
	#[inject] _db: DatabaseConnection,
	#[inject] session: SessionData,
	#[inject] store: SessionStoreRef,
) -> std::result::Result<UserInfo, ServerFnError> {
	let mut session = session;

	// Construct request from parameters
	let request = LoginRequest { email, password };

	// Validate request
	request
		.validate()
		.map_err(|e| ServerFnError::application(format!("Validation failed: {}", e)))?;

	// Find user by email
	let manager = User::objects();
	let user = manager
		.filter(
			User::field_email(),
			FilterOperator::Eq,
			FilterValue::String(request.email.trim().to_string()),
		)
		.first()
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?
		.ok_or_else(|| ServerFnError::server(401, "Invalid credentials"))?;

	// Check password
	let password_valid = user
		.check_password(&request.password)
		.map_err(|e| ServerFnError::application(format!("Password verification failed: {}", e)))?;

	if !password_valid {
		return Err(ServerFnError::server(401, "Invalid credentials"));
	}

	// Check if user is active
	if !user.is_active() {
		return Err(ServerFnError::server(403, "User account is inactive"));
	}

	// Session fixation prevention: `SessionAuthExt::login` rotates the
	// session ID (keeping the middleware's `Set-Cookie` header in sync via
	// the request-scoped `ActiveSessionId` holder, see #3827), writes the
	// user's primary key under `USER_ID_SESSION_KEY`, deletes the old store
	// entry, and persists the rotated session in one step. See #4446.
	session
		.login(&store, user.id())
		.map_err(|e| ServerFnError::application(format!("Session error: {}", e)))?;

	Ok(UserInfo::from(user))
}

/// Register new user
///
/// `_csrf_token` is auto-appended by the `form!` macro; see [`login`] for details.
#[server_fn]
pub async fn register(
	username: String,
	email: String,
	password: String,
	password_confirmation: String,
	_csrf_token: String,
	#[inject] db: DatabaseConnection,
) -> std::result::Result<(), ServerFnError> {
	// Construct request from parameters
	let request = RegisterRequest {
		username,
		email,
		password,
		password_confirmation,
	};

	// Validate request
	request
		.validate()
		.map_err(|e| ServerFnError::application(format!("Validation failed: {}", e)))?;

	// Validate password match
	request
		.validate_passwords_match()
		.map_err(ServerFnError::application)?;

	// Check if user already exists
	let existing = User::objects()
		.filter(
			User::field_email(),
			FilterOperator::Eq,
			FilterValue::String(request.email.trim().to_string()),
		)
		.first()
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?;

	if existing.is_some() {
		return Err(ServerFnError::application(
			"Email already exists".to_string(),
		));
	}

	// Create new user
	let mut new_user = User::new(
		request.username.trim().to_string(),
		request.email.trim().to_string(),
		None,
		true,
		None,
	);

	// Set password
	new_user
		.set_password(&request.password)
		.map_err(|e| ServerFnError::application(format!("Password hashing failed: {}", e)))?;

	// Save to database
	User::objects()
		.create_with_conn(&db, &new_user)
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?;

	Ok(())
}

/// Logout user
#[server_fn]
pub async fn logout(
	#[inject] session: SessionData,
	#[inject] store: SessionStoreRef,
) -> std::result::Result<(), ServerFnError> {
	let mut session = session;
	// Rotate the session id, drop the user-id key, and persist the rotated
	// session in one step — see `SessionAuthExt` for the design rationale.
	session.logout(&store);
	Ok(())
}

/// Get current logged-in user
#[server_fn]
pub async fn current_user(
	#[inject] _db: DatabaseConnection,
	#[inject] session: SessionData,
) -> std::result::Result<Option<UserInfo>, ServerFnError> {
	// Get user ID from session
	let user_id = match session.get::<Uuid>(USER_ID_SESSION_KEY) {
		Some(id) => id,
		None => return Ok(None),
	};

	// Find user by ID
	let user = User::objects()
		.filter(
			User::field_id(),
			FilterOperator::Eq,
			FilterValue::String(user_id.to_string()),
		)
		.first()
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?;

	Ok(user.map(UserInfo::from))
}
