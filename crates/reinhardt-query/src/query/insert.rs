//! INSERT statement builder
//!
//! This module provides the `InsertStatement` type for building SQL INSERT queries.

use crate::{
	expr::SimpleExpr,
	types::{DynIden, IntoIden, IntoTableRef, TableRef},
	value::{IntoValue, Value, Values},
};

use super::{
	returning::ReturningClause,
	select::SelectStatement,
	traits::{QueryBuilderTrait, QueryStatementBuilder, QueryStatementWriter},
};

/// Source of data for INSERT statement
///
/// This enum represents the data source for an INSERT statement.
/// It can be either explicit values (VALUES clause) or a subquery (SELECT statement).
#[derive(Debug, Clone)]
pub enum InsertSource {
	/// Explicit values for INSERT (VALUES clause)
	Values(Vec<Vec<Value>>),
	/// Subquery for INSERT FROM SELECT
	Subquery(Box<SelectStatement>),
}

impl Default for InsertSource {
	fn default() -> Self {
		Self::Values(Vec::new())
	}
}

/// Mutually exclusive backend-specific INSERT prefixes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum InsertModifier {
	#[default]
	None,
	SqliteReplace,
	SqliteIgnore,
	MySqlIgnore,
}

/// INSERT statement builder
///
/// This struct provides a fluent API for constructing INSERT queries.
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_query::prelude::*;
///
/// let query = Query::insert()
///     .into_table("users")
///     .columns(["name", "email"])
///     .values_panic(["Alice", "alice@example.com"])
///     .values_panic(["Bob", "bob@example.com"]);
/// ```
#[derive(Debug, Clone)]
pub struct InsertStatement {
	pub(crate) table: Option<TableRef>,
	pub(crate) columns: Vec<DynIden>,
	pub(crate) source: InsertSource,
	pub(crate) expression_values: Option<Vec<Vec<SimpleExpr>>>,
	pub(crate) returning: Option<ReturningClause>,
	pub(crate) returning_exprs: Option<Vec<SimpleExpr>>,
	pub(crate) on_conflict: Option<super::on_conflict::OnConflict>,
	pub(crate) overriding_system_value: bool,
	pub(crate) default_values: bool,
	pub(crate) modifier: InsertModifier,
}

impl InsertStatement {
	/// Create a new INSERT statement
	pub fn new() -> Self {
		Self {
			table: None,
			columns: Vec::new(),
			source: InsertSource::Values(Vec::new()),
			expression_values: None,
			returning: None,
			returning_exprs: None,
			on_conflict: None,
			overriding_system_value: false,
			default_values: false,
			modifier: InsertModifier::None,
		}
	}

	/// Take the ownership of data in the current [`InsertStatement`]
	pub fn take(&mut self) -> Self {
		Self {
			table: self.table.take(),
			columns: std::mem::take(&mut self.columns),
			source: std::mem::replace(&mut self.source, InsertSource::Values(Vec::new())),
			expression_values: self.expression_values.take(),
			returning: self.returning.take(),
			returning_exprs: self.returning_exprs.take(),
			on_conflict: self.on_conflict.take(),
			overriding_system_value: std::mem::take(&mut self.overriding_system_value),
			default_values: std::mem::take(&mut self.default_values),
			modifier: std::mem::take(&mut self.modifier),
		}
	}

	/// Set the table to insert into
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::insert()
	///     .into_table("users");
	/// ```
	pub fn into_table<T>(&mut self, tbl: T) -> &mut Self
	where
		T: IntoTableRef,
	{
		self.table = Some(tbl.into_table_ref());
		self
	}

	/// Add a column to insert into
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::insert()
	///     .into_table("users")
	///     .column("name")
	///     .column("email");
	/// ```
	pub fn column<C>(&mut self, col: C) -> &mut Self
	where
		C: IntoIden,
	{
		self.default_values = false;
		self.columns.push(col.into_iden());
		self
	}

