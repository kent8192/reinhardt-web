//! Type-safe query filter using Field and Lookup
//!
//! Provides a completely type-safe filtering system using reinhardt-orm's
//! Field<M, T> and Lookup types.

use crate::filter::{FilterBackend, FilterResult};
use crate::ordering_field::OrderingField;
use async_trait::async_trait;
use reinhardt_db::orm::{Lookup, Model, QueryFieldCompiler};
use std::collections::HashMap;
use std::marker::PhantomData;

/// Type-safe query filter
///
/// Combines Field-based lookups and ordering into SQL WHERE and ORDER BY clauses.
/// All field access is compile-time checked for correctness.
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_filters::QueryFilter;
/// use reinhardt_db::orm::Field;
///
/// let filter = QueryFilter::<Post>::new()
///     .add(Field::new(vec!["title"]).icontains("rust"))
///     .add(Field::new(vec!["created_at"]).year().gte(2024))
///     .order_by(Field::new(vec!["title"]).asc());
/// ```
pub struct QueryFilter<M: Model> {
	lookups: Vec<Lookup<M>>,
	or_groups: Vec<Vec<Lookup<M>>>, // Each inner Vec is OR'd together, outer Vec is AND'd
	ordering: Vec<OrderingField<M>>,
	_phantom: PhantomData<M>,
}

impl<M: Model> QueryFilter<M> {
	/// Create a new empty filter
	pub fn new() -> Self {
		Self {
			lookups: Vec::new(),
			or_groups: Vec::new(),
			ordering: Vec::new(),
			_phantom: PhantomData,
		}
	}

	/// Add a lookup condition
	///
	/// All conditions are combined with AND.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// let filter = QueryFilter::<Post>::new()
	///     .with_lookup(Field::new(vec!["title"]).icontains("rust"))
	///     .with_lookup(Field::new(vec!["age"]).gte(18));
	/// ```
	pub fn with_lookup(mut self, lookup: Lookup<M>) -> Self {
		self.lookups.push(lookup);
		self
	}

	/// Add multiple lookups at once
	pub fn add_all(mut self, lookups: Vec<Lookup<M>>) -> Self {
		self.lookups.extend(lookups);
		self
	}

	/// Add an ordering field
	///
	/// Multiple ordering fields are applied in the order they are added.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// let filter = QueryFilter::<Post>::new()
	///     .order_by(Field::new(vec!["created_at"]).desc())
	///     .order_by(Field::new(vec!["title"]).asc());
	/// ```
	pub fn order_by(mut self, field: OrderingField<M>) -> Self {
		self.ordering.push(field);
		self
	}

	/// Add multiple ordering fields at once
	pub fn order_by_all(mut self, fields: Vec<OrderingField<M>>) -> Self {
		self.ordering.extend(fields);
		self
	}

	/// Add an OR group (lookups within the group are OR'd together)
	///
	/// # Examples
	///
	/// ```rust,ignore
	// (title ICONTAINS 'rust' OR content ICONTAINS 'rust')
	/// let filter = QueryFilter::<Post>::new()
	///     .add_or_group(vec![
	///         Field::new(vec!["title"]).icontains("rust"),
	///         Field::new(vec!["content"]).icontains("rust"),
	///     ]);
	/// ```
	pub fn add_or_group(mut self, lookups: Vec<Lookup<M>>) -> Self {
		if !lookups.is_empty() {
			self.or_groups.push(lookups);
		}
		self
	}

	/// Add multiple OR groups from multi-term search
	///
	/// Each term becomes an OR group, and all groups are AND'd together.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_filters::MultiTermSearch;
	///
	/// let terms = vec!["rust", "web"];
	/// let term_lookups = MultiTermSearch::search_terms::<Post>(terms);
	/// let filter = QueryFilter::<Post>::new()
	///     .add_multi_term(term_lookups);
	/// ```
	pub fn add_multi_term(mut self, term_lookups: Vec<Vec<Lookup<M>>>) -> Self {
		for lookups in term_lookups {
			if !lookups.is_empty() {
				self.or_groups.push(lookups);
			}
		}
		self
	}

