//! Query builder with dialect support

use std::sync::Arc;

use reinhardt_query::prelude::{
	Alias, ColumnRef, DeleteStatement, Expr, ExprTrait, InsertStatement, Query,
	QueryBuilder as RqQueryBuilder, SelectStatement, SimpleExpr, UpdateStatement, Value, Values,
};

use super::{
	backend::DatabaseBackend,
	error::{DatabaseError, DatabaseErrorKind, Result},
	types::{DatabaseType, QueryResult, QueryValue, Row},
};

/// Convert QueryValue to reinhardt-query Value
fn query_value_to_sea_value(qv: &QueryValue) -> Value {
	match qv {
		// BigInt(None) is used for generic NULL values across all dialects
		// (consistent with PostgreSQL, MySQL, SQLite backend implementations)
		QueryValue::Null => Value::BigInt(None),
		QueryValue::Bool(b) => Value::Bool(Some(*b)),
		QueryValue::Int32(i) => Value::Int(Some(*i)),
		QueryValue::Int(i) => Value::BigInt(Some(*i)),
		QueryValue::Uint(i) => Value::BigUnsigned(Some(*i)),
		QueryValue::Float(f) => Value::Double(Some(*f)),
		QueryValue::String(s) => Value::String(Some(Box::new(s.clone()))),
		QueryValue::Bytes(b) => Value::Bytes(Some(Box::new(b.clone()))),
		QueryValue::Timestamp(dt) => Value::ChronoDateTimeUtc(Some(Box::new(*dt))),
		QueryValue::NaiveTimestamp(dt) => Value::ChronoDateTime(Some(Box::new(*dt))),
		QueryValue::Uuid(u) => Value::Uuid(Some(Box::new(*u))),
		QueryValue::Json(value) => Value::Json(value.clone()),
		#[cfg(feature = "pgvector")]
		QueryValue::Vector(values) => Value::Vector(values.clone().map(Box::new)),
		QueryValue::StringArray(values) => Value::Array(
			reinhardt_query::value::ArrayType::String,
			Some(Box::new(
				values
					.iter()
					.cloned()
					.map(|value| Value::String(Some(Box::new(value))))
					.collect(),
			)),
		),
		QueryValue::IntArray(values) => Value::Array(
			reinhardt_query::value::ArrayType::Int,
			Some(Box::new(
				values
					.iter()
					.copied()
					.map(|value| Value::Int(Some(value)))
					.collect(),
			)),
		),
		QueryValue::BigIntArray(values) => Value::Array(
			reinhardt_query::value::ArrayType::BigInt,
			Some(Box::new(
				values
					.iter()
					.copied()
					.map(|value| Value::BigInt(Some(value)))
					.collect(),
			)),
		),
		QueryValue::BoolArray(values) => Value::Array(
			reinhardt_query::value::ArrayType::Bool,
			Some(Box::new(
				values
					.iter()
					.copied()
					.map(|value| Value::Bool(Some(value)))
					.collect(),
			)),
		),
		QueryValue::FloatArray(values) => Value::Array(
			reinhardt_query::value::ArrayType::Float,
			Some(Box::new(
				values
					.iter()
					.copied()
					.map(|value| Value::Float(Some(value)))
					.collect(),
			)),
		),
		QueryValue::DoubleArray(values) => Value::Array(
			reinhardt_query::value::ArrayType::Double,
			Some(Box::new(
				values
					.iter()
					.copied()
					.map(|value| Value::Double(Some(value)))
					.collect(),
			)),
		),
		QueryValue::UuidArray(values) => Value::Array(
			reinhardt_query::value::ArrayType::Uuid,
			Some(Box::new(
				values
					.iter()
					.copied()
					.map(|value| Value::Uuid(Some(Box::new(value))))
					.collect(),
			)),
		),
		QueryValue::NullableStringArray(values) => Value::Array(
			reinhardt_query::value::ArrayType::String,
			Some(Box::new(
				values
					.iter()
					.cloned()
					.map(|value| Value::String(value.map(Box::new)))
					.collect(),
			)),
		),
		QueryValue::NullableIntArray(values) => Value::Array(
			reinhardt_query::value::ArrayType::Int,
			Some(Box::new(values.iter().copied().map(Value::Int).collect())),
		),
		QueryValue::NullableBigIntArray(values) => Value::Array(
			reinhardt_query::value::ArrayType::BigInt,
			Some(Box::new(
				values.iter().copied().map(Value::BigInt).collect(),
			)),
		),
		QueryValue::NullableBoolArray(values) => Value::Array(
			reinhardt_query::value::ArrayType::Bool,
			Some(Box::new(values.iter().copied().map(Value::Bool).collect())),
		),
		QueryValue::NullableFloatArray(values) => Value::Array(
			reinhardt_query::value::ArrayType::Float,
			Some(Box::new(values.iter().copied().map(Value::Float).collect())),
		),
		QueryValue::NullableDoubleArray(values) => Value::Array(
			reinhardt_query::value::ArrayType::Double,
			Some(Box::new(
				values.iter().copied().map(Value::Double).collect(),
			)),
		),
		QueryValue::NullableUuidArray(values) => Value::Array(
			reinhardt_query::value::ArrayType::Uuid,
			Some(Box::new(
				values
					.iter()
					.copied()
					.map(|value| Value::Uuid(value.map(Box::new)))
					.collect(),
			)),
		),
		// NOW() is handled specially in build() methods, should not reach here
		QueryValue::Now => {
			panic!("QueryValue::Now should be handled in build() method, not converted to Value")
		}
	}
}

fn query_value_expression(value: &QueryValue) -> SimpleExpr {
	if matches!(value, QueryValue::Now) {
		Expr::current_timestamp().into_simple_expr()
	} else {
		Expr::val(query_value_to_sea_value(value)).into_simple_expr()
	}
}

fn native_builder_expression(value: &QueryValue, backend: DatabaseType) -> Result<SimpleExpr> {
	// Preserve the raw builder's JSON-text array storage outside PostgreSQL.
	if backend != DatabaseType::Postgres {
		super::types::validate_json_array(value)?;
	}
	let json = if backend != DatabaseType::Postgres {
		match value {
			QueryValue::StringArray(values) => Some(serde_json::to_string(values)),
			QueryValue::IntArray(values) => Some(serde_json::to_string(values)),
			QueryValue::BigIntArray(values) => Some(serde_json::to_string(values)),
			QueryValue::BoolArray(values) => Some(serde_json::to_string(values)),
			QueryValue::FloatArray(values) => Some(serde_json::to_string(values)),
			QueryValue::DoubleArray(values) => Some(serde_json::to_string(values)),
			QueryValue::UuidArray(values) => Some(serde_json::to_string(values)),
			QueryValue::NullableStringArray(values) => Some(serde_json::to_string(values)),
			QueryValue::NullableIntArray(values) => Some(serde_json::to_string(values)),
			QueryValue::NullableBigIntArray(values) => Some(serde_json::to_string(values)),
			QueryValue::NullableBoolArray(values) => Some(serde_json::to_string(values)),
			QueryValue::NullableFloatArray(values) => Some(serde_json::to_string(values)),
			QueryValue::NullableDoubleArray(values) => Some(serde_json::to_string(values)),
			QueryValue::NullableUuidArray(values) => Some(serde_json::to_string(values)),
			_ => None,
		}
	} else {
		None
	};
	if let Some(json) = json {
		let json = json.map_err(|error| {
			DatabaseError::new(
				DatabaseErrorKind::Query,
				format!("failed to encode builder array: {error}"),
			)
			.with_source(error)
		})?;
		return Ok(Expr::val(json).into_simple_expr());
	}
	Ok(query_value_expression(value))
}

enum BuilderPredicate {
	Equal(String, QueryValue),
	In(String, Vec<QueryValue>),
}

impl BuilderPredicate {
	fn expression_with(
		&self,
		convert: &impl Fn(&QueryValue) -> Result<SimpleExpr>,
	) -> Result<SimpleExpr> {
		match self {
			Self::Equal(column, value)
				if !matches!(value, QueryValue::Now)
					&& query_value_to_sea_value(value).is_null() =>
			{
				Ok(Expr::col(Alias::new(column)).is_null())
			}
			Self::Equal(column, value) => Ok(Expr::col(Alias::new(column)).eq(convert(value)?)),
			Self::In(column, values) => Ok(Expr::col(Alias::new(column))
				.is_in(values.iter().map(convert).collect::<Result<Vec<_>>>()?)),
		}
	}

	fn values(&self) -> &[QueryValue] {
		match self {
			Self::Equal(_, value) => std::slice::from_ref(value),
			Self::In(_, values) => values,
		}
	}
}

fn legacy_builder_parameters<'a>(values: impl Iterator<Item = &'a QueryValue>) -> Vec<QueryValue> {
	values
		.filter(|value| {
			!matches!(value, QueryValue::Now) && !query_value_to_sea_value(value).is_null()
		})
		.cloned()
		.collect()
}

enum NativeBuilderStatement {
	Insert(Box<InsertStatement>),
	Update(UpdateStatement),
	Select(Box<SelectStatement>),
	Delete(DeleteStatement),
}

