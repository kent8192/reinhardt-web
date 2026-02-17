//! SQL reserved words list.
//!
//! This module contains a list of SQL reserved words from major database systems
//! (PostgreSQL, MySQL, SQLite) to prevent their use as identifiers.

/// Checks if a word is a SQL reserved word.
///
/// This function checks against reserved words from:
/// - SQL standard (SQL-2016)
/// - PostgreSQL
/// - MySQL
/// - SQLite
///
/// # Examples
///
/// ```
/// use reinhardt_core::validators::reserved::is_sql_reserved_word;
///
/// assert!(is_sql_reserved_word("select"));
/// assert!(is_sql_reserved_word("table"));
/// assert!(!is_sql_reserved_word("user_profile"));
/// ```
pub fn is_sql_reserved_word(word: &str) -> bool {
	matches!(
		word,
		// SQL Standard Keywords
		"select"
            | "from"
            | "where"
            | "insert"
            | "update"
            | "delete"
            | "create"
            | "drop"
            | "alter"
            | "table"
            | "index"
            | "view"
            | "trigger"
            | "procedure"
            | "function"
            | "database"
            | "schema"
            | "primary"
            | "foreign"
            | "key"
            | "references"
            | "constraint"
            | "unique"
            | "check"
            | "default"
            | "null"
            | "not"
            | "and"
            | "or"
            | "in"
            | "between"
            | "like"
            | "is"
            | "exists"
            | "case"
            | "when"
            | "then"
            | "else"
            | "end"
            | "as"
            | "on"
            | "using"
            | "join"
            | "inner"
            | "outer"
            | "left"
            | "right"
            | "full"
            | "cross"
            | "union"
            | "intersect"
            | "except"
            | "group"
            | "by"
            | "having"
            | "order"
            | "asc"
            | "desc"
            | "limit"
            | "offset"
            | "distinct"
            | "all"
            | "any"
            | "some"
            | "into"
            | "values"
            | "set"
            | "cascade"
            | "restrict"
            | "grant"
            | "revoke"
            | "commit"
            | "rollback"
            | "begin"
            | "transaction"
            | "savepoint"
            // Data Types
            | "integer"
            | "int"
            | "smallint"
            | "bigint"
            | "decimal"
            | "numeric"
            | "real"
            | "float"
            | "double"
            | "char"
            | "varchar"
            | "text"
            | "boolean"
            | "date"
            | "time"
            | "timestamp"
            | "interval"
            | "blob"
            | "clob"
            // PostgreSQL Specific
            | "serial"
            | "bigserial"
            | "smallserial"
            | "json"
            | "jsonb"
            | "uuid"
            | "array"
            | "hstore"
            | "returning"
            | "conflict"
            | "do"
            | "nothing"
            | "excluded"
            // MySQL Specific
            | "auto_increment"
            | "unsigned"
            | "zerofill"
            | "enum"
            | "show"
            | "describe"
            | "explain"
            // Additional Common Keywords
            | "with"
            | "recursive"
            | "over"
            | "partition"
            | "window"
            | "row"
            | "rows"
            | "range"
            | "unbounded"
            | "preceding"
            | "following"
            | "current"
            | "first"
            | "last"
            | "only"
            | "for"
            | "if"
            | "elseif"
            | "while"
            | "loop"
            | "repeat"
            | "until"
            | "declare"
            | "cursor"
            | "fetch"
            | "open"
            | "close"
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_common_reserved_words() {
		assert!(is_sql_reserved_word("select"));
		assert!(is_sql_reserved_word("from"));
		assert!(is_sql_reserved_word("where"));
		assert!(is_sql_reserved_word("table"));
		assert!(is_sql_reserved_word("create"));
		assert!(is_sql_reserved_word("drop"));
	}

	#[rstest]
	fn test_data_types() {
		assert!(is_sql_reserved_word("integer"));
		assert!(is_sql_reserved_word("varchar"));
		assert!(is_sql_reserved_word("timestamp"));
		assert!(is_sql_reserved_word("boolean"));
	}

	#[rstest]
	fn test_postgresql_keywords() {
		assert!(is_sql_reserved_word("serial"));
		assert!(is_sql_reserved_word("jsonb"));
		assert!(is_sql_reserved_word("returning"));
	}

	#[rstest]
	fn test_not_reserved() {
		assert!(!is_sql_reserved_word("user"));
		assert!(!is_sql_reserved_word("profile"));
		assert!(!is_sql_reserved_word("user_profile"));
		assert!(!is_sql_reserved_word("email"));
	}
}