	/// Add multiple columns to insert into
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::insert()
	///     .into_table("users")
	///     .columns(["name", "email", "created_at"]);
	/// ```
	pub fn columns<I, C>(&mut self, cols: I) -> &mut Self
	where
		I: IntoIterator<Item = C>,
		C: IntoIden,
	{
		for col in cols {
			self.column(col);
		}
		self
	}

	/// Add values for the columns
	///
	/// Returns `Err` if the number of values doesn't match the number of columns.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let result = Query::insert()
	///     .into_table("users")
	///     .columns(["name", "email"])
	///     .values(vec!["Alice".into(), "alice@example.com".into()]);
	/// ```
	pub fn values(&mut self, values: Vec<Value>) -> Result<&mut Self, String> {
		if !self.columns.is_empty() && values.len() != self.columns.len() {
			return Err(format!(
				"Number of values ({}) doesn't match number of columns ({})",
				values.len(),
				self.columns.len()
			));
		}
		self.default_values = false;
		if let Some(rows) = &mut self.expression_values {
			rows.push(values.into_iter().map(SimpleExpr::Value).collect());
			return Ok(self);
		}
		match &mut self.source {
			InsertSource::Values(vals) => vals.push(values),
			InsertSource::Subquery(_) => {
				self.source = InsertSource::Values(vec![values]);
			}
		}
		Ok(self)
	}

	/// Add values for the columns (panics on mismatch)
	///
	/// # Panics
	///
	/// Panics if the number of values doesn't match the number of columns.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::insert()
	///     .into_table("users")
	///     .columns(["name", "email"])
	///     .values_panic(["Alice", "alice@example.com"])
	///     .values_panic(["Bob", "bob@example.com"]);
	/// ```
	pub fn values_panic<I, V>(&mut self, values: I) -> &mut Self
	where
		I: IntoIterator<Item = V>,
		V: IntoValue,
	{
		let values: Vec<Value> = values.into_iter().map(|v| v.into_value()).collect();
		if !self.columns.is_empty() && values.len() != self.columns.len() {
			panic!(
				"Number of values ({}) doesn't match number of columns ({})",
				values.len(),
				self.columns.len()
			);
		}
		self.default_values = false;
		if let Some(rows) = &mut self.expression_values {
			rows.push(values.into_iter().map(SimpleExpr::Value).collect());
			return self;
		}
		match &mut self.source {
			InsertSource::Values(vals) => vals.push(values),
			InsertSource::Subquery(_) => {
				self.source = InsertSource::Values(vec![values]);
			}
		}
		self
	}

	/// Append a VALUES row containing structural SQL expressions.
	///
	/// Values inside expressions retain their renderer-generated argument order.
	/// Existing value rows are preserved, and subsequent [`Self::values`] rows
	/// are appended in call order. Backend-checked rendering validates nested
	/// expressions. Construction and rendering have native/WASM parity (P2).
	///
	/// Returns an error when the row length differs from an already-set column
	/// list. A row replaces a previous SELECT source, matching [`Self::values`].
	///
	/// ```
	/// use reinhardt_query::{Expr, PostgresQueryBuilder, Query, QueryStatementBuilder};
	/// let mut statement = Query::insert();
	/// statement.into_table("events").columns(["name", "created_at"]);
	/// statement.values_expr(vec![Expr::val("created").into(), Expr::current_timestamp().into()])?;
	/// let (sql, values) = statement.build(PostgresQueryBuilder);
	/// assert_eq!(sql, "INSERT INTO \"events\" (\"name\", \"created_at\") VALUES ($1, CURRENT_TIMESTAMP)");
	/// assert_eq!(values.len(), 1);
	/// # Ok::<(), String>(())
	/// ```
	pub fn values_expr(&mut self, values: Vec<SimpleExpr>) -> Result<&mut Self, String> {
		if !self.columns.is_empty() && values.len() != self.columns.len() {
			return Err(format!(
				"Number of values ({}) doesn't match number of columns ({})",
				values.len(),
				self.columns.len()
			));
		}
		self.default_values = false;
		if self.expression_values.is_none() {
			let previous = std::mem::replace(&mut self.source, InsertSource::Values(Vec::new()));
			let rows = match previous {
				InsertSource::Values(rows) => rows
					.into_iter()
					.map(|row| row.into_iter().map(SimpleExpr::Value).collect())
					.collect(),
				InsertSource::Subquery(_) => Vec::new(),
			};
			self.expression_values = Some(rows);
		}
		if let Some(rows) = &mut self.expression_values {
			rows.push(values);
		}
		Ok(self)
	}

