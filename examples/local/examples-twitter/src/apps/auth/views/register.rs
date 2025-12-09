//! Register view handlers
//!
//! Handles user registration endpoints
//!
//! Uses reinhardt ORM (Manager/QuerySet) for database operations.

use crate::apps::auth::models::User;
use crate::apps::auth::serializers::RegisterRequest;
use reinhardt::db::orm::{FilterOperator, FilterValue, Model};
use reinhardt::db::DatabaseConnection;
use reinhardt::post;
use reinhardt::{BaseUser, Error, Json, Response, StatusCode, ViewResult};
use validator::Validate;

/// Register a new user
///
/// POST /accounts/auth/register/
/// Request body:
/// ```json
/// {
///   "email": "user@example.com",
///   "username": "username",
///   "password": "password123",
///   "password_confirmation": "password123"
/// }
/// ```
/// Success response: 204 No Content
/// Error responses:
/// - 422 Unprocessable Entity: Validation errors
/// - 409 Conflict: Email already exists
#[post("/register/", name = "register", use_inject = true)]
pub async fn register(
	Json(register_req): Json<RegisterRequest>,
	#[inject] db: DatabaseConnection,
) -> ViewResult<Response> {
	// Validate request (automatic JSON parsing by Json extractor)
	register_req
		.validate()
		.map_err(|e| Error::Validation(format!("Validation failed: {}", e)))?;

	// Validate passwords match
	register_req
		.validate_passwords_match()
		.map_err(|e| Error::Validation(format!("Password validation failed: {}", e)))?;

	// Check if email already exists using Manager/QuerySet API
	let existing = User::objects()
		.filter(
			User::field_email(),
			FilterOperator::Eq,
			FilterValue::String(register_req.email.trim().to_string()),
		)
		.first()
		.await;

	if existing.is_ok() && existing.unwrap().is_some() {
		return Err(Error::Http("Email already exists".into()));
	}

	// Create new user using generated new() function
	// new() auto-generates id, last_login, created_at, and ManyToManyField instances
	let mut new_user = User::new(
		register_req.username.trim().to_string(),
		register_req.email.trim().to_string(),
		None, // password_hash will be set after hashing
		true, // is_active
	);

	// Hash password using BaseUser trait
	new_user
		.set_password(&register_req.password)
		.map_err(|e| Error::Database(format!("Password hashing failed: {}", e)))?;

	// Create user in database using Manager
	User::objects().create_with_conn(&db, &new_user).await?;

	// Return 204 No Content
	Ok(Response::new(StatusCode::NO_CONTENT))
}
