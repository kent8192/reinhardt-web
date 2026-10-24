//! ON CONFLICT clause for INSERT statements.
//!
//! This module provides the [`OnConflict`] builder for constructing
//! ON CONFLICT clauses used in upsert operations.

use crate::{
	expr::SimpleExpr,
	types::{BinOper, DynIden, IntoIden},
};

/// Target for ON CONFLICT clause.
#[derive(Debug, Clone)]
pub enum OnConflictTarget {
	/// Single column target
	Column(DynIden),
	/// Multiple columns target
	///
	/// An empty vector represents an omitted conflict target.
	Columns(Vec<DynIden>),
}

/// Action for ON CONFLICT clause.
#[derive(Debug, Clone)]
pub enum OnConflictAction {
	/// DO NOTHING - skip the conflicting row
	DoNothing,
	/// DO UPDATE SET - update specified columns
	DoUpdate(Vec<DynIden>),
}

/// ON CONFLICT clause builder for INSERT statements.
///
/// # Examples
///
/// ```rust
/// use reinhardt_query::query::OnConflict;
///
/// // INSERT ... ON CONFLICT (id) DO NOTHING
/// let on_conflict = OnConflict::column("id").do_nothing();
///
/// // INSERT ... ON CONFLICT (id) DO UPDATE SET name, email
/// let on_conflict = OnConflict::column("id")
///     .update_columns(["name", "email"]);
/// ```
#[derive(Debug, Clone)]
pub struct OnConflict {
	pub(crate) target: OnConflictTarget,
	pub(crate) action: OnConflictAction,
	pub(crate) constraint: Option<DynIden>,
	pub(crate) action_condition: Option<Box<SimpleExpr>>,
}

impl Default for OnConflict {
	fn default() -> Self {
		Self::new()
	}
}

impl OnConflict {
	/// Create a targetless ON CONFLICT DO NOTHING clause.
	///
	/// PostgreSQL and SQLite render `ON CONFLICT DO NOTHING`, handling conflicts
	/// on any usable unique constraint or index. MySQL uses its existing no-op
	/// `ON DUPLICATE KEY UPDATE` emulation and requires an insert column.
	///
	/// **P2 (behavioral parity):** This builder behaves identically on native and
	/// WASM targets.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_query::{OnConflict, PostgresQueryBuilder, Query, QueryStatementBuilder};
	///
	/// let sql = Query::insert()
	///     .into_table("users")
	///     .columns(["id"])
	///     .values_panic([1])
	///     .on_conflict(OnConflict::new().do_nothing())
	///     .to_string(PostgresQueryBuilder);
	/// assert_eq!(sql, "INSERT INTO \"users\" (\"id\") VALUES (1) ON CONFLICT DO NOTHING");
	/// ```
	#[must_use]
	pub fn new() -> Self {
		Self {
			target: OnConflictTarget::Columns(Vec::new()),
			action: OnConflictAction::DoNothing,
			constraint: None,
			action_condition: None,
		}
	}

	/// Create an ON CONFLICT clause targeting a single column.
	pub fn column<C: IntoIden>(col: C) -> Self {
		Self {
			target: OnConflictTarget::Column(col.into_iden()),
			action: OnConflictAction::DoNothing,
			constraint: None,
			action_condition: None,
		}
	}

	/// Create an ON CONFLICT clause targeting multiple columns.
	///
	/// An empty iterator omits the target, equivalent to [`Self::new`].
	pub fn columns<I, C>(cols: I) -> Self
	where
		I: IntoIterator<Item = C>,
		C: IntoIden,
	{
		Self {
			target: OnConflictTarget::Columns(cols.into_iter().map(|c| c.into_iden()).collect()),
			action: OnConflictAction::DoNothing,
			constraint: None,
			action_condition: None,
		}
	}

	/// Target a named unique constraint in PostgreSQL or CockroachDB.
	///
	/// Checked builders reject this target in MySQL and SQLite. The identifier
	/// is quoted by the selected backend. Native and WASM behavior is identical.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_query::{OnConflict, PostgresQueryBuilder, Query, QueryStatementBuilder};
	/// let sql = Query::insert().into_table("users").columns(["id"])
	///     .values_panic([1])
	///     .on_conflict(OnConflict::constraint("users_pkey").update_columns(["id"]))
	///     .to_string(PostgresQueryBuilder);
	/// assert!(sql.contains("ON CONFLICT ON CONSTRAINT \"users_pkey\" DO UPDATE"));
	/// ```
	#[must_use]
	pub fn constraint<C: IntoIden>(name: C) -> Self {
		Self {
			constraint: Some(name.into_iden()),
			..Self::new()
		}
	}