	/// Add a RETURNING clause with multiple columns
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::insert()
	///     .into_table("users")
	///     .columns(["name", "email"])
	///     .values_panic(["Alice", "alice@example.com"])
	///     .returning(["id", "created_at"]);
	/// ```
	pub fn returning<I, C>(&mut self, cols: I) -> &mut Self
	where
		I: IntoIterator<Item = C>,
		C: crate::types::IntoColumnRef,
	{
		self.returning = Some(ReturningClause::columns(cols));
		self.returning_exprs = None;
		self
	}

	/// Add a RETURNING clause for a single column
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::insert()
	///     .into_table("users")
	///     .columns(["name"])
	///     .values_panic(["Alice"])
	///     .returning_col(Alias::new("id"));
	/// ```
	pub fn returning_col<C>(&mut self, col: C) -> &mut Self
	where
		C: crate::types::IntoColumnRef,
	{
		self.returning = Some(ReturningClause::columns([col]));
		self.returning_exprs = None;
		self
	}

	/// Add a RETURNING clause with expressions.
	///
	/// Expressions can alias physical database columns to caller-visible names.
	pub fn returning_exprs<I, E>(&mut self, expressions: I) -> &mut Self
	where
		I: IntoIterator<Item = E>,
		E: Into<SimpleExpr>,
	{
		self.returning = None;
		self.returning_exprs = Some(expressions.into_iter().map(Into::into).collect());
		self
	}

	/// Use SQLite's INSERT OR REPLACE conflict algorithm.
	///
	/// This preserves SQLite's delete-and-insert replacement semantics, which
	/// differ from ON CONFLICT DO UPDATE. Checked builders reject other backends;
	/// legacy non-SQLite builders panic rather than silently discard this option.
	/// Construction/rendering has native/WASM behavioral parity (P2).
	/// The last backend-specific INSERT modifier replaces earlier modifiers.
	///
	/// ```
	/// use reinhardt_query::{Query, SqliteQueryBuilder, Value, Values};
	/// let statement = Query::insert().into_table("results").columns(["id"])
	///     .values_panic([42]).sqlite_or_replace().take();
	/// let (sql, values) = SqliteQueryBuilder.build_insert_checked(&statement).unwrap();
	/// assert_eq!(sql, "INSERT OR REPLACE INTO \"results\" (\"id\") VALUES (?)");
	/// assert_eq!(values, Values(vec![Value::Int(Some(42))]));
	/// ```
	pub fn sqlite_or_replace(&mut self) -> &mut Self {
		self.modifier = InsertModifier::SqliteReplace;
		self
	}

	/// Use MySQL's INSERT IGNORE modifier.
	///
	/// This preserves MySQL's warning and constraint handling, rather than
	/// simulating a no-op update on duplicate keys. Checked builders reject other
	/// backends; legacy builders panic instead of discarding the modifier.
	/// The last backend-specific INSERT modifier replaces earlier modifiers.
	/// Construction/rendering has native/WASM behavioral parity (P2).
	///
	/// ```
	/// use reinhardt_query::{MySqlQueryBuilder, Query, Value, Values};
	/// let statement = Query::insert().into_table("results").columns(["id"])
	///     .values_panic([42]).mysql_ignore().take();
	/// let (sql, values) = MySqlQueryBuilder.build_insert_checked(&statement).unwrap();
	/// assert_eq!(sql, "INSERT IGNORE INTO `results` (`id`) VALUES (?)");
	/// assert_eq!(values, Values(vec![Value::Int(Some(42))]));
	/// ```
	pub fn mysql_ignore(&mut self) -> &mut Self {
		self.modifier = InsertModifier::MySqlIgnore;
		self
	}

