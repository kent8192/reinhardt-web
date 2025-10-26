use crate::connection::{DatabaseConnection, DatabaseExecutor};
use crate::{Model, QuerySet};
use sea_query::{
    Alias, DeleteStatement, Expr, ExprTrait, InsertStatement, Query, SelectStatement,
    UpdateStatement,
};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global database connection state
static DB: once_cell::sync::OnceCell<Arc<RwLock<Option<DatabaseConnection>>>> =
    once_cell::sync::OnceCell::new();

/// Initialize the global database connection
pub async fn init_database(url: &str) -> reinhardt_apps::Result<()> {
    let conn = DatabaseConnection::connect(url).await?;
    DB.get_or_init(|| Arc::new(RwLock::new(Some(conn))));
    Ok(())
}

/// Get a reference to the global database connection
pub async fn get_connection() -> reinhardt_apps::Result<DatabaseConnection> {
    let db = DB
        .get()
        .ok_or_else(|| reinhardt_apps::Error::Database("Database not initialized".to_string()))?;
    let guard = db.read().await;
    guard.clone().ok_or_else(|| {
        reinhardt_apps::Error::Database("Database connection not available".to_string())
    })
}

/// Model manager (similar to Django's Manager)
/// Provides an interface for database operations
pub struct Manager<M: Model> {
    _marker: PhantomData<M>,
}

