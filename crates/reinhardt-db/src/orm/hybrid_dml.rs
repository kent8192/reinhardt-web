//! Hybrid Property Integration with DML Operations
//!
//! This module provides integration between reinhardt-hybrid properties
//! and DML (Data Manipulation Language) operations like INSERT and UPDATE.
//!
//! Based on SQLAlchemy's hybrid property DML support.
//!
//! # Identifier quoting
//!
//! Table and column names are always emitted as quoted identifiers using the
//! configured backend's escaping: backticks for MySQL, double quotes for
//! PostgreSQL and SQLite, and double quotes when no backend is configured.
//! Table names may be schema-qualified with `.` (`schema.table`); each segment
//! is quoted separately. Column names are quoted as a single identifier.
//!
//! Quoted identifiers are matched exactly. On PostgreSQL an unquoted `Users`
//! used to fold to `users`; it is now the distinct identifier `"Users"`, so
//! pass names in the case the table was created with. A column name such as
//! `alias.col` is one identifier, not a qualified reference.
//!
//! Quoting prevents a name from changing the statement structure, but it does
//! not decide which tables or columns a caller may write. Applications that
//! derive identifiers from untrusted input must still restrict them to an
//! allow-list.
//!
//! # Value binding
//!
//! Values are always returned as bind parameters, never rendered into the SQL
//! text. For a hybrid property with an SQL expression, each `(column)`
//! reference in the expression is replaced with `(placeholder)` and the value
//! is bound once per reference. String escaping differs between backends
//! (MySQL treats a backslash as an escape character unless
//! `NO_BACKSLASH_ESCAPES` is set), so an inline literal could end early and let
//! the rest of the value run as SQL.
//!
//! The expression itself is emitted as written and must come from trusted
//! code.

use crate::backends::backend::DatabaseBackend;
use crate::backends::optimization::QuoteStyle;
use crate::hybrid::HybridProperty;
use std::collections::HashMap;
use std::sync::Arc;

/// Internal marker key for expanded hybrid property values.
///
/// When a hybrid property expands to multiple columns (e.g., Point(x, y) -> x, y),
/// we store them under this special key to distinguish from regular columns.
///
/// Uses a single underscore prefix following Rust conventions for internal identifiers.
const EXPANDED_MARKER: &str = "_expanded";

/// Resolve the identifier quoting style for an optional backend.
///
/// Without a backend the builders fall back to ANSI double quotes, mirroring
/// the `?` placeholder fallback used for values.
fn quote_style(backend: Option<&Arc<dyn DatabaseBackend>>) -> QuoteStyle {
	backend.map_or(QuoteStyle::Ansi, |backend| backend.database_type().into())
}

/// Render a hybrid SQL expression with `value` bound at each `(column)` reference.
///
/// Every reference gets its own placeholder, so `?`-style backends receive one
/// parameter per occurrence. An expression without a reference is returned
/// unchanged and binds nothing.
fn bind_hybrid_expression(
	expr: &str,
	column: &str,
	value: &str,
	param_index: &mut usize,
	placeholder: impl Fn(usize) -> String,
	params: &mut Vec<String>,
) -> String {
	let reference = format!("({})", column);
	let mut segments = expr.split(reference.as_str());
	// `split` always yields at least one segment.
	let mut sql = segments.next().unwrap_or_default().to_string();
	for segment in segments {
		sql.push('(');
		sql.push_str(&placeholder(*param_index));
		sql.push(')');
		sql.push_str(segment);
		*param_index += 1;
		params.push(value.to_string());
	}
	sql
}

/// A value that can be inserted/updated, either direct or from a hybrid property
#[derive(Debug, Clone)]
pub enum DmlValue {
	/// Direct value
	Direct(String),
	/// SQL expression of a hybrid property, as written.
	///
	/// The builders bind the caller's value at each `(column)` reference when
	/// the statement is built.
	HybridExpression(String),
	/// Multiple columns from hybrid property (e.g., Point(x, y) -> x, y)
	Expanded(Vec<(String, String)>),
}

/// Builder for INSERT statements with hybrid property support
pub struct InsertBuilder {
	table_name: String,
	values: HashMap<String, DmlValue>,
	/// Values bound into `DmlValue::HybridExpression` entries, keyed by column.
	hybrid_values: HashMap<String, String>,
	backend: Option<Arc<dyn DatabaseBackend>>,
}

impl InsertBuilder {
	/// Create a new INSERT builder with hybrid property support
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::hybrid_dml::InsertBuilder;
	///
	/// let builder = InsertBuilder::new("users");
	/// // Can chain: .value().hybrid_value().build()
	/// ```
	pub fn new(table_name: &str) -> Self {
		Self {
			table_name: table_name.to_string(),
			values: HashMap::new(),
			hybrid_values: HashMap::new(),
			backend: None,
		}
	}

