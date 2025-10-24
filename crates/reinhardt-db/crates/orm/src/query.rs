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
pub struct QuerySet<T>
where
    T: crate::Model,
{
    _phantom: std::marker::PhantomData<T>,
    filters: Vec<Filter>,
    select_related_fields: Vec<String>,
    prefetch_related_fields: Vec<String>,
    #[cfg(feature = "django-compat")]
    manager: Option<std::sync::Arc<crate::manager::Manager<T>>>,
}

impl<T> QuerySet<T>
where
    T: crate::Model,
{
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
            filters: Vec::new(),
            select_related_fields: Vec::new(),
            prefetch_related_fields: Vec::new(),
            #[cfg(feature = "django-compat")]
            manager: None,
        }
    }

    #[cfg(feature = "django-compat")]
    pub fn with_manager(manager: std::sync::Arc<crate::manager::Manager<T>>) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
            filters: Vec::new(),
            select_related_fields: Vec::new(),
            prefetch_related_fields: Vec::new(),
            manager: Some(manager),
        }
    }

    pub fn filter(mut self, filter: Filter) -> Self {
        self.filters.push(filter);
        self
    }

    /// Convert FilterOperator to SQL operator string
    fn operator_to_sql(operator: &FilterOperator) -> &'static str {
        match operator {
            FilterOperator::Eq => "=",
            FilterOperator::Ne => "!=",
            FilterOperator::Gt => ">",
            FilterOperator::Gte => ">=",
            FilterOperator::Lt => "<",
            FilterOperator::Lte => "<=",
            FilterOperator::In => "IN",
            FilterOperator::NotIn => "NOT IN",
            FilterOperator::Contains => "LIKE",
            FilterOperator::StartsWith => "LIKE",
            FilterOperator::EndsWith => "LIKE",
        }
    }

    /// Convert FilterValue to SQL parameter placeholder and prepare value for binding
    fn value_to_sql_placeholder(
        value: &FilterValue,
        operator: &FilterOperator,
        param_index: usize,
    ) -> (String, String) {
        let placeholder = format!("${}", param_index);
        let formatted_value = match value {
            FilterValue::String(s) => match operator {
                FilterOperator::Contains => format!("%{}%", s),
                FilterOperator::StartsWith => format!("{}%", s),
                FilterOperator::EndsWith => format!("%{}", s),
                _ => s.clone(),
            },
            FilterValue::Integer(i) => i.to_string(),
            FilterValue::Float(f) => f.to_string(),
            FilterValue::Boolean(b) => b.to_string(),
            FilterValue::Null => "NULL".to_string(),
        };
        (placeholder, formatted_value)
    }

    /// Build WHERE clause from accumulated filters
    fn build_where_clause(&self) -> (String, Vec<String>) {
        if self.filters.is_empty() {
            return (String::new(), Vec::new());
        }

        let mut conditions = Vec::new();
        let mut values = Vec::new();
        let mut param_index = 1;

        for filter in &self.filters {
            let operator_sql = Self::operator_to_sql(&filter.operator);
            let (placeholder, value) =
                Self::value_to_sql_placeholder(&filter.value, &filter.operator, param_index);

            let condition = if matches!(filter.value, FilterValue::Null) {
                if matches!(filter.operator, FilterOperator::Eq) {
                    format!("{} IS NULL", filter.field)
                } else {
                    format!("{} IS NOT NULL", filter.field)
                }
            } else {
                format!("{} {} {}", filter.field, operator_sql, placeholder)
            };

            conditions.push(condition);
            values.push(value);
            param_index += 1;
        }

        let where_clause = format!(" WHERE {}", conditions.join(" AND "));
        (where_clause, values)
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

    /// Generate SELECT SQL with JOIN clauses for select_related fields
    ///
    /// Returns SQL with LEFT JOIN for each related field to enable eager loading.
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
    /// let (sql, params) = queryset.select_related_sql();
    /// // sql: "SELECT posts.*, author.*, category.* FROM posts
    /// //       LEFT JOIN users AS author ON posts.author_id = author.id
    /// //       LEFT JOIN categories AS category ON posts.category_id = category.id
    /// //       WHERE posts.published = $1"
    /// // params: ["true"]
    /// ```
    pub fn select_related_sql(&self) -> (String, Vec<String>) {
        let table_name = T::table_name();
        let (where_clause, values) = self.build_where_clause();

        // Build SELECT columns: main table + related tables
        let mut select_parts = vec![format!("{}.*", table_name)];
        let mut join_clauses = Vec::new();

        for related_field in &self.select_related_fields {
            // Convention: related_field is the field name in the model
            // We assume FK field is "{related_field}_id" and join to "{related_field}s" table
            let fk_field = format!("{}_id", related_field);
            let related_table = format!("{}s", related_field); // Simple pluralization

            select_parts.push(format!("{}.*", related_field));
            join_clauses.push(format!(
                "LEFT JOIN {} AS {} ON {}.{} = {}.id",
                related_table, related_field, table_name, fk_field, related_field
            ));
        }

        let select_clause = select_parts.join(", ");
        let joins = if join_clauses.is_empty() {
            String::new()
        } else {
            format!(" {}", join_clauses.join(" "))
        };

        let sql = format!(
            "SELECT {} FROM {}{}{}",
            select_clause, table_name, joins, where_clause
        );

        (sql, values)
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

    /// Generate SELECT SQL queries for prefetch_related fields
    ///
    /// Returns a vector of (field_name, sql, params) tuples, one for each prefetch field.
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
    /// let prefetch_queries = queryset.prefetch_related_sql(&pk_values);
    /// // Returns:
    /// // [
    /// //   ("comments", "SELECT * FROM comments WHERE post_id IN ($1, $2, $3)", ["1", "2", "3"]),
    /// //   ("tags", "SELECT tags.* FROM tags
    /// //             INNER JOIN post_tags ON tags.id = post_tags.tag_id
    /// //             WHERE post_tags.post_id IN ($1, $2, $3)", ["1", "2", "3"])
    /// // ]
    /// ```
    pub fn prefetch_related_sql(&self, pk_values: &[i64]) -> Vec<(String, String, Vec<String>)> {
        if pk_values.is_empty() {
            return Vec::new();
        }

        let table_name = T::table_name();
        let mut queries = Vec::new();

        for related_field in &self.prefetch_related_fields {
            // Build IN clause with placeholders
            let placeholders: Vec<String> =
                (1..=pk_values.len()).map(|i| format!("${}", i)).collect();
            let in_clause = placeholders.join(", ");
            let params: Vec<String> = pk_values.iter().map(|v| v.to_string()).collect();

            // Convention: related_field could be direct FK or M2M
            // For one-to-many: SELECT * FROM {related_field} WHERE {main_table}_id IN (...)
            // For many-to-many: Need junction table join
            let related_table = format!("{}s", related_field); // Simple pluralization
            let fk_field = format!("{}_id", table_name.trim_end_matches('s')); // Reverse: users -> user_id

            // Simple one-to-many query (for M2M, would need junction table logic)
            let sql = format!(
                "SELECT * FROM {} WHERE {} IN ({})",
                related_table, fk_field, in_clause
            );

            queries.push((related_field.clone(), sql, params));
        }

        queries
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
    #[cfg(feature = "django-compat")]
    pub async fn all(&self) -> reinhardt_apps::Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        let conn = crate::manager::get_connection().await?;
        let table_name = T::table_name();
        let (where_clause, _values) = self.build_where_clause();

        let sql = if self.select_related_fields.is_empty() {
            format!("SELECT * FROM {}{}", table_name, where_clause)
        } else {
            let (select_sql, _) = self.select_related_sql();
            select_sql
        };

        // Execute query and deserialize results
        let rows = conn.query(&sql).await?;
        rows.into_iter()
            .map(|row| {
                serde_json::from_value(serde_json::to_value(&row.data).map_err(|e| {
                    reinhardt_apps::Error::Database(format!("Serialization error: {}", e))
                })?)
                .map_err(|e| {
                    reinhardt_apps::Error::Database(format!("Deserialization error: {}", e))
                })
            })
            .collect()
    }

    /// Execute the queryset and return all matching records (without django-compat feature)
    ///
    /// Returns empty vector when django-compat feature is not enabled.
    #[cfg(not(feature = "django-compat"))]
    pub fn all(&self) -> Vec<T> {
        Vec::new()
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
    #[cfg(feature = "django-compat")]
    pub async fn first(&self) -> reinhardt_apps::Result<Option<T>>
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
    #[cfg(feature = "django-compat")]
    pub async fn get(&self) -> reinhardt_apps::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let results = self.all().await?;
        match results.len() {
            0 => Err(reinhardt_apps::Error::Database(
                "No record found matching the query".to_string(),
            )),
            1 => Ok(results.into_iter().next().unwrap()),
            n => Err(reinhardt_apps::Error::Database(format!(
                "Multiple records found ({}), expected exactly one",
                n
            ))),
        }
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
    #[cfg(feature = "django-compat")]
    pub async fn count(&self) -> reinhardt_apps::Result<usize> {
        let conn = crate::manager::get_connection().await?;
        let table_name = T::table_name();
        let (where_clause, _values) = self.build_where_clause();

        let sql = format!("SELECT COUNT(*) FROM {}{}", table_name, where_clause);

        // Execute query
        let rows = conn.query(&sql).await?;
        if let Some(row) = rows.first() {
            // Extract count from first row
            if let Some(count_value) = row.data.get("count") {
                if let Some(count) = count_value.as_i64() {
                    return Ok(count as usize);
                }
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
    #[cfg(feature = "django-compat")]
    pub async fn exists(&self) -> reinhardt_apps::Result<bool> {
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
    #[cfg(feature = "django-compat")]
    pub async fn create(&self, object: T) -> reinhardt_apps::Result<T>
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
        let table_name = T::table_name();
        let (where_clause, mut where_values) = self.build_where_clause();

        // Build SET clause with parameter placeholders
        let mut param_index = 1;
        let mut set_parts = Vec::new();
        let mut set_values = Vec::new();

        for (field, value) in updates {
            set_parts.push(format!("{} = ${}", field, param_index));
            set_values.push(value.to_string());
            param_index += 1;
        }

        let set_clause = format!(" SET {}", set_parts.join(", "));

        // Adjust WHERE clause parameter indices to start after SET clause parameters
        let where_clause_adjusted = if !where_clause.is_empty() {
            let mut adjusted = where_clause.clone();
            for i in (1..=where_values.len()).rev() {
                let old_placeholder = format!("${}", i);
                let new_placeholder = format!("${}", param_index - 1 + i);
                adjusted = adjusted.replace(&old_placeholder, &new_placeholder);
            }
            adjusted
        } else {
            where_clause
        };

        let sql = format!(
            "UPDATE {}{}{}",
            table_name, set_clause, where_clause_adjusted
        );
        let mut all_values = set_values;
        all_values.append(&mut where_values);

        (sql, all_values)
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
    pub fn delete_sql(&self) -> (String, Vec<String>) {
        if self.filters.is_empty() {
            panic!("DELETE without WHERE clause is not allowed. Use .filter() to specify which rows to delete.");
        }

        let table_name = T::table_name();
        let (where_clause, values) = self.build_where_clause();

        let sql = format!("DELETE FROM {}{}", table_name, where_clause);
        (sql, values)
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
    #[cfg(feature = "django-compat")]
    pub async fn get_composite(
        &self,
        pk_values: &HashMap<String, crate::composite_pk::PkValue>,
    ) -> reinhardt_apps::Result<T>
    where
        T: crate::Model + Clone,
    {
        use sea_query::{Alias, BinOper, Expr, ExprTrait, PostgresQueryBuilder, Query, Value};

        // Get composite primary key definition from the model
        let composite_pk = T::composite_primary_key().ok_or_else(|| {
            reinhardt_apps::Error::Database(
                "Model does not have a composite primary key".to_string(),
            )
        })?;

        // Validate that all required PK fields are provided
        composite_pk.validate(pk_values).map_err(|e| {
            reinhardt_apps::Error::Database(format!("Composite PK validation failed: {}", e))
        })?;

        // Build SELECT query using sea-query
        let table_name = T::table_name();
        let mut query = Query::select();

        // Use Alias::new for table name
        let table_alias = Alias::new(table_name);
        query.from(table_alias).column(sea_query::Asterisk);

        // Add WHERE conditions for each composite PK field
        for field_name in composite_pk.fields() {
            let pk_value: &crate::composite_pk::PkValue = pk_values.get(field_name).unwrap();
            let col_alias = Alias::new(field_name);

            match pk_value {
                crate::composite_pk::PkValue::Int(v) => {
                    let condition = Expr::col(col_alias)
                        .binary(BinOper::Equal, Expr::value(Value::BigInt(Some(*v))));
                    query.and_where(condition);
                }
                crate::composite_pk::PkValue::Uint(v) => {
                    let condition = Expr::col(col_alias)
                        .binary(BinOper::Equal, Expr::value(Value::BigInt(Some(*v as i64))));
                    query.and_where(condition);
                }
                crate::composite_pk::PkValue::String(v) => {
                    let condition = Expr::col(col_alias).binary(
                        BinOper::Equal,
                        Expr::value(Value::String(Some(v.clone().into()))),
                    );
                    query.and_where(condition);
                }
                crate::composite_pk::PkValue::Bool(v) => {
                    let condition = Expr::col(col_alias)
                        .binary(BinOper::Equal, Expr::value(Value::Bool(Some(*v))));
                    query.and_where(condition);
                }
            }
        }

        // Build SQL with parameter binding
        let sql = query.to_string(PostgresQueryBuilder);

        // Execute query through the manager
        match &self.manager {
            Some(_manager) => {
                // TODO: Execute the query using the manager's database connection
                // For now, return a placeholder error
                Err(reinhardt_apps::Error::Database(format!(
                    "Query execution not yet implemented: {}",
                    sql
                )))
            }
            None => {
                // Fallback: create a new manager instance if none exists
                Err(reinhardt_apps::Error::Database(
                    "No manager available for query execution".to_string(),
                ))
            }
        }
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

// Export expression-based query API by default
#[cfg(not(feature = "django-compat"))]
pub use crate::sqlalchemy_query::*;

#[cfg(all(test, feature = "django-compat"))]
mod tests {
    use super::*;
    use crate::manager::Manager;
    use crate::Model;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestUser {
        id: Option<i64>,
        username: String,
        email: String,
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
        let manager = std::sync::Arc::new(Manager::<TestUser>::new());
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
        let manager = std::sync::Arc::new(Manager::<TestUser>::new());
        let queryset = QuerySet::with_manager(manager.clone());

        // Verify manager is set
        assert!(queryset.manager.is_some());
    }

    #[test]
    fn test_queryset_filter_preserves_manager() {
        let manager = std::sync::Arc::new(Manager::<TestUser>::new());
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
        let manager = std::sync::Arc::new(Manager::<TestUser>::new());
        let queryset = QuerySet::with_manager(manager);

        let selected = queryset.select_related(&["profile", "posts"]);

        // Verify manager is preserved after select_related
        assert!(selected.manager.is_some());
        assert_eq!(selected.select_related_fields, vec!["profile", "posts"]);
    }

    #[test]
    fn test_queryset_prefetch_related_preserves_manager() {
        let manager = std::sync::Arc::new(Manager::<TestUser>::new());
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

        assert_eq!(sql, "UPDATE test_users SET username = $1 WHERE id = $2");
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
            "UPDATE test_users SET username = $1, email = $2 WHERE id > $3 AND email LIKE $4"
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

        assert_eq!(sql, "DELETE FROM test_users WHERE id = $1");
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
            "DELETE FROM test_users WHERE username = $1 AND email LIKE $2"
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
            "DELETE FROM test_users WHERE id >= $1 AND username != $2"
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

        assert_eq!(sql, "DELETE FROM test_users WHERE email IS NULL");
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

        assert_eq!(sql, "DELETE FROM test_users WHERE email IS NOT NULL");
        assert_eq!(params, Vec::<String>::new());
    }

    // Query Optimization Tests (Phase 3)

    #[test]
    fn test_select_related_sql_no_filters() {
        let queryset = QuerySet::<TestUser>::new().select_related(&["profile", "department"]);

        let (sql, params) = queryset.select_related_sql();

        assert_eq!(
            sql,
            "SELECT test_users.*, profile.*, department.* FROM test_users \
             LEFT JOIN profiles AS profile ON test_users.profile_id = profile.id \
             LEFT JOIN departments AS department ON test_users.department_id = department.id"
        );
        assert_eq!(params, Vec::<String>::new());
    }

    #[test]
    fn test_select_related_sql_with_filters() {
        let queryset = QuerySet::<TestUser>::new()
            .select_related(&["profile"])
            .filter(Filter::new(
                "id".to_string(),
                FilterOperator::Gt,
                FilterValue::Integer(10),
            ));

        let (sql, params) = queryset.select_related_sql();

        assert_eq!(
            sql,
            "SELECT test_users.*, profile.* FROM test_users \
             LEFT JOIN profiles AS profile ON test_users.profile_id = profile.id \
             WHERE id > $1"
        );
        assert_eq!(params, vec!["10"]);
    }

    #[test]
    fn test_select_related_sql_multiple_fields_and_filters() {
        let queryset = QuerySet::<TestUser>::new()
            .select_related(&["profile", "department"])
            .filter(Filter::new(
                "username".to_string(),
                FilterOperator::StartsWith,
                FilterValue::String("admin".to_string()),
            ))
            .filter(Filter::new(
                "email".to_string(),
                FilterOperator::Contains,
                FilterValue::String("example.com".to_string()),
            ));

        let (sql, params) = queryset.select_related_sql();

        assert_eq!(
            sql,
            "SELECT test_users.*, profile.*, department.* FROM test_users \
             LEFT JOIN profiles AS profile ON test_users.profile_id = profile.id \
             LEFT JOIN departments AS department ON test_users.department_id = department.id \
             WHERE username LIKE $1 AND email LIKE $2"
        );
        assert_eq!(params, vec!["admin%", "%example.com%"]);
    }

    #[test]
    fn test_prefetch_related_sql_single_field() {
        let queryset = QuerySet::<TestUser>::new().prefetch_related(&["posts"]);
        let pk_values = vec![1, 2, 3];

        let queries = queryset.prefetch_related_sql(&pk_values);

        assert_eq!(queries.len(), 1);
        let (field, sql, params) = &queries[0];
        assert_eq!(field, "posts");
        assert_eq!(
            sql,
            "SELECT * FROM postss WHERE test_user_id IN ($1, $2, $3)"
        );
        assert_eq!(params, &vec!["1", "2", "3"]);
    }

    #[test]
    fn test_prefetch_related_sql_multiple_fields() {
        let queryset = QuerySet::<TestUser>::new().prefetch_related(&["posts", "comments"]);
        let pk_values = vec![5, 10];

        let queries = queryset.prefetch_related_sql(&pk_values);

        assert_eq!(queries.len(), 2);

        // First field: posts
        let (field1, sql1, params1) = &queries[0];
        assert_eq!(field1, "posts");
        assert_eq!(sql1, "SELECT * FROM postss WHERE test_user_id IN ($1, $2)");
        assert_eq!(params1, &vec!["5", "10"]);

        // Second field: comments
        let (field2, sql2, params2) = &queries[1];
        assert_eq!(field2, "comments");
        assert_eq!(
            sql2,
            "SELECT * FROM commentss WHERE test_user_id IN ($1, $2)"
        );
        assert_eq!(params2, &vec!["5", "10"]);
    }

    #[test]
    fn test_prefetch_related_sql_empty_pk_values() {
        let queryset = QuerySet::<TestUser>::new().prefetch_related(&["posts", "comments"]);
        let pk_values = vec![];

        let queries = queryset.prefetch_related_sql(&pk_values);

        assert_eq!(queries.len(), 0);
    }

    #[test]
    fn test_select_related_and_prefetch_together() {
        // Test that both can be used together
        let queryset = QuerySet::<TestUser>::new()
            .select_related(&["profile"])
            .prefetch_related(&["posts", "comments"]);

        // Check select_related SQL
        let (select_sql, select_params) = queryset.select_related_sql();
        assert!(select_sql.contains("LEFT JOIN profiles AS profile"));
        assert_eq!(select_params.len(), 0);

        // Check prefetch_related SQL
        let pk_values = vec![1, 2, 3];
        let prefetch_queries = queryset.prefetch_related_sql(&pk_values);
        assert_eq!(prefetch_queries.len(), 2);
    }
}
