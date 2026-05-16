//! SQLite PRAGMA helpers shared between native introspection
//! (`SQLiteIntrospector::introspect_table` in `migrations/introspection.rs`)
//! and the `SchemaEditor`-scoped introspection used during table recreation
//! (`read_sqlite_table_via_editor` in `migrations/executor.rs`).
//!
//! Why a shared module? Both introspection paths must produce structurally
//! identical `ColumnDefinition`s for a given table, because `SqliteTableRecreation`
//! consumes the output to emit a new `CREATE TABLE`. PR #4449 (issue
//! #4447) duplicated the `dflt_value` quote handling and the raw
//! `PRAGMA <name>({identifier})` interpolation across both paths. This
//! module centralizes that logic so the two paths cannot drift.
//!
//! Default-value handling decision:
//!
//! `PRAGMA table_info(<table>).dflt_value` returns the literal SQL fragment
//! from the column's `DEFAULT` clause — including the surrounding quotes for
//! string defaults (e.g. `'pending'`, not `pending`). We preserve that
//! fragment **verbatim** here. The downstream `format!("DEFAULT {}", default)`
//! emission sites in `operations.rs` then round-trip correctly
//! (`DEFAULT 'pending'`, not the invalid `DEFAULT pending`), and
//! `operations.rs::convert_default_value` already handles both quoted and
//! plain forms, so the SeaQuery emission path round-trips correctly too.
//! See issue #4454 for context.

/// Wraps a SQLite identifier for use as the argument of a `PRAGMA`
/// command, e.g. `PRAGMA table_info('users')`.
///
/// SQLite accepts unquoted bare identifiers, double-quoted identifiers,
/// and single-quoted string-literal identifiers in PRAGMA arguments.
/// We use the single-quoted form because it tolerates the widest set of
/// characters (including reserved keywords used as table/index names)
/// and matches SQLite's documented PRAGMA syntax.
///
/// Embedded single quotes are escaped by doubling, per SQLite's standard
/// string-literal escaping rules.
///
/// All current callers pass identifiers that originate from internal
/// migration operations (no user-controlled data), so this helper exists
/// for robustness against quoted/special-character identifiers, not as
/// an injection-defense boundary.
pub(crate) fn quote_pragma_identifier(name: &str) -> String {
	format!("'{}'", name.replace('\'', "''"))
}

/// Returns the `dflt_value` from `PRAGMA table_info` in the form that
/// downstream DDL emission expects.
///
/// SQLite returns `dflt_value` as a raw SQL fragment (e.g. `'pending'`
/// including the surrounding quotes for string defaults,
/// `CURRENT_TIMESTAMP` for SQL constants, `42` for numeric defaults).
/// We preserve that fragment verbatim, only trimming surrounding
/// whitespace, so the two emission paths in `operations.rs` round-trip:
///
/// - The raw `format!("DEFAULT {}", default)` paths emit valid DDL
///   directly (e.g. `DEFAULT 'pending'`).
/// - `convert_default_value` already strips quotes from quoted strings
///   and re-quotes plain strings, so both forms feed it correctly.
pub(crate) fn normalize_default_value(raw: &str) -> String {
	raw.trim().to_string()
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::*;

	#[rstest]
	#[case("users", "'users'")]
	#[case("user_sessions", "'user_sessions'")]
	#[case("idx_users_email", "'idx_users_email'")]
	fn quote_pragma_identifier_wraps_plain_identifier_in_single_quotes(
		#[case] input: &str,
		#[case] expected: &str,
	) {
		// Arrange / Act
		let quoted = quote_pragma_identifier(input);

		// Assert
		assert_eq!(quoted, expected);
	}

	#[rstest]
	#[case("o'brien", "'o''brien'")]
	#[case("''", "''''''")]
	fn quote_pragma_identifier_doubles_embedded_single_quotes(
		#[case] input: &str,
		#[case] expected: &str,
	) {
		// Arrange / Act
		let quoted = quote_pragma_identifier(input);

		// Assert
		assert_eq!(quoted, expected);
	}

	#[rstest]
	#[case("'pending'", "'pending'")]
	#[case("'active'", "'active'")]
	#[case("CURRENT_TIMESTAMP", "CURRENT_TIMESTAMP")]
	#[case("42", "42")]
	#[case("3.14", "3.14")]
	#[case("NULL", "NULL")]
	fn normalize_default_value_preserves_sql_fragment_verbatim(
		#[case] raw: &str,
		#[case] expected: &str,
	) {
		// Arrange / Act
		let normalized = normalize_default_value(raw);

		// Assert
		assert_eq!(normalized, expected);
	}

	#[rstest]
	#[case("  'pending'  ", "'pending'")]
	#[case("\tCURRENT_TIMESTAMP\n", "CURRENT_TIMESTAMP")]
	fn normalize_default_value_trims_surrounding_whitespace(
		#[case] raw: &str,
		#[case] expected: &str,
	) {
		// Arrange / Act
		let normalized = normalize_default_value(raw);

		// Assert
		assert_eq!(normalized, expected);
	}
}
