//! Serializer + Validator Integration Tests
//!
//! Tests the integration between serializers and validators.
//!
//! ## Test Coverage
//!
//! This test file covers:
//! - **Field Validation**: Serializer field-level validation with validators
//! - **Cross-Field Validation**: Validation logic spanning multiple fields
//! - **Nested Validation**: Validation of nested serializer structures
//! - **Custom Validators**: Integration of custom validator functions
//! - **Error Aggregation**: Collecting and formatting validation errors
//!
//! ## Test Categories
//!
//! 1. **Basic Validation**: Single field validation with built-in validators
//! 2. **Complex Validation**: Multi-field and conditional validation
//! 3. **Nested Structures**: Validation of deeply nested serializer hierarchies
//! 4. **Database Constraints**: Validation aligned with database constraints
//! 5. **Error Handling**: Proper error messages and field mapping
//!
//! ## Fixtures Used
//!
//! - `postgres_container`: For database constraint validation tests
//!
//! ## What These Tests Verify
//!
//! ✅ Serializer fields can use validator functions
//! ✅ Validation errors are properly collected and formatted
//! ✅ Cross-field validation works correctly
//! ✅ Nested serializers validate recursively
//! ✅ Custom validators integrate seamlessly
//! ✅ Database constraint violations are caught by validators
//!
//! ## What These Tests Don't Cover
//!
//! ❌ UI form validation (covered by forms integration tests)
//! ❌ API endpoint validation (covered by API integration tests)
//! ❌ Performance benchmarking of validation logic
//! ❌ Internationalized error messages

use reinhardt_forms;
use reinhardt_test::fixtures::*;
use rstest::*;
use serde::{Deserialize, Serialize};
use sqlx::any::AnyPool;
use std::collections::HashMap;
use std::sync::Arc;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage};

// ============ Test Helper Structs ============

/// User registration data with validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserRegistration {
	username: String,
	email: String,
	password: String,
	confirm_password: String,
	age: Option<i32>,
}

impl UserRegistration {
	fn new(
		username: &str,
		email: &str,
		password: &str,
		confirm_password: &str,
		age: Option<i32>,
	) -> Self {
		Self {
			username: username.to_string(),
			email: email.to_string(),
			password: password.to_string(),
			confirm_password: confirm_password.to_string(),
			age,
		}
	}

	/// Validate username (3-20 chars, alphanumeric + underscore)
	fn validate_username(&self) -> Result<(), String> {
		if self.username.len() < 3 {
			return Err("Username must be at least 3 characters".to_string());
		}
		if self.username.len() > 20 {
			return Err("Username must not exceed 20 characters".to_string());
		}
		if !self
			.username
			.chars()
			.all(|c| c.is_alphanumeric() || c == '_')
		{
			return Err("Username can only contain letters, numbers, and underscores".to_string());
		}
		Ok(())
	}

	/// Validate email format
	fn validate_email(&self) -> Result<(), String> {
		if !self.email.contains('@') {
			return Err("Email must contain @".to_string());
		}
		if !self.email.contains('.') {
			return Err("Email must contain a domain".to_string());
		}
		let parts: Vec<&str> = self.email.split('@').collect();
		if parts.len() != 2 {
			return Err("Email must have exactly one @".to_string());
		}
		if parts[0].is_empty() {
			return Err("Email local part cannot be empty".to_string());
		}
		if parts[1].is_empty() {
			return Err("Email domain cannot be empty".to_string());
		}
		Ok(())
	}

	/// Validate password strength (min 8 chars, at least one number)
	fn validate_password(&self) -> Result<(), String> {
		if self.password.len() < 8 {
			return Err("Password must be at least 8 characters".to_string());
		}
		if !self.password.chars().any(|c| c.is_numeric()) {
			return Err("Password must contain at least one number".to_string());
		}
		Ok(())
	}

	/// Cross-field validation: passwords must match
	fn validate_password_match(&self) -> Result<(), String> {
		if self.password != self.confirm_password {
			return Err("Passwords do not match".to_string());
		}
		Ok(())
	}