	/// Use SQLite's INSERT OR IGNORE conflict algorithm.
	///
	/// This selects SQLite's statement conflict algorithm without adding an
	/// ON CONFLICT target. Checked builders reject other backends; legacy builders
	/// panic instead of discarding the modifier. The last backend-specific INSERT
	/// modifier replaces earlier modifiers.
	/// Construction/rendering has native/WASM behavioral parity (P2).
	///
	/// ```
	/// use reinhardt_query::{Query, SqliteQueryBuilder, Value, Values};
	/// let statement = Query::insert().into_table("results").columns(["id"])
	///     .values_panic([42]).sqlite_or_ignore().take();
	/// let (sql, values) = SqliteQueryBuilder.build_insert_checked(&statement).unwrap();
	/// assert_eq!(sql, "INSERT OR IGNORE INTO \"results\" (\"id\") VALUES (?)");
	/// assert_eq!(values, Values(vec![Value::Int(Some(42))]));
	/// ```
	pub fn sqlite_or_ignore(&mut self) -> &mut Self {
		self.modifier = InsertModifier::SqliteIgnore;
		self
	}

	/// Set ON CONFLICT clause for upsert behavior.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	/// use reinhardt_query::query::OnConflict;
	///
	/// let query = Query::insert()
	///     .into_table("users")
	///     .columns(["id", "name"])
	///     .values_panic([1, "Alice"])
	///     .on_conflict(OnConflict::column("id").update_columns(["name"]));
	/// ```
	pub fn on_conflict(&mut self, on_conflict: super::on_conflict::OnConflict) -> &mut Self {
		self.on_conflict = Some(on_conflict);
		self
	}

	/// Add PostgreSQL's `OVERRIDING SYSTEM VALUE` clause.
	///
	/// This permits explicit values for columns declared as `GENERATED ALWAYS AS IDENTITY`.
	/// Non-PostgreSQL query builders ignore the clause.
	pub fn overriding_system_value(&mut self) -> &mut Self {
		self.overriding_system_value = true;
		self
	}

	/// Add a RETURNING * clause
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::insert()
	///     .into_table("users")
	///     .columns(["name", "email"])
	///     .values_panic(["Alice", "alice@example.com"])
	///     .returning_all();
	/// ```
	pub fn returning_all(&mut self) -> &mut Self {
		self.returning = Some(ReturningClause::all());
		self.returning_exprs = None;
		self
	}

	/// Use a subquery as the data source for INSERT
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let select = Query::select()
	///     .column("name")
	///     .column("email")
	///     .from("temp_users");
	///
	/// let query = Query::insert()
	///     .into_table("users")
	///     .columns(["name", "email"])
	///     .from_subquery(select);
	/// ```
	pub fn from_subquery(&mut self, select: SelectStatement) -> &mut Self {
		self.default_values = false;
		self.expression_values = None;
		self.source = InsertSource::Subquery(Box::new(select));
		self
	}

	/// Insert one row using the table's column defaults.
	///
	/// SQLite does not support combining `DEFAULT VALUES` with `ON CONFLICT`,
	/// so the SQLite query builder rejects that combination.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::insert()
	///     .into_table("settings")
	///     .default_values();
	/// ```
	pub fn default_values(&mut self) -> &mut Self {
		self.columns.clear();
		self.source = InsertSource::Values(Vec::new());
		self.expression_values = None;
		self.default_values = true;
		self
	}

	/// Get the values if this is a VALUES source
	///
	/// Returns `None` if the source is a subquery or contains expression rows.
	pub fn get_values(&self) -> Option<&Vec<Vec<Value>>> {
		if self.expression_values.is_some() {
			return None;
		}
		match &self.source {
			InsertSource::Values(vals) => Some(vals),
			InsertSource::Subquery(_) => None,
		}
	}
}

impl Default for InsertStatement {
	fn default() -> Self {
		Self::new()
	}
}

