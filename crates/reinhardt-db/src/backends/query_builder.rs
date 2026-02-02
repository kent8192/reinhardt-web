//! Query builder with dialect support

use std::sync::Arc;

use sea_query::{Alias, Asterisk, Expr, ExprTrait, Query, SelectStatement, Value};

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

/// Conflict target specifying which constraint or columns trigger the conflict
#[derive(Debug, Clone)]
pub enum ConflictTarget {
	/// Specify conflict columns (e.g., `ON CONFLICT (email, tenant_id)`)
	Columns(Vec<String>),
	/// Specify a named constraint (e.g., `ON CONFLICT ON CONSTRAINT users_email_key`)
	/// Note: Only supported by PostgreSQL
	Constraint(String),
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

/// Fluent builder for ON CONFLICT clause with advanced options
///
/// This builder provides a more fluent API for constructing ON CONFLICT clauses,
/// with support for:
/// - Column-based and constraint-based conflict targets
/// - Conditional updates with WHERE clauses
/// - Explicit column assignments using EXCLUDED values
///
/// # Example
///
/// ```rust,ignore
/// // Basic upsert on email column
/// builder.on_conflict(OnConflictClause::columns(vec!["email"])
///     .do_update(vec!["name", "updated_at"]))
///
/// // Upsert with conditional WHERE clause
/// builder.on_conflict(OnConflictClause::columns(vec!["email"])
///     .do_update(vec!["name", "updated_at"])
///     .where_clause("users.updated_at < EXCLUDED.updated_at"))
///
/// // Upsert on named constraint (PostgreSQL only)
/// builder.on_conflict(OnConflictClause::constraint("users_email_key")
///     .do_update(vec!["name"]))
/// ```
#[derive(Debug, Clone)]
pub struct OnConflictClause {
	/// The conflict target (columns or constraint)
	target: Option<ConflictTarget>,
	/// The action to take on conflict
	action: OnConflictClauseAction,
	/// Optional WHERE clause for conditional updates (PostgreSQL/SQLite only)
	where_condition: Option<String>,
}

/// Action to take when a conflict occurs
#[derive(Debug, Clone)]
pub enum OnConflictClauseAction {
	/// Do nothing on conflict
	DoNothing,
	/// Update specified columns on conflict
	DoUpdate {
		/// Columns to update on conflict
		update_columns: Vec<String>,
	},
}

impl OnConflictClause {
	/// Create a new ON CONFLICT clause targeting specific columns
	///
	/// # Arguments
	///
	/// * `columns` - Columns that form the conflict target
	///
	/// # Example
	///
	/// ```rust,ignore
	/// OnConflictClause::columns(vec!["email", "tenant_id"])
	///     .do_update(vec!["name"])
	/// ```
	pub fn columns(columns: Vec<impl Into<String>>) -> Self {
		Self {
			target: Some(ConflictTarget::Columns(
				columns.into_iter().map(Into::into).collect(),
			)),
			action: OnConflictClauseAction::DoNothing,
			where_condition: None,
		}
	}

	/// Create a new ON CONFLICT clause targeting a named constraint
	///
	/// Note: This is only supported by PostgreSQL
	///
	/// # Arguments
	///
	/// * `constraint_name` - Name of the constraint to target
	///
	/// # Example
	///
	/// ```rust,ignore
	/// OnConflictClause::constraint("users_email_key")
	///     .do_update(vec!["name"])
	/// ```
	pub fn constraint(constraint_name: impl Into<String>) -> Self {
		Self {
			target: Some(ConflictTarget::Constraint(constraint_name.into())),
			action: OnConflictClauseAction::DoNothing,
			where_condition: None,
		}
	}

	/// Create a new ON CONFLICT clause with no specific target
	///
	/// This matches any unique constraint violation. Note that for SQLite
	/// with DO UPDATE, a target is required.
	///
	/// # Example
	///
	/// ```rust,ignore
	/// OnConflictClause::any()
	///     .do_nothing()
	/// ```
	pub fn any() -> Self {
		Self {
			target: None,
			action: OnConflictClauseAction::DoNothing,
			where_condition: None,
		}
	}

	/// Set the action to DO NOTHING on conflict
	///
	/// # Example
	///
	/// ```rust,ignore
	/// OnConflictClause::columns(vec!["email"])
	///     .do_nothing()
	/// ```
	pub fn do_nothing(mut self) -> Self {
		self.action = OnConflictClauseAction::DoNothing;
		self
	}

	/// Set the action to DO UPDATE with specified columns
	///
	/// The updated values will be taken from the EXCLUDED pseudo-table
	/// (or VALUES() function for MySQL).
	///
	/// # Arguments
	///
	/// * `columns` - Columns to update when conflict occurs
	///
	/// # Example
	///
	/// ```rust,ignore
	/// OnConflictClause::columns(vec!["email"])
	///     .do_update(vec!["name", "updated_at"])
	/// ```
	pub fn do_update(mut self, columns: Vec<impl Into<String>>) -> Self {
		self.action = OnConflictClauseAction::DoUpdate {
			update_columns: columns.into_iter().map(Into::into).collect(),
		};
		self
	}