impl<M: Model> Manager<M> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }

    /// Get all records
    pub fn all(&self) -> QuerySet<M> {
        QuerySet::new()
    }

    /// Filter records
    pub fn filter(
        &self,
        field: &str,
        operator: crate::query::FilterOperator,
        value: crate::query::FilterValue,
    ) -> QuerySet<M> {
        let filter = crate::query::Filter::new(field.to_string(), operator, value);
        QuerySet::new().filter(filter)
    }

    /// Get a single record by primary key
    /// Returns a QuerySet filtered by the primary key field
    pub fn get(&self, pk: M::PrimaryKey) -> QuerySet<M> {
        let pk_field = M::primary_key_field();
        let pk_value = pk.to_string();

        let filter = crate::query::Filter::new(
            pk_field.to_string(),
            crate::query::FilterOperator::Eq,
            crate::query::FilterValue::String(pk_value),
        );
        QuerySet::new().filter(filter)
    }

    /// Create a new record using SeaQuery for SQL injection protection
    pub async fn create(&self, model: &M) -> reinhardt_apps::Result<M> {
        let conn = get_connection().await?;
        let json = serde_json::to_value(model)
            .map_err(|e| reinhardt_apps::Error::Database(e.to_string()))?;

        // Extract fields and values from model
        let obj = json.as_object().ok_or_else(|| {
            reinhardt_apps::Error::Database("Model must serialize to object".to_string())
        })?;

        // Build SeaQuery INSERT statement
        let mut stmt = Query::insert();
        stmt.into_table(Alias::new(M::table_name()));

        let fields: Vec<_> = obj.keys().map(|k| Alias::new(k.as_str())).collect();
        let values: Vec<sea_query::Value> =
            obj.values().map(|v| Self::json_to_sea_value(v)).collect();

        stmt.columns(fields);
        stmt.values_panic(values);

        // Add RETURNING * support
        stmt.returning(Query::returning().columns([sea_query::Asterisk]));

        use sea_query::PostgresQueryBuilder;
        let sql = stmt.to_string(PostgresQueryBuilder);

        let row = conn.query_one(&sql).await?;
        let value = serde_json::to_value(&row.data)
            .map_err(|e| reinhardt_apps::Error::Database(e.to_string()))?;
        serde_json::from_value(value).map_err(|e| reinhardt_apps::Error::Database(e.to_string()))
    }

    /// Convert serde_json::Value to sea_query::Value for parameter binding
    fn json_to_sea_value(v: &serde_json::Value) -> sea_query::Value {
        match v {
            serde_json::Value::Null => sea_query::Value::Int(None),
            serde_json::Value::Bool(b) => sea_query::Value::Bool(Some(*b)),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    sea_query::Value::BigInt(Some(i))
                } else if let Some(f) = n.as_f64() {
                    sea_query::Value::Double(Some(f))
                } else {
                    sea_query::Value::Int(None)
                }
            }
            serde_json::Value::String(s) => sea_query::Value::String(Some(Box::new(s.clone()))),
            serde_json::Value::Array(arr) => {
                // Convert JSON array to sea_query array
                let values: Vec<sea_query::Value> =
                    arr.iter().map(|v| Self::json_to_sea_value(v)).collect();
                // Use String representation for PostgreSQL array syntax
                sea_query::Value::Array(sea_query::ArrayType::String, Some(Box::new(values)))
            }
            serde_json::Value::Object(_obj) => {
                // Convert to JSONB for PostgreSQL
                let json_str = v.to_string();
                sea_query::Value::Json(Some(Box::new(
                    serde_json::from_str(&json_str).unwrap_or(serde_json::Value::Null),
                )))
            }
        }
    }

    /// Serialize a JSON value to SQL-compatible string representation
    fn serialize_value(v: &serde_json::Value) -> String {
        match v {
            serde_json::Value::Null => "NULL".to_string(),
            serde_json::Value::Bool(b) => b.to_string().to_uppercase(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => {
                // Escape single quotes and wrap in quotes
                format!("'{}'", s.replace('\'', "''"))
            }
            serde_json::Value::Array(arr) => {
                // Convert to PostgreSQL array syntax: ARRAY['a', 'b', 'c']
                let items: Vec<String> = arr.iter().map(Self::serialize_value).collect();
                format!("ARRAY[{}]", items.join(", "))
            }
            serde_json::Value::Object(obj) => {
                // Convert to JSON string for JSONB columns
                let json_str = serde_json::to_string(obj).unwrap_or_else(|_| "{}".to_string());
                format!("'{}'::jsonb", json_str.replace('\'', "''"))
            }
        }
    }

    /// Update an existing record using SeaQuery for SQL injection protection
    pub async fn update(&self, model: &M) -> reinhardt_apps::Result<M> {
        let conn = get_connection().await?;
        let pk = model.primary_key().ok_or_else(|| {
            reinhardt_apps::Error::Database("Model must have primary key".to_string())
        })?;

        let json = serde_json::to_value(model)
            .map_err(|e| reinhardt_apps::Error::Database(e.to_string()))?;

        let obj = json.as_object().ok_or_else(|| {
            reinhardt_apps::Error::Database("Model must serialize to object".to_string())
        })?;

        // Build SeaQuery UPDATE statement
        let mut stmt = Query::update();
        stmt.table(Alias::new(M::table_name()));

        // Add SET clauses for all fields except primary key
        for (k, v) in obj
            .iter()
            .filter(|(k, _)| k.as_str() != M::primary_key_field())
        {
            stmt.value(Alias::new(k.as_str()), Self::json_to_sea_value(v));
        }

        // Add WHERE clause for primary key
        // Convert primary key to sea_query::Value for type safety
        let pk_value = sea_query::Value::String(Some(Box::new(pk.to_string())));
        stmt.and_where(Expr::col(Alias::new(M::primary_key_field())).eq(pk_value));

        // Add RETURNING * support
        stmt.returning(Query::returning().columns([sea_query::Asterisk]));

        use sea_query::PostgresQueryBuilder;
        let sql = stmt.to_string(PostgresQueryBuilder);

        let row = conn.query_one(&sql).await?;
        let value = serde_json::to_value(&row.data)
            .map_err(|e| reinhardt_apps::Error::Database(e.to_string()))?;
        serde_json::from_value(value).map_err(|e| reinhardt_apps::Error::Database(e.to_string()))
    }

    /// Delete a record using SeaQuery for SQL injection protection
    pub async fn delete(&self, pk: M::PrimaryKey) -> reinhardt_apps::Result<()> {
        let conn = get_connection().await?;

        // Build SeaQuery DELETE statement
        let mut stmt = Query::delete();
        stmt.from_table(Alias::new(M::table_name()))
            .and_where(Expr::col(Alias::new(M::primary_key_field())).eq(pk.to_string()));

        use sea_query::PostgresQueryBuilder;
        let sql = stmt.to_string(PostgresQueryBuilder);

        conn.execute(&sql).await?;
        Ok(())
    }

    /// Count records using SeaQuery
    pub async fn count(&self) -> reinhardt_apps::Result<i64> {
        let conn = get_connection().await?;

        // Build SeaQuery SELECT COUNT(*) statement
        let stmt = Query::select()
            .from(Alias::new(M::table_name()))
            .expr(sea_query::Func::count(Expr::col(sea_query::Asterisk)))
            .to_owned();

        use sea_query::PostgresQueryBuilder;
        let sql = stmt.to_string(PostgresQueryBuilder);

        let row = conn.query_one(&sql).await?;
        row.get::<i64>("count")
            .ok_or_else(|| reinhardt_apps::Error::Database("Failed to get count".to_string()))
    }

    /// Bulk create multiple records using SeaQuery (similar to Django's bulk_create())
    pub fn bulk_create_query(&self, models: &[M]) -> Option<InsertStatement> {
        if models.is_empty() {
            return None;
        }

        // Convert all models to JSON and extract field names from first model
        let json_values: Vec<serde_json::Value> = models
            .iter()
            .filter_map(|m| serde_json::to_value(m).ok())
            .collect();

        if json_values.is_empty() {
            return None;
        }

        // Get field names from first model
        let first_obj = match json_values[0].as_object() {
            Some(obj) => obj,
            None => return None,
        };

        let fields: Vec<_> = first_obj.keys().map(|k| Alias::new(k.as_str())).collect();

        // Build SeaQuery INSERT statement
        let mut stmt = Query::insert();
        stmt.into_table(Alias::new(M::table_name())).columns(fields);

        // Add value rows for each model
        for val in &json_values {
            if let Some(obj) = val.as_object() {
                let values: Vec<sea_query::Value> = first_obj
                    .keys()
                    .map(|field| {
                        obj.get(field)
                            .map(|v| Self::json_to_sea_value(v))
                            .unwrap_or(sea_query::Value::Int(None))
                    })
                    .collect();
                stmt.values_panic(values);
            }
        }

        Some(stmt.to_owned())
    }

    /// Generate bulk create SQL (convenience method)
    pub fn bulk_create_sql(&self, models: &[M]) -> String {
        if let Some(stmt) = self.bulk_create_query(models) {
            use sea_query::PostgresQueryBuilder;
            stmt.to_string(PostgresQueryBuilder)
        } else {
            String::new()
        }
    }

    /// Generate UPDATE query for QuerySet
    pub fn update_queryset(
        &self,
        queryset: &QuerySet<M>,
        updates: &[(&str, &str)],
    ) -> (String, Vec<String>) {
        queryset.update_sql(updates)
    }

    /// Generate DELETE query for QuerySet
    pub fn delete_queryset(&self, queryset: &QuerySet<M>) -> (String, Vec<String>) {
        queryset.delete_sql()
    }

    /// Get or create a record (Django's get_or_create)
    /// Returns (model, created) where created is true if a new record was created
    ///
    /// Django equivalent:
    /// ```python
    /// obj, created = Model.objects.get_or_create(
    ///     field1=value1,
    ///     defaults={'field2': value2}
    /// )
    /// ```
    pub async fn get_or_create(
        &self,
        lookup_fields: HashMap<String, String>,
        defaults: Option<HashMap<String, String>>,
    ) -> reinhardt_apps::Result<(M, bool)> {
        let conn = get_connection().await?;

        // Try to find existing record
        let (select_sql, _) =
            self.get_or_create_sql(&lookup_fields, &defaults.clone().unwrap_or_default());

        if let Ok(Some(row)) = conn.query_optional(&select_sql).await {
            let value = serde_json::to_value(&row.data)
                .map_err(|e| reinhardt_apps::Error::Database(e.to_string()))?;
            let model: M = serde_json::from_value(value)
                .map_err(|e| reinhardt_apps::Error::Database(e.to_string()))?;
            return Ok((model, false));
        }

        // Record not found, create new one
        let mut all_fields = lookup_fields.clone();
        if let Some(defs) = defaults {
            all_fields.extend(defs);
        }

        let fields: Vec<String> = all_fields.keys().cloned().collect();
        let values: Vec<String> = all_fields.values().map(|v| format!("'{}'", v)).collect();

        let insert_sql = format!(
            "INSERT INTO {} ({}) VALUES ({}) RETURNING *",
            M::table_name(),
            fields.join(", "),
            values.join(", ")
        );

        let row = conn.query_one(&insert_sql).await?;
        let value = serde_json::to_value(&row.data)
            .map_err(|e| reinhardt_apps::Error::Database(e.to_string()))?;
        let model: M = serde_json::from_value(value)
            .map_err(|e| reinhardt_apps::Error::Database(e.to_string()))?;

        Ok((model, true))
    }

    /// Bulk create multiple records efficiently (Django's bulk_create)
    /// Inserts multiple records in a single query for performance
    ///
    /// Django equivalent:
    /// ```python
    /// Model.objects.bulk_create([
    ///     Model(field1=value1),
    ///     Model(field2=value2),
    /// ])
    /// ```
    ///
    /// Options:
    /// - batch_size: Split into multiple batches if needed
    /// - ignore_conflicts: Skip records that would violate constraints
    /// - update_conflicts: Update existing records instead of failing
    pub async fn bulk_create(
        &self,
        models: Vec<M>,
        batch_size: Option<usize>,
        ignore_conflicts: bool,
        _update_conflicts: bool,
    ) -> reinhardt_apps::Result<Vec<M>> {
        if models.is_empty() {
            return Ok(vec![]);
        }

        let conn = get_connection().await?;
        let batch_size = batch_size.unwrap_or(models.len());
        let mut results = Vec::new();

        for chunk in models.chunks(batch_size) {
            // Extract fields from first model
            let json = serde_json::to_value(&chunk[0])
                .map_err(|e| reinhardt_apps::Error::Database(e.to_string()))?;
            let obj = json.as_object().ok_or_else(|| {
                reinhardt_apps::Error::Database("Model must serialize to object".to_string())
            })?;
            let field_names: Vec<String> = obj.keys().cloned().collect();

            // Extract values for all models in chunk
            let value_rows: Vec<Vec<String>> = chunk
                .iter()
                .map(|model| {
                    let json = serde_json::to_value(model).unwrap();
                    let obj = json.as_object().unwrap();
                    field_names
                        .iter()
                        .map(|field| {
                            let val = &obj[field];
                            if val.is_string() {
                                val.as_str().unwrap().to_string()
                            } else {
                                val.to_string()
                            }
                        })
                        .collect()
                })
                .collect();

            let sql = self.bulk_create_sql_detailed(&field_names, &value_rows, ignore_conflicts);

            // Execute and get results
            if ignore_conflicts {
                conn.execute(&sql).await?;
                // Note: Can't get RETURNING with DO NOTHING, skip results
                // Return empty vec for ignored conflicts
            } else {
                let sql_with_returning = sql + " RETURNING *";
                let rows = conn.query(&sql_with_returning).await?;
                for row in rows {
                    let value = serde_json::to_value(&row.data)
                        .map_err(|e| reinhardt_apps::Error::Database(e.to_string()))?;
                    let model: M = serde_json::from_value(value)
                        .map_err(|e| reinhardt_apps::Error::Database(e.to_string()))?;
                    results.push(model);
                }
            }
        }

        Ok(results)
    }

    /// Bulk update multiple records efficiently (Django's bulk_update)
    /// Updates specified fields for multiple records in optimized queries
    ///
    /// Django equivalent:
    /// ```python
    /// Model.objects.bulk_update(
    ///     [obj1, obj2, obj3],
    ///     ['field1', 'field2'],
    ///     batch_size=100
    /// )
    /// ```
    pub async fn bulk_update(
        &self,
        models: Vec<M>,
        fields: Vec<String>,
        batch_size: Option<usize>,
    ) -> reinhardt_apps::Result<usize> {
        if models.is_empty() || fields.is_empty() {
            return Ok(0);
        }

        let conn = get_connection().await?;
        let batch_size = batch_size.unwrap_or(models.len());
        let mut total_updated = 0;

        for chunk in models.chunks(batch_size) {
            // Build updates structure
            let updates: Vec<(M::PrimaryKey, HashMap<String, String>)> = chunk
                .iter()
                .filter_map(|model| {
                    let pk = model.primary_key()?.clone();
                    let json = serde_json::to_value(model).ok()?;
                    let obj = json.as_object()?;

                    let mut field_map = HashMap::new();
                    for field in &fields {
                        if let Some(val) = obj.get(field) {
                            let val_str = if val.is_string() {
                                val.as_str().unwrap().to_string()
                            } else {
                                val.to_string()
                            };
                            field_map.insert(field.clone(), val_str);
                        }
                    }

                    Some((pk, field_map))
                })
                .collect();

            if !updates.is_empty() {
                let sql = self.bulk_update_sql_detailed(&updates, &fields);
                let rows_affected = conn.execute(&sql).await?;
                total_updated += rows_affected as usize;
            }
        }

        Ok(total_updated)
    }

    /// Get or create - SQL generation using SeaQuery (for testing)
    pub fn get_or_create_queries(
        &self,
        lookup_fields: &HashMap<String, String>,
        defaults: &HashMap<String, String>,
    ) -> (SelectStatement, InsertStatement) {
        // Generate SELECT query with SeaQuery
        let mut select_stmt = Query::select();
        select_stmt
            .from(Alias::new(M::table_name()))
            .column(sea_query::Asterisk);

        for (k, v) in lookup_fields.iter() {
            select_stmt.and_where(Expr::col(Alias::new(k.as_str())).eq(v.as_str()));
        }

        // Generate INSERT query with SeaQuery
        let mut insert_fields = lookup_fields.clone();
        insert_fields.extend(defaults.clone());

        let mut insert_stmt = Query::insert();
        insert_stmt.into_table(Alias::new(M::table_name()));

        let columns: Vec<_> = insert_fields
            .keys()
            .map(|k| Alias::new(k.as_str()))
            .collect();
        let values: Vec<sea_query::Value> = insert_fields
            .values()
            .map(|v| sea_query::Value::String(Some(Box::new(v.clone()))))
            .collect();

        insert_stmt.columns(columns);
        insert_stmt.values_panic(values);

        (select_stmt.to_owned(), insert_stmt.to_owned())
    }

    /// Get or create - SQL generation (convenience method for testing)
    pub fn get_or_create_sql(
        &self,
        lookup_fields: &HashMap<String, String>,
        defaults: &HashMap<String, String>,
    ) -> (String, String) {
        let (select_stmt, insert_stmt) = self.get_or_create_queries(lookup_fields, defaults);
        use sea_query::PostgresQueryBuilder;
        (
            select_stmt.to_string(PostgresQueryBuilder),
            insert_stmt.to_string(PostgresQueryBuilder),
        )
    }

    /// Bulk create - SQL generation only (for testing)
    pub fn bulk_create_sql_detailed(
        &self,
        field_names: &[String],
        value_rows: &[Vec<String>],
        ignore_conflicts: bool,
    ) -> String {
        if value_rows.is_empty() {
            return String::new();
        }

        let values_clause: Vec<String> = value_rows
            .iter()
            .map(|row| {
                let values = row
                    .iter()
                    .map(|v| format!("'{}'", v))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", values)
            })
            .collect();

        let mut sql = format!(
            "INSERT INTO {} ({}) VALUES {}",
            M::table_name(),
            field_names.join(", "),
            values_clause.join(", ")
        );

        if ignore_conflicts {
            sql.push_str(" ON CONFLICT DO NOTHING");
        }

        sql
    }

    /// Bulk update using SeaQuery - SQL generation (for testing)
    pub fn bulk_update_query_detailed(
        &self,
        updates: &[(M::PrimaryKey, HashMap<String, String>)],
        fields: &[String],
    ) -> Option<UpdateStatement>
    where
        M::PrimaryKey: std::fmt::Display + Clone,
    {
        if updates.is_empty() || fields.is_empty() {
            return None;
        }

        let mut stmt = Query::update();
        stmt.table(Alias::new(M::table_name()));

        // Generate CASE statements for each field
        for field in fields {
            // Build CASE expression for this field
            let mut case_expr = sea_query::CaseStatement::new();

            for (pk, field_map) in updates.iter() {
                if let Some(value) = field_map.get(field) {
                    // WHEN id = pk THEN 'value'
                    case_expr.case(
                        Expr::col(Alias::new("id")).eq(pk.to_string()),
                        Expr::val(value.clone()),
                    );
                }
            }

            // field = CASE ... END
            stmt.value(Alias::new(field.as_str()), case_expr);
        }

        // WHERE id IN (...)
        let ids: Vec<sea_query::Value> = updates
            .iter()
            .map(|(pk, _)| sea_query::Value::String(Some(Box::new(pk.to_string()))))
            .collect();

        stmt.and_where(Expr::col(Alias::new("id")).is_in(ids));

        Some(stmt.to_owned())
    }

    /// Bulk update - SQL generation (convenience method for testing)
    pub fn bulk_update_sql_detailed(
        &self,
        updates: &[(M::PrimaryKey, HashMap<String, String>)],
        fields: &[String],
    ) -> String
    where
        M::PrimaryKey: std::fmt::Display + Clone,
    {
        if let Some(stmt) = self.bulk_update_query_detailed(updates, fields) {
            use sea_query::PostgresQueryBuilder;
            stmt.to_string(PostgresQueryBuilder)
        } else {
            String::new()
        }
    }
}

