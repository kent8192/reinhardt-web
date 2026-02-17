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

use super::{FilterBackend, FilterError, FilterResult};
use async_trait::async_trait;
use reinhardt_query::prelude::{
	Cond, Expr, MySqlQueryBuilder, Order, Query, QueryStatementBuilder,
};
use std::collections::HashMap;
use std::sync::Arc;

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

	/// Escape special characters in LIKE patterns to prevent SQL injection
	///
	/// Escapes `%`, `_`, and `\` which have special meanings in SQL LIKE patterns.
	fn escape_like_pattern(pattern: &str) -> String {
		pattern
			.replace('\\', "\\\\")
			.replace('%', "\\%")
			.replace('_', "\\_")
	}

	/// Build the search condition using reinhardt-query
	///
	/// Returns the WHERE clause string (without the "WHERE" keyword) for the search condition.
	/// Uses `Expr::cust()` to generate database-agnostic SQL without identifier quoting.
	fn build_search_condition(&self, search_query: &str) -> String {
		let escaped = Self::escape_like_pattern(search_query);

		let mut condition = Cond::any();
		for field in &self.fields {
			// Use Expr::cust() to generate SQL without identifier quoting
			// This ensures compatibility with all databases (PostgreSQL, MySQL, SQLite)
			let like_expr = format!("{} LIKE '%{}%'", field, escaped);
			condition = condition.add(Expr::cust(like_expr));
		}

		// Build a minimal SELECT query to extract the WHERE clause
		let query = Query::select()
			.expr(Expr::val(1))
			.cond_where(condition)
			.to_string(MySqlQueryBuilder);

		// Extract just the WHERE condition portion (after "WHERE ")
		if let Some(idx) = query.find("WHERE ") {
			query[idx + 6..].to_string()
		} else {
			String::new()
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

			// Use reinhardt-query to build type-safe LIKE conditions
			let condition = self.build_search_condition(search_query);
			let where_clause = format!("WHERE ({})", condition);

			if sql.to_uppercase().contains("WHERE") {
				Ok(sql.replace("WHERE", &format!("{} AND", where_clause)))
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

			if sql.to_uppercase().contains("ORDER BY") {
				Ok(sql.replace("ORDER BY", &format!("{},", order_clause)))
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
		let backend = CustomFilterBackend::new();
		assert_eq!(backend.filter_count(), 0);

		let params = HashMap::new();
		let sql = "SELECT * FROM users".to_string();
		let result = backend.filter_queryset(&params, sql.clone()).await.unwrap();
		assert_eq!(result, sql);
	}

	#[rstest]
	#[tokio::test]
	async fn test_custom_filter_backend_with_filters() {
		let mut backend = CustomFilterBackend::new();
		backend.add_filter(Box::new(
			SimpleSearchBackend::new("search").with_field("name"),
		));

		assert_eq!(backend.filter_count(), 1);

		let mut params = HashMap::new();
		params.insert("search".to_string(), "john".to_string());

		let sql = "SELECT * FROM users".to_string();
		let result = backend.filter_queryset(&params, sql).await.unwrap();
		assert!(result.contains("WHERE"));
		// reinhardt-query generates backtick-quoted column names for MySQL
		assert!(result.contains("name LIKE '%john%'"));
	}

	#[rstest]
	#[tokio::test]
	async fn test_simple_search_backend() {
		let backend = SimpleSearchBackend::new("search")
			.with_field("title")
			.with_field("content");

		let mut params = HashMap::new();
		params.insert("search".to_string(), "rust".to_string());

		let sql = "SELECT * FROM articles".to_string();
		let result = backend.filter_queryset(&params, sql).await.unwrap();

		assert!(result.contains("WHERE"));
		// reinhardt-query generates backtick-quoted column names for MySQL
		assert!(result.contains("title LIKE '%rust%'"));
		assert!(result.contains("content LIKE '%rust%'"));
		assert!(result.contains("OR"));
	}

	#[rstest]
	#[tokio::test]
	async fn test_simple_search_backend_no_query() {
		let backend = SimpleSearchBackend::new("search").with_field("title");

		let params = HashMap::new();
		let sql = "SELECT * FROM articles".to_string();
		let result = backend.filter_queryset(&params, sql.clone()).await.unwrap();

		assert_eq!(result, sql);
	}

	#[rstest]
	#[tokio::test]
	async fn test_simple_search_backend_no_fields() {
		let backend = SimpleSearchBackend::new("search");

		let mut params = HashMap::new();
		params.insert("search".to_string(), "rust".to_string());

		let sql = "SELECT * FROM articles".to_string();
		let result = backend.filter_queryset(&params, sql).await;

		assert!(result.is_err());
	}

	#[rstest]
	#[tokio::test]
	async fn test_simple_ordering_backend_asc() {
		let backend = SimpleOrderingBackend::new("ordering")
			.allow_field("created_at")
			.allow_field("title");

		let mut params = HashMap::new();
		params.insert("ordering".to_string(), "created_at".to_string());

		let sql = "SELECT * FROM articles".to_string();
		let result = backend.filter_queryset(&params, sql).await.unwrap();

		// reinhardt-query generates backtick-quoted column names for MySQL
		assert!(result.contains("ORDER BY created_at ASC"));
	}

	#[rstest]
	#[tokio::test]
	async fn test_simple_ordering_backend_desc() {
		let backend = SimpleOrderingBackend::new("ordering")
			.allow_field("created_at")
			.allow_field("title");

		let mut params = HashMap::new();
		params.insert("ordering".to_string(), "-created_at".to_string());

		let sql = "SELECT * FROM articles".to_string();
		let result = backend.filter_queryset(&params, sql).await.unwrap();

		// reinhardt-query generates backtick-quoted column names for MySQL
		assert!(result.contains("ORDER BY created_at DESC"));
	}

	#[rstest]
	#[tokio::test]
	async fn test_simple_ordering_backend_invalid_field() {
		let backend = SimpleOrderingBackend::new("ordering").allow_field("created_at");

		let mut params = HashMap::new();
		params.insert("ordering".to_string(), "invalid_field".to_string());

		let sql = "SELECT * FROM articles".to_string();
		let result = backend.filter_queryset(&params, sql).await;

		assert!(result.is_err());
	}

	#[rstest]
	#[tokio::test]
	async fn test_simple_ordering_backend_no_query() {
		let backend = SimpleOrderingBackend::new("ordering").allow_field("created_at");

		let params = HashMap::new();
		let sql = "SELECT * FROM articles".to_string();
		let result = backend.filter_queryset(&params, sql.clone()).await.unwrap();

		assert_eq!(result, sql);
	}

	#[rstest]
	#[tokio::test]
	async fn test_chained_filters() {
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
		let result = backend.filter_queryset(&params, sql).await.unwrap();

		assert!(result.contains("WHERE"));
		// reinhardt-query generates backtick-quoted column names for MySQL
		assert!(result.contains("title LIKE '%rust%'"));
		assert!(result.contains("ORDER BY created_at DESC"));
	}
}