	/// Add a WHERE clause for conditional updates
	///
	/// The WHERE clause is evaluated before the update is performed.
	/// Only rows matching the condition will be updated.
	///
	/// Note: Only supported by PostgreSQL and SQLite. MySQL does not support
	/// conditional updates in ON DUPLICATE KEY UPDATE.
	///
	/// # Arguments
	///
	/// * `condition` - SQL condition expression
	///
	/// # Example
	///
	/// ```rust,ignore
	/// // Only update if the new data is newer
	/// OnConflictClause::columns(vec!["email"])
	///     .do_update(vec!["name", "updated_at"])
	///     .where_clause("users.updated_at < EXCLUDED.updated_at")
	///
	/// // Only update if version is greater
	/// OnConflictClause::columns(vec!["id"])
	///     .do_update(vec!["data", "version"])
	///     .where_clause("users.version < EXCLUDED.version")
	/// ```
	pub fn where_clause(mut self, condition: impl Into<String>) -> Self {
		self.where_condition = Some(condition.into());
		self
	}
}

/// INSERT query builder
pub struct InsertBuilder {
	backend: Arc<dyn DatabaseBackend>,
	table: String,
	columns: Vec<String>,
	values: Vec<QueryValue>,
	returning: Option<Vec<String>>,
	on_conflict: Option<OnConflictAction>,
	on_conflict_clause: Option<OnConflictClause>,
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
			on_conflict_clause: None,
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

