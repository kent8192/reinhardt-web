//! Query builder with dialect support
//!
//! This module provides high-level query builders for INSERT, UPDATE, SELECT, and DELETE operations
//! using SeaQuery for cross-database compatibility.
//!
//! # Identifier Quoting
//!
//! All SQL identifiers (table names, column names) are automatically quoted according to the database backend:
//!
//! | Backend | Quote Character | Example |
//! |---------|-----------------|---------|
//! | PostgreSQL | Double quotes (`"`) | `INSERT INTO "users" ("name", "email") VALUES ($1, $2)` |
//! | MySQL | Backticks (`` ` ``) | `` INSERT INTO `users` (`name`, `email`) VALUES (?, ?) `` |
//! | SQLite | Double quotes (`"`) | `INSERT INTO "users" ("name", "email") VALUES (?, ?)` |
//!
//! This quoting preserves case sensitivity and allows special characters in identifiers.
//!
//! # Testing Generated SQL
//!
//! When writing tests that check generated SQL strings, account for the quoted identifiers:
//!
//! ```rust,ignore
//! let (sql, _) = InsertBuilder::new(backend, "users")
//!     .value("name", "Alice")
//!     .build();
//!
//! // ❌ Fails - doesn't account for quotes
//! assert!(sql.contains("INSERT INTO users"));
//!
//! // ✅ Works - checks parts separately
//! assert!(sql.contains("INSERT INTO") && sql.contains("users"));
//!
//! // ✅ Works - includes quotes (PostgreSQL)
//! assert!(sql.contains(r#"INSERT INTO "users""#));
//! ```
//!
//! See the test cases in this module for more examples.

use std::sync::Arc;

use sea_query::{Alias, Asterisk, Expr, ExprTrait, Query, Value};

use super::{
	backend::DatabaseBackend,
	error::Result,
	types::{QueryResult, QueryValue, Row},
};

/// Convert QueryValue to SeaQuery Value
fn query_value_to_sea_value(qv: &QueryValue) -> Value {
	match qv {
		// BigInt(None) is used for generic NULL values across all dialects
		// (consistent with PostgreSQL, MySQL, SQLite backend implementations)
		QueryValue::Null => Value::BigInt(None),
		QueryValue::Bool(b) => Value::Bool(Some(*b)),
		QueryValue::Int(i) => Value::BigInt(Some(*i)),
		QueryValue::Float(f) => Value::Double(Some(*f)),
		QueryValue::String(s) => Value::String(Some(s.clone())),
		QueryValue::Bytes(b) => Value::Bytes(Some(b.clone())),
		QueryValue::Timestamp(dt) => Value::ChronoDateTimeUtc(Some(*dt)),
		QueryValue::Uuid(u) => Value::Uuid(Some(*u)),
		// NOW() is handled specially in build() methods, should not reach here
		QueryValue::Now => {
			panic!("QueryValue::Now should be handled in build() method, not converted to Value")
		}
	}
}

/// ON CONFLICT action for INSERT statements
#[derive(Debug, Clone)]
pub enum OnConflictAction {
	/// Do nothing on conflict (PostgreSQL: ON CONFLICT DO NOTHING, MySQL: INSERT IGNORE, SQLite: INSERT OR IGNORE)
	DoNothing {
		/// Conflict columns (PostgreSQL only)
		conflict_columns: Option<Vec<String>>,
	},
	/// Update on conflict (PostgreSQL: ON CONFLICT DO UPDATE, MySQL: ON DUPLICATE KEY UPDATE)
	DoUpdate {
		/// Conflict columns (PostgreSQL only)
		conflict_columns: Option<Vec<String>>,
		/// Columns to update on conflict
		update_columns: Vec<String>,
	},
}

/// INSERT query builder
pub struct InsertBuilder {
	backend: Arc<dyn DatabaseBackend>,
	table: String,
	columns: Vec<String>,
	values: Vec<QueryValue>,
	returning: Option<Vec<String>>,
	on_conflict: Option<OnConflictAction>,
}

impl InsertBuilder {
	pub fn new(backend: Arc<dyn DatabaseBackend>, table: impl Into<String>) -> Self {
		Self {
			backend,
			table: table.into(),
			columns: Vec::new(),
			values: Vec::new(),
			returning: None,
			on_conflict: None,
		}
	}

	pub fn value(mut self, column: impl Into<String>, value: impl Into<QueryValue>) -> Self {
		self.columns.push(column.into());
		self.values.push(value.into());
		self
	}

