//! Unified query interface facade
//!
//! This module provides a unified entry point for querying functionality.
//! By default, it exports the expression-based query API (SQLAlchemy-style).
//! When the `django-compat` feature is enabled, it exports the Django QuerySet API.

use serde::{Deserialize, Serialize};

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
    Float(f64),
    Boolean(bool),
    Null,
}

#[derive(Debug, Clone)]
pub struct Filter {
    pub field: String,
    pub operator: FilterOperator,
    pub value: FilterValue,
}

impl Filter {
    pub fn new(field: String, operator: FilterOperator, value: FilterValue) -> Self {
        Self {
            field,
            operator,
            value,
        }
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
pub struct QuerySet<T> {
    _phantom: std::marker::PhantomData<T>,
    select_related_fields: Vec<String>,
    prefetch_related_fields: Vec<String>,
}

impl<T> QuerySet<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
            select_related_fields: Vec::new(),
            prefetch_related_fields: Vec::new(),
        }
    }

    pub fn filter(&self, _filter: Filter) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
            select_related_fields: self.select_related_fields.clone(),
            prefetch_related_fields: self.prefetch_related_fields.clone(),
        }
    }

    /// Eagerly load related objects using JOIN queries
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let posts = Post::objects()
    ///     .select_related(&["author", "category"])
    ///     .all();
    /// ```
    pub fn select_related(mut self, fields: &[&str]) -> Self {
        self.select_related_fields = fields.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Eagerly load related objects using separate queries
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let posts = Post::objects()
    ///     .prefetch_related(&["comments", "tags"])
    ///     .all();
    /// ```
    pub fn prefetch_related(mut self, fields: &[&str]) -> Self {
        self.prefetch_related_fields = fields.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn all(&self) -> Vec<T> {
        Vec::new()
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
    /// let created = User::objects().create(user)?;
    /// ```
    pub fn create(&self, object: T) -> T
    where
        T: crate::Model + Clone,
    {
        // Stub implementation - in real implementation, this would:
        // 1. Insert the object into the database
        // 2. Set the primary key from the database response
        // 3. Return the created object
        object.clone()
    }

    pub fn update_sql(&self, updates: &[(&str, &str)]) -> String {
        // Stub implementation - would generate UPDATE SQL
        let set_clause = updates
            .iter()
            .map(|(field, value)| format!("{} = '{}'", field, value))
            .collect::<Vec<_>>()
            .join(", ");
        format!("UPDATE table SET {}", set_clause)
    }

    pub fn delete_sql(&self) -> String {
        // Stub implementation - would generate DELETE SQL
        "DELETE FROM table".to_string()
    }
}

impl<T> Default for QuerySet<T> {
    fn default() -> Self {
        Self::new()
    }
}

// Export expression-based query API by default
#[cfg(not(feature = "django-compat"))]
pub use crate::sqlalchemy_query::*;