	/// Set ON CONFLICT behavior using the fluent `OnConflictClause` builder
	///
	/// This method provides a more flexible API for specifying conflict handling,
	/// including support for:
	/// - Column-based conflict targets
	/// - Constraint-based conflict targets (PostgreSQL only)
	/// - Conditional updates with WHERE clauses
	///
	/// # Arguments
	///
	/// * `clause` - The ON CONFLICT clause configuration
	///
	/// # Example
	///
	/// ```rust,ignore
	/// // Basic upsert on email column
	/// builder.on_conflict(OnConflictClause::columns(vec!["email"])
	///     .do_update(vec!["name", "updated_at"]))
	///
	/// // Upsert with conditional WHERE clause (only update if newer)
	/// builder.on_conflict(OnConflictClause::columns(vec!["email"])
	///     .do_update(vec!["name", "updated_at"])
	///     .where_clause("users.updated_at < EXCLUDED.updated_at"))
	///
	/// // Upsert on named constraint (PostgreSQL only)
	/// builder.on_conflict(OnConflictClause::constraint("users_email_key")
	///     .do_update(vec!["name"]))
	///
	/// // Do nothing on any conflict
	/// builder.on_conflict(OnConflictClause::any()
	///     .do_nothing())
	/// ```
	pub fn on_conflict(mut self, clause: OnConflictClause) -> Self {
		self.on_conflict_clause = Some(clause);
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
		// Prefer the new OnConflictClause over the legacy OnConflictAction
		if let Some(ref clause) = self.on_conflict_clause {
			sql = self.apply_new_on_conflict_clause(sql, clause);
		} else if let Some(ref on_conflict) = self.on_conflict {
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

	/// Apply new OnConflictClause to SQL string based on database type
	///
	/// This method handles the enhanced ON CONFLICT clause with support for:
	/// - Column-based conflict targets
	/// - Constraint-based conflict targets (PostgreSQL only)
	/// - WHERE clauses for conditional updates
	fn apply_new_on_conflict_clause(&self, mut sql: String, clause: &OnConflictClause) -> String {
		use super::types::DatabaseType;

		match self.backend.database_type() {
			DatabaseType::Postgres => {
				// Build conflict target
				let target_str = match &clause.target {
					Some(ConflictTarget::Columns(cols)) => {
						format!("({})", cols.join(", "))
					}
					Some(ConflictTarget::Constraint(name)) => {
						format!("ON CONSTRAINT {}", name)
					}
					None => String::new(),
				};

				match &clause.action {
					OnConflictClauseAction::DoNothing => {
						if target_str.is_empty() {
							sql.push_str(" ON CONFLICT DO NOTHING");
						} else {
							sql.push_str(&format!(" ON CONFLICT {} DO NOTHING", target_str));
						}
					}
					OnConflictClauseAction::DoUpdate { update_columns } => {
						let update_str = update_columns
							.iter()
							.map(|col| format!("{} = EXCLUDED.{}", col, col))
							.collect::<Vec<_>>()
							.join(", ");

						let mut clause_str =
							format!(" ON CONFLICT {} DO UPDATE SET {}", target_str, update_str);

						// Add WHERE clause if specified
						if let Some(ref where_cond) = clause.where_condition {
							clause_str.push_str(&format!(" WHERE {}", where_cond));
						}

						sql.push_str(&clause_str);
					}
				}
			}
			DatabaseType::Mysql => {
				// MySQL does not support conflict targets or WHERE clauses
				// Warn if constraint-based target is used
				if let Some(ConflictTarget::Constraint(_)) = &clause.target {
					// MySQL doesn't support ON CONFLICT ON CONSTRAINT
					// Fall back to standard MySQL behavior
				}

				match &clause.action {
					OnConflictClauseAction::DoNothing => {
						sql = sql.replacen("INSERT", "INSERT IGNORE", 1);
					}
					OnConflictClauseAction::DoUpdate { update_columns } => {
						let update_str = update_columns
							.iter()
							.map(|col| format!("{} = VALUES({})", col, col))
							.collect::<Vec<_>>()
							.join(", ");

						sql.push_str(&format!(" ON DUPLICATE KEY UPDATE {}", update_str));
						// Note: MySQL does not support WHERE clause in ON DUPLICATE KEY UPDATE
					}
				}
			}
			DatabaseType::Sqlite => {
				// SQLite uses similar syntax to PostgreSQL but with lowercase 'excluded'
				match &clause.action {
					OnConflictClauseAction::DoNothing => {
						sql = sql.replacen("INSERT", "INSERT OR IGNORE", 1);
					}
					OnConflictClauseAction::DoUpdate { update_columns } => {
						// SQLite requires conflict columns for DO UPDATE
						let conflict_str = match &clause.target {
							Some(ConflictTarget::Columns(cols)) => {
								if cols.is_empty() {
									panic!(
										"SQLite ON CONFLICT requires non-empty conflict_columns for DO UPDATE"
									);
								}
								format!("({})", cols.join(", "))
							}
							Some(ConflictTarget::Constraint(_)) => {
								// SQLite doesn't support ON CONSTRAINT syntax
								panic!("SQLite does not support ON CONFLICT ON CONSTRAINT syntax");
							}
							None => {
								// SQLite requires conflict target for DO UPDATE
								return sql;
							}
						};

						if update_columns.is_empty() {
							panic!(
								"update_columns cannot be empty for OnConflictClauseAction::DoUpdate"
							);
						}

						let update_str = update_columns
							.iter()
							.map(|col| format!("{} = excluded.{}", col, col)) // lowercase 'excluded'
							.collect::<Vec<_>>()
							.join(", ");

						let mut clause_str =
							format!(" ON CONFLICT {} DO UPDATE SET {}", conflict_str, update_str);

						// Add WHERE clause if specified
						if let Some(ref where_cond) = clause.where_condition {
							clause_str.push_str(&format!(" WHERE {}", where_cond));
						}

						sql.push_str(&clause_str);
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

	/// Convert to INSERT FROM SELECT builder
	///
	/// This method is mutually exclusive with `value()`. When `from_select()` is
	/// called, all previously added values are discarded and the SELECT statement
	/// is used as the source of data.
	///
	/// # Arguments
	///
	/// * `columns` - Columns to insert into
	/// * `select_stmt` - The SELECT statement to use as data source
	///
	/// # Example
	///
	/// ```rust,ignore
	/// let select = Query::select()
	///     .columns([Alias::new("id"), Alias::new("name")])
	///     .from(Alias::new("source_table"))
	///     .to_owned();
	///
	/// builder.from_select(vec!["id", "name"], select)
	/// ```
	pub fn from_select(
		self,
		columns: Vec<&str>,
		select_stmt: SelectStatement,
	) -> InsertFromSelectBuilder {
		InsertFromSelectBuilder::new(self.backend, &self.table, columns, select_stmt)
			.with_returning(self.returning)
			.with_on_conflict(self.on_conflict)
	}
}

/// INSERT FROM SELECT query builder
///
/// Builds INSERT INTO ... SELECT statements for inserting data from a subquery.
///
/// # Example
///
/// ```rust,ignore
/// use sea_query::{Alias, Query};
///
/// let select = Query::select()
///     .columns([Alias::new("id"), Alias::new("name")])
///     .from(Alias::new("source_table"))
///     .to_owned();
///
/// let builder = InsertFromSelectBuilder::new(
///     backend,
///     "target_table",
///     vec!["id", "name"],
///     select,
/// );
///
/// let (sql, _) = builder.build();
/// // Generates: INSERT INTO "target_table" ("id", "name") SELECT "id", "name" FROM "source_table"
/// ```
pub struct InsertFromSelectBuilder {
	backend: Arc<dyn DatabaseBackend>,
	table: String,
	columns: Vec<String>,
	select_stmt: SelectStatement,
	returning: Option<Vec<String>>,
	on_conflict: Option<OnConflictAction>,
}

impl InsertFromSelectBuilder {
	pub fn new(
		backend: Arc<dyn DatabaseBackend>,
		table: impl Into<String>,
		columns: Vec<&str>,
		select_stmt: SelectStatement,
	) -> Self {
		Self {
			backend,
			table: table.into(),
			columns: columns.iter().map(|s| (*s).to_owned()).collect(),
			select_stmt,
			returning: None,
			on_conflict: None,
		}
	}

	fn with_returning(mut self, returning: Option<Vec<String>>) -> Self {
		self.returning = returning;
		self
	}

	fn with_on_conflict(mut self, on_conflict: Option<OnConflictAction>) -> Self {
		self.on_conflict = on_conflict;
		self
	}

	pub fn returning(mut self, columns: Vec<&str>) -> Self {
		if self.backend.supports_returning() {
			self.returning = Some(columns.iter().map(|s| (*s).to_owned()).collect());
		}
		self
	}

	pub fn on_conflict_do_nothing(mut self, conflict_columns: Option<Vec<String>>) -> Self {
		self.on_conflict = Some(OnConflictAction::DoNothing { conflict_columns });
		self
	}

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

		let column_refs: Vec<Alias> = self.columns.iter().map(Alias::new).collect();
		stmt.columns(column_refs);

		stmt.select_from(self.select_stmt.clone())
			.expect("Failed to set SELECT statement for INSERT");

		if let Some(ref cols) = self.returning {
			for col in cols {
				stmt.returning(Query::returning().column(Alias::new(col)));
			}
		}

		let mut sql = match self.backend.database_type() {
			DatabaseType::Postgres => stmt.to_string(PostgresQueryBuilder),
			DatabaseType::Mysql => stmt.to_string(MysqlQueryBuilder),
			DatabaseType::Sqlite => stmt.to_string(SqliteQueryBuilder),
		};

		if let Some(ref on_conflict) = self.on_conflict {
			sql = self.apply_on_conflict_clause(sql, on_conflict);
		}

		(sql, Vec::new())
	}

	fn apply_on_conflict_clause(&self, mut sql: String, action: &OnConflictAction) -> String {
		use super::types::DatabaseType;

		match self.backend.database_type() {
			DatabaseType::Postgres => match action {
				OnConflictAction::DoNothing { conflict_columns } => {
					if let Some(cols) = conflict_columns {
						sql.push_str(&format!(" ON CONFLICT ({}) DO NOTHING", cols.join(", ")));
					} else {
						sql.push_str(" ON CONFLICT DO NOTHING");
					}
				}
				OnConflictAction::DoUpdate {
					conflict_columns,
					update_columns,
				} => {
					let conflict_str = conflict_columns
						.as_ref()
						.map(|cols| format!("({})", cols.join(", ")))
						.unwrap_or_default();
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
			DatabaseType::Mysql => match action {
				OnConflictAction::DoNothing { .. } => {
					sql = sql.replacen("INSERT", "INSERT IGNORE", 1);
				}
				OnConflictAction::DoUpdate {
					update_columns, ..
				} => {
					let update_str = update_columns
						.iter()
						.map(|col| format!("{} = VALUES({})", col, col))
						.collect::<Vec<_>>()
						.join(", ");
					sql.push_str(&format!(" ON DUPLICATE KEY UPDATE {}", update_str));
				}
			},
			DatabaseType::Sqlite => match action {
				OnConflictAction::DoNothing { .. } => {
					sql = sql.replacen("INSERT", "INSERT OR IGNORE", 1);
				}
				OnConflictAction::DoUpdate {
					conflict_columns,
					update_columns,
				} => {
					let conflict_str = match conflict_columns {
						Some(cols) if !cols.is_empty() => format!("({})", cols.join(", ")),
						_ => return sql,
					};
					let update_str = update_columns
						.iter()
						.map(|col| format!("{} = excluded.{}", col, col))
						.collect::<Vec<_>>()
						.join(", ");
					sql.push_str(&format!(
						" ON CONFLICT {} DO UPDATE SET {}",
						conflict_str, update_str
					));
				}
			},
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

	// Mock backends for different database types
	struct MockMysqlBackend;

	#[async_trait::async_trait]
	impl DatabaseBackend for MockMysqlBackend {
		fn database_type(&self) -> DatabaseType {
			DatabaseType::Mysql
		}
		fn placeholder(&self, index: usize) -> String {
			format!("?{}", index)
		}
		fn supports_returning(&self) -> bool {
			false
		}
		fn supports_on_conflict(&self) -> bool {
			false
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

	struct MockSqliteBackend;

	#[async_trait::async_trait]
	impl DatabaseBackend for MockSqliteBackend {
		fn database_type(&self) -> DatabaseType {
			DatabaseType::Sqlite
		}
		fn placeholder(&self, index: usize) -> String {
			format!("?{}", index)
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

	// Tests for OnConflictClause (new fluent API)

	// ==========================================
	// PostgreSQL Tests - Exact SQL Verification
	// ==========================================

	#[test]
	fn test_on_conflict_clause_columns_do_nothing_postgres_exact_sql() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(OnConflictClause::columns(vec!["email"]).do_nothing());
		let (sql, params) = builder.build();

		// Assert - verify exact SQL structure
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\") VALUES ('test@example.com') ON CONFLICT (email) DO NOTHING"
		);
		assert_eq!(params.len(), 1);
		assert!(matches!(&params[0], QueryValue::String(s) if s == "test@example.com"));
	}

	#[test]
	fn test_on_conflict_clause_columns_do_update_postgres_exact_sql() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.value("name", QueryValue::String("Test User".to_string()))
			.on_conflict(
				OnConflictClause::columns(vec!["email"]).do_update(vec!["name", "updated_at"]),
			);
		let (sql, params) = builder.build();

		// Assert - verify exact SQL structure with EXCLUDED references
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\", \"name\") VALUES ('test@example.com', 'Test User') ON CONFLICT (email) DO UPDATE SET name = EXCLUDED.name, updated_at = EXCLUDED.updated_at"
		);
		assert_eq!(params.len(), 2);
	}

	#[test]
	fn test_on_conflict_clause_with_where_postgres_exact_sql() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.value("version", QueryValue::Int(2))
			.on_conflict(
				OnConflictClause::columns(vec!["email"])
					.do_update(vec!["version"])
					.where_clause("users.version < EXCLUDED.version"),
			);
		let (sql, params) = builder.build();

		// Assert - verify WHERE clause is appended correctly
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\", \"version\") VALUES ('test@example.com', 2) ON CONFLICT (email) DO UPDATE SET version = EXCLUDED.version WHERE users.version < EXCLUDED.version"
		);
		assert_eq!(params.len(), 2);
	}

	#[test]
	fn test_on_conflict_clause_constraint_postgres_exact_sql() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(OnConflictClause::constraint("users_email_key").do_update(vec!["name"]));
		let (sql, _) = builder.build();

		// Assert - verify ON CONSTRAINT syntax
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\") VALUES ('test@example.com') ON CONFLICT ON CONSTRAINT users_email_key DO UPDATE SET name = EXCLUDED.name"
		);
	}

	#[test]
	fn test_on_conflict_clause_any_do_nothing_postgres_exact_sql() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(OnConflictClause::any().do_nothing());
		let (sql, _) = builder.build();

		// Assert - verify no conflict target specified
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\") VALUES ('test@example.com') ON CONFLICT DO NOTHING"
		);
	}

	#[test]
	fn test_on_conflict_clause_multiple_columns_postgres_exact_sql() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("tenant_id", QueryValue::Int(1))
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(
				OnConflictClause::columns(vec!["tenant_id", "email"]).do_update(vec!["name"]),
			);
		let (sql, _) = builder.build();

		// Assert - verify multiple conflict columns
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"tenant_id\", \"email\") VALUES (1, 'test@example.com') ON CONFLICT (tenant_id, email) DO UPDATE SET name = EXCLUDED.name"
		);
	}

	#[test]
	fn test_on_conflict_clause_multiple_update_columns_postgres() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("id", QueryValue::Int(1))
			.on_conflict(OnConflictClause::columns(vec!["id"]).do_update(vec![
				"name",
				"email",
				"updated_at",
				"version",
			]));
		let (sql, _) = builder.build();

		// Assert - verify all update columns are included
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"id\") VALUES (1) ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, email = EXCLUDED.email, updated_at = EXCLUDED.updated_at, version = EXCLUDED.version"
		);
	}

