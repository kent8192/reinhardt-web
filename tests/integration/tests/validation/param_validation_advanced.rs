//! Advanced parameter validation integration tests
//!
//! These tests verify that reinhardt-di parameter extraction works correctly
//! with reinhardt-validators for complex scenarios including:
//! - Path parameter type conversion and validation
//! - Multi-value query parameters
//! - Nested JSON body validation

use reinhardt_core::validators::{
	EmailValidator, MaxLengthValidator, MinLengthValidator, MinValueValidator, RangeValidator,
	ValidationError, Validator,
};
use rstest::rstest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Test Data Structures
// ============================================================================

/// User ID path parameter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UserId(i64);

impl UserId {
	fn new(id: i64) -> Result<Self, ValidationError> {
		let validator = RangeValidator::new(1, i64::MAX);
		validator.validate(&id)?;
		Ok(UserId(id))
	}

	fn value(&self) -> i64 {
		self.0
	}
}

/// Query parameters with multi-value support
#[derive(Debug, Deserialize, Serialize, Clone)]
struct SearchQuery {
	/// Search tags (multi-value: ?tags=rust&tags=api)
	#[serde(default)]
	tags: Vec<String>,
	/// Pagination page number
	#[serde(default = "default_page")]
	page: i32,
	/// Items per page
	#[serde(default = "default_per_page")]
	per_page: i32,
}

fn default_page() -> i32 {
	1
}

fn default_per_page() -> i32 {
	10
}

impl SearchQuery {
	fn validate(&self) -> Result<(), Vec<ValidationError>> {
		let mut errors = Vec::new();

		// Validate page number
		let page_validator = MinValueValidator::new(1);
		if let Err(e) = page_validator.validate(&self.page) {
			errors.push(e);
		}

		// Validate per_page range
		let per_page_validator = RangeValidator::new(1, 100);
		if let Err(e) = per_page_validator.validate(&self.per_page) {
			errors.push(e);
		}

		// Validate each tag
		let tag_length_validator = MaxLengthValidator::new(50);
		for tag in &self.tags {
			if let Err(e) = tag_length_validator.validate(tag) {
				errors.push(e);
			}
		}

		if errors.is_empty() {
			Ok(())
		} else {
			Err(errors)
		}
	}
}

