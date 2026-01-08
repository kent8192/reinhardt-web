//! Authentication server functions
//!
//! Server functions for user authentication and session management.
//! These are accessible from both WASM (client stubs) and server (handlers).

use crate::shared::types::UserInfo;

// Server-only imports - only needed for server build
#[cfg(not(target_arch = "wasm32"))]
use {
	crate::apps::auth::models::User,
	crate::shared::types::{LoginRequest, RegisterRequest},
	reinhardt::db::orm::{FilterOperator, FilterValue, Model},
	reinhardt::middleware::session::{SessionData, SessionStoreRef},
	reinhardt::pages::server_fn::{ServerFnError, server_fn},
	reinhardt::{BaseUser, DatabaseConnection},
	uuid::Uuid,
	validator::Validate,
};

// WASM-only imports
#[cfg(target_arch = "wasm32")]
use reinhardt::pages::server_fn::{ServerFnError, server_fn};

/// Login user and return user info (stateless, no session management)
#[cfg(not(target_arch = "wasm32"))]
#[server_fn(use_inject = true)]
pub async fn login(
	email: String,
	password: String,
	#[inject] _db: DatabaseConnection,
) -> std::result::Result<UserInfo, ServerFnError> {
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

	// Return user info (client will manage state)
	Ok(UserInfo::from(user))
}

/// Login - WASM client stub
#[cfg(target_arch = "wasm32")]
#[server_fn]
pub async fn login(
	email: String,
	password: String,
) -> std::result::Result<UserInfo, ServerFnError> {
	// This body is replaced by the macro with HTTP request code
	unreachable!("This function body should be replaced by the server_fn macro")
}

/// Register new user
#[cfg(not(target_arch = "wasm32"))]
#[server_fn(use_inject = true)]
pub async fn register(
	username: String,
	email: String,
	password: String,
	password_confirmation: String,
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

/// Register - WASM client stub
#[cfg(target_arch = "wasm32")]
#[server_fn]
pub async fn register(
	username: String,
	email: String,
	password: String,
	password_confirmation: String,
) -> std::result::Result<(), ServerFnError> {
	// This body is replaced by the macro with HTTP request code
	unreachable!("This function body should be replaced by the server_fn macro")
}

/// Logout user
#[cfg(not(target_arch = "wasm32"))]
#[server_fn(use_inject = true)]
pub async fn logout(
	#[inject] session: SessionData,
	#[inject] store: SessionStoreRef,
) -> std::result::Result<(), ServerFnError> {
	// Delete session from store
	store.inner().delete(&session.id);
	Ok(())
}

/// Logout - WASM client stub
#[cfg(target_arch = "wasm32")]
#[server_fn]
pub async fn logout() -> std::result::Result<(), ServerFnError> {
	// This body is replaced by the macro with HTTP request code
	unreachable!("This function body should be replaced by the server_fn macro")
}

/// Get current logged-in user
#[cfg(not(target_arch = "wasm32"))]
#[server_fn(use_inject = true)]
pub async fn current_user(
	#[inject] _db: DatabaseConnection,
	#[inject] session: SessionData,
) -> std::result::Result<Option<UserInfo>, ServerFnError> {
	// Get user ID from session
	let user_id = match session.get::<Uuid>("user_id") {
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

/// Current user - WASM client stub
#[cfg(target_arch = "wasm32")]
#[server_fn]
pub async fn current_user() -> std::result::Result<Option<UserInfo>, ServerFnError> {
	// This body is replaced by the macro with HTTP request code
	unreachable!("This function body should be replaced by the server_fn macro")
}