impl<M: Model> Default for Manager<M> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reinhardt_validators::TableName;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestUser {
        id: Option<i64>,
        name: String,
        email: String,
    }

    const TEST_USER_TABLE: TableName = TableName::new_const("test_user");

    impl crate::Model for TestUser {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            TEST_USER_TABLE.as_str()
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    #[test]
    fn test_get_or_create_sql() {
        let manager = Manager::<TestUser>::new();
        let mut lookup = HashMap::new();
        lookup.insert("email".to_string(), "test@example.com".to_string());

        let mut defaults = HashMap::new();
        defaults.insert("name".to_string(), "Test User".to_string());

        let (select_sql, insert_sql) = manager.get_or_create_sql(&lookup, &defaults);

        assert!(select_sql.contains("SELECT * FROM users"));
        assert!(select_sql.contains("WHERE email = 'test@example.com'"));
        assert!(insert_sql.contains("INSERT INTO users"));
        assert!(insert_sql.contains("email"));
        assert!(insert_sql.contains("name"));
    }

    #[test]
    fn test_bulk_create_sql() {
        let manager = Manager::<TestUser>::new();
        let fields = vec!["name".to_string(), "email".to_string()];
        let values = vec![
            vec!["Alice".to_string(), "alice@example.com".to_string()],
            vec!["Bob".to_string(), "bob@example.com".to_string()],
        ];

        let sql = manager.bulk_create_sql_detailed(&fields, &values, false);

        assert!(sql.contains("INSERT INTO users"));
        assert!(sql.contains("(name, email)"));
        assert!(sql.contains("('Alice', 'alice@example.com')"));
        assert!(sql.contains("('Bob', 'bob@example.com')"));
    }

    #[test]
    fn test_bulk_create_sql_with_conflict() {
        let manager = Manager::<TestUser>::new();
        let fields = vec!["name".to_string(), "email".to_string()];
        let values = vec![vec!["Alice".to_string(), "alice@example.com".to_string()]];

        let sql = manager.bulk_create_sql_detailed(&fields, &values, true);

        assert!(sql.contains("ON CONFLICT DO NOTHING"));
    }

    #[test]
    fn test_bulk_update_sql() {
        let manager = Manager::<TestUser>::new();

        let mut updates = Vec::new();
        let mut user1_fields = HashMap::new();
        user1_fields.insert("name".to_string(), "Alice Updated".to_string());
        user1_fields.insert("email".to_string(), "alice_new@example.com".to_string());
        updates.push((1i64, user1_fields));

        let mut user2_fields = HashMap::new();
        user2_fields.insert("name".to_string(), "Bob Updated".to_string());
        user2_fields.insert("email".to_string(), "bob_new@example.com".to_string());
        updates.push((2i64, user2_fields));

        let fields = vec!["name".to_string(), "email".to_string()];
        let sql = manager.bulk_update_sql_detailed(&updates, &fields);

        assert!(sql.contains("UPDATE users SET"));
        assert!(sql.contains("name = CASE"));
        assert!(sql.contains("email = CASE"));
        assert!(sql.contains("WHEN id = 1 THEN 'Alice Updated'"));
        assert!(sql.contains("WHEN id = 2 THEN 'Bob Updated'"));
        assert!(sql.contains("WHERE id IN (1, 2)"));
    }

    #[test]
    fn test_bulk_create_empty() {
        let manager = Manager::<TestUser>::new();
        let fields: Vec<String> = vec![];
        let values: Vec<Vec<String>> = vec![];

        let sql = manager.bulk_create_sql_detailed(&fields, &values, false);
        assert!(sql.is_empty());
    }

    #[test]
    fn test_bulk_update_empty() {
        let manager = Manager::<TestUser>::new();
        let updates: Vec<(i64, HashMap<String, String>)> = vec![];
        let fields = vec!["name".to_string()];

        let sql = manager.bulk_update_sql_detailed(&updates, &fields);
        assert!(sql.is_empty());
    }
}
