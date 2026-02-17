//! Utility functions for page rendering.
//!
//! This module provides common utility functions used across the page module.

use std::borrow::Cow;

/// Escapes HTML special characters in a string.
///
/// This function replaces the following characters:
/// - `&` → `&amp;`
/// - `<` → `&lt;`
/// - `>` → `&gt;`
/// - `"` → `&quot;`
/// - `'` → `&#x27;`
///
/// Returns a borrowed reference if no escaping is needed,
/// or an owned string if any characters were escaped.
pub(crate) fn html_escape(s: &str) -> Cow<'_, str> {
	if s.contains(['&', '<', '>', '"', '\'']) {
		let mut escaped = String::with_capacity(s.len() + 8);
		for c in s.chars() {
			match c {
				'&' => escaped.push_str("&amp;"),
				'<' => escaped.push_str("&lt;"),
				'>' => escaped.push_str("&gt;"),
				'"' => escaped.push_str("&quot;"),
				'\'' => escaped.push_str("&#x27;"),
				_ => escaped.push(c),
			}
		}
		Cow::Owned(escaped)
	} else {
		Cow::Borrowed(s)
	}
}

/// HTML boolean attributes that should only be set when the value is truthy.
///
/// Boolean attributes in HTML are special: the presence of the attribute alone
/// makes it active, regardless of its value. For example:
/// - `<button disabled="">` is disabled
/// - `<button disabled="false">` is STILL disabled
/// - `<button>` is NOT disabled (attribute absent)
///
/// This list follows the HTML5 specification for boolean attributes.
pub const BOOLEAN_ATTRS: &[&str] = &[
	"allowfullscreen",
	"async",
	"autofocus",
	"autoplay",
	"checked",
	"controls",
	"default",
	"defer",
	"disabled",
	"formnovalidate",
	"hidden",
	"inert",
	"ismap",
	"itemscope",
	"loop",
	"multiple",
	"muted",
	"nomodule",
	"novalidate",
	"open",
	"playsinline",
	"readonly",
	"required",
	"reversed",
	"selected",
	"truespeed",
];

/// Checks if a boolean attribute value should result in the attribute being set.
///
/// Returns `true` if the value is non-empty and not "false" or "0".
/// Returns `false` for empty strings, "false", or "0", meaning the attribute
/// should NOT be set (to properly disable the boolean attribute).
pub fn is_boolean_attr_truthy(value: &str) -> bool {
	!value.is_empty() && value != "false" && value != "0"
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_html_escape_no_special_chars() {
		assert_eq!(html_escape("Hello World"), Cow::Borrowed("Hello World"));
	}

	#[rstest]
	fn test_html_escape_ampersand() {
		assert_eq!(
			html_escape("a & b"),
			Cow::<str>::Owned("a &amp; b".to_string())
		);
	}

	#[rstest]
	fn test_html_escape_angle_brackets() {
		assert_eq!(
			html_escape("<div>"),
			Cow::<str>::Owned("&lt;div&gt;".to_string())
		);
	}

	#[rstest]
	fn test_html_escape_quotes() {
		assert_eq!(
			html_escape("\"test\" 'value'"),
			Cow::<str>::Owned("&quot;test&quot; &#x27;value&#x27;".to_string())
		);
	}

	#[rstest]
	fn test_is_boolean_attr_truthy() {
		// Truthy values
		assert!(is_boolean_attr_truthy("true"));
		assert!(is_boolean_attr_truthy("1"));
		assert!(is_boolean_attr_truthy("disabled"));
		assert!(is_boolean_attr_truthy("yes"));

		// Falsy values
		assert!(!is_boolean_attr_truthy(""));
		assert!(!is_boolean_attr_truthy("false"));
		assert!(!is_boolean_attr_truthy("0"));
	}
}
