//! Authentication server functions
//!
//! Server functions for user authentication and session management.
use crate::apps::auth::shared::types::UserInfo;
use reinhardt::pages::server_fn::{ServerFnError, server_fn};
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
	let request = LoginRequest { email, password };
	request
		.validate()
		.map_err(|e| ServerFnError::application(format!("Validation failed: {}", e)))?;
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
	let password_valid = user
		.check_password(&request.password)
		.map_err(|e| ServerFnError::application(format!("Password verification failed: {}", e)))?;
	if !password_valid {
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
	let request = RegisterRequest {
		username,
		email,
		password,
		password_confirmation,
	};
	request
		.validate()
		.map_err(|e| ServerFnError::application(format!("Validation failed: {}", e)))?;
	request
		.validate_passwords_match()
		.map_err(ServerFnError::application)?;
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
	let mut new_user = User::new(
		request.username.trim().to_string(),
		request.email.trim().to_string(),
		None,
		true,
		false,
		None,
	);
	new_user
		.set_password(&request.password)
		.map_err(|e| ServerFnError::application(format!("Password hashing failed: {}", e)))?;
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
	session.logout(&store);
	Ok(())
}
/// Get current logged-in user
#[server_fn]
pub async fn current_user(
	#[inject] _db: DatabaseConnection,
	#[inject] session: SessionData,
) -> std::result::Result<Option<UserInfo>, ServerFnError> {
	let user_id = match session.get::<Uuid>(USER_ID_SESSION_KEY) {
		Some(id) => id,
		None => return Ok(None),
	};
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