	/// Compile lookups to SQL WHERE clause
	fn compile_where_clause(&self) -> Option<String> {
		let mut all_conditions = Vec::new();

		// Add regular AND conditions
		if !self.lookups.is_empty() {
			let conditions: Vec<String> = self
				.lookups
				.iter()
				.map(|lookup| QueryFieldCompiler::compile(lookup))
				.collect();
			all_conditions.extend(conditions);
		}

		// Add OR groups (each group is AND'd with others)
		for or_group in &self.or_groups {
			if or_group.is_empty() {
				continue;
			}

			let or_conditions: Vec<String> = or_group
				.iter()
				.map(|lookup| QueryFieldCompiler::compile(lookup))
				.collect();

			if or_conditions.len() == 1 {
				all_conditions.push(or_conditions[0].clone());
			} else {
				all_conditions.push(format!("({})", or_conditions.join(" OR ")));
			}
		}

		if all_conditions.is_empty() {
			return None;
		}

		if all_conditions.len() == 1 {
			Some(all_conditions[0].clone())
		} else {
			Some(format!("({})", all_conditions.join(" AND ")))
		}
	}

	/// Compile ordering to SQL ORDER BY clause
	fn compile_order_clause(&self) -> Option<String> {
		if self.ordering.is_empty() {
			return None;
		}

		let order_parts: Vec<String> = self.ordering.iter().map(|field| field.to_sql()).collect();

		Some(order_parts.join(", "))
	}

	/// Append order clause to existing ORDER BY
	///
	/// Supports:
	/// - Appending to existing ORDER BY clause
	/// - Merging multiple ordering fields
	///
	/// # Examples
	///
	/// ```ignore
	/// // Input: "SELECT * FROM posts ORDER BY created_at DESC"
	/// // New ordering: "title ASC"
	/// // Output: "SELECT * FROM posts ORDER BY created_at DESC, title ASC"
	/// ```
	fn append_order_clause(&self, sql: &str, new_order: &str) -> String {
		// Find ORDER BY position
		if let Some(order_by_pos) = sql.find("ORDER BY") {
			let before_order = &sql[..order_by_pos + 8]; // Include "ORDER BY"
			let after_order = &sql[order_by_pos + 8..];

			// Find end of ORDER BY clause (before LIMIT, OFFSET, or end of string)
			let end_markers = ["LIMIT", "OFFSET", ";"];
			let mut end_pos = after_order.len();

			for marker in &end_markers {
				if let Some(pos) = after_order.find(marker) {
					end_pos = end_pos.min(pos);
				}
			}

			let existing_order = after_order[..end_pos].trim();
			let remaining = &after_order[end_pos..];

			// Merge existing and new order clauses
			// Ensure space before remaining clause if it exists
			if remaining.is_empty() {
				format!("{} {}, {}", before_order, existing_order, new_order)
			} else {
				format!(
					"{} {}, {} {}",
					before_order,
					existing_order,
					new_order,
					remaining.trim()
				)
			}
		} else {
			// No ORDER BY found, append it
			format!("{} ORDER BY {}", sql, new_order)
		}
	}
}

impl<M: Model> Default for QueryFilter<M> {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl<M: Model> FilterBackend for QueryFilter<M> {
	async fn filter_queryset(
		&self,
		_params: &HashMap<String, String>,
		mut sql: String,
	) -> FilterResult<String> {
		// Add WHERE clause if we have lookups
		if let Some(where_clause) = self.compile_where_clause() {
			sql = if sql.contains("WHERE") {
				// Already has WHERE, add with AND
				sql.replace("WHERE", &format!("WHERE {} AND", where_clause))
			} else {
				// No WHERE yet, add it
				format!("{} WHERE {}", sql, where_clause)
			};
		}

		// Add ORDER BY clause if we have ordering
		if let Some(order_clause) = self.compile_order_clause() {
			if sql.contains("ORDER BY") {
				// Already has ORDER BY, append to it
				sql = self.append_order_clause(&sql, &order_clause);
			} else {
				sql = format!("{} ORDER BY {}", sql, order_clause);
			}
		}

		Ok(sql)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_db::orm::Field;

	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	struct TestPost {
		id: i64,
		title: String,
		content: String,
		age: i32,
		created_at: String,
	}

	impl Model for TestPost {
		type PrimaryKey = i64;

		fn table_name() -> &'static str {
			"test_posts"
		}

		fn primary_key(&self) -> Option<&Self::PrimaryKey> {
			Some(&self.id)
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = value;
		}
	}

	#[tokio::test]
	async fn test_single_lookup() {
		let lookup = Field::<TestPost, String>::new(vec!["title"]).eq("Test".to_string());
		let filter = QueryFilter::new().with_lookup(lookup);

		let sql = "SELECT * FROM test_posts".to_string();
		let result = filter.filter_queryset(&HashMap::new(), sql).await.unwrap();

		assert!(result.contains("WHERE"));
		assert!(result.contains("title = 'Test'"));
	}

