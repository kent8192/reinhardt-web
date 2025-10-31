//! PostgreSQL-specific advanced features
//!
//! This module provides PostgreSQL-specific advanced query features inspired by
//! Django's `django/contrib/postgres/aggregates/` and `django/contrib/postgres/search/`.
//!
//! # Available Features
//!
//! - **ArrayAgg**: Array aggregation function
//! - **JsonbBuildObject**: JSONB object construction
//! - **FullTextSearch**: Full-text search functionality
//! - **ArrayOverlap**: Array overlap operations
//!
//! # Example
//!
//! ```rust
//! use reinhardt_orm::{ArrayAgg, FullTextSearch};
//!
//! // Aggregate values into an array
//! let agg = ArrayAgg::<String>::new("tags".to_string()).distinct();
//! assert!(agg.to_sql().contains("ARRAY_AGG(DISTINCT"));
//!
//! // Full-text search
//! let search = FullTextSearch::new("content".to_string(), "rust programming".to_string());
//! assert!(search.to_sql().contains("to_tsvector"));
//! ```

use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// PostgreSQL ARRAY_AGG aggregation function
///
/// Aggregates values into a PostgreSQL array.
///
/// # Example
///
/// ```rust
/// use reinhardt_orm::ArrayAgg;
///
/// let agg = ArrayAgg::<i32>::new("score".to_string());
/// assert_eq!(agg.to_sql(), "ARRAY_AGG(score)");
///
/// let distinct_agg = ArrayAgg::<String>::new("category".to_string()).distinct();
/// assert_eq!(distinct_agg.to_sql(), "ARRAY_AGG(DISTINCT category)");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrayAgg<T> {
    field: String,
    distinct: bool,
    ordering: Option<Vec<String>>,
    _phantom: PhantomData<T>,
}

impl<T> ArrayAgg<T> {
    /// Create a new ArrayAgg for the specified field
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_orm::ArrayAgg;
    ///
    /// let agg = ArrayAgg::<String>::new("name".to_string());
    /// assert_eq!(agg.to_sql(), "ARRAY_AGG(name)");
    /// ```
    pub fn new(field: String) -> Self {
        Self {
            field,
            distinct: false,
            ordering: None,
            _phantom: PhantomData,
        }
    }

    /// Apply DISTINCT to the aggregation
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_orm::ArrayAgg;
    ///
    /// let agg = ArrayAgg::<i32>::new("id".to_string()).distinct();
    /// assert!(agg.to_sql().contains("DISTINCT"));
    /// ```
    pub fn distinct(mut self) -> Self {
        self.distinct = true;
        self
    }

    /// Add ORDER BY clause to the aggregation
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_orm::ArrayAgg;
    ///
    /// let agg = ArrayAgg::<String>::new("name".to_string())
    ///     .order_by(vec!["created_at DESC".to_string()]);
    /// assert!(agg.to_sql().contains("ORDER BY"));
    /// ```
    pub fn order_by(mut self, fields: Vec<String>) -> Self {
        self.ordering = Some(fields);
        self
    }

    /// Generate SQL for this aggregation
    pub fn to_sql(&self) -> String {
        let mut sql = String::from("ARRAY_AGG(");

        if self.distinct {
            sql.push_str("DISTINCT ");
        }

        sql.push_str(&self.field);

        if let Some(ref ordering) = self.ordering {
            sql.push_str(" ORDER BY ");
            sql.push_str(&ordering.join(", "));
        }

        sql.push(')');
        sql
    }
}

/// PostgreSQL JSONB_BUILD_OBJECT function
///
/// Constructs a JSONB object from key-value pairs.
///
/// # Example
///
/// ```rust
/// use reinhardt_orm::JsonbBuildObject;
///
/// let builder = JsonbBuildObject::new()
///     .add("id", "user_id")
///     .add("name", "user_name");
/// assert!(builder.to_sql().contains("jsonb_build_object"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonbBuildObject {
    pairs: Vec<(String, String)>,
}

impl JsonbBuildObject {
    /// Create a new JSONB object builder
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_orm::JsonbBuildObject;
    ///
    /// let builder = JsonbBuildObject::new();
    /// assert_eq!(builder.to_sql(), "jsonb_build_object()");
    /// ```
    pub fn new() -> Self {
        Self { pairs: Vec::new() }
    }

