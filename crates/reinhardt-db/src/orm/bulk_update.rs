//! Bulk Update Operations with Hybrid Property Support
//!
//! This module provides bulk update operations that integrate with hybrid properties.
//! Based on SQLAlchemy's bulk update functionality.
//!
//! # Identifier quoting
//!
//! The table name and every SET and WHERE column are emitted as ANSI
//! double-quoted identifiers with embedded `"` doubled, so a name can never
//! terminate its identifier and change the statement structure. Each name is
//! quoted as a single identifier. ANSI quoting matches PostgreSQL and SQLite.
//! MySQL reads `"name"` as a string literal unless `ANSI_QUOTES` is enabled in
//! `sql_mode`, so the generated statement is not valid MySQL without it.
//!
//! Quoting does not decide which tables or columns a caller may write.
//! Applications that derive identifiers from untrusted input must still
//! restrict them to an allow-list.

use crate::backends::optimization::QuoteStyle;
use crate::hybrid::HybridProperty;
use std::collections::HashMap;

/// Strategy for synchronizing session state after bulk update
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynchronizeStrategy {
	/// Evaluate changes using Python/Rust code (instance-level)
	Evaluate,
	/// Fetch fresh data from database (SQL-level)
	Fetch,
	/// Don't synchronize (fastest but potentially stale)
	False,
}

/// Bulk update builder for ORM operations
pub struct BulkUpdateBuilder {
	table_name: String,
	updates: HashMap<String, String>,
	where_clause: Option<(String, String)>,
	synchronize: SynchronizeStrategy,
}