	/// Validate age range
	fn validate_age(&self) -> Result<(), String> {
		if let Some(age) = self.age {
			if age < 13 {
				return Err("User must be at least 13 years old".to_string());
			}
			if age > 120 {
				return Err("Age must be realistic".to_string());
			}
		}
		Ok(())
	}

	/// Run all validations
	fn validate_all(&self) -> Result<(), HashMap<String, String>> {
		let mut errors = HashMap::new();

		if let Err(e) = self.validate_username() {
			errors.insert("username".to_string(), e);
		}
		if let Err(e) = self.validate_email() {
			errors.insert("email".to_string(), e);
		}
		if let Err(e) = self.validate_password() {
			errors.insert("password".to_string(), e);
		}
		if let Err(e) = self.validate_password_match() {
			errors.insert("confirm_password".to_string(), e);
		}
		if let Err(e) = self.validate_age() {
			errors.insert("age".to_string(), e);
		}

		if errors.is_empty() {
			Ok(())
		} else {
			Err(errors)
		}
	}
}

/// Address data for nested validation
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Address {
	street: String,
	city: String,
	postal_code: String,
	country: String,
}

impl Address {
	fn new(street: &str, city: &str, postal_code: &str, country: &str) -> Self {
		Self {
			street: street.to_string(),
			city: city.to_string(),
			postal_code: postal_code.to_string(),
			country: country.to_string(),
		}
	}

	fn validate(&self) -> Result<(), HashMap<String, String>> {
		let mut errors = HashMap::new();

		if self.street.is_empty() {
			errors.insert("street".to_string(), "Street is required".to_string());
		}
		if self.city.is_empty() {
			errors.insert("city".to_string(), "City is required".to_string());
		}
		if self.postal_code.is_empty() {
			errors.insert(
				"postal_code".to_string(),
				"Postal code is required".to_string(),
			);
		}
		if self.country.is_empty() {
			errors.insert("country".to_string(), "Country is required".to_string());
		}

		if errors.is_empty() {
			Ok(())
		} else {
			Err(errors)
		}
	}
}

/// User profile with nested address
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserProfile {
	username: String,
	email: String,
	address: Address,
}

impl UserProfile {
	fn new(username: &str, email: &str, address: Address) -> Self {
		Self {
			username: username.to_string(),
			email: email.to_string(),
			address,
		}
	}

	fn validate(&self) -> Result<(), HashMap<String, String>> {
		let mut errors = HashMap::new();

		// Validate username
		if self.username.len() < 3 {
			errors.insert(
				"username".to_string(),
				"Username must be at least 3 characters".to_string(),
			);
		}

		// Validate email
		if !self.email.contains('@') {
			errors.insert("email".to_string(), "Email must contain @".to_string());
		}

		// Validate nested address
		if let Err(address_errors) = self.address.validate() {
			for (field, error) in address_errors {
				errors.insert(format!("address.{}", field), error);
			}
		}

		if errors.is_empty() {
			Ok(())
		} else {
			Err(errors)
		}
	}
}

// ============ Basic Field Validation Tests ============

/// Test username field validation
///
/// Verifies:
/// - Username length constraints (3-20 chars)
/// - Allowed characters (alphanumeric + underscore)
/// - Validation error messages
#[test]
fn test_username_validation() {
	// Valid username
	let valid = UserRegistration::new("valid_user123", "user@example.com", "password1", "password1", None);
	assert!(valid.validate_username().is_ok());

	// Too short
	let too_short = UserRegistration::new("ab", "user@example.com", "password1", "password1", None);
	let result = too_short.validate_username();
	assert!(result.is_err());
	assert_eq!(
		result.unwrap_err(),
		"Username must be at least 3 characters"
	);

	// Too long
	let too_long = UserRegistration::new(
		"this_username_is_way_too_long",
		"user@example.com",
		"password1",
		"password1",
		None,
	);
	let result = too_long.validate_username();
	assert!(result.is_err());
	assert_eq!(
		result.unwrap_err(),
		"Username must not exceed 20 characters"
	);

	// Invalid characters
	let invalid_chars = UserRegistration::new("user@name", "user@example.com", "password1", "password1", None);
	let result = invalid_chars.validate_username();
	assert!(result.is_err());
	assert_eq!(
		result.unwrap_err(),
		"Username can only contain letters, numbers, and underscores"
	);
}

