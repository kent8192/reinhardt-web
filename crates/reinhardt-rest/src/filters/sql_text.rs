//! SQL text helpers for filter backends that return rendered SQL strings.
//!
//! [`FilterBackend`](super::FilterBackend) passes SQL as a string and has no
//! channel for bound parameters, so search text must be embedded as a string
//! literal. That literal has to keep its boundary under every lexer the SQL may
//! reach: MySQL and MariaDB with or without `NO_BACKSLASH_ESCAPES`, PostgreSQL
//! with either `standard_conforming_strings` setting, and SQLite. Doubling
//! single quotes is exact in all of them as long as the text contains no
//! backslash, while no backslash encoding means the same thing in every mode.
//! Text containing a backslash is therefore rejected rather than rewritten.
//! Line breaks are rejected as well, so inserted text cannot end a line comment
//! that one backend reads as code (see [`find_sql_keyword`]).

use super::backend::{find_clause_end, find_sql_keyword};
use super::{FilterError, FilterResult};

/// Keywords that end a top-level `WHERE` condition.
const WHERE_CLAUSE_END: [&str; 5] = ["GROUP BY", "ORDER BY", "LIMIT", "OFFSET", "HAVING"];

/// Render `text` as a single-quoted SQL string literal.
///
/// Returns [`FilterError::InvalidParameter`] when `text` contains a backslash,
/// whose meaning depends on the backend's escape mode, a NUL character, which
/// PostgreSQL text cannot hold, or a line break.
pub(super) fn quoted_text(text: &str) -> FilterResult<String> {
	if text.contains(['\\', '\0', '\n', '\r']) {
		return Err(FilterError::InvalidParameter(
			"search text must not contain backslash, NUL or line break characters".to_string(),
		));
	}
	Ok(format!("'{}'", text.replace('\'', "''")))
}

/// Check that a configured column or function name is a plain or
/// dot-qualified SQL identifier before it is written into SQL unquoted.
pub(super) fn validate_identifier(name: &str, kind: &str) -> FilterResult<()> {
	let is_identifier = name.split('.').all(|part| {
		let mut chars = part.chars();
		chars
			.next()
			.is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
			&& chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
	});
	if is_identifier {
		Ok(())
	} else {
		Err(FilterError::InvalidParameter(format!(
			"{kind} must be a plain or dot-qualified SQL identifier"
		)))
	}
}

/// Add `condition` to the first `WHERE` clause of `sql`, or insert a new
/// `WHERE` clause before the first clause that must follow it.
///
/// Both conditions are parenthesized so that an `OR` on either side cannot
/// escape the conjunction. Clause detection shares the limits and errors of
/// [`find_sql_keyword`].
pub(super) fn and_where(sql: &str, condition: &str) -> FilterResult<String> {
	let (prefix, existing, remainder) = match find_sql_keyword(sql, "WHERE")? {
		Some(where_pos) => {
			let content_start = where_pos + "WHERE".len();
			let content_end = find_clause_end(sql, content_start, &WHERE_CLAUSE_END)?;
			(
				&sql[..where_pos],
				Some(sql[content_start..content_end].trim()),
				&sql[content_end..],
			)
		}
		None => {
			let insert_pos = find_clause_end(sql, 0, &WHERE_CLAUSE_END)?;
			(&sql[..insert_pos], None, &sql[insert_pos..])
		}
	};

	let mut result = prefix.trim_end().to_string();
	result.push_str(" WHERE (");
	result.push_str(condition);
	result.push(')');
	if let Some(existing) = existing {
		result.push_str(" AND (");
		result.push_str(existing);
		result.push(')');
	}
	let remainder = remainder.trim();
	if !remainder.is_empty() {
		result.push(' ');
		result.push_str(remainder);
	}
	Ok(result)
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	#[case::plain("rust", "'rust'")]
	#[case::quote("it's", "'it''s'")]
	#[case::quote_only("'", "''''")]
	#[case::non_ascii("ﬀ résumé", "'ﬀ résumé'")]
	fn quoted_text_doubles_single_quotes(#[case] text: &str, #[case] expected: &str) {
		// Act
		let literal = quoted_text(text).unwrap();

		// Assert
		assert_eq!(literal, expected);
	}

	#[rstest]
	#[case::backslash_quote("x\\' OR 1=1 -- ")]
	#[case::trailing_backslash("x\\")]
	#[case::nul("x\0y")]
	#[case::line_feed("x\ny")]
	#[case::carriage_return("x\ry")]
	fn quoted_text_rejects_mode_dependent_characters(#[case] text: &str) {
		// Act
		let result = quoted_text(text);

		// Assert
		assert!(
			matches!(result, Err(FilterError::InvalidParameter(_))),
			"expected InvalidParameter, got {result:?}"
		);
	}

	#[rstest]
	#[case::plain("title")]
	#[case::qualified("articles.title")]
	#[case::underscore("_score_v2")]
	fn validate_identifier_accepts_plain_names(#[case] name: &str) {
		// Act
		let result = validate_identifier(name, "field name");

		// Assert
		assert!(result.is_ok(), "expected {name:?} to be accepted");
	}

	#[rstest]
	#[case::empty("")]
	#[case::leading_digit("1title")]
	#[case::empty_segment("articles..title")]
	#[case::space("title desc")]
	#[case::call("score('x', title)")]
	#[case::comment("title--")]
	#[case::quoted("`title`")]
	fn validate_identifier_rejects_sql_structure(#[case] name: &str) {
		// Act
		let result = validate_identifier(name, "field name");

		// Assert
		assert!(
			matches!(result, Err(FilterError::InvalidParameter(_))),
			"expected {name:?} to be rejected, got {result:?}"
		);
	}

	#[rstest]
	#[case::no_where("SELECT * FROM articles", "SELECT * FROM articles WHERE (c)")]
	#[case::existing_where(
		"SELECT * FROM articles WHERE tenant_id = 1 OR public = 1",
		"SELECT * FROM articles WHERE (c) AND (tenant_id = 1 OR public = 1)"
	)]
	#[case::lowercase_where(
		"select * from articles where tenant_id = 1",
		"select * from articles WHERE (c) AND (tenant_id = 1)"
	)]
	#[case::where_before_order_by(
		"SELECT * FROM articles WHERE tenant_id = 1 ORDER BY id LIMIT 5",
		"SELECT * FROM articles WHERE (c) AND (tenant_id = 1) ORDER BY id LIMIT 5"
	)]
	#[case::no_where_before_group_by(
		"SELECT tag FROM articles GROUP BY tag",
		"SELECT tag FROM articles WHERE (c) GROUP BY tag"
	)]
	#[case::where_in_earlier_literal(
		"SELECT id, 'x WHERE y' FROM articles WHERE tenant_id = 1",
		"SELECT id, 'x WHERE y' FROM articles WHERE (c) AND (tenant_id = 1)"
	)]
	#[case::where_in_dollar_quote(
		"SELECT $$ WHERE $$ FROM articles WHERE tenant_id = 1",
		"SELECT $$ WHERE $$ FROM articles WHERE (c) AND (tenant_id = 1)"
	)]
	#[case::clause_keyword_in_literal(
		"SELECT * FROM articles WHERE note = 'x ORDER BY y' ORDER BY id",
		"SELECT * FROM articles WHERE (c) AND (note = 'x ORDER BY y') ORDER BY id"
	)]
	fn and_where_parenthesizes_both_conditions(#[case] sql: &str, #[case] expected: &str) {
		// Act
		let result = and_where(sql, "c").unwrap();

		// Assert
		assert_eq!(result, expected);
	}
}