	/// Set the database backend for placeholder generation and identifier quoting
	pub fn with_backend(mut self, backend: Arc<dyn DatabaseBackend>) -> Self {
		self.backend = Some(backend);
		self
	}
	/// Add a direct column value
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::hybrid_dml::InsertBuilder;
	///
	/// let builder = InsertBuilder::new("users")
	///     .value("name", "Alice")
	///     .value("email", "alice@example.com");
	///
	/// let (sql, params) = builder.build();
	/// // SQL output order may vary, verify components separately
	/// assert!(sql.starts_with("INSERT INTO \"users\" ("));
	/// assert!(sql.contains("\"name\""));
	/// assert!(sql.contains("\"email\""));
	/// assert!(sql.contains(") VALUES (?, ?)"));
	/// assert_eq!(params.len(), 2);
	/// // HashMap doesn't guarantee order, so check both values are present
	/// assert!(params.contains(&"Alice".to_string()));
	/// assert!(params.contains(&"alice@example.com".to_string()));
	/// ```
	pub fn value(mut self, column: &str, value: &str) -> Self {
		self.values
			.insert(column.to_string(), DmlValue::Direct(value.to_string()));
		self
	}
	/// Add a hybrid property value
	///
	/// This method integrates hybrid properties with DML operations.
	/// If the hybrid property has an SQL expression, it will be used with the
	/// value bound at each `(column)` reference; otherwise, the value is treated
	/// as a direct parameter.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::hybrid_dml::InsertBuilder;
	/// use reinhardt_db::hybrid::HybridProperty;
	///
	/// struct User { email: String }
	///
	/// let lower_email = HybridProperty::new(|user: &User| user.email.to_lowercase())
	///     .with_expression(|| "LOWER(email)".to_string());
	///
	/// let builder = InsertBuilder::new("users")
	///     .hybrid_value("email", &lower_email, "TEST@EXAMPLE.COM");
	///
	/// let (sql, params) = builder.build();
	/// assert_eq!(
	///     sql,
	///     "INSERT INTO \"users\" (\"email\") VALUES (LOWER(?))",
	///     "Expected INSERT with LOWER expression, got: {}",
	///     sql
	/// );
	/// assert_eq!(params, vec!["TEST@EXAMPLE.COM"]);
	/// ```
	pub fn hybrid_value<T, R>(
		mut self,
		column: &str,
		property: &HybridProperty<T, R>,
		value: &str,
	) -> Self {
		// If the property has an expression, use it; otherwise treat as direct value
		if let Some(expr) = property.expression() {
			// Keep the expression as written; `build` binds the value at each
			// `(column)` reference once placeholder numbering is known.
			self.values
				.insert(column.to_string(), DmlValue::HybridExpression(expr));
			self.hybrid_values
				.insert(column.to_string(), value.to_string());
		} else {
			// No expression, use direct value
			self.values
				.insert(column.to_string(), DmlValue::Direct(value.to_string()));
		}
		self
	}
	/// Add an expanded hybrid property (e.g., Point -> x, y)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::hybrid_dml::InsertBuilder;
	///
	/// let builder = InsertBuilder::new("points")
	///     .expanded_hybrid(vec![("x", "10"), ("y", "20")]);
	///
	/// let (sql, params) = builder.build();
	/// assert_eq!(
	///     sql,
	///     "INSERT INTO \"points\" (\"x\", \"y\") VALUES (?, ?)",
	///     "Expected INSERT with expanded columns x and y, got: {}",
	///     sql
	/// );
	/// assert_eq!(params.len(), 2);
	/// assert_eq!(params[0], "10");
	/// assert_eq!(params[1], "20");
	/// ```
	pub fn expanded_hybrid(mut self, columns: Vec<(&str, &str)>) -> Self {
		let expanded = columns
			.into_iter()
			.map(|(col, val)| (col.to_string(), val.to_string()))
			.collect();

		// Add a special marker for expanded values
		self.values
			.insert(EXPANDED_MARKER.to_string(), DmlValue::Expanded(expanded));
		self
	}
	/// Build the SQL INSERT statement
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::hybrid_dml::InsertBuilder;
	///
	/// let builder = InsertBuilder::new("users")
	///     .value("name", "Bob")
	///     .value("age", "25");
	///
	/// let (sql, params) = builder.build();
	/// // SQL output order may vary, verify components separately
	/// assert!(sql.starts_with("INSERT INTO \"users\" ("));
	/// assert!(sql.contains("\"name\""));
	/// assert!(sql.contains("\"age\""));
	/// assert!(sql.contains(") VALUES (?, ?)"));
	/// assert_eq!(params.len(), 2);
	/// assert!(params.contains(&"Bob".to_string()));
	/// assert!(params.contains(&"25".to_string()));
	/// ```
	pub fn build(&self) -> (String, Vec<String>) {
		let mut columns = Vec::new();
		let mut placeholders = Vec::new();
		let mut params = Vec::new();
		let mut param_index = 1;
		let quote = quote_style(self.backend.as_ref());

		// Get placeholder function
		let get_placeholder = |index: usize| -> String {
			if let Some(ref backend) = self.backend {
				backend.placeholder(index)
			} else {
				// Fallback to ? for backward compatibility
				"?".to_string()
			}
		};

		// Handle expanded values first
		if let Some(DmlValue::Expanded(expanded)) = self.values.get(EXPANDED_MARKER) {
			for (col, val) in expanded {
				columns.push(quote.quote_identifier(col));
				placeholders.push(get_placeholder(param_index));
				param_index += 1;
				params.push(val.clone());
			}
		}

		// Handle regular values
		for (col, val) in &self.values {
			if col == EXPANDED_MARKER {
				continue;
			}
			match val {
				DmlValue::Direct(v) => {
					columns.push(quote.quote_identifier(col));
					placeholders.push(get_placeholder(param_index));
					param_index += 1;
					params.push(v.clone());
				}
				DmlValue::HybridExpression(expr) => {
					columns.push(quote.quote_identifier(col));
					placeholders.push(bind_hybrid_expression(
						expr,
						col,
						&self.hybrid_values[col],
						&mut param_index,
						get_placeholder,
						&mut params,
					));
				}
				DmlValue::Expanded(_) => {
					// Already handled above
				}
			}
		}

		let sql = format!(
			"INSERT INTO {} ({}) VALUES ({})",
			quote.quote_qualified_identifier(&self.table_name),
			columns.join(", "),
			placeholders.join(", ")
		);

		(sql, params)
	}
}