	pub fn returning(mut self, columns: Vec<&str>) -> Self {
		if self.backend.supports_returning() {
			self.returning = Some(columns.iter().map(|s| (*s).to_owned()).collect());
		}
		self
	}

	/// Set ON CONFLICT DO NOTHING behavior
	///
	/// # Arguments
	///
	/// * `conflict_columns` - Columns to check for conflict (PostgreSQL only, None for all unique constraints)
	///
	/// # Example
	///
	/// ```rust,ignore
	/// builder.on_conflict_do_nothing(Some(vec!["email".to_string()]))
	/// ```
	pub fn on_conflict_do_nothing(mut self, conflict_columns: Option<Vec<String>>) -> Self {
		self.on_conflict = Some(OnConflictAction::DoNothing { conflict_columns });
		self
	}

	/// Set ON CONFLICT DO UPDATE behavior
	///
	/// # Arguments
	///
	/// * `conflict_columns` - Columns to check for conflict (PostgreSQL only)
	/// * `update_columns` - Columns to update on conflict
	///
	/// # Example
	///
	/// ```rust,ignore
	/// builder.on_conflict_do_update(
	///     Some(vec!["email".to_string()]),
	///     vec!["name".to_string(), "updated_at".to_string()],
	/// )
	/// ```
	pub fn on_conflict_do_update(
		mut self,
		conflict_columns: Option<Vec<String>>,
		update_columns: Vec<String>,
	) -> Self {
		self.on_conflict = Some(OnConflictAction::DoUpdate {
			conflict_columns,
			update_columns,
		});
		self
	}

	pub fn build(&self) -> (String, Vec<QueryValue>) {
		use super::types::DatabaseType;
		use sea_query::{MysqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder};

		let mut stmt = Query::insert()
			.into_table(Alias::new(&self.table))
			.to_owned();

		// Add columns
		let column_refs: Vec<Alias> = self.columns.iter().map(Alias::new).collect();
		stmt.columns(column_refs);

		// Add values
		if !self.values.is_empty() {
			let sea_values: Vec<Expr> = self
				.values
				.iter()
				.map(|v| Expr::val(query_value_to_sea_value(v)))
				.collect();
			stmt.values(sea_values).unwrap();
		}

		// Add RETURNING clause if supported
		if let Some(ref cols) = self.returning {
			for col in cols {
				stmt.returning(Query::returning().column(Alias::new(col)));
			}
		}

		// Build SQL based on database type
		let mut sql = match self.backend.database_type() {
			DatabaseType::Postgres => stmt.to_string(PostgresQueryBuilder),
			DatabaseType::Mysql => stmt.to_string(MysqlQueryBuilder),
			DatabaseType::Sqlite => stmt.to_string(SqliteQueryBuilder),
		};

		// Add ON CONFLICT clause if specified
		if let Some(ref on_conflict) = self.on_conflict {
			sql = self.apply_on_conflict_clause(sql, on_conflict);
		}

		(sql, self.values.clone())
	}

	/// Apply ON CONFLICT clause to SQL string based on database type
	fn apply_on_conflict_clause(&self, mut sql: String, action: &OnConflictAction) -> String {
		use super::types::DatabaseType;

		match self.backend.database_type() {
			DatabaseType::Postgres => match action {
				OnConflictAction::DoNothing { conflict_columns } => {
					if let Some(cols) = conflict_columns {
						let cols_str = cols.join(", ");
						sql.push_str(&format!(" ON CONFLICT ({}) DO NOTHING", cols_str));
					} else {
						sql.push_str(" ON CONFLICT DO NOTHING");
					}
				}
				OnConflictAction::DoUpdate {
					conflict_columns,
					update_columns,
				} => {
					let conflict_str = if let Some(cols) = conflict_columns {
						format!("({})", cols.join(", "))
					} else {
						String::new()
					};

					let update_str = update_columns
						.iter()
						.map(|col| format!("{} = EXCLUDED.{}", col, col))
						.collect::<Vec<_>>()
						.join(", ");

					sql.push_str(&format!(
						" ON CONFLICT {} DO UPDATE SET {}",
						conflict_str, update_str
					));
				}
			},
			DatabaseType::Mysql => {
				match action {
					OnConflictAction::DoNothing { .. } => {
						// MySQL: INSERT IGNORE
						sql = sql.replacen("INSERT", "INSERT IGNORE", 1);
					}
					OnConflictAction::DoUpdate {
						conflict_columns: _,
						update_columns,
					} => {
						// MySQL: ON DUPLICATE KEY UPDATE
						let update_str = update_columns
							.iter()
							.map(|col| format!("{} = VALUES({})", col, col))
							.collect::<Vec<_>>()
							.join(", ");

						sql.push_str(&format!(" ON DUPLICATE KEY UPDATE {}", update_str));
					}
				}
			}
			DatabaseType::Sqlite => {
				match action {
					OnConflictAction::DoNothing { .. } => {
						// SQLite: INSERT OR IGNORE
						sql = sql.replacen("INSERT", "INSERT OR IGNORE", 1);
					}
					OnConflictAction::DoUpdate {
						conflict_columns,
						update_columns,
					} => {
						// SQLite: ON CONFLICT DO UPDATE (SQLite 3.24.0+)
						let conflict_str = if let Some(cols) = conflict_columns {
							if cols.is_empty() {
								panic!(
									"SQLite ON CONFLICT requires non-empty conflict_columns for DO UPDATE"
								);
							}
							format!("({})", cols.join(", "))
						} else {
							// SQLite requires conflict target - skip ON CONFLICT clause
							return sql;
						};

						if update_columns.is_empty() {
							panic!("update_columns cannot be empty for OnConflictAction::DoUpdate");
						}

						let update_str = update_columns
							.iter()
							.map(|col| format!("{} = excluded.{}", col, col)) // lowercase 'excluded'
							.collect::<Vec<_>>()
							.join(", ");

						sql.push_str(&format!(
							" ON CONFLICT {} DO UPDATE SET {}",
							conflict_str, update_str
						));
					}
				}
			}
		}

		sql
	}