    /// Add a key-value pair to the JSONB object
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_orm::JsonbBuildObject;
    ///
    /// let builder = JsonbBuildObject::new()
    ///     .add("user_id", "id")
    ///     .add("user_name", "name");
    /// let sql = builder.to_sql();
    /// assert!(sql.contains("'user_id'"));
    /// assert!(sql.contains("id"));
    /// ```
    pub fn add(mut self, key: &str, value_field: &str) -> Self {
        self.pairs.push((key.to_string(), value_field.to_string()));
        self
    }

    /// Generate SQL for this JSONB object construction
    pub fn to_sql(&self) -> String {
        let mut sql = String::from("jsonb_build_object(");

        let parts: Vec<String> = self
            .pairs
            .iter()
            .flat_map(|(k, v)| vec![format!("'{}'", k), v.clone()])
            .collect();

        sql.push_str(&parts.join(", "));
        sql.push(')');
        sql
    }
}

impl Default for JsonbBuildObject {
    fn default() -> Self {
        Self::new()
    }
}

/// PostgreSQL Full-Text Search
///
/// Provides full-text search capabilities using PostgreSQL's tsvector and tsquery.
///
/// # Example
///
/// ```rust
/// use reinhardt_orm::FullTextSearch;
///
/// let search = FullTextSearch::new("content".to_string(), "rust programming".to_string());
/// assert!(search.to_sql().contains("to_tsvector"));
/// assert!(search.to_sql().contains("to_tsquery"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullTextSearch {
    vector_field: String,
    query: String,
    config: String,
}

impl FullTextSearch {
    /// Create a new full-text search with default English configuration
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_orm::FullTextSearch;
    ///
    /// let search = FullTextSearch::new("title".to_string(), "database".to_string());
    /// assert_eq!(search.config(), "english");
    /// ```
    pub fn new(field: String, query: String) -> Self {
        Self {
            vector_field: field,
            query,
            config: "english".to_string(),
        }
    }

    /// Set a custom text search configuration (language)
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_orm::FullTextSearch;
    ///
    /// let search = FullTextSearch::new("content".to_string(), "bonjour".to_string())
    ///     .with_config("french".to_string());
    /// assert_eq!(search.config(), "french");
    /// ```
    pub fn with_config(mut self, config: String) -> Self {
        self.config = config;
        self
    }

    /// Get the current configuration
    pub fn config(&self) -> &str {
        &self.config
    }

    /// Generate SQL for this full-text search
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_orm::FullTextSearch;
    ///
    /// let search = FullTextSearch::new("body".to_string(), "rust".to_string());
    /// let sql = search.to_sql();
    /// assert!(sql.contains("to_tsvector('english', body)"));
    /// assert!(sql.contains("to_tsquery('english', 'rust')"));
    /// ```
    pub fn to_sql(&self) -> String {
        format!(
            "to_tsvector('{}', {}) @@ to_tsquery('{}', '{}')",
            self.config, self.vector_field, self.config, self.query
        )
    }
}

/// PostgreSQL Array Overlap Operator
///
/// Tests whether two arrays have any elements in common.
///
/// # Example
///
/// ```rust
/// use reinhardt_orm::ArrayOverlap;
///
/// let overlap = ArrayOverlap::new("tags".to_string(), vec!["rust".to_string(), "web".to_string()]);
/// assert!(overlap.to_sql().contains("&&"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrayOverlap {
    field: String,
    values: Vec<String>,
}

impl ArrayOverlap {
    /// Create a new array overlap check
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_orm::ArrayOverlap;
    ///
    /// let overlap = ArrayOverlap::new(
    ///     "categories".to_string(),
    ///     vec!["tech".to_string(), "science".to_string()]
    /// );
    /// assert!(overlap.to_sql().contains("ARRAY"));
    /// ```
    pub fn new(field: String, values: Vec<String>) -> Self {
        Self { field, values }
    }

