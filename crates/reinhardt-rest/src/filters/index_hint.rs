//! Database index hint system for query optimization
//!
//! Provides intelligent index usage hints to optimize database query performance.
//!
//! # Examples
//!
//! ```
//! use crate::filters::{FilterBackend, IndexHintFilter, IndexStrategy, DatabaseType};
//! use std::collections::HashMap;
//!
//! # async fn example() {
//! // Create filter with index hints for MySQL
//! let filter = IndexHintFilter::for_database(DatabaseType::MySQL)
//!     .with_index("idx_users_email", IndexStrategy::Use)
//!     .with_index("idx_users_created_at", IndexStrategy::Force);
//!
//! let params = HashMap::new();
//! let sql = "SELECT * FROM users WHERE email = 'test@example.com'".to_string();
//! let result = filter.filter_queryset(&params, sql).await;
//! // Verify the filter backend processes the query successfully
//! assert!(result.is_ok());
//! # }
//! ```

use super::{FilterBackend, FilterResult};
use super::optimizer::DatabaseType;
use async_trait::async_trait;
use regex::Regex;
use std::collections::HashMap;

/// Index usage strategy
///
/// Specifies how the database should use the suggested index.
///
/// # Examples
///
/// ```
/// use crate::filters::IndexStrategy;
///
/// let strategy = IndexStrategy::Use;
/// let force_strategy = IndexStrategy::Force;
/// let ignore_strategy = IndexStrategy::Ignore;
/// // Verify strategies are created successfully
/// let _: IndexStrategy = strategy;
/// let _: IndexStrategy = force_strategy;
/// let _: IndexStrategy = ignore_strategy;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexStrategy {
	/// Suggest using the index (USE INDEX hint)
	///
	/// The database optimizer may choose to use the index or not.
	Use,

	/// Force using the index (FORCE INDEX hint)
	///
	/// The database optimizer will strongly prefer this index.
	Force,

	/// Ignore the index (IGNORE INDEX hint)
	///
	/// The database optimizer will not use this index.
	Ignore,
}

/// Configuration for an index hint
///
/// # Examples
///
/// ```
/// use crate::filters::{IndexHint, IndexStrategy};
///
/// let hint = IndexHint::new("idx_users_email", IndexStrategy::Use);
/// // Verify the index hint is created successfully
/// assert_eq!(hint.index_name, "idx_users_email");
/// ```
#[derive(Debug, Clone)]
pub struct IndexHint {
	/// Name of the index
	pub index_name: String,

	/// Strategy for using the index
	pub strategy: IndexStrategy,

	/// Table name (optional, for multi-table queries)
	pub table_name: Option<String>,
}

impl IndexHint {
	/// Create a new index hint
	///
	/// # Arguments
	///
	/// * `index_name` - Name of the database index
	/// * `strategy` - Strategy for using the index
	///
	/// # Examples
	///
	/// ```
	/// use crate::filters::{IndexHint, IndexStrategy};
	///
	/// let hint = IndexHint::new("idx_users_email", IndexStrategy::Use);
	/// // Verify the hint is created successfully
	/// assert_eq!(hint.index_name, "idx_users_email");
	/// assert_eq!(hint.strategy, IndexStrategy::Use);
	/// ```
	pub fn new(index_name: impl Into<String>, strategy: IndexStrategy) -> Self {
		Self {
			index_name: index_name.into(),
			strategy,
			table_name: None,
		}
	}

	/// Specify the table name for this index hint
	///
	/// Useful for multi-table queries where index names might be ambiguous.
	///
	/// # Examples
	///
	/// ```
	/// use crate::filters::{IndexHint, IndexStrategy};
	///
	/// let hint = IndexHint::new("idx_email", IndexStrategy::Use)
	///     .for_table("users");
	/// // Verify the table name is set
	/// assert_eq!(hint.table_name, Some("users".to_string()));
	/// ```
	pub fn for_table(mut self, table_name: impl Into<String>) -> Self {
		self.table_name = Some(table_name.into());
		self
	}