/// Builder for UPDATE statements with hybrid property support
pub struct UpdateBuilder {
	table_name: String,
	values: HashMap<String, DmlValue>,
	/// Values bound into `DmlValue::HybridExpression` entries, keyed by column.
	hybrid_values: HashMap<String, String>,
	where_clause: Option<(String, String)>,
	backend: Option<Arc<dyn DatabaseBackend>>,
}

impl UpdateBuilder {
	/// Create a new UPDATE builder with hybrid property support
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::hybrid_dml::UpdateBuilder;
	///
	/// let builder = UpdateBuilder::new("users");
	/// // Can chain: .set().where_clause().build()
	/// ```
	pub fn new(table_name: &str) -> Self {
		Self {
			table_name: table_name.to_string(),
			values: HashMap::new(),
			hybrid_values: HashMap::new(),
			where_clause: None,
			backend: None,
		}
	}

	/// Set the database backend for placeholder generation and identifier quoting
	pub fn with_backend(mut self, backend: Arc<dyn DatabaseBackend>) -> Self {
		self.backend = Some(backend);
		self
	}
	/// Add a direct column value
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::hybrid_dml::UpdateBuilder;
	///
	/// let builder = UpdateBuilder::new("users")
	///     .set("name", "Charlie")
	///     .set("age", "30")
	///     .where_clause("id", "1");
	///
	/// let (sql, params) = builder.build();
	/// // SQL output order may vary, verify components separately
	/// assert!(sql.starts_with("UPDATE \"users\" SET "));
	/// assert!(sql.contains("\"name\"=?"));
	/// assert!(sql.contains("\"age\"=?"));
	/// assert!(sql.contains(" WHERE \"id\"=?"));
	/// assert_eq!(params.len(), 3);
	/// assert!(params.contains(&"Charlie".to_string()));
	/// assert!(params.contains(&"30".to_string()));
	/// assert!(params.contains(&"1".to_string()));
	/// ```
	pub fn set(mut self, column: &str, value: &str) -> Self {
		self.values
			.insert(column.to_string(), DmlValue::Direct(value.to_string()));
		self
	}
	/// Add a hybrid property value
	///
	/// This method integrates hybrid properties with UPDATE operations.
	/// If the hybrid property has an SQL expression, it will be used with the
	/// value bound at each `(column)` reference; otherwise, the value is treated
	/// as a direct parameter.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::hybrid_dml::UpdateBuilder;
	/// use reinhardt_db::hybrid::HybridProperty;
	///
	/// struct User { email: String }
	///
	/// let lower_email = HybridProperty::new(|user: &User| user.email.to_lowercase())
	///     .with_expression(|| "LOWER(email)".to_string());
	///
	/// let builder = UpdateBuilder::new("users")
	///     .set_hybrid("email", &lower_email, "UPDATED@EXAMPLE.COM")
	///     .where_clause("id", "1");
	///
	/// let (sql, params) = builder.build();
	/// assert_eq!(sql, "UPDATE \"users\" SET \"email\"=LOWER(?) WHERE \"id\"=?");
	/// assert_eq!(params, vec!["UPDATED@EXAMPLE.COM", "1"]);
	/// ```
	pub fn set_hybrid<T, R>(
		mut self,
		column: &str,
		property: &HybridProperty<T, R>,
		value: &str,
	) -> Self {
		// If the property has an expression, use it; otherwise treat as direct value
		if let Some(expr) = property.expression() {
			// Keep the expression as written; `build` binds the value at each
			// `(column)` reference once placeholder numbering is known.
			self.values
				.insert(column.to_string(), DmlValue::HybridExpression(expr));
			self.hybrid_values
				.insert(column.to_string(), value.to_string());
		} else {
			// No expression, use direct value
			self.values
				.insert(column.to_string(), DmlValue::Direct(value.to_string()));
		}
		self
	}
	/// Add an expanded hybrid property (e.g., Point -> x, y)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::hybrid_dml::UpdateBuilder;
	///
	/// let builder = UpdateBuilder::new("points")
	///     .set_expanded(vec![("x", "100"), ("y", "200")])
	///     .where_clause("id", "5");
	///
	/// let (sql, params) = builder.build();
	/// assert!(sql.contains("UPDATE \"points\" SET \"x\"=?, \"y\"=?"));
	/// assert!(sql.contains("WHERE \"id\"=?"));
	/// assert_eq!(params.len(), 3);
	/// assert_eq!(params[0], "100");
	/// assert_eq!(params[1], "200");
	/// assert_eq!(params[2], "5");
	/// ```
	pub fn set_expanded(mut self, columns: Vec<(&str, &str)>) -> Self {
		let expanded = columns
			.into_iter()
			.map(|(col, val)| (col.to_string(), val.to_string()))
			.collect();

		self.values
			.insert(EXPANDED_MARKER.to_string(), DmlValue::Expanded(expanded));
		self
	}
	/// Add a parameterized WHERE clause using column equality.
	///
	/// Uses `"column" = ?` with a bind parameter to prevent SQL injection.
	/// The column name is quoted and escaped like every other identifier.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::hybrid_dml::UpdateBuilder;
	///
	/// let builder = UpdateBuilder::new("users")
	///     .set("status", "active")
	///     .where_clause("id", "42");
	///
	/// let (sql, params) = builder.build();
	/// assert!(sql.contains("UPDATE \"users\" SET \"status\"=?"));
	/// assert!(sql.contains("WHERE \"id\"=?"));
	/// assert_eq!(params.len(), 2);
	/// assert_eq!(params[0], "active");
	/// assert_eq!(params[1], "42");
	/// ```
	pub fn where_clause(mut self, column: &str, value: &str) -> Self {
		self.where_clause = Some((column.to_string(), value.to_string()));
		self
	}
	/// Build the SQL UPDATE statement
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::hybrid_dml::UpdateBuilder;
	///
	/// let builder = UpdateBuilder::new("users")
	///     .set("name", "David")
	///     .where_clause("id", "10");
	///
	/// let (sql, params) = builder.build();
	/// assert!(sql.contains("UPDATE \"users\" SET \"name\"=?"));
	/// assert!(sql.contains("WHERE \"id\"=?"));
	/// assert_eq!(params.len(), 2);
	/// assert_eq!(params[0], "David");
	/// assert_eq!(params[1], "10");
	/// ```
	pub fn build(&self) -> (String, Vec<String>) {
		let mut set_clauses = Vec::new();
		let mut params = Vec::new();
		let mut param_index = 1;
		let quote = quote_style(self.backend.as_ref());

		// Get placeholder function
		let get_placeholder = |index: usize| -> String {
			if let Some(ref backend) = self.backend {
				backend.placeholder(index)
			} else {
				// Fallback to ? for backward compatibility
				"?".to_string()
			}
		};

		// Handle expanded values first
		if let Some(DmlValue::Expanded(expanded)) = self.values.get(EXPANDED_MARKER) {
			for (col, val) in expanded {
				set_clauses.push(format!(
					"{}={}",
					quote.quote_identifier(col),
					get_placeholder(param_index)
				));
				param_index += 1;
				params.push(val.clone());
			}
		}

		// Handle regular values
		for (col, val) in &self.values {
			if col == EXPANDED_MARKER {
				continue;
			}
			match val {
				DmlValue::Direct(v) => {
					set_clauses.push(format!(
						"{}={}",
						quote.quote_identifier(col),
						get_placeholder(param_index)
					));
					param_index += 1;
					params.push(v.clone());
				}
				DmlValue::HybridExpression(expr) => {
					let expr = bind_hybrid_expression(
						expr,
						col,
						&self.hybrid_values[col],
						&mut param_index,
						get_placeholder,
						&mut params,
					);
					set_clauses.push(format!("{}={}", quote.quote_identifier(col), expr));
				}
				DmlValue::Expanded(_) => {
					// Already handled above
				}
			}
		}

		let mut sql = format!(
			"UPDATE {} SET {}",
			quote.quote_qualified_identifier(&self.table_name),
			set_clauses.join(", ")
		);

		if let Some((column, value)) = &self.where_clause {
			sql.push_str(&format!(" WHERE {}=?", quote.quote_identifier(column)));
			params.push(value.clone());
		}

		(sql, params)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::backends::error::Result;
	use crate::backends::types::{DatabaseType, QueryResult, QueryValue, Row, TransactionExecutor};
	use rstest::rstest;

	/// Backend stub that only reports a dialect; the builders never execute SQL.
	struct DialectBackend(DatabaseType);

	#[async_trait::async_trait]
	impl DatabaseBackend for DialectBackend {
		fn database_type(&self) -> DatabaseType {
			self.0
		}
		fn placeholder(&self, index: usize) -> String {
			match self.0 {
				DatabaseType::Postgres => format!("${}", index),
				DatabaseType::Mysql | DatabaseType::Sqlite => "?".to_string(),
			}
		}
		fn supports_returning(&self) -> bool {
			false
		}
		fn supports_on_conflict(&self) -> bool {
			false
		}
		async fn execute(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<QueryResult> {
			unreachable!("hybrid DML builders only build SQL and never execute it")
		}
		async fn fetch_one(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<Row> {
			unreachable!("hybrid DML builders only build SQL and never execute it")
		}
		async fn fetch_all(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<Vec<Row>> {
			unreachable!("hybrid DML builders only build SQL and never execute it")
		}
		async fn fetch_optional(
			&self,
			_sql: &str,
			_params: Vec<QueryValue>,
		) -> Result<Option<Row>> {
			unreachable!("hybrid DML builders only build SQL and never execute it")
		}
		async fn begin(&self) -> Result<Box<dyn TransactionExecutor>> {
			unreachable!("hybrid DML builders only build SQL and never execute it")
		}
		fn as_any(&self) -> &dyn std::any::Any {
			self
		}
	}

	#[test]
	fn test_insert_builder_simple() {
		let builder = InsertBuilder::new("person")
			.value("first_name", "John")
			.value("last_name", "Doe");

		let (sql, params) = builder.build();
		// SQL output order may vary, verify components separately
		assert!(
			sql.starts_with("INSERT INTO \"person\" ("),
			"Expected INSERT INTO person, got: {}",
			sql
		);
		assert!(
			sql.contains("\"first_name\""),
			"Expected first_name in SQL: {}",
			sql
		);
		assert!(
			sql.contains("\"last_name\""),
			"Expected last_name in SQL: {}",
			sql
		);
		assert!(
			sql.contains(") VALUES (?, ?)"),
			"Expected VALUES (?, ?), got: {}",
			sql
		);
		assert_eq!(
			params.len(),
			2,
			"Expected 2 parameters, got: {}",
			params.len()
		);
		assert!(
			params.contains(&"John".to_string()),
			"Expected 'John' in params: {:?}",
			params
		);
		assert!(
			params.contains(&"Doe".to_string()),
			"Expected 'Doe' in params: {:?}",
			params
		);
	}

	#[test]
	fn test_update_builder_simple() {
		let builder = UpdateBuilder::new("person")
			.set("first_name", "Jane")
			.where_clause("id", "1");

		let (sql, params) = builder.build();
		assert!(
			sql.contains("UPDATE \"person\" SET \"first_name\"=?"),
			"Expected UPDATE person SET, got: {}",
			sql
		);
		assert!(
			sql.contains("WHERE \"id\"=?"),
			"Expected parameterized WHERE, got: {}",
			sql
		);
		assert_eq!(
			params.len(),
			2,
			"Expected 2 parameters, got: {}",
			params.len()
		);
		assert_eq!(params[0], "Jane", "Expected 'Jane', got: {}", params[0]);
		assert_eq!(params[1], "1", "Expected '1', got: {}", params[1]);
	}

	#[test]
	fn test_insert_builder_expanded() {
		let builder = InsertBuilder::new("points").expanded_hybrid(vec![("x", "10"), ("y", "20")]);

		let (sql, params) = builder.build();
		assert_eq!(
			sql, "INSERT INTO \"points\" (\"x\", \"y\") VALUES (?, ?)",
			"Expected INSERT with expanded columns, got: {}",
			sql
		);
		assert_eq!(
			params.len(),
			2,
			"Expected 2 parameters, got: {}",
			params.len()
		);
		assert_eq!(
			params[0], "10",
			"Expected '10' as first param, got: {}",
			params[0]
		);
		assert_eq!(
			params[1], "20",
			"Expected '20' as second param, got: {}",
			params[1]
		);
	}

	#[test]
	fn test_update_builder_expanded() {
		let builder = UpdateBuilder::new("points")
			.set_expanded(vec![("x", "30"), ("y", "40")])
			.where_clause("id", "1");

		let (sql, params) = builder.build();
		assert!(
			sql.contains("UPDATE \"points\" SET \"x\"=?, \"y\"=?"),
			"Expected UPDATE with expanded columns, got: {}",
			sql
		);
		assert!(
			sql.contains("WHERE \"id\"=?"),
			"Expected parameterized WHERE, got: {}",
			sql
		);
		assert_eq!(
			params.len(),
			3,
			"Expected 3 parameters, got: {}",
			params.len()
		);
		assert_eq!(params[0], "30");
		assert_eq!(params[1], "40");
		assert_eq!(params[2], "1");
	}

	#[test]
	fn test_insert_builder_with_hybrid_expression() {
		struct User {
			email: String,
		}

		let lower_email = HybridProperty::new(|user: &User| user.email.to_lowercase())
			.with_expression(|| "LOWER(email)".to_string());

		let builder = InsertBuilder::new("users")
			.value("name", "John")
			.hybrid_value("email", &lower_email, "TEST@EXAMPLE.COM");

		let (sql, params) = builder.build();
		// SQL output order may vary, verify components separately
		assert!(
			sql.starts_with("INSERT INTO \"users\" ("),
			"Expected INSERT INTO users, got: {}",
			sql
		);
		assert!(
			sql.contains("\"name\""),
			"Expected 'name' column in SQL: {}",
			sql
		);
		assert!(
			sql.contains("\"email\""),
			"Expected 'email' column in SQL: {}",
			sql
		);
		assert!(
			sql.contains("LOWER(?)"),
			"Expected LOWER expression with a bound value in SQL: {}",
			sql
		);
		assert_eq!(
			params.len(),
			2,
			"Expected 2 parameters, got: {}",
			params.len()
		);
		assert!(
			params.contains(&"John".to_string()),
			"Expected 'John' in params: {:?}",
			params
		);
		assert!(
			params.contains(&"TEST@EXAMPLE.COM".to_string()),
			"Expected 'TEST@EXAMPLE.COM' in params: {:?}",
			params
		);
	}

	#[test]
	fn test_insert_builder_with_hybrid_no_expression() {
		struct User {
			email: String,
		}

		let simple_prop = HybridProperty::new(|user: &User| user.email.clone());

		let builder = InsertBuilder::new("users")
			.value("name", "John")
			.hybrid_value("email", &simple_prop, "test@example.com");

		let (sql, params) = builder.build();
		// SQL output order may vary, verify components separately
		assert!(
			sql.starts_with("INSERT INTO \"users\" ("),
			"Expected INSERT INTO users, got: {}",
			sql
		);
		assert!(
			sql.contains("\"name\""),
			"Expected 'name' column in SQL: {}",
			sql
		);
		assert!(
			sql.contains("\"email\""),
			"Expected 'email' column in SQL: {}",
			sql
		);
		assert!(
			sql.contains(") VALUES (?, ?)"),
			"Expected VALUES (?, ?), got: {}",
			sql
		);
		assert_eq!(
			params.len(),
			2,
			"Expected 2 parameters, got: {}",
			params.len()
		);
		assert!(
			params.contains(&"John".to_string()),
			"Expected 'John' in params: {:?}",
			params
		);
		assert!(
			params.contains(&"test@example.com".to_string()),
			"Expected 'test@example.com' in params: {:?}",
			params
		);
	}

	#[test]
	fn test_update_builder_with_hybrid_expression() {
		struct User {
			email: String,
		}

		let lower_email = HybridProperty::new(|user: &User| user.email.to_lowercase())
			.with_expression(|| "LOWER(email)".to_string());

		let builder = UpdateBuilder::new("users")
			.set_hybrid("email", &lower_email, "UPDATED@EXAMPLE.COM")
			.where_clause("id", "1");

		let (sql, params) = builder.build();
		assert_eq!(
			sql,
			"UPDATE \"users\" SET \"email\"=LOWER(?) WHERE \"id\"=?"
		);
		assert_eq!(params, vec!["UPDATED@EXAMPLE.COM", "1"]);
	}

	#[test]
	fn test_update_builder_with_hybrid_no_expression() {
		struct User {
			email: String,
		}

		let simple_prop = HybridProperty::new(|user: &User| user.email.clone());

		let builder = UpdateBuilder::new("users")
			.set_hybrid("email", &simple_prop, "updated@example.com")
			.where_clause("id", "1");

		let (sql, params) = builder.build();
		assert!(
			sql.contains("UPDATE \"users\" SET \"email\"=?"),
			"Expected UPDATE with direct value, got: {}",
			sql
		);
		assert!(
			sql.contains("WHERE \"id\"=?"),
			"Expected parameterized WHERE, got: {}",
			sql
		);
		assert_eq!(
			params.len(),
			2,
			"Expected 2 parameters, got: {}",
			params.len()
		);
		assert_eq!(params[0], "updated@example.com");
		assert_eq!(params[1], "1");
	}

	fn lower_email() -> HybridProperty<String, String> {
		HybridProperty::new(|s: &String| s.to_lowercase())
			.with_expression(|| "LOWER(email)".to_string())
	}

	#[rstest]
	#[case::quote("test@o'brien.com")]
	#[case::quote_breakout("'; DROP TABLE users;--")]
	#[case::backslash_quote_breakout("x\\' OR 1=1 -- ")]
	#[case::nul("x\0' OR 1=1 -- ")]
	fn test_insert_builder_hybrid_value_binds_value(#[case] value: &str) {
		// Arrange
		let builder = InsertBuilder::new("users").hybrid_value("email", &lower_email(), value);

		// Act
		let (sql, params) = builder.build();

		// Assert
		assert_eq!(sql, "INSERT INTO \"users\" (\"email\") VALUES (LOWER(?))");
		assert_eq!(params, vec![value.to_string()]);
	}

	#[rstest]
	#[case::quote("test@o'brien.com")]
	#[case::quote_breakout("'; DROP TABLE users;--")]
	#[case::backslash_quote_breakout("x\\' OR 1=1 -- ")]
	#[case::nul("x\0' OR 1=1 -- ")]
	fn test_update_builder_set_hybrid_binds_value(#[case] value: &str) {
		// Arrange
		let builder = UpdateBuilder::new("users")
			.set_hybrid("email", &lower_email(), value)
			.where_clause("id", "1");

		// Act
		let (sql, params) = builder.build();

		// Assert
		assert_eq!(
			sql,
			"UPDATE \"users\" SET \"email\"=LOWER(?) WHERE \"id\"=?"
		);
		assert_eq!(params, vec![value.to_string(), "1".to_string()]);
	}

	#[rstest]
	#[case::single_reference("LOWER(email)", "LOWER($1)", 1)]
	#[case::repeated_reference(
		"CONCAT(LOWER(email), '-', UPPER(email))",
		"CONCAT(LOWER($1), '-', UPPER($2))",
		2
	)]
	#[case::no_reference("LOWER('fixed')", "LOWER('fixed')", 0)]
	fn test_insert_builder_binds_each_hybrid_reference(
		#[case] expression: &'static str,
		#[case] expected_expression: &str,
		#[case] bound_count: usize,
	) {
		// Arrange
		let prop: HybridProperty<String, String> =
			HybridProperty::new(|s: &String| s.clone()).with_expression(move || expression.into());
		let builder = InsertBuilder::new("users")
			.with_backend(Arc::new(DialectBackend(DatabaseType::Postgres)))
			.hybrid_value("email", &prop, "x\\' OR 1=1 -- ");

		// Act
		let (sql, params) = builder.build();

		// Assert
		assert_eq!(
			sql,
			format!("INSERT INTO \"users\" (\"email\") VALUES ({expected_expression})")
		);
		assert_eq!(params, vec!["x\\' OR 1=1 -- ".to_string(); bound_count]);
	}

	#[rstest]
	fn test_update_builder_numbers_hybrid_and_direct_values_in_order() {
		// Arrange
		let builder = UpdateBuilder::new("users")
			.with_backend(Arc::new(DialectBackend(DatabaseType::Postgres)))
			.set("name", "Alice")
			.set_hybrid("email", &lower_email(), "ALICE@EXAMPLE.COM");

		// Act
		let (sql, params) = builder.build();

		// Assert
		// Column order follows HashMap iteration; either way each placeholder
		// number must point at that column's value.
		let expected_params = match sql.as_str() {
			"UPDATE \"users\" SET \"name\"=$1, \"email\"=LOWER($2)" => {
				vec!["Alice", "ALICE@EXAMPLE.COM"]
			}
			"UPDATE \"users\" SET \"email\"=LOWER($1), \"name\"=$2" => {
				vec!["ALICE@EXAMPLE.COM", "Alice"]
			}
			other => panic!("unexpected SQL: {other}"),
		};
		assert_eq!(params, expected_params);
	}

	#[rstest]
	#[case::benign_identifiers("users", "name", "INSERT INTO \"users\" (\"name\") VALUES (?)")]
	#[case::schema_qualified_table(
		"app.users",
		"name",
		"INSERT INTO \"app\".\"users\" (\"name\") VALUES (?)"
	)]
	#[case::table_name_breakout(
		"users\" (id) VALUES (1); DROP TABLE accounts; --",
		"name",
		"INSERT INTO \"users\"\" (id) VALUES (1); DROP TABLE accounts; --\" (\"name\") VALUES (?)"
	)]
	#[case::column_name_breakout(
		"users",
		"name\") VALUES (1); DROP TABLE accounts; --",
		"INSERT INTO \"users\" (\"name\"\") VALUES (1); DROP TABLE accounts; --\") VALUES (?)"
	)]
	fn test_insert_builder_quotes_identifiers(
		#[case] table: &str,
		#[case] column: &str,
		#[case] expected_sql: &str,
	) {
		// Arrange
		let builder = InsertBuilder::new(table).value(column, "Alice");

		// Act
		let (sql, params) = builder.build();

		// Assert
		assert_eq!(sql, expected_sql);
		assert_eq!(params, vec!["Alice".to_string()]);
	}

	#[rstest]
	fn test_insert_builder_quotes_expanded_columns() {
		// Arrange
		let builder = InsertBuilder::new("points")
			.expanded_hybrid(vec![("x\") VALUES (1); DROP TABLE points; --", "10")]);

		// Act
		let (sql, params) = builder.build();

		// Assert
		assert_eq!(
			sql,
			"INSERT INTO \"points\" (\"x\"\") VALUES (1); DROP TABLE points; --\") VALUES (?)"
		);
		assert_eq!(params, vec!["10".to_string()]);
	}

	#[rstest]
	#[case::postgres(
		DatabaseType::Postgres,
		"INSERT INTO \"app\".\"users\" (\"na\"\"m`e\") VALUES ($1)"
	)]
	#[case::mysql(
		DatabaseType::Mysql,
		"INSERT INTO `app`.`users` (`na\"m``e`) VALUES (?)"
	)]
	#[case::sqlite(
		DatabaseType::Sqlite,
		"INSERT INTO \"app\".\"users\" (\"na\"\"m`e\") VALUES (?)"
	)]
	fn test_insert_builder_quotes_identifiers_for_backend(
		#[case] database_type: DatabaseType,
		#[case] expected_sql: &str,
	) {
		// Arrange
		let builder = InsertBuilder::new("app.users")
			.with_backend(Arc::new(DialectBackend(database_type)))
			.value("na\"m`e", "Alice");

		// Act
		let (sql, params) = builder.build();

		// Assert
		assert_eq!(sql, expected_sql);
		assert_eq!(params, vec!["Alice".to_string()]);
	}

	#[rstest]
	#[case::benign_identifiers(
		"users",
		"name",
		"id",
		"UPDATE \"users\" SET \"name\"=? WHERE \"id\"=?"
	)]
	#[case::table_name_breakout(
		"users\" SET \"role\"='admin' --",
		"name",
		"id",
		"UPDATE \"users\"\" SET \"\"role\"\"='admin' --\" SET \"name\"=? WHERE \"id\"=?"
	)]
	#[case::set_column_breakout(
		"users",
		"role\"='admin', \"name",
		"id",
		"UPDATE \"users\" SET \"role\"\"='admin', \"\"name\"=? WHERE \"id\"=?"
	)]
	#[case::where_column_breakout(
		"users",
		"name",
		"id\"=? OR 1=1 --",
		"UPDATE \"users\" SET \"name\"=? WHERE \"id\"\"=? OR 1=1 --\"=?"
	)]
	fn test_update_builder_quotes_identifiers(
		#[case] table: &str,
		#[case] column: &str,
		#[case] where_column: &str,
		#[case] expected_sql: &str,
	) {
		// Arrange
		let builder = UpdateBuilder::new(table)
			.set(column, "Alice")
			.where_clause(where_column, "1");

		// Act
		let (sql, params) = builder.build();

		// Assert
		assert_eq!(sql, expected_sql);
		assert_eq!(params, vec!["Alice".to_string(), "1".to_string()]);
	}

	#[rstest]
	fn test_update_builder_quotes_expanded_columns() {
		// Arrange
		let builder = UpdateBuilder::new("points")
			.set_expanded(vec![("x\"=1, \"y", "10")])
			.where_clause("id", "1");

		// Act
		let (sql, params) = builder.build();

		// Assert
		assert_eq!(
			sql,
			"UPDATE \"points\" SET \"x\"\"=1, \"\"y\"=? WHERE \"id\"=?"
		);
		assert_eq!(params, vec!["10".to_string(), "1".to_string()]);
	}

	#[rstest]
	fn test_update_builder_uses_backticks_for_mysql_backend() {
		// Arrange
		let builder = UpdateBuilder::new("users`; DROP TABLE accounts; --")
			.with_backend(Arc::new(DialectBackend(DatabaseType::Mysql)))
			.set("na`me", "Alice")
			.where_clause("i`d", "1");

		// Act
		let (sql, params) = builder.build();

		// Assert
		assert_eq!(
			sql,
			"UPDATE `users``; DROP TABLE accounts; --` SET `na``me`=? WHERE `i``d`=?"
		);
		assert_eq!(params, vec!["Alice".to_string(), "1".to_string()]);
	}
}
