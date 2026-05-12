//! Pattern length/segment limits and parameter value validators.
//!
//! These helpers are shared by `PathPattern` (for compile-time pattern
//! validation), `PathMatcher` (for post-match `path`-type traversal
//! rejection), and the URL reverser (for parameter injection defense).

/// Maximum allowed length for a URL pattern string in bytes.
/// Patterns exceeding this limit are rejected to prevent ReDoS attacks
/// from excessively long or complex regex patterns.
pub(super) const MAX_PATTERN_LENGTH: usize = 1024;

/// Maximum allowed number of path segments in a URL pattern.
/// Patterns with more segments than this are rejected to prevent
/// resource exhaustion from deeply nested URL structures.
pub(super) const MAX_PATH_SEGMENTS: usize = 32;

/// Maximum allowed size for compiled regex (in bytes).
/// This limits the compiled regex DFA size to prevent memory exhaustion.
pub(super) const MAX_REGEX_SIZE: usize = 1 << 20; // 1 MiB

/// Convert a type specifier to its corresponding regex pattern
///
/// This function maps type specifiers from `{<type:name>}` syntax
/// to appropriate regex patterns for URL matching.
///
/// # Supported Type Specifiers
///
/// | Type | Pattern | Description |
/// |------|---------|-------------|
/// | `int` | `[0-9]+` | Unsigned integer (legacy) |
/// | `i8`, `i16`, `i32`, `i64` | `-?[0-9]+` | Signed integers |
/// | `u8`, `u16`, `u32`, `u64` | `[0-9]+` | Unsigned integers |
/// | `f32`, `f64` | `-?[0-9]+(?:\.[0-9]+)?` | Floating point |
/// | `str` | `[^/]+` | Any non-slash characters (default) |
/// | `uuid` | UUID regex | UUID format |
/// | `slug` | `[a-z0-9]+(?:-[a-z0-9]+)*` | Lowercase slug |
/// | `path` | `.+` | Any characters **including** path separators (`/`); `..` segments are rejected by post-match validation |
/// | `bool` | `true\|false\|1\|0` | Boolean literals |
/// | `email` | Email regex | Email format |
/// | `date` | `[0-9]{4}-[0-9]{2}-[0-9]{2}` | ISO 8601 date |
pub(super) fn type_spec_to_regex(type_spec: &str) -> &'static str {
	match type_spec {
		// Basic types (legacy)
		"int" => r"[0-9]+",
		"str" => r"[^/]+",
		"uuid" => r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}",
		"slug" => r"[a-z0-9]+(?:-[a-z0-9]+)*",
		// Matches any characters including path separators (`/`).
		// A pattern like `/files/{<path:filepath>}` will match
		// `/files/a/b/c.txt`, capturing `a/b/c.txt` as a single value.
		// Directory traversal (`..` segments) is rejected by post-match
		// validation in extract_params() and match_path_linear().
		"path" => r".+",
		// Signed integers
		"i8" | "i16" | "i32" | "i64" => r"-?[0-9]+",
		// Unsigned integers
		"u8" | "u16" | "u32" | "u64" => r"[0-9]+",
		// Floating point
		"f32" | "f64" => r"-?[0-9]+(?:\.[0-9]+)?",
		// Other types
		"bool" => r"true|false|1|0",
		"email" => r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}",
		"date" => r"[0-9]{4}-[0-9]{2}-[0-9]{2}",
		// Default: treat as string
		_ => r"[^/]+",
	}
}

/// Validate that a matched path value does not contain directory traversal sequences.
///
/// This provides defense-in-depth for `path` type parameters by checking
/// extracted values for `..` segments that could enable path traversal.
///
/// Rejects:
/// - `..` as a path segment (forward-slash or backslash separated)
/// - Percent-encoded traversal sequences (`%2e`, `%2f`, `%2E`, `%2F`, `%5c`, `%5C`)
/// - Null bytes (literal or encoded `%00`)
/// - Absolute paths starting with `/` or `\`
pub(super) fn validate_path_param(value: &str) -> bool {
	// Reject null bytes
	if value.contains('\0') {
		return false;
	}

	// Reject percent-encoded dangerous characters:
	// %2e / %2E = '.', %2f / %2F = '/', %5c / %5C = '\', %00 = null
	let lower = value.to_ascii_lowercase();
	if lower.contains("%2e")
		|| lower.contains("%2f")
		|| lower.contains("%5c")
		|| lower.contains("%00")
	{
		return false;
	}

	// Reject absolute paths
	if value.starts_with('/') || value.starts_with('\\') {
		return false;
	}

	// Check for `..` as a complete path segment (forward-slash separated)
	for segment in value.split('/') {
		if segment == ".." {
			return false;
		}
	}
	// Also reject backslash-separated `..` segments
	for segment in value.split('\\') {
		if segment == ".." {
			return false;
		}
	}

	true
}

/// Validate a parameter value for URL reversal against injection attacks.
///
/// Rejects values containing:
/// - Path separators (`/`, `\`)
/// - Query string delimiters (`?`)
/// - Fragment identifiers (`#`)
/// - Null bytes
/// - Path traversal sequences (`..`)
/// - Percent-encoded dangerous characters (`%2f`, `%2e`, `%5c`, `%3f`, `%23`, `%00`)
pub(crate) fn validate_reverse_param(value: &str) -> bool {
	// Reject null bytes
	if value.contains('\0') {
		return false;
	}

	// Reject path separators and URL-special characters
	if value.contains('/') || value.contains('\\') || value.contains('?') || value.contains('#') {
		return false;
	}

	// Reject path traversal
	if value == ".." || value.starts_with("../") || value.ends_with("/..") || value.contains("/../")
	{
		return false;
	}

	// Reject percent-encoded dangerous characters
	let lower = value.to_ascii_lowercase();
	if lower.contains("%2f")
		|| lower.contains("%2e")
		|| lower.contains("%5c")
		|| lower.contains("%3f")
		|| lower.contains("%23")
		|| lower.contains("%00")
	{
		return false;
	}

	true
}
