//! Shared utilities for HTML attribute name conversion.

/// Converts a Rust identifier to an HTML attribute name.
///
/// Strips `r#` prefix and converts `_` to `-`.
pub(crate) fn ident_to_html_attr_name(name: &str) -> String {
	let name = name.strip_prefix("r#").unwrap_or(name);
	name.replace('_', "-")
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	#[case("class", "class")]
	#[case("data_testid", "data-testid")]
	#[case("aria_label", "aria-label")]
	#[case("r#type", "type")]
	#[case("r#for", "for")]
	#[case("type", "type")]
	fn test_ident_to_html_attr_name(#[case] input: &str, #[case] expected: &str) {
		// Act
		let result = ident_to_html_attr_name(input);

		// Assert
		assert_eq!(result, expected);
	}
}