	/// Generate SQL hint clause for this index
	///
	/// # Examples
	///
	/// ```
	/// use crate::filters::{IndexHint, IndexStrategy, DatabaseType};
	///
	/// let hint = IndexHint::new("idx_users_email", IndexStrategy::Use);
	/// let mysql_sql = hint.to_sql_hint(DatabaseType::MySQL);
	/// // Verify MySQL hint is generated correctly
	/// assert!(mysql_sql.contains("USE INDEX"));
	///
	/// let sqlite_sql = hint.to_sql_hint(DatabaseType::SQLite);
	/// // Verify SQLite hint is generated correctly
	/// assert!(sqlite_sql.contains("INDEXED BY"));
	/// ```
	pub fn to_sql_hint(&self, db_type: DatabaseType) -> String {
		match db_type {
			DatabaseType::MySQL => self.to_mysql_hint(),
			DatabaseType::SQLite => self.to_sqlite_hint(),
			DatabaseType::PostgreSQL => self.to_postgresql_hint(),
		}
	}

	/// Generate MySQL-specific index hint
	fn to_mysql_hint(&self) -> String {
		let hint_type = match self.strategy {
			IndexStrategy::Use => "USE INDEX",
			IndexStrategy::Force => "FORCE INDEX",
			IndexStrategy::Ignore => "IGNORE INDEX",
		};

		format!("{} ({})", hint_type, self.index_name)
	}

	/// Generate SQLite-specific index hint
	fn to_sqlite_hint(&self) -> String {
		match self.strategy {
			IndexStrategy::Use | IndexStrategy::Force => {
				format!("INDEXED BY {}", self.index_name)
			}
			IndexStrategy::Ignore => "NOT INDEXED".to_string(),
		}
	}

	/// Generate PostgreSQL-specific index hint (comment-based)
	fn to_postgresql_hint(&self) -> String {
		let hint_type = match self.strategy {
			IndexStrategy::Use => "USE",
			IndexStrategy::Force => "FORCE",
			IndexStrategy::Ignore => "IGNORE",
		};

		format!("/* {} INDEX: {} */", hint_type, self.index_name)
	}
}

/// Filter backend that adds database index hints to optimize query performance
///
/// This filter helps optimize database queries by suggesting which indexes
/// the query planner should use, force, or ignore.
///
/// # Database Compatibility
///
/// - **MySQL/MariaDB**: Native index hints (USE INDEX, FORCE INDEX, IGNORE INDEX)
/// - **SQLite**: INDEXED BY clause and NOT INDEXED
/// - **PostgreSQL**: Comment-based hints (for documentation only)
///
/// # Examples
///
/// ```
/// use crate::filters::{FilterBackend, IndexHintFilter, IndexStrategy, DatabaseType};
/// use std::collections::HashMap;
///
/// # async fn example() {
/// // MySQL example
/// let mysql_filter = IndexHintFilter::for_database(DatabaseType::MySQL)
///     .with_index("idx_users_email", IndexStrategy::Use)
///     .with_index("idx_users_created_at", IndexStrategy::Force);
///
/// // SQLite example
/// let sqlite_filter = IndexHintFilter::for_database(DatabaseType::SQLite)
///     .with_index("idx_users_email", IndexStrategy::Use);
///
/// let params = HashMap::new();
/// let sql = "SELECT * FROM users WHERE email = 'test@example.com'".to_string();
/// let result = mysql_filter.filter_queryset(&params, sql).await;
/// // Verify the filter backend processes the query successfully
/// assert!(result.is_ok());
/// # }
/// ```
#[derive(Debug)]
pub struct IndexHintFilter {
	hints: Vec<IndexHint>,
	enabled: bool,
	db_type: DatabaseType,
}

impl Default for IndexHintFilter {
	fn default() -> Self {
		Self::new()
	}
}

impl IndexHintFilter {
	/// Create a new index hint filter with MySQL as default database
	///
	/// # Examples
	///
	/// ```
	/// use crate::filters::IndexHintFilter;
	///
	/// let filter = IndexHintFilter::new();
	/// // Verify the filter is created successfully
	/// let _: IndexHintFilter = filter;
	/// ```
	pub fn new() -> Self {
		Self {
			hints: Vec::new(),
			enabled: true,
			db_type: DatabaseType::MySQL,
		}
	}

