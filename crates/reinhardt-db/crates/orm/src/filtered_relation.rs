//! # Filtered Relations
//!
//! Conditional relationship loading with dynamic filtering.
//!
//! This module implements SQLAlchemy-inspired filtered relations, allowing you to
//! create relationship JOINs with additional WHERE clause conditions. This is useful
//! for scenarios like:
//! - Loading only active/published related records
//! - Filtering related records by date ranges
//! - Creating multiple filtered views of the same relationship
//!
//! # Examples
//!
//! ```
//! use reinhardt_db::orm::filtered_relation::{FilteredRelation, FilterCondition};
//! use reinhardt_db::orm::query_fields::{LookupType, LookupValue};
//!
//! // Create a filtered relation for active posts only
//! let active_posts = FilteredRelation::new("posts")
//!     .filter("status", LookupType::Exact, LookupValue::String("active".to_string()))
//!     .filter("published", LookupType::Exact, LookupValue::Bool(true));
//!
//! // Generate SQL: WHERE status = 'active' AND published = true
//! let sql = active_posts.to_sql("posts", "p");
//! assert!(sql.contains("status = 'active'"));
//! assert!(sql.contains("published = true"));
//! ```
//!
//! This module is inspired by SQLAlchemy's with_expression and relationship loaders.
//! Copyright 2005-2025 SQLAlchemy authors and contributors
//! Licensed under MIT License. See THIRD-PARTY-NOTICES for details.

use crate::query_fields::{LookupType, LookupValue};
use std::collections::HashMap;

/// A single filter condition for a filtered relation
#[derive(Debug, Clone)]
pub struct FilterCondition {
	/// Field name to filter on
	field: String,
	/// Lookup type (exact, contains, gt, etc.)
	lookup_type: LookupType,
	/// Value to compare against
	value: LookupValue,
}

impl FilterCondition {
	/// Create a new filter condition
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::filtered_relation::FilterCondition;
	/// use reinhardt_db::orm::query_fields::{LookupType, LookupValue};
	///
	/// let condition = FilterCondition::new(
	///     "status",
	///     LookupType::Exact,
	///     LookupValue::String("active".to_string())
	/// );
	/// ```
	pub fn new(field: impl Into<String>, lookup_type: LookupType, value: LookupValue) -> Self {
		Self {
			field: field.into(),
			lookup_type,
			value,
		}
	}

	/// Get the field name
	pub fn field(&self) -> &str {
		&self.field
	}

	/// Get the lookup type
	pub fn lookup_type(&self) -> &LookupType {
		&self.lookup_type
	}

	/// Get the lookup value
	pub fn value(&self) -> &LookupValue {
		&self.value
	}

	/// Convert to SQL WHERE clause fragment
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::filtered_relation::FilterCondition;
	/// use reinhardt_db::orm::query_fields::{LookupType, LookupValue};
	///
	/// let condition = FilterCondition::new(
	///     "age",
	///     LookupType::Gt,
	///     LookupValue::Int(18)
	/// );
	/// let sql = condition.to_sql("users", "u");
	/// assert_eq!(sql, "u.age > 18");
	/// ```
	pub fn to_sql(&self, _table: &str, alias: &str) -> String {
		let field_ref = format!("{}.{}", alias, self.field);

		match &self.lookup_type {
			LookupType::Exact => format!("{} = {}", field_ref, self.format_value()),
			LookupType::IExact => format!("LOWER({}) = LOWER({})", field_ref, self.format_value()),
			LookupType::Ne => format!("{} != {}", field_ref, self.format_value()),
			LookupType::Gt => format!("{} > {}", field_ref, self.format_value()),
			LookupType::Gte => format!("{} >= {}", field_ref, self.format_value()),
			LookupType::Lt => format!("{} < {}", field_ref, self.format_value()),
			LookupType::Lte => format!("{} <= {}", field_ref, self.format_value()),
			LookupType::Contains => format!("{} LIKE '%{}%'", field_ref, self.extract_string()),
			LookupType::IContains => format!("{} ILIKE '%{}%'", field_ref, self.extract_string()),
			LookupType::StartsWith => format!("{} LIKE '{}%'", field_ref, self.extract_string()),
			LookupType::IStartsWith => format!("{} ILIKE '{}%'", field_ref, self.extract_string()),
			LookupType::EndsWith => format!("{} LIKE '%{}'", field_ref, self.extract_string()),
			LookupType::IEndsWith => format!("{} ILIKE '%{}'", field_ref, self.extract_string()),
			LookupType::IsNull => format!("{} IS NULL", field_ref),
			LookupType::IsNotNull => format!("{} IS NOT NULL", field_ref),
			LookupType::In => {
				if let LookupValue::Array(values) = &self.value {
					let formatted_values: Vec<String> =
						values.iter().map(|v| self.format_lookup_value(v)).collect();
					format!("{} IN ({})", field_ref, formatted_values.join(", "))
				} else {
					format!("{} IN ({})", field_ref, self.format_value())
				}
			}
			LookupType::NotIn => {
				if let LookupValue::Array(values) = &self.value {
					let formatted_values: Vec<String> =
						values.iter().map(|v| self.format_lookup_value(v)).collect();
					format!("{} NOT IN ({})", field_ref, formatted_values.join(", "))
				} else {
					format!("{} NOT IN ({})", field_ref, self.format_value())
				}
			}
			LookupType::Range => {
				if let LookupValue::Range(start, end) = &self.value {
					format!(
						"{} BETWEEN {} AND {}",
						field_ref,
						self.format_lookup_value(start),
						self.format_lookup_value(end)
					)
				} else {
					format!("{} BETWEEN 0 AND 0", field_ref)
				}
			}
			LookupType::Regex => format!("{} ~ '{}'", field_ref, self.extract_string()),
			LookupType::IRegex => format!("{} ~* '{}'", field_ref, self.extract_string()),
		}
	}

