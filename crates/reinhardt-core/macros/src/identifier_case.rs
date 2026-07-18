//! Identifier case conversion for generated framework names.

/// Convert a Rust identifier to `snake_case`.
///
/// Word boundaries are inserted between lowercase and uppercase characters,
/// and before the final uppercase character in an acronym when it is followed
/// by lowercase text. This keeps the convention predictable for names such as
/// `BlogPost` (`blog_post`) and `HTTPRoute` (`http_route`).
pub(crate) fn to_snake_case(name: &str) -> String {
	let chars: Vec<char> = name.chars().collect();
	let mut result = String::with_capacity(name.len() + 4);

	for (index, character) in chars.iter().copied().enumerate() {
		if character.is_ascii_uppercase() && index > 0 {
			let previous = chars[index - 1];
			let next_is_lowercase = chars
				.get(index + 1)
				.is_some_and(|next| next.is_ascii_lowercase());

			if previous.is_ascii_lowercase() || (previous.is_ascii_uppercase() && next_is_lowercase)
			{
				result.push('_');
			}
		}

		result.push(character.to_ascii_lowercase());
	}

	result
}
