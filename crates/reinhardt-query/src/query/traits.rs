//! Query statement traits
//!
//! This module defines the core traits for building and executing SQL queries.

use std::{any::Any, fmt::Debug, ops::Range};

use crate::backend::{PostgresQueryBuilder, SqlWriter};
use crate::value::Values;

/// Replace parameter placeholders in SQL with inline value literals.
///
/// Supports both numbered (`$1, $2, ...`) and positional (`?`) placeholders.
/// This enables `to_string()` to produce complete SQL with values inlined,
/// matching reinhardt-query behavior for debugging and non-parameterized execution.
/// Quoted identifiers, string literals (including PostgreSQL dollar quotes),
/// and SQL comments are preserved. Only tokens in the original SQL are replaced;
/// placeholder-like text inside inserted values is never interpreted again.
/// This standalone helper uses PostgreSQL lexical rules and infers the
/// placeholder style. It cannot distinguish raw bind markers from generated
/// parameters once both are present in the SQL string. Built-in PostgreSQL
/// statements avoid that ambiguity by rendering managed values directly in
/// `to_string()`. Other backends use their own placeholder style and lexical rules
/// (including MySQL's default string escapes).
/// Numbered tokens take precedence over `?` tokens, which can be PostgreSQL
/// operators. Placeholders without a corresponding value are left unchanged.
/// Numbered placeholders use PostgreSQL literals; positional placeholders retain
/// generic literal syntax, including `X'...'` for bytes.
pub fn inline_params(sql: &str, values: &Values) -> String {
	inline_params_with_dialect(
		sql,
		values,
		InlineDialect {
			mysql: false,
			postgres: true,
			numbered: None,
		},
	)
}

#[derive(Clone, Copy)]
struct InlineDialect {
	mysql: bool,
	postgres: bool,
	numbered: Option<bool>,
}

fn inline_params_with_dialect(sql: &str, values: &Values, dialect: InlineDialect) -> String {
	if values.is_empty() {
		return sql.to_string();
	}

	let placeholders = placeholder_ranges(sql, dialect);
	let numbered = dialect.numbered.unwrap_or_else(|| {
		placeholders
			.iter()
			.any(|range| sql.as_bytes()[range.start] == b'$')
	});
	let mut result = String::with_capacity(sql.len());
	let mut copied_until = 0;
	let mut positional_index = 0;

	for range in placeholders {
		let token = &sql[range.clone()];
		let index = if let Some(number) = token.strip_prefix('$') {
			number.parse::<usize>().ok().and_then(|n| n.checked_sub(1))
		} else if !numbered {
			let index = positional_index;
			positional_index += 1;
			Some(index)
		} else {
			None
		};
		if let Some(value) = index.and_then(|index| values.0.get(index)) {
			result.push_str(&sql[copied_until..range.start]);
			let literal = if dialect.postgres && numbered {
				PostgresQueryBuilder::value_to_sql_literal(value)
			} else {
				value.to_sql_literal()
			};
			result.push_str(&literal);
			copied_until = range.end;
		}
	}
	result.push_str(&sql[copied_until..]);
	result
}