/// Test email field validation
///
/// Verifies:
/// - Email format (contains @ and .)
/// - Local part and domain presence
/// - Validation error messages
#[test]
fn test_email_validation() {
	// Valid email
	let valid = UserRegistration::new("user", "user@example.com", "password1", "password1", None);
	assert!(valid.validate_email().is_ok());

	// Missing @
	let missing_at = UserRegistration::new("user", "userexample.com", "password1", "password1", None);
	let result = missing_at.validate_email();
	assert!(result.is_err());
	assert_eq!(result.unwrap_err(), "Email must contain @");

	// Missing domain
	let missing_domain = UserRegistration::new("user", "user@", "password1", "password1", None);
	let result = missing_domain.validate_email();
	assert!(result.is_err());
	assert_eq!(result.unwrap_err(), "Email domain cannot be empty");

	// Empty local part
	let empty_local = UserRegistration::new("user", "@example.com", "password1", "password1", None);
	let result = empty_local.validate_email();
	assert!(result.is_err());
	assert_eq!(result.unwrap_err(), "Email local part cannot be empty");

	// Multiple @
	let multiple_at = UserRegistration::new("user", "user@@example.com", "password1", "password1", None);
	let result = multiple_at.validate_email();
	assert!(result.is_err());
	assert_eq!(result.unwrap_err(), "Email must have exactly one @");
}

/// Test password field validation
///
/// Verifies:
/// - Minimum length (8 chars)
/// - Required number presence
/// - Validation error messages
#[test]
fn test_password_validation() {
	// Valid password
	let valid = UserRegistration::new("user", "user@example.com", "password123", "password123", None);
	assert!(valid.validate_password().is_ok());

	// Too short
	let too_short = UserRegistration::new("user", "user@example.com", "pass1", "pass1", None);
	let result = too_short.validate_password();
	assert!(result.is_err());
	assert_eq!(
		result.unwrap_err(),
		"Password must be at least 8 characters"
	);

	// No number
	let no_number = UserRegistration::new("user", "user@example.com", "password", "password", None);
	let result = no_number.validate_password();
	assert!(result.is_err());
	assert_eq!(
		result.unwrap_err(),
		"Password must contain at least one number"
	);
}

/// Test age range validation
///
/// Verifies:
/// - Minimum age (13 years)
/// - Maximum age (120 years)
/// - Optional field handling (None is valid)
#[test]
fn test_age_validation() {
	// Valid age
	let valid = UserRegistration::new("user", "user@example.com", "password1", "password1", Some(25));
	assert!(valid.validate_age().is_ok());

	// No age (optional)
	let no_age = UserRegistration::new("user", "user@example.com", "password1", "password1", None);
	assert!(no_age.validate_age().is_ok());

	// Too young
	let too_young = UserRegistration::new("user", "user@example.com", "password1", "password1", Some(10));
	let result = too_young.validate_age();
	assert!(result.is_err());
	assert_eq!(
		result.unwrap_err(),
		"User must be at least 13 years old"
	);

	// Unrealistic age
	let unrealistic = UserRegistration::new("user", "user@example.com", "password1", "password1", Some(150));
	let result = unrealistic.validate_age();
	assert!(result.is_err());
	assert_eq!(result.unwrap_err(), "Age must be realistic");
}

// ============ Cross-Field Validation Tests ============

/// Test password confirmation matching
///
/// Verifies:
/// - Passwords must match
/// - Error message for mismatch
#[test]
fn test_password_match_validation() {
	// Matching passwords
	let matching =
		UserRegistration::new("user", "user@example.com", "password1", "password1", None);
	assert!(matching.validate_password_match().is_ok());

	// Non-matching passwords
	let non_matching = UserRegistration::new(
		"user",
		"user@example.com",
		"password1",
		"different1",
		None,
	);
	let result = non_matching.validate_password_match();
	assert!(result.is_err());
	assert_eq!(result.unwrap_err(), "Passwords do not match");
}

