//! Credential redaction utilities for safe logging and debug output

/// Redacts credentials from a URL string for safe display in logs and debug output.
///
/// Handles common database URL formats:
/// - `postgres://user:password@host:5432/db` -> `postgres://***:***@host:5432/db`
/// - `mysql://root:secret@localhost/mydb` -> `mysql://***:***@localhost/mydb`
/// - URLs without credentials are returned unchanged
///
/// # Examples
///
/// ```
/// use reinhardt_conf::settings::redact::redact_url_credentials;
///
/// assert_eq!(
///     redact_url_credentials("postgres://user:pass@localhost/db"),
///     "postgres://***:***@localhost/db"
/// );
///
/// assert_eq!(
///     redact_url_credentials("postgres://localhost/db"),
///     "postgres://localhost/db"
/// );
///
/// // Non-URL strings are returned as [REDACTED]
/// assert_eq!(
///     redact_url_credentials("not-a-url"),
///     "[REDACTED]"
/// );
/// ```
pub fn redact_url_credentials(url: &str) -> String {
	// Try to find the scheme separator
	let Some(scheme_end) = url.find("://") else {
		return "[REDACTED]".to_string();
	};

	let after_scheme = &url[scheme_end + 3..];

	// Find the @ separator (credentials end here)
	let Some(at_pos) = after_scheme.find('@') else {
		// No credentials in URL
		return url.to_string();
	};

	let scheme_part = &url[..scheme_end + 3];
	let host_part = &after_scheme[at_pos..]; // includes the @

	format!("{scheme_part}***:***{host_part}")
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	#[case(
		"postgres://user:password@localhost:5432/mydb",
		"postgres://***:***@localhost:5432/mydb"
	)]
	#[case(
		"mysql://root:secret@127.0.0.1:3306/app",
		"mysql://***:***@127.0.0.1:3306/app"
	)]
	#[case(
		"postgresql://admin:p%40ssw0rd@db.example.com/prod",
		"postgresql://***:***@db.example.com/prod"
	)]
	fn redact_url_with_credentials(#[case] input: &str, #[case] expected: &str) {
		// Arrange
		// (input provided via rstest parameters)

		// Act
		let result = redact_url_credentials(input);

		// Assert
		assert_eq!(result, expected);
	}

	#[rstest]
	#[case("postgres://localhost:5432/mydb", "postgres://localhost:5432/mydb")]
	#[case("sqlite:///path/to/db.sqlite3", "sqlite:///path/to/db.sqlite3")]
	fn redact_url_without_credentials_returns_unchanged(
		#[case] input: &str,
		#[case] expected: &str,
	) {
		// Arrange
		// (input provided via rstest parameters)

		// Act
		let result = redact_url_credentials(input);

		// Assert
		assert_eq!(result, expected);
	}

	#[rstest]
	#[case("not-a-url", "[REDACTED]")]
	#[case("plain-string", "[REDACTED]")]
	#[case("", "[REDACTED]")]
	fn redact_non_url_returns_redacted(#[case] input: &str, #[case] expected: &str) {
		// Arrange
		// (input provided via rstest parameters)

		// Act
		let result = redact_url_credentials(input);

		// Assert
		assert_eq!(result, expected);
	}

	#[rstest]
	fn redact_url_with_only_user() {
		// Arrange
		let url = "postgres://user@localhost/db";

		// Act
		let result = redact_url_credentials(url);

		// Assert
		assert_eq!(result, "postgres://***:***@localhost/db");
	}
}