fn native_builder_pair(
	backend: DatabaseType,
	statement: NativeBuilderStatement,
) -> Result<(String, Values)> {
	use reinhardt_query::prelude::{MySqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder};
	macro_rules! build {
		($statement:expr, $method:ident) => {
			match backend {
				DatabaseType::Postgres => PostgresQueryBuilder.$method($statement),
				DatabaseType::Mysql => MySqlQueryBuilder.$method($statement),
				DatabaseType::Sqlite => SqliteQueryBuilder.$method($statement),
			}
		};
	}
	match &statement {
		NativeBuilderStatement::Insert(stmt) => build!(stmt, build_insert_checked),
		NativeBuilderStatement::Update(stmt) => build!(stmt, build_update_checked),
		NativeBuilderStatement::Select(stmt) => build!(stmt, build_select_checked),
		NativeBuilderStatement::Delete(stmt) => build!(stmt, build_delete_checked),
	}
	.map_err(|error| DatabaseError::new(DatabaseErrorKind::Query, error.to_string()).into())
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
		/// Conflict columns (PostgreSQL and SQLite). SQLite 3.35.0+ accepts
		/// `None` to match any unique constraint.
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
	/// This matches any unique constraint violation. SQLite 3.35.0+ supports
	/// targetless `DO UPDATE`; PostgreSQL requires a target for `DO UPDATE`.
	/// See the [SQLite UPSERT documentation](https://www.sqlite.org/lang_upsert.html).
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::backends::query_builder::OnConflictClause;
	///
	/// let ignore = OnConflictClause::any().do_nothing();
	/// // Update on any unique constraint violation on SQLite 3.35.0+.
	/// let upsert = OnConflictClause::any().do_update(vec!["name"]);
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
	/// PostgreSQL and SQLite preserve the specified uniqueness target. With
	/// `any()`, any uniqueness conflict is ignored. SQLite still reports NOT NULL,
	/// CHECK, and foreign-key violations. MySQL uses `INSERT IGNORE`.
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
	/// # Safety
	///
	/// This method embeds the `condition` string directly into the generated SQL
	/// without any escaping or parameterization. The caller **must** ensure that
	/// the input is trusted and not derived from user-controlled data.
	///
	/// # SQL Injection Risk
	///
	/// Passing unsanitized user input to this method will result in a SQL injection
	/// vulnerability. Always use hardcoded or application-controlled expressions.
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

// The fluent configuration takes precedence over the legacy conflict API.
fn apply_insert_conflict(
	statement: &mut InsertStatement,
	backend: DatabaseType,
	legacy: Option<&OnConflictAction>,
	fluent: Option<&OnConflictClause>,
	insert_from_select: bool,
) -> Result<()> {
	use reinhardt_query::OnConflict;
	let legacy_target;
	let (target, update_columns, condition) = if let Some(clause) = fluent {
		let updates = match &clause.action {
			OnConflictClauseAction::DoNothing => None,
			OnConflictClauseAction::DoUpdate { update_columns } => Some(update_columns),
		};
		(
			clause.target.as_ref(),
			updates,
			clause.where_condition.as_ref(),
		)
	} else if let Some(action) = legacy {
		let (columns, updates) = match action {
			OnConflictAction::DoNothing { conflict_columns } => (conflict_columns, None),
			OnConflictAction::DoUpdate {
				conflict_columns,
				update_columns,
			} => (conflict_columns, Some(update_columns)),
		};
		legacy_target = columns
			.as_ref()
			.filter(|columns| {
				// The legacy SELECT API accepts an empty SQLite target as targetless.
				!insert_from_select || backend != DatabaseType::Sqlite || !columns.is_empty()
			})
			.map(|columns| ConflictTarget::Columns(columns.clone()));
		(legacy_target.as_ref(), updates, None)
	} else {
		return Ok(());
	};

	if matches!(target, Some(ConflictTarget::Constraint(_))) && backend != DatabaseType::Postgres {
		return Err(DatabaseError::new(
			DatabaseErrorKind::Unsupported,
			match backend {
				DatabaseType::Mysql => "MySQL does not support named conflict targets",
				DatabaseType::Sqlite => "SQLite does not support ON CONFLICT ON CONSTRAINT syntax",
				DatabaseType::Postgres => {
					unreachable!("PostgreSQL constraint targets are supported")
				}
			},
		)
		.into());
	}
	if condition.is_some() && backend == DatabaseType::Mysql {
		return Err(DatabaseError::new(
			DatabaseErrorKind::Unsupported,
			"MySQL does not support conditional ON DUPLICATE KEY UPDATE",
		)
		.into());
	}
	if condition.is_some() && update_columns.is_none() {
		return Err(DatabaseError::new(
			DatabaseErrorKind::Syntax,
			"ON CONFLICT DO NOTHING cannot have an update condition",
		)
		.into());
	}
	if update_columns.is_none() {
		match backend {
			DatabaseType::Mysql => {
				statement.mysql_ignore();
				return Ok(());
			}
			DatabaseType::Sqlite if fluent.is_none() => {
				statement.sqlite_or_ignore();
				return Ok(());
			}
			DatabaseType::Postgres | DatabaseType::Sqlite => {}
		}
	}
	if backend == DatabaseType::Sqlite
		&& matches!(target, Some(ConflictTarget::Columns(columns)) if columns.is_empty())
	{
		return Err(DatabaseError::new(
			DatabaseErrorKind::Syntax,
			if update_columns.is_some() {
				"SQLite ON CONFLICT requires non-empty conflict_columns for DO UPDATE"
			} else {
				"SQLite ON CONFLICT requires non-empty conflict_columns for DO NOTHING"
			},
		)
		.into());
	}
	if update_columns.is_some_and(|columns| columns.is_empty()) {
		return Err(DatabaseError::new(
			DatabaseErrorKind::Syntax,
			if fluent.is_some() {
				"update_columns cannot be empty for OnConflictClauseAction::DoUpdate"
			} else {
				"update_columns cannot be empty for OnConflictAction::DoUpdate"
			},
		)
		.into());
	}
	let mut conflict = match target {
		Some(ConflictTarget::Columns(columns)) => {
			OnConflict::columns(columns.iter().map(Alias::new))
		}
		Some(ConflictTarget::Constraint(name)) => OnConflict::constraint(Alias::new(name)),
		None => OnConflict::new(),
	};
	if let Some(columns) = update_columns {
		conflict = conflict.update_columns(columns.iter().map(Alias::new));
	}
	if let Some(condition) = condition {
		// This explicit public API accepts caller-owned SQL, never a framework template.
		conflict = conflict.action_and_where(Expr::cust(condition));
	}
	statement.on_conflict(conflict);
	Ok(())
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
	/// Creates a new instance.
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

	/// Performs the value operation.
	pub fn value(mut self, column: impl Into<String>, value: impl Into<QueryValue>) -> Self {
		self.columns.push(column.into());
		self.values.push(value.into());
		self
	}

	/// Performs the returning operation.
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
	/// * `conflict_columns` - Columns to check for conflict (PostgreSQL and SQLite).
	///   `None` matches any unique constraint on SQLite 3.35.0+.
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

	fn statement_with(
		&self,
		convert: impl Fn(&QueryValue) -> Result<SimpleExpr>,
	) -> Result<InsertStatement> {
		let mut statement = Query::insert()
			.into_table(Alias::new(&self.table))
			.to_owned();
		statement.columns(self.columns.iter().map(Alias::new));
		if !self.values.is_empty() {
			let values = self
				.values
				.iter()
				.map(convert)
				.collect::<Result<Vec<_>>>()?;
			statement.values_expr(values).map_err(|error| {
				DatabaseError::new(
					DatabaseErrorKind::Query,
					format!("failed to set insert values (column/value count mismatch): {error}"),
				)
			})?;
		}
		if let Some(columns) = &self.returning {
			statement.returning(columns.iter().map(Alias::new));
		}
		apply_insert_conflict(
			&mut statement,
			self.backend.database_type(),
			self.on_conflict.as_ref(),
			self.on_conflict_clause.as_ref(),
			false,
		)?;
		Ok(statement)
	}

	fn build_native(&self) -> Result<(String, Values)> {
		let backend = self.backend.database_type();
		native_builder_pair(
			backend,
			NativeBuilderStatement::Insert(Box::new(
				self.statement_with(|value| native_builder_expression(value, backend))?,
			)),
		)
	}

	/// Build SQL and legacy parameters for callers of the raw API.
	/// NULL and current-time expressions do not consume argument slots.
	pub fn build(&self) -> Result<(String, Vec<QueryValue>)> {
		let statement = self.statement_with(|value| Ok(query_value_expression(value)))?;
		let (sql, _) = native_builder_pair(
			self.backend.database_type(),
			NativeBuilderStatement::Insert(Box::new(statement)),
		)?;
		Ok((sql, legacy_builder_parameters(self.values.iter())))
	}

	/// Execute the checked typed statement and its exact renderer arguments.
	pub async fn execute(&self) -> Result<QueryResult> {
		self.backend
			.execute_generated(self.build_native()?, None)
			.await
	}

	/// Fetch a row from the checked typed statement and exact renderer arguments.
	pub async fn fetch_one(&self) -> Result<Row> {
		self.backend
			.fetch_one_generated(self.build_native()?, None)
			.await
	}

	/// Convert to INSERT FROM SELECT builder
	///
	/// This method is mutually exclusive with `value()`. When `from_select()` is
	/// called, all previously added values are discarded and the SELECT statement
	/// is used as the source of data. RETURNING and both legacy and fluent conflict
	/// settings are retained. A fluent `OnConflictClause` takes precedence over
	/// legacy conflict settings, as it does for a VALUES insert.
	///
	/// SQLite UPSERT sources are wrapped in a derived table with `WHERE TRUE` to
	/// disambiguate the conflict clause while preserving SELECT filters, ordering,
	/// limits, and compound queries.
	///
	/// # Arguments
	///
	/// * `columns` - Columns to insert into
	/// * `select_stmt` - The SELECT statement to use as data source
	///
	/// # Example
	///
	/// ```rust
	/// # #[cfg(feature = "sqlite")]
	/// # #[tokio::main]
	/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// use std::sync::Arc;
	/// use reinhardt_db::backends::{
	///     dialect::SqliteBackend,
	///     query_builder::{InsertBuilder, OnConflictClause},
	/// };
	/// use reinhardt_query::prelude::{Expr, Query};
	///
	/// let pool = sqlx::SqlitePool::connect("sqlite::memory:").await?;
	/// let backend = Arc::new(SqliteBackend::new(pool));
	/// let select = Query::select().expr(Expr::val(4_i64)).to_owned();
	/// let (sql, params) = InsertBuilder::new(backend, "users")
	///     .on_conflict(OnConflictClause::columns(vec!["id"]).do_update(vec!["id"]))
	///     .from_select(vec!["id"], select)
	///     .build();
	/// assert_eq!(sql, "INSERT INTO \"users\" (\"id\") SELECT 4 ON CONFLICT (\"id\") DO UPDATE SET \"id\" = EXCLUDED.\"id\"");
	/// assert!(params.is_empty());
	/// # Ok(())
	/// # }
	/// # #[cfg(not(feature = "sqlite"))]
	/// # fn main() {}
	/// ```
	pub fn from_select(
		self,
		columns: Vec<&str>,
		select_stmt: SelectStatement,
	) -> InsertFromSelectBuilder {
		InsertFromSelectBuilder::new(self.backend, &self.table, columns, select_stmt)
			.with_returning(self.returning)
			.with_on_conflict(self.on_conflict)
			.with_on_conflict_clause(self.on_conflict_clause)
	}
}

/// INSERT FROM SELECT query builder
///
/// Builds INSERT INTO ... SELECT statements for inserting data from a subquery.
///
/// # Example
///
/// ```rust,ignore
/// use reinhardt_query::prelude::{QueryStatementBuilder, Alias, Query};
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
	on_conflict_clause: Option<OnConflictClause>,
}

impl InsertFromSelectBuilder {
	/// Creates a new instance.
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
			on_conflict_clause: None,
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

	fn with_on_conflict_clause(mut self, clause: Option<OnConflictClause>) -> Self {
		self.on_conflict_clause = clause;
		self
	}

	/// Configure a fluent conflict target, action and optional caller SQL condition.
	pub fn on_conflict(mut self, clause: OnConflictClause) -> Self {
		self.on_conflict_clause = Some(clause);
		self
	}

