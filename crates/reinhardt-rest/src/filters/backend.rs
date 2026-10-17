//! Custom filter backend system
//!
//! Provides a pluggable filter backend architecture for specialized filtering needs.
//!
//! # Examples
//!
//! ```
//! use reinhardt_rest::filters::{FilterBackend, FilterResult, CustomFilterBackend};
//! use std::collections::HashMap;
//! use async_trait::async_trait;
//!
//! // Define a custom filter backend
//! struct MyCustomFilter;
//!
//! #[async_trait]
//! impl FilterBackend for MyCustomFilter {
//!     async fn filter_queryset(
//!         &self,
//!         query_params: &HashMap<String, String>,
//!         sql: String,
//!     ) -> FilterResult<String> {
//!         // Custom filtering logic
//!         Ok(sql)
//!     }
//! }
//!
//! # async fn example() {
//! // Use the custom filter
//! let filter = MyCustomFilter;
//! let mut backend = CustomFilterBackend::new();
//! backend.add_filter(Box::new(filter));
//! // Verify the filter is added successfully
//! assert_eq!(backend.filter_count(), 1);
//! # }
//! ```

use super::{DatabaseDialect, FilterBackend, FilterError, FilterResult};
use async_trait::async_trait;
use reinhardt_query::SimpleExpr;
use reinhardt_query::prelude::{
	Alias, Cond, Expr, ExprTrait, MySqlQueryBuilder, Order, PostgresQueryBuilder, Query,
	QueryStatementBuilder,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Find a SQL keyword in `sql` using ASCII case-insensitive matching followed by whitespace.
///
/// Returns the byte position of the keyword start if found.
/// Uses byte-level ASCII comparison to avoid allocation and Unicode case-folding issues
/// (e.g., "ß" uppercasing to "SS" which would shift byte indices).
///
/// Quoted strings, quoted identifiers, comments and PostgreSQL dollar-quoted
/// strings are skipped. Filters insert quoted search text at the returned
/// position, so a match inside a literal would move that literal's boundary
/// and turn search text into SQL. The filters do not know which backend runs
/// their SQL, so a region is skipped only when MySQL (in either backslash
/// mode), PostgreSQL and SQLite leave no quoted string open past its end;
/// otherwise [`FilterError::InvalidQuery`] is returned. That rejects quoted
/// strings with an odd run of backslashes before the closing quote, nested
/// block comments, and comments or dollar-quoted strings whose text would open
/// a string under the backend that reads it as code. SQL that ends inside a
/// quoted string, identifier or comment is rejected too, because text appended
/// to it would not be code. `#>`, `#>>` and `#-` are read as PostgreSQL
/// operators, so a MySQL `#` comment starting with `>` or `-` must not contain
/// these keywords; filters reject line breaks in inserted search text so such a
/// comment cannot be ended by it.
///
/// NOTE: Keywords inside subqueries can still match. A full SQL parser is out
/// of scope for this filter layer; callers should be aware of this limitation.
pub(super) fn find_sql_keyword(sql: &str, keyword: &str) -> FilterResult<Option<usize>> {
	let sql_bytes = sql.as_bytes();
	let kw_bytes = keyword.as_bytes();
	let kw_len = kw_bytes.len();
	// End of a line that MySQL reads as a `#` comment and PostgreSQL as code.
	let mut mysql_comment_end = 0;
	let mut i = 0;

	while i < sql_bytes.len() {
		if let Some(end) = skip_non_code(sql, i)? {
			if i < mysql_comment_end && end > mysql_comment_end {
				return Err(backend_dependent_boundary());
			}
			i = end;
			continue;
		}
		if sql_bytes[i] == b'#' {
			// SQL may end on such a line: PostgreSQL reads it as code, and line
			// breaks are rejected in inserted search text, so text appended to a
			// MySQL comment here stays commented out.
			mysql_comment_end =
				mysql_comment_end.max(line_end(sql_bytes, i).unwrap_or(sql_bytes.len()));
		}

		// Keyword must match (ASCII case-insensitive) and be followed by
		// whitespace or end of string, and preceded by whitespace, start of
		// string, or ')'
		let matched = sql_bytes
			.get(i..i + kw_len)
			.is_some_and(|candidate| candidate.eq_ignore_ascii_case(kw_bytes));
		let after_ok = sql_bytes
			.get(i + kw_len)
			.is_none_or(u8::is_ascii_whitespace);
		let before_ok = i == 0 || {
			let prev = sql_bytes[i - 1];
			prev.is_ascii_whitespace() || prev == b')'
		};

		if matched && after_ok && before_ok {
			return Ok(Some(i));
		}
		i += 1;
	}

	Ok(None)
}

fn backend_dependent_boundary() -> FilterError {
	FilterError::InvalidQuery(
		"SQL contains a quoted string or comment whose end depends on the database backend"
			.to_string(),
	)
}

fn unterminated_region() -> FilterError {
	FilterError::InvalidQuery("SQL ends inside a quoted string or comment".to_string())
}

/// Return the index just past the quoted string, quoted identifier, comment or
/// dollar-quoted string starting at `i`, or `None` when `i` starts SQL code.
///
/// Regions that reach the end of `sql` without their terminator, including
/// line comments without a following line break, are rejected.
fn skip_non_code(sql: &str, i: usize) -> FilterResult<Option<usize>> {
	let bytes = sql.as_bytes();
	let end = match bytes[i] {
		b'\'' | b'"' => quoted_string_end(bytes, i)?,
		b'`' => quoted_identifier_end(bytes, i)?,
		b'-' if bytes.get(i + 1) == Some(&b'-') => {
			let end = line_end(bytes, i)?;
			// MySQL reads `--` without following whitespace as two minus signs.
			if !bytes
				.get(i + 2)
				.is_none_or(|next| next.is_ascii_whitespace() || next.is_ascii_control())
			{
				ensure_code_closes(sql, i + 2, end)?;
			}
			end
		}
		// `#>`, `#>>` and `#-` are handled by the caller as PostgreSQL operators.
		b'#' if !matches!(bytes.get(i + 1), Some(b'>' | b'-')) => {
			let end = line_end(bytes, i)?;
			// PostgreSQL reads `#` as an operator.
			ensure_code_closes(sql, i + 1, end)?;
			end
		}
		b'/' if bytes.get(i + 1) == Some(&b'*') => {
			let body_start = i + 2;
			let body_end = sql[body_start..]
				.find("*/")
				.map(|offset| body_start + offset)
				.ok_or_else(unterminated_region)?;
			// PostgreSQL nests block comments; MySQL and SQLite do not.
			if sql[body_start..body_end].contains("/*") {
				return Err(backend_dependent_boundary());
			}
			// MySQL executes the body of `/*!` comments.
			if bytes.get(body_start) == Some(&b'!') {
				ensure_code_closes(sql, body_start + 1, body_end)?;
			}
			body_end + 2
		}
		b'$' => {
			let Some((tag_end, body_end)) = dollar_quote(sql, i)? else {
				return Ok(None);
			};
			// MySQL reads the tags as identifiers and the body as code.
			ensure_code_closes(sql, tag_end, body_end)?;
			body_end + tag_end - i
		}
		_ => return Ok(None),
	};
	Ok(Some(end))
}

/// Check that `sql[start..end]`, read as SQL code, leaves no quoted string,
/// comment or dollar-quoted string open past `end`.
fn ensure_code_closes(sql: &str, start: usize, end: usize) -> FilterResult<()> {
	let mut i = start;
	while i < end {
		match skip_non_code(sql, i)? {
			Some(next) if next > end => return Err(backend_dependent_boundary()),
			Some(next) => i = next,
			None => i += 1,
		}
	}
	Ok(())
}

/// Locate a PostgreSQL dollar-quoted string starting at `start`, returning the
/// end of its opening tag and the start of its closing tag.
fn dollar_quote(sql: &str, start: usize) -> FilterResult<Option<(usize, usize)>> {
	let bytes = sql.as_bytes();
	let is_tag_byte = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_' || !byte.is_ascii();
	// `$` inside an identifier or before a digit (`$1`) does not open a string.
	if start > 0 && (is_tag_byte(bytes[start - 1]) || bytes[start - 1] == b'$') {
		return Ok(None);
	}
	let mut tag_end = start + 1;
	if bytes
		.get(tag_end)
		.is_some_and(|&byte| is_tag_byte(byte) && !byte.is_ascii_digit())
	{
		while bytes.get(tag_end).is_some_and(|&byte| is_tag_byte(byte)) {
			tag_end += 1;
		}
	}
	if bytes.get(tag_end) != Some(&b'$') {
		return Ok(None);
	}
	tag_end += 1;
	let tag = &sql[start..tag_end];
	let body_end = sql[tag_end..]
		.find(tag)
		.map(|offset| tag_end + offset)
		.ok_or_else(unterminated_region)?;
	Ok(Some((tag_end, body_end)))
}

/// Return the index just past the single- or double-quoted string at `start`.
///
/// A doubled quote continues the string; unterminated strings are rejected.
/// Backslash runs of even length, or followed by any byte other than the
/// quote, leave the string at the same end whether or not the backend treats
/// backslash as an escape character; an odd run before the quote does not and
/// is rejected.
fn quoted_string_end(bytes: &[u8], start: usize) -> FilterResult<usize> {
	let quote = bytes[start];
	let mut i = start + 1;
	while i < bytes.len() {
		if bytes[i] == b'\\' {
			let run = bytes[i..].iter().take_while(|&&byte| byte == b'\\').count();
			if run % 2 == 1 && bytes.get(i + run) == Some(&quote) {
				return Err(backend_dependent_boundary());
			}
			i += run;
		} else if bytes[i] == quote {
			if bytes.get(i + 1) != Some(&quote) {
				return Ok(i + 1);
			}
			i += 2;
		} else {
			i += 1;
		}
	}
	Err(unterminated_region())
}

/// Return the index just past the backtick-quoted identifier at `start`.
fn quoted_identifier_end(bytes: &[u8], start: usize) -> FilterResult<usize> {
	let mut i = start + 1;
	while i < bytes.len() {
		if bytes[i] == b'`' {
			if bytes.get(i + 1) != Some(&b'`') {
				return Ok(i + 1);
			}
			i += 2;
		} else {
			i += 1;
		}
	}
	Err(unterminated_region())
}

/// Return the index of the line break ending the line that contains `start`.
fn line_end(bytes: &[u8], start: usize) -> FilterResult<usize> {
	bytes[start..]
		.iter()
		.position(|&byte| matches!(byte, b'\n' | b'\r'))
		.map(|offset| start + offset)
		.ok_or_else(unterminated_region)
}

/// Find the end position of a SQL clause by locating the next top-level keyword.
///
/// Searches for each keyword in `end_keywords` after `start_pos` in `sql`,
/// returning the earliest match position. If no keyword is found, returns `sql.len()`.
/// `start_pos` must lie outside quoted regions and comments. Errors are those
/// of [`find_sql_keyword`].
///
/// NOTE: Keywords inside subqueries can still match. A full SQL parser is out
/// of scope for this filter layer.
pub(super) fn find_clause_end(
	sql: &str,
	start_pos: usize,
	end_keywords: &[&str],
) -> FilterResult<usize> {
	let mut end = sql.len();
	for keyword in end_keywords {
		if let Some(pos) = find_sql_keyword(&sql[start_pos..], keyword)? {
			end = end.min(start_pos + pos);
		}
	}
	Ok(end)
}

/// A composable filter backend that chains multiple filters
///
/// # Examples
///
/// ```
/// use reinhardt_rest::filters::{FilterBackend, CustomFilterBackend};
/// use std::collections::HashMap;
/// use async_trait::async_trait;
///
/// struct SearchFilter;
/// struct OrderingFilter;
///
/// #[async_trait]
/// impl FilterBackend for SearchFilter {
///     async fn filter_queryset(
///         &self,
///         query_params: &HashMap<String, String>,
///         sql: String,
///     ) -> reinhardt_rest::filters::FilterResult<String> {
///         Ok(sql)
///     }
/// }
///
/// #[async_trait]
/// impl FilterBackend for OrderingFilter {
///     async fn filter_queryset(
///         &self,
///         query_params: &HashMap<String, String>,
///         sql: String,
///     ) -> reinhardt_rest::filters::FilterResult<String> {
///         Ok(sql)
///     }
/// }
///
/// # async fn example() {
/// let mut backend = CustomFilterBackend::new();
/// backend.add_filter(Box::new(SearchFilter));
/// backend.add_filter(Box::new(OrderingFilter));
///
/// let params = HashMap::new();
/// let result = backend.filter_queryset(&params, "SELECT * FROM users".to_string()).await;
/// // Verify the filter chain processes the query successfully
/// assert!(result.is_ok());
/// # }
/// ```
#[derive(Default)]
pub struct CustomFilterBackend {
	filters: Vec<Arc<dyn FilterBackend>>,
}

impl CustomFilterBackend {
	/// Create a new custom filter backend
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::filters::CustomFilterBackend;
	///
	/// let backend = CustomFilterBackend::new();
	/// // Verify backend is created with no filters
	/// assert_eq!(backend.filter_count(), 0);
	/// ```
	pub fn new() -> Self {
		Self {
			filters: Vec::new(),
		}
	}

	/// Add a filter to the backend chain
	///
	/// Filters are applied in the order they are added.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::filters::{FilterBackend, CustomFilterBackend};
	/// use std::collections::HashMap;
	/// use async_trait::async_trait;
	///
	/// struct MyFilter;
	///
	/// #[async_trait]
	/// impl FilterBackend for MyFilter {
	///     async fn filter_queryset(
	///         &self,
	///         query_params: &HashMap<String, String>,
	///         sql: String,
	///     ) -> reinhardt_rest::filters::FilterResult<String> {
	///         Ok(sql)
	///     }
	/// }
	///
	/// let mut backend = CustomFilterBackend::new();
	/// backend.add_filter(Box::new(MyFilter));
	/// // Verify the filter is added successfully
	/// assert_eq!(backend.filter_count(), 1);
	/// ```
	pub fn add_filter(&mut self, filter: Box<dyn FilterBackend>) {
		self.filters.push(Arc::from(filter));
	}

	/// Get the number of filters in the chain
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::filters::{FilterBackend, CustomFilterBackend};
	/// use std::collections::HashMap;
	/// use async_trait::async_trait;
	///
	/// struct MyFilter;
	///
	/// #[async_trait]
	/// impl FilterBackend for MyFilter {
	///     async fn filter_queryset(
	///         &self,
	///         query_params: &HashMap<String, String>,
	///         sql: String,
	///     ) -> reinhardt_rest::filters::FilterResult<String> {
	///         Ok(sql)
	///     }
	/// }
	///
	/// let mut backend = CustomFilterBackend::new();
	/// // Verify initial count is 0
	/// assert_eq!(backend.filter_count(), 0);
	/// backend.add_filter(Box::new(MyFilter));
	/// // Verify count increases to 1
	/// assert_eq!(backend.filter_count(), 1);
	/// ```
	pub fn filter_count(&self) -> usize {
		self.filters.len()
	}
}

#[async_trait]
impl FilterBackend for CustomFilterBackend {
	async fn filter_queryset(
		&self,
		query_params: &HashMap<String, String>,
		mut sql: String,
	) -> FilterResult<String> {
		for filter in &self.filters {
			sql = filter.filter_queryset(query_params, sql).await?;
		}
		Ok(sql)
	}
}

/// A simple search filter backend implementation
///
/// # Examples
///
/// ```
/// use reinhardt_rest::filters::{FilterBackend, SimpleSearchBackend};
/// use std::collections::HashMap;
///
/// # async fn example() {
/// let backend = SimpleSearchBackend::new("search")
///     .with_field("title");
/// let mut params = HashMap::new();
/// params.insert("search".to_string(), "rust".to_string());
///
/// let sql = "SELECT * FROM articles".to_string();
/// let result = backend.filter_queryset(&params, sql).await.unwrap();
/// // Verify WHERE clause is added
/// assert!(result.contains("WHERE"));
/// # }
/// ```
pub struct SimpleSearchBackend {
	param_name: String,
	fields: Vec<String>,
	dialect: DatabaseDialect,
}

impl SimpleSearchBackend {
	/// Create a new simple search backend
	///
	/// # Arguments
	///
	/// * `param_name` - The query parameter name to look for (e.g., "search", "q")
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::filters::SimpleSearchBackend;
	///
	/// let backend = SimpleSearchBackend::new("search");
	/// // Verify backend is created with the correct parameter name
	/// let _: SimpleSearchBackend = backend;
	/// ```
	pub fn new(param_name: impl Into<String>) -> Self {
		Self {
			param_name: param_name.into(),
			fields: Vec::new(),
			dialect: DatabaseDialect::default(),
		}
	}

	/// Add a field to search in
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::filters::SimpleSearchBackend;
	///
	/// let backend = SimpleSearchBackend::new("search")
	///     .with_field("title")
	///     .with_field("content");
	/// // Verify fields are added successfully
	/// let _: SimpleSearchBackend = backend;
	/// ```
	pub fn with_field(mut self, field: impl Into<String>) -> Self {
		self.fields.push(field.into());
		self
	}

	/// Set the database dialect for query generation
	///
	/// Different databases use different identifier quoting styles.
	/// MySQL uses backticks (`column`) while PostgreSQL uses double quotes ("column").
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::filters::{SimpleSearchBackend, DatabaseDialect};
	///
	/// let backend = SimpleSearchBackend::new("search")
	///     .with_field("title")
	///     .with_dialect(DatabaseDialect::PostgreSQL);
	/// // Verify backend is configured for PostgreSQL
	/// let _: SimpleSearchBackend = backend;
	/// ```
	pub fn with_dialect(mut self, dialect: DatabaseDialect) -> Self {
		self.dialect = dialect;
		self
	}

	/// Escape LIKE wildcards so that search text matches literally
	///
	/// Escapes `%`, `_`, and `\` with a backslash, PostgreSQL's default LIKE
	/// escape character.
	fn escape_like_pattern(pattern: &str) -> String {
		pattern
			.replace('\\', "\\\\")
			.replace('%', "\\%")
			.replace('_', "\\_")
	}

	/// Build one `LIKE` condition per search field
	///
	/// The search text is escaped exactly once, and the rendered string
	/// literals end at the same position under MySQL, PostgreSQL and SQLite
	/// rules, so filters that run after this one can still scan the SQL with
	/// [`find_sql_keyword`]:
	///
	/// - MySQL: `contains` escapes the text and renders `ESCAPE 0x5C`.
	/// - PostgreSQL: the pattern is escaped here and rendered without an
	///   `ESCAPE` clause, because backslash is already PostgreSQL's default LIKE
	///   escape character. The `ESCAPE '\'` that `contains` renders for
	///   PostgreSQL is an unterminated string under MySQL rules, so
	///   `find_sql_keyword` rejects SQL containing it.
	fn build_search_conditions(&self, search_query: &str) -> Vec<SimpleExpr> {
		match self.dialect {
			DatabaseDialect::MySQL => self
				.fields
				.iter()
				.map(|field| Expr::col(Alias::new(field)).contains(search_query))
				.collect(),
			DatabaseDialect::PostgreSQL => {
				let pattern = format!("%{}%", Self::escape_like_pattern(search_query));
				self.fields
					.iter()
					.map(|field| Expr::col(Alias::new(field)).like(pattern.as_str()))
					.collect()
			}
		}
	}
}

#[async_trait]
impl FilterBackend for SimpleSearchBackend {
	async fn filter_queryset(
		&self,
		query_params: &HashMap<String, String>,
		sql: String,
	) -> FilterResult<String> {
		if let Some(search_query) = query_params.get(&self.param_name) {
			if self.fields.is_empty() {
				return Err(FilterError::InvalidParameter(
					"No search fields configured".to_string(),
				));
			}
			// The rendered search text must not end a line comment that a
			// backend reads differently from find_sql_keyword. The search text
			// is embedded in an unbound string literal, and no encoding of a
			// backslash in it matches the same rows under every MySQL and
			// PostgreSQL string mode.
			if search_query.contains(['\\', '\n', '\r']) {
				return Err(FilterError::InvalidParameter(
					"search text must not contain backslash or line break characters".to_string(),
				));
			}

			let conditions = self.build_search_conditions(search_query);

			// Combine all conditions with OR logic
			let mut condition = Cond::any();
			for cond in conditions {
				condition = condition.add(cond);
			}

			// Build a minimal SELECT query to extract the WHERE clause
			// SeaQuery properly escapes values when generating SQL
			// Use the appropriate QueryBuilder based on dialect
			let query = match self.dialect {
				DatabaseDialect::MySQL => Query::select()
					.expr(Expr::val(1))
					.cond_where(condition)
					.to_string(MySqlQueryBuilder),
				DatabaseDialect::PostgreSQL => Query::select()
					.expr(Expr::val(1))
					.cond_where(condition)
					.to_string(PostgresQueryBuilder),
			};

			// Extract just the WHERE condition portion (after "WHERE ")
			let condition_str = if let Some(idx) = query.find("WHERE ") {
				query[idx + 6..].to_string()
			} else {
				String::new()
			};

			let where_clause = format!("WHERE ({})", condition_str);

			// Append search condition using proper SQL composition
			// instead of string replacement which can corrupt complex WHERE clauses.
			// Uses ASCII case-insensitive keyword scanning to avoid allocation and
			// Unicode case-folding byte-length divergence from to_uppercase().
			if let Some(where_pos) = find_sql_keyword(&sql, "WHERE")? {
				// Skip past "WHERE" keyword and any trailing whitespace
				let after_keyword = where_pos + "WHERE".len();
				let content_start = sql[after_keyword..]
					.bytes()
					.position(|b| !b.is_ascii_whitespace())
					.map(|p| after_keyword + p)
					.unwrap_or(after_keyword);

				// Find the end of the existing WHERE clause by locating
				// the next top-level SQL keyword (GROUP BY, ORDER BY, LIMIT, etc.)
				let clause_end_keywords = ["GROUP BY", "ORDER BY", "LIMIT", "OFFSET", "HAVING"];
				let end_pos = find_clause_end(&sql, content_start, &clause_end_keywords)?;

				let existing_where = sql[content_start..end_pos].trim();
				let remainder = &sql[end_pos..];
				let prefix = &sql[..where_pos];

				Ok(format!(
					"{}WHERE ({}) AND ({}) {}",
					prefix,
					existing_where,
					condition_str,
					remainder.trim()
				))
			} else {
				Ok(format!("{} {}", sql, where_clause))
			}
		} else {
			Ok(sql)
		}
	}
}

/// A simple ordering filter backend implementation
///
/// # Examples
///
/// ```
/// use reinhardt_rest::filters::{FilterBackend, SimpleOrderingBackend};
/// use std::collections::HashMap;
///
/// # async fn example() {
/// let backend = SimpleOrderingBackend::new("ordering")
///     .allow_field("created_at")
///     .allow_field("title");
///
/// let mut params = HashMap::new();
/// params.insert("ordering".to_string(), "-created_at".to_string());
///
/// let sql = "SELECT * FROM articles".to_string();
/// let result = backend.filter_queryset(&params, sql).await.unwrap();
/// // Verify ORDER BY clause is added with DESC direction
/// assert!(result.contains("ORDER BY created_at DESC"));
/// # }
/// ```
pub struct SimpleOrderingBackend {
	param_name: String,
	allowed_fields: Vec<String>,
}

impl SimpleOrderingBackend {
	/// Create a new simple ordering backend
	///
	/// # Arguments
	///
	/// * `param_name` - The query parameter name to look for (e.g., "ordering", "sort")
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::filters::SimpleOrderingBackend;
	///
	/// let backend = SimpleOrderingBackend::new("ordering");
	/// // Verify backend is created with the correct parameter name
	/// let _: SimpleOrderingBackend = backend;
	/// ```
	pub fn new(param_name: impl Into<String>) -> Self {
		Self {
			param_name: param_name.into(),
			allowed_fields: Vec::new(),
		}
	}

	/// Add an allowed field for ordering
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::filters::SimpleOrderingBackend;
	///
	/// let backend = SimpleOrderingBackend::new("ordering")
	///     .allow_field("created_at")
	///     .allow_field("title");
	/// // Verify fields are allowed successfully
	/// let _: SimpleOrderingBackend = backend;
	/// ```
	pub fn allow_field(mut self, field: impl Into<String>) -> Self {
		self.allowed_fields.push(field.into());
		self
	}

	/// Build the ORDER BY clause
	///
	/// Returns the ORDER BY clause string (without the "ORDER BY" keywords).
	/// Generates database-agnostic SQL without identifier quoting.
	fn build_order_clause(&self, field: &str, order: Order) -> String {
		let order_str = match order {
			Order::Asc => "ASC",
			Order::Desc => "DESC",
		};
		format!("{} {}", field, order_str)
	}
}

#[async_trait]
impl FilterBackend for SimpleOrderingBackend {
	async fn filter_queryset(
		&self,
		query_params: &HashMap<String, String>,
		sql: String,
	) -> FilterResult<String> {
		if let Some(ordering) = query_params.get(&self.param_name) {
			// Parse the ordering parameter: "-field" for DESC, "field" for ASC
			let (field, order) = if let Some(field_name) = ordering.strip_prefix('-') {
				(field_name, Order::Desc)
			} else {
				(ordering.as_str(), Order::Asc)
			};

			if !self.allowed_fields.contains(&field.to_string()) {
				return Err(FilterError::InvalidParameter(format!(
					"Field '{}' is not allowed for ordering",
					field
				)));
			}

			// Use reinhardt-query to build type-safe ORDER BY clause
			let order_expr = self.build_order_clause(field, order);
			let order_clause = format!("ORDER BY {}", order_expr);

			// Append new ordering criteria to existing ORDER BY clause
			// instead of replacing it, which would destroy previous orderings.
			// Uses ASCII case-insensitive keyword scanning to avoid allocation and
			// Unicode case-folding byte-length divergence from to_uppercase().
			if let Some(order_pos) = find_sql_keyword(&sql, "ORDER BY")? {
				// Skip past "ORDER BY" keyword and any trailing whitespace
				let after_keyword = order_pos + "ORDER BY".len();
				let content_start = sql[after_keyword..]
					.bytes()
					.position(|b| !b.is_ascii_whitespace())
					.map(|p| after_keyword + p)
					.unwrap_or(after_keyword);

				// Find end of existing ORDER BY clause (next top-level keyword)
				let clause_end_keywords = ["LIMIT", "OFFSET"];
				let end_pos = find_clause_end(&sql, content_start, &clause_end_keywords)?;

				let existing_order = sql[content_start..end_pos].trim_end();
				let remainder = &sql[end_pos..];
				let prefix = &sql[..order_pos];

				Ok(format!(
					"{}ORDER BY {}, {} {}",
					prefix,
					existing_order,
					order_expr,
					remainder.trim()
				)
				.trim_end()
				.to_string())
			} else {
				Ok(format!("{} {}", sql, order_clause))
			}
		} else {
			Ok(sql)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	#[tokio::test]
	async fn test_custom_filter_backend_empty() {
		// Arrange
		let backend = CustomFilterBackend::new();
		let params = HashMap::new();
		let sql = "SELECT * FROM users".to_string();

		// Act
		let count = backend.filter_count();
		let result = backend.filter_queryset(&params, sql.clone()).await.unwrap();

		// Assert
		assert_eq!(count, 0);
		assert_eq!(result, sql);
	}

	#[rstest]
	#[tokio::test]
	async fn test_custom_filter_backend_with_filters() {
		// Arrange
		let mut backend = CustomFilterBackend::new();
		backend.add_filter(Box::new(
			SimpleSearchBackend::new("search").with_field("name"),
		));
		let mut params = HashMap::new();
		params.insert("search".to_string(), "john".to_string());
		let sql = "SELECT * FROM users".to_string();

		// Act
		let count = backend.filter_count();
		let result = backend.filter_queryset(&params, sql).await.unwrap();

		// Assert
		assert_eq!(count, 1);
		assert!(result.contains("WHERE"));
		assert!(result.contains("`name` LIKE '%john%'"));
	}

	#[rstest]
	#[tokio::test]
	async fn test_simple_search_backend() {
		// Arrange
		let backend = SimpleSearchBackend::new("search")
			.with_field("title")
			.with_field("content");
		let mut params = HashMap::new();
		params.insert("search".to_string(), "rust".to_string());
		let sql = "SELECT * FROM articles".to_string();

		// Act
		let result = backend.filter_queryset(&params, sql).await.unwrap();

		// Assert
		assert!(result.contains("WHERE"));
		assert!(result.contains("`title` LIKE '%rust%'"));
		assert!(result.contains("`content` LIKE '%rust%'"));
		assert!(result.contains("OR"));
	}

	#[rstest]
	#[tokio::test]
	async fn test_simple_search_backend_no_query() {
		// Arrange
		let backend = SimpleSearchBackend::new("search").with_field("title");
		let params = HashMap::new();
		let sql = "SELECT * FROM articles".to_string();

		// Act
		let result = backend.filter_queryset(&params, sql.clone()).await.unwrap();

		// Assert
		assert_eq!(result, sql);
	}

	#[rstest]
	#[tokio::test]
	async fn test_simple_search_backend_no_fields() {
		// Arrange
		let backend = SimpleSearchBackend::new("search");
		let mut params = HashMap::new();
		params.insert("search".to_string(), "rust".to_string());
		let sql = "SELECT * FROM articles".to_string();

		// Act
		let result = backend.filter_queryset(&params, sql).await;

		// Assert
		assert!(result.is_err());
	}

	#[rstest]
	#[tokio::test]
	async fn test_simple_search_backend_postgres() {
		// Arrange
		let backend = SimpleSearchBackend::new("search")
			.with_field("title")
			.with_dialect(DatabaseDialect::PostgreSQL);
		let mut params = HashMap::new();
		params.insert("search".to_string(), "rust".to_string());
		let sql = "SELECT * FROM articles".to_string();

		// Act
		let result = backend.filter_queryset(&params, sql).await.unwrap();

		// Assert
		assert!(result.contains("WHERE"));
		assert!(result.contains("\"title\" LIKE '%rust%'"));
	}

	#[rstest]
	#[case("' OR '1'='1")]
	#[case("'; DROP TABLE articles; --")]
	#[case("' UNION SELECT * FROM users --")]
	#[tokio::test]
	async fn test_simple_search_backend_sql_injection_prevention(#[case] payload: &str) {
		// Arrange
		let backend = SimpleSearchBackend::new("search").with_field("title");
		let mut params = HashMap::new();
		params.insert("search".to_string(), payload.to_string());
		let sql = "SELECT * FROM articles".to_string();

		// Act
		let result = backend.filter_queryset(&params, sql).await.unwrap();

		// Assert
		assert!(
			result.contains("LIKE"),
			"Result should contain LIKE clause for payload: {}",
			payload
		);
		// Single quotes must be balanced (even count) to prevent SQL injection
		let single_quote_count = result.matches('\'').count();
		assert!(
			single_quote_count % 2 == 0,
			"SQL injection vulnerability: unbalanced single quotes in result for payload: {}. Result: {}",
			payload,
			result
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_simple_ordering_backend_asc() {
		// Arrange
		let backend = SimpleOrderingBackend::new("ordering")
			.allow_field("created_at")
			.allow_field("title");
		let mut params = HashMap::new();
		params.insert("ordering".to_string(), "created_at".to_string());
		let sql = "SELECT * FROM articles".to_string();

		// Act
		let result = backend.filter_queryset(&params, sql).await.unwrap();

		// Assert
		assert!(result.contains("ORDER BY created_at ASC"));
	}

	#[rstest]
	#[tokio::test]
	async fn test_simple_ordering_backend_desc() {
		// Arrange
		let backend = SimpleOrderingBackend::new("ordering")
			.allow_field("created_at")
			.allow_field("title");
		let mut params = HashMap::new();
		params.insert("ordering".to_string(), "-created_at".to_string());
		let sql = "SELECT * FROM articles".to_string();

		// Act
		let result = backend.filter_queryset(&params, sql).await.unwrap();

		// Assert
		assert!(result.contains("ORDER BY created_at DESC"));
	}

	#[rstest]
	#[tokio::test]
	async fn test_simple_ordering_backend_invalid_field() {
		// Arrange
		let backend = SimpleOrderingBackend::new("ordering").allow_field("created_at");
		let mut params = HashMap::new();
		params.insert("ordering".to_string(), "invalid_field".to_string());
		let sql = "SELECT * FROM articles".to_string();

		// Act
		let result = backend.filter_queryset(&params, sql).await;

		// Assert
		assert!(result.is_err());
	}

	#[rstest]
	#[tokio::test]
	async fn test_simple_ordering_backend_no_query() {
		// Arrange
		let backend = SimpleOrderingBackend::new("ordering").allow_field("created_at");
		let params = HashMap::new();
		let sql = "SELECT * FROM articles".to_string();

		// Act
		let result = backend.filter_queryset(&params, sql.clone()).await.unwrap();

		// Assert
		assert_eq!(result, sql);
	}

	#[rstest]
	#[tokio::test]
	async fn test_chained_filters() {
		// Arrange
		let mut backend = CustomFilterBackend::new();
		backend.add_filter(Box::new(
			SimpleSearchBackend::new("search").with_field("title"),
		));
		backend.add_filter(Box::new(
			SimpleOrderingBackend::new("ordering").allow_field("created_at"),
		));
		let mut params = HashMap::new();
		params.insert("search".to_string(), "rust".to_string());
		params.insert("ordering".to_string(), "-created_at".to_string());
		let sql = "SELECT * FROM articles".to_string();

		// Act
		let result = backend.filter_queryset(&params, sql).await.unwrap();

		// Assert
		assert!(result.contains("WHERE"));
		assert!(result.contains("`title` LIKE '%rust%'"));
		assert!(result.contains("ORDER BY created_at DESC"));
	}

	// Regression test for #2685: SimpleOrderingBackend must preserve existing ORDER BY
	#[rstest]
	#[tokio::test]
	async fn test_ordering_preserves_existing_order_by() {
		// Arrange
		let backend = SimpleOrderingBackend::new("ordering").allow_field("title");
		let mut params = HashMap::new();
		params.insert("ordering".to_string(), "title".to_string());
		let sql = "SELECT * FROM articles ORDER BY created_at ASC".to_string();

		// Act
		let result = backend.filter_queryset(&params, sql).await.unwrap();

		// Assert - both ordering criteria must be present
		assert!(
			result.contains("ORDER BY created_at ASC, title ASC"),
			"Expected existing ORDER BY to be preserved with new criteria appended, got: {}",
			result
		);
	}

	// Regression test for #2685: ORDER BY with LIMIT must be preserved correctly
	#[rstest]
	#[tokio::test]
	async fn test_ordering_preserves_existing_order_by_with_limit() {
		// Arrange
		let backend = SimpleOrderingBackend::new("ordering").allow_field("title");
		let mut params = HashMap::new();
		params.insert("ordering".to_string(), "-title".to_string());
		let sql = "SELECT * FROM articles ORDER BY created_at ASC LIMIT 10".to_string();

		// Act
		let result = backend.filter_queryset(&params, sql).await.unwrap();

		// Assert - both ordering criteria and LIMIT must be present
		assert!(
			result.contains("ORDER BY created_at ASC, title DESC"),
			"Expected existing ORDER BY to be preserved, got: {}",
			result
		);
		assert!(
			result.contains("LIMIT 10"),
			"Expected LIMIT clause to be preserved, got: {}",
			result
		);
	}

	// Regression test for #2687: SimpleSearchBackend must preserve existing WHERE clause
	#[rstest]
	#[tokio::test]
	async fn test_search_preserves_existing_where_clause() {
		// Arrange
		let backend = SimpleSearchBackend::new("search").with_field("title");
		let mut params = HashMap::new();
		params.insert("search".to_string(), "rust".to_string());
		let sql = "SELECT * FROM articles WHERE status = 'published'".to_string();

		// Act
		let result = backend.filter_queryset(&params, sql).await.unwrap();

		// Assert - both the original WHERE condition and search condition must be present
		assert!(
			result.contains("status = 'published'"),
			"Expected original WHERE condition to be preserved, got: {}",
			result
		);
		assert!(
			result.contains("`title` LIKE '%rust%'"),
			"Expected search condition to be added, got: {}",
			result
		);
		assert!(
			result.contains("AND"),
			"Expected AND joining original and search conditions, got: {}",
			result
		);
	}

	// Regression test for #2687: Complex WHERE clause must not be corrupted
	#[rstest]
	#[tokio::test]
	async fn test_search_preserves_complex_where_clause() {
		// Arrange
		let backend = SimpleSearchBackend::new("search").with_field("name");
		let mut params = HashMap::new();
		params.insert("search".to_string(), "john".to_string());
		let sql = "SELECT * FROM users WHERE (age > 18 AND active = true) ORDER BY id".to_string();

		// Act
		let result = backend.filter_queryset(&params, sql).await.unwrap();

		// Assert - original complex condition, search condition, and ORDER BY must all be present
		assert!(
			result.contains("age > 18 AND active = true"),
			"Expected complex WHERE condition to be preserved, got: {}",
			result
		);
		assert!(
			result.contains("`name` LIKE '%john%'"),
			"Expected search condition to be added, got: {}",
			result
		);
		assert!(
			result.contains("ORDER BY"),
			"Expected ORDER BY clause to be preserved, got: {}",
			result
		);
	}

	// Regression test for #2687: Empty query (no WHERE) should add WHERE normally
	#[rstest]
	#[tokio::test]
	async fn test_search_adds_where_to_empty_query() {
		// Arrange
		let backend = SimpleSearchBackend::new("search").with_field("title");
		let mut params = HashMap::new();
		params.insert("search".to_string(), "rust".to_string());
		let sql = "SELECT * FROM articles".to_string();

		// Act
		let result = backend.filter_queryset(&params, sql).await.unwrap();

		// Assert
		assert!(
			result.contains("WHERE"),
			"Expected WHERE clause to be added, got: {}",
			result
		);
		assert!(
			result.contains("`title` LIKE '%rust%'"),
			"Expected search condition, got: {}",
			result
		);
	}

	// Regression test for #2685: Empty query (no ORDER BY) should add ORDER BY normally
	#[rstest]
	#[tokio::test]
	async fn test_ordering_adds_order_by_to_empty_query() {
		// Arrange
		let backend = SimpleOrderingBackend::new("ordering").allow_field("created_at");
		let mut params = HashMap::new();
		params.insert("ordering".to_string(), "-created_at".to_string());
		let sql = "SELECT * FROM articles".to_string();

		// Act
		let result = backend.filter_queryset(&params, sql).await.unwrap();

		// Assert
		assert_eq!(
			result, "SELECT * FROM articles ORDER BY created_at DESC",
			"Expected ORDER BY to be appended to query without existing ORDER BY"
		);
	}

	#[rstest]
	#[case::plain("SELECT * FROM t WHERE x = 1", "WHERE", Some(16))]
	#[case::lowercase("select * from t where x = 1", "WHERE", Some(16))]
	#[case::single_quoted("SELECT 'a WHERE b' FROM t WHERE x = 1", "WHERE", Some(26))]
	#[case::doubled_quote("SELECT 'it''s WHERE ' FROM t WHERE x", "WHERE", Some(29))]
	#[case::even_backslash_run("SELECT 'a\\\\' FROM t WHERE x", "WHERE", Some(20))]
	#[case::double_quoted("SELECT \"a WHERE b\" FROM t WHERE x", "WHERE", Some(26))]
	#[case::backtick("SELECT `a WHERE b` FROM t WHERE x", "WHERE", Some(26))]
	#[case::line_comment("SELECT 1 -- WHERE x\nFROM t WHERE y", "WHERE", Some(27))]
	#[case::line_comment_without_space("SELECT 1 --WHERE x\nFROM t WHERE y", "WHERE", Some(26))]
	#[case::hash_comment("SELECT 1 # WHERE x\nFROM t WHERE y", "WHERE", Some(26))]
	#[case::block_comment("SELECT 1 /* WHERE */ FROM t WHERE y", "WHERE", Some(28))]
	#[case::json_operator(
		"SELECT * FROM t WHERE d #>> '{a}' = 'x' ORDER BY id",
		"ORDER BY",
		Some(40)
	)]
	#[case::dollar_quoted("SELECT $$ WHERE $$ FROM t WHERE x", "WHERE", Some(26))]
	#[case::positional_parameter("SELECT $1 FROM t WHERE x", "WHERE", Some(17))]
	#[case::line_comment_with_quote("SELECT 1 -- it's\nFROM t WHERE x", "WHERE", Some(24))]
	#[case::json_path_operator("SELECT d #> '{a}' FROM t WHERE x", "WHERE", Some(25))]
	#[case::only_in_literal("SELECT 'x WHERE y' FROM t", "WHERE", None)]
	#[case::json_operator_on_last_line("SELECT * FROM t WHERE d #>> '{a}' = 'x'", "ORDER BY", None)]
	#[case::identifier_suffix("SELECT * FROM t WHERE_x", "WHERE", None)]
	fn test_find_sql_keyword_skips_quoted_text_and_comments(
		#[case] sql: &str,
		#[case] keyword: &str,
		#[case] expected: Option<usize>,
	) {
		// Act
		let position = find_sql_keyword(sql, keyword).unwrap();

		// Assert
		assert_eq!(position, expected);
	}

	#[rstest]
	#[case::odd_backslash_run_single("SELECT 'a\\' WHERE ' FROM t WHERE x")]
	#[case::odd_backslash_run_triple("SELECT 'a\\\\\\' WHERE ' FROM t WHERE x")]
	#[case::odd_backslash_run_double_quoted("SELECT \"a\\\" WHERE \" FROM t WHERE x")]
	#[case::odd_backslash_run_before_doubled_quote("SELECT 'a\\'' WHERE ' FROM t WHERE x")]
	#[case::line_comment_opening_mysql_string("SELECT 1 --it's\nFROM t WHERE x")]
	#[case::hash_comment_opening_postgres_string("SELECT 1 # it's\nFROM t WHERE x")]
	#[case::string_crossing_mysql_hash_comment("SELECT d #>> 'a\nb' FROM t WHERE x")]
	#[case::dollar_quote_opening_mysql_string("SELECT $$it's$$ FROM t WHERE x")]
	#[case::executable_comment_opening_string("SELECT /*! 'x */ 1 FROM t WHERE x")]
	#[case::unterminated_string("SELECT 'x FROM t WHERE y")]
	#[case::unterminated_identifier("SELECT `x FROM t WHERE y")]
	#[case::unterminated_block_comment("SELECT /* x FROM t WHERE y")]
	#[case::unterminated_dollar_quote("SELECT $$ x FROM t WHERE y")]
	#[case::trailing_line_comment("SELECT * FROM t -- WHERE")]
	#[case::trailing_hash_comment("SELECT * FROM t # note")]
	#[case::nested_block_comment("SELECT /* a /* b */ WHERE c */ 1 FROM t WHERE x")]
	fn test_find_sql_keyword_rejects_backend_dependent_boundaries(#[case] sql: &str) {
		// Act
		let result = find_sql_keyword(sql, "WHERE");

		// Assert
		assert!(
			matches!(result, Err(FilterError::InvalidQuery(_))),
			"expected InvalidQuery, got {result:?}"
		);
	}

	#[rstest]
	#[case::line_feed("rust\nnews")]
	#[case::carriage_return("rust\rnews")]
	#[tokio::test]
	async fn test_search_rejects_line_breaks(#[case] search: &str) {
		// Arrange
		let backend = SimpleSearchBackend::new("search").with_field("title");
		let params = HashMap::from([("search".to_string(), search.to_string())]);

		// Act
		let result = backend
			.filter_queryset(&params, "SELECT * FROM articles".to_string())
			.await;

		// Assert
		assert!(
			matches!(result, Err(FilterError::InvalidParameter(_))),
			"expected InvalidParameter, got {result:?}"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_search_rejects_sql_ending_in_comment() {
		// Arrange
		let backend = SimpleSearchBackend::new("search").with_field("title");
		let params = HashMap::from([("search".to_string(), "rust".to_string())]);

		// Act
		let result = backend
			.filter_queryset(&params, "SELECT * FROM articles -- all rows".to_string())
			.await;

		// Assert
		assert!(
			matches!(result, Err(FilterError::InvalidQuery(_))),
			"expected InvalidQuery, got {result:?}"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_search_inserts_condition_outside_earlier_literals() {
		// Arrange
		let backend = SimpleSearchBackend::new("search").with_field("title");
		let params = HashMap::from([("search".to_string(), "rust".to_string())]);
		let sql = "SELECT id, 'x WHERE y' AS note FROM articles WHERE tenant_id = 1".to_string();

		// Act
		let result = backend.filter_queryset(&params, sql).await.unwrap();

		// Assert
		assert_eq!(
			result,
			"SELECT id, 'x WHERE y' AS note FROM articles WHERE (tenant_id = 1) AND (`title` LIKE '%rust%' ESCAPE 0x5C) "
		);
	}

	#[rstest]
	#[case::mysql(
		DatabaseDialect::MySQL,
		"SELECT * FROM articles WHERE (`title` LIKE '%rust%' ESCAPE 0x5C) ORDER BY created_at ASC"
	)]
	#[case::postgresql(
		DatabaseDialect::PostgreSQL,
		"SELECT * FROM articles WHERE (\"title\" LIKE '%rust%') ORDER BY created_at ASC"
	)]
	#[tokio::test]
	async fn test_search_output_is_valid_input_for_ordering(
		#[case] dialect: DatabaseDialect,
		#[case] expected: &str,
	) {
		// Arrange
		let search = SimpleSearchBackend::new("search")
			.with_field("title")
			.with_dialect(dialect);
		let ordering = SimpleOrderingBackend::new("ordering").allow_field("created_at");
		let params = HashMap::from([
			("search".to_string(), "rust".to_string()),
			("ordering".to_string(), "created_at".to_string()),
		]);

		// Act
		let searched = search
			.filter_queryset(&params, "SELECT * FROM articles".to_string())
			.await
			.unwrap();
		let result = ordering.filter_queryset(&params, searched).await;

		// Assert
		assert_eq!(result.unwrap(), expected);
	}

	#[rstest]
	#[case::mysql(
		DatabaseDialect::MySQL,
		"SELECT * FROM articles WHERE (`title` LIKE '%a\\_b\\%c%' ESCAPE 0x5C)"
	)]
	#[case::postgresql(
		DatabaseDialect::PostgreSQL,
		"SELECT * FROM articles WHERE (\"title\" LIKE '%a\\_b\\%c%')"
	)]
	#[tokio::test]
	async fn test_search_escapes_like_wildcards_once(
		#[case] dialect: DatabaseDialect,
		#[case] expected: &str,
	) {
		// Arrange
		let backend = SimpleSearchBackend::new("search")
			.with_field("title")
			.with_dialect(dialect);
		let params = HashMap::from([("search".to_string(), "a_b%c".to_string())]);

		// Act
		let result = backend
			.filter_queryset(&params, "SELECT * FROM articles".to_string())
			.await;

		// Assert
		assert_eq!(result.unwrap(), expected);
	}

	#[rstest]
	#[case::mysql(DatabaseDialect::MySQL)]
	#[case::postgresql(DatabaseDialect::PostgreSQL)]
	#[tokio::test]
	async fn test_search_rejects_backslash(#[case] dialect: DatabaseDialect) {
		// Arrange
		let backend = SimpleSearchBackend::new("search")
			.with_field("title")
			.with_dialect(dialect);
		let params = HashMap::from([("search".to_string(), "C:\\temp".to_string())]);

		// Act
		let result = backend
			.filter_queryset(&params, "SELECT * FROM articles".to_string())
			.await;

		// Assert
		assert!(
			matches!(result, Err(FilterError::InvalidParameter(_))),
			"expected InvalidParameter, got {result:?}"
		);
	}
}
