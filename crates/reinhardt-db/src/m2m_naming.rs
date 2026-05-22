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
/// Returns `format!("{source_table.to_lowercase()}_{snake_case(field_name)}")`.
/// The source table already encodes the app label via the Django
/// convention `{app}_{model}`, so we deliberately do not re-prefix with the
/// app label — that would double-prefix in practice. The field name is
/// snake_cased so a `field_name` like `FollowedBy` produces
/// `..._followed_by`, agreeing with the column convention enforced by the
/// migration autodetector and ORM accessor.
///
/// # Examples
///
/// ```
/// use reinhardt_db::m2m_naming::default_through_table;
///
/// assert_eq!(default_through_table("auth_user", "groups"), "auth_user_groups");
/// assert_eq!(default_through_table("Auth_User", "Groups"), "auth_user_groups");
/// assert_eq!(default_through_table("auth_user", "FollowedBy"), "auth_user_followed_by");
/// ```
pub fn default_through_table(source_table: &str, field_name: &str) -> String {
	format!(
		"{}_{}",
		source_table.to_lowercase(),
		to_snake_case(field_name)
	)
}

/// Default `(source_column, target_column)` for a ManyToMany intermediate table.
///
/// Both inputs are the *actual lowercased table names* of the source and
/// target models. The convention matches the ORM accessor's fallback
/// (`format!("{}_id", S::table_name().to_lowercase())` in
/// `crate::orm::many_to_many_accessor`) and the migration autodetector's
/// emit site in `crate::migrations::autodetector::generate_migrations`:
///
/// - Non-self-referential: `("{source_table}_id", "{target_table}_id")`
/// - Self-referential (`source_table == target_table`): the bare convention
///   would collide, so `from_/to_` prefixes are applied to keep the two
///   columns distinct.
///
/// # Examples
///
/// ```
/// use reinhardt_db::m2m_naming::default_m2m_columns;
///
/// assert_eq!(
///     default_m2m_columns("auth_user", "auth_group"),
///     ("auth_user_id".to_string(), "auth_group_id".to_string()),
/// );
///
/// // Self-referential (e.g. `User.following -> User`).
/// assert_eq!(
///     default_m2m_columns("auth_user", "auth_user"),
///     ("from_auth_user_id".to_string(), "to_auth_user_id".to_string()),
/// );
/// ```
pub fn default_m2m_columns(source_table: &str, target_table: &str) -> (String, String) {
	let source = source_table.to_lowercase();
	let target = target_table.to_lowercase();
	if source == target {
		(format!("from_{}_id", source), format!("to_{}_id", target))
	} else {
		(format!("{}_id", source), format!("{}_id", target))
	}
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
			default_m2m_columns("auth_user", "auth_group"),
			("auth_user_id".to_string(), "auth_group_id".to_string()),
		);
	}

	#[test]
	fn m2m_columns_self_referential_stays_distinct() {
		// Self-referential M2M (e.g. User.following -> User) must produce two
		// distinct column names, otherwise the intermediate table cannot model
		// a directed relationship.
		let (from, to) = default_m2m_columns("auth_user", "auth_user");
		assert_eq!(from, "from_auth_user_id");
		assert_eq!(to, "to_auth_user_id");
		assert_ne!(from, to);
	}

	#[test]
	fn m2m_columns_lowercases_inputs() {
		assert_eq!(
			default_m2m_columns("Auth_User", "Auth_Group"),
			("auth_user_id".to_string(), "auth_group_id".to_string()),
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
