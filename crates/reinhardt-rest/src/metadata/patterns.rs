//! Regular expression validation patterns for field validation

use serde::{Deserialize, Serialize};

/// Common validation patterns for field validation
///
/// # Examples
///
/// ```
/// use reinhardt_rest::metadata::ValidationPattern;
///
/// let pattern = ValidationPattern::email();
/// assert!(pattern.is_valid("user@example.com"));
/// assert!(!pattern.is_valid("invalid-email"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationPattern {
	/// The regular expression pattern
	pub pattern: String,
	/// Description of what the pattern validates
	pub description: String,
	/// Example values that match this pattern
	#[serde(skip_serializing_if = "Option::is_none")]
	pub examples: Option<Vec<String>>,
}

impl ValidationPattern {
	/// Creates a new validation pattern
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::metadata::ValidationPattern;
	///
	/// let pattern = ValidationPattern::new(
	///     r"^\d{3}-\d{4}$",
	///     "Phone number format (XXX-XXXX)"
	/// );
	/// assert_eq!(pattern.pattern, r"^\d{3}-\d{4}$");
	/// ```
	pub fn new(pattern: impl Into<String>, description: impl Into<String>) -> Self {
		Self {
			pattern: pattern.into(),
			description: description.into(),
			examples: None,
		}
	}

	/// Creates a new validation pattern with examples
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::metadata::ValidationPattern;
	///
	/// let pattern = ValidationPattern::with_examples(
	///     r"^\d{3}-\d{4}$",
	///     "Phone number format",
	///     vec!["123-4567", "555-0100"]
	/// );
	/// assert_eq!(pattern.examples.as_ref().unwrap().len(), 2);
	/// ```
	pub fn with_examples(
		pattern: impl Into<String>,
		description: impl Into<String>,
		examples: Vec<impl Into<String>>,
	) -> Self {
		Self {
			pattern: pattern.into(),
			description: description.into(),
			examples: Some(examples.into_iter().map(|e| e.into()).collect()),
		}
	}

	/// Email address pattern
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::metadata::ValidationPattern;
	///
	/// let pattern = ValidationPattern::email();
	/// assert!(pattern.is_valid("user@example.com"));
	/// assert!(pattern.is_valid("test.user+tag@example.co.uk"));
	/// assert!(!pattern.is_valid("invalid-email"));
	/// assert!(!pattern.is_valid("@example.com"));
	/// ```
	pub fn email() -> Self {
		Self::with_examples(
			r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$",
			"Email address",
			vec!["user@example.com", "test.user@example.co.uk"],
		)
	}

	/// URL pattern
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::metadata::ValidationPattern;
	///
	/// let pattern = ValidationPattern::url();
	/// assert!(pattern.is_valid("https://example.com"));
	/// assert!(pattern.is_valid("http://example.com/path"));
	/// assert!(!pattern.is_valid("not-a-url"));
	/// ```
	pub fn url() -> Self {
		Self::with_examples(
			r"^https?://[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}(/.*)?$",
			"URL",
			vec!["https://example.com", "http://example.com/path"],
		)
	}

	/// UUID pattern (version 4)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::metadata::ValidationPattern;
	///
	/// let pattern = ValidationPattern::uuid();
	/// assert!(pattern.is_valid("123e4567-e89b-12d3-a456-426614174000"));
	/// assert!(!pattern.is_valid("not-a-uuid"));
	/// ```
	pub fn uuid() -> Self {
		Self::with_examples(
			r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$",
			"UUID",
			vec!["123e4567-e89b-12d3-a456-426614174000"],
		)
	}

	/// Alphanumeric pattern (letters and numbers only)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::metadata::ValidationPattern;
	///
	/// let pattern = ValidationPattern::alphanumeric();
	/// assert!(pattern.is_valid("abc123"));
	/// assert!(pattern.is_valid("TestUser123"));
	/// assert!(!pattern.is_valid("user@name"));
	/// assert!(!pattern.is_valid("user-name"));
	/// ```
	pub fn alphanumeric() -> Self {
		Self::with_examples(
			r"^[a-zA-Z0-9]+$",
			"Alphanumeric characters only",
			vec!["abc123", "TestUser"],
		)
	}