	// ==========================================
	// MySQL Tests - Exact SQL Verification
	// ==========================================

	#[test]
	fn test_on_conflict_clause_do_nothing_mysql_exact_sql() {
		// Arrange
		let backend = Arc::new(MockMysqlBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(OnConflictClause::columns(vec!["email"]).do_nothing());
		let (sql, _) = builder.build();

		// Assert - MySQL uses INSERT IGNORE syntax
		assert_eq!(
			sql,
			"INSERT IGNORE INTO `users` (`email`) VALUES ('test@example.com')"
		);
	}

	#[test]
	fn test_on_conflict_clause_do_update_mysql_exact_sql() {
		// Arrange
		let backend = Arc::new(MockMysqlBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(OnConflictClause::columns(vec!["email"]).do_update(vec!["name"]));
		let (sql, _) = builder.build();

		// Assert - MySQL uses ON DUPLICATE KEY UPDATE with VALUES() function
		assert_eq!(
			sql,
			"INSERT INTO `users` (`email`) VALUES ('test@example.com') ON DUPLICATE KEY UPDATE name = VALUES(name)"
		);
	}

	#[test]
	fn test_on_conflict_clause_do_update_multiple_columns_mysql() {
		// Arrange
		let backend = Arc::new(MockMysqlBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("id", QueryValue::Int(1))
			.on_conflict(OnConflictClause::columns(vec!["id"]).do_update(vec![
				"name",
				"email",
				"updated_at",
			]));
		let (sql, _) = builder.build();

		// Assert - verify multiple update columns with VALUES() syntax
		assert_eq!(
			sql,
			"INSERT INTO `users` (`id`) VALUES (1) ON DUPLICATE KEY UPDATE name = VALUES(name), email = VALUES(email), updated_at = VALUES(updated_at)"
		);
	}

	#[test]
	fn test_on_conflict_clause_where_ignored_mysql() {
		// Arrange
		let backend = Arc::new(MockMysqlBackend);

		// Act - MySQL does not support WHERE clause, but should not error
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(
				OnConflictClause::columns(vec!["email"])
					.do_update(vec!["name"])
					.where_clause("users.version < VALUES(version)"),
			);
		let (sql, _) = builder.build();

		// Assert - WHERE clause is ignored for MySQL
		assert_eq!(
			sql,
			"INSERT INTO `users` (`email`) VALUES ('test@example.com') ON DUPLICATE KEY UPDATE name = VALUES(name)"
		);
		assert!(!sql.contains("WHERE"));
	}

	// ==========================================
	// SQLite Tests - Exact SQL Verification
	// ==========================================

	#[test]
	fn test_on_conflict_clause_do_nothing_sqlite_exact_sql() {
		// Arrange
		let backend = Arc::new(MockSqliteBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(OnConflictClause::columns(vec!["email"]).do_nothing());
		let (sql, _) = builder.build();

		// Assert - SQLite uses INSERT OR IGNORE syntax
		assert_eq!(
			sql,
			"INSERT OR IGNORE INTO \"users\" (\"email\") VALUES ('test@example.com')"
		);
	}

	#[test]
	fn test_on_conflict_clause_do_update_sqlite_exact_sql() {
		// Arrange
		let backend = Arc::new(MockSqliteBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(OnConflictClause::columns(vec!["email"]).do_update(vec!["name"]));
		let (sql, _) = builder.build();

		// Assert - SQLite uses lowercase 'excluded' pseudo-table
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\") VALUES ('test@example.com') ON CONFLICT (email) DO UPDATE SET name = excluded.name"
		);
	}

	#[test]
	fn test_on_conflict_clause_with_where_sqlite_exact_sql() {
		// Arrange
		let backend = Arc::new(MockSqliteBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(
				OnConflictClause::columns(vec!["email"])
					.do_update(vec!["version"])
					.where_clause("users.version < excluded.version"),
			);
		let (sql, _) = builder.build();

		// Assert - SQLite supports WHERE clause
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\") VALUES ('test@example.com') ON CONFLICT (email) DO UPDATE SET version = excluded.version WHERE users.version < excluded.version"
		);
	}

	#[test]
	fn test_on_conflict_clause_multiple_columns_sqlite() {
		// Arrange
		let backend = Arc::new(MockSqliteBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("tenant_id", QueryValue::Int(1))
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(
				OnConflictClause::columns(vec!["tenant_id", "email"]).do_update(vec!["name"]),
			);
		let (sql, _) = builder.build();

		// Assert - verify multiple conflict columns
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"tenant_id\", \"email\") VALUES (1, 'test@example.com') ON CONFLICT (tenant_id, email) DO UPDATE SET name = excluded.name"
		);
	}

	#[test]
	fn test_on_conflict_clause_any_do_nothing_sqlite() {
		// Arrange
		let backend = Arc::new(MockSqliteBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(OnConflictClause::any().do_nothing());
		let (sql, _) = builder.build();

		// Assert - SQLite uses INSERT OR IGNORE even with any() target
		assert_eq!(
			sql,
			"INSERT OR IGNORE INTO \"users\" (\"email\") VALUES ('test@example.com')"
		);
	}

	// ==========================================
	// Legacy API Tests - Backwards Compatibility
	// ==========================================

	#[test]
	fn test_legacy_on_conflict_do_nothing_still_works() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act - using legacy API
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict_do_nothing(Some(vec!["email".to_string()]));
		let (sql, _) = builder.build();

		// Assert - legacy API should still work
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\") VALUES ('test@example.com') ON CONFLICT (email) DO NOTHING"
		);
	}

	#[test]
	fn test_legacy_on_conflict_do_update_still_works() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act - using legacy API
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict_do_update(
				Some(vec!["email".to_string()]),
				vec!["name".to_string(), "updated_at".to_string()],
			);
		let (sql, _) = builder.build();

		// Assert - legacy API should still work
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\") VALUES ('test@example.com') ON CONFLICT (email) DO UPDATE SET name = EXCLUDED.name, updated_at = EXCLUDED.updated_at"
		);
	}

	#[test]
	fn test_new_api_takes_precedence_over_legacy() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act - both APIs used, new should take precedence
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict_do_nothing(Some(vec!["email".to_string()])) // legacy
			.on_conflict(OnConflictClause::columns(vec!["email"]).do_update(vec!["name"])); // new
		let (sql, _) = builder.build();

		// Assert - new API should be used (DO UPDATE, not DO NOTHING)
		assert!(sql.contains("DO UPDATE SET"));
		assert!(!sql.contains("DO NOTHING"));
	}