	/// Performs the returning operation.
	pub fn returning(mut self, columns: Vec<&str>) -> Self {
		if self.backend.supports_returning() {
			self.returning = Some(columns.iter().map(|s| (*s).to_owned()).collect());
		}
		self
	}

	/// Performs the on conflict do nothing operation.
	pub fn on_conflict_do_nothing(mut self, conflict_columns: Option<Vec<String>>) -> Self {
		self.on_conflict = Some(OnConflictAction::DoNothing { conflict_columns });
		self
	}

	/// Set ON CONFLICT DO UPDATE behavior.
	///
	/// On SQLite 3.35.0+, `None` or an empty conflict column list matches any
	/// unique constraint. PostgreSQL requires a conflict target.
	/// SQLite SELECT sources use a typed always-true WHERE guard to avoid the
	/// [UPSERT parsing ambiguity](https://www.sqlite.org/lang_upsert.html#parsing_ambiguity),
	/// including targetless updates. Conflict actions precede RETURNING.
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

	fn statement(&self) -> Result<InsertStatement> {
		let mut statement = Query::insert()
			.into_table(Alias::new(&self.table))
			.columns(self.columns.iter().map(Alias::new))
			.from_subquery(self.select_stmt.clone())
			.to_owned();
		if let Some(columns) = &self.returning {
			statement.returning(columns.iter().map(Alias::new));
		}
		apply_insert_conflict(
			&mut statement,
			self.backend.database_type(),
			self.on_conflict.as_ref(),
			self.on_conflict_clause.as_ref(),
			true,
		)?;
		Ok(statement)
	}

	fn build_native(&self) -> Result<(String, Values)> {
		native_builder_pair(
			self.backend.database_type(),
			NativeBuilderStatement::Insert(Box::new(self.statement()?)),
		)
	}

	/// Render a standalone inline statement, preserving the legacy empty parameter list.
	/// Runtime methods use checked, bound rendering instead.
	///
	/// # Panics
	/// Panics for unsupported or invalid conflict configuration; runtime methods
	/// return the corresponding checked error before database execution.
	pub fn build(&self) -> (String, Vec<QueryValue>) {
		use reinhardt_query::prelude::{
			MySqlQueryBuilder, PostgresQueryBuilder, QueryStatementBuilder, SqliteQueryBuilder,
		};
		let statement = self
			.statement()
			.expect("invalid INSERT SELECT conflict configuration");
		let sql = match self.backend.database_type() {
			DatabaseType::Postgres => statement.to_string(PostgresQueryBuilder),
			DatabaseType::Mysql => statement.to_string(MySqlQueryBuilder),
			DatabaseType::Sqlite => statement.to_string(SqliteQueryBuilder),
		};
		(sql, Vec::new())
	}

	/// Execute the checked typed statement and exact renderer arguments.
	pub async fn execute(&self) -> Result<QueryResult> {
		self.backend
			.execute_generated(self.build_native()?, None)
			.await
	}

	/// Fetch a row from the checked typed statement and exact renderer arguments.
	pub async fn fetch_one(&self) -> Result<Row> {
		self.backend
			.fetch_one_generated(self.build_native()?, None)
			.await
	}
}

/// UPDATE query builder
pub struct UpdateBuilder {
	backend: Arc<dyn DatabaseBackend>,
	table: String,
	sets: Vec<(String, QueryValue)>,
	wheres: Vec<BuilderPredicate>,
}

impl UpdateBuilder {
	/// Creates a new instance.
	pub fn new(backend: Arc<dyn DatabaseBackend>, table: impl Into<String>) -> Self {
		Self {
			backend,
			table: table.into(),
			sets: Vec::new(),
			wheres: Vec::new(),
		}
	}

	/// Adds a bound SET value.
	pub fn set(mut self, column: impl Into<String>, value: impl Into<QueryValue>) -> Self {
		self.sets.push((column.into(), value.into()));
		self
	}

	/// Sets the column to the database's `CURRENT_TIMESTAMP` expression.
	///
	/// This expression does not consume a bound parameter.
	pub fn set_now(mut self, column: impl Into<String>) -> Self {
		self.sets.push((column.into(), QueryValue::Now));
		self
	}

	/// Adds an equality predicate, using `IS NULL` for `QueryValue::Null`.
	pub fn where_eq(mut self, column: impl Into<String>, value: impl Into<QueryValue>) -> Self {
		self.wheres
			.push(BuilderPredicate::Equal(column.into(), value.into()));
		self
	}

	fn statement_with(
		&self,
		convert: impl Fn(&QueryValue) -> Result<SimpleExpr>,
	) -> Result<UpdateStatement> {
		let mut stmt = Query::update().table(Alias::new(&self.table)).to_owned();
		for (column, value) in &self.sets {
			stmt.value_expr(Alias::new(column), convert(value)?);
		}
		for predicate in &self.wheres {
			stmt.and_where(predicate.expression_with(&convert)?);
		}
		Ok(stmt)
	}

	fn build_native(&self) -> Result<(String, Values)> {
		let backend = self.backend.database_type();
		native_builder_pair(
			backend,
			NativeBuilderStatement::Update(
				self.statement_with(|value| native_builder_expression(value, backend))?,
			),
		)
	}

	/// Builds SQL and legacy parameters for callers of the raw API.
	pub fn build(&self) -> (String, Vec<QueryValue>) {
		use reinhardt_query::prelude::{
			MySqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder,
		};
		// This converter only constructs expressions and cannot return Err.
		let stmt = self
			.statement_with(|value| Ok(query_value_expression(value)))
			.expect("legacy builder expressions are infallible");
		let sql = match self.backend.database_type() {
			DatabaseType::Postgres => PostgresQueryBuilder.build_update(&stmt).0,
			DatabaseType::Mysql => MySqlQueryBuilder.build_update(&stmt).0,
			DatabaseType::Sqlite => SqliteQueryBuilder.build_update(&stmt).0,
		};
		let params = legacy_builder_parameters(
			self.sets
				.iter()
				.map(|(_, value)| value)
				.chain(self.wheres.iter().flat_map(BuilderPredicate::values)),
		);
		(sql, params)
	}

	/// Executes the typed statement and exact renderer arguments.
	pub async fn execute(&self) -> Result<QueryResult> {
		self.backend
			.execute_generated(self.build_native()?, None)
			.await
	}
}

/// SELECT query builder
pub struct SelectBuilder {
	backend: Arc<dyn DatabaseBackend>,
	columns: Vec<String>,
	table: String,
	wheres: Vec<BuilderPredicate>,
	limit: Option<i64>,
}

impl SelectBuilder {
	/// Creates a new instance.
	pub fn new(backend: Arc<dyn DatabaseBackend>) -> Self {
		Self {
			backend,
			columns: vec!["*".into()],
			table: String::new(),
			wheres: Vec::new(),
			limit: None,
		}
	}

	/// Selects the supplied column identifiers.
	pub fn columns(mut self, columns: Vec<&str>) -> Self {
		self.columns = columns.iter().map(|column| (*column).into()).collect();
		self
	}

	/// Selects the source table.
	pub fn from(mut self, table: impl Into<String>) -> Self {
		self.table = table.into();
		self
	}

	/// Adds an equality predicate.
	pub fn where_eq(mut self, column: impl Into<String>, value: impl Into<QueryValue>) -> Self {
		self.wheres
			.push(BuilderPredicate::Equal(column.into(), value.into()));
		self
	}

	/// Sets a bound row limit; negative limits are omitted.
	pub fn limit(mut self, limit: i64) -> Self {
		self.limit = Some(limit);
		self
	}

	fn statement_with(
		&self,
		convert: impl Fn(&QueryValue) -> Result<SimpleExpr>,
	) -> Result<SelectStatement> {
		let mut stmt = Query::select().from(Alias::new(&self.table)).to_owned();
		if self.columns.as_slice() == ["*"] {
			stmt.column(ColumnRef::asterisk());
		} else {
			for column in &self.columns {
				stmt.column(Alias::new(column));
			}
		}
		for predicate in &self.wheres {
			stmt.and_where(predicate.expression_with(&convert)?);
		}
		if let Some(limit) = self.limit.and_then(|limit| u64::try_from(limit).ok()) {
			stmt.limit(limit);
		}
		Ok(stmt)
	}

	fn build_native(&self) -> Result<(String, Values)> {
		let backend = self.backend.database_type();
		native_builder_pair(
			backend,
			NativeBuilderStatement::Select(Box::new(
				self.statement_with(|value| native_builder_expression(value, backend))?,
			)),
		)
	}

	/// Builds SQL and legacy parameters for callers of the raw API.
	pub fn build(&self) -> (String, Vec<QueryValue>) {
		use reinhardt_query::prelude::{
			MySqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder,
		};
		// This converter only constructs expressions and cannot return Err.
		let stmt = self
			.statement_with(|value| Ok(query_value_expression(value)))
			.expect("legacy builder expressions are infallible");
		let sql = match self.backend.database_type() {
			DatabaseType::Postgres => PostgresQueryBuilder.build_select(&stmt).0,
			DatabaseType::Mysql => MySqlQueryBuilder.build_select(&stmt).0,
			DatabaseType::Sqlite => SqliteQueryBuilder.build_select(&stmt).0,
		};
		let mut params =
			legacy_builder_parameters(self.wheres.iter().flat_map(BuilderPredicate::values));
		if let Some(limit) = self.limit.filter(|limit| *limit >= 0) {
			params.push(QueryValue::Int(limit));
		}
		(sql, params)
	}

	/// Fetches all rows through the typed native provider.
	pub async fn fetch_all(&self) -> Result<Vec<Row>> {
		self.backend
			.fetch_all_generated(self.build_native()?, None)
			.await
	}

	/// Fetches one row through the typed native provider.
	pub async fn fetch_one(&self) -> Result<Row> {
		self.backend
			.fetch_one_generated(self.build_native()?, None)
			.await
	}
}

/// DELETE query builder
pub struct DeleteBuilder {
	backend: Arc<dyn DatabaseBackend>,
	table: String,
	wheres: Vec<BuilderPredicate>,
}

impl DeleteBuilder {
	/// Creates a new instance.
	pub fn new(backend: Arc<dyn DatabaseBackend>, table: impl Into<String>) -> Self {
		Self {
			backend,
			table: table.into(),
			wheres: Vec::new(),
		}
	}

	/// Adds an equality predicate.
	pub fn where_eq(mut self, column: impl Into<String>, value: impl Into<QueryValue>) -> Self {
		self.wheres
			.push(BuilderPredicate::Equal(column.into(), value.into()));
		self
	}