    /// Generate SQL for the array overlap check
    pub fn to_sql(&self) -> String {
        let array_literal = format!(
            "ARRAY[{}]",
            self.values
                .iter()
                .map(|v| format!("'{}'", v))
                .collect::<Vec<_>>()
                .join(", ")
        );
        format!("{} && {}", self.field, array_literal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array_agg_basic() {
        let agg = ArrayAgg::<i32>::new("score".to_string());
        assert_eq!(agg.to_sql(), "ARRAY_AGG(score)");
    }

    #[test]
    fn test_array_agg_distinct() {
        let agg = ArrayAgg::<String>::new("category".to_string()).distinct();
        assert_eq!(agg.to_sql(), "ARRAY_AGG(DISTINCT category)");
    }

    #[test]
    fn test_array_agg_with_ordering() {
        let agg =
            ArrayAgg::<i32>::new("id".to_string()).order_by(vec!["created_at DESC".to_string()]);
        assert_eq!(agg.to_sql(), "ARRAY_AGG(id ORDER BY created_at DESC)");
    }

    #[test]
    fn test_array_agg_distinct_with_ordering() {
        let agg = ArrayAgg::<String>::new("name".to_string())
            .distinct()
            .order_by(vec!["name ASC".to_string(), "id DESC".to_string()]);
        assert_eq!(
            agg.to_sql(),
            "ARRAY_AGG(DISTINCT name ORDER BY name ASC, id DESC)"
        );
    }

    #[test]
    fn test_jsonb_build_object_empty() {
        let builder = JsonbBuildObject::new();
        assert_eq!(builder.to_sql(), "jsonb_build_object()");
    }

    #[test]
    fn test_jsonb_build_object_single_pair() {
        let builder = JsonbBuildObject::new().add("id", "user_id");
        assert_eq!(builder.to_sql(), "jsonb_build_object('id', user_id)");
    }

    #[test]
    fn test_jsonb_build_object_multiple_pairs() {
        let builder = JsonbBuildObject::new()
            .add("id", "user_id")
            .add("name", "user_name")
            .add("email", "user_email");
        assert_eq!(
            builder.to_sql(),
            "jsonb_build_object('id', user_id, 'name', user_name, 'email', user_email)"
        );
    }

    #[test]
    fn test_full_text_search_basic() {
        let search = FullTextSearch::new("content".to_string(), "rust".to_string());
        assert_eq!(
            search.to_sql(),
            "to_tsvector('english', content) @@ to_tsquery('english', 'rust')"
        );
    }

    #[test]
    fn test_full_text_search_custom_config() {
        let search = FullTextSearch::new("title".to_string(), "database".to_string())
            .with_config("french".to_string());
        assert_eq!(
            search.to_sql(),
            "to_tsvector('french', title) @@ to_tsquery('french', 'database')"
        );
    }

    #[test]
    fn test_full_text_search_complex_query() {
        let search = FullTextSearch::new("body".to_string(), "rust & programming".to_string());
        let sql = search.to_sql();
        assert!(sql.contains("to_tsvector('english', body)"));
        assert!(sql.contains("to_tsquery('english', 'rust & programming')"));
    }

    #[test]
    fn test_array_overlap_basic() {
        let overlap = ArrayOverlap::new(
            "tags".to_string(),
            vec!["rust".to_string(), "web".to_string()],
        );
        assert_eq!(overlap.to_sql(), "tags && ARRAY['rust', 'web']");
    }

    #[test]
    fn test_array_overlap_single_value() {
        let overlap = ArrayOverlap::new("categories".to_string(), vec!["tech".to_string()]);
        assert_eq!(overlap.to_sql(), "categories && ARRAY['tech']");
    }

    #[test]
    fn test_array_overlap_multiple_values() {
        let overlap = ArrayOverlap::new(
            "labels".to_string(),
            vec![
                "important".to_string(),
                "urgent".to_string(),
                "reviewed".to_string(),
            ],
        );
        assert_eq!(
            overlap.to_sql(),
            "labels && ARRAY['important', 'urgent', 'reviewed']"
        );
    }

    #[test]
    fn test_array_agg_type_safety() {
        let int_agg = ArrayAgg::<i32>::new("scores".to_string());
        let string_agg = ArrayAgg::<String>::new("names".to_string());

        assert_eq!(int_agg.to_sql(), "ARRAY_AGG(scores)");
        assert_eq!(string_agg.to_sql(), "ARRAY_AGG(names)");
    }

    #[test]
    fn test_jsonb_build_object_default() {
        let builder = JsonbBuildObject::default();
        assert_eq!(builder.to_sql(), "jsonb_build_object()");
    }

    #[test]
    fn test_full_text_search_config_getter() {
        let search = FullTextSearch::new("text".to_string(), "query".to_string());
        assert_eq!(search.config(), "english");

        let search_fr = search.with_config("french".to_string());
        assert_eq!(search_fr.config(), "french");
    }
}
