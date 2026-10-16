//! Redirect URL validation utilities for preventing open redirect attacks
//!
//! Provides functions to validate redirect URLs against a list of allowed hosts,
//! preventing attackers from redirecting users to malicious external sites.

use percent_encoding::percent_decode_str;
use std::collections::HashSet;
use url::Url;

/// Errors that can occur during redirect URL validation
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RedirectValidationError {
	/// URL uses a dangerous protocol (javascript:, data:, vbscript:)
	#[error("Dangerous protocol in redirect URL: {0}")]
	DangerousProtocol(String),

	/// URL redirects to an untrusted host
	#[error("Redirect to untrusted host: {0}")]
	UntrustedHost(String),

	/// URL is protocol-relative (//) which could redirect to external site
	#[error("Protocol-relative URL not allowed in redirects")]
	ProtocolRelative,

	/// URL contains encoded characters that could bypass validation
	#[error("Suspicious URL encoding detected")]
	SuspiciousEncoding,

	/// URL is malformed and cannot be parsed
	#[error("Malformed redirect URL: {0}")]
	MalformedUrl(String),
}

/// Validate a redirect URL against a set of allowed hosts.
///
/// This function performs comprehensive validation:
/// 1. Rejects dangerous protocols (javascript:, data:, vbscript:)
/// 2. Allows relative URLs starting with `/` (but not `//`)
/// 3. Rejects protocol-relative URLs (`//`)
/// 4. Validates absolute URL hosts against the allowed hosts list
/// 5. Detects URL-encoded bypass attempts
/// 6. Rejects backslashes and control characters (raw or percent-encoded),
///    because browsers normalize inputs such as `/\host` into `//host`
///
/// Non-ASCII paths, raw or percent-encoded as UTF-8, are accepted.
///
/// # Arguments
///
/// * `url` - The redirect URL to validate
/// * `allowed_hosts` - Set of allowed hostnames (e.g., `{"example.com", "www.example.com"}`)
///
/// # Errors
///
/// Returns `RedirectValidationError` if the URL is unsafe for redirection.
///
/// # Examples
///
/// ```
/// use reinhardt_core::security::redirect::validate_redirect_url;
/// use std::collections::HashSet;
///
/// let allowed: HashSet<String> = ["example.com"].iter().map(|s| s.to_string()).collect();
///
/// // Relative URLs are safe
/// assert!(validate_redirect_url("/dashboard", &allowed).is_ok());
///
/// // Allowed host is safe
/// assert!(validate_redirect_url("https://example.com/page", &allowed).is_ok());
///
/// // External host is rejected
/// assert!(validate_redirect_url("https://evil.com/phish", &allowed).is_err());
///
/// // Dangerous protocols are rejected
/// assert!(validate_redirect_url("javascript:alert(1)", &allowed).is_err());
/// ```
pub fn validate_redirect_url(
	url: &str,
	allowed_hosts: &HashSet<String>,
) -> Result<(), RedirectValidationError> {
	let trimmed = url.trim();

	if trimmed.is_empty() {
		return Ok(());
	}
	// Browsers read `\` as `/` and drop tabs, newlines and leading C0
	// controls, so `/\evil.com` or `/<TAB>/evil.com` navigates to `//evil.com`
	// even though it does not start with `//` here.
	if trimmed.contains('\\') || trimmed.chars().any(char::is_control) {
		return Err(RedirectValidationError::SuspiciousEncoding);
	}

	// Check for URL-encoded bypass attempts before any other processing.
	// Decoded bytes are read as UTF-8 so that encoded non-ASCII text is not
	// mistaken for C1 control characters.
	let decoded = percent_decode_str(trimmed).decode_utf8_lossy();
	if decoded != trimmed
		&& (decoded.contains('\\')
			|| decoded.starts_with("//")
			|| decoded.chars().any(char::is_control))
	{
		return Err(RedirectValidationError::SuspiciousEncoding);
	}
	if decoded != trimmed {
		// Re-check the decoded version for dangerous patterns
		if has_dangerous_protocol(&decoded) {
			return Err(RedirectValidationError::SuspiciousEncoding);
		}
	}

	// Check for dangerous protocols
	if has_dangerous_protocol(trimmed) {
		let protocol = trimmed
			.split(':')
			.next()
			.unwrap_or("unknown")
			.to_lowercase();
		return Err(RedirectValidationError::DangerousProtocol(protocol));
	}

	// Check for protocol-relative URLs (//)
	if trimmed.starts_with("//") {
		return Err(RedirectValidationError::ProtocolRelative);
	}

	// Allow relative URLs (starting with / but not //)
	if trimmed.starts_with('/') {
		return Ok(());
	}

	// Allow fragment-only URLs
	if trimmed.starts_with('#') {
		return Ok(());
	}

	// Allow query-only URLs
	if trimmed.starts_with('?') {
		return Ok(());
	}

	// Parse as absolute URL and validate host
	match Url::parse(trimmed) {
		Ok(parsed) => {
			// Verify the scheme is http or https
			match parsed.scheme() {
				"http" | "https" => {}
				scheme => {
					return Err(RedirectValidationError::DangerousProtocol(
						scheme.to_string(),
					));
				}
			}

			// Validate host against allowed hosts
			match parsed.host_str() {
				Some(host) => {
					if !allowed_hosts.contains(host) {
						return Err(RedirectValidationError::UntrustedHost(host.to_string()));
					}
					Ok(())
				}
				None => Err(RedirectValidationError::MalformedUrl(
					"URL has no host".to_string(),
				)),
			}
		}
		Err(_) => {
			// If it can't be parsed as absolute URL, treat as relative path
			// but only if it doesn't look like it has a scheme
			if trimmed.contains("://") || trimmed.contains(':') {
				Err(RedirectValidationError::MalformedUrl(trimmed.to_string()))
			} else {
				// Treat as relative URL
				Ok(())
			}
		}
	}
}