	/// Adds one IN predicate containing the complete input set.
	///
	/// Repeated calls and equality predicates are combined with AND.
	/// An empty set adds a false predicate, so no rows are deleted.
	///
	/// # Example
	///
	/// ```rust
	/// # #[cfg(feature = "sqlite")]
	/// # #[tokio::main]
	/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// use reinhardt_db::backends::{DatabaseConnection, QueryValue};
	///
	/// let db = DatabaseConnection::connect_sqlite("sqlite::memory:").await?;
	/// let (sql, params) = db.delete("users")
	///     .where_in("status", vec!["inactive".into(), "archived".into()])
	///     .build();
	///
	/// assert_eq!(sql, "DELETE FROM \"users\" WHERE \"status\" IN (?, ?)");
	/// assert_eq!(params, vec![
	///     QueryValue::String("inactive".into()),
	///     QueryValue::String("archived".into()),
	/// ]);
	/// # Ok(())
	/// # }
	/// # #[cfg(not(feature = "sqlite"))]
	/// # fn main() {}
	/// ```
	pub fn where_in(mut self, column: impl Into<String> + Clone, values: Vec<QueryValue>) -> Self {
		self.wheres
			.push(BuilderPredicate::In(column.into(), values));
		self
	}

	fn statement_with(
		&self,
		convert: impl Fn(&QueryValue) -> Result<SimpleExpr>,
	) -> Result<DeleteStatement> {
		let mut stmt = Query::delete()
			.from_table(Alias::new(&self.table))
			.to_owned();
		for predicate in &self.wheres {
			stmt.and_where(predicate.expression_with(&convert)?);
		}
		Ok(stmt)
	}

	fn build_native(&self) -> Result<(String, Values)> {
		let backend = self.backend.database_type();
		native_builder_pair(
			backend,
			NativeBuilderStatement::Delete(
				self.statement_with(|value| native_builder_expression(value, backend))?,
			),
		)
	}

	/// Builds SQL and legacy parameters for callers of the raw API.
	pub fn build(&self) -> (String, Vec<QueryValue>) {
		use reinhardt_query::prelude::{
			MySqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder,
		};
		// This converter only constructs expressions and cannot return Err.
		let stmt = self
			.statement_with(|value| Ok(query_value_expression(value)))
			.expect("legacy builder expressions are infallible");
		let sql = match self.backend.database_type() {
			DatabaseType::Postgres => PostgresQueryBuilder.build_delete(&stmt).0,
			DatabaseType::Mysql => MySqlQueryBuilder.build_delete(&stmt).0,
			DatabaseType::Sqlite => SqliteQueryBuilder.build_delete(&stmt).0,
		};
		let params =
			legacy_builder_parameters(self.wheres.iter().flat_map(BuilderPredicate::values));
		(sql, params)
	}

	/// Executes the typed statement and exact renderer arguments.
	pub async fn execute(&self) -> Result<QueryResult> {
		self.backend
			.execute_generated(self.build_native()?, None)
			.await
	}
}

/// ANALYZE statement builder for updating database statistics
///
/// The ANALYZE statement updates table statistics used by the query planner
/// to optimize query execution plans.
///
/// # Database Support
///
/// | Database | Syntax | Notes |
/// |----------|--------|-------|
/// | PostgreSQL | `ANALYZE [VERBOSE] [table [(columns...)]]` | Supports verbose mode and column-level analysis |
/// | MySQL | `ANALYZE TABLE table` | Requires an explicit, non-empty table name |
/// | SQLite | `ANALYZE [table_or_index]` | Analyzes entire database if no target specified |
/// | CockroachDB | `ANALYZE table` | PostgreSQL-compatible syntax |
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_db::backends::{AnalyzeBuilder, DatabaseBackend};
/// use std::sync::Arc;
/// # async fn example(backend: Arc<dyn DatabaseBackend>) -> Result<(), reinhardt_core::exception::Error> {
///
/// // Analyze all tables (PostgreSQL and SQLite only)
/// let builder = AnalyzeBuilder::new(backend.clone());
/// builder.execute().await?;
///
/// // Analyze specific table
/// let builder = AnalyzeBuilder::new(backend.clone())
///     .table("users");
/// builder.execute().await?;
///
/// // Analyze specific columns (PostgreSQL only)
/// let builder = AnalyzeBuilder::new(backend.clone())
///     .table("users")
///     .columns(vec!["email", "created_at"])
///     .verbose(true);
/// builder.execute().await?;
/// # Ok(())
/// # }
/// ```
pub struct AnalyzeBuilder {
	backend: Arc<dyn DatabaseBackend>,
	table: Option<String>,
	columns: Vec<String>,
	verbose: bool,
}

impl AnalyzeBuilder {
	/// Create a new ANALYZE builder
	///
	/// Without a table, PostgreSQL and SQLite analyze the entire database.
	/// MySQL requires an explicit, non-empty table name set with [`Self::table`];
	/// [`Self::execute`] returns a database error with kind [`DatabaseErrorKind::Unsupported`] otherwise.
	pub fn new(backend: Arc<dyn DatabaseBackend>) -> Self {
		Self {
			backend,
			table: None,
			columns: Vec::new(),
			verbose: false,
		}
	}

	/// Set the table to analyze
	///
	/// # Arguments
	///
	/// * `table` - The name of the table to analyze
	pub fn table(mut self, table: impl Into<String>) -> Self {
		self.table = Some(table.into());
		self
	}

	/// Set specific columns to analyze (PostgreSQL only)
	///
	/// This option is ignored on MySQL and SQLite as they don't support
	/// column-level ANALYZE.
	///
	/// # Arguments
	///
	/// * `columns` - List of column names to analyze
	pub fn columns(mut self, columns: Vec<&str>) -> Self {
		self.columns = columns.iter().map(|s| (*s).to_owned()).collect();
		self
	}

	/// Enable verbose output (PostgreSQL only)
	///
	/// When enabled, PostgreSQL will print progress messages as it analyzes.
	/// This option is ignored on MySQL and SQLite.
	pub fn verbose(mut self, verbose: bool) -> Self {
		self.verbose = verbose;
		self
	}

	fn statement(&self) -> reinhardt_query::query::AnalyzeStatement {
		let mut statement = Query::analyze().to_owned();
		if let Some(table) = &self.table {
			if self.backend.database_type() == DatabaseType::Postgres && !self.columns.is_empty() {
				statement.table_columns(Alias::new(table), self.columns.iter().map(Alias::new));
			} else {
				statement.table(Alias::new(table));
			}
		}
		// Preserve this backend API's documented non-PostgreSQL option behavior.
		if self.backend.database_type() == DatabaseType::Postgres && self.verbose {
			statement.verbose();
		}
		statement
	}

	fn build_native(&self) -> Result<(String, Values)> {
		use reinhardt_query::prelude::{
			MySqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder,
		};
		if self.backend.database_type() == DatabaseType::Mysql
			&& self.table.as_ref().is_none_or(String::is_empty)
		{
			return Err(DatabaseError::new(
				DatabaseErrorKind::Unsupported,
				"MySQL ANALYZE requires an explicit table; use AnalyzeBuilder::table()",
			)
			.into());
		}
		let statement = self.statement();
		let built = match self.backend.database_type() {
			DatabaseType::Postgres => PostgresQueryBuilder.build_analyze_checked(&statement),
			DatabaseType::Mysql => MySqlQueryBuilder.build_analyze_checked(&statement),
			DatabaseType::Sqlite => SqliteQueryBuilder.build_analyze_checked(&statement),
		};
		built.map_err(|error| {
			DatabaseError::new(DatabaseErrorKind::Unsupported, error.to_string()).into()
		})
	}

	/// Render standalone ANALYZE SQL with typed identifier escaping.
	/// Runtime execution also checks that the selected backend supports its target.
	pub fn build(&self) -> String {
		use reinhardt_query::prelude::{
			MySqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder,
		};
		let statement = self.statement();
		match self.backend.database_type() {
			DatabaseType::Postgres => PostgresQueryBuilder.build_analyze(&statement).0,
			DatabaseType::Mysql => MySqlQueryBuilder.build_analyze(&statement).0,
			DatabaseType::Sqlite => SqliteQueryBuilder.build_analyze(&statement).0,
		}
	}