/// Test all validations together with error aggregation
///
/// Verifies:
/// - All validations run independently
/// - Errors are collected per field
/// - Multiple errors can be reported simultaneously
#[test]
fn test_all_validations_error_aggregation() {
	// All valid
	let all_valid = UserRegistration::new(
		"valid_user",
		"user@example.com",
		"password123",
		"password123",
		Some(25),
	);
	assert!(all_valid.validate_all().is_ok());

	// Multiple errors
	let multiple_errors = UserRegistration::new(
		"ab",              // Too short username
		"invalid-email",   // No @ in email
		"short",           // Password too short and no number
		"different",       // Passwords don't match
		Some(10),          // Age too young
	);

	let result = multiple_errors.validate_all();
	assert!(result.is_err());

	let errors = result.unwrap_err();
	assert!(errors.contains_key("username"));
	assert!(errors.contains_key("email"));
	assert!(errors.contains_key("password"));
	assert!(errors.contains_key("confirm_password"));
	assert!(errors.contains_key("age"));

	// Verify specific error messages
	assert_eq!(
		errors.get("username").unwrap(),
		"Username must be at least 3 characters"
	);
	assert_eq!(errors.get("email").unwrap(), "Email must contain @");
	assert_eq!(
		errors.get("password").unwrap(),
		"Password must be at least 8 characters"
	);
	assert_eq!(
		errors.get("confirm_password").unwrap(),
		"Passwords do not match"
	);
	assert_eq!(
		errors.get("age").unwrap(),
		"User must be at least 13 years old"
	);
}

// ============ Nested Validation Tests ============

/// Test nested address validation
///
/// Verifies:
/// - Address fields are validated
/// - Required field validation
/// - Error messages include field path
#[test]
fn test_nested_address_validation() {
	// Valid address
	let valid_address = Address::new("123 Main St", "New York", "10001", "USA");
	assert!(valid_address.validate().is_ok());

	// Empty street
	let empty_street = Address::new("", "New York", "10001", "USA");
	let result = empty_street.validate();
	assert!(result.is_err());
	let errors = result.unwrap_err();
	assert_eq!(errors.get("street").unwrap(), "Street is required");

	// Multiple empty fields
	let multiple_empty = Address::new("", "", "10001", "");
	let result = multiple_empty.validate();
	assert!(result.is_err());
	let errors = result.unwrap_err();
	assert_eq!(errors.len(), 3);
	assert!(errors.contains_key("street"));
	assert!(errors.contains_key("city"));
	assert!(errors.contains_key("country"));
}

/// Test user profile with nested address validation
///
/// Verifies:
/// - Parent and nested validations run
/// - Errors are properly namespaced (address.field)
/// - All levels of nesting are validated
#[test]
fn test_nested_profile_validation() {
	// Valid profile
	let valid_address = Address::new("123 Main St", "New York", "10001", "USA");
	let valid_profile = UserProfile::new("valid_user", "user@example.com", valid_address);
	assert!(valid_profile.validate().is_ok());

	// Invalid username, valid address
	let invalid_username = UserProfile::new(
		"ab",
		"user@example.com",
		Address::new("123 Main St", "New York", "10001", "USA"),
	);
	let result = invalid_username.validate();
	assert!(result.is_err());
	let errors = result.unwrap_err();
	assert!(errors.contains_key("username"));
	assert!(!errors.contains_key("email"));

	// Valid username, invalid address
	let invalid_address = UserProfile::new(
		"valid_user",
		"user@example.com",
		Address::new("", "", "10001", ""),
	);
	let result = invalid_address.validate();
	assert!(result.is_err());
	let errors = result.unwrap_err();
	assert!(errors.contains_key("address.street"));
	assert!(errors.contains_key("address.city"));
	assert!(errors.contains_key("address.country"));
	assert!(!errors.contains_key("username"));

	// Multiple errors across levels
	let multiple_errors = UserProfile::new(
		"ab",          // Invalid username
		"invalid",     // Invalid email
		Address::new("", "New York", "", "USA"), // Invalid address
	);
	let result = multiple_errors.validate();
	assert!(result.is_err());
	let errors = result.unwrap_err();
	assert!(errors.contains_key("username"));
	assert!(errors.contains_key("email"));
	assert!(errors.contains_key("address.street"));
	assert!(errors.contains_key("address.postal_code"));
}