impl QueryStatementBuilder for InsertStatement {
	fn to_string<T: QueryBuilderTrait>(&self, query_builder: T) -> String {
		crate::query::traits::postgres_to_string(
			self,
			query_builder,
			crate::backend::PostgresQueryBuilder::build_insert_with_writer,
		)
	}

	fn build_any(&self, query_builder: &dyn QueryBuilderTrait) -> (String, Values) {
		use crate::backend::{
			MySqlQueryBuilder, PostgresQueryBuilder, QueryBuilder, SqliteQueryBuilder,
		};
		use std::any::Any;

		let any_builder = query_builder as &dyn Any;

		if let Some(pg) = any_builder.downcast_ref::<PostgresQueryBuilder>() {
			return pg.build_insert(self);
		}

		if let Some(mysql) = any_builder.downcast_ref::<MySqlQueryBuilder>() {
			return mysql.build_insert(self);
		}

		if let Some(sqlite) = any_builder.downcast_ref::<SqliteQueryBuilder>() {
			return sqlite.build_insert(self);
		}

		panic!(
			"Unsupported query builder type. Use PostgresQueryBuilder, MySqlQueryBuilder, or SqliteQueryBuilder."
		);
	}
}

impl QueryStatementWriter for InsertStatement {}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Query;
	use rstest::rstest;

	#[rstest]
	fn test_insert_basic() {
		let mut query = InsertStatement::new();
		query
			.into_table("users")
			.columns(["name", "email"])
			.values_panic(["Alice", "alice@example.com"]);

		assert!(query.table.is_some());
		assert_eq!(query.columns.len(), 2);
		let values = query.get_values().expect("should have values");
		assert_eq!(values.len(), 1);
		assert_eq!(values[0].len(), 2);
	}

	#[rstest]
	fn test_insert_multiple_rows() {
		let mut query = InsertStatement::new();
		query
			.into_table("users")
			.columns(["name", "email"])
			.values_panic(["Alice", "alice@example.com"])
			.values_panic(["Bob", "bob@example.com"]);

		let values = query.get_values().expect("should have values");
		assert_eq!(values.len(), 2);
	}

	#[rstest]
	fn test_insert_default_values_builds_for_each_backend() {
		use crate::backend::{MySqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder};

		// Arrange
		let mut query = InsertStatement::new();
		query.into_table("users").default_values();
		let query = query.take();

		// Act / Assert
		assert_eq!(
			query.build(PostgresQueryBuilder).0,
			"INSERT INTO \"users\" DEFAULT VALUES"
		);
		assert_eq!(
			query.build(SqliteQueryBuilder).0,
			"INSERT INTO \"users\" DEFAULT VALUES"
		);
		assert_eq!(
			query.build(MySqlQueryBuilder).0,
			"INSERT INTO `users` () VALUES ()"
		);
	}

	#[rstest]
	fn test_mysql_default_values_preserves_do_nothing_conflict_handling() {
		use crate::backend::MySqlQueryBuilder;
		use crate::query::OnConflict;

		// Arrange
		let mut query = InsertStatement::new();
		query
			.into_table("settings")
			.default_values()
			.on_conflict(OnConflict::column("key").do_nothing());

		// Act
		let sql = query.build(MySqlQueryBuilder).0;

		// Assert
		assert_eq!(
			sql,
			"INSERT INTO `settings` () VALUES () ON DUPLICATE KEY UPDATE `key` = `key`"
		);
	}

	#[rstest]
	#[should_panic(expected = "SQLite does not support ON CONFLICT with DEFAULT VALUES")]
	fn test_sqlite_default_values_rejects_conflict_handling() {
		use crate::backend::SqliteQueryBuilder;
		use crate::query::OnConflict;

		let mut query = InsertStatement::new();
		query
			.into_table("settings")
			.default_values()
			.on_conflict(OnConflict::column("key").do_nothing());

		query.build(SqliteQueryBuilder);
	}

	#[rstest]
	fn test_insert_without_source_does_not_default_values() {
		use crate::backend::PostgresQueryBuilder;

		// Arrange
		let mut query = InsertStatement::new();
		query.into_table("users");

		// Act / Assert
		assert_eq!(query.build(PostgresQueryBuilder).0, "INSERT INTO \"users\"");
	}

	#[test]
	#[should_panic(expected = "Number of values")]
	fn test_insert_values_mismatch() {
		let mut query = InsertStatement::new();
		query
			.into_table("users")
			.columns(["name", "email"])
			.values_panic(["Alice"]); // Should panic: 1 value, 2 columns
	}

	#[test]
	fn test_insert_returning() {
		let mut query = InsertStatement::new();
		query
			.into_table("users")
			.columns(["name"])
			.values_panic(["Alice"])
			.returning(["id", "created_at"]);

		assert!(query.returning.is_some());
		let returning = query.returning.unwrap();
		assert!(!returning.is_all());
	}

	#[test]
	fn test_insert_returning_all() {
		let mut query = InsertStatement::new();
		query
			.into_table("users")
			.columns(["name"])
			.values_panic(["Alice"])
			.returning_all();

		assert!(query.returning.is_some());
		let returning = query.returning.unwrap();
		assert!(returning.is_all());
	}

	#[test]
	fn test_insert_take() {
		let mut query = InsertStatement::new();
		query
			.into_table("users")
			.columns(["name"])
			.values_panic(["Alice"]);

		let taken = query.take();
		assert!(taken.table.is_some());
		assert!(query.table.is_none());
	}

	#[test]
	fn test_insert_from_subquery() {
		let mut query = InsertStatement::new();
		let select = Query::select()
			.column("name")
			.column("email")
			.from("temp_users")
			.to_owned();

		query
			.into_table("users")
			.columns(["name", "email"])
			.from_subquery(select);

		assert!(query.table.is_some());
		assert_eq!(query.columns.len(), 2);
		assert!(
			query.get_values().is_none(),
			"should not have values when using subquery"
		);
	}
}