/// Check if a redirect URL is safe (convenience wrapper).
///
/// Returns `true` if the URL passes validation, `false` otherwise.
///
/// # Examples
///
/// ```
/// use reinhardt_core::security::redirect::is_safe_redirect;
/// use std::collections::HashSet;
///
/// let allowed: HashSet<String> = ["example.com"].iter().map(|s| s.to_string()).collect();
///
/// assert!(is_safe_redirect("/dashboard", &allowed));
/// assert!(is_safe_redirect("https://example.com/page", &allowed));
/// assert!(!is_safe_redirect("https://evil.com", &allowed));
/// assert!(!is_safe_redirect("javascript:alert(1)", &allowed));
/// ```
pub fn is_safe_redirect(url: &str, allowed_hosts: &HashSet<String>) -> bool {
	validate_redirect_url(url, allowed_hosts).is_ok()
}

/// Check if a URL string contains a dangerous protocol
fn has_dangerous_protocol(url: &str) -> bool {
	let lower = url.to_lowercase();
	let trimmed = lower.trim_start();

	trimmed.starts_with("javascript:")
		|| trimmed.starts_with("data:")
		|| trimmed.starts_with("vbscript:")
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	fn allowed_hosts() -> HashSet<String> {
		["example.com", "www.example.com", "sub.example.com"]
			.iter()
			.map(|s| s.to_string())
			.collect()
	}

	#[rstest]
	#[case("/dashboard")]
	#[case("/path/to/page")]
	#[case("/")]
	fn test_validate_redirect_url_allows_relative_urls(#[case] url: &str) {
		// Arrange
		let hosts = allowed_hosts();

		// Act
		let result = validate_redirect_url(url, &hosts);

		// Assert
		assert!(result.is_ok(), "Expected Ok for relative URL: {url}");
	}

	// Browsers read `\` as `/` and drop tabs, newlines and leading C0 controls,
	// so each of these reaches `//evil.test` in a browser.
	#[rstest]
	#[case("/\\evil.test/path")]
	#[case("\\\\evil.test/path")]
	#[case("\\/evil.test/path")]
	#[case("/\\example.com/path")]
	#[case("/\t/evil.test/path")]
	#[case("/\n/evil.test/path")]
	#[case("\u{1}//evil.test/path")]
	#[case("/%5cevil.test/path")]
	#[case("/%5Cevil.test/path")]
	#[case("/%2f%2fevil.test/path")]
	#[case("%2F%2Fevil.test/path")]
	fn test_validate_redirect_url_rejects_network_path_variants(#[case] url: &str) {
		// Arrange
		let hosts = allowed_hosts();

		// Act
		let result = validate_redirect_url(url, &hosts);

		// Assert
		assert_eq!(
			result,
			Err(RedirectValidationError::SuspiciousEncoding),
			"Expected SuspiciousEncoding for network-path variant: {url:?}"
		);
	}

	#[rstest]
	#[case("/日本語/")]
	#[case("/search?q=%E6%97%A5%E6%9C%AC")]
	#[case("/users/caf%C3%A9/")]
	#[case("/files/a%2Fb")]
	#[case("/login?next=/dashboard")]
	fn test_validate_redirect_url_allows_non_ascii_and_encoded_paths(#[case] url: &str) {
		// Arrange
		let hosts = allowed_hosts();

		// Act
		let result = validate_redirect_url(url, &hosts);

		// Assert
		assert_eq!(result, Ok(()), "Expected Ok for same-origin path: {url}");
	}

	#[rstest]
	#[case("https://example.com/page")]
	#[case("https://www.example.com/")]
	#[case("http://sub.example.com/path")]
	fn test_validate_redirect_url_allows_trusted_hosts(#[case] url: &str) {
		// Arrange
		let hosts = allowed_hosts();

		// Act
		let result = validate_redirect_url(url, &hosts);

		// Assert
		assert!(result.is_ok(), "Expected Ok for trusted host URL: {url}");
	}

	#[rstest]
	#[case("https://evil.com/phish")]
	#[case("https://attacker.org/steal")]
	#[case("http://malicious.net")]
	fn test_validate_redirect_url_rejects_untrusted_hosts(#[case] url: &str) {
		// Arrange
		let hosts = allowed_hosts();

		// Act
		let result = validate_redirect_url(url, &hosts);

		// Assert
		assert!(
			matches!(result, Err(RedirectValidationError::UntrustedHost(_))),
			"Expected UntrustedHost error for: {url}"
		);
	}

	#[rstest]
	#[case("javascript:alert(1)")]
	#[case("data:text/html,<script>alert(1)</script>")]
	#[case("vbscript:alert(1)")]
	#[case("JAVASCRIPT:alert(1)")]
	fn test_validate_redirect_url_rejects_dangerous_protocols(#[case] url: &str) {
		// Arrange
		let hosts = allowed_hosts();

		// Act
		let result = validate_redirect_url(url, &hosts);

		// Assert
		assert!(
			result.is_err(),
			"Expected error for dangerous protocol: {url}"
		);
	}

	#[rstest]
	fn test_validate_redirect_url_rejects_protocol_relative() {
		// Arrange
		let hosts = allowed_hosts();

		// Act
		let result = validate_redirect_url("//evil.com/path", &hosts);

		// Assert
		assert!(
			matches!(result, Err(RedirectValidationError::ProtocolRelative)),
			"Expected ProtocolRelative error"
		);
	}

	#[rstest]
	fn test_validate_redirect_url_allows_empty() {
		// Arrange
		let hosts = allowed_hosts();

		// Act
		let result = validate_redirect_url("", &hosts);

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_redirect_url_allows_fragment_only() {
		// Arrange
		let hosts = allowed_hosts();

		// Act
		let result = validate_redirect_url("#section", &hosts);

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_redirect_url_detects_encoded_bypass() {
		// Arrange
		let hosts = allowed_hosts();

		// Act - URL-encoded "javascript:alert(1)"
		let result = validate_redirect_url("%6Aavascript:alert(1)", &hosts);

		// Assert
		assert!(result.is_err(), "Expected error for encoded bypass attempt");
	}

	#[rstest]
	fn test_is_safe_redirect_convenience() {
		// Arrange
		let hosts = allowed_hosts();

		// Act & Assert
		assert!(is_safe_redirect("/dashboard", &hosts));
		assert!(is_safe_redirect("https://example.com", &hosts));
		assert!(!is_safe_redirect("https://evil.com", &hosts));
		assert!(!is_safe_redirect("javascript:alert(1)", &hosts));
	}
}