	/// Format the value for SQL
	fn format_value(&self) -> String {
		self.format_lookup_value(&self.value)
	}

	/// Format a LookupValue for SQL
	fn format_lookup_value(&self, value: &LookupValue) -> String {
		match value {
			LookupValue::String(s) => format!("'{}'", s.replace('\'', "''")),
			LookupValue::Int(i) => i.to_string(),
			LookupValue::Float(f) => f.to_string(),
			LookupValue::Bool(b) => if *b { "true" } else { "false" }.to_string(),
			LookupValue::Null => "NULL".to_string(),
			LookupValue::Array(_) => "()".to_string(),
			LookupValue::Range(_, _) => "".to_string(),
		}
	}

	/// Extract string value (for LIKE patterns)
	fn extract_string(&self) -> String {
		match &self.value {
			LookupValue::String(s) => s.replace('\'', "''"),
			_ => String::new(),
		}
	}
}

/// A filtered relation definition with multiple conditions
///
/// FilteredRelation allows you to create JOINs with additional WHERE clause conditions,
/// which is useful for loading only specific subsets of related records.
///
/// # Examples
///
/// ```
/// use reinhardt_db::orm::filtered_relation::FilteredRelation;
/// use reinhardt_db::orm::query_fields::{LookupType, LookupValue};
///
/// let filtered = FilteredRelation::new("comments")
///     .filter("approved", LookupType::Exact, LookupValue::Bool(true))
///     .filter("deleted", LookupType::Exact, LookupValue::Bool(false));
///
/// assert_eq!(filtered.relation_name(), "comments");
/// assert_eq!(filtered.conditions().len(), 2);
/// ```
#[derive(Debug, Clone)]
pub struct FilteredRelation {
	/// Name of the relationship to filter
	relation_name: String,
	/// List of filter conditions
	conditions: Vec<FilterCondition>,
	/// Optional alias for this filtered relation
	alias: Option<String>,
}

impl FilteredRelation {
	/// Create a new filtered relation
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::filtered_relation::FilteredRelation;
	///
	/// let filtered = FilteredRelation::new("posts");
	/// assert_eq!(filtered.relation_name(), "posts");
	/// ```
	pub fn new(relation_name: impl Into<String>) -> Self {
		Self {
			relation_name: relation_name.into(),
			conditions: Vec::new(),
			alias: None,
		}
	}

	/// Add a filter condition
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::filtered_relation::FilteredRelation;
	/// use reinhardt_db::orm::query_fields::{LookupType, LookupValue};
	///
	/// let filtered = FilteredRelation::new("posts")
	///     .filter("status", LookupType::Exact, LookupValue::String("published".to_string()));
	/// assert_eq!(filtered.conditions().len(), 1);
	/// ```
	pub fn filter(
		mut self,
		field: impl Into<String>,
		lookup_type: LookupType,
		value: LookupValue,
	) -> Self {
		self.conditions
			.push(FilterCondition::new(field, lookup_type, value));
		self
	}

