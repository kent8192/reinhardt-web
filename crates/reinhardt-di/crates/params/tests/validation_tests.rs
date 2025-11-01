//! Tests for parameter validation using ValidationConstraints

#[cfg(feature = "validation")]
mod tests {
	use reinhardt_params::{Path, Query, WithValidation};

	#[test]
	fn test_path_number_validation() {
		let path = Path(42i32);
		let constrained = path.min_value(0).max_value(100);

		// Test validation methods
		assert!(constrained.validate_number(&42).is_ok());
		assert!(constrained.validate_number(&0).is_ok());
		assert!(constrained.validate_number(&100).is_ok());
		assert!(constrained.validate_number(&-1).is_err());
		assert!(constrained.validate_number(&101).is_err());
	}

	#[test]
	fn test_path_string_validation() {
		let path = Path("test".to_string());
		let constrained = path.min_length(2).max_length(10);

		// Test validation methods
		assert!(constrained.validate_string("test").is_ok());
		assert!(constrained.validate_string("ab").is_ok());
		assert!(constrained.validate_string("0123456789").is_ok());
		assert!(constrained.validate_string("a").is_err());
		assert!(constrained.validate_string("this is too long").is_err());
	}

	#[test]
	fn test_query_validation_min_only() {
		let query = Query(10i32);
		let constrained = query.min_value(5);

		assert!(constrained.validate_number(&10).is_ok());
		assert!(constrained.validate_number(&5).is_ok());
		assert!(constrained.validate_number(&100).is_ok());
		assert!(constrained.validate_number(&4).is_err());
	}

	#[test]
	fn test_query_validation_max_only() {
		let query = Query(10i32);
		let constrained = query.max_value(50);

		assert!(constrained.validate_number(&10).is_ok());
		assert!(constrained.validate_number(&50).is_ok());
		assert!(constrained.validate_number(&0).is_ok());
		assert!(constrained.validate_number(&51).is_err());
	}

	#[test]
	fn test_chained_string_constraints() {
		let path = Path("hello".to_string());
		let constrained = path.min_length(3).max_length(20);

		assert!(constrained.validate_string("hello").is_ok());
		assert!(constrained.validate_string("abc").is_ok());
		assert!(constrained.validate_string("12345678901234567890").is_ok());
		assert!(constrained.validate_string("ab").is_err());
		assert!(
			constrained
				.validate_string("123456789012345678901")
				.is_err()
		);
	}

	#[test]
	fn test_validation_constraints_deref() {
		let path = Path(42i32);
		let constrained = path.min_value(0).max_value(100);

		// Test Deref trait - access the inner value
		assert_eq!(constrained.0, 42);
	}

	#[test]
	fn test_validation_constraints_into_inner() {
		let path = Path("test".to_string());
		let constrained = path.min_length(2).max_length(10);

		// Test into_inner
		let inner = constrained.into_inner();
		assert_eq!(inner.0, "test");
	}

	#[test]
	fn test_float_validation() {
		let path = Path(3.14f64);
		let constrained = path.min_value(0.0).max_value(10.0);

		assert!(constrained.validate_number(&3.14).is_ok());
		assert!(constrained.validate_number(&0.0).is_ok());
		assert!(constrained.validate_number(&10.0).is_ok());
		assert!(constrained.validate_number(&-0.1).is_err());
		assert!(constrained.validate_number(&10.1).is_err());
	}

	/// Test regex pattern validation
	#[test]
	fn test_regex_validation() {
		let path = Path("abc123".to_string());
		let constrained = path.regex(r"^[a-z]+\d+$");

		// Valid patterns
		assert!(constrained.validate_string("abc123").is_ok());
		assert!(constrained.validate_string("test456").is_ok());

		// Invalid patterns
		assert!(constrained.validate_string("ABC123").is_err()); // uppercase
		assert!(constrained.validate_string("123abc").is_err()); // numbers first
		assert!(constrained.validate_string("abc").is_err()); // no numbers
	}

	/// Test email validation
	#[test]
	fn test_email_validation() {
		let path = Path("test@example.com".to_string());
		let constrained = path.email();

		// Valid emails
		assert!(constrained.validate_string("test@example.com").is_ok());
		assert!(
			constrained
				.validate_string("user.name@domain.co.jp")
				.is_ok()
		);
		assert!(constrained.validate_string("admin+tag@company.org").is_ok());

		// Invalid emails
		assert!(constrained.validate_string("invalid").is_err());
		assert!(constrained.validate_string("@example.com").is_err());
		assert!(constrained.validate_string("test@").is_err());
		assert!(constrained.validate_string("test @example.com").is_err());
	}

	/// Test URL validation
	#[test]
	fn test_url_validation() {
		let path = Path("https://example.com".to_string());
		let constrained = path.url();

		// Valid URLs
		assert!(constrained.validate_string("https://example.com").is_ok());
		assert!(constrained.validate_string("http://localhost:8000").is_ok());
		assert!(
			constrained
				.validate_string("https://example.com/path?query=value")
				.is_ok()
		);

		// Invalid URLs
		assert!(constrained.validate_string("invalid").is_err());
		assert!(constrained.validate_string("ftp://example.com").is_err()); // scheme not allowed
		assert!(constrained.validate_string("//example.com").is_err());
	}

	/// Test combining multiple validation constraints
	#[test]
	fn test_combined_validations() {
		let path = Path("test@example.com".to_string());
		let constrained = path.min_length(5).max_length(50).email();

		// Valid: meets all constraints
		assert!(constrained.validate_string("test@example.com").is_ok());
		assert!(constrained.validate_string("a@example.com").is_ok());

		// Invalid: too short
		assert!(constrained.validate_string("a@b.").is_err());

		// Invalid: too long (>50 chars)
		assert!(
			constrained
				.validate_string("verylongemailaddressthatshouldexceedfiftycharacters@example.com")
				.is_err()
		);

		// Invalid: not an email
		assert!(constrained.validate_string("not-an-email").is_err());
	}

	/// Test regex combined with length validation
	#[test]
	fn test_regex_with_length_validation() {
		let path = Path("abc123".to_string());
		let constrained = path.min_length(3).max_length(10).regex(r"^[a-z]+\d+$");

		// Valid: meets all constraints
		assert!(constrained.validate_string("abc123").is_ok());
		assert!(constrained.validate_string("xyz9").is_ok());

		// Invalid: too short
		assert!(constrained.validate_string("ab").is_err());

		// Invalid: too long
		assert!(constrained.validate_string("abcdefghijk123").is_err());

		// Invalid: doesn't match regex
		assert!(constrained.validate_string("ABC123").is_err());
	}

	/// Test builder pattern chaining
	#[test]
	fn test_chained_constraints() {
		let path = Path("https://example.com/path".to_string());
		let constrained = path.min_length(10).max_length(100).url();

		// Valid
		assert!(
			constrained
				.validate_string("https://example.com/path")
				.is_ok()
		);

		// Invalid: too short
		assert!(constrained.validate_string("http://a").is_err());

		// Invalid: not a URL
		assert!(constrained.validate_string("not-a-url-string").is_err());

		// Test that we can access inner value through Deref
		assert_eq!(constrained.0, "https://example.com/path");
	}
}