	/// Slug pattern (URL-friendly identifier)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::metadata::ValidationPattern;
	///
	/// let pattern = ValidationPattern::slug();
	/// assert!(pattern.is_valid("my-blog-post"));
	/// assert!(pattern.is_valid("article-123"));
	/// assert!(!pattern.is_valid("My Blog Post"));
	/// assert!(!pattern.is_valid("article_123"));
	/// ```
	pub fn slug() -> Self {
		Self::with_examples(
			r"^[a-z0-9]+(?:-[a-z0-9]+)*$",
			"URL slug (lowercase, hyphens only)",
			vec!["my-blog-post", "article-123"],
		)
	}

	/// Phone number pattern (international format)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::metadata::ValidationPattern;
	///
	/// let pattern = ValidationPattern::phone();
	/// assert!(pattern.is_valid("+1-234-567-8900"));
	/// assert!(pattern.is_valid("+81-90-1234-5678"));
	/// assert!(!pattern.is_valid("123-456-7890")); // Missing country code
	/// ```
	pub fn phone() -> Self {
		Self::with_examples(
			r"^\+[1-9]\d{0,3}-\d{1,4}-\d{1,4}-\d{1,9}$",
			"Phone number (international format with country code)",
			vec!["+1-234-567-8900", "+81-90-1234-5678"],
		)
	}

	/// Hex color pattern
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::metadata::ValidationPattern;
	///
	/// let pattern = ValidationPattern::hex_color();
	/// assert!(pattern.is_valid("#FF5733"));
	/// assert!(pattern.is_valid("#fff"));
	/// assert!(!pattern.is_valid("FF5733")); // Missing #
	/// assert!(!pattern.is_valid("#GG5733")); // Invalid hex
	/// ```
	pub fn hex_color() -> Self {
		Self::with_examples(
			r"^#([A-Fa-f0-9]{6}|[A-Fa-f0-9]{3})$",
			"Hexadecimal color code",
			vec!["#FF5733", "#fff"],
		)
	}

	/// IP address pattern (IPv4)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::metadata::ValidationPattern;
	///
	/// let pattern = ValidationPattern::ipv4();
	/// assert!(pattern.is_valid("192.168.1.1"));
	/// assert!(pattern.is_valid("10.0.0.1"));
	/// assert!(!pattern.is_valid("256.1.1.1")); // Invalid range
	/// assert!(!pattern.is_valid("192.168.1")); // Incomplete
	/// ```
	pub fn ipv4() -> Self {
		Self::with_examples(
			r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$",
			"IPv4 address",
			vec!["192.168.1.1", "10.0.0.1"],
		)
	}

	/// Date pattern (YYYY-MM-DD)
	///
	/// Note: This pattern only validates the format, not the validity of the date itself.
	/// For example, it will accept "2023-13-01" (invalid month) or "2023-02-30" (invalid day).
	/// Use a proper date parsing library for full validation.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::metadata::ValidationPattern;
	///
	/// let pattern = ValidationPattern::date();
	/// assert!(pattern.is_valid("2023-12-25"));
	/// assert!(pattern.is_valid("2024-01-01"));
	/// assert!(!pattern.is_valid("23-12-25")); // Wrong format
	/// assert!(!pattern.is_valid("2023/12/25")); // Wrong separator
	/// ```
	pub fn date() -> Self {
		Self::with_examples(
			r"^\d{4}-\d{2}-\d{2}$",
			"Date in YYYY-MM-DD format (format validation only)",
			vec!["2023-12-25", "2024-01-01"],
		)
	}

	/// Time pattern (HH:MM:SS)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::metadata::ValidationPattern;
	///
	/// let pattern = ValidationPattern::time();
	/// assert!(pattern.is_valid("14:30:00"));
	/// assert!(pattern.is_valid("09:05:30"));
	/// assert!(!pattern.is_valid("25:00:00")); // Invalid hour
	/// assert!(!pattern.is_valid("14:30")); // Missing seconds
	/// ```
	pub fn time() -> Self {
		Self::with_examples(
			r"^([01]\d|2[0-3]):([0-5]\d):([0-5]\d)$",
			"Time in HH:MM:SS format",
			vec!["14:30:00", "09:05:30"],
		)
	}