	// ==========================================
	// Edge Cases and Error Conditions
	// ==========================================

	#[test]
	fn test_on_conflict_clause_single_column_single_update() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("id", QueryValue::Int(1))
			.on_conflict(OnConflictClause::columns(vec!["id"]).do_update(vec!["name"]));
		let (sql, _) = builder.build();

		// Assert
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"id\") VALUES (1) ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name"
		);
	}

	#[test]
	fn test_on_conflict_clause_with_returning() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.returning(vec!["id", "created_at"])
			.on_conflict(OnConflictClause::columns(vec!["email"]).do_update(vec!["name"]));
		let (sql, _) = builder.build();

		// Assert - RETURNING should come before ON CONFLICT in SeaQuery output
		assert!(sql.contains("RETURNING"));
		assert!(sql.contains("ON CONFLICT"));
	}

	#[test]
	fn test_on_conflict_clause_with_null_value() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.value("name", QueryValue::Null)
			.on_conflict(OnConflictClause::columns(vec!["email"]).do_update(vec!["name"]));
		let (sql, params) = builder.build();

		// Assert
		assert!(sql.contains("ON CONFLICT (email) DO UPDATE SET"));
		assert_eq!(params.len(), 2);
		assert!(matches!(params[1], QueryValue::Null));
	}

	#[test]
	fn test_on_conflict_clause_with_integer_values() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act
		let builder = InsertBuilder::new(backend, "counters")
			.value("key", QueryValue::String("visits".to_string()))
			.value("count", QueryValue::Int(1))
			.on_conflict(OnConflictClause::columns(vec!["key"]).do_update(vec!["count"]));
		let (sql, params) = builder.build();

		// Assert
		assert_eq!(
			sql,
			"INSERT INTO \"counters\" (\"key\", \"count\") VALUES ('visits', 1) ON CONFLICT (key) DO UPDATE SET count = EXCLUDED.count"
		);
		assert_eq!(params.len(), 2);
		assert!(matches!(params[1], QueryValue::Int(1)));
	}

	#[test]
	fn test_on_conflict_clause_complex_where_condition() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act
		let builder = InsertBuilder::new(backend, "documents")
			.value("id", QueryValue::Int(1))
			.on_conflict(
				OnConflictClause::columns(vec!["id"])
					.do_update(vec!["content", "version"])
					.where_clause(
						"documents.version < EXCLUDED.version AND documents.locked = false",
					),
			);
		let (sql, _) = builder.build();

		// Assert - complex WHERE with AND condition
		assert_eq!(
			sql,
			"INSERT INTO \"documents\" (\"id\") VALUES (1) ON CONFLICT (id) DO UPDATE SET content = EXCLUDED.content, version = EXCLUDED.version WHERE documents.version < EXCLUDED.version AND documents.locked = false"
		);
	}

	// ==========================================
	// Fluent API Chain Tests
	// ==========================================

	#[test]
	fn test_fluent_api_do_nothing_then_do_update_uses_last() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act - chain do_nothing then do_update
		let clause = OnConflictClause::columns(vec!["email"])
			.do_nothing()
			.do_update(vec!["name"]);
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(clause);
		let (sql, _) = builder.build();

		// Assert - last action (do_update) should be used
		assert!(sql.contains("DO UPDATE SET"));
		assert!(!sql.contains("DO NOTHING"));
	}

	#[test]
	fn test_fluent_api_do_update_then_do_nothing_uses_last() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act - chain do_update then do_nothing
		let clause = OnConflictClause::columns(vec!["email"])
			.do_update(vec!["name"])
			.do_nothing();
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(clause);
		let (sql, _) = builder.build();

		// Assert - last action (do_nothing) should be used
		assert!(sql.contains("DO NOTHING"));
		assert!(!sql.contains("DO UPDATE"));
	}

	#[test]
	fn test_conflict_target_types() {
		// Act - test ConflictTarget::Columns
		let columns_clause = OnConflictClause::columns(vec!["a", "b"]);
		assert!(matches!(
			columns_clause.target,
			Some(ConflictTarget::Columns(ref cols)) if cols == &vec!["a".to_string(), "b".to_string()]
		));

		// Act - test ConflictTarget::Constraint
		let constraint_clause = OnConflictClause::constraint("my_constraint");
		assert!(matches!(
			constraint_clause.target,
			Some(ConflictTarget::Constraint(ref name)) if name == "my_constraint"
		));

		// Act - test no target (any)
		let any_clause = OnConflictClause::any();
		assert!(any_clause.target.is_none());

		// Verify they all build correctly with separate backends
		let backend1: Arc<dyn DatabaseBackend> = Arc::new(MockBackend);
		let builder1 = InsertBuilder::new(backend1, "t")
			.value("x", QueryValue::Int(1))
			.on_conflict(columns_clause.do_nothing());
		let (sql1, _) = builder1.build();
		assert!(sql1.contains("ON CONFLICT (a, b) DO NOTHING"));

		let backend2: Arc<dyn DatabaseBackend> = Arc::new(MockBackend);
		let builder2 = InsertBuilder::new(backend2, "t")
			.value("x", QueryValue::Int(1))
			.on_conflict(constraint_clause.do_nothing());
		let (sql2, _) = builder2.build();
		assert!(sql2.contains("ON CONFLICT ON CONSTRAINT my_constraint DO NOTHING"));

		let backend3: Arc<dyn DatabaseBackend> = Arc::new(MockBackend);
		let builder3 = InsertBuilder::new(backend3, "t")
			.value("x", QueryValue::Int(1))
			.on_conflict(any_clause.do_nothing());
		let (sql3, _) = builder3.build();
		assert!(sql3.contains("ON CONFLICT DO NOTHING"));
	}

	// ==========================================
	// Parameter Preservation Tests
	// ==========================================

	#[test]
	fn test_parameters_preserved_with_on_conflict() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("id", QueryValue::Int(42))
			.value("name", QueryValue::String("John".to_string()))
			.value("active", QueryValue::Bool(true))
			.on_conflict(OnConflictClause::columns(vec!["id"]).do_update(vec!["name", "active"]));
		let (_, params) = builder.build();

		// Assert - all parameters should be preserved in order
		assert_eq!(params.len(), 3);
		assert!(matches!(params[0], QueryValue::Int(42)));
		assert!(matches!(&params[1], QueryValue::String(s) if s == "John"));
		assert!(matches!(params[2], QueryValue::Bool(true)));
	}

	// ==========================================
	// INSERT FROM SELECT Tests
	// ==========================================

	#[test]
	fn test_insert_from_select_basic_postgres() {
		// Arrange
		let backend = Arc::new(MockBackend);
		let select = Query::select()
			.column(Alias::new("id"))
			.column(Alias::new("name"))
			.from(Alias::new("source_table"))
			.to_owned();

		// Act
		let builder =
			InsertFromSelectBuilder::new(backend, "target_table", vec!["id", "name"], select);
		let (sql, params) = builder.build();

		// Assert
		assert_eq!(
			sql,
			"INSERT INTO \"target_table\" (\"id\", \"name\") SELECT \"id\", \"name\" FROM \"source_table\""
		);
		assert!(params.is_empty());
	}

	#[test]
	fn test_insert_from_select_with_where_clause() {
		// Arrange
		let backend = Arc::new(MockBackend);
		let select = Query::select()
			.column(Alias::new("id"))
			.column(Alias::new("name"))
			.from(Alias::new("users"))
			.and_where(Expr::col(Alias::new("status")).eq("inactive"))
			.to_owned();

		// Act
		let builder =
			InsertFromSelectBuilder::new(backend, "archived_users", vec!["id", "name"], select);
		let (sql, params) = builder.build();

		// Assert
		assert_eq!(
			sql,
			"INSERT INTO \"archived_users\" (\"id\", \"name\") SELECT \"id\", \"name\" FROM \"users\" WHERE \"status\" = 'inactive'"
		);
		assert!(params.is_empty());
	}

	#[test]
	fn test_insert_from_select_with_returning() {
		// Arrange
		let backend = Arc::new(MockBackend);
		let select = Query::select()
			.column(Alias::new("id"))
			.column(Alias::new("name"))
			.from(Alias::new("source_table"))
			.to_owned();

		// Act
		let builder =
			InsertFromSelectBuilder::new(backend, "target_table", vec!["id", "name"], select)
				.returning(vec!["id"]);
		let (sql, params) = builder.build();

		// Assert
		assert!(sql.contains("RETURNING"));
		assert!(sql.contains("\"id\""));
		assert!(params.is_empty());
	}

	#[test]
	fn test_insert_from_select_on_conflict_do_nothing_postgres() {
		// Arrange
		let backend = Arc::new(MockBackend);
		let select = Query::select()
			.column(Alias::new("id"))
			.column(Alias::new("name"))
			.from(Alias::new("source_table"))
			.to_owned();

		// Act
		let builder =
			InsertFromSelectBuilder::new(backend, "target_table", vec!["id", "name"], select)
				.on_conflict_do_nothing(Some(vec!["id".to_string()]));
		let (sql, params) = builder.build();

		// Assert
		assert!(sql.contains("ON CONFLICT (id) DO NOTHING"));
		assert!(params.is_empty());
	}

	#[test]
	fn test_insert_from_select_on_conflict_do_update_postgres() {
		// Arrange
		let backend = Arc::new(MockBackend);
		let select = Query::select()
			.column(Alias::new("id"))
			.column(Alias::new("name"))
			.from(Alias::new("source_table"))
			.to_owned();

		// Act
		let builder =
			InsertFromSelectBuilder::new(backend, "target_table", vec!["id", "name"], select)
				.on_conflict_do_update(Some(vec!["id".to_string()]), vec!["name".to_string()]);
		let (sql, params) = builder.build();

		// Assert
		assert!(sql.contains("ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name"));
		assert!(params.is_empty());
	}

	#[test]
	fn test_insert_from_select_mysql() {
		// Arrange
		let backend = Arc::new(MockMysqlBackend);
		let select = Query::select()
			.column(Alias::new("id"))
			.column(Alias::new("name"))
			.from(Alias::new("source_table"))
			.to_owned();

		// Act
		let builder =
			InsertFromSelectBuilder::new(backend, "target_table", vec!["id", "name"], select);
		let (sql, params) = builder.build();

		// Assert
		assert_eq!(
			sql,
			"INSERT INTO `target_table` (`id`, `name`) SELECT `id`, `name` FROM `source_table`"
		);
		assert!(params.is_empty());
	}

	#[test]
	fn test_insert_from_select_mysql_ignore() {
		// Arrange
		let backend = Arc::new(MockMysqlBackend);
		let select = Query::select()
			.column(Alias::new("id"))
			.column(Alias::new("name"))
			.from(Alias::new("source_table"))
			.to_owned();

		// Act
		let builder =
			InsertFromSelectBuilder::new(backend, "target_table", vec!["id", "name"], select)
				.on_conflict_do_nothing(None);
		let (sql, params) = builder.build();

		// Assert
		assert!(sql.starts_with("INSERT IGNORE INTO"));
		assert!(params.is_empty());
	}

	#[test]
	fn test_insert_from_select_sqlite() {
		// Arrange
		let backend = Arc::new(MockSqliteBackend);
		let select = Query::select()
			.column(Alias::new("id"))
			.column(Alias::new("name"))
			.from(Alias::new("source_table"))
			.to_owned();

		// Act
		let builder =
			InsertFromSelectBuilder::new(backend, "target_table", vec!["id", "name"], select);
		let (sql, params) = builder.build();

		// Assert
		assert_eq!(
			sql,
			"INSERT INTO \"target_table\" (\"id\", \"name\") SELECT \"id\", \"name\" FROM \"source_table\""
		);
		assert!(params.is_empty());
	}

	#[test]
	fn test_insert_from_select_sqlite_or_ignore() {
		// Arrange
		let backend = Arc::new(MockSqliteBackend);
		let select = Query::select()
			.column(Alias::new("id"))
			.column(Alias::new("name"))
			.from(Alias::new("source_table"))
			.to_owned();

		// Act
		let builder =
			InsertFromSelectBuilder::new(backend, "target_table", vec!["id", "name"], select)
				.on_conflict_do_nothing(None);
		let (sql, params) = builder.build();

		// Assert
		assert!(sql.starts_with("INSERT OR IGNORE INTO"));
		assert!(params.is_empty());
	}

	#[test]
	fn test_insert_builder_from_select_conversion() {
		// Arrange
		let backend = Arc::new(MockBackend);
		let select = Query::select()
			.column(Alias::new("id"))
			.column(Alias::new("name"))
			.from(Alias::new("source_table"))
			.to_owned();

		// Act
		let builder =
			InsertBuilder::new(backend, "target_table").from_select(vec!["id", "name"], select);
		let (sql, params) = builder.build();

		// Assert
		assert_eq!(
			sql,
			"INSERT INTO \"target_table\" (\"id\", \"name\") SELECT \"id\", \"name\" FROM \"source_table\""
		);
		assert!(params.is_empty());
	}

	#[test]
	fn test_insert_builder_from_select_preserves_returning() {
		// Arrange
		let backend = Arc::new(MockBackend);
		let select = Query::select()
			.column(Alias::new("id"))
			.column(Alias::new("name"))
			.from(Alias::new("source_table"))
			.to_owned();

		// Act
		let builder = InsertBuilder::new(backend, "target_table")
			.returning(vec!["id"])
			.from_select(vec!["id", "name"], select);
		let (sql, _) = builder.build();

		// Assert
		assert!(sql.contains("RETURNING"));
	}

	#[test]
	fn test_insert_builder_from_select_preserves_on_conflict() {
		// Arrange
		let backend = Arc::new(MockBackend);
		let select = Query::select()
			.column(Alias::new("id"))
			.column(Alias::new("name"))
			.from(Alias::new("source_table"))
			.to_owned();

		// Act
		let builder = InsertBuilder::new(backend, "target_table")
			.on_conflict_do_nothing(Some(vec!["id".to_string()]))
			.from_select(vec!["id", "name"], select);
		let (sql, _) = builder.build();

		// Assert
		assert!(sql.contains("ON CONFLICT (id) DO NOTHING"));
	}
}