	/// Create an index hint filter for a specific database type
	///
	/// # Examples
	///
	/// ```
	/// use crate::filters::{IndexHintFilter, DatabaseType};
	///
	/// let mysql_filter = IndexHintFilter::for_database(DatabaseType::MySQL);
	/// let sqlite_filter = IndexHintFilter::for_database(DatabaseType::SQLite);
	/// let pg_filter = IndexHintFilter::for_database(DatabaseType::PostgreSQL);
	/// // Verify filters are created for each database type
	/// let _: IndexHintFilter = mysql_filter;
	/// let _: IndexHintFilter = sqlite_filter;
	/// let _: IndexHintFilter = pg_filter;
	/// ```
	pub fn for_database(db_type: DatabaseType) -> Self {
		Self {
			hints: Vec::new(),
			enabled: true,
			db_type,
		}
	}

	/// Add an index hint to the filter
	///
	/// # Arguments
	///
	/// * `index_name` - Name of the database index
	/// * `strategy` - Strategy for using the index
	///
	/// # Examples
	///
	/// ```
	/// use crate::filters::{IndexHintFilter, IndexStrategy};
	///
	/// let filter = IndexHintFilter::new()
	///     .with_index("idx_users_email", IndexStrategy::Use)
	///     .with_index("idx_users_created_at", IndexStrategy::Force);
	/// // Verify the filter is configured with hints
	/// let _: IndexHintFilter = filter;
	/// ```
	pub fn with_index(mut self, index_name: impl Into<String>, strategy: IndexStrategy) -> Self {
		self.hints.push(IndexHint::new(index_name, strategy));
		self
	}

	/// Add a custom index hint
	///
	/// # Examples
	///
	/// ```
	/// use crate::filters::{IndexHintFilter, IndexHint, IndexStrategy};
	///
	/// let hint = IndexHint::new("idx_email", IndexStrategy::Use)
	///     .for_table("users");
	///
	/// let filter = IndexHintFilter::new()
	///     .with_hint(hint);
	/// // Verify the filter is configured with the custom hint
	/// let _: IndexHintFilter = filter;
	/// ```
	pub fn with_hint(mut self, hint: IndexHint) -> Self {
		self.hints.push(hint);
		self
	}

	/// Enable or disable index hints
	///
	/// When disabled, hints are not applied to queries.
	///
	/// # Examples
	///
	/// ```
	/// use crate::filters::{IndexHintFilter, IndexStrategy};
	///
	/// let filter = IndexHintFilter::new()
	///     .with_index("idx_users_email", IndexStrategy::Use)
	///     .set_enabled(false);  // Temporarily disable hints
	/// // Verify the filter is configured with hints disabled
	/// let _: IndexHintFilter = filter;
	/// ```
	pub fn set_enabled(mut self, enabled: bool) -> Self {
		self.enabled = enabled;
		self
	}

	/// Apply index hints to SQL query
	///
	/// Parses SQL and injects database-specific index hints after table names
	/// in FROM and JOIN clauses.
	fn apply_hints(&self, sql: String) -> FilterResult<String> {
		if !self.enabled || self.hints.is_empty() {
			return Ok(sql);
		}

		match self.db_type {
			DatabaseType::MySQL => self.apply_mysql_hints(sql),
			DatabaseType::SQLite => self.apply_sqlite_hints(sql),
			DatabaseType::PostgreSQL => self.apply_postgresql_hints(sql),
		}
	}

	/// Apply MySQL-specific index hints
	///
	/// Injects hints after table names in FROM and JOIN clauses.
	/// Example: `FROM users` -> `FROM users USE INDEX (idx_email)`
	fn apply_mysql_hints(&self, sql: String) -> FilterResult<String> {
		self.apply_hints_internal(sql, DatabaseType::MySQL)
	}

	/// Apply SQLite-specific index hints
	///
	/// Injects hints after table names in FROM and JOIN clauses.
	/// Example: `FROM users` -> `FROM users INDEXED BY idx_email`
	fn apply_sqlite_hints(&self, sql: String) -> FilterResult<String> {
		self.apply_hints_internal(sql, DatabaseType::SQLite)
	}