// Locate complete placeholder tokens before rendering any values. Byte scanning
// is safe here: delimiters are ASCII and slices end only at token boundaries.
fn placeholder_ranges(sql: &str, dialect: InlineDialect) -> Vec<Range<usize>> {
	let bytes = sql.as_bytes();
	let mut ranges = Vec::new();
	let mut i = 0;
	while i < bytes.len() {
		match bytes[i] {
			b'\'' | b'"' | b'`' if bytes[i] != b'`' || !dialect.postgres => {
				let escape = (dialect.mysql && matches!(bytes[i], b'\'' | b'"'))
					|| (dialect.postgres
						&& bytes[i] == b'\''
						&& i > 0 && matches!(bytes[i - 1], b'e' | b'E')
						&& (i == 1 || !is_identifier_continue(bytes[i - 2])));
				i = quoted_end(bytes, i, escape);
			}
			b'-' if bytes.get(i + 1) == Some(&b'-')
				&& (!dialect.mysql
					|| bytes
						.get(i + 2)
						.is_some_and(|ch| ch.is_ascii_whitespace() || ch.is_ascii_control())) =>
			{
				i += 2;
				while i < bytes.len() && !matches!(bytes[i], b'\n' | b'\r') {
					i += 1;
				}
			}
			b'#' if dialect.mysql => {
				while i < bytes.len() && !matches!(bytes[i], b'\n' | b'\r') {
					i += 1;
				}
			}
			b'/' if dialect.mysql
				&& bytes[i..].starts_with(b"/*!")
				&& (!bytes.get(i + 3).is_some_and(u8::is_ascii_digit)
					|| bytes[i + 3..]
						.iter()
						.take_while(|ch| ch.is_ascii_digit())
						.count() >= 5) =>
			{
				// Executable comment contents use SQL syntax, bounded by the comment end.
				let start = i + 3;
				let end = sql[start..]
					.find("*/")
					.map_or(bytes.len(), |offset| start + offset);
				for range in placeholder_ranges(&sql[start..end], dialect) {
					ranges.push(start + range.start..start + range.end);
				}
				i = (end + 2).min(bytes.len());
			}
			b'/' if bytes.get(i + 1) == Some(&b'*') => {
				i += 2;
				let mut depth = 1;
				while i < bytes.len() && depth > 0 {
					if dialect.postgres && bytes[i..].starts_with(b"/*") {
						depth += 1;
						i += 2;
					} else if bytes[i..].starts_with(b"*/") {
						depth -= 1;
						i += 2;
					} else {
						i += 1;
					}
				}
			}
			b'$' => {
				let start = i;
				i += 1;
				if dialect.numbered != Some(false) && bytes.get(i).is_some_and(u8::is_ascii_digit) {
					while bytes.get(i).is_some_and(u8::is_ascii_digit) {
						i += 1;
					}
					ranges.push(start..i);
				} else if dialect.postgres
					&& let Some(end) = dollar_quoted_end(sql, start)
				{
					i = end;
				}
			}
			b'?' if dialect.numbered != Some(true) => {
				ranges.push(i..i + 1);
				i += 1;
			}
			ch if is_identifier_start(ch) => {
				// Dollar signs may be part of unquoted PostgreSQL identifiers.
				i += 1;
				while i < bytes.len() && is_identifier_continue(bytes[i]) {
					i += 1;
				}
			}
			_ => i += 1,
		}
	}
	ranges
}

fn is_identifier_start(ch: u8) -> bool {
	ch.is_ascii_alphabetic() || ch == b'_' || !ch.is_ascii()
}

fn is_identifier_continue(ch: u8) -> bool {
	is_identifier_start(ch) || ch.is_ascii_digit() || ch == b'$'
}

fn quoted_end(bytes: &[u8], start: usize, escape: bool) -> usize {
	let quote = bytes[start];
	let mut i = start + 1;
	while i < bytes.len() {
		if escape && bytes[i] == b'\\' {
			i = (i + 2).min(bytes.len());
		} else if bytes[i] == quote {
			i += 1;
			if bytes.get(i) != Some(&quote) {
				break;
			}
			i += 1;
		} else {
			i += 1;
		}
	}
	i
}

fn dollar_quoted_end(sql: &str, start: usize) -> Option<usize> {
	let bytes = sql.as_bytes();
	let mut end = start + 1;
	if end < bytes.len() && is_identifier_start(bytes[end]) {
		end += 1;
		while end < bytes.len() && (is_identifier_start(bytes[end]) || bytes[end].is_ascii_digit())
		{
			end += 1;
		}
	}
	if bytes.get(end) != Some(&b'$') {
		return None;
	}
	end += 1;
	let delimiter = &sql[start..end];
	Some(
		sql[end..]
			.find(delimiter)
			.map_or(sql.len(), |offset| end + offset + delimiter.len()),
	)
}

fn inline_statement<S: QueryStatementBuilder + ?Sized, T: QueryBuilderTrait>(
	statement: &S,
	query_builder: T,
) -> String {
	// Backtick-quoting builders use MySQL comment syntax.
	let (placeholder, numbered) = query_builder.placeholder();
	let dialect = InlineDialect {
		mysql: query_builder.quote_char() == '`',
		postgres: numbered && placeholder == "$",
		numbered: Some(numbered),
	};
	let (sql, values) = statement.build(query_builder);
	inline_params_with_dialect(&sql, &values, dialect)
}