	/// Set an alias for this filtered relation
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::filtered_relation::FilteredRelation;
	///
	/// let filtered = FilteredRelation::new("posts")
	///     .with_alias("active_posts");
	/// assert_eq!(filtered.alias(), Some("active_posts"));
	/// ```
	pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
		self.alias = Some(alias.into());
		self
	}

	/// Get the relation name
	pub fn relation_name(&self) -> &str {
		&self.relation_name
	}

	/// Get all filter conditions
	pub fn conditions(&self) -> &[FilterCondition] {
		&self.conditions
	}

	/// Get the alias
	pub fn alias(&self) -> Option<&str> {
		self.alias.as_deref()
	}

	/// Convert to SQL WHERE clause
	///
	/// Generates a WHERE clause combining all conditions with AND.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::filtered_relation::FilteredRelation;
	/// use reinhardt_db::orm::query_fields::{LookupType, LookupValue};
	///
	/// let filtered = FilteredRelation::new("posts")
	///     .filter("status", LookupType::Exact, LookupValue::String("active".to_string()))
	///     .filter("published", LookupType::Exact, LookupValue::Bool(true));
	///
	/// let sql = filtered.to_sql("posts", "p");
	/// assert!(sql.contains("p.status = 'active'"));
	/// assert!(sql.contains("AND"));
	/// assert!(sql.contains("p.published = true"));
	/// ```
	pub fn to_sql(&self, table: &str, alias: &str) -> String {
		if self.conditions.is_empty() {
			return String::new();
		}

		let clauses: Vec<String> = self
			.conditions
			.iter()
			.map(|c| c.to_sql(table, alias))
			.collect();

		clauses.join(" AND ")
	}

	/// Check if there are any conditions
	pub fn has_conditions(&self) -> bool {
		!self.conditions.is_empty()
	}

	/// Get the number of conditions
	pub fn condition_count(&self) -> usize {
		self.conditions.len()
	}
}

/// Builder for creating filtered relations with a fluent API
///
/// # Examples
///
/// ```
/// use reinhardt_db::orm::filtered_relation::FilteredRelationBuilder;
/// use reinhardt_db::orm::query_fields::{LookupType, LookupValue};
///
/// let filtered = FilteredRelationBuilder::new("comments")
///     .exact("status", "approved")
///     .is_true("visible")
///     .build();
///
/// assert_eq!(filtered.conditions().len(), 2);
/// ```
pub struct FilteredRelationBuilder {
	relation: FilteredRelation,
}

impl FilteredRelationBuilder {
	/// Create a new builder
	pub fn new(relation_name: impl Into<String>) -> Self {
		Self {
			relation: FilteredRelation::new(relation_name),
		}
	}

	/// Add exact match condition
	pub fn exact(mut self, field: impl Into<String>, value: impl Into<String>) -> Self {
		self.relation =
			self.relation
				.filter(field, LookupType::Exact, LookupValue::String(value.into()));
		self
	}

	/// Add greater than condition
	pub fn gt(mut self, field: impl Into<String>, value: i64) -> Self {
		self.relation = self
			.relation
			.filter(field, LookupType::Gt, LookupValue::Int(value));
		self
	}

	/// Add less than condition
	pub fn lt(mut self, field: impl Into<String>, value: i64) -> Self {
		self.relation = self
			.relation
			.filter(field, LookupType::Lt, LookupValue::Int(value));
		self
	}

	/// Add boolean true condition
	pub fn is_true(mut self, field: impl Into<String>) -> Self {
		self.relation = self
			.relation
			.filter(field, LookupType::Exact, LookupValue::Bool(true));
		self
	}

	/// Add boolean false condition
	pub fn is_false(mut self, field: impl Into<String>) -> Self {
		self.relation = self
			.relation
			.filter(field, LookupType::Exact, LookupValue::Bool(false));
		self
	}

	/// Add IS NOT NULL condition
	pub fn not_null(mut self, field: impl Into<String>) -> Self {
		self.relation = self
			.relation
			.filter(field, LookupType::IsNotNull, LookupValue::Null);
		self
	}

	/// Set alias
	pub fn alias(mut self, alias: impl Into<String>) -> Self {
		self.relation = self.relation.with_alias(alias);
		self
	}

