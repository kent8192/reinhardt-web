//! Default naming convention for ManyToMany intermediate tables.
//!
//! This module is the single source of truth for the default through-table and
//! foreign-key column names that `reinhardt-db` synthesises when a user-declared
//! `ManyToMany` relationship does not provide an explicit `through`,
//! `source_field`, or `target_field`.
//!
//! Three call sites depend on these conventions and must stay byte-for-byte
//! aligned, otherwise migrations diverge from runtime queries:
//!
//! 1. `crate::orm::many_to_many_accessor::ManyToManyAccessor` — runtime queries.
//! 2. `crate::migrations::autodetector::MigrationAutodetector::create_intermediate_table_for_m2m`
//!    — writes the intermediate table into `to_state`.
//! 3. `crate::migrations::autodetector::MigrationAutodetector::detect_created_many_to_many`
//!    — looks the intermediate table up in `from_state` to decide whether to
//!    emit a `CreateModel` for it.
//!
//! The module deliberately lives at the crate root rather than under
//! `migrations::` because the `orm` and `migrations` features are independent
//! — gating these helpers behind `migrations` would break an `orm`-only build
//! at the `ManyToManyAccessor` use-site.
//!
//! Issue #4659 was a direct consequence of these three sites drifting apart;
//! PR #4663 fixed the drift with a manual lockstep-comment chain. This module
//! replaces that chain so future drift is a compile-time impossibility rather
//! than a review burden. See issue #4665 for context.

/// Convert a PascalCase / camelCase / dotted / dashed name to snake_case.
///
/// Local copy of the ASCII-only algorithm used by
/// `crate::migrations::autodetector::to_snake_case`. Duplicated here so this
/// module compiles without the `migrations` feature; the two implementations
/// MUST stay aligned. Consolidating the multiple `to_snake_case` copies in
/// `reinhardt-db` is tracked as a follow-up to #4665.
fn to_snake_case(name: &str) -> String {
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

/// Default name for a ManyToMany intermediate (through) table.
///
/// Returns `format!("{source_table}_{field_name}")` after lower-casing both
/// inputs. The source table already encodes the app label via the Django
/// convention `{app}_{model}`, so we deliberately do not re-prefix with the
/// app label — that would double-prefix in practice.
///
/// # Examples
///
/// ```
/// use reinhardt_db::m2m_naming::default_through_table;
///
/// assert_eq!(default_through_table("auth_user", "groups"), "auth_user_groups");
/// assert_eq!(default_through_table("Auth_User", "Groups"), "auth_user_groups");
/// ```
pub fn default_through_table(source_table: &str, field_name: &str) -> String {
	format!(
		"{}_{}",
		source_table.to_lowercase(),
		field_name.to_lowercase()
	)
}

/// Default `(source_column, target_column)` for a ManyToMany intermediate table.
///
/// Returns `("from_{snake(source_model)}_id", "to_{snake(target_model)}_id")`.
/// The `from_`/`to_` prefixes make the columns unambiguous even when the
/// relationship is self-referential (`source_model == target_model`), which is
/// the exact case a column convention like `{table}_id` cannot represent.
///
/// # Examples
///
/// ```
/// use reinhardt_db::m2m_naming::default_m2m_columns;
///
/// assert_eq!(
///     default_m2m_columns("User", "Group"),
///     ("from_user_id".to_string(), "to_group_id".to_string()),
/// );
///
/// // Self-referential (User <-> User "follows" relationship) — names stay distinct.
/// assert_eq!(
///     default_m2m_columns("User", "User"),
///     ("from_user_id".to_string(), "to_user_id".to_string()),
/// );
/// ```
pub fn default_m2m_columns(source_model: &str, target_model: &str) -> (String, String) {
	(
		format!("from_{}_id", to_snake_case(source_model)),
		format!("to_{}_id", to_snake_case(target_model)),
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn through_table_basic() {
		assert_eq!(
			default_through_table("auth_user", "groups"),
			"auth_user_groups"
		);
	}

	#[test]
	fn through_table_lowercases_mixed_case() {
		assert_eq!(
			default_through_table("Auth_User", "Groups"),
			"auth_user_groups"
		);
		assert_eq!(
			default_through_table("AUTH_USER", "GROUPS"),
			"auth_user_groups"
		);
	}

	#[test]
	fn through_table_self_referential_field() {
		// "User" model with a self-referential "following" field still works —
		// the source table only appears once in the table name.
		assert_eq!(
			default_through_table("auth_user", "following"),
			"auth_user_following"
		);
	}

	#[test]
	fn m2m_columns_basic() {
		assert_eq!(
			default_m2m_columns("User", "Group"),
			("from_user_id".to_string(), "to_group_id".to_string()),
		);
	}

	#[test]
	fn m2m_columns_self_referential_stays_distinct() {
		// Self-referential M2M (e.g. User.following -> User) must produce two
		// distinct column names, otherwise the intermediate table cannot model
		// a directed relationship.
		let (from, to) = default_m2m_columns("User", "User");
		assert_eq!(from, "from_user_id");
		assert_eq!(to, "to_user_id");
		assert_ne!(from, to);
	}

	#[test]
	fn m2m_columns_handles_pascal_case_model() {
		assert_eq!(
			default_m2m_columns("BlogPost", "Tag"),
			("from_blog_post_id".to_string(), "to_tag_id".to_string()),
		);
	}

	#[test]
	fn m2m_columns_handles_acronym() {
		assert_eq!(
			default_m2m_columns("HTTPRequest", "APIKey"),
			(
				"from_http_request_id".to_string(),
				"to_api_key_id".to_string()
			),
		);
	}

	// Cross-validate the local `to_snake_case` against the autodetector's
	// implementation whenever both features are compiled. Turns the
	// "MUST stay aligned" requirement at the top of this file into an
	// automatic check that catches drift the next time either copy is
	// edited. See issue #4665 for the follow-up that will consolidate
	// these into a single function.
	#[cfg(feature = "migrations")]
	mod cross_validation {
		use super::to_snake_case as orm_copy;
		use crate::migrations::autodetector::to_snake_case as autodetector_copy;

		#[test]
		fn to_snake_case_matches_autodetector() {
			let cases = [
				"",
				"a",
				"A",
				"User",
				"BlogPost",
				"HTTPRequest",
				"APIKey",
				"XMLHTTPRequest",
				"already_snake",
				"Mixed-Case_Name",
				"public.users",
				"With Space",
				"ALLCAPS",
				"camelCase",
				"PascalCase",
			];
			for input in &cases {
				assert_eq!(
					orm_copy(input),
					autodetector_copy(input),
					"to_snake_case diverged for {input:?}"
				);
			}
		}
	}
}