	/// Execute the checked typed ANALYZE statement and owned renderer arguments.
	pub async fn execute(&self) -> Result<QueryResult> {
		self.backend
			.execute_generated(self.build_native()?, None)
			.await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::backends::backend::DatabaseBackend;
	use crate::backends::error::DatabaseErrorKind;
	use crate::backends::types::{DatabaseType, QueryResult, QueryValue, Row, TransactionExecutor};
	use rstest::rstest;

	#[rstest]
	#[case(
		DatabaseType::Postgres,
		"UPDATE \"users\" SET \"name\" = $1, \"updated_at\" = CURRENT_TIMESTAMP, \"nickname\" = NULL WHERE \"id\" = $2",
		"SELECT * FROM \"users\" WHERE \"name\" = $1 LIMIT $2",
		"DELETE FROM \"users\" WHERE \"id\" IN ($1, $2)"
	)]
	#[case(
		DatabaseType::Mysql,
		"UPDATE `users` SET `name` = ?, `updated_at` = CURRENT_TIMESTAMP, `nickname` = NULL WHERE `id` = ?",
		"SELECT * FROM `users` WHERE `name` = ? LIMIT ?",
		"DELETE FROM `users` WHERE `id` IN (?, ?)"
	)]
	#[case(
		DatabaseType::Sqlite,
		"UPDATE \"users\" SET \"name\" = ?, \"updated_at\" = CURRENT_TIMESTAMP, \"nickname\" = NULL WHERE \"id\" = ?",
		"SELECT * FROM \"users\" WHERE \"name\" = ? LIMIT ?",
		"DELETE FROM \"users\" WHERE \"id\" IN (?, ?)"
	)]
	fn native_builders_keep_null_time_limit_and_in_argument_order(
		#[case] dialect: DatabaseType,
		#[case] expected_update: &str,
		#[case] expected_select: &str,
		#[case] expected_delete: &str,
	) {
		// Arrange: no native driver is needed to verify each dialect's exact contract.
		let backend: Arc<dyn DatabaseBackend> = match dialect {
			DatabaseType::Postgres => Arc::new(MockBackend),
			DatabaseType::Mysql => Arc::new(MockMysqlBackend),
			DatabaseType::Sqlite => Arc::new(MockSqliteBackend),
		};
		let name = "quoted' ? $42";
		// Act
		let update = UpdateBuilder::new(backend.clone(), "users")
			.set("name", name)
			.set_now("updated_at")
			.set("nickname", QueryValue::Null)
			.where_eq("id", 7_i64)
			.build_native()
			.unwrap();
		let select = SelectBuilder::new(backend.clone())
			.from("users")
			.where_eq("name", name)
			.limit(1)
			.build_native()
			.unwrap();
		let delete = DeleteBuilder::new(backend, "users")
			.where_in("id", vec![7_i64.into(), 8_i64.into()])
			.build_native()
			.unwrap();
		// Assert: NULL and CURRENT_TIMESTAMP consume no slots; LIMIT owns its slot.
		assert_eq!(
			update,
			(
				expected_update.into(),
				Values(vec![name.into(), 7_i64.into()])
			)
		);
		assert_eq!(
			select,
			(
				expected_select.into(),
				Values(vec![name.into(), Value::BigUnsigned(Some(1))])
			)
		);
		assert_eq!(
			delete,
			(
				expected_delete.into(),
				Values(vec![7_i64.into(), 8_i64.into()])
			)
		);
	}

	#[rstest]
	#[case(DatabaseType::Postgres)]
	#[case(DatabaseType::Mysql)]
	#[case(DatabaseType::Sqlite)]
	fn native_builder_arrays_retain_explicit_backend_storage(#[case] backend: DatabaseType) {
		// Arrange
		let input = QueryValue::StringArray(vec!["quoted' ? $42".into()]);
		// Act
		let expression = native_builder_expression(&input, backend).unwrap();
		// Assert
		let expected = if backend == DatabaseType::Postgres {
			query_value_to_sea_value(&input)
		} else {
			Value::from(r#"["quoted' ? $42"]"#)
		};
		assert!(matches!(expression, SimpleExpr::Value(value) if value == expected));
	}

	#[rstest]
	#[case::string(QueryValue::NullableStringArray(vec![None, Some("kept".to_owned()), None]), reinhardt_query::value::ArrayType::String, vec![Value::String(None), Value::from("kept"), Value::String(None)])]
	#[case::int(QueryValue::NullableIntArray(vec![None, Some(i32::MAX), None]), reinhardt_query::value::ArrayType::Int, vec![Value::Int(None), Value::Int(Some(i32::MAX)), Value::Int(None)])]
	#[case::bigint(QueryValue::NullableBigIntArray(vec![None, Some(i64::MAX), None]), reinhardt_query::value::ArrayType::BigInt, vec![Value::BigInt(None), Value::BigInt(Some(i64::MAX)), Value::BigInt(None)])]
	#[case::bool(QueryValue::NullableBoolArray(vec![None, Some(false), None]), reinhardt_query::value::ArrayType::Bool, vec![Value::Bool(None), Value::Bool(Some(false)), Value::Bool(None)])]
	#[case::float(QueryValue::NullableFloatArray(vec![None, Some(1.5), None]), reinhardt_query::value::ArrayType::Float, vec![Value::Float(None), Value::Float(Some(1.5)), Value::Float(None)])]
	#[case::double(QueryValue::NullableDoubleArray(vec![None, Some(-2.5), None]), reinhardt_query::value::ArrayType::Double, vec![Value::Double(None), Value::Double(Some(-2.5)), Value::Double(None)])]
	#[case::uuid(QueryValue::NullableUuidArray(vec![None, Some(uuid::Uuid::nil()), None]), reinhardt_query::value::ArrayType::Uuid, vec![Value::Uuid(None), Value::Uuid(Some(Box::new(uuid::Uuid::nil()))), Value::Uuid(None)])]
	#[case::empty_string(QueryValue::NullableStringArray(vec![]), reinhardt_query::value::ArrayType::String, vec![])]
	#[case::empty_int(QueryValue::NullableIntArray(vec![]), reinhardt_query::value::ArrayType::Int, vec![])]
	#[case::empty_bigint(QueryValue::NullableBigIntArray(vec![]), reinhardt_query::value::ArrayType::BigInt, vec![])]
	#[case::empty_bool(QueryValue::NullableBoolArray(vec![]), reinhardt_query::value::ArrayType::Bool, vec![])]
	#[case::empty_float(QueryValue::NullableFloatArray(vec![]), reinhardt_query::value::ArrayType::Float, vec![])]
	#[case::empty_double(QueryValue::NullableDoubleArray(vec![]), reinhardt_query::value::ArrayType::Double, vec![])]
	#[case::empty_uuid(QueryValue::NullableUuidArray(vec![]), reinhardt_query::value::ArrayType::Uuid, vec![])]
	fn query_builder_preserves_nullable_array_types(
		#[case] input: QueryValue,
		#[case] array_type: reinhardt_query::value::ArrayType,
		#[case] expected_elements: Vec<Value>,
	) {
		// Act
		let value = query_value_to_sea_value(&input);

		// Assert
		assert_eq!(
			value,
			Value::Array(array_type, Some(Box::new(expected_elements)))
		);
	}

	#[rstest]
	#[case::int32_min(QueryValue::Int32(i32::MIN), Value::Int(Some(i32::MIN)))]
	#[case::int32_max(QueryValue::Int32(i32::MAX), Value::Int(Some(i32::MAX)))]
	#[case::bigint_small(QueryValue::Int(3), Value::BigInt(Some(3)))]
	#[case::bigint_max(QueryValue::Int(i64::MAX), Value::BigInt(Some(i64::MAX)))]
	fn query_builder_preserves_integer_parameter_width(
		#[case] input: QueryValue,
		#[case] expected: Value,
	) {
		// Act
		let value = query_value_to_sea_value(&input);

		// Assert
		assert_eq!(value, expected);
	}

	// Mock transaction executor for testing
	struct MockTransactionExecutor {
		backend: DatabaseType,
	}

	#[async_trait::async_trait]
	impl TransactionExecutor for MockTransactionExecutor {
		fn backend(&self) -> DatabaseType {
			self.backend
		}

		async fn execute(&mut self, _sql: &str, _params: Vec<QueryValue>) -> Result<QueryResult> {
			Ok(QueryResult {
				rows_affected: 0,
				last_insert_id: None,
			})
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
			Ok(QueryResult {
				rows_affected: 1,
				last_insert_id: None,
			})
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
			Ok(Box::new(MockTransactionExecutor {
				backend: DatabaseType::Postgres,
			}))
		}
	}

	#[test]
	fn test_delete_builder_basic() {
		let backend = Arc::new(MockBackend);
		let builder = DeleteBuilder::new(backend, "users");
		let (sql, params) = builder.build();

		// reinhardt-query uses quotes for identifiers
		assert_eq!(sql, "DELETE FROM \"users\"");
		assert!(params.is_empty());
	}

	#[test]
	fn test_delete_builder_where_eq() {
		let backend = Arc::new(MockBackend);
		let builder = DeleteBuilder::new(backend, "users").where_eq("id", QueryValue::Int(1));
		let (sql, params) = builder.build();

		// reinhardt-query uses parameterized queries with placeholders
		assert_eq!(sql, "DELETE FROM \"users\" WHERE \"id\" = $1");
		assert_eq!(params.len(), 1);
		assert!(matches!(params[0], QueryValue::Int(1)));
	}

	#[rstest]
	fn test_delete_builder_where_in() {
		// Arrange
		let backend = Arc::new(MockBackend);
		let builder = DeleteBuilder::new(backend, "users")
			.where_in("id", vec![QueryValue::Int(1), QueryValue::Int(2)]);

		// Act
		let (sql, params) = builder.build();

		// Assert
		assert_eq!(sql, "DELETE FROM \"users\" WHERE \"id\" IN ($1, $2)");
		assert_eq!(params, vec![QueryValue::Int(1), QueryValue::Int(2)]);
	}

	#[test]
	fn test_delete_builder_multiple_conditions() {
		let backend = Arc::new(MockBackend);
		let builder = DeleteBuilder::new(backend, "users")
			.where_eq("status", QueryValue::String("inactive".to_string()))
			.where_eq("age", QueryValue::Int(18));
		let (sql, params) = builder.build();

		// reinhardt-query uses parameterized queries with placeholders
		assert_eq!(
			sql,
			"DELETE FROM \"users\" WHERE \"status\" = $1 AND \"age\" = $2"
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
			Ok(QueryResult {
				rows_affected: 1,
				last_insert_id: None,
			})
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
			Ok(Box::new(MockTransactionExecutor {
				backend: DatabaseType::Mysql,
			}))
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
			Ok(QueryResult {
				rows_affected: 1,
				last_insert_id: None,
			})
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
			Ok(Box::new(MockTransactionExecutor {
				backend: DatabaseType::Sqlite,
			}))
		}
	}

	#[tokio::test]
	async fn test_mock_transaction_executors_report_origin_backend() {
		let postgres_transaction = MockBackend.begin().await.unwrap();
		let mysql_transaction = MockMysqlBackend.begin().await.unwrap();
		let sqlite_transaction = MockSqliteBackend.begin().await.unwrap();

		assert_eq!(postgres_transaction.backend(), DatabaseType::Postgres);
		assert_eq!(mysql_transaction.backend(), DatabaseType::Mysql);
		assert_eq!(sqlite_transaction.backend(), DatabaseType::Sqlite);
	}

	// Tests for OnConflictClause (new fluent API)

	// ==========================================
	// PostgreSQL Tests - Exact SQL Verification
	// ==========================================

	#[rstest]
	#[case(
		DatabaseType::Postgres,
		"INSERT INTO \"users\" (\"nullable\", \"id\", \"touched\", \"name\") VALUES (NULL, $1, CURRENT_TIMESTAMP, $2)"
	)]
	#[case(
		DatabaseType::Mysql,
		"INSERT INTO `users` (`nullable`, `id`, `touched`, `name`) VALUES (NULL, ?, CURRENT_TIMESTAMP, ?)"
	)]
	#[case(
		DatabaseType::Sqlite,
		"INSERT INTO \"users\" (\"nullable\", \"id\", \"touched\", \"name\") VALUES (NULL, ?, CURRENT_TIMESTAMP, ?)"
	)]
	fn native_insert_null_time_argument_order(
		#[case] dialect: DatabaseType,
		#[case] expected: &str,
	) {
		// Arrange
		let backend: Arc<dyn DatabaseBackend> = match dialect {
			DatabaseType::Postgres => Arc::new(MockBackend),
			DatabaseType::Mysql => Arc::new(MockMysqlBackend),
			DatabaseType::Sqlite => Arc::new(MockSqliteBackend),
		};
		let builder = InsertBuilder::new(backend, "users")
			.value("nullable", QueryValue::Null)
			.value("id", 1_i64)
			.value("touched", QueryValue::Now)
			.value("name", "payload' ? $9");
		// Act
		let native = builder.build_native().unwrap();
		let public = builder.build().unwrap();
		// Assert
		assert_eq!(
			native,
			(
				expected.into(),
				Values(vec![1_i64.into(), "payload' ? $9".into()])
			)
		);
		assert_eq!(
			public,
			(
				expected.into(),
				vec![
					QueryValue::Int(1),
					QueryValue::String("payload' ? $9".into())
				]
			)
		);
	}

	#[rstest]
	#[case(
		DatabaseType::Postgres,
		"INSERT INTO \"users\" (\"id\", \"name\") SELECT $1, $2 ON CONFLICT (\"id\") DO UPDATE SET \"name\" = EXCLUDED.\"name\""
	)]
	#[case(
		DatabaseType::Mysql,
		"INSERT INTO `users` (`id`, `name`) SELECT ?, ? ON DUPLICATE KEY UPDATE `name` = VALUES(`name`)"
	)]
	#[case(
		DatabaseType::Sqlite,
		"INSERT INTO \"users\" (\"id\", \"name\") SELECT ?, ? ON CONFLICT (\"id\") DO UPDATE SET \"name\" = EXCLUDED.\"name\""
	)]
	fn insert_select_conversion_keeps_fluent_conflict_and_values(
		#[case] dialect: DatabaseType,
		#[case] expected: &str,
	) {
		// Arrange
		let backend: Arc<dyn DatabaseBackend> = match dialect {
			DatabaseType::Postgres => Arc::new(MockBackend),
			DatabaseType::Mysql => Arc::new(MockMysqlBackend),
			DatabaseType::Sqlite => Arc::new(MockSqliteBackend),
		};
		let builder = InsertBuilder::new(backend, "users")
			.on_conflict(OnConflictClause::columns(vec!["id"]).do_update(vec!["name"]))
			.from_select(
				vec!["id", "name"],
				Query::select()
					.expr(Expr::val(1_i64))
					.expr(Expr::val("source' ? $9"))
					.to_owned(),
			);
		// Act
		let native = builder.build_native().unwrap();
		let (inline, legacy) = builder.build();
		// Assert
		assert_eq!(
			native,
			(
				expected.into(),
				Values(vec![1_i64.into(), "source' ? $9".into()])
			)
		);
		assert!(inline.contains("'source'' ? $9'"));
		assert!(legacy.is_empty());
	}

	#[rstest]
	#[case::legacy(false)]
	#[case::fluent(true)]
	fn sqlite_insert_preserves_targetless_update(#[case] fluent: bool) {
		// Arrange
		let builder = InsertBuilder::new(Arc::new(MockSqliteBackend), "users").value("id", 1_i64);
		let builder = if fluent {
			builder.on_conflict(OnConflictClause::any().do_update(vec!["id"]))
		} else {
			builder.on_conflict_do_update(None, vec!["id".into()])
		};
		// Act
		let (sql, values) = builder.build_native().unwrap();
		// Assert
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"id\") VALUES (?) ON CONFLICT DO UPDATE SET \"id\" = EXCLUDED.\"id\""
		);
		assert_eq!(values, Values(vec![1_i64.into()]));
	}

	#[test]
	fn test_on_conflict_clause_columns_do_nothing_postgres_exact_sql() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(OnConflictClause::columns(vec!["email"]).do_nothing());
		let (sql, params) = builder.build().unwrap();

		// Assert - verify exact SQL structure (reinhardt-query uses parameterized queries)
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\") VALUES ($1) ON CONFLICT (\"email\") DO NOTHING"
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
		let (sql, params) = builder.build().unwrap();

		// Assert - verify exact SQL structure with EXCLUDED references (reinhardt-query uses parameterized queries)
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\", \"name\") VALUES ($1, $2) ON CONFLICT (\"email\") DO UPDATE SET \"name\" = EXCLUDED.\"name\", \"updated_at\" = EXCLUDED.\"updated_at\""
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
		let (sql, params) = builder.build().unwrap();

		// Assert - verify WHERE clause is appended correctly (reinhardt-query uses parameterized queries)
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\", \"version\") VALUES ($1, $2) ON CONFLICT (\"email\") DO UPDATE SET \"version\" = EXCLUDED.\"version\" WHERE users.version < EXCLUDED.version"
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
		let (sql, _) = builder.build().unwrap();

		// Assert - verify ON CONSTRAINT syntax (reinhardt-query uses parameterized queries)
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\") VALUES ($1) ON CONFLICT ON CONSTRAINT \"users_email_key\" DO UPDATE SET \"name\" = EXCLUDED.\"name\""
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
		let (sql, _) = builder.build().unwrap();

		// Assert - verify no conflict target specified (reinhardt-query uses parameterized queries)
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\") VALUES ($1) ON CONFLICT DO NOTHING"
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
		let (sql, _) = builder.build().unwrap();

		// Assert - verify multiple conflict columns (reinhardt-query uses parameterized queries)
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"tenant_id\", \"email\") VALUES ($1, $2) ON CONFLICT (\"tenant_id\", \"email\") DO UPDATE SET \"name\" = EXCLUDED.\"name\""
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
		let (sql, _) = builder.build().unwrap();

		// Assert - verify all update columns are included (reinhardt-query uses parameterized queries)
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"id\") VALUES ($1) ON CONFLICT (\"id\") DO UPDATE SET \"name\" = EXCLUDED.\"name\", \"email\" = EXCLUDED.\"email\", \"updated_at\" = EXCLUDED.\"updated_at\", \"version\" = EXCLUDED.\"version\""
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
		let (sql, _) = builder.build().unwrap();

		// Assert - MySQL uses INSERT IGNORE syntax (reinhardt-query uses parameterized queries)
		assert_eq!(sql, "INSERT IGNORE INTO `users` (`email`) VALUES (?)");
	}

	#[test]
	fn test_on_conflict_clause_do_update_mysql_exact_sql() {
		// Arrange
		let backend = Arc::new(MockMysqlBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(OnConflictClause::columns(vec!["email"]).do_update(vec!["name"]));
		let (sql, _) = builder.build().unwrap();

		// Assert - MySQL uses ON DUPLICATE KEY UPDATE with VALUES() function (reinhardt-query uses parameterized queries)
		assert_eq!(
			sql,
			"INSERT INTO `users` (`email`) VALUES (?) ON DUPLICATE KEY UPDATE `name` = VALUES(`name`)"
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
		let (sql, _) = builder.build().unwrap();

		// Assert - verify multiple update columns with VALUES() syntax (reinhardt-query uses parameterized queries)
		assert_eq!(
			sql,
			"INSERT INTO `users` (`id`) VALUES (?) ON DUPLICATE KEY UPDATE `name` = VALUES(`name`), `email` = VALUES(`email`), `updated_at` = VALUES(`updated_at`)"
		);
	}

	#[rstest]
	fn test_on_conflict_clause_where_rejected_mysql() {
		// Arrange
		let backend = Arc::new(MockMysqlBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(
				OnConflictClause::columns(vec!["email"])
					.do_update(vec!["name"])
					.where_clause("users.version < VALUES(version)"),
			);
		let error = builder.build().unwrap_err();

		// Assert
		assert_eq!(error.database_kind(), Some(DatabaseErrorKind::Unsupported));
		assert!(
			error
				.to_string()
				.contains("MySQL does not support conditional ON DUPLICATE KEY UPDATE")
		);
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
		let (sql, _) = builder.build().unwrap();

		// Assert - the fluent API retains its SQLite conflict target.
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\") VALUES (?) ON CONFLICT (\"email\") DO NOTHING"
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
		let (sql, _) = builder.build().unwrap();

		// Assert - typed SQLite EXCLUDED identifiers preserve the upsert assignment.
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\") VALUES (?) ON CONFLICT (\"email\") DO UPDATE SET \"name\" = EXCLUDED.\"name\""
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
		let (sql, _) = builder.build().unwrap();

		// Assert - SQLite supports WHERE clause (reinhardt-query uses parameterized queries)
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\") VALUES (?) ON CONFLICT (\"email\") DO UPDATE SET \"version\" = EXCLUDED.\"version\" WHERE users.version < excluded.version"
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
		let (sql, _) = builder.build().unwrap();

		// Assert - verify multiple conflict columns (reinhardt-query uses parameterized queries)
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"tenant_id\", \"email\") VALUES (?, ?) ON CONFLICT (\"tenant_id\", \"email\") DO UPDATE SET \"name\" = EXCLUDED.\"name\""
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
		let (sql, _) = builder.build().unwrap();

		// Assert - an omitted target handles any uniqueness conflict.
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\") VALUES (?) ON CONFLICT DO NOTHING"
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
		let (sql, _) = builder.build().unwrap();

		// Assert - legacy API should still work (reinhardt-query uses parameterized queries)
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\") VALUES ($1) ON CONFLICT (\"email\") DO NOTHING"
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
		let (sql, _) = builder.build().unwrap();

		// Assert - legacy API should still work (reinhardt-query uses parameterized queries)
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\") VALUES ($1) ON CONFLICT (\"email\") DO UPDATE SET \"name\" = EXCLUDED.\"name\", \"updated_at\" = EXCLUDED.\"updated_at\""
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
		let (sql, _) = builder.build().unwrap();

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
		let (sql, _) = builder.build().unwrap();

		// Assert (reinhardt-query uses parameterized queries)
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"id\") VALUES ($1) ON CONFLICT (\"id\") DO UPDATE SET \"name\" = EXCLUDED.\"name\""
		);
	}

	#[rstest]
	fn test_on_conflict_clause_with_returning() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.returning(vec!["id", "created_at"])
			.on_conflict(OnConflictClause::columns(vec!["email"]).do_update(vec!["name"]));
		let (sql, _) = builder.build().unwrap();

		// Assert
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\") VALUES ($1) ON CONFLICT (\"email\") DO UPDATE SET \"name\" = EXCLUDED.\"name\" RETURNING \"id\", \"created_at\""
		);
	}

	#[rstest]
	fn test_on_conflict_clause_with_null_value() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.value("name", QueryValue::Null)
			.on_conflict(OnConflictClause::columns(vec!["email"]).do_update(vec!["name"]));
		let (sql, params) = builder.build().unwrap();

		// Assert
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"email\", \"name\") VALUES ($1, NULL) ON CONFLICT (\"email\") DO UPDATE SET \"name\" = EXCLUDED.\"name\""
		);
		assert_eq!(params, vec![QueryValue::from("test@example.com")]);
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
		let (sql, params) = builder.build().unwrap();

		// Assert (reinhardt-query uses parameterized queries)
		assert_eq!(
			sql,
			"INSERT INTO \"counters\" (\"key\", \"count\") VALUES ($1, $2) ON CONFLICT (\"key\") DO UPDATE SET \"count\" = EXCLUDED.\"count\""
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
		let (sql, _) = builder.build().unwrap();

		// Assert - complex WHERE with AND condition (reinhardt-query uses parameterized queries)
		assert_eq!(
			sql,
			"INSERT INTO \"documents\" (\"id\") VALUES ($1) ON CONFLICT (\"id\") DO UPDATE SET \"content\" = EXCLUDED.\"content\", \"version\" = EXCLUDED.\"version\" WHERE documents.version < EXCLUDED.version AND documents.locked = false"
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
		let (sql, _) = builder.build().unwrap();

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
		let (sql, _) = builder.build().unwrap();

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
		let (sql1, _) = builder1.build().unwrap();
		assert!(sql1.contains("ON CONFLICT (\"a\", \"b\") DO NOTHING"));

		let backend2: Arc<dyn DatabaseBackend> = Arc::new(MockBackend);
		let builder2 = InsertBuilder::new(backend2, "t")
			.value("x", QueryValue::Int(1))
			.on_conflict(constraint_clause.do_nothing());
		let (sql2, _) = builder2.build().unwrap();
		assert!(sql2.contains("ON CONFLICT ON CONSTRAINT \"my_constraint\" DO NOTHING"));

		let backend3: Arc<dyn DatabaseBackend> = Arc::new(MockBackend);
		let builder3 = InsertBuilder::new(backend3, "t")
			.value("x", QueryValue::Int(1))
			.on_conflict(any_clause.do_nothing());
		let (sql3, _) = builder3.build().unwrap();
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
		let (_, params) = builder.build().unwrap();

		// Assert - all parameters should be preserved in order
		assert_eq!(params.len(), 3);
		assert!(matches!(params[0], QueryValue::Int(42)));
		assert!(matches!(&params[1], QueryValue::String(s) if s == "John"));
		assert!(matches!(params[2], QueryValue::Bool(true)));
	}

	// ==========================================
	// ANALYZE Builder Tests
	// ==========================================

	#[test]
	fn test_analyze_builder_postgres_database_wide() {
		let backend = Arc::new(MockBackend);
		let builder = AnalyzeBuilder::new(backend);
		let sql = builder.build();
		assert_eq!(sql, "ANALYZE");
	}

	#[test]
	fn test_analyze_builder_postgres_specific_table() {
		let backend = Arc::new(MockBackend);
		let builder = AnalyzeBuilder::new(backend).table("users");
		let sql = builder.build();
		assert_eq!(sql, "ANALYZE \"users\"");
	}

	#[test]
	fn test_analyze_builder_postgres_verbose() {
		let backend = Arc::new(MockBackend);
		let builder = AnalyzeBuilder::new(backend).table("users").verbose(true);
		let sql = builder.build();
		assert_eq!(sql, "ANALYZE VERBOSE \"users\"");
	}

	#[test]
	fn test_analyze_builder_postgres_with_columns() {
		let backend = Arc::new(MockBackend);
		let builder = AnalyzeBuilder::new(backend)
			.table("users")
			.columns(vec!["email", "created_at"]);
		let sql = builder.build();
		assert_eq!(sql, "ANALYZE \"users\" (\"email\", \"created_at\")");
	}

	#[test]
	fn test_analyze_builder_postgres_verbose_with_columns() {
		let backend = Arc::new(MockBackend);
		let builder = AnalyzeBuilder::new(backend)
			.table("users")
			.columns(vec!["email"])
			.verbose(true);
		let sql = builder.build();
		assert_eq!(sql, "ANALYZE VERBOSE \"users\" (\"email\")");
	}

	#[test]
	fn test_analyze_builder_mysql_specific_table() {
		let backend = Arc::new(MockMysqlBackend);
		let builder = AnalyzeBuilder::new(backend).table("users");
		let sql = builder.build();
		assert_eq!(sql, "ANALYZE TABLE `users`");
	}

	#[test]
	fn test_analyze_builder_mysql_database_wide() {
		let backend = Arc::new(MockMysqlBackend);
		let builder = AnalyzeBuilder::new(backend);
		let sql = builder.build();
		assert_eq!(sql, "ANALYZE TABLE");
	}

	#[cfg(feature = "mysql")]
	#[rstest]
	#[case::unspecified(None)]
	#[case::empty(Some(""))]
	#[tokio::test]
	async fn test_analyze_builder_mysql_rejects_missing_table_before_execution(
		#[case] table: Option<&str>,
	) {
		// Arrange: a closed native pool would fail if SQL execution were attempted.
		let pool = sqlx::mysql::MySqlPoolOptions::new()
			.connect_lazy("mysql://localhost/analyze_test")
			.unwrap();
		pool.close().await;
		let backend = Arc::new(crate::backends::MySqlBackend::new(pool));
		let mut builder = AnalyzeBuilder::new(backend);
		if let Some(table) = table {
			builder = builder.table(table);
		}

		// Act
		let error = builder.execute().await.unwrap_err();

		// Assert
		assert_eq!(error.database_kind(), Some(DatabaseErrorKind::Unsupported));
		assert!(
			error
				.to_string()
				.contains("MySQL ANALYZE requires an explicit table; use AnalyzeBuilder::table()")
		);
	}

	#[cfg(feature = "mysql")]
	#[rstest]
	#[tokio::test]
	async fn test_analyze_builder_mysql_explicit_table_reaches_executor() {
		// Arrange
		let pool = sqlx::mysql::MySqlPoolOptions::new()
			.connect_lazy("mysql://localhost/analyze_test")
			.unwrap();
		pool.close().await;
		let backend = Arc::new(crate::backends::MySqlBackend::new(pool));

		// Act
		let error = AnalyzeBuilder::new(backend)
			.table("users")
			.execute()
			.await
			.unwrap_err();

		// Assert: a valid target passes validation and reaches the closed pool.
		assert_eq!(error.database_kind(), Some(DatabaseErrorKind::Connection));
	}

	#[rstest]
	#[case::postgres(DatabaseType::Postgres)]
	#[case::sqlite(DatabaseType::Sqlite)]
	#[tokio::test]
	async fn test_analyze_builder_database_wide_supported_backends(#[case] database: DatabaseType) {
		// Arrange
		let backend: Arc<dyn DatabaseBackend> = match database {
			DatabaseType::Postgres => Arc::new(MockBackend),
			DatabaseType::Sqlite => Arc::new(MockSqliteBackend),
			DatabaseType::Mysql => unreachable!("covered by MySQL target tests"),
		};

		// Act
		let result = AnalyzeBuilder::new(backend).execute().await.unwrap();

		// Assert
		assert_eq!(result.rows_affected, 1);
	}

	#[test]
	fn test_analyze_builder_sqlite_database_wide() {
		let backend = Arc::new(MockSqliteBackend);
		let builder = AnalyzeBuilder::new(backend);
		let sql = builder.build();
		assert_eq!(sql, "ANALYZE");
	}

	#[test]
	fn test_analyze_builder_sqlite_specific_table() {
		let backend = Arc::new(MockSqliteBackend);
		let builder = AnalyzeBuilder::new(backend).table("users");
		let sql = builder.build();
		assert_eq!(sql, "ANALYZE \"users\"");
	}

	// ==========================================
	// INSERT FROM SELECT Tests
	// ==========================================

	#[rstest]
	#[case::postgres(
		DatabaseType::Postgres,
		"INSERT INTO \"target_table\" (\"id\", \"name\") SELECT \"id\", \"name\" FROM \"source_table\" ON CONFLICT (\"id\") DO UPDATE SET \"name\" = EXCLUDED.\"name\""
	)]
	#[case::mysql(
		DatabaseType::Mysql,
		"INSERT INTO `target_table` (`id`, `name`) SELECT `id`, `name` FROM `source_table` ON DUPLICATE KEY UPDATE `name` = VALUES(`name`)"
	)]
	#[case::sqlite(
		DatabaseType::Sqlite,
		"INSERT INTO \"target_table\" (\"id\", \"name\") SELECT \"id\", \"name\" FROM \"source_table\" WHERE TRUE ON CONFLICT (\"id\") DO UPDATE SET \"name\" = EXCLUDED.\"name\""
	)]
	fn insert_from_select_upsert_sql(#[case] db_type: DatabaseType, #[case] expected_sql: &str) {
		// Arrange
		let backend: Arc<dyn DatabaseBackend> = match db_type {
			DatabaseType::Postgres => Arc::new(MockBackend),
			DatabaseType::Mysql => Arc::new(MockMysqlBackend),
			DatabaseType::Sqlite => Arc::new(MockSqliteBackend),
		};
		let source = Query::select()
			.columns(["id", "name"])
			.from("source_table")
			.to_owned();
		let builder = InsertFromSelectBuilder::new(
			backend,
			"target_table",
			vec!["id".into(), "name".into()],
			source,
		)
		.on_conflict_do_update(Some(vec!["id".into()]), vec!["name".into()]);

		// Act
		let (sql, params) = builder.build();

		// Assert
		assert_eq!(sql, expected_sql);
		assert_eq!(params, vec![]);
	}

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

		// Assert (inline values in SQL, no parameters)
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
		assert!(sql.contains("ON CONFLICT (\"id\") DO NOTHING"));
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
		assert!(sql.contains("ON CONFLICT (\"id\") DO UPDATE SET \"name\" = EXCLUDED.\"name\""));
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
		assert!(sql.contains("ON CONFLICT (\"id\") DO NOTHING"));
	}

	#[rstest]
	#[case::postgres_update(
		DatabaseType::Postgres,
		OnConflictClause::columns(vec!["id"]).do_update(vec!["id"]),
		"INSERT INTO \"users\" (\"id\") SELECT 4 ON CONFLICT (\"id\") DO UPDATE SET \"id\" = EXCLUDED.\"id\""
	)]
	#[case::postgres_nothing(
		DatabaseType::Postgres,
		OnConflictClause::columns(vec!["id"]).do_nothing(),
		"INSERT INTO \"users\" (\"id\") SELECT 4 ON CONFLICT (\"id\") DO NOTHING"
	)]
	#[case::postgres_any(
		DatabaseType::Postgres,
		OnConflictClause::any().do_nothing(),
		"INSERT INTO \"users\" (\"id\") SELECT 4 ON CONFLICT DO NOTHING"
	)]
	#[case::postgres_constraint(
		DatabaseType::Postgres,
		OnConflictClause::constraint("users_pkey").do_update(vec!["id"]),
		"INSERT INTO \"users\" (\"id\") SELECT 4 ON CONFLICT ON CONSTRAINT \"users_pkey\" DO UPDATE SET \"id\" = EXCLUDED.\"id\""
	)]
	#[case::postgres_condition(
		DatabaseType::Postgres,
		OnConflictClause::columns(vec!["id"]).do_update(vec!["id"])
			.where_clause("users.id < EXCLUDED.id"),
		"INSERT INTO \"users\" (\"id\") SELECT 4 ON CONFLICT (\"id\") DO UPDATE SET \"id\" = EXCLUDED.\"id\" WHERE users.id < EXCLUDED.id"
	)]
	#[case::mysql_update(
		DatabaseType::Mysql,
		OnConflictClause::columns(vec!["id"]).do_update(vec!["id"]),
		"INSERT INTO `users` (`id`) SELECT 4 ON DUPLICATE KEY UPDATE `id` = VALUES(`id`)"
	)]
	#[case::mysql_nothing(
		DatabaseType::Mysql,
		OnConflictClause::any().do_nothing(),
		"INSERT IGNORE INTO `users` (`id`) SELECT 4"
	)]
	#[case::sqlite_update(
		DatabaseType::Sqlite,
		OnConflictClause::columns(vec!["id"]).do_update(vec!["id"]),
		"INSERT INTO \"users\" (\"id\") SELECT 4 ON CONFLICT (\"id\") DO UPDATE SET \"id\" = EXCLUDED.\"id\""
	)]
	#[case::sqlite_condition(
		DatabaseType::Sqlite,
		OnConflictClause::columns(vec!["id"]).do_update(vec!["id"])
			.where_clause("users.id < excluded.id"),
		"INSERT INTO \"users\" (\"id\") SELECT 4 ON CONFLICT (\"id\") DO UPDATE SET \"id\" = EXCLUDED.\"id\" WHERE users.id < excluded.id"
	)]
	#[case::sqlite_nothing(
		DatabaseType::Sqlite,
		OnConflictClause::any().do_nothing(),
		"INSERT INTO \"users\" (\"id\") SELECT 4 ON CONFLICT DO NOTHING"
	)]
	#[case::sqlite_targeted_nothing(
		DatabaseType::Sqlite,
		OnConflictClause::columns(vec!["id"]).do_nothing(),
		"INSERT INTO \"users\" (\"id\") SELECT 4 ON CONFLICT (\"id\") DO NOTHING"
	)]
	fn test_insert_builder_from_select_preserves_fluent_conflict(
		#[case] db_type: DatabaseType,
		#[case] clause: OnConflictClause,
		#[case] expected_sql: &str,
	) {
		// Arrange
		let backend: Arc<dyn DatabaseBackend> = match db_type {
			DatabaseType::Postgres => Arc::new(MockBackend),
			DatabaseType::Mysql => Arc::new(MockMysqlBackend),
			DatabaseType::Sqlite => Arc::new(MockSqliteBackend),
		};
		let select = Query::select().expr(Expr::val(4)).to_owned();
		let builder = InsertBuilder::new(backend, "users").on_conflict(clause);

		// Act
		let (sql, params) = builder.from_select(vec!["id"], select).build();

		// Assert
		assert_eq!(sql, expected_sql);
		assert_eq!(params, Vec::new());
	}

	#[rstest]
	#[case::legacy_first(true)]
	#[case::fluent_first(false)]
	fn test_insert_builder_from_select_fluent_conflict_overrides_legacy(
		#[case] legacy_first: bool,
	) {
		// Arrange
		let backend = Arc::new(MockBackend);
		let select = Query::select().expr(Expr::val(4)).to_owned();
		let clause = OnConflictClause::columns(vec!["id"]).do_update(vec!["id"]);
		let builder = InsertBuilder::new(backend, "users");
		let builder = if legacy_first {
			builder.on_conflict_do_nothing(None).on_conflict(clause)
		} else {
			builder.on_conflict(clause).on_conflict_do_nothing(None)
		};

		// Act
		let (sql, params) = builder.from_select(vec!["id"], select).build();

		// Assert
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"id\") SELECT 4 ON CONFLICT (\"id\") DO UPDATE SET \"id\" = EXCLUDED.\"id\""
		);
		assert_eq!(params, Vec::new());
	}

	#[rstest]
	fn test_insert_builder_from_select_fluent_conflict_precedes_returning() {
		// Arrange
		let backend = Arc::new(MockBackend);
		let select = Query::select().expr(Expr::val(4)).to_owned();
		let builder = InsertBuilder::new(backend, "users")
			.returning(vec!["id"])
			.on_conflict(OnConflictClause::columns(vec!["id"]).do_update(vec!["id"]));

		// Act
		let (sql, params) = builder.from_select(vec!["id"], select).build();

		// Assert
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"id\") SELECT 4 ON CONFLICT (\"id\") DO UPDATE SET \"id\" = EXCLUDED.\"id\" RETURNING \"id\""
		);
		assert_eq!(params, Vec::new());
	}

	#[rstest]
	fn test_insert_builder_from_select_legacy_conflict_precedes_returning() {
		// Arrange
		let backend = Arc::new(MockBackend);
		let select = Query::select().expr(Expr::val(4)).to_owned();
		let builder = InsertBuilder::new(backend, "users")
			.returning(vec!["id"])
			.on_conflict_do_nothing(Some(vec!["id".into()]));

		// Act
		let (sql, params) = builder.from_select(vec!["id"], select).build();

		// Assert
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"id\") SELECT 4 ON CONFLICT (\"id\") DO NOTHING RETURNING \"id\""
		);
		assert_eq!(params, Vec::new());
	}

	#[rstest]
	#[should_panic(expected = "invalid INSERT SELECT conflict configuration")]
	fn test_insert_builder_from_select_build_rejects_invalid_fluent_conflict() {
		// Arrange
		let backend = Arc::new(MockSqliteBackend);
		let select = Query::select().expr(Expr::val(4)).to_owned();
		let builder = InsertBuilder::new(backend, "users")
			.on_conflict(OnConflictClause::columns(Vec::<String>::new()).do_update(vec!["id"]))
			.from_select(vec!["id"], select);

		// Act
		// Assert: the infallible build API reports the rendering error through a panic.
		builder.build();
	}

	// ==========================================
	// Error Handling Tests - Panics Replaced with Result Errors
	// ==========================================

	#[rstest]
	#[test]
	fn test_sqlite_empty_conflict_columns_returns_error() {
		// Arrange
		let backend: Arc<dyn DatabaseBackend> = Arc::new(MockSqliteBackend);
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict_do_update(
				Some(vec![]), // empty conflict columns
				vec!["name".to_string()],
			);

		// Act
		let result = builder.build();

		// Assert: Should return error instead of panicking
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert_eq!(err.database_kind(), Some(DatabaseErrorKind::Syntax));
	}

	#[rstest]
	#[test]
	fn test_sqlite_empty_update_columns_returns_error() {
		// Arrange
		let backend: Arc<dyn DatabaseBackend> = Arc::new(MockSqliteBackend);
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict_do_update(
				Some(vec!["email".to_string()]),
				vec![], // empty update columns
			);

		// Act
		let result = builder.build();

		// Assert: Should return error instead of panicking
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert_eq!(err.database_kind(), Some(DatabaseErrorKind::Syntax));
	}

	#[rstest]
	#[test]
	fn test_sqlite_constraint_target_returns_error() {
		// Arrange
		let backend: Arc<dyn DatabaseBackend> = Arc::new(MockSqliteBackend);
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(OnConflictClause::constraint("users_email_key").do_update(vec!["name"]));

		// Act
		let result = builder.build();

		// Assert: Should return NotSupported error instead of panicking
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert_eq!(err.database_kind(), Some(DatabaseErrorKind::Unsupported));
	}

	#[rstest]
	#[test]
	fn test_sqlite_empty_conflict_columns_new_api_returns_error() {
		// Arrange
		let backend: Arc<dyn DatabaseBackend> = Arc::new(MockSqliteBackend);
		let empty_cols: Vec<String> = vec![];
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(OnConflictClause::columns(empty_cols).do_update(vec!["name"]));

		// Act
		let result = builder.build();

		// Assert: Should return error instead of panicking
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert_eq!(err.database_kind(), Some(DatabaseErrorKind::Syntax));
	}

	#[rstest]
	#[test]
	fn test_sqlite_empty_update_columns_new_api_returns_error() {
		// Arrange
		let backend: Arc<dyn DatabaseBackend> = Arc::new(MockSqliteBackend);
		let empty_update: Vec<String> = vec![];
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(OnConflictClause::columns(vec!["email"]).do_update(empty_update));

		// Act
		let result = builder.build();

		// Assert: Should return error instead of panicking
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert_eq!(err.database_kind(), Some(DatabaseErrorKind::Syntax));
	}

	#[rstest]
	#[test]
	fn test_postgres_build_succeeds_with_valid_input() {
		// Arrange
		let backend: Arc<dyn DatabaseBackend> = Arc::new(MockBackend);
		let builder = InsertBuilder::new(backend, "users")
			.value("email", QueryValue::String("test@example.com".to_string()))
			.on_conflict(OnConflictClause::columns(vec!["email"]).do_update(vec!["name"]));

		// Act
		let result = builder.build();

		// Assert: Should succeed for PostgreSQL
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_insert_build_succeeds_with_matching_columns_and_values() {
		// Arrange
		let backend: Arc<dyn DatabaseBackend> = Arc::new(MockBackend);
		let builder = InsertBuilder::new(backend, "users")
			.value("name", QueryValue::String("Alice".to_string()))
			.value("email", QueryValue::String("alice@example.com".to_string()));

		// Act
		let result = builder.build();

		// Assert
		assert!(result.is_ok());
		let (sql, params) = result.unwrap();
		assert!(sql.contains("\"name\""));
		assert!(sql.contains("\"email\""));
		assert_eq!(params.len(), 2);
	}

	#[rstest]
	fn test_insert_build_returns_error_on_column_value_mismatch() {
		// Arrange: manually create a mismatch between columns and values
		let backend: Arc<dyn DatabaseBackend> = Arc::new(MockBackend);
		let mut builder = InsertBuilder::new(backend, "users");
		builder.columns = vec!["name".to_string(), "email".to_string()];
		// Only one value for two columns
		builder.values = vec![QueryValue::String("Alice".to_string())];

		// Act
		let result = builder.build();

		// Assert: should return an error, not panic
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert_eq!(err.database_kind(), Some(DatabaseErrorKind::Query));
	}

	// =========================================================================
	// Issue #2558: SelectBuilder::build() returns parameterized SQL
	// =========================================================================

	#[rstest]
	fn test_select_builder_uses_parameterized_sql() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act
		let builder = SelectBuilder::new(backend)
			.from("users")
			.where_eq("id", QueryValue::Int(42));
		let (sql, params) = builder.build();

		// Assert - SQL should contain placeholder $1, not inlined value 42
		assert_eq!(sql, "SELECT * FROM \"users\" WHERE \"id\" = $1");
		assert_eq!(params.len(), 1);
		assert!(matches!(params[0], QueryValue::Int(42)));
	}

	#[rstest]
	fn test_select_builder_parameterized_multiple_wheres() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act
		let builder = SelectBuilder::new(backend)
			.columns(vec!["id", "name"])
			.from("users")
			.where_eq("status", QueryValue::String("active".to_string()))
			.where_eq("age", QueryValue::Int(18));
		let (sql, params) = builder.build();

		// Assert - parameterized with $1 and $2
		assert_eq!(
			sql,
			"SELECT \"id\", \"name\" FROM \"users\" WHERE \"status\" = $1 AND \"age\" = $2"
		);
		assert_eq!(params.len(), 2);
	}

	#[rstest]
	fn test_select_builder_no_where_no_params() {
		// Arrange
		let backend = Arc::new(MockBackend);

		// Act
		let builder = SelectBuilder::new(backend).from("users");
		let (sql, params) = builder.build();

		// Assert
		assert_eq!(sql, "SELECT * FROM \"users\"");
		assert!(params.is_empty());
	}
}