	/// Build the filtered relation
	pub fn build(self) -> FilteredRelation {
		self.relation
	}
}

/// Registry for storing filtered relations
///
/// Allows you to define filtered relations once and reuse them across queries.
///
/// # Examples
///
/// ```
/// use reinhardt_db::orm::filtered_relation::{FilteredRelation, FilteredRelationRegistry};
/// use reinhardt_db::orm::query_fields::{LookupType, LookupValue};
///
/// let mut registry = FilteredRelationRegistry::new();
///
/// let active_posts = FilteredRelation::new("posts")
///     .filter("status", LookupType::Exact, LookupValue::String("active".to_string()));
///
/// registry.register("active_posts", active_posts);
/// assert!(registry.get("active_posts").is_some());
/// ```
#[derive(Debug, Default)]
pub struct FilteredRelationRegistry {
	relations: HashMap<String, FilteredRelation>,
}

impl FilteredRelationRegistry {
	/// Create a new registry
	pub fn new() -> Self {
		Self::default()
	}

	/// Register a filtered relation
	pub fn register(&mut self, name: impl Into<String>, relation: FilteredRelation) {
		self.relations.insert(name.into(), relation);
	}

	/// Get a filtered relation by name
	pub fn get(&self, name: &str) -> Option<&FilteredRelation> {
		self.relations.get(name)
	}

	/// Remove a filtered relation
	pub fn remove(&mut self, name: &str) -> Option<FilteredRelation> {
		self.relations.remove(name)
	}

	/// Check if a relation exists
	pub fn contains(&self, name: &str) -> bool {
		self.relations.contains_key(name)
	}

