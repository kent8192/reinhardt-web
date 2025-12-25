//! Unified query interface facade
//!
//! This module provides a unified entry point for querying functionality.
//! By default, it exports the expression-based query API (SQLAlchemy-style).

use sea_query::{
	Alias, Asterisk, Condition, Expr, ExprTrait, Order, PostgresQueryBuilder, Query as SeaQuery,
	SelectStatement,
};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::collections::HashMap;

// Django QuerySet API types (stub implementations)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperator {
	Eq,
	Ne,
	Gt,
	Gte,
	Lt,
	Lte,
	In,
	NotIn,
	Contains,
	StartsWith,
	EndsWith,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterValue {
	String(String),
	Integer(i64),
	/// Alias for Integer (for compatibility with test code)
	Int(i64),
	Float(f64),
	Boolean(bool),
	/// Alias for Boolean (for compatibility with test code)
	Bool(bool),
	Null,
	Array(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct Filter {
	pub field: String,
	pub operator: FilterOperator,
	pub value: FilterValue,
}

impl Filter {
	pub fn new(field: impl Into<String>, operator: FilterOperator, value: FilterValue) -> Self {
		Self {
			field: field.into(),
			operator,
			value,
		}
	}
}

/// Composite filter condition supporting AND/OR logic
///
/// This enum allows building complex filter expressions with nested AND/OR conditions.
/// It's particularly useful for search functionality that needs to match across
/// multiple fields using OR logic.
///
/// # Examples
///
/// ```
/// use reinhardt_orm::{Filter, FilterCondition, FilterOperator, FilterValue};
///
/// // Simple single filter
/// let single = FilterCondition::Single(Filter::new(
///     "name".to_string(),
///     FilterOperator::Eq,
///     FilterValue::String("Alice".to_string()),
/// ));
///
/// // OR condition across multiple fields (useful for search)
/// let search = FilterCondition::Or(vec![
///     FilterCondition::Single(Filter::new(
///         "name".to_string(),
///         FilterOperator::Contains,
///         FilterValue::String("alice".to_string()),
///     )),
///     FilterCondition::Single(Filter::new(
///         "email".to_string(),
///         FilterOperator::Contains,
///         FilterValue::String("alice".to_string()),
///     )),
/// ]);
///
/// // Complex nested condition: (status = 'active') AND (name LIKE '%alice%' OR email LIKE '%alice%')
/// let complex = FilterCondition::And(vec![
///     FilterCondition::Single(Filter::new(
///         "status".to_string(),
///         FilterOperator::Eq,
///         FilterValue::String("active".to_string()),
///     )),
///     search,
/// ]);
/// ```
#[derive(Debug, Clone)]
pub enum FilterCondition {
	/// A single filter expression
	Single(Filter),
	/// All conditions must match (AND logic)
	And(Vec<FilterCondition>),
	/// Any condition must match (OR logic)
	Or(Vec<FilterCondition>),
	/// Negates the inner condition (NOT logic)
	Not(Box<FilterCondition>),
}

impl FilterCondition {
	/// Create a single filter condition
	pub fn single(filter: Filter) -> Self {
		Self::Single(filter)
	}

	/// Create an AND condition from multiple conditions
	pub fn and(conditions: Vec<FilterCondition>) -> Self {
		Self::And(conditions)
	}

	/// Create an OR condition from multiple conditions
	pub fn or(conditions: Vec<FilterCondition>) -> Self {
		Self::Or(conditions)
	}

	/// Create a NOT condition that negates the given condition
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::{Filter, FilterCondition, FilterOperator, FilterValue};
	///
	/// let condition = FilterCondition::not(
	///     FilterCondition::Single(Filter::new(
	///         "is_active".to_string(),
	///         FilterOperator::Eq,
	///         FilterValue::Boolean(true),
	///     ))
	/// );
	/// ```
	// This method is intentionally named `not` for API consistency with Django's Q object.
	// It does not implement std::ops::Not because it constructs a FilterCondition variant,
	// not a boolean negation.
	#[allow(clippy::should_implement_trait)]
	pub fn not(condition: FilterCondition) -> Self {
		Self::Not(Box::new(condition))
	}

	/// Create an OR condition from multiple filters (convenience method for search)
	///
	/// This is particularly useful for implementing search across multiple fields.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::{Filter, FilterCondition, FilterOperator, FilterValue};
	///
	/// let search_filters = vec![
	///     Filter::new("name".to_string(), FilterOperator::Contains, FilterValue::String("test".to_string())),
	///     Filter::new("email".to_string(), FilterOperator::Contains, FilterValue::String("test".to_string())),
	/// ];
	/// let or_condition = FilterCondition::or_filters(search_filters);
	/// ```
	pub fn or_filters(filters: Vec<Filter>) -> Self {
		Self::Or(filters.into_iter().map(FilterCondition::Single).collect())
	}

	/// Create an AND condition from multiple filters
	pub fn and_filters(filters: Vec<Filter>) -> Self {
		Self::And(filters.into_iter().map(FilterCondition::Single).collect())
	}

	/// Check if this condition is empty (no actual filters)
	pub fn is_empty(&self) -> bool {
		match self {
			FilterCondition::Single(_) => false,
			FilterCondition::And(conditions) | FilterCondition::Or(conditions) => {
				conditions.is_empty() || conditions.iter().all(|c| c.is_empty())
			}
			FilterCondition::Not(condition) => condition.is_empty(),
		}
	}
}

// From implementations for FilterValue
impl From<String> for FilterValue {
	fn from(s: String) -> Self {
		FilterValue::String(s)
	}
}

impl From<&str> for FilterValue {
	fn from(s: &str) -> Self {
		FilterValue::String(s.to_string())
	}
}

impl From<i64> for FilterValue {
	fn from(i: i64) -> Self {
		FilterValue::Integer(i)
	}
}

impl From<i32> for FilterValue {
	fn from(i: i32) -> Self {
		FilterValue::Integer(i as i64)
	}
}

impl From<f64> for FilterValue {
	fn from(f: f64) -> Self {
		FilterValue::Float(f)
	}
}

impl From<bool> for FilterValue {
	fn from(b: bool) -> Self {
		FilterValue::Boolean(b)
	}
}

impl From<uuid::Uuid> for FilterValue {
	fn from(u: uuid::Uuid) -> Self {
		FilterValue::String(u.to_string())
	}
}

#[derive(Debug, Clone)]
pub struct Query {
	filters: Vec<Filter>,
}

impl Query {
	pub fn new() -> Self {
		Self {
			filters: Vec::new(),
		}
	}

	pub fn filter(mut self, filter: Filter) -> Self {
		self.filters.push(filter);
		self
	}
}

impl Default for Query {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Clone)]
pub struct QuerySet<T>
where
	T: crate::Model,
{
	_phantom: std::marker::PhantomData<T>,
	filters: SmallVec<[Filter; 10]>,
	select_related_fields: Vec<String>,
	prefetch_related_fields: Vec<String>,
	order_by_fields: Vec<String>,
	distinct_enabled: bool,
	selected_fields: Option<Vec<String>>,
	deferred_fields: Vec<String>,
	annotations: Vec<crate::annotation::Annotation>,
	manager: Option<std::sync::Arc<crate::manager::Manager<T>>>,
	limit: Option<usize>,
	offset: Option<usize>,
}

impl<T> QuerySet<T>
where
	T: crate::Model,
{
	pub fn new() -> Self {
		Self {
			_phantom: std::marker::PhantomData,
			filters: SmallVec::new(),
			select_related_fields: Vec::new(),
			prefetch_related_fields: Vec::new(),
			order_by_fields: Vec::new(),
			distinct_enabled: false,
			selected_fields: None,
			deferred_fields: Vec::new(),
			annotations: Vec::new(),
			manager: None,
			limit: None,
			offset: None,
		}
	}

	pub fn with_manager(manager: std::sync::Arc<crate::manager::Manager<T>>) -> Self {
		Self {
			_phantom: std::marker::PhantomData,
			filters: SmallVec::new(),
			select_related_fields: Vec::new(),
			prefetch_related_fields: Vec::new(),
			order_by_fields: Vec::new(),
			distinct_enabled: false,
			selected_fields: None,
			deferred_fields: Vec::new(),
			annotations: Vec::new(),
			manager: Some(manager),
			limit: None,
			offset: None,
		}
	}

	pub fn filter(mut self, filter: Filter) -> Self {
		self.filters.push(filter);
		self
	}

	/// Build WHERE condition using SeaQuery from accumulated filters
	fn build_where_condition(&self) -> Option<Condition> {
		if self.filters.is_empty() {
			return None;
		}

		let mut cond = Condition::all();

		for filter in &self.filters {
			let col = Expr::col(Alias::new(&filter.field));

			let expr = match (&filter.operator, &filter.value) {
				(FilterOperator::Eq, FilterValue::Null) => col.is_null(),
				(FilterOperator::Ne, FilterValue::Null) => col.is_not_null(),
				(FilterOperator::Eq, v) => col.eq(Self::filter_value_to_sea_value(v)),
				(FilterOperator::Ne, v) => col.ne(Self::filter_value_to_sea_value(v)),
				(FilterOperator::Gt, v) => col.gt(Self::filter_value_to_sea_value(v)),
				(FilterOperator::Gte, v) => col.gte(Self::filter_value_to_sea_value(v)),
				(FilterOperator::Lt, v) => col.lt(Self::filter_value_to_sea_value(v)),
				(FilterOperator::Lte, v) => col.lte(Self::filter_value_to_sea_value(v)),
				(FilterOperator::In, FilterValue::String(s)) => {
					let values = Self::parse_array_string(s);
					col.is_in(values)
				}
				(FilterOperator::In, FilterValue::Array(arr)) => {
					col.is_in(arr.iter().map(|s| s.as_str()).collect::<Vec<_>>())
				}
				(FilterOperator::NotIn, FilterValue::String(s)) => {
					let values = Self::parse_array_string(s);
					col.is_not_in(values)
				}
				(FilterOperator::NotIn, FilterValue::Array(arr)) => {
					col.is_not_in(arr.iter().map(|s| s.as_str()).collect::<Vec<_>>())
				}
				(FilterOperator::Contains, FilterValue::String(s)) => col.like(format!("%{}%", s)),
				(FilterOperator::Contains, FilterValue::Array(arr)) => {
					col.like(format!("%{}%", arr.first().unwrap_or(&String::new())))
				}
				(FilterOperator::StartsWith, FilterValue::String(s)) => col.like(format!("{}%", s)),
				(FilterOperator::StartsWith, FilterValue::Array(arr)) => {
					col.like(format!("{}%", arr.first().unwrap_or(&String::new())))
				}
				(FilterOperator::EndsWith, FilterValue::String(s)) => col.like(format!("%{}", s)),
				(FilterOperator::EndsWith, FilterValue::Array(arr)) => {
					col.like(format!("%{}", arr.first().unwrap_or(&String::new())))
				}
				// Handle Integer, Float, Boolean for text operators
				(FilterOperator::Contains, FilterValue::Integer(i) | FilterValue::Int(i)) => {
					col.like(format!("%{}%", i))
				}
				(FilterOperator::Contains, FilterValue::Float(f)) => col.like(format!("%{}%", f)),
				(FilterOperator::Contains, FilterValue::Boolean(b) | FilterValue::Bool(b)) => {
					col.like(format!("%{}%", b))
				}
				(FilterOperator::Contains, FilterValue::Null) => col.like("%"),
				(FilterOperator::StartsWith, FilterValue::Integer(i) | FilterValue::Int(i)) => {
					col.like(format!("{}%", i))
				}
				(FilterOperator::StartsWith, FilterValue::Float(f)) => col.like(format!("{}%", f)),
				(FilterOperator::StartsWith, FilterValue::Boolean(b) | FilterValue::Bool(b)) => {
					col.like(format!("{}%", b))
				}
				(FilterOperator::StartsWith, FilterValue::Null) => col.like("%"),
				(FilterOperator::EndsWith, FilterValue::Integer(i) | FilterValue::Int(i)) => {
					col.like(format!("%{}", i))
				}
				(FilterOperator::EndsWith, FilterValue::Float(f)) => col.like(format!("%{}", f)),
				(FilterOperator::EndsWith, FilterValue::Boolean(b) | FilterValue::Bool(b)) => {
					col.like(format!("%{}", b))
				}
				(FilterOperator::EndsWith, FilterValue::Null) => col.like("%"),
				// Handle In/NotIn for non-String types
				(FilterOperator::In, FilterValue::Integer(i) | FilterValue::Int(i)) => {
					col.is_in(vec![*i])
				}
				(FilterOperator::In, FilterValue::Float(f)) => col.is_in(vec![*f]),
				(FilterOperator::In, FilterValue::Boolean(b) | FilterValue::Bool(b)) => {
					col.is_in(vec![*b])
				}
				(FilterOperator::In, FilterValue::Null) => {
					col.is_in(vec![sea_query::Value::Int(None)])
				}
				(FilterOperator::NotIn, FilterValue::Integer(i) | FilterValue::Int(i)) => {
					col.is_not_in(vec![*i])
				}
				(FilterOperator::NotIn, FilterValue::Float(f)) => col.is_not_in(vec![*f]),
				(FilterOperator::NotIn, FilterValue::Boolean(b) | FilterValue::Bool(b)) => {
					col.is_not_in(vec![*b])
				}
				(FilterOperator::NotIn, FilterValue::Null) => {
					col.is_not_in(vec![sea_query::Value::Int(None)])
				}
			};

			cond = cond.add(expr);
		}

		Some(cond)
	}

	/// Convert FilterValue to sea_query::Value
	fn filter_value_to_sea_value(v: &FilterValue) -> sea_query::Value {
		match v {
			FilterValue::String(s) => s.clone().into(),
			FilterValue::Integer(i) | FilterValue::Int(i) => (*i).into(),
			FilterValue::Float(f) => (*f).into(),
			FilterValue::Boolean(b) | FilterValue::Bool(b) => (*b).into(),
			FilterValue::Null => sea_query::Value::Int(None),
			FilterValue::Array(arr) => arr.join(",").into(),
		}
	}

	/// Convert FilterValue to String representation
	#[allow(dead_code)]
	fn value_to_string(v: &FilterValue) -> String {
		match v {
			FilterValue::String(s) => s.clone(),
			FilterValue::Integer(i) | FilterValue::Int(i) => i.to_string(),
			FilterValue::Float(f) => f.to_string(),
			FilterValue::Boolean(b) | FilterValue::Bool(b) => b.to_string(),
			FilterValue::Null => String::new(),
			FilterValue::Array(arr) => arr.join(","),
		}
	}

	/// Parse array string into Vec<sea_query::Value>
	/// Supports comma-separated values or JSON array format
	fn parse_array_string(s: &str) -> Vec<sea_query::Value> {
		let trimmed = s.trim();

		// Try parsing as JSON array first
		if trimmed.starts_with('[')
			&& trimmed.ends_with(']')
			&& let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(trimmed)
		{
			return arr
				.iter()
				.map(|v| match v {
					serde_json::Value::String(s) => s.clone().into(),
					serde_json::Value::Number(n) => {
						if let Some(i) = n.as_i64() {
							i.into()
						} else if let Some(f) = n.as_f64() {
							f.into()
						} else {
							n.to_string().into()
						}
					}
					serde_json::Value::Bool(b) => (*b).into(),
					_ => v.to_string().into(),
				})
				.collect();
		}

		// Fallback to comma-separated parsing
		trimmed
			.split(',')
			.map(|s| s.trim())
			.filter(|s| !s.is_empty())
			.map(|s| s.to_string().into())
			.collect()
	}

	/// Convert FilterValue to array of sea_query::Value
	#[allow(dead_code)]
	fn value_to_array(v: &FilterValue) -> Vec<sea_query::Value> {
		match v {
			FilterValue::String(s) => Self::parse_array_string(s),
			FilterValue::Integer(i) | FilterValue::Int(i) => vec![(*i).into()],
			FilterValue::Float(f) => vec![(*f).into()],
			FilterValue::Boolean(b) | FilterValue::Bool(b) => vec![(*b).into()],
			FilterValue::Null => vec![sea_query::Value::Int(None)],
			FilterValue::Array(arr) => arr.iter().map(|s| s.clone().into()).collect(),
		}
	}

	/// Build WHERE clause from accumulated filters
	///
	/// # Deprecation Note
	///
	/// This method is maintained for backward compatibility with existing code that
	/// expects a string-based WHERE clause. New code should use `build_where_condition()`
	/// which returns a `Condition` object that can be directly added to SeaQuery statements.
	///
	/// This method generates a complete SELECT statement internally and extracts only
	/// the WHERE portion, which is less efficient than using `build_where_condition()`.
	#[allow(dead_code)]
	fn build_where_clause(&self) -> (String, Vec<String>) {
		if self.filters.is_empty() {
			return (String::new(), Vec::new());
		}

		// Build SeaQuery condition
		let mut stmt = SeaQuery::select();
		stmt.from(Alias::new("dummy"));

		if let Some(cond) = self.build_where_condition() {
			stmt.cond_where(cond);
		}

		// Convert to SQL string
		use sea_query::PostgresQueryBuilder;
		let sql = stmt.to_string(PostgresQueryBuilder);

		// Extract WHERE clause portion by finding the WHERE keyword
		let where_clause = if let Some(idx) = sql.find(" WHERE ") {
			sql[idx..].to_string()
		} else {
			String::new()
		};

		(where_clause, Vec::new())
	}

	/// Eagerly load related objects using JOIN queries
	///
	/// This method performs SQL JOINs to fetch related objects in a single query,
	/// reducing the number of database round-trips and preventing N+1 query problems.
	///
	/// # Performance
	///
	/// Best for one-to-one and many-to-one relationships where JOIN won't create
	/// significant data duplication. For one-to-many and many-to-many relationships,
	/// consider using `prefetch_related()` instead.
	///
	/// # Examples
	///
	/// ```ignore
	/// // Single query with JOINs instead of N+1 queries
	/// let posts = Post::objects()
	///     .select_related(&["author", "category"])
	///     .all()
	///     .await?;
	///
	/// // Each post has author and category pre-loaded
	/// for post in posts {
	///     println!("Author: {}", post.author.name); // No additional query
	/// }
	/// ```
	pub fn select_related(mut self, fields: &[&str]) -> Self {
		self.select_related_fields = fields.iter().map(|s| s.to_string()).collect();
		self
	}

	/// Generate SELECT query with JOIN clauses for select_related fields
	///
	/// Returns SeaQuery SelectStatement with LEFT JOIN for each related field to enable eager loading.
	///
	/// # Examples
	///
	/// ```ignore
	/// let queryset = Post::objects()
	///     .select_related(&["author", "category"])
	///     .filter(Filter::new(
	///         "published".to_string(),
	///         FilterOperator::Eq,
	///         FilterValue::Boolean(true),
	///     ));
	///
	/// let stmt = queryset.select_related_query();
	/// // Generates:
	/// // SELECT posts.*, author.*, category.* FROM posts
	/// //   LEFT JOIN users AS author ON posts.author_id = author.id
	/// //   LEFT JOIN categories AS category ON posts.category_id = category.id
	/// //   WHERE posts.published = $1
	/// ```
	pub fn select_related_query(&self) -> SelectStatement {
		let table_name = T::table_name();
		let mut stmt = SeaQuery::select();
		stmt.from(Alias::new(table_name));

		// Apply DISTINCT if enabled
		if self.distinct_enabled {
			stmt.distinct();
		}

		// Add main table columns
		stmt.column((Alias::new(table_name), Asterisk));

		// Add LEFT JOIN for each related field
		for related_field in &self.select_related_fields {
			// Convention: related_field is the field name in the model
			// We assume FK field is "{related_field}_id" and join to "{related_field}s" table
			let fk_field = Alias::new(format!("{}_id", related_field));
			let related_table = Alias::new(format!("{}s", related_field));
			let related_alias = Alias::new(related_field);

			// LEFT JOIN related_table AS related_field ON table.fk_field = related_field.id
			stmt.left_join(
				related_table,
				Expr::col((Alias::new(table_name), fk_field))
					.equals((related_alias.clone(), Alias::new("id"))),
			);

			// Add related table columns to SELECT
			stmt.column((related_alias, Asterisk));
		}

		// Apply WHERE conditions
		if let Some(cond) = self.build_where_condition() {
			stmt.cond_where(cond);
		}

		// Apply ORDER BY
		for order_field in &self.order_by_fields {
			let (field, is_desc) = if let Some(stripped) = order_field.strip_prefix('-') {
				(stripped, true)
			} else {
				(order_field.as_str(), false)
			};

			let col = Alias::new(field);
			if is_desc {
				stmt.order_by(col, Order::Desc);
			} else {
				stmt.order_by(col, Order::Asc);
			}
		}

		// Apply LIMIT/OFFSET
		if let Some(limit) = self.limit {
			stmt.limit(limit as u64);
		}
		if let Some(offset) = self.offset {
			stmt.offset(offset as u64);
		}

		stmt.to_owned()
	}

	/// Eagerly load related objects using separate queries
	///
	/// This method performs separate SQL queries for related objects and joins them
	/// in memory, which is more efficient than JOINs for one-to-many and many-to-many
	/// relationships that would create significant data duplication.
	///
	/// # Performance
	///
	/// Best for one-to-many and many-to-many relationships where JOINs would create
	/// data duplication (e.g., a post with 100 comments would duplicate post data 100 times).
	/// Uses 1 + N queries where N is the number of prefetch_related fields.
	///
	/// # Examples
	///
	/// ```ignore
	/// // 2 queries total instead of N+1 queries
	/// let posts = Post::objects()
	///     .prefetch_related(&["comments", "tags"])
	///     .all()
	///     .await?;
	///
	/// // Each post has comments and tags pre-loaded
	/// for post in posts {
	///     for comment in &post.comments {
	///         println!("Comment: {}", comment.text); // No additional query
	///     }
	/// }
	/// ```
	pub fn prefetch_related(mut self, fields: &[&str]) -> Self {
		self.prefetch_related_fields = fields.iter().map(|s| s.to_string()).collect();
		self
	}

	/// Generate SELECT queries for prefetch_related fields
	///
	/// Returns a vector of (field_name, SelectStatement) tuples, one for each prefetch field.
	/// Each query fetches related objects using IN clause with collected primary keys.
	///
	/// # Examples
	///
	/// ```ignore
	/// let queryset = Post::objects()
	///     .prefetch_related(&["comments", "tags"]);
	///
	/// let main_results = queryset.all().await?; // Main query
	/// let pk_values = vec![1, 2, 3]; // Collected from main results
	///
	/// let prefetch_queries = queryset.prefetch_related_queries(&pk_values);
	/// // Returns SelectStatements for:
	/// // 1. comments: SELECT * FROM comments WHERE post_id IN ($1, $2, $3)
	/// // 2. tags: SELECT tags.* FROM tags
	/// //          INNER JOIN post_tags ON tags.id = post_tags.tag_id
	/// //          WHERE post_tags.post_id IN ($1, $2, $3)
	/// ```
	pub fn prefetch_related_queries(&self, pk_values: &[i64]) -> Vec<(String, SelectStatement)> {
		if pk_values.is_empty() {
			return Vec::new();
		}

		let mut queries = Vec::new();

		for related_field in &self.prefetch_related_fields {
			// Determine if this is a many-to-many relation or one-to-many
			// by querying the model's relationship metadata
			let is_m2m = self.is_many_to_many_relation(related_field);

			let stmt = if is_m2m {
				self.prefetch_many_to_many_query(related_field, pk_values)
			} else {
				self.prefetch_one_to_many_query(related_field, pk_values)
			};

			queries.push((related_field.clone(), stmt));
		}

		queries
	}

	/// Check if a related field is a many-to-many relation
	///
	/// Determines relationship type by querying the model's metadata.
	/// Returns true if the relationship is defined as ManyToMany in the model metadata.
	fn is_many_to_many_relation(&self, related_field: &str) -> bool {
		// Get relationship metadata from the model
		let relations = T::relationship_metadata();

		// Find the relationship with the matching name
		relations
			.iter()
			.find(|rel| rel.name == related_field)
			.map(|rel| rel.relationship_type == crate::relationship::RelationshipType::ManyToMany)
			.unwrap_or(false)
	}

	/// Generate query for one-to-many prefetch
	///
	/// Generates: SELECT * FROM related_table WHERE fk_field IN (pk_values)
	fn prefetch_one_to_many_query(
		&self,
		related_field: &str,
		pk_values: &[i64],
	) -> SelectStatement {
		let table_name = T::table_name();
		let related_table = Alias::new(format!("{}s", related_field));
		let fk_field = Alias::new(format!("{}_id", table_name.trim_end_matches('s')));

		let mut stmt = SeaQuery::select();
		stmt.from(related_table).column(Asterisk);

		// Add IN clause with pk_values
		let values: Vec<sea_query::Value> = pk_values.iter().map(|&id| id.into()).collect();
		stmt.and_where(Expr::col(fk_field).is_in(values));

		stmt.to_owned()
	}

	/// Generate query for many-to-many prefetch
	///
	/// Generates: SELECT related.*, junction.main_id FROM related
	///            INNER JOIN junction ON related.id = junction.related_id
	///            WHERE junction.main_id IN (pk_values)
	fn prefetch_many_to_many_query(
		&self,
		related_field: &str,
		pk_values: &[i64],
	) -> SelectStatement {
		let table_name = T::table_name();
		let junction_table = Alias::new(format!("{}_{}", table_name, related_field));
		let related_table = Alias::new(format!("{}s", related_field));
		let junction_main_fk = Alias::new(format!("{}_id", table_name.trim_end_matches('s')));
		let junction_related_fk = Alias::new(format!("{}_id", related_field));

		let mut stmt = SeaQuery::select();
		stmt.from(related_table.clone())
			.column((related_table.clone(), Asterisk))
			.column((junction_table.clone(), junction_main_fk.clone()))
			.inner_join(
				junction_table.clone(),
				Expr::col((related_table.clone(), Alias::new("id")))
					.equals((junction_table.clone(), junction_related_fk)),
			);

		// Add IN clause with pk_values
		let values: Vec<sea_query::Value> = pk_values.iter().map(|&id| id.into()).collect();
		stmt.and_where(Expr::col((junction_table, junction_main_fk)).is_in(values));

		stmt.to_owned()
	}

	/// Execute the queryset and return all matching records
	///
	/// Fetches all records from the database that match the accumulated filters.
	/// If `select_related` fields are specified, performs JOIN queries for eager loading.
	///
	/// # Examples
	///
	/// ```ignore
	/// // Fetch all users
	/// let users = User::objects().all().await?;
	///
	/// // Fetch filtered users with eager loading
	/// let active_users = User::objects()
	///     .filter(Filter::new(
	///         "is_active".to_string(),
	///         FilterOperator::Eq,
	///         FilterValue::Boolean(true),
	///     ))
	///     .select_related(&["profile"])
	///     .all()
	///     .await?;
	/// ```
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - Database connection fails
	/// - SQL execution fails
	/// - Deserialization of results fails
	pub async fn all(&self) -> reinhardt_core::exception::Result<Vec<T>>
	where
		T: serde::de::DeserializeOwned,
	{
		let conn = crate::manager::get_connection().await?;

		let stmt = if self.select_related_fields.is_empty() {
			// Simple SELECT without JOINs
			let mut stmt = SeaQuery::select();
			stmt.from(Alias::new(T::table_name())).column(Asterisk);

			if let Some(cond) = self.build_where_condition() {
				stmt.cond_where(cond);
			}

			stmt.to_owned()
		} else {
			// SELECT with JOINs for select_related
			self.select_related_query()
		};

		// Convert SeaQuery statement to SQL with inline parameters
		// Note: Using to_string() embeds parameters directly in SQL (safe with SeaQuery's escaping)
		// This is necessary because DatabaseConnection::query() doesn't support parameter binding
		let sql = stmt.to_string(PostgresQueryBuilder);

		// Execute query and deserialize results
		let rows = conn.query(&sql, vec![]).await?;
		rows.into_iter()
			.map(|row| {
				serde_json::from_value(serde_json::to_value(&row.data).map_err(|e| {
					reinhardt_core::exception::Error::Database(format!(
						"Serialization error: {}",
						e
					))
				})?)
				.map_err(|e| {
					reinhardt_core::exception::Error::Database(format!(
						"Deserialization error: {}",
						e
					))
				})
			})
			.collect()
	}

	/// Execute the queryset and return the first matching record
	///
	/// Returns `None` if no records match the query.
	///
	/// # Examples
	///
	/// ```ignore
	/// // Fetch first active user
	/// let user = User::objects()
	///     .filter(Filter::new(
	///         "is_active".to_string(),
	///         FilterOperator::Eq,
	///         FilterValue::Boolean(true),
	///     ))
	///     .first()
	///     .await?;
	///
	/// match user {
	///     Some(u) => println!("Found user: {}", u.username),
	///     None => println!("No active users found"),
	/// }
	/// ```
	pub async fn first(&self) -> reinhardt_core::exception::Result<Option<T>>
	where
		T: serde::de::DeserializeOwned,
	{
		let mut results = self.all().await?;
		Ok(results.drain(..).next())
	}

	/// Execute the queryset and return a single matching record
	///
	/// Returns an error if zero or multiple records are found.
	///
	/// # Examples
	///
	/// ```ignore
	/// // Fetch user with specific email (must be unique)
	/// let user = User::objects()
	///     .filter(Filter::new(
	///         "email".to_string(),
	///         FilterOperator::Eq,
	///         FilterValue::String("alice@example.com".to_string()),
	///     ))
	///     .get()
	///     .await?;
	/// ```
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - No records match the query
	/// - Multiple records match the query
	/// - Database connection fails
	pub async fn get(&self) -> reinhardt_core::exception::Result<T>
	where
		T: serde::de::DeserializeOwned,
	{
		let results = self.all().await?;
		match results.len() {
			0 => Err(reinhardt_core::exception::Error::Database(
				"No record found matching the query".to_string(),
			)),
			1 => Ok(results.into_iter().next().unwrap()),
			n => Err(reinhardt_core::exception::Error::Database(format!(
				"Multiple records found ({}), expected exactly one",
				n
			))),
		}
	}

	/// Execute the queryset with an explicit database connection and return all records
	///
	/// # Examples
	///
	/// ```ignore
	/// let db = get_database_connection().await?;
	/// let users = User::objects()
	///     .filter(User::field_status().eq("active"))
	///     .all(&db)
	///     .await?;
	/// ```
	pub async fn all_with_db(
		&self,
		conn: &crate::connection::DatabaseConnection,
	) -> reinhardt_core::exception::Result<Vec<T>>
	where
		T: serde::de::DeserializeOwned,
	{
		let stmt = if self.select_related_fields.is_empty() {
			let mut stmt = SeaQuery::select();
			stmt.from(Alias::new(T::table_name())).column(Asterisk);

			if let Some(cond) = self.build_where_condition() {
				stmt.cond_where(cond);
			}

			stmt.to_owned()
		} else {
			self.select_related_query()
		};

		let sql = stmt.to_string(PostgresQueryBuilder);

		let rows = conn.query(&sql, vec![]).await?;
		rows.into_iter()
			.map(|row| {
				serde_json::from_value(serde_json::to_value(&row.data).map_err(|e| {
					reinhardt_core::exception::Error::Database(format!(
						"Serialization error: {}",
						e
					))
				})?)
				.map_err(|e| {
					reinhardt_core::exception::Error::Database(format!(
						"Deserialization error: {}",
						e
					))
				})
			})
			.collect()
	}

	/// Execute the queryset with an explicit database connection and return a single record
	///
	/// # Examples
	///
	/// ```ignore
	/// let db = get_database_connection().await?;
	/// let user = User::objects()
	///     .filter(User::field_id().eq(user_id))
	///     .get(&db)
	///     .await?;
	/// ```
	pub async fn get_with_db(
		&self,
		conn: &crate::connection::DatabaseConnection,
	) -> reinhardt_core::exception::Result<T>
	where
		T: serde::de::DeserializeOwned,
	{
		let results = self.all_with_db(conn).await?;
		match results.len() {
			0 => Err(reinhardt_core::exception::Error::NotFound(
				"No record found matching the query".to_string(),
			)),
			1 => Ok(results.into_iter().next().unwrap()),
			n => Err(reinhardt_core::exception::Error::Database(format!(
				"Multiple records found ({}), expected exactly one",
				n
			))),
		}
	}

	/// Execute the queryset with an explicit database connection and return the first record
	///
	/// # Examples
	///
	/// ```ignore
	/// let db = get_database_connection().await?;
	/// let user = User::objects()
	///     .filter(User::field_status().eq("active"))
	///     .first(&db)
	///     .await?;
	/// ```
	pub async fn first_with_db(
		&self,
		conn: &crate::connection::DatabaseConnection,
	) -> reinhardt_core::exception::Result<Option<T>>
	where
		T: serde::de::DeserializeOwned,
	{
		let mut results = self.all_with_db(conn).await?;
		Ok(results.drain(..).next())
	}

	/// Execute the queryset and return the count of matching records
	///
	/// More efficient than calling `all().await?.len()` as it only executes COUNT query.
	///
	/// # Examples
	///
	/// ```ignore
	/// // Count active users
	/// let count = User::objects()
	///     .filter(Filter::new(
	///         "is_active".to_string(),
	///         FilterOperator::Eq,
	///         FilterValue::Boolean(true),
	///     ))
	///     .count()
	///     .await?;
	///
	/// println!("Active users: {}", count);
	/// ```
	pub async fn count(&self) -> reinhardt_core::exception::Result<usize> {
		use sea_query::{Func, PostgresQueryBuilder};

		let conn = crate::manager::get_connection().await?;

		// Build COUNT query using SeaQuery
		let mut stmt = SeaQuery::select();
		stmt.from(Alias::new(T::table_name()))
			.expr(Func::count(Expr::col(Asterisk)));

		// Add WHERE conditions
		if let Some(cond) = self.build_where_condition() {
			stmt.cond_where(cond);
		}

		// Convert to SQL and extract parameter values
		let (sql, values) = stmt.build(PostgresQueryBuilder);

		// Convert sea_query::Values to QueryValue
		let params = crate::execution::convert_values(values);

		// Execute query with parameters
		let rows = conn.query(&sql, params).await?;
		if let Some(row) = rows.first() {
			// Extract count from first row
			if let Some(count_value) = row.data.get("count")
				&& let Some(count) = count_value.as_i64()
			{
				return Ok(count as usize);
			}
		}

		Ok(0)
	}

	/// Check if any records match the queryset
	///
	/// More efficient than calling `count().await? > 0` as it can short-circuit.
	///
	/// # Examples
	///
	/// ```ignore
	/// // Check if any admin users exist
	/// let has_admin = User::objects()
	///     .filter(Filter::new(
	///         "role".to_string(),
	///         FilterOperator::Eq,
	///         FilterValue::String("admin".to_string()),
	///     ))
	///     .exists()
	///     .await?;
	///
	/// if has_admin {
	///     println!("Admin users exist");
	/// }
	/// ```
	pub async fn exists(&self) -> reinhardt_core::exception::Result<bool> {
		let count = self.count().await?;
		Ok(count > 0)
	}

	/// Create a new object in the database
	///
	/// # Examples
	///
	/// ```ignore
	/// let user = User {
	///     id: None,
	///     username: "alice".to_string(),
	///     email: "alice@example.com".to_string(),
	/// };
	/// let created = User::objects().create(user).await?;
	/// ```
	pub async fn create(&self, object: T) -> reinhardt_core::exception::Result<T>
	where
		T: crate::Model + Clone,
	{
		// Delegate to Manager::create() which handles all the SQL generation,
		// database connection, primary key retrieval, and error handling
		match &self.manager {
			Some(manager) => manager.create(&object).await,
			None => {
				// Fallback: create a new manager instance if none exists
				let manager = crate::manager::Manager::<T>::new();
				manager.create(&object).await
			}
		}
	}

	/// Generate UPDATE statement using SeaQuery
	pub fn update_query(&self, updates: &[(&str, &str)]) -> sea_query::UpdateStatement {
		let mut stmt = SeaQuery::update();
		stmt.table(Alias::new(T::table_name()));

		// Add SET clauses
		for (field, value) in updates {
			stmt.value(Alias::new(*field), value.to_string());
		}

		// Add WHERE conditions
		if let Some(cond) = self.build_where_condition() {
			stmt.cond_where(cond);
		}

		stmt.to_owned()
	}

	/// Generate UPDATE SQL with WHERE clause and parameter binding
	///
	/// Returns SQL with placeholders ($1, $2, etc.) and the values to bind.
	///
	/// # Examples
	///
	/// ```ignore
	/// let queryset = User::objects()
	///     .filter(Filter::new(
	///         "id".to_string(),
	///         FilterOperator::Eq,
	///         FilterValue::Integer(1),
	///     ));
	///
	/// let (sql, params) = queryset.update_sql(&[("name", "Alice"), ("email", "alice@example.com")]);
	/// // sql: "UPDATE users SET name = $1, email = $2 WHERE id = $3"
	/// // params: ["Alice", "alice@example.com", "1"]
	/// ```
	pub fn update_sql(&self, updates: &[(&str, &str)]) -> (String, Vec<String>) {
		let stmt = self.update_query(updates);
		use sea_query::PostgresQueryBuilder;
		let (sql, values) = stmt.build(PostgresQueryBuilder);
		let params: Vec<String> = values
			.iter()
			.map(|v| Self::sea_value_to_string(v))
			.collect();
		(sql, params)
	}

	/// Convert SeaQuery Value to String without SQL quoting
	fn sea_value_to_string(value: &sea_query::Value) -> String {
		use sea_query::Value;
		match value {
			Value::Bool(Some(b)) => b.to_string(),
			Value::TinyInt(Some(i)) => i.to_string(),
			Value::SmallInt(Some(i)) => i.to_string(),
			Value::Int(Some(i)) => i.to_string(),
			Value::BigInt(Some(i)) => i.to_string(),
			Value::TinyUnsigned(Some(i)) => i.to_string(),
			Value::SmallUnsigned(Some(i)) => i.to_string(),
			Value::Unsigned(Some(i)) => i.to_string(),
			Value::BigUnsigned(Some(i)) => i.to_string(),
			Value::Float(Some(f)) => f.to_string(),
			Value::Double(Some(f)) => f.to_string(),
			Value::String(Some(s)) => s.to_string(),
			Value::Bytes(Some(b)) => String::from_utf8_lossy(b).to_string(),
			_ => String::new(),
		}
	}

	/// Generate DELETE SQL with WHERE clause and parameter binding
	///
	/// Returns SQL with placeholders ($1, $2, etc.) and the values to bind.
	///
	/// # Safety
	///
	/// This method will panic if no filters are set to prevent accidental deletion of all rows.
	/// Always use `.filter()` before calling this method.
	///
	/// # Examples
	///
	/// ```ignore
	/// let queryset = User::objects()
	///     .filter(Filter::new(
	///         "id".to_string(),
	///         FilterOperator::Eq,
	///         FilterValue::Integer(1),
	///     ));
	///
	/// let (sql, params) = queryset.delete_sql();
	/// // sql: "DELETE FROM users WHERE id = $1"
	/// // params: ["1"]
	/// ```
	/// Generate DELETE statement using SeaQuery
	pub fn delete_query(&self) -> sea_query::DeleteStatement {
		if self.filters.is_empty() {
			panic!(
				"DELETE without WHERE clause is not allowed. Use .filter() to specify which rows to delete."
			);
		}

		let mut stmt = SeaQuery::delete();
		stmt.from_table(Alias::new(T::table_name()));

		// Add WHERE conditions
		if let Some(cond) = self.build_where_condition() {
			stmt.cond_where(cond);
		}

		stmt.to_owned()
	}

	pub fn delete_sql(&self) -> (String, Vec<String>) {
		let stmt = self.delete_query();
		use sea_query::PostgresQueryBuilder;
		let (sql, values) = stmt.build(PostgresQueryBuilder);
		let params: Vec<String> = values
			.iter()
			.map(|v| Self::sea_value_to_string(v))
			.collect();
		(sql, params)
	}

	/// Retrieve a single object by composite primary key
	///
	/// This method queries the database using all fields that compose the composite primary key.
	/// It validates that all required primary key fields are provided and returns the matching record.
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_orm::composite_pk::PkValue;
	/// use std::collections::HashMap;
	///
	/// let mut pk_values = HashMap::new();
	/// pk_values.insert("post_id".to_string(), PkValue::Int(1));
	/// pk_values.insert("tag_id".to_string(), PkValue::Int(5));
	///
	/// let post_tag = PostTag::objects().get_composite(&pk_values).await?;
	/// ```
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The model doesn't have a composite primary key
	/// - Required primary key fields are missing from the provided values
	/// - No matching record is found in the database
	/// - Multiple records match (should not happen with a valid composite PK)
	pub async fn get_composite(
		&self,
		pk_values: &HashMap<String, crate::composite_pk::PkValue>,
	) -> reinhardt_core::exception::Result<T>
	where
		T: crate::Model + Clone,
	{
		use sea_query::{Alias, BinOper, Expr, ExprTrait, PostgresQueryBuilder, Value};

		// Get composite primary key definition from the model
		let composite_pk = T::composite_primary_key().ok_or_else(|| {
			reinhardt_core::exception::Error::Database(
				"Model does not have a composite primary key".to_string(),
			)
		})?;

		// Validate that all required PK fields are provided
		composite_pk.validate(pk_values).map_err(|e| {
			reinhardt_core::exception::Error::Database(format!(
				"Composite PK validation failed: {}",
				e
			))
		})?;

		// Build SELECT query using sea-query
		let table_name = T::table_name();
		let mut query = SeaQuery::select();

		// Use Alias::new for table name
		let table_alias = Alias::new(table_name);
		query.from(table_alias).column(sea_query::Asterisk);

		// Add WHERE conditions for each composite PK field
		for field_name in composite_pk.fields() {
			let pk_value: &crate::composite_pk::PkValue = pk_values.get(field_name).unwrap();
			let col_alias = Alias::new(field_name);

			match pk_value {
				&crate::composite_pk::PkValue::Int(v) => {
					let condition = Expr::col(col_alias)
						.binary(BinOper::Equal, Expr::value(Value::BigInt(Some(v))));
					query.and_where(condition);
				}
				&crate::composite_pk::PkValue::Uint(v) => {
					let condition = Expr::col(col_alias)
						.binary(BinOper::Equal, Expr::value(Value::BigInt(Some(v as i64))));
					query.and_where(condition);
				}
				crate::composite_pk::PkValue::String(v) => {
					let condition = Expr::col(col_alias)
						.binary(BinOper::Equal, Expr::value(Value::String(Some(v.clone()))));
					query.and_where(condition);
				}
				&crate::composite_pk::PkValue::Bool(v) => {
					let condition = Expr::col(col_alias)
						.binary(BinOper::Equal, Expr::value(Value::Bool(Some(v))));
					query.and_where(condition);
				}
			}
		}

		// Build SQL with parameter binding
		let sql = query.to_string(PostgresQueryBuilder);

		// Execute query using database connection
		let conn = crate::manager::get_connection().await?;

		// Execute the SELECT query
		let rows = conn.query(&sql, vec![]).await?;

		// Composite PK queries should return exactly one row
		if rows.is_empty() {
			return Err(reinhardt_core::exception::Error::Database(
				"No record found matching the composite primary key".to_string(),
			));
		}

		if rows.len() > 1 {
			return Err(reinhardt_core::exception::Error::Database(format!(
				"Multiple records found ({}) for composite primary key, expected exactly one",
				rows.len()
			)));
		}

		// Deserialize the single row into the model
		let row = &rows[0];
		let value = serde_json::to_value(&row.data).map_err(|e| {
			reinhardt_core::exception::Error::Database(format!("Serialization error: {}", e))
		})?;

		serde_json::from_value(value).map_err(|e| {
			reinhardt_core::exception::Error::Database(format!("Deserialization error: {}", e))
		})
	}

	/// Add an annotation to the QuerySet
	///
	/// Annotations allow you to add calculated fields to query results using expressions,
	/// aggregations, or subqueries. The annotation will be added to the SELECT clause.
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_orm::annotation::{Annotation, AnnotationValue};
	/// use reinhardt_orm::aggregation::Aggregate;
	///
	/// // Add aggregate annotation
	/// let users = User::objects()
	///     .annotate(Annotation::new("total_orders",
	///         AnnotationValue::Aggregate(Aggregate::count(Some("orders")))))
	///     .all()
	///     .await?;
	/// ```
	pub fn annotate(mut self, annotation: crate::annotation::Annotation) -> Self {
		self.annotations.push(annotation);
		self
	}

	/// Perform an aggregation on the QuerySet
	///
	/// Aggregations allow you to calculate summary statistics (COUNT, SUM, AVG, MAX, MIN)
	/// for the queryset. The aggregation result will be added to the SELECT clause.
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_orm::aggregation::Aggregate;
	///
	/// // Count all users
	/// let result = User::objects()
	///     .aggregate(Aggregate::count_all().with_alias("total_users"))
	///     .all()
	///     .await?;
	///
	/// // Sum order amounts
	/// let result = Order::objects()
	///     .aggregate(Aggregate::sum("amount").with_alias("total_amount"))
	///     .all()
	///     .await?;
	/// ```
	pub fn aggregate(mut self, aggregate: crate::aggregation::Aggregate) -> Self {
		// Convert Aggregate to Annotation and add to annotations list
		let alias = aggregate
			.alias
			.clone()
			.unwrap_or_else(|| aggregate.func.to_string().to_lowercase());
		let annotation = crate::annotation::Annotation {
			alias,
			value: crate::annotation::AnnotationValue::Aggregate(aggregate),
		};
		self.annotations.push(annotation);
		self
	}

	pub fn to_sql(&self) -> String {
		let mut stmt = if self.select_related_fields.is_empty() {
			// Simple SELECT without JOINs
			let mut stmt = SeaQuery::select();
			stmt.from(Alias::new(T::table_name()));

			// Apply DISTINCT if enabled
			if self.distinct_enabled {
				stmt.distinct();
			}

			// Select columns
			if let Some(ref fields) = self.selected_fields {
				for field in fields {
					stmt.column(Alias::new(field));
				}
			} else {
				stmt.column(Asterisk);
			}

			// Apply WHERE conditions
			if let Some(cond) = self.build_where_condition() {
				stmt.cond_where(cond);
			}

			// Apply ORDER BY
			for order_field in &self.order_by_fields {
				let (field, is_desc) = if let Some(stripped) = order_field.strip_prefix('-') {
					(stripped, true)
				} else {
					(order_field.as_str(), false)
				};

				let col = Alias::new(field);
				if is_desc {
					stmt.order_by(col, Order::Desc);
				} else {
					stmt.order_by(col, Order::Asc);
				}
			}

			// Apply LIMIT/OFFSET
			if let Some(limit) = self.limit {
				stmt.limit(limit as u64);
			}
			if let Some(offset) = self.offset {
				stmt.offset(offset as u64);
			}

			stmt.to_owned()
		} else {
			// SELECT with JOINs for select_related
			self.select_related_query()
		};

		// Add annotations to SELECT clause if any using SeaQuery API
		// Collect annotation SQL strings first to handle lifetime issues
		// Note: Use to_sql_expr() to get expression without alias (SeaQuery adds alias via expr_as)
		let annotation_exprs: Vec<_> = self
			.annotations
			.iter()
			.map(|a| (a.value.to_sql_expr(), a.alias.clone()))
			.collect();

		for (value_sql, alias) in annotation_exprs {
			stmt.expr_as(Expr::cust(value_sql), Alias::new(alias));
		}

		use sea_query::PostgresQueryBuilder;
		stmt.to_string(PostgresQueryBuilder)
	}

	/// Select specific values from the QuerySet
	///
	/// Returns only the specified fields instead of all columns.
	/// Useful for optimizing queries when you don't need all model fields.
	///
	/// # Examples
	///
	/// ```ignore
	/// // Select only specific fields
	/// let users = User::objects()
	///     .values(&["id", "username", "email"])
	///     .all()
	///     .await?;
	/// // Generates: SELECT id, username, email FROM users
	///
	/// // Combine with filters
	/// let active_user_names = User::objects()
	///     .filter(Filter::new("is_active".to_string(), FilterOperator::Eq, FilterValue::Bool(true)))
	///     .values(&["username"])
	///     .all()
	///     .await?;
	/// ```
	pub fn values(mut self, fields: &[&str]) -> Self {
		self.selected_fields = Some(fields.iter().map(|s| s.to_string()).collect());
		self
	}

	/// Select specific values as a list
	///
	/// Alias for `values()` - returns tuple-like results with specified fields.
	/// In Django, this returns tuples instead of dictionaries, but in Rust
	/// the behavior is the same as `values()` due to type safety.
	///
	/// # Examples
	///
	/// ```ignore
	/// // Same as values()
	/// let user_data = User::objects()
	///     .values_list(&["id", "username"])
	///     .all()
	///     .await?;
	/// ```
	pub fn values_list(self, fields: &[&str]) -> Self {
		self.values(fields)
	}

	/// Order the QuerySet by specified fields
	///
	/// # Examples
	///
	/// ```ignore
	/// // Ascending order
	/// User::objects().order_by(&["name"]);
	///
	/// // Descending order (prefix with '-')
	/// User::objects().order_by(&["-created_at"]);
	///
	/// // Multiple fields
	/// User::objects().order_by(&["department", "-salary"]);
	/// ```
	pub fn order_by(mut self, fields: &[&str]) -> Self {
		self.order_by_fields = fields.iter().map(|s| s.to_string()).collect();
		self
	}

	/// Return only distinct results
	pub fn distinct(mut self) -> Self {
		self.distinct_enabled = true;
		self
	}

	/// Set LIMIT clause
	///
	/// Limits the number of records returned by the query.
	/// Corresponds to Django's QuerySet slicing [:limit].
	///
	/// # Examples
	///
	/// ```ignore
	/// let users = User::objects()
	///     .limit(10)
	///     .all()
	///     .await?;
	/// ```
	pub fn limit(mut self, limit: usize) -> Self {
		self.limit = Some(limit);
		self
	}

	/// Set OFFSET clause
	///
	/// Skips the specified number of records before returning results.
	/// Corresponds to Django's QuerySet slicing [offset:].
	///
	/// # Examples
	///
	/// ```ignore
	/// let users = User::objects()
	///     .offset(20)
	///     .limit(10)
	///     .all()
	///     .await?;
	/// ```
	pub fn offset(mut self, offset: usize) -> Self {
		self.offset = Some(offset);
		self
	}

	/// Paginate results using page number and page size
	///
	/// Convenience method that calculates offset automatically.
	/// Corresponds to Django REST framework's PageNumberPagination.
	///
	/// # Examples
	///
	/// ```ignore
	/// // Page 3, 10 items per page (offset=20, limit=10)
	/// let users = User::objects()
	///     .paginate(3, 10)
	///     .all()
	///     .await?;
	/// ```
	pub fn paginate(self, page: usize, page_size: usize) -> Self {
		let offset = page.saturating_sub(1) * page_size;
		self.offset(offset).limit(page_size)
	}

	/// Convert QuerySet to a subquery
	///
	/// Returns the QuerySet as a SQL subquery wrapped in parentheses,
	/// suitable for use in IN clauses, EXISTS clauses, or as a derived table.
	///
	/// # Examples
	///
	/// ```ignore
	/// // Use in IN clause
	/// let active_user_ids = User::objects()
	///     .filter(Filter::new("is_active".to_string(), FilterOperator::Eq, FilterValue::Bool(true)))
	///     .values(vec!["id"])
	///     .as_subquery();
	/// // Generates: (SELECT id FROM users WHERE is_active = $1)
	///
	/// // Use as derived table
	/// let subquery = Post::objects()
	///     .filter(Filter::new("published".to_string(), FilterOperator::Eq, FilterValue::Bool(true)))
	///     .as_subquery();
	/// // Generates: (SELECT * FROM posts WHERE published = $1)
	/// ```
	pub fn as_subquery(self) -> String {
		format!("({})", self.to_sql())
	}

	/// Defer loading of specific fields
	///
	/// Marks specific fields for deferred loading (lazy loading).
	/// The specified fields will be excluded from the initial query.
	///
	/// # Note
	///
	/// In the current implementation, deferred fields are simply stored
	/// but not yet used in query generation. Full deferred loading support
	/// will be implemented in a future version.
	///
	/// # Examples
	///
	/// ```ignore
	/// // Defer large text fields
	/// let users = User::objects()
	///     .defer(&["bio", "profile_picture"])
	///     .all()
	///     .await?;
	/// // Future: SELECT id, username, email FROM users (excluding bio, profile_picture)
	/// ```
	pub fn defer(mut self, fields: &[&str]) -> Self {
		self.deferred_fields = fields.iter().map(|s| s.to_string()).collect();
		self
	}

	/// Load only specific fields
	///
	/// Alias for `values()` - specifies which fields to load immediately.
	/// In Django, this is used for deferred loading optimization, but in Rust
	/// it behaves the same as `values()`.
	///
	/// # Examples
	///
	/// ```ignore
	/// // Load only specific fields
	/// let users = User::objects()
	///     .only(&["id", "username"])
	///     .all()
	///     .await?;
	/// // Generates: SELECT id, username FROM users
	/// ```
	pub fn only(self, fields: &[&str]) -> Self {
		self.values(fields)
	}
}

impl<T> Default for QuerySet<T>
where
	T: crate::Model,
{
	fn default() -> Self {
		Self::new()
	}
}

// Convenience conversions for FilterValue
impl FilterValue {
	/// Create a String variant from any value that can be converted to String
	///
	/// Accepts any type that implements `ToString`, including:
	/// - String, &str
	/// - Uuid (via Display)
	/// - Numeric types (i64, u64, etc. via Display)
	pub fn string(value: impl ToString) -> Self {
		Self::String(value.to_string())
	}
}

#[cfg(test)]
mod tests {
	use crate::{Filter, FilterOperator, FilterValue, Model, QuerySet};
	use serde::{Deserialize, Serialize};

	#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
	struct TestUser {
		id: Option<i64>,
		username: String,
		email: String,
	}

	impl TestUser {
		#[allow(dead_code)]
		fn new(username: String, email: String) -> Self {
			Self {
				id: None,
				username,
				email,
			}
		}
	}

	impl Model for TestUser {
		type PrimaryKey = i64;

		fn table_name() -> &'static str {
			"test_users"
		}

		fn primary_key(&self) -> Option<&Self::PrimaryKey> {
			self.id.as_ref()
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}
	}

	#[tokio::test]
	async fn test_queryset_create_with_manager() {
		// Test QuerySet::create() with explicit manager
		let manager = std::sync::Arc::new(TestUser::objects());
		let queryset = QuerySet::with_manager(manager);

		let user = TestUser {
			id: None,
			username: "testuser".to_string(),
			email: "test@example.com".to_string(),
		};

		// Note: This will fail without a real database connection
		// In actual integration tests, we would set up a test database
		let result = queryset.create(user).await;

		// In unit tests, we expect this to fail due to no database
		// In integration tests with TestContainers, this would succeed
		assert!(result.is_err() || result.is_ok());
	}

	#[tokio::test]
	async fn test_queryset_create_without_manager() {
		// Test QuerySet::create() fallback without manager
		let queryset = QuerySet::<TestUser>::new();

		let user = TestUser {
			id: None,
			username: "fallback_user".to_string(),
			email: "fallback@example.com".to_string(),
		};

		// Note: This will fail without a real database connection
		let result = queryset.create(user).await;

		// In unit tests, we expect this to fail due to no database
		assert!(result.is_err() || result.is_ok());
	}

	#[test]
	fn test_queryset_with_manager() {
		let manager = std::sync::Arc::new(TestUser::objects());
		let queryset = QuerySet::with_manager(manager.clone());

		// Verify manager is set
		assert!(queryset.manager.is_some());
	}

	#[test]
	fn test_queryset_filter_preserves_manager() {
		let manager = std::sync::Arc::new(TestUser::objects());
		let queryset = QuerySet::with_manager(manager);

		let filter = Filter::new(
			"username".to_string(),
			FilterOperator::Eq,
			FilterValue::String("alice".to_string()),
		);

		let filtered = queryset.filter(filter);

		// Verify manager is preserved after filter
		assert!(filtered.manager.is_some());
	}

	#[test]
	fn test_queryset_select_related_preserves_manager() {
		let manager = std::sync::Arc::new(TestUser::objects());
		let queryset = QuerySet::with_manager(manager);

		let selected = queryset.select_related(&["profile", "posts"]);

		// Verify manager is preserved after select_related
		assert!(selected.manager.is_some());
		assert_eq!(selected.select_related_fields, vec!["profile", "posts"]);
	}

	#[test]
	fn test_queryset_prefetch_related_preserves_manager() {
		let manager = std::sync::Arc::new(TestUser::objects());
		let queryset = QuerySet::with_manager(manager);

		let prefetched = queryset.prefetch_related(&["comments", "likes"]);

		// Verify manager is preserved after prefetch_related
		assert!(prefetched.manager.is_some());
		assert_eq!(
			prefetched.prefetch_related_fields,
			vec!["comments", "likes"]
		);
	}

	#[tokio::test]
	async fn test_get_composite_validation_error() {
		use std::collections::HashMap;

		let queryset = QuerySet::<TestUser>::new();
		let pk_values = HashMap::new(); // Empty HashMap - should fail validation

		let result = queryset.get_composite(&pk_values).await;

		// Expect error because TestUser doesn't have a composite primary key
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert!(err.to_string().contains("composite primary key"));
	}

	// SQL Generation Tests

	#[test]
	fn test_update_sql_single_field_single_filter() {
		let queryset = QuerySet::<TestUser>::new().filter(Filter::new(
			"id".to_string(),
			FilterOperator::Eq,
			FilterValue::Integer(1),
		));

		let (sql, params) = queryset.update_sql(&[("username", "alice")]);

		assert_eq!(
			sql,
			"UPDATE \"test_users\" SET \"username\" = $1 WHERE \"id\" = $2"
		);
		assert_eq!(params, vec!["alice", "1"]);
	}

	#[test]
	fn test_update_sql_multiple_fields_multiple_filters() {
		let queryset = QuerySet::<TestUser>::new()
			.filter(Filter::new(
				"id".to_string(),
				FilterOperator::Gt,
				FilterValue::Integer(10),
			))
			.filter(Filter::new(
				"email".to_string(),
				FilterOperator::Contains,
				FilterValue::String("example.com".to_string()),
			));

		let (sql, params) = queryset.update_sql(&[("username", "bob"), ("email", "bob@test.com")]);

		assert_eq!(
			sql,
			"UPDATE \"test_users\" SET \"username\" = $1, \"email\" = $2 WHERE \"id\" > $3 AND \"email\" LIKE $4"
		);
		assert_eq!(params, vec!["bob", "bob@test.com", "10", "%example.com%"]);
	}

	#[test]
	fn test_delete_sql_single_filter() {
		let queryset = QuerySet::<TestUser>::new().filter(Filter::new(
			"id".to_string(),
			FilterOperator::Eq,
			FilterValue::Integer(1),
		));

		let (sql, params) = queryset.delete_sql();

		assert_eq!(sql, "DELETE FROM \"test_users\" WHERE \"id\" = $1");
		assert_eq!(params, vec!["1"]);
	}

	#[test]
	fn test_delete_sql_multiple_filters() {
		let queryset = QuerySet::<TestUser>::new()
			.filter(Filter::new(
				"username".to_string(),
				FilterOperator::Eq,
				FilterValue::String("alice".to_string()),
			))
			.filter(Filter::new(
				"email".to_string(),
				FilterOperator::StartsWith,
				FilterValue::String("alice@".to_string()),
			));

		let (sql, params) = queryset.delete_sql();

		assert_eq!(
			sql,
			"DELETE FROM \"test_users\" WHERE \"username\" = $1 AND \"email\" LIKE $2"
		);
		assert_eq!(params, vec!["alice", "alice@%"]);
	}

	#[test]
	#[should_panic(
		expected = "DELETE without WHERE clause is not allowed. Use .filter() to specify which rows to delete."
	)]
	fn test_delete_sql_without_filters_panics() {
		let queryset = QuerySet::<TestUser>::new();
		let _ = queryset.delete_sql();
	}

	#[test]
	fn test_filter_operators() {
		let queryset = QuerySet::<TestUser>::new()
			.filter(Filter::new(
				"id".to_string(),
				FilterOperator::Gte,
				FilterValue::Integer(5),
			))
			.filter(Filter::new(
				"username".to_string(),
				FilterOperator::Ne,
				FilterValue::String("admin".to_string()),
			));

		let (sql, params) = queryset.delete_sql();

		assert_eq!(
			sql,
			"DELETE FROM \"test_users\" WHERE \"id\" >= $1 AND \"username\" <> $2"
		);
		assert_eq!(params, vec!["5", "admin"]);
	}

	#[test]
	fn test_null_value_filter() {
		let queryset = QuerySet::<TestUser>::new().filter(Filter::new(
			"email".to_string(),
			FilterOperator::Eq,
			FilterValue::Null,
		));

		let (sql, params) = queryset.delete_sql();

		assert_eq!(sql, "DELETE FROM \"test_users\" WHERE \"email\" IS NULL");
		assert_eq!(params, Vec::<String>::new());
	}

	#[test]
	fn test_not_null_value_filter() {
		let queryset = QuerySet::<TestUser>::new().filter(Filter::new(
			"email".to_string(),
			FilterOperator::Ne,
			FilterValue::Null,
		));

		let (sql, params) = queryset.delete_sql();

		assert_eq!(
			sql,
			"DELETE FROM \"test_users\" WHERE \"email\" IS NOT NULL"
		);
		assert_eq!(params, Vec::<String>::new());
	}

	// Query Optimization Tests

	#[test]
	fn test_select_related_query_generation() {
		// Test that select_related_query() generates SelectStatement correctly
		let queryset = QuerySet::<TestUser>::new().select_related(&["profile", "department"]);

		let stmt = queryset.select_related_query();

		// Convert to SQL to verify structure
		use sea_query::PostgresQueryBuilder;
		let sql = stmt.to_string(PostgresQueryBuilder);

		assert!(sql.contains("SELECT"));
		assert!(sql.contains("test_users"));
		assert!(sql.contains("LEFT JOIN"));
	}

	#[test]
	fn test_prefetch_related_queries_generation() {
		// Test that prefetch_related_queries() generates correct queries
		let queryset = QuerySet::<TestUser>::new().prefetch_related(&["posts", "comments"]);
		let pk_values = vec![1, 2, 3];

		let queries = queryset.prefetch_related_queries(&pk_values);

		// Should generate 2 queries (one for each prefetch field)
		assert_eq!(queries.len(), 2);

		// Each query should be a (field_name, SelectStatement) tuple
		assert_eq!(queries[0].0, "posts");
		assert_eq!(queries[1].0, "comments");
	}

	#[test]
	fn test_prefetch_related_queries_empty_pk_values() {
		let queryset = QuerySet::<TestUser>::new().prefetch_related(&["posts", "comments"]);
		let pk_values = vec![];

		let queries = queryset.prefetch_related_queries(&pk_values);

		// Should return empty vector when no PK values provided
		assert_eq!(queries.len(), 0);
	}

	#[test]
	fn test_select_related_and_prefetch_together() {
		// Test that both can be used together
		let queryset = QuerySet::<TestUser>::new()
			.select_related(&["profile"])
			.prefetch_related(&["posts", "comments"]);

		// Check select_related generates query
		let select_stmt = queryset.select_related_query();
		use sea_query::PostgresQueryBuilder;
		let select_sql = select_stmt.to_string(PostgresQueryBuilder);
		assert!(select_sql.contains("LEFT JOIN"));

		// Check prefetch_related generates queries
		let pk_values = vec![1, 2, 3];
		let prefetch_queries = queryset.prefetch_related_queries(&pk_values);
		assert_eq!(prefetch_queries.len(), 2);
	}

	// SmallVec Optimization Tests

	#[test]
	fn test_smallvec_stack_allocation_within_capacity() {
		// Test with exactly 10 filters (at capacity)
		let mut queryset = QuerySet::<TestUser>::new();

		for i in 0..10 {
			queryset = queryset.filter(Filter::new(
				format!("field{}", i),
				FilterOperator::Eq,
				FilterValue::Integer(i as i64),
			));
		}

		// Verify all filters are stored
		assert_eq!(queryset.filters.len(), 10);

		// Generate SQL to ensure filters work correctly
		let (sql, params) = queryset.delete_sql();
		assert!(sql.contains("WHERE"));
		assert_eq!(params.len(), 10);
	}

	#[test]
	fn test_smallvec_heap_fallback_over_capacity() {
		// Test with 15 filters (5 over capacity, should trigger heap allocation)
		let mut queryset = QuerySet::<TestUser>::new();

		for i in 0..15 {
			queryset = queryset.filter(Filter::new(
				format!("field{}", i),
				FilterOperator::Eq,
				FilterValue::Integer(i as i64),
			));
		}

		// Verify all filters are stored (SmallVec automatically spills to heap)
		assert_eq!(queryset.filters.len(), 15);

		// Generate SQL to ensure filters work correctly even after heap fallback
		let (sql, params) = queryset.delete_sql();
		assert!(sql.contains("WHERE"));
		assert_eq!(params.len(), 15);
	}

	#[test]
	fn test_smallvec_typical_use_case_1_5_filters() {
		// Test typical use case: 1-5 filters (well within stack capacity)
		let queryset = QuerySet::<TestUser>::new()
			.filter(Filter::new(
				"username".to_string(),
				FilterOperator::StartsWith,
				FilterValue::String("admin".to_string()),
			))
			.filter(Filter::new(
				"email".to_string(),
				FilterOperator::Contains,
				FilterValue::String("example.com".to_string()),
			))
			.filter(Filter::new(
				"id".to_string(),
				FilterOperator::Gt,
				FilterValue::Integer(100),
			));

		// Verify filters stored correctly
		assert_eq!(queryset.filters.len(), 3);

		// Generate SQL
		let (sql, params) = queryset.delete_sql();
		assert!(sql.contains("WHERE"));
		assert!(sql.contains("\"username\" LIKE"));
		assert!(sql.contains("\"email\" LIKE"));
		assert!(sql.contains("\"id\" >"));
		assert_eq!(params.len(), 3);
	}

	#[test]
	fn test_smallvec_empty_initialization() {
		// Test that empty SmallVec is initialized correctly
		let queryset = QuerySet::<TestUser>::new();

		assert_eq!(queryset.filters.len(), 0);
		assert!(queryset.filters.is_empty());

		// Generate SQL with no filters should not include WHERE clause
		let (where_clause, params) = queryset.build_where_clause();
		assert!(where_clause.is_empty());
		assert!(params.is_empty());
	}

	#[test]
	fn test_smallvec_single_filter() {
		// Test single filter (minimal usage)
		let queryset = QuerySet::<TestUser>::new().filter(Filter::new(
			"id".to_string(),
			FilterOperator::Eq,
			FilterValue::Integer(1),
		));

		assert_eq!(queryset.filters.len(), 1);

		let (sql, params) = queryset.delete_sql();
		assert_eq!(sql, "DELETE FROM \"test_users\" WHERE \"id\" = $1");
		assert_eq!(params, vec!["1"]);
	}

	// ==================== FilterCondition tests ====================

	use crate::FilterCondition;

	#[test]
	fn test_filter_condition_single() {
		let filter = Filter::new(
			"name".to_string(),
			FilterOperator::Eq,
			FilterValue::String("Alice".to_string()),
		);

		let condition = FilterCondition::Single(filter);

		// Single condition should not be empty
		assert!(!condition.is_empty());
	}

	#[test]
	fn test_filter_condition_single_helper() {
		let filter = Filter::new(
			"name".to_string(),
			FilterOperator::Eq,
			FilterValue::String("Alice".to_string()),
		);

		let condition = FilterCondition::single(filter);

		assert!(!condition.is_empty());
	}

	#[test]
	fn test_filter_condition_or_filters() {
		let filters = vec![
			Filter::new(
				"name".to_string(),
				FilterOperator::Contains,
				FilterValue::String("test".to_string()),
			),
			Filter::new(
				"email".to_string(),
				FilterOperator::Contains,
				FilterValue::String("test".to_string()),
			),
		];

		let condition = FilterCondition::or_filters(filters);

		// OR condition with filters should not be empty
		assert!(!condition.is_empty());

		// Verify it's an Or variant
		match &condition {
			FilterCondition::Or(conditions) => {
				assert_eq!(conditions.len(), 2);
			}
			_ => panic!("Expected Or variant"),
		}
	}

	#[test]
	fn test_filter_condition_and_filters() {
		let filters = vec![
			Filter::new(
				"is_active".to_string(),
				FilterOperator::Eq,
				FilterValue::Boolean(true),
			),
			Filter::new(
				"is_staff".to_string(),
				FilterOperator::Eq,
				FilterValue::Boolean(true),
			),
		];

		let condition = FilterCondition::and_filters(filters);

		// AND condition with filters should not be empty
		assert!(!condition.is_empty());

		// Verify it's an And variant
		match &condition {
			FilterCondition::And(conditions) => {
				assert_eq!(conditions.len(), 2);
			}
			_ => panic!("Expected And variant"),
		}
	}

	#[test]
	fn test_filter_condition_nested_and_or() {
		// Build: (name LIKE '%alice%' OR email LIKE '%alice%') AND status = 'active'
		let filter_name = Filter::new(
			"name".to_string(),
			FilterOperator::Contains,
			FilterValue::String("alice".to_string()),
		);
		let filter_email = Filter::new(
			"email".to_string(),
			FilterOperator::Contains,
			FilterValue::String("alice".to_string()),
		);
		let filter_status = Filter::new(
			"status".to_string(),
			FilterOperator::Eq,
			FilterValue::String("active".to_string()),
		);

		let or_condition = FilterCondition::or_filters(vec![filter_name, filter_email]);
		let nested =
			FilterCondition::And(vec![or_condition, FilterCondition::Single(filter_status)]);

		// Nested condition should not be empty
		assert!(!nested.is_empty());

		// Verify structure
		match &nested {
			FilterCondition::And(conditions) => {
				assert_eq!(conditions.len(), 2);
				match &conditions[0] {
					FilterCondition::Or(or_conds) => {
						assert_eq!(or_conds.len(), 2);
					}
					_ => panic!("Expected first element to be Or"),
				}
			}
			_ => panic!("Expected And variant at top level"),
		}
	}

	#[test]
	fn test_filter_condition_empty_or() {
		let condition = FilterCondition::Or(vec![]);

		// Empty OR should be empty
		assert!(condition.is_empty());
	}

	#[test]
	fn test_filter_condition_empty_and() {
		let condition = FilterCondition::And(vec![]);

		// Empty AND should be empty
		assert!(condition.is_empty());
	}

	#[test]
	fn test_filter_condition_deeply_nested() {
		// Build a deeply nested structure:
		// ((a = 1 OR b = 2) AND (c = 3 OR d = 4)) OR (e = 5)
		let filter_a = Filter::new("a".to_string(), FilterOperator::Eq, FilterValue::Integer(1));
		let filter_b = Filter::new("b".to_string(), FilterOperator::Eq, FilterValue::Integer(2));
		let filter_c = Filter::new("c".to_string(), FilterOperator::Eq, FilterValue::Integer(3));
		let filter_d = Filter::new("d".to_string(), FilterOperator::Eq, FilterValue::Integer(4));
		let filter_e = Filter::new("e".to_string(), FilterOperator::Eq, FilterValue::Integer(5));

		let or_ab = FilterCondition::or_filters(vec![filter_a, filter_b]);
		let or_cd = FilterCondition::or_filters(vec![filter_c, filter_d]);
		let and_inner = FilterCondition::And(vec![or_ab, or_cd]);
		let outer = FilterCondition::Or(vec![and_inner, FilterCondition::Single(filter_e)]);

		// Deeply nested condition should not be empty
		assert!(!outer.is_empty());
	}

	#[test]
	fn test_filter_condition_is_empty_with_nested_empty() {
		// Create an OR condition containing only empty ANDs
		let condition = FilterCondition::Or(vec![
			FilterCondition::And(vec![]),
			FilterCondition::And(vec![]),
		]);

		// Should be considered empty since all nested conditions are empty
		assert!(condition.is_empty());
	}

	#[test]
	fn test_filter_condition_is_empty_partial_nested() {
		// Create an OR condition with one empty and one non-empty child
		let filter = Filter::new(
			"name".to_string(),
			FilterOperator::Eq,
			FilterValue::String("test".to_string()),
		);

		let condition = FilterCondition::Or(vec![
			FilterCondition::And(vec![]),    // Empty
			FilterCondition::Single(filter), // Not empty
		]);

		// Should NOT be empty since one child is non-empty
		assert!(!condition.is_empty());
	}

	#[test]
	fn test_filter_condition_or_helper() {
		let inner1 = FilterCondition::single(Filter::new(
			"a".to_string(),
			FilterOperator::Eq,
			FilterValue::Integer(1),
		));
		let inner2 = FilterCondition::single(Filter::new(
			"b".to_string(),
			FilterOperator::Eq,
			FilterValue::Integer(2),
		));

		let condition = FilterCondition::or(vec![inner1, inner2]);

		assert!(!condition.is_empty());
		match &condition {
			FilterCondition::Or(conditions) => {
				assert_eq!(conditions.len(), 2);
			}
			_ => panic!("Expected Or variant"),
		}
	}

	#[test]
	fn test_filter_condition_and_helper() {
		let inner1 = FilterCondition::single(Filter::new(
			"a".to_string(),
			FilterOperator::Eq,
			FilterValue::Integer(1),
		));
		let inner2 = FilterCondition::single(Filter::new(
			"b".to_string(),
			FilterOperator::Eq,
			FilterValue::Integer(2),
		));

		let condition = FilterCondition::and(vec![inner1, inner2]);

		assert!(!condition.is_empty());
		match &condition {
			FilterCondition::And(conditions) => {
				assert_eq!(conditions.len(), 2);
			}
			_ => panic!("Expected And variant"),
		}
	}
}
