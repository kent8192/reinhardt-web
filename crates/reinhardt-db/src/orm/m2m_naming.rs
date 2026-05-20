//! Canonical M2M default-naming helpers (orm-internal copy).
//!
//! The migration autodetector and the ORM runtime must agree exactly on
//! the default through-table name for an M2M field. The autodetector
//! lives behind the `migrations` feature flag, so the ORM cannot pull
//! its helpers in directly without forcing every `orm`-only consumer to
//! also enable `migrations` — exactly the pitfall that the previous
//! revision of this fix introduced.
//!
//! This module therefore keeps a small, feature-independent copy of the
//! snake_case conversion and the canonical M2M through-table rule. It
//! must stay in lockstep with `migrations::to_snake_case` and the
//! through-table compositions in
//! `crates/reinhardt-db/src/migrations/autodetector.rs`
//! (`create_intermediate_table_for_m2m` and
//! `detect_created_many_to_many`). If the two diverge, runtime M2M
//! reads/writes target a table that `makemigrations` never produced —
//! the regression #4659 surfaced.
//!
//! #4665 tracks promoting one of the two copies into a shared
//! feature-independent location so this duplication can go away.

/// Convert a name to snake_case. Mirrors `migrations::to_snake_case`.
///
/// Handles camelCase/PascalCase, acronyms (`HTTPResponse` →
/// `http_response`), and `_`/`-`/`.`/space separators.
pub(crate) fn to_snake_case(name: &str) -> String {
	if name.is_empty() {
		return String::new();
	}

	let mut result = String::with_capacity(name.len() + 4);
	let chars: Vec<char> = name.chars().collect();
	let mut prev_was_separator = true;

	for i in 0..chars.len() {
		let ch = chars[i];

		if ch == '_' || ch == '-' || ch == ' ' || ch == '.' {
			if !prev_was_separator && !result.is_empty() {
				result.push('_');
			}
			prev_was_separator = true;
		} else if ch.is_ascii_uppercase() {
			if !prev_was_separator && i > 0 {
				let prev = chars[i - 1];
				let next = chars.get(i + 1);

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

/// Canonical default through-table name for an M2M field.
///
/// `{source_table.to_lowercase()}_{to_snake_case(field_name)}`. Must
/// match what `MigrationAutodetector::create_intermediate_table_for_m2m`
/// and `detect_created_many_to_many` synthesize (see module-level docs).
pub(crate) fn default_through_table_name(source_table: &str, field_name: &str) -> String {
	format!("{}_{}", source_table.to_lowercase(), to_snake_case(field_name))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn snake_case_basics() {
		assert_eq!(to_snake_case("User"), "user");
		assert_eq!(to_snake_case("BlogPost"), "blog_post");
		assert_eq!(to_snake_case("HTTPResponse"), "http_response");
	}

	#[test]
	fn through_table_lowercases_source_and_snake_cases_field() {
		assert_eq!(
			default_through_table_name("AuthUser", "myField"),
			"authuser_my_field"
		);
		assert_eq!(
			default_through_table_name("dm_room", "members"),
			"dm_room_members"
		);
	}
}