#[cfg(test)]
mod sqlite_replace_tests {
	use crate::{
		CockroachDBQueryBuilder, MySqlQueryBuilder, PostgresQueryBuilder, Query, QueryBuildError,
		SqliteQueryBuilder, Value, Values,
	};
	use rstest::rstest;
	#[rstest]
	fn checked_replace_preserves_values_and_take() {
		// Arrange
		let mut statement = Query::insert();
		statement
			.into_table("results")
			.columns(["id", "result"])
			.values_panic(vec![Value::Int(Some(42)), Value::String(None)])
			.sqlite_or_replace();
		// Act
		let taken = statement.take();
		let (sql, values) = SqliteQueryBuilder.build_insert_checked(&taken).unwrap();
		// Assert
		assert_eq!(
			sql,
			"INSERT OR REPLACE INTO \"results\" (\"id\", \"result\") VALUES (?, NULL)"
		);
		assert_eq!(values, Values(vec![Value::Int(Some(42))]));
		assert_eq!(statement.modifier, super::InsertModifier::None);
		for (result, backend) in [
			(
				PostgresQueryBuilder.build_insert_checked(&taken),
				"PostgreSQL",
			),
			(MySqlQueryBuilder.build_insert_checked(&taken), "MySQL"),
			(
				CockroachDBQueryBuilder::new().build_insert_checked(&taken),
				"CockroachDB",
			),
		] {
			assert_eq!(
				result,
				Err(QueryBuildError::UnsupportedBackendFeature {
					feature: "SQLite INSERT OR REPLACE",
					backend
				})
			);
		}
	}
}

#[cfg(test)]
mod ignore_modifier_tests {
	use crate::{
		CockroachDBQueryBuilder, MySqlQueryBuilder, PostgresQueryBuilder, Query, QueryBuildError,
		QueryBuilder, SqliteQueryBuilder, Value, Values,
	};
	use rstest::rstest;