	pub async fn execute(&self) -> Result<QueryResult> {
		let (sql, params) = self.build();
		self.backend.execute(&sql, params).await
	}

	pub async fn fetch_one(&self) -> Result<Row> {
		let (sql, params) = self.build();
		self.backend.fetch_one(&sql, params).await
	}
}

/// UPDATE query builder
pub struct UpdateBuilder {
	backend: Arc<dyn DatabaseBackend>,
	table: String,
	sets: Vec<(String, QueryValue)>,
	wheres: Vec<(String, String, QueryValue)>,
}

impl UpdateBuilder {
	pub fn new(backend: Arc<dyn DatabaseBackend>, table: impl Into<String>) -> Self {
		Self {
			backend,
			table: table.into(),
			sets: Vec::new(),
			wheres: Vec::new(),
		}
	}

	pub fn set(mut self, column: impl Into<String>, value: impl Into<QueryValue>) -> Self {
		self.sets.push((column.into(), value.into()));
		self
	}

	pub fn set_now(mut self, column: impl Into<String>) -> Self {
		self.sets.push((column.into(), QueryValue::Now));
		self
	}

	pub fn where_eq(mut self, column: impl Into<String>, value: impl Into<QueryValue>) -> Self {
		self.wheres
			.push((column.into(), "=".to_string(), value.into()));
		self
	}

	pub fn build(&self) -> (String, Vec<QueryValue>) {
		use super::types::DatabaseType;
		use sea_query::{MysqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder};

		let mut stmt = Query::update().table(Alias::new(&self.table)).to_owned();

		// Add SET clauses
		for (col, val) in &self.sets {
			if matches!(val, QueryValue::Now) {
				stmt.value(Alias::new(col), Expr::cust("NOW()"));
				continue;
			}
			stmt.value(Alias::new(col), query_value_to_sea_value(val));
		}

		// Add WHERE clauses
		for (col, op, val) in &self.wheres {
			if op == "=" {
				stmt.and_where(
					Expr::col(Alias::new(col)).eq(Expr::val(query_value_to_sea_value(val))),
				);
			}
		}

		// Build SQL based on database type
		let sql = match self.backend.database_type() {
			DatabaseType::Postgres => stmt.to_string(PostgresQueryBuilder),
			DatabaseType::Mysql => stmt.to_string(MysqlQueryBuilder),
			DatabaseType::Sqlite => stmt.to_string(SqliteQueryBuilder),
		};

		// Preserve parameter order: first SET values, then WHERE values
		let mut params = Vec::new();
		for (_, val) in &self.sets {
			if !matches!(val, QueryValue::Now) {
				params.push(val.clone());
			}
		}
		for (_, _, val) in &self.wheres {
			params.push(val.clone());
		}

		(sql, params)
	}

	pub async fn execute(&self) -> Result<QueryResult> {
		let (sql, params) = self.build();
		self.backend.execute(&sql, params).await
	}
}

/// SELECT query builder
pub struct SelectBuilder {
	backend: Arc<dyn DatabaseBackend>,
	columns: Vec<String>,
	table: String,
	wheres: Vec<(String, String, QueryValue)>,
	limit: Option<i64>,
}

