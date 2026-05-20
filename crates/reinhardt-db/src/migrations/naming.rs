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
//! Issue #4659 was a direct consequence of these three sites drifting apart;
//! PR #4663 fixed the drift with a manual lockstep-comment chain. This module
//! replaces that chain so future drift is a compile-time impossibility rather
//! than a review burden. See issue #4665 for context.

use super::autodetector::to_snake_case;

/// Default name for a ManyToMany intermediate (through) table.
///
/// Returns `format!("{source_table}_{field_name}")` after lower-casing both
/// inputs. The source table already encodes the app label via the Django
/// convention `{app}_{model}`, so we deliberately do not re-prefix with the
/// app label — that would double-prefix in practice.
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_db::migrations::naming::default_through_table;
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
/// ```rust,ignore
/// use reinhardt_db::migrations::naming::default_m2m_columns;
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
}