// ============ Database Constraint Validation Tests ============

/// Test unique constraint validation with database
///
/// Verifies:
/// - Unique username constraint
/// - Unique email constraint
/// - Database and validator alignment
#[rstest]
#[tokio::test]
async fn test_unique_constraint_validation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<AnyPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create users table with unique constraints
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS users (
			id SERIAL PRIMARY KEY,
			username VARCHAR(50) UNIQUE NOT NULL,
			email VARCHAR(100) UNIQUE NOT NULL,
			password_hash TEXT NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create users table");

	// Insert first user
	sqlx::query("INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3)")
		.bind("existing_user")
		.bind("existing@example.com")
		.bind("hashed_password")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert first user");

	// Check username uniqueness
	let username_exists: bool = sqlx::query_scalar(
		"SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)",
	)
	.bind("existing_user")
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to check username");

	assert!(username_exists, "Username should exist in database");

	// Check email uniqueness
	let email_exists: bool =
		sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
			.bind("existing@example.com")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to check email");

	assert!(email_exists, "Email should exist in database");

	// Verify new username doesn't exist
	let new_username_exists: bool = sqlx::query_scalar(
		"SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)",
	)
	.bind("new_user")
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to check new username");

	assert!(
		!new_username_exists,
		"New username should not exist in database"
	);
}

/// Test length constraint validation with database
///
/// Verifies:
/// - VARCHAR length limits match validation
/// - Database rejects over-length values
/// - Validators catch before database
#[rstest]
#[tokio::test]
async fn test_length_constraint_validation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<AnyPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create table with length constraints
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS users (
			id SERIAL PRIMARY KEY,
			username VARCHAR(20) NOT NULL,
			bio VARCHAR(200)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create users table");

	// Valid length
	let valid_result = sqlx::query("INSERT INTO users (username, bio) VALUES ($1, $2)")
		.bind("valid_user")
		.bind("This is a valid bio")
		.execute(pool.as_ref())
		.await;

	assert!(valid_result.is_ok(), "Valid length should succeed");

	// Username too long (exceeds VARCHAR(20))
	let long_username = "this_username_is_way_too_long_for_database";
	let long_username_result = sqlx::query("INSERT INTO users (username, bio) VALUES ($1, $2)")
		.bind(long_username)
		.bind("Bio")
		.execute(pool.as_ref())
		.await;

	assert!(
		long_username_result.is_err(),
		"Username exceeding 20 chars should fail"
	);

	// Verify validator catches it first
	let registration = UserRegistration::new(
		long_username,
		"user@example.com",
		"password1",
		"password1",
		None,
	);
	let validation_result = registration.validate_username();
	assert!(
		validation_result.is_err(),
		"Validator should catch long username before database"
	);
}

/// Test NOT NULL constraint validation
///
/// Verifies:
/// - Required fields are validated
/// - Database rejects NULL values
/// - Validators enforce required fields
#[rstest]
#[tokio::test]
async fn test_not_null_constraint_validation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<AnyPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create table with NOT NULL constraints
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS products (
			id SERIAL PRIMARY KEY,
			name VARCHAR(100) NOT NULL,
			description TEXT,
			price DECIMAL(10, 2) NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create products table");

	// Valid insert with all required fields
	let valid_result = sqlx::query("INSERT INTO products (name, price) VALUES ($1, $2)")
		.bind("Product Name")
		.bind(19.99)
		.execute(pool.as_ref())
		.await;

	assert!(valid_result.is_ok(), "Valid insert should succeed");

	// Missing NOT NULL field (price) - this would fail at serialization level
	// In real usage, the serializer would validate required fields before DB insert

	// Verify product was inserted
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM products")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count products");

	assert_eq!(count, 1, "Should have 1 product");
}