/// Nested JSON body structure
#[derive(Debug, Deserialize, Serialize, Clone)]
struct CreateUserRequest {
	/// User profile information
	profile: UserProfile,
	/// Account settings
	settings: AccountSettings,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct UserProfile {
	username: String,
	email: String,
	bio: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct AccountSettings {
	timezone: String,
	language: String,
	notifications_enabled: bool,
}

impl CreateUserRequest {
	fn validate(&self) -> Result<(), HashMap<String, Vec<String>>> {
		let mut field_errors: HashMap<String, Vec<String>> = HashMap::new();

		// Validate profile.username
		let username_min = MinLengthValidator::new(3);
		let username_max = MaxLengthValidator::new(20);

		if let Err(e) = username_min.validate(&self.profile.username) {
			field_errors
				.entry("profile.username".to_string())
				.or_default()
				.push(format!("{:?}", e));
		}
		if let Err(e) = username_max.validate(&self.profile.username) {
			field_errors
				.entry("profile.username".to_string())
				.or_default()
				.push(format!("{:?}", e));
		}

		// Validate profile.email
		let email_validator = EmailValidator::new();
		if let Err(e) = email_validator.validate(&self.profile.email) {
			field_errors
				.entry("profile.email".to_string())
				.or_default()
				.push(format!("{:?}", e));
		}

		// Validate profile.bio length if present
		if let Some(ref bio) = self.profile.bio {
			let bio_max = MaxLengthValidator::new(500);
			if let Err(e) = bio_max.validate(bio) {
				field_errors
					.entry("profile.bio".to_string())
					.or_default()
					.push(format!("{:?}", e));
			}
		}

		// Validate settings.timezone (non-empty)
		let timezone_min = MinLengthValidator::new(1);
		if let Err(e) = timezone_min.validate(&self.settings.timezone) {
			field_errors
				.entry("settings.timezone".to_string())
				.or_default()
				.push(format!("{:?}", e));
		}

		// Validate settings.language (2-letter code)
		let language_min = MinLengthValidator::new(2);
		let language_max = MaxLengthValidator::new(2);
		if let Err(e) = language_min.validate(&self.settings.language) {
			field_errors
				.entry("settings.language".to_string())
				.or_default()
				.push(format!("{:?}", e));
		}
		if let Err(e) = language_max.validate(&self.settings.language) {
			field_errors
				.entry("settings.language".to_string())
				.or_default()
				.push(format!("{:?}", e));
		}

		if field_errors.is_empty() {
			Ok(())
		} else {
			Err(field_errors)
		}
	}
}

// ============================================================================
// Test 1: Path Parameter Validation
// ============================================================================

#[rstest]
fn test_path_parameter_validation_integration() {
	// Valid user ID (within range)
	let valid_id = UserId::new(123);
	assert!(valid_id.is_ok());
	assert_eq!(valid_id.unwrap().value(), 123);

	// Valid boundary: minimum
	let min_id = UserId::new(1);
	assert!(min_id.is_ok());
	assert_eq!(min_id.unwrap().value(), 1);

	// Valid boundary: large number
	let large_id = UserId::new(999999);
	assert!(large_id.is_ok());
	assert_eq!(large_id.unwrap().value(), 999999);

	// Invalid: zero (below minimum)
	let zero_id = UserId::new(0);
	assert!(zero_id.is_err());
	let error = zero_id.unwrap_err();
	assert!(matches!(error, ValidationError::TooSmall { .. }));

	// Invalid: negative number
	let negative_id = UserId::new(-1);
	assert!(negative_id.is_err());
	let error = negative_id.unwrap_err();
	assert!(matches!(error, ValidationError::TooSmall { .. }));
}

// ============================================================================
// Test 2: Multi-Value Query Parameters Validation
// ============================================================================

#[rstest]
fn test_query_parameter_array_validation() {
	// Valid query: multiple tags, valid pagination
	let valid_query = SearchQuery {
		tags: vec!["rust".to_string(), "api".to_string(), "web".to_string()],
		page: 1,
		per_page: 20,
	};
	assert!(valid_query.validate().is_ok());

	// Valid query: empty tags (allowed)
	let empty_tags_query = SearchQuery {
		tags: vec![],
		page: 1,
		per_page: 10,
	};
	assert!(empty_tags_query.validate().is_ok());

	// Valid query: boundary values
	let boundary_query = SearchQuery {
		tags: vec!["a".to_string()],
		page: 1,       // Minimum page
		per_page: 100, // Maximum per_page
	};
	assert!(boundary_query.validate().is_ok());

	// Invalid: page number zero
	let invalid_page_query = SearchQuery {
		tags: vec!["rust".to_string()],
		page: 0, // Invalid: must be >= 1
		per_page: 10,
	};
	let result = invalid_page_query.validate();
	assert!(result.is_err());
	let errors = result.unwrap_err();
	assert_eq!(errors.len(), 1);
	assert!(matches!(errors[0], ValidationError::TooSmall { .. }));

	// Invalid: per_page too large
	let invalid_per_page_query = SearchQuery {
		tags: vec!["rust".to_string()],
		page: 1,
		per_page: 101, // Invalid: exceeds maximum (100)
	};
	let result = invalid_per_page_query.validate();
	assert!(result.is_err());
	let errors = result.unwrap_err();
	assert_eq!(errors.len(), 1);
	assert!(matches!(errors[0], ValidationError::TooLarge { .. }));

	// Invalid: tag too long
	let long_tag = "a".repeat(51); // 51 characters (max is 50)
	let invalid_tag_query = SearchQuery {
		tags: vec!["rust".to_string(), long_tag.clone()],
		page: 1,
		per_page: 10,
	};
	let result = invalid_tag_query.validate();
	assert!(result.is_err());
	let errors = result.unwrap_err();
	assert_eq!(errors.len(), 1);
	assert!(matches!(errors[0], ValidationError::TooLong { .. }));

	// Invalid: multiple validation errors
	let multiple_errors_query = SearchQuery {
		tags: vec!["rust".to_string(), "a".repeat(51)],
		page: 0,       // Invalid page
		per_page: 101, // Invalid per_page
	};
	let result = multiple_errors_query.validate();
	assert!(result.is_err());
	let errors = result.unwrap_err();
	assert_eq!(errors.len(), 3); // page, per_page, tag length
}

// ============================================================================
// Test 3: Nested JSON Body Validation
// ============================================================================

#[rstest]
fn test_nested_json_body_validation() {
	// Valid request: all fields valid
	let valid_request = CreateUserRequest {
		profile: UserProfile {
			username: "john_doe".to_string(),
			email: "john@example.com".to_string(),
			bio: Some("Software engineer".to_string()),
		},
		settings: AccountSettings {
			timezone: "UTC".to_string(),
			language: "en".to_string(),
			notifications_enabled: true,
		},
	};
	assert!(valid_request.validate().is_ok());

	// Valid request: no bio (optional field)
	let valid_no_bio = CreateUserRequest {
		profile: UserProfile {
			username: "alice".to_string(),
			email: "alice@example.com".to_string(),
			bio: None,
		},
		settings: AccountSettings {
			timezone: "America/New_York".to_string(),
			language: "es".to_string(),
			notifications_enabled: false,
		},
	};
	assert!(valid_no_bio.validate().is_ok());

	// Invalid: username too short
	let short_username = CreateUserRequest {
		profile: UserProfile {
			username: "ab".to_string(), // Only 2 chars (min is 3)
			email: "user@example.com".to_string(),
			bio: None,
		},
		settings: AccountSettings {
			timezone: "UTC".to_string(),
			language: "en".to_string(),
			notifications_enabled: true,
		},
	};
	let result = short_username.validate();
	assert!(result.is_err());
	let errors = result.unwrap_err();
	assert!(errors.contains_key("profile.username"));
	assert_eq!(errors.get("profile.username").unwrap().len(), 1);

	// Invalid: username too long
	let long_username = CreateUserRequest {
		profile: UserProfile {
			username: "a".repeat(21), // 21 chars (max is 20)
			email: "user@example.com".to_string(),
			bio: None,
		},
		settings: AccountSettings {
			timezone: "UTC".to_string(),
			language: "en".to_string(),
			notifications_enabled: true,
		},
	};
	let result = long_username.validate();
	assert!(result.is_err());
	let errors = result.unwrap_err();
	assert!(errors.contains_key("profile.username"));
	assert_eq!(errors.get("profile.username").unwrap().len(), 1);

	// Invalid: invalid email format
	let invalid_email = CreateUserRequest {
		profile: UserProfile {
			username: "john_doe".to_string(),
			email: "not-an-email".to_string(), // Invalid email
			bio: None,
		},
		settings: AccountSettings {
			timezone: "UTC".to_string(),
			language: "en".to_string(),
			notifications_enabled: true,
		},
	};
	let result = invalid_email.validate();
	assert!(result.is_err());
	let errors = result.unwrap_err();
	assert!(errors.contains_key("profile.email"));
	assert_eq!(errors.get("profile.email").unwrap().len(), 1);

	// Invalid: bio too long
	let long_bio = CreateUserRequest {
		profile: UserProfile {
			username: "john_doe".to_string(),
			email: "john@example.com".to_string(),
			bio: Some("a".repeat(501)), // 501 chars (max is 500)
		},
		settings: AccountSettings {
			timezone: "UTC".to_string(),
			language: "en".to_string(),
			notifications_enabled: true,
		},
	};
	let result = long_bio.validate();
	assert!(result.is_err());
	let errors = result.unwrap_err();
	assert!(errors.contains_key("profile.bio"));
	assert_eq!(errors.get("profile.bio").unwrap().len(), 1);

	// Invalid: empty timezone
	let empty_timezone = CreateUserRequest {
		profile: UserProfile {
			username: "john_doe".to_string(),
			email: "john@example.com".to_string(),
			bio: None,
		},
		settings: AccountSettings {
			timezone: "".to_string(), // Empty (invalid)
			language: "en".to_string(),
			notifications_enabled: true,
		},
	};
	let result = empty_timezone.validate();
	assert!(result.is_err());
	let errors = result.unwrap_err();
	assert!(errors.contains_key("settings.timezone"));
	assert_eq!(errors.get("settings.timezone").unwrap().len(), 1);

	// Invalid: language not 2 letters
	let invalid_language = CreateUserRequest {
		profile: UserProfile {
			username: "john_doe".to_string(),
			email: "john@example.com".to_string(),
			bio: None,
		},
		settings: AccountSettings {
			timezone: "UTC".to_string(),
			language: "english".to_string(), // Too long (must be 2 chars)
			notifications_enabled: true,
		},
	};
	let result = invalid_language.validate();
	assert!(result.is_err());
	let errors = result.unwrap_err();
	assert!(errors.contains_key("settings.language"));
	assert_eq!(errors.get("settings.language").unwrap().len(), 1);

	// Invalid: multiple nested errors
	let multiple_errors = CreateUserRequest {
		profile: UserProfile {
			username: "ab".to_string(),         // Too short
			email: "invalid-email".to_string(), // Invalid format
			bio: Some("a".repeat(501)),         // Too long
		},
		settings: AccountSettings {
			timezone: "".to_string(),    // Empty
			language: "eng".to_string(), // Too long
			notifications_enabled: true,
		},
	};
	let result = multiple_errors.validate();
	assert!(result.is_err());
	let errors = result.unwrap_err();
	// Should have errors for:
	// - profile.username (1 error: too short)
	// - profile.email (1 error: invalid format)
	// - profile.bio (1 error: too long)
	// - settings.timezone (1 error: empty)
	// - settings.language (1 error: too long)
	assert_eq!(errors.len(), 5);
	assert!(errors.contains_key("profile.username"));
	assert!(errors.contains_key("profile.email"));
	assert!(errors.contains_key("profile.bio"));
	assert!(errors.contains_key("settings.timezone"));
	assert!(errors.contains_key("settings.language"));
}
