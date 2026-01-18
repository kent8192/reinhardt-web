//! Validator Integration Tests
//!
//! Tests validator functionality with database integration, covering:
//! - EmailValidator: RFC 5322 compliant email validation
//! - URLValidator: HTTP/HTTPS URL validation
//! - RangeValidator: Numeric range validation with boundary analysis
//! - RequiredValidator: Non-empty field validation
//! - MaxLengthValidator: Maximum length validation
//! - MinLengthValidator: Minimum length validation
//! - RegexValidator: Custom pattern validation
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container
//!
//! **Test Strategy:**
//! - Happy Path: Valid inputs
//! - Error Cases: Invalid inputs
//! - Boundary Analysis: Edge cases
//! - Equivalence Partitioning: rstest cases

use reinhardt_db::orm::validators::{
	EmailValidator, MaxLengthValidator, MinLengthValidator, RangeValidator, RegexValidator,
	RequiredValidator, URLValidator, Validator,
};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// EmailValidator Tests
// ============================================================================

/// Happy Path: EmailValidator - Valid email address validation
///
/// **Test Intent**: Verify that valid email address formats can be correctly validated
///
/// **Integration Point**: EmailValidator → PostgreSQL storage
///
/// **Test Cases**:
/// - Standard email address
/// - Local part with dots
/// - With plus sign
/// - With subdomain
#[rstest]
#[case("user@example.com", true)]
#[case("user.name@example.com", true)]
#[case("user+tag@example.com", true)]
#[case("test@subdomain.example.com", true)]
#[case("valid.email+tag@example.co.uk", true)]
#[tokio::test]
async fn test_email_validator_valid_emails(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	#[case] email: &str,
	#[case] expected_valid: bool,
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create test table for storing validated emails
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS validated_emails (
			id SERIAL PRIMARY KEY,
			email TEXT NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Validate email
	let validator = EmailValidator::new();
	let result = validator.validate(email);

	assert_eq!(
		result.is_ok(),
		expected_valid,
		"Email validation failed for: {}",
		email
	);

	if expected_valid {
		// Store validated email in database
		sqlx::query("INSERT INTO validated_emails (email) VALUES ($1)")
			.bind(email)
			.execute(pool.as_ref())
			.await
			.expect("Failed to insert email");

		// Verify stored email
		let stored_email: String =
			sqlx::query("SELECT email FROM validated_emails WHERE email = $1")
				.bind(email)
				.fetch_one(pool.as_ref())
				.await
				.expect("Failed to fetch email")
				.get(0);

		assert_eq!(stored_email, email);
	}
}

/// Error Cases: EmailValidator - Invalid email format validation
///
/// **Test Intent**: Verify that invalid email formats can be correctly rejected
///
/// **Test Cases**:
/// - No @ sign
/// - Multiple @ signs
/// - Empty local part
/// - Empty domain part
/// - Invalid domain
#[rstest]
#[case("invalid-email")]
#[case("@example.com")]
#[case("user@")]
#[case("user@@example.com")]
#[case("user@.com")]
#[case("user@example")]
#[case("")]
#[tokio::test]
async fn test_email_validator_invalid_emails(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	#[case] invalid_email: &str,
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let validator = EmailValidator::new();
	let result = validator.validate(invalid_email);

	assert!(
		result.is_err(),
		"Expected validation to fail for: {}",
		invalid_email
	);

	// Verify error message
	if let Err(e) = result {
		let error_msg = e.to_string();
		assert!(
			error_msg.contains("email") || error_msg.contains("valid"),
			"Error message should mention email or validation"
		);
	}
}

// ============================================================================
// URLValidator Tests
// ============================================================================

/// Happy Path: URLValidator - Valid URL validation
///
/// **Test Intent**: Verify that valid HTTP/HTTPS URLs can be correctly validated
///
/// **Test Cases**:
/// - Basic HTTPS URL
/// - HTTP URL
/// - URL with path
/// - URL with query parameters
#[rstest]
#[case("https://example.com")]
#[case("http://example.com")]
#[case("https://www.example.com/path/to/page")]
#[case("https://api.example.com/v1/users")]
#[case("https://example.com/search?q=test")]
#[tokio::test]
async fn test_url_validator_valid_urls(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	#[case] url: &str,
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create test table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS validated_urls (
			id SERIAL PRIMARY KEY,
			url TEXT NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Validate URL
	let validator = URLValidator::new();
	let result = validator.validate(url);

	assert!(result.is_ok(), "URL validation failed for: {}", url);

	// Store validated URL
	sqlx::query("INSERT INTO validated_urls (url) VALUES ($1)")
		.bind(url)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert URL");

	// Verify stored URL
	let stored_url: String = sqlx::query("SELECT url FROM validated_urls WHERE url = $1")
		.bind(url)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to fetch URL")
		.get(0);

	assert_eq!(stored_url, url);
}

/// Error Cases: URLValidator - Invalid URL format validation
///
/// **Test Intent**: Verify that invalid URLs can be correctly rejected
///
/// **Test Cases**:
/// - No scheme
/// - Invalid scheme
/// - No domain
/// - Malformed format
#[rstest]
#[case("example.com")]
#[case("ftp://example.com")]
#[case("http://")]
#[case("://example.com")]
#[case("not-a-url")]
#[case("")]
#[tokio::test]
async fn test_url_validator_invalid_urls(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	#[case] invalid_url: &str,
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let validator = URLValidator::new();
	let result = validator.validate(invalid_url);

	assert!(
		result.is_err(),
		"Expected validation to fail for: {}",
		invalid_url
	);
}

// ============================================================================
// RangeValidator Tests - Boundary Analysis
// ============================================================================

/// Boundary Analysis: RangeValidator - Minimum, maximum, and boundary value validation
///
/// **Test Intent**: Verify that boundary values of numeric ranges can be correctly validated
///
/// **Boundary Analysis**:
/// - Minimum value (min)
/// - Minimum value - 1 (invalid)
/// - Maximum value (max)
/// - Maximum value + 1 (invalid)
/// - Middle value (valid)
#[rstest]
#[case("0", Some(0), Some(100), true)] // Minimum value (within boundary)
#[case("-1", Some(0), Some(100), false)] // Minimum value - 1 (outside boundary)
#[case("100", Some(0), Some(100), true)] // Maximum value (within boundary)
#[case("101", Some(0), Some(100), false)] // Maximum value + 1 (outside boundary)
#[case("50", Some(0), Some(100), true)] // Middle value (within boundary)
#[case("18", Some(18), Some(65), true)] // Minimum value (age restriction)
#[case("17", Some(18), Some(65), false)] // Underage
#[case("65", Some(18), Some(65), true)] // Maximum age
#[case("66", Some(18), Some(65), false)] // Exceeds
#[tokio::test]
async fn test_range_validator_boundary_values(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	#[case] value: &str,
	#[case] min: Option<i64>,
	#[case] max: Option<i64>,
	#[case] expected_valid: bool,
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create test table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS validated_ranges (
			id SERIAL PRIMARY KEY,
			value INTEGER NOT NULL,
			min_value INTEGER,
			max_value INTEGER
		)",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Validate range
	let validator = RangeValidator::new(min, max);
	let result = validator.validate(value);

	assert_eq!(
		result.is_ok(),
		expected_valid,
		"Range validation failed for value: {}, min: {:?}, max: {:?}",
		value,
		min,
		max
	);

	if expected_valid {
		let numeric_value: i32 = value.parse().unwrap();
		// Store validated value
		sqlx::query(
			"INSERT INTO validated_ranges (value, min_value, max_value) VALUES ($1, $2, $3)",
		)
		.bind(numeric_value)
		.bind(min.map(|v| v as i32))
		.bind(max.map(|v| v as i32))
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert range value");

		// Verify stored value
		let stored_value: i32 = sqlx::query("SELECT value FROM validated_ranges WHERE value = $1")
			.bind(numeric_value)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to fetch value")
			.get(0);

		assert_eq!(stored_value, numeric_value);
	}
}

/// Boundary Analysis: RangeValidator - One-sided constraints only
///
/// **Test Intent**: Verify that constraints with only minimum or only maximum can be correctly validated
#[rstest]
#[case("10", Some(0), None, true)] // Minimum only - valid
#[case("-1", Some(0), None, false)] // Minimum only - invalid
#[case("50", None, Some(100), true)] // Maximum only - valid
#[case("101", None, Some(100), false)] // Maximum only - invalid
#[tokio::test]
async fn test_range_validator_one_sided_constraints(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	#[case] value: &str,
	#[case] min: Option<i64>,
	#[case] max: Option<i64>,
	#[case] expected_valid: bool,
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let validator = RangeValidator::new(min, max);
	let result = validator.validate(value);

	assert_eq!(result.is_ok(), expected_valid);
}

// ============================================================================
// Equivalence Partitioning: Various Validator Types
// ============================================================================

/// Equivalence Partitioning: RequiredValidator - Required field validation
///
/// **Test Intent**: Verify that empty strings and whitespace-only strings can be correctly rejected
#[rstest]
#[case("valid text", true)]
#[case("", false)] // Empty string
#[case("   ", false)] // Whitespace only
#[case("a", true)] // Single character
#[tokio::test]
async fn test_required_validator_equivalence_classes(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	#[case] value: &str,
	#[case] expected_valid: bool,
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let validator = RequiredValidator::new();
	let result = validator.validate(value);

	assert_eq!(result.is_ok(), expected_valid);
}

/// Equivalence Partitioning: MaxLengthValidator - Maximum length validation
///
/// **Test Intent**: Verify that maximum length constraints can be correctly validated
#[rstest]
#[case("abc", 5, true)] // Within limit
#[case("hello", 5, true)] // Boundary value (equal)
#[case("toolong", 5, false)] // Exceeds
#[case("", 5, true)] // Empty string
#[tokio::test]
async fn test_max_length_validator_equivalence_classes(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	#[case] value: &str,
	#[case] max_length: usize,
	#[case] expected_valid: bool,
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let validator = MaxLengthValidator::new(max_length);
	let result = validator.validate(value);

	assert_eq!(result.is_ok(), expected_valid);
}

/// Equivalence Partitioning: MinLengthValidator - Minimum length validation
///
/// **Test Intent**: Verify that minimum length constraints can be correctly validated
#[rstest]
#[case("hello world", 3, true)] // Above limit
#[case("abc", 3, true)] // Boundary value (equal)
#[case("ab", 3, false)] // Insufficient
#[case("", 1, false)] // Empty string
#[tokio::test]
async fn test_min_length_validator_equivalence_classes(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	#[case] value: &str,
	#[case] min_length: usize,
	#[case] expected_valid: bool,
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let validator = MinLengthValidator::new(min_length);
	let result = validator.validate(value);

	assert_eq!(result.is_ok(), expected_valid);
}

/// Equivalence Partitioning: RegexValidator - Custom pattern validation
///
/// **Test Intent**: Verify custom validation using regular expression patterns
#[rstest]
#[case(r"^\d{3}-\d{4}$", "123-4567", true)] // Postal code pattern - valid
#[case(r"^\d{3}-\d{4}$", "abc-defg", false)] // Postal code pattern - invalid
#[case(r"^[A-Z]{3}$", "ABC", true)] // 3 uppercase letters - valid
#[case(r"^[A-Z]{3}$", "abc", false)] // 3 uppercase letters - invalid with lowercase
#[case(r"^[A-Z]{3}$", "AB", false)] // 3 uppercase letters - insufficient length
#[tokio::test]
async fn test_regex_validator_equivalence_classes(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	#[case] pattern: &str,
	#[case] value: &str,
	#[case] expected_valid: bool,
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let validator = RegexValidator::new(pattern);
	let result = validator.validate(value);

	assert_eq!(result.is_ok(), expected_valid);
}

// ============================================================================
// Combined Validation Scenarios
// ============================================================================

/// Combined Validation: Validation combining multiple validators
///
/// **Test Intent**: Verify that multiple validators can be applied sequentially
///
/// **Scenario**: Comprehensive email address validation
/// - Required: Non-empty
/// - MaxLength: Maximum 254 characters (RFC 5321)
/// - Email: Correct email format
#[rstest]
#[tokio::test]
async fn test_combined_validators_email_comprehensive(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create test table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS comprehensive_validation (
			id SERIAL PRIMARY KEY,
			email TEXT NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	let test_email = "valid.user+tag@example.com";

	// Validator chain
	let required = RequiredValidator::new();
	let max_length = MaxLengthValidator::new(254);
	let email = EmailValidator::new();

	// Apply all validators
	assert!(required.validate(test_email).is_ok());
	assert!(max_length.validate(test_email).is_ok());
	assert!(email.validate(test_email).is_ok());

	// Store validated email
	sqlx::query("INSERT INTO comprehensive_validation (email) VALUES ($1)")
		.bind(test_email)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert email");

	// Verify
	let count: i64 = sqlx::query("SELECT COUNT(*) FROM comprehensive_validation")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count records")
		.get(0);

	assert_eq!(count, 1);
}

/// Combined Validation: Combined validation of URL + length constraints
///
/// **Test Intent**: Verify that URL validator can be combined with length constraints for validation
#[rstest]
#[case("https://example.com", true)]
#[case(
	"https://this-is-a-very-long-domain-name-that-exceeds-maximum-length-limit-for-testing-purposes.com",
	false
)]
#[tokio::test]
async fn test_combined_validators_url_with_length(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	#[case] url: &str,
	#[case] expected_valid: bool,
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let url_validator = URLValidator::new();
	let max_length_validator = MaxLengthValidator::new(50);

	let url_result = url_validator.validate(url);
	let length_result = max_length_validator.validate(url);

	let is_valid = url_result.is_ok() && length_result.is_ok();

	assert_eq!(is_valid, expected_valid);
}