impl BulkUpdateBuilder {
	/// Create a new bulk update builder for efficient batch updates
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::bulk_update::BulkUpdateBuilder;
	///
	/// let builder = BulkUpdateBuilder::new("users");
	/// // Can chain: .set().where_clause().build()
	/// ```
	pub fn new(table_name: &str) -> Self {
		Self {
			table_name: table_name.to_string(),
			updates: HashMap::new(),
			where_clause: None,
			synchronize: SynchronizeStrategy::Evaluate,
		}
	}
	/// Set a regular column value
	///
	pub fn set(mut self, column: &str, value: &str) -> Self {
		self.updates.insert(column.to_string(), value.to_string());
		self
	}
	/// Set a hybrid property value (direct property, e.g., fname -> first_name)
	///
	pub fn set_hybrid<T, R>(
		mut self,
		column: &str,
		_property: &HybridProperty<T, R>,
		value: &str,
	) -> Self {
		// For direct hybrid properties, just set the underlying column
		self.updates.insert(column.to_string(), value.to_string());
		self
	}
	/// Set a hybrid property with update expression (e.g., name -> first_name, last_name)
	///
	pub fn set_hybrid_expanded(mut self, updates: Vec<(&str, &str)>) -> Self {
		for (col, val) in updates {
			self.updates.insert(col.to_string(), val.to_string());
		}
		self
	}
	/// Set a parameterized WHERE clause using column equality.
	///
	/// Uses `column = ?` with a bind parameter to prevent SQL injection.
	pub fn where_clause(mut self, column: &str, value: &str) -> Self {
		self.where_clause = Some((column.to_string(), value.to_string()));
		self
	}
	/// Set synchronization strategy
	///
	pub fn synchronize(mut self, strategy: SynchronizeStrategy) -> Self {
		self.synchronize = strategy;
		self
	}
	/// Build the SQL UPDATE statement
	///
	pub fn build(&self) -> (String, Vec<String>, SynchronizeStrategy) {
		let mut set_clauses = Vec::new();
		let mut params = Vec::new();
		let quote = QuoteStyle::Ansi;

		// Build SET clauses with quoted column names
		for (col, val) in &self.updates {
			set_clauses.push(format!("{}=?", quote.quote_identifier(col)));
			params.push(val.clone());
		}

		let mut sql = format!(
			"UPDATE {} SET {}",
			quote.quote_identifier(&self.table_name),
			set_clauses.join(", ")
		);

		if let Some((column, value)) = &self.where_clause {
			sql.push_str(&format!(" WHERE {}=?", quote.quote_identifier(column)));
			params.push(value.clone());
		}

		(sql, params, self.synchronize)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[test]
	fn test_bulk_update_plain() {
		let builder = BulkUpdateBuilder::new("person").set("first_name", "Dr.");

		let (sql, params, _) = builder.build();
		assert!(sql.contains("UPDATE \"person\""));
		assert!(sql.contains("\"first_name\"=?"));
		assert_eq!(params, vec!["Dr."]);
	}

	#[test]
	fn test_bulk_update_with_where() {
		let builder = BulkUpdateBuilder::new("person")
			.set("first_name", "Dr.")
			.where_clause("id", "3");

		let (sql, params, _) = builder.build();
		assert!(sql.contains("UPDATE \"person\""));
		assert!(sql.contains("\"first_name\"=?"));
		assert!(sql.contains("WHERE \"id\"=?"));
		assert_eq!(params, vec!["Dr.", "3"]);
	}

	#[test]
	fn test_bulk_update_expanded() {
		let builder = BulkUpdateBuilder::new("person")
			.set_hybrid_expanded(vec![("first_name", "Dr."), ("last_name", "No")]);

		let (sql, params, _) = builder.build();
		assert!(sql.contains("UPDATE \"person\" SET"));
		assert!(sql.contains("\"first_name\"=?"));
		assert!(sql.contains("\"last_name\"=?"));
		assert_eq!(params.len(), 2);
	}

	#[test]
	fn test_synchronize_strategy() {
		let builder = BulkUpdateBuilder::new("person")
			.set("first_name", "Dr.")
			.synchronize(SynchronizeStrategy::Fetch);

		let (_, _, strategy) = builder.build();
		assert_eq!(strategy, SynchronizeStrategy::Fetch);
	}

	#[rstest]
	#[case::benign_identifiers(
		"person",
		"first_name",
		"id",
		"UPDATE \"person\" SET \"first_name\"=? WHERE \"id\"=?"
	)]
	#[case::table_name_breakout(
		"person\" SET \"is_admin\"=TRUE --",
		"first_name",
		"id",
		"UPDATE \"person\"\" SET \"\"is_admin\"\"=TRUE --\" SET \"first_name\"=? WHERE \"id\"=?"
	)]
	#[case::set_column_breakout(
		"person",
		"is_admin\"=TRUE, \"first_name",
		"id",
		"UPDATE \"person\" SET \"is_admin\"\"=TRUE, \"\"first_name\"=? WHERE \"id\"=?"
	)]
	#[case::where_column_breakout(
		"person",
		"first_name",
		"id\"=? OR 1=1 --",
		"UPDATE \"person\" SET \"first_name\"=? WHERE \"id\"\"=? OR 1=1 --\"=?"
	)]
	fn test_bulk_update_quotes_identifiers(
		#[case] table: &str,
		#[case] column: &str,
		#[case] where_column: &str,
		#[case] expected_sql: &str,
	) {
		// Arrange
		let builder = BulkUpdateBuilder::new(table)
			.set(column, "Dr.")
			.where_clause(where_column, "3");

		// Act
		let (sql, params, _) = builder.build();

		// Assert
		assert_eq!(sql, expected_sql);
		assert_eq!(params, vec!["Dr.".to_string(), "3".to_string()]);
	}

	#[rstest]
	fn test_bulk_update_quotes_expanded_columns() {
		// Arrange
		let builder = BulkUpdateBuilder::new("person")
			.set_hybrid_expanded(vec![("first_name\"=NULL, \"last_name", "Dr.")]);

		// Act
		let (sql, params, _) = builder.build();

		// Assert
		assert_eq!(
			sql,
			"UPDATE \"person\" SET \"first_name\"\"=NULL, \"\"last_name\"=?"
		);
		assert_eq!(params, vec!["Dr.".to_string()]);
	}
}