impl SelectBuilder {
	pub fn new(backend: Arc<dyn DatabaseBackend>) -> Self {
		Self {
			backend,
			columns: vec!["*".to_string()],
			table: String::new(),
			wheres: Vec::new(),
			limit: None,
		}
	}

	pub fn columns(mut self, columns: Vec<&str>) -> Self {
		self.columns = columns.iter().map(|s| s.to_string()).collect();
		self
	}

	pub fn from(mut self, table: impl Into<String>) -> Self {
		self.table = table.into();
		self
	}

	pub fn where_eq(mut self, column: impl Into<String>, value: impl Into<QueryValue>) -> Self {
		self.wheres
			.push((column.into(), "=".to_string(), value.into()));
		self
	}

	pub fn limit(mut self, limit: i64) -> Self {
		self.limit = Some(limit);
		self
	}

	pub fn build(&self) -> (String, Vec<QueryValue>) {
		use super::types::DatabaseType;
		use sea_query::{MysqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder};

		let mut stmt = Query::select().from(Alias::new(&self.table)).to_owned();

		// Add columns
		if self.columns == vec!["*".to_string()] {
			stmt.column(Asterisk);
		} else {
			for col in &self.columns {
				stmt.column(Alias::new(col));
			}
		}

		// Add WHERE clauses
		for (col, op, val) in &self.wheres {
			if op == "=" {
				stmt.and_where(
					Expr::col(Alias::new(col)).eq(Expr::val(query_value_to_sea_value(val))),
				);
			}
		}

		// Add LIMIT
		if let Some(limit) = self.limit {
			stmt.limit(limit as u64);
		}

		// Build SQL
		let sql = match self.backend.database_type() {
			DatabaseType::Postgres => stmt.to_string(PostgresQueryBuilder),
			DatabaseType::Mysql => stmt.to_string(MysqlQueryBuilder),
			DatabaseType::Sqlite => stmt.to_string(SqliteQueryBuilder),
		};

		// Collect parameters
		let params: Vec<QueryValue> = self.wheres.iter().map(|(_, _, val)| val.clone()).collect();

		(sql, params)
	}

	pub async fn fetch_all(&self) -> Result<Vec<Row>> {
		let (sql, params) = self.build();
		self.backend.fetch_all(&sql, params).await
	}

	pub async fn fetch_one(&self) -> Result<Row> {
		let (sql, params) = self.build();
		self.backend.fetch_one(&sql, params).await
	}
}

/// DELETE query builder
pub struct DeleteBuilder {
	backend: Arc<dyn DatabaseBackend>,
	table: String,
	wheres: Vec<(String, String, QueryValue)>,
}

impl DeleteBuilder {
	pub fn new(backend: Arc<dyn DatabaseBackend>, table: impl Into<String>) -> Self {
		Self {
			backend,
			table: table.into(),
			wheres: Vec::new(),
		}
	}

	pub fn where_eq(mut self, column: impl Into<String>, value: impl Into<QueryValue>) -> Self {
		self.wheres
			.push((column.into(), "=".to_string(), value.into()));
		self
	}

	pub fn where_in(mut self, column: impl Into<String> + Clone, values: Vec<QueryValue>) -> Self {
		for value in values {
			self.wheres
				.push((column.clone().into(), "IN".to_string(), value));
		}
		self
	}

	pub fn build(&self) -> (String, Vec<QueryValue>) {
		use super::types::DatabaseType;
		use sea_query::{MysqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder};

		let mut stmt = Query::delete()
			.from_table(Alias::new(&self.table))
			.to_owned();

		// Add WHERE clauses
		for (col, op, val) in &self.wheres {
			match op.as_str() {
				"=" => {
					stmt.and_where(
						Expr::col(Alias::new(col)).eq(Expr::val(query_value_to_sea_value(val))),
					);
				}
				"IN" => {
					stmt.and_where(
						Expr::col(Alias::new(col))
							.is_in([Expr::val(query_value_to_sea_value(val))]),
					);
				}
				_ => {}
			}
		}

		// Build SQL
		let sql = match self.backend.database_type() {
			DatabaseType::Postgres => stmt.to_string(PostgresQueryBuilder),
			DatabaseType::Mysql => stmt.to_string(MysqlQueryBuilder),
			DatabaseType::Sqlite => stmt.to_string(SqliteQueryBuilder),
		};

		// Collect parameters
		let params: Vec<QueryValue> = self.wheres.iter().map(|(_, _, val)| val.clone()).collect();

		(sql, params)
	}