	/// Add a typed condition to the DO UPDATE action.
	///
	/// Repeated calls combine conditions with AND. PostgreSQL, CockroachDB and
	/// SQLite support this clause; checked MySQL builds reject it. A condition
	/// on DO NOTHING is invalid. Calling [`Self::do_nothing`] clears conditions.
	/// Bound condition values follow insert and subquery values in argument order.
	/// Native and WASM behavior is identical.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_query::{Expr, ExprTrait, OnConflict, PostgresQueryBuilder, Query};
	/// let statement = Query::insert().into_table("users").columns(["id"])
	///     .values_panic([1])
	///     .on_conflict(OnConflict::column("id").update_columns(["id"])
	///         .action_and_where(Expr::col("id").gt(0)))
	///     .to_owned();
	/// let (sql, values) = PostgresQueryBuilder.build_insert_checked(&statement).unwrap();
	/// assert!(sql.ends_with("WHERE \"id\" > $2"));
	/// assert_eq!(values.0, vec![1.into(), 0.into()]);
	/// ```
	#[must_use]
	pub fn action_and_where<E: Into<SimpleExpr>>(mut self, condition: E) -> Self {
		let condition = condition.into();
		self.action_condition = Some(Box::new(match self.action_condition.take() {
			Some(previous) => SimpleExpr::Binary(previous, BinOper::And, Box::new(condition)),
			None => condition,
		}));
		self
	}

	/// Set the action to DO NOTHING.
	#[must_use]
	pub fn do_nothing(mut self) -> Self {
		self.action = OnConflictAction::DoNothing;
		self.action_condition = None;
		self
	}

	/// Set the action to DO UPDATE SET with specified columns.
	///
	/// PostgreSQL requires a nonempty conflict target for this action and panics
	/// during rendering if the target is omitted. SQLite 3.35 and later and MySQL
	/// support targetless updates.
	#[must_use]
	pub fn update_columns<I, C>(mut self, cols: I) -> Self
	where
		I: IntoIterator<Item = C>,
		C: IntoIden,
	{
		self.action = OnConflictAction::DoUpdate(cols.into_iter().map(|c| c.into_iden()).collect());
		self
	}

	/// Consume and return self (for compatibility with reinhardt-query API pattern).
	#[must_use]
	pub fn to_owned(self) -> Self {
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_on_conflict_column_do_nothing() {
		// Arrange & Act
		let on_conflict = OnConflict::column("id").do_nothing();

		// Assert
		assert!(matches!(on_conflict.target, OnConflictTarget::Column(_)));
		assert!(matches!(on_conflict.action, OnConflictAction::DoNothing));
	}

	#[test]
	fn test_on_conflict_column_update_columns() {
		// Arrange & Act
		let on_conflict = OnConflict::column("id").update_columns(["name", "email"]);

		// Assert
		assert!(matches!(on_conflict.target, OnConflictTarget::Column(_)));
		match &on_conflict.action {
			OnConflictAction::DoUpdate(cols) => {
				assert_eq!(cols.len(), 2);
			}
			_ => panic!("Expected DoUpdate action"),
		}
	}

	#[test]
	fn test_on_conflict_columns_do_nothing() {
		// Arrange & Act
		let on_conflict = OnConflict::columns(["id", "tenant_id"]).do_nothing();

		// Assert
		match &on_conflict.target {
			OnConflictTarget::Columns(cols) => {
				assert_eq!(cols.len(), 2);
			}
			_ => panic!("Expected Columns target"),
		}
		assert!(matches!(on_conflict.action, OnConflictAction::DoNothing));
	}

	#[test]
	fn test_on_conflict_columns_update_columns() {
		// Arrange & Act
		let on_conflict =
			OnConflict::columns(["id", "tenant_id"]).update_columns(["name", "email", "age"]);

		// Assert
		match &on_conflict.target {
			OnConflictTarget::Columns(cols) => {
				assert_eq!(cols.len(), 2);
			}
			_ => panic!("Expected Columns target"),
		}
		match &on_conflict.action {
			OnConflictAction::DoUpdate(cols) => {
				assert_eq!(cols.len(), 3);
			}
			_ => panic!("Expected DoUpdate action"),
		}
	}

	#[test]
	fn test_on_conflict_to_owned() {
		// Arrange & Act
		let on_conflict = OnConflict::column("id").do_nothing().to_owned();

		// Assert
		assert!(matches!(on_conflict.target, OnConflictTarget::Column(_)));
		assert!(matches!(on_conflict.action, OnConflictAction::DoNothing));
	}

	#[test]
	fn test_on_conflict_default_action_is_do_nothing() {
		// Arrange & Act
		let on_conflict = OnConflict::column("id");

		// Assert
		assert!(matches!(on_conflict.action, OnConflictAction::DoNothing));
	}
}