	#[rstest]
	#[case(
		true,
		"INSERT IGNORE INTO `results` (`id`, `text`, `absent`) VALUES (?, ?, NULL)"
	)]
	#[case(
		false,
		"INSERT OR IGNORE INTO \"results\" (\"id\", \"text\", \"absent\") VALUES (?, ?, NULL)"
	)]
	fn ignore_modifier_keeps_values_and_resets_on_take(
		#[case] mysql: bool,
		#[case] expected_sql: &str,
	) {
		// Arrange: modifier selection is last-call-wins, including the existing replacement mode.
		let mut statement = Query::insert();
		statement
			.into_table("results")
			.columns(["id", "text", "absent"])
			.values_panic([
				Value::Int(Some(42)),
				"quote' ? $1".into(),
				Value::String(None),
			])
			.sqlite_or_replace();
		if mysql {
			statement.sqlite_or_ignore().mysql_ignore();
		} else {
			statement.mysql_ignore().sqlite_or_ignore();
		}
		// Act
		let taken = statement.take();
		let result = if mysql {
			MySqlQueryBuilder.build_insert_checked(&taken)
		} else {
			SqliteQueryBuilder.build_insert_checked(&taken)
		};
		// Assert
		assert_eq!(
			result.unwrap(),
			(
				expected_sql.into(),
				Values(vec![42.into(), "quote' ? $1".into()])
			)
		);
		statement
			.into_table("results")
			.columns(["id"])
			.values_panic([7]);
		assert_eq!(
			PostgresQueryBuilder
				.build_insert_checked(&statement)
				.unwrap(),
			(
				"INSERT INTO \"results\" (\"id\") VALUES ($1)".into(),
				Values(vec![7.into()])
			)
		);
	}

	#[rstest]
	#[case(true, "MySQL INSERT IGNORE")]
	#[case(false, "SQLite INSERT OR IGNORE")]
	fn checked_ignore_rejects_every_other_backend(
		#[case] mysql: bool,
		#[case] feature: &'static str,
	) {
		// Arrange
		let mut statement = Query::insert();
		statement
			.into_table("results")
			.columns(["id"])
			.values_panic([42]);
		if mysql {
			statement.mysql_ignore();
		} else {
			statement.sqlite_or_ignore();
		}
		// Act / Assert
		let other = if mysql {
			(
				SqliteQueryBuilder.build_insert_checked(&statement),
				"SQLite",
			)
		} else {
			(MySqlQueryBuilder.build_insert_checked(&statement), "MySQL")
		};
		for (result, backend) in [
			(
				PostgresQueryBuilder.build_insert_checked(&statement),
				"PostgreSQL",
			),
			(
				CockroachDBQueryBuilder::new().build_insert_checked(&statement),
				"CockroachDB",
			),
			other,
		] {
			assert_eq!(
				result,
				Err(QueryBuildError::UnsupportedBackendFeature { feature, backend })
			);
		}
	}

	#[rstest]
	#[case(true)]
	#[case(false)]
	fn unchecked_ignore_cannot_silently_discard_modifier(#[case] mysql: bool) {
		// Arrange / Act
		let render = || {
			let mut statement = Query::insert();
			statement
				.into_table("results")
				.columns(["id"])
				.values_panic([42]);
			if mysql {
				statement.mysql_ignore();
			} else {
				statement.sqlite_or_ignore();
			}
			PostgresQueryBuilder.build_insert(&statement)
		};
		// Assert
		assert!(std::panic::catch_unwind(render).is_err());
	}

	#[rstest]
	fn replace_modifier_replaces_an_earlier_ignore() {
		// Arrange
		let mut statement = Query::insert();
		statement
			.into_table("results")
			.columns(["id"])
			.values_panic([42])
			.mysql_ignore()
			.sqlite_or_ignore()
			.sqlite_or_replace();
		// Act / Assert
		assert_eq!(
			SqliteQueryBuilder.build_insert_checked(&statement).unwrap(),
			(
				"INSERT OR REPLACE INTO \"results\" (\"id\") VALUES (?)".into(),
				Values(vec![42.into()])
			)
		);
	}
}
