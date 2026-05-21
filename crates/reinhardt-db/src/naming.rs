//! Identifier-naming helpers shared by the migration autodetector and the
//! ORM runtime.
//!
//! These functions are intentionally feature-flag-agnostic so that the
//! `orm` feature (which composes through-table names at runtime) and the
//! `migrations` feature (which composes the same names at autodetect time)
//! see exactly the same canonical rule. Keeping a single implementation
//! here prevents the runtime/migration divergence that #4659 surfaced.

/// Convert an identifier to `snake_case`.
///
/// Handles multiple separators (`_`, `.`, `-`, space) and camelCase
/// boundaries, including acronym runs (`HTTPRequest` -> `http_request`).
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_db::naming::to_snake_case;
///
/// assert_eq!(to_snake_case("BlogPost"), "blog_post");
/// assert_eq!(to_snake_case("HTTPRequest"), "http_request");
/// assert_eq!(to_snake_case("public.users"), "public_users");
/// ```
pub fn to_snake_case(name: &str) -> String {
	if name.is_empty() {
		return String::new();
	}

	let mut result = String::with_capacity(name.len() + 4);
	let chars: Vec<char> = name.chars().collect();
	let mut prev_was_separator = true; // Treat start as separator to avoid leading underscore

	for i in 0..chars.len() {
		let ch = chars[i];

		// Handle separators: _, -, space, .
		if ch == '_' || ch == '-' || ch == ' ' || ch == '.' {
			// Only add underscore if previous char was not a separator
			if !prev_was_separator && !result.is_empty() {
				result.push('_');
			}
			prev_was_separator = true;
		} else if ch.is_ascii_uppercase() {
			if !prev_was_separator && i > 0 {
				let prev = chars[i - 1];
				let next = chars.get(i + 1);

				// Add underscore if:
				// 1. Previous char is lowercase (normal camelCase boundary)
				// OR
				// 2. Previous char is uppercase AND next char exists AND is lowercase
				//    (this handles acronyms like HTTPRequest -> http_request)
				if prev.is_ascii_lowercase()
					|| (prev.is_ascii_uppercase() && next.is_some_and(|&n| n.is_ascii_lowercase()))
				{
					result.push('_');
				}
			}
			result.push(ch.to_ascii_lowercase());
			prev_was_separator = false;
		} else {
			result.push(ch.to_ascii_lowercase());
			prev_was_separator = false;
		}
	}

	result
}