	/// Get all registered names
	pub fn names(&self) -> Vec<&String> {
		self.relations.keys().collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_filter_condition_exact() {
		let condition = FilterCondition::new(
			"status",
			LookupType::Exact,
			LookupValue::String("active".to_string()),
		);

		let sql = condition.to_sql("posts", "p");
		assert_eq!(sql, "p.status = 'active'");
	}

	#[test]
	fn test_filter_condition_gt() {
		let condition = FilterCondition::new("age", LookupType::Gt, LookupValue::Int(18));

		let sql = condition.to_sql("users", "u");
		assert_eq!(sql, "u.age > 18");
	}

	#[test]
	fn test_filter_condition_contains() {
		let condition = FilterCondition::new(
			"title",
			LookupType::Contains,
			LookupValue::String("rust".to_string()),
		);

		let sql = condition.to_sql("posts", "p");
		assert_eq!(sql, "p.title LIKE '%rust%'");
	}

	#[test]
	fn test_filter_condition_is_null() {
		let condition = FilterCondition::new("deleted_at", LookupType::IsNull, LookupValue::Null);

		let sql = condition.to_sql("posts", "p");
		assert_eq!(sql, "p.deleted_at IS NULL");
	}

	#[test]
	fn test_filter_condition_in() {
		let condition = FilterCondition::new(
			"status",
			LookupType::In,
			LookupValue::Array(vec![
				LookupValue::String("active".to_string()),
				LookupValue::String("pending".to_string()),
			]),
		);

		let sql = condition.to_sql("posts", "p");
		assert_eq!(sql, "p.status IN ('active', 'pending')");
	}

	#[test]
	fn test_filtered_relation_single_condition() {
		let filtered = FilteredRelation::new("posts").filter(
			"status",
			LookupType::Exact,
			LookupValue::String("active".to_string()),
		);

		assert_eq!(filtered.relation_name(), "posts");
		assert_eq!(filtered.conditions().len(), 1);
		assert!(filtered.has_conditions());
	}

	#[test]
	fn test_filtered_relation_multiple_conditions() {
		let filtered = FilteredRelation::new("posts")
			.filter(
				"status",
				LookupType::Exact,
				LookupValue::String("active".to_string()),
			)
			.filter("published", LookupType::Exact, LookupValue::Bool(true))
			.filter("views", LookupType::Gt, LookupValue::Int(100));

		assert_eq!(filtered.condition_count(), 3);

		let sql = filtered.to_sql("posts", "p");
		assert!(sql.contains("p.status = 'active'"));
		assert!(sql.contains("AND"));
		assert!(sql.contains("p.published = true"));
		assert!(sql.contains("p.views > 100"));
	}

	#[test]
	fn test_filtered_relation_with_alias() {
		let filtered = FilteredRelation::new("posts")
			.with_alias("active_posts")
			.filter(
				"status",
				LookupType::Exact,
				LookupValue::String("active".to_string()),
			);

		assert_eq!(filtered.alias(), Some("active_posts"));
	}

	#[test]
	fn test_filtered_relation_empty_conditions() {
		let filtered = FilteredRelation::new("posts");

		assert!(!filtered.has_conditions());
		assert_eq!(filtered.condition_count(), 0);
		assert_eq!(filtered.to_sql("posts", "p"), "");
	}

	#[test]
	fn test_filtered_relation_builder_exact() {
		let filtered = FilteredRelationBuilder::new("users")
			.exact("username", "alice")
			.build();

		assert_eq!(filtered.conditions().len(), 1);
	}

	#[test]
	fn test_filtered_relation_builder_numeric() {
		let filtered = FilteredRelationBuilder::new("products")
			.gt("price", 100)
			.lt("stock", 1000)
			.build();

		assert_eq!(filtered.condition_count(), 2);
	}

	#[test]
	fn test_filtered_relation_builder_boolean() {
		let filtered = FilteredRelationBuilder::new("posts")
			.is_true("published")
			.is_false("deleted")
			.build();

		let sql = filtered.to_sql("posts", "p");
		assert!(sql.contains("p.published = true"));
		assert!(sql.contains("p.deleted = false"));
	}

	#[test]
	fn test_filtered_relation_builder_with_alias() {
		let filtered = FilteredRelationBuilder::new("comments")
			.is_true("approved")
			.alias("approved_comments")
			.build();

		assert_eq!(filtered.alias(), Some("approved_comments"));
	}

	#[test]
	fn test_filtered_relation_builder_not_null() {
		let filtered = FilteredRelationBuilder::new("orders")
			.not_null("shipped_at")
			.build();

		let sql = filtered.to_sql("orders", "o");
		assert_eq!(sql, "o.shipped_at IS NOT NULL");
	}

	#[test]
	fn test_registry_register_and_get() {
		let mut registry = FilteredRelationRegistry::new();

		let filtered = FilteredRelation::new("posts").filter(
			"status",
			LookupType::Exact,
			LookupValue::String("active".to_string()),
		);

		registry.register("active_posts", filtered);

		let retrieved = registry.get("active_posts");
		assert!(retrieved.is_some());
		assert_eq!(retrieved.unwrap().relation_name(), "posts");
	}

	#[test]
	fn test_registry_remove() {
		let mut registry = FilteredRelationRegistry::new();

		let filtered = FilteredRelation::new("posts").filter(
			"status",
			LookupType::Exact,
			LookupValue::String("active".to_string()),
		);

		registry.register("active_posts", filtered);
		assert!(registry.contains("active_posts"));

		let removed = registry.remove("active_posts");
		assert!(removed.is_some());
		assert!(!registry.contains("active_posts"));
	}

	#[test]
	fn test_registry_names() {
		let mut registry = FilteredRelationRegistry::new();

		registry.register("active_posts", FilteredRelation::new("posts"));
		registry.register("published_posts", FilteredRelation::new("posts"));

		let names = registry.names();
		assert_eq!(names.len(), 2);
		assert!(names.contains(&&"active_posts".to_string()));
		assert!(names.contains(&&"published_posts".to_string()));
	}

	#[test]
	fn test_filter_condition_range() {
		let condition = FilterCondition::new(
			"age",
			LookupType::Range,
			LookupValue::Range(
				Box::new(LookupValue::Int(18)),
				Box::new(LookupValue::Int(65)),
			),
		);

		let sql = condition.to_sql("users", "u");
		assert_eq!(sql, "u.age BETWEEN 18 AND 65");
	}

	#[test]
	fn test_filter_condition_regex() {
		let condition = FilterCondition::new(
			"email",
			LookupType::Regex,
			LookupValue::String(".*@example\\.com$".to_string()),
		);

		let sql = condition.to_sql("users", "u");
		assert_eq!(sql, "u.email ~ '.*@example\\.com$'");
	}

	#[test]
	fn test_sql_injection_protection() {
		let condition = FilterCondition::new(
			"name",
			LookupType::Exact,
			LookupValue::String("O'Brien".to_string()),
		);

		let sql = condition.to_sql("users", "u");
		assert_eq!(sql, "u.name = 'O''Brien'");
	}
}