	pub async fn execute(&self) -> Result<QueryResult> {
		let (sql, params) = self.build();
		self.backend.execute(&sql, params).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::backends::backend::DatabaseBackend;
	use crate::backends::types::{DatabaseType, QueryResult, QueryValue, Row, TransactionExecutor};

	// Mock transaction executor for testing
	struct MockTransactionExecutor;

	#[async_trait::async_trait]
	impl TransactionExecutor for MockTransactionExecutor {
		async fn execute(&mut self, _sql: &str, _params: Vec<QueryValue>) -> Result<QueryResult> {
			Ok(QueryResult { rows_affected: 0 })
		}

		async fn fetch_one(&mut self, _sql: &str, _params: Vec<QueryValue>) -> Result<Row> {
			Ok(Row::new())
		}

		async fn fetch_all(&mut self, _sql: &str, _params: Vec<QueryValue>) -> Result<Vec<Row>> {
			Ok(Vec::new())
		}

		async fn fetch_optional(
			&mut self,
			_sql: &str,
			_params: Vec<QueryValue>,
		) -> Result<Option<Row>> {
			Ok(None)
		}

		async fn commit(self: Box<Self>) -> Result<()> {
			Ok(())
		}

		async fn rollback(self: Box<Self>) -> Result<()> {
			Ok(())
		}
	}

	struct MockBackend;

	#[async_trait::async_trait]
	impl DatabaseBackend for MockBackend {
		fn database_type(&self) -> DatabaseType {
			DatabaseType::Postgres
		}

		fn placeholder(&self, index: usize) -> String {
			format!("${}", index)
		}

		fn supports_returning(&self) -> bool {
			true
		}

		fn supports_on_conflict(&self) -> bool {
			true
		}

		async fn execute(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<QueryResult> {
			Ok(QueryResult { rows_affected: 1 })
		}

		async fn fetch_one(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<Row> {
			Ok(Row::new())
		}

		async fn fetch_all(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<Vec<Row>> {
			Ok(Vec::new())
		}

		async fn fetch_optional(
			&self,
			_sql: &str,
			_params: Vec<QueryValue>,
		) -> Result<Option<Row>> {
			Ok(None)
		}

		fn as_any(&self) -> &dyn std::any::Any {
			self
		}

		async fn begin(&self) -> Result<Box<dyn TransactionExecutor>> {
			Ok(Box::new(MockTransactionExecutor))
		}
	}

	#[test]
	fn test_delete_builder_basic() {
		let backend = Arc::new(MockBackend);
		let builder = DeleteBuilder::new(backend, "users");
		let (sql, params) = builder.build();

		// SeaQuery uses quotes for identifiers
		assert_eq!(sql, "DELETE FROM \"users\"");
		assert!(params.is_empty());
	}

	#[test]
	fn test_delete_builder_where_eq() {
		let backend = Arc::new(MockBackend);
		let builder = DeleteBuilder::new(backend, "users").where_eq("id", QueryValue::Int(1));
		let (sql, params) = builder.build();

		// SeaQuery embeds values directly in SQL when using to_string()
		assert_eq!(sql, "DELETE FROM \"users\" WHERE \"id\" = 1");
		assert_eq!(params.len(), 1);
		assert!(matches!(params[0], QueryValue::Int(1)));
	}

	#[test]
	fn test_delete_builder_where_in() {
		let backend = Arc::new(MockBackend);
		let builder = DeleteBuilder::new(backend, "users")
			.where_in("id", vec![QueryValue::Int(1), QueryValue::Int(2)]);
		let (sql, params) = builder.build();

		// SeaQuery embeds values directly in SQL when using to_string()
		assert_eq!(
			sql,
			"DELETE FROM \"users\" WHERE \"id\" IN (1) AND \"id\" IN (2)"
		);
		assert_eq!(params.len(), 2);
		assert!(matches!(params[0], QueryValue::Int(1)));
		assert!(matches!(params[1], QueryValue::Int(2)));
	}

	#[test]
	fn test_delete_builder_multiple_conditions() {
		let backend = Arc::new(MockBackend);
		let builder = DeleteBuilder::new(backend, "users")
			.where_eq("status", QueryValue::String("inactive".to_string()))
			.where_eq("age", QueryValue::Int(18));
		let (sql, params) = builder.build();

		// SeaQuery embeds values directly in SQL when using to_string()
		assert_eq!(
			sql,
			"DELETE FROM \"users\" WHERE \"status\" = 'inactive' AND \"age\" = 18"
		);
		assert_eq!(params.len(), 2);
	}
}