// Built-in statements can retain value provenance by choosing the writer before
// rendering. Keep the default build() path for other backends and downstream
// QueryStatementBuilder implementations, including their build() overrides.
pub(crate) fn postgres_to_string<S: QueryStatementBuilder, T: QueryBuilderTrait>(
	statement: &S,
	query_builder: T,
	render: fn(&PostgresQueryBuilder, &S, SqlWriter) -> (String, Values),
) -> String {
	if let Some(postgres) = (&query_builder as &dyn Any).downcast_ref::<PostgresQueryBuilder>() {
		render(
			postgres,
			statement,
			SqlWriter::new_inlined(PostgresQueryBuilder::value_to_sql_literal),
		)
		.0
	} else {
		inline_statement(statement, query_builder)
	}
}

/// Trait for building query statements
///
/// This trait provides methods to build SQL statements for different database backends
/// and collect query parameters.
pub trait QueryStatementBuilder: Debug {
	/// Build SQL statement for a database backend and collect query parameters
	///
	/// This is the primary method for generating parameterized SQL queries.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::select()
	///     .column(Expr::col("name"))
	///     .from("users");
	///
	/// // Build for PostgreSQL
	/// let (sql, values) = query.build(PostgresQueryBuilder);
	/// // sql = "SELECT \"name\" FROM \"users\""
	/// // values = Values(vec![])
	/// ```
	fn build_any(&self, query_builder: &dyn QueryBuilderTrait) -> (String, Values);

	/// Build SQL statement for a database backend and return SQL string
	/// with values inlined as SQL literals.
	///
	/// This produces a complete SQL string with parameter values embedded
	/// directly, suitable for debugging, inspection, or execution against
	/// databases that do not support parameterized queries.
	///
	/// Built-in PostgreSQL statements inline only values supplied to the builder.
	/// Raw SQL fragments, including `$n` bind markers in [`crate::Expr::cust`], are
	/// preserved verbatim. Such markers still require caller-supplied bindings;
	/// prefer fully managed parameters when executing queries with [`Self::build`].
	/// Byte values use typed PostgreSQL `bytea` hex literals, including within
	/// custom expression templates and subqueries. Empty bytes remain distinct
	/// from SQL NULL. MySQL and SQLite retain their `X'...'` byte literal syntax.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_query::{Expr, PostgresQueryBuilder, Query, QueryStatementBuilder};
	///
	/// let query = Query::select()
	///     .expr(Expr::cust("$1::int4"))
	///     .expr(Expr::val(42))
	///     .to_owned();
	///
	/// assert_eq!(query.to_string(PostgresQueryBuilder), "SELECT $1::int4, 42");
	/// ```
	fn to_string<T: QueryBuilderTrait>(&self, query_builder: T) -> String {
		inline_statement(self, query_builder)
	}

	/// Build SQL statement with parameter collection
	///
	/// This is a convenience method that wraps `build_any()` with a concrete
	/// query builder type.
	fn build<T: QueryBuilderTrait>(&self, query_builder: T) -> (String, Values) {
		self.build_any(&query_builder)
	}
}

/// Trait for query statement writers
///
/// This trait extends [`QueryStatementBuilder`] with additional methods for
/// writing SQL statements.
pub trait QueryStatementWriter: QueryStatementBuilder {}

/// Placeholder trait for query builders (will be implemented in Phase 5)
///
/// This trait defines the interface for database-specific query builders
/// that generate SQL syntax for different backends (PostgreSQL, MySQL, SQLite).
pub trait QueryBuilderTrait: Debug + Any {
	/// Get placeholder format for this backend
	///
	/// Returns a tuple of (placeholder_format, is_numbered):
	/// - PostgreSQL: ("$", true) -> $1, $2, $3...
	/// - MySQL: ("?", false) -> ?, ?, ?...
	/// - SQLite: ("?", false) -> ?, ?, ?...
	fn placeholder(&self) -> (&str, bool);

	/// Get quote character for this backend
	///
	/// - PostgreSQL: " (double quote)
	/// - MySQL: ` (backtick)
	/// - SQLite: " (double quote)
	fn quote_char(&self) -> char;
}