	/// Internal method to apply hints with proper table matching
	fn apply_hints_internal(&self, sql: String, db_type: DatabaseType) -> FilterResult<String> {
		// Find all table references in the SQL
		let table_regex = Regex::new(r"(?i)\b(FROM|JOIN)\s+(\w+)\b").map_err(|e| {
			super::FilterError::InvalidQuery(format!("Invalid regex: {}", e))
		})?;

		let mut table_positions: Vec<(usize, String, String)> = Vec::new(); // (position, keyword, table_name)

		for caps in table_regex.captures_iter(&sql) {
			if let Some(pos) = caps.get(0) {
				let keyword = caps[1].to_string();
				let table_name = caps[2].to_string();
				table_positions.push((pos.end(), keyword, table_name));
			}
		}

		// Build a map of table name to hints
		let mut table_hints: HashMap<String, Vec<String>> = HashMap::new();
		let mut unassigned_hints: Vec<String> = Vec::new();

		for hint in &self.hints {
			let hint_sql = hint.to_sql_hint(db_type);
			if let Some(ref table_name) = hint.table_name {
				table_hints
					.entry(table_name.clone())
					.or_default()
					.push(hint_sql);
			} else {
				unassigned_hints.push(hint_sql);
			}
		}

		// Apply hints by reconstructing the SQL
		let mut result = String::new();
		let mut last_pos = 0;
		let mut unassigned_idx = 0;

		for (end_pos, keyword, table_name) in table_positions {
			// Add the part before this match
			let start_match = end_pos - keyword.len() - table_name.len() - 1;
			result.push_str(&sql[last_pos..start_match]);

			// Add the keyword and table name
			result.push_str(&keyword);
			result.push(' ');
			result.push_str(&table_name);

			// Add table-specific hints if any
			if let Some(hints) = table_hints.get(&table_name.to_lowercase()) {
				for hint_sql in hints {
					result.push(' ');
					result.push_str(hint_sql);
				}
			}
			// Otherwise, use next unassigned hint if available
			else if unassigned_idx < unassigned_hints.len() {
				result.push(' ');
				result.push_str(&unassigned_hints[unassigned_idx]);
				unassigned_idx += 1;
			}

			last_pos = end_pos;
		}

		// Add remaining SQL
		result.push_str(&sql[last_pos..]);

		Ok(result)
	}

	/// Apply PostgreSQL-specific index hints (as comments)
	///
	/// PostgreSQL doesn't support native index hints, so we add them as comments
	/// for documentation purposes. The actual optimization would require
	/// session-level SET commands.
	fn apply_postgresql_hints(&self, sql: String) -> FilterResult<String> {
		let mut result = sql;

		for hint in &self.hints {
			let hint_sql = hint.to_sql_hint(DatabaseType::PostgreSQL);
			// For PostgreSQL, we just prepend the comment
			result = format!("{} {}", hint_sql, result);
		}

		Ok(result)
	}
}