	#[tokio::test]
	async fn test_multiple_lookups() {
		let filter = QueryFilter::new()
			.with_lookup(Field::<TestPost, String>::new(vec!["title"]).icontains("rust"))
			.with_lookup(Field::<TestPost, i32>::new(vec!["age"]).gte(18));

		let sql = "SELECT * FROM test_posts".to_string();
		let result = filter.filter_queryset(&HashMap::new(), sql).await.unwrap();

		assert!(result.contains("WHERE"));
		assert!(result.contains("title"));
		assert!(result.contains("age >= 18"));
		assert!(result.contains(" AND "));
	}

	#[tokio::test]
	async fn test_ordering() {
		use crate::field_extensions::FieldOrderingExt;

		let filter =
			QueryFilter::new().order_by(Field::<TestPost, String>::new(vec!["title"]).asc());

		let sql = "SELECT * FROM test_posts".to_string();
		let result = filter.filter_queryset(&HashMap::new(), sql).await.unwrap();

		assert!(result.contains("ORDER BY"));
		assert!(result.contains("title ASC"));
	}

	#[tokio::test]
	async fn test_lookup_and_ordering() {
		use crate::field_extensions::FieldOrderingExt;

		let filter = QueryFilter::new()
			.with_lookup(Field::<TestPost, String>::new(vec!["title"]).icontains("rust"))
			.order_by(Field::<TestPost, String>::new(vec!["created_at"]).desc());

		let sql = "SELECT * FROM test_posts".to_string();
		let result = filter.filter_queryset(&HashMap::new(), sql).await.unwrap();

		assert!(result.contains("WHERE"));
		assert!(result.contains("ORDER BY"));
		assert!(result.contains("created_at DESC"));
	}

	#[tokio::test]
	async fn test_append_to_existing_order_by() {
		use crate::field_extensions::FieldOrderingExt;

		let filter =
			QueryFilter::new().order_by(Field::<TestPost, String>::new(vec!["title"]).asc());

		let sql = "SELECT * FROM test_posts ORDER BY created_at DESC".to_string();
		let result = filter.filter_queryset(&HashMap::new(), sql).await.unwrap();

		assert!(result.contains("ORDER BY"));
		assert!(result.contains("created_at DESC"));
		assert!(result.contains("title ASC"));
		assert!(result.contains("created_at DESC, title ASC"));
	}

	#[tokio::test]
	async fn test_append_order_with_limit() {
		use crate::field_extensions::FieldOrderingExt;

		let filter =
			QueryFilter::new().order_by(Field::<TestPost, String>::new(vec!["title"]).asc());

		let sql = "SELECT * FROM test_posts ORDER BY created_at DESC LIMIT 10".to_string();
		let result = filter.filter_queryset(&HashMap::new(), sql).await.unwrap();

		assert!(result.contains("ORDER BY"));
		assert!(result.contains("created_at DESC, title ASC"));
		assert!(result.contains("LIMIT 10"));
		// Ensure LIMIT comes after ORDER BY
		let order_pos = result.find("ORDER BY").unwrap();
		let limit_pos = result.find("LIMIT").unwrap();
		assert!(order_pos < limit_pos);
	}

	#[test]
	fn test_append_order_clause_method() {
		let filter = QueryFilter::<TestPost>::new();

		// Test with existing ORDER BY
		let sql = "SELECT * FROM test_posts ORDER BY created_at DESC";
		let result = filter.append_order_clause(sql, "title ASC");
		assert_eq!(
			result,
			"SELECT * FROM test_posts ORDER BY created_at DESC, title ASC"
		);

		// Test with LIMIT
		let sql = "SELECT * FROM test_posts ORDER BY created_at DESC LIMIT 10";
		let result = filter.append_order_clause(sql, "title ASC");
		assert_eq!(
			result,
			"SELECT * FROM test_posts ORDER BY created_at DESC, title ASC LIMIT 10"
		);

		// Test with OFFSET
		let sql = "SELECT * FROM test_posts ORDER BY created_at DESC OFFSET 5";
		let result = filter.append_order_clause(sql, "title ASC");
		assert_eq!(
			result,
			"SELECT * FROM test_posts ORDER BY created_at DESC, title ASC OFFSET 5"
		);
	}
}