	/// Validates if a value matches this pattern
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::metadata::ValidationPattern;
	///
	/// let pattern = ValidationPattern::email();
	/// assert!(pattern.is_valid("user@example.com"));
	/// assert!(!pattern.is_valid("invalid"));
	/// ```
	pub fn is_valid(&self, value: &str) -> bool {
		regex::Regex::new(&self.pattern)
			.map(|re| re.is_match(value))
			.unwrap_or(false)
	}

	/// Returns the pattern as a string for use in OpenAPI schemas
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::metadata::ValidationPattern;
	///
	/// let pattern = ValidationPattern::email();
	/// assert!(pattern.as_openapi_pattern().starts_with('^'));
	/// ```
	pub fn as_openapi_pattern(&self) -> &str {
		&self.pattern
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_email_pattern() {
		let pattern = ValidationPattern::email();

		// Valid emails
		assert!(pattern.is_valid("user@example.com"));
		assert!(pattern.is_valid("test.user@example.com"));
		assert!(pattern.is_valid("user+tag@example.co.uk"));
		assert!(pattern.is_valid("user_name@example.com"));

		// Invalid emails
		assert!(!pattern.is_valid("invalid-email"));
		assert!(!pattern.is_valid("@example.com"));
		assert!(!pattern.is_valid("user@"));
		assert!(!pattern.is_valid("user@.com"));
	}

	#[rstest]
	fn test_url_pattern() {
		let pattern = ValidationPattern::url();

		// Valid URLs
		assert!(pattern.is_valid("https://example.com"));
		assert!(pattern.is_valid("http://example.com"));
		assert!(pattern.is_valid("https://example.com/path"));
		assert!(pattern.is_valid("https://sub.example.com"));

		// Invalid URLs
		assert!(!pattern.is_valid("not-a-url"));
		assert!(!pattern.is_valid("ftp://example.com"));
		assert!(!pattern.is_valid("example.com"));
	}

	#[rstest]
	fn test_uuid_pattern() {
		let pattern = ValidationPattern::uuid();

		// Valid UUIDs
		assert!(pattern.is_valid("123e4567-e89b-12d3-a456-426614174000"));
		assert!(pattern.is_valid("550e8400-e29b-41d4-a716-446655440000"));

		// Invalid UUIDs
		assert!(!pattern.is_valid("not-a-uuid"));
		assert!(!pattern.is_valid("123e4567-e89b-12d3-a456"));
		assert!(!pattern.is_valid("123e4567e89b12d3a456426614174000"));
	}

	#[rstest]
	fn test_alphanumeric_pattern() {
		let pattern = ValidationPattern::alphanumeric();

		// Valid alphanumeric
		assert!(pattern.is_valid("abc123"));
		assert!(pattern.is_valid("TestUser"));
		assert!(pattern.is_valid("ABC"));
		assert!(pattern.is_valid("123"));

		// Invalid (contains special characters)
		assert!(!pattern.is_valid("user@name"));
		assert!(!pattern.is_valid("user-name"));
		assert!(!pattern.is_valid("user_name"));
		assert!(!pattern.is_valid("user name"));
	}

	#[rstest]
	fn test_slug_pattern() {
		let pattern = ValidationPattern::slug();

		// Valid slugs
		assert!(pattern.is_valid("my-blog-post"));
		assert!(pattern.is_valid("article-123"));
		assert!(pattern.is_valid("simple"));

		// Invalid slugs
		assert!(!pattern.is_valid("My Blog Post"));
		assert!(!pattern.is_valid("article_123"));
		assert!(!pattern.is_valid("-start-hyphen"));
		assert!(!pattern.is_valid("end-hyphen-"));
	}

	#[rstest]
	fn test_phone_pattern() {
		let pattern = ValidationPattern::phone();

		// Valid phone numbers
		assert!(pattern.is_valid("+1-234-567-8900"));
		assert!(pattern.is_valid("+81-90-1234-5678"));
		assert!(pattern.is_valid("+44-20-7946-0958"));

		// Invalid phone numbers
		assert!(!pattern.is_valid("123-456-7890")); // Missing +
		assert!(!pattern.is_valid("+1234567890")); // No separators
		assert!(!pattern.is_valid("1-234-567-8900")); // Missing +
	}

	#[rstest]
	fn test_hex_color_pattern() {
		let pattern = ValidationPattern::hex_color();

		// Valid colors
		assert!(pattern.is_valid("#FF5733"));
		assert!(pattern.is_valid("#fff"));
		assert!(pattern.is_valid("#000000"));
		assert!(pattern.is_valid("#ABC"));

		// Invalid colors
		assert!(!pattern.is_valid("FF5733")); // Missing #
		assert!(!pattern.is_valid("#GG5733")); // Invalid hex
		assert!(!pattern.is_valid("#FF57")); // Wrong length
	}

	#[rstest]
	fn test_ipv4_pattern() {
		let pattern = ValidationPattern::ipv4();

		// Valid IPs
		assert!(pattern.is_valid("192.168.1.1"));
		assert!(pattern.is_valid("10.0.0.1"));
		assert!(pattern.is_valid("255.255.255.255"));
		assert!(pattern.is_valid("0.0.0.0"));

		// Invalid IPs
		assert!(!pattern.is_valid("256.1.1.1")); // Out of range
		assert!(!pattern.is_valid("192.168.1")); // Incomplete
		assert!(!pattern.is_valid("192.168.1.1.1")); // Too many octets
	}

	#[rstest]
	fn test_date_pattern() {
		let pattern = ValidationPattern::date();

		// Valid format (not checking date validity)
		assert!(pattern.is_valid("2023-12-25"));
		assert!(pattern.is_valid("2024-01-01"));
		assert!(pattern.is_valid("1999-01-31"));

		// Invalid format
		assert!(!pattern.is_valid("23-12-25")); // Wrong format
		assert!(!pattern.is_valid("2023/12/25")); // Wrong separator

		// Note: This pattern only validates format, not date validity
		// So "2023-13-01" will pass the regex (but is an invalid date)
		assert!(pattern.is_valid("2023-13-01")); // Invalid month, but valid format
	}

	#[rstest]
	fn test_time_pattern() {
		let pattern = ValidationPattern::time();

		// Valid times
		assert!(pattern.is_valid("14:30:00"));
		assert!(pattern.is_valid("09:05:30"));
		assert!(pattern.is_valid("00:00:00"));
		assert!(pattern.is_valid("23:59:59"));

		// Invalid times
		assert!(!pattern.is_valid("25:00:00")); // Invalid hour
		assert!(!pattern.is_valid("14:30")); // Missing seconds
		assert!(!pattern.is_valid("14:60:00")); // Invalid minute
		assert!(!pattern.is_valid("14:30:60")); // Invalid second
	}

	#[rstest]
	fn test_custom_pattern() {
		let pattern = ValidationPattern::new(r"^\d{3}-\d{4}$", "Phone extension");

		assert!(pattern.is_valid("123-4567"));
		assert!(!pattern.is_valid("1234567"));
		assert!(!pattern.is_valid("abc-defg"));
	}

	#[rstest]
	fn test_pattern_with_examples() {
		let pattern = ValidationPattern::with_examples(
			r"^\d{3}$",
			"Three digit code",
			vec!["123", "456", "789"],
		);

		assert_eq!(pattern.examples.as_ref().unwrap().len(), 3);
		assert!(
			pattern
				.examples
				.as_ref()
				.unwrap()
				.contains(&"123".to_string())
		);
	}

	#[rstest]
	fn test_as_openapi_pattern() {
		let pattern = ValidationPattern::email();
		let openapi_pattern = pattern.as_openapi_pattern();

		assert!(!openapi_pattern.is_empty());
		assert!(openapi_pattern.starts_with('^'));
	}

	#[rstest]
	fn test_pattern_serialization() {
		let pattern = ValidationPattern::email();
		let json = serde_json::to_string(&pattern).unwrap();

		assert!(json.contains("pattern"));
		assert!(json.contains("description"));
		assert!(json.contains("examples"));
	}
}