#[async_trait]
impl FilterBackend for IndexHintFilter {
	async fn filter_queryset(
		&self,
		_query_params: &HashMap<String, String>,
		sql: String,
	) -> FilterResult<String> {
		self.apply_hints(sql)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_index_strategy_variants() {
		let strategies = [
			IndexStrategy::Use,
			IndexStrategy::Force,
			IndexStrategy::Ignore,
		];
		assert_eq!(strategies.len(), 3);
	}

	#[test]
	fn test_index_hint_creation() {
		let hint = IndexHint::new("idx_users_email", IndexStrategy::Use);
		assert_eq!(hint.index_name, "idx_users_email");
		assert_eq!(hint.strategy, IndexStrategy::Use);
		assert!(hint.table_name.is_none());
	}

	#[test]
	fn test_index_hint_with_table() {
		let hint = IndexHint::new("idx_email", IndexStrategy::Force).for_table("users");
		assert_eq!(hint.index_name, "idx_email");
		assert_eq!(hint.strategy, IndexStrategy::Force);
		assert_eq!(hint.table_name, Some("users".to_string()));
	}

	#[test]
	fn test_index_hint_to_sql_mysql_use() {
		let hint = IndexHint::new("idx_users_email", IndexStrategy::Use);
		let sql = hint.to_sql_hint(DatabaseType::MySQL);
		assert_eq!(sql, "USE INDEX (idx_users_email)");
	}

	#[test]
	fn test_index_hint_to_sql_mysql_force() {
		let hint = IndexHint::new("idx_users_created_at", IndexStrategy::Force);
		let sql = hint.to_sql_hint(DatabaseType::MySQL);
		assert_eq!(sql, "FORCE INDEX (idx_users_created_at)");
	}

	#[test]
	fn test_index_hint_to_sql_mysql_ignore() {
		let hint = IndexHint::new("idx_users_status", IndexStrategy::Ignore);
		let sql = hint.to_sql_hint(DatabaseType::MySQL);
		assert_eq!(sql, "IGNORE INDEX (idx_users_status)");
	}

	#[test]
	fn test_index_hint_to_sql_sqlite_use() {
		let hint = IndexHint::new("idx_users_email", IndexStrategy::Use);
		let sql = hint.to_sql_hint(DatabaseType::SQLite);
		assert_eq!(sql, "INDEXED BY idx_users_email");
	}

	#[test]
	fn test_index_hint_to_sql_sqlite_force() {
		let hint = IndexHint::new("idx_users_created_at", IndexStrategy::Force);
		let sql = hint.to_sql_hint(DatabaseType::SQLite);
		assert_eq!(sql, "INDEXED BY idx_users_created_at");
	}

	#[test]
	fn test_index_hint_to_sql_sqlite_ignore() {
		let hint = IndexHint::new("idx_users_status", IndexStrategy::Ignore);
		let sql = hint.to_sql_hint(DatabaseType::SQLite);
		assert_eq!(sql, "NOT INDEXED");
	}

	#[test]
	fn test_index_hint_to_sql_postgresql() {
		let hint = IndexHint::new("idx_users_email", IndexStrategy::Use);
		let sql = hint.to_sql_hint(DatabaseType::PostgreSQL);
		assert_eq!(sql, "/* USE INDEX: idx_users_email */");
	}

	#[test]
	fn test_index_hint_filter_creation() {
		let filter = IndexHintFilter::new();
		assert!(filter.hints.is_empty());
		assert!(filter.enabled);
	}

	#[test]
	fn test_index_hint_filter_with_hints() {
		let filter = IndexHintFilter::new()
			.with_index("idx_users_email", IndexStrategy::Use)
			.with_index("idx_users_created_at", IndexStrategy::Force);

		assert_eq!(filter.hints.len(), 2);
		assert_eq!(filter.hints[0].index_name, "idx_users_email");
		assert_eq!(filter.hints[1].index_name, "idx_users_created_at");
	}

	#[test]
	fn test_index_hint_filter_disable() {
		let filter = IndexHintFilter::new()
			.with_index("idx_users_email", IndexStrategy::Use)
			.set_enabled(false);

		assert!(!filter.enabled);
	}

	#[tokio::test]
	async fn test_index_hint_filter_disabled_passthrough() {
		let filter = IndexHintFilter::new()
			.with_index("idx_users_email", IndexStrategy::Use)
			.set_enabled(false);

		let params = HashMap::new();
		let sql = "SELECT * FROM users".to_string();
		let result = filter.filter_queryset(&params, sql.clone()).await.unwrap();

		assert_eq!(result, sql);
	}

	#[tokio::test]
	async fn test_index_hint_filter_no_hints_passthrough() {
		let filter = IndexHintFilter::new();

		let params = HashMap::new();
		let sql = "SELECT * FROM users".to_string();
		let result = filter.filter_queryset(&params, sql.clone()).await.unwrap();

		assert_eq!(result, sql);
	}

	// MySQL hint injection tests

	#[tokio::test]
	async fn test_mysql_use_index_hint_injection() {
		let filter = IndexHintFilter::for_database(DatabaseType::MySQL)
			.with_index("idx_users_email", IndexStrategy::Use);

		let params = HashMap::new();
		let sql = "SELECT * FROM users WHERE email = 'test@example.com'".to_string();
		let result = filter.filter_queryset(&params, sql).await.unwrap();

		assert!(result.contains("FROM users USE INDEX (idx_users_email)"));
	}

	#[tokio::test]
	async fn test_mysql_force_index_hint_injection() {
		let filter = IndexHintFilter::for_database(DatabaseType::MySQL)
			.with_index("idx_users_id", IndexStrategy::Force);

		let params = HashMap::new();
		let sql = "SELECT * FROM users WHERE id = 1".to_string();
		let result = filter.filter_queryset(&params, sql).await.unwrap();

		assert!(result.contains("FROM users FORCE INDEX (idx_users_id)"));
	}

	#[tokio::test]
	async fn test_mysql_ignore_index_hint_injection() {
		let filter = IndexHintFilter::for_database(DatabaseType::MySQL)
			.with_index("idx_users_status", IndexStrategy::Ignore);

		let params = HashMap::new();
		let sql = "SELECT * FROM users WHERE status = 'active'".to_string();
		let result = filter.filter_queryset(&params, sql).await.unwrap();

		assert!(result.contains("FROM users IGNORE INDEX (idx_users_status)"));
	}

	#[tokio::test]
	async fn test_mysql_multiple_hints() {
		let filter = IndexHintFilter::for_database(DatabaseType::MySQL)
			.with_index("idx_users_email", IndexStrategy::Use)
			.with_index("idx_orders_user_id", IndexStrategy::Force);

		let params = HashMap::new();
		let sql = "SELECT * FROM users JOIN orders ON users.id = orders.user_id".to_string();
		let result = filter.filter_queryset(&params, sql).await.unwrap();

		assert!(result.contains("FROM users USE INDEX (idx_users_email)"));
		assert!(result.contains("JOIN orders FORCE INDEX (idx_orders_user_id)"));
	}

	#[tokio::test]
	async fn test_mysql_hint_with_table_name() {
		let filter = IndexHintFilter::for_database(DatabaseType::MySQL)
			.with_hint(IndexHint::new("idx_email", IndexStrategy::Use).for_table("users"));

		let params = HashMap::new();
		let sql = "SELECT * FROM users JOIN orders ON users.id = orders.user_id".to_string();
		let result = filter.filter_queryset(&params, sql).await.unwrap();

		assert!(result.contains("FROM users USE INDEX (idx_email)"));
		// orders should not have the hint
		assert!(!result.contains("JOIN orders USE INDEX"));
	}

	// SQLite hint injection tests

	#[tokio::test]
	async fn test_sqlite_indexed_by_hint() {
		let filter = IndexHintFilter::for_database(DatabaseType::SQLite)
			.with_index("idx_users_email", IndexStrategy::Use);

		let params = HashMap::new();
		let sql = "SELECT * FROM users WHERE email = 'test@example.com'".to_string();
		let result = filter.filter_queryset(&params, sql).await.unwrap();

		assert!(result.contains("FROM users INDEXED BY idx_users_email"));
	}

	#[tokio::test]
	async fn test_sqlite_indexed_by_force() {
		let filter = IndexHintFilter::for_database(DatabaseType::SQLite)
			.with_index("idx_users_id", IndexStrategy::Force);

		let params = HashMap::new();
		let sql = "SELECT * FROM users WHERE id = 1".to_string();
		let result = filter.filter_queryset(&params, sql).await.unwrap();

		assert!(result.contains("FROM users INDEXED BY idx_users_id"));
	}

	#[tokio::test]
	async fn test_sqlite_not_indexed_hint() {
		let filter = IndexHintFilter::for_database(DatabaseType::SQLite)
			.with_index("idx_users_status", IndexStrategy::Ignore);

		let params = HashMap::new();
		let sql = "SELECT * FROM users WHERE status = 'active'".to_string();
		let result = filter.filter_queryset(&params, sql).await.unwrap();

		assert!(result.contains("FROM users NOT INDEXED"));
	}

	#[tokio::test]
	async fn test_sqlite_multiple_tables_with_hints() {
		let filter = IndexHintFilter::for_database(DatabaseType::SQLite)
			.with_index("idx_users_email", IndexStrategy::Use)
			.with_index("idx_orders_user_id", IndexStrategy::Force);

		let params = HashMap::new();
		let sql = "SELECT * FROM users JOIN orders ON users.id = orders.user_id".to_string();
		let result = filter.filter_queryset(&params, sql).await.unwrap();

		assert!(result.contains("FROM users INDEXED BY idx_users_email"));
		assert!(result.contains("JOIN orders INDEXED BY idx_orders_user_id"));
	}

	// PostgreSQL hint injection tests

	#[tokio::test]
	async fn test_postgresql_comment_hint() {
		let filter = IndexHintFilter::for_database(DatabaseType::PostgreSQL)
			.with_index("idx_users_email", IndexStrategy::Use);

		let params = HashMap::new();
		let sql = "SELECT * FROM users WHERE email = 'test@example.com'".to_string();
		let result = filter.filter_queryset(&params, sql).await.unwrap();

		assert!(result.contains("/* USE INDEX: idx_users_email */"));
		assert!(result.contains("SELECT * FROM users"));
	}

	#[tokio::test]
	async fn test_postgresql_multiple_hints() {
		let filter = IndexHintFilter::for_database(DatabaseType::PostgreSQL)
			.with_index("idx_users_email", IndexStrategy::Use)
			.with_index("idx_users_created_at", IndexStrategy::Force);

		let params = HashMap::new();
		let sql = "SELECT * FROM users".to_string();
		let result = filter.filter_queryset(&params, sql).await.unwrap();

		assert!(result.contains("/* USE INDEX: idx_users_email */"));
		assert!(result.contains("/* FORCE INDEX: idx_users_created_at */"));
	}

	// Complex query tests

	#[tokio::test]
	async fn test_mysql_join_with_multiple_hints() {
		let filter = IndexHintFilter::for_database(DatabaseType::MySQL)
			.with_hint(IndexHint::new("idx_email", IndexStrategy::Use).for_table("users"))
			.with_hint(IndexHint::new("idx_user_id", IndexStrategy::Force).for_table("orders"));

		let params = HashMap::new();
		let sql =
            "SELECT * FROM users INNER JOIN orders ON users.id = orders.user_id WHERE users.email = 'test@example.com'"
                .to_string();
		let result = filter.filter_queryset(&params, sql).await.unwrap();

		assert!(result.contains("FROM users USE INDEX (idx_email)"));
		assert!(result.contains("JOIN orders FORCE INDEX (idx_user_id)"));
	}

	#[tokio::test]
	async fn test_mysql_case_insensitive_from_join() {
		let filter = IndexHintFilter::for_database(DatabaseType::MySQL)
			.with_index("idx_users_email", IndexStrategy::Use);

		let params = HashMap::new();
		let sql = "SELECT * from users where email = 'test@example.com'".to_string();
		let result = filter.filter_queryset(&params, sql).await.unwrap();

		assert!(result.contains("from users USE INDEX (idx_users_email)"));
	}

	#[tokio::test]
	async fn test_sqlite_left_join_with_hint() {
		let filter = IndexHintFilter::for_database(DatabaseType::SQLite)
			.with_hint(IndexHint::new("idx_user_id", IndexStrategy::Use).for_table("orders"));

		let params = HashMap::new();
		let sql = "SELECT * FROM users LEFT JOIN orders ON users.id = orders.user_id".to_string();
		let result = filter.filter_queryset(&params, sql).await.unwrap();

		assert!(result.contains("JOIN orders INDEXED BY idx_user_id"));
	}

	#[test]
	fn test_for_database_constructor() {
		let mysql_filter = IndexHintFilter::for_database(DatabaseType::MySQL);
		assert_eq!(mysql_filter.db_type, DatabaseType::MySQL);

		let sqlite_filter = IndexHintFilter::for_database(DatabaseType::SQLite);
		assert_eq!(sqlite_filter.db_type, DatabaseType::SQLite);

		let pg_filter = IndexHintFilter::for_database(DatabaseType::PostgreSQL);
		assert_eq!(pg_filter.db_type, DatabaseType::PostgreSQL);
	}
}
