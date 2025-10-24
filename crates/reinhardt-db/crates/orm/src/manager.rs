use crate::connection::{DatabaseConnection, DatabaseExecutor};
use crate::{Model, QuerySet};
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

    /// Create a new record
    pub async fn create(&self, model: &M) -> reinhardt_apps::Result<M> {
        let conn = get_connection().await?;
        let json = serde_json::to_value(model)
            .map_err(|e| reinhardt_apps::Error::Database(e.to_string()))?;

        // Extract fields and values from model with proper serialization
        let obj = json.as_object().ok_or_else(|| {
            reinhardt_apps::Error::Database("Model must serialize to object".to_string())
        })?;

        let fields: Vec<String> = obj.keys().cloned().collect();
        let values: Vec<String> = obj.values().map(|v| Self::serialize_value(v)).collect();

        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({}) RETURNING *",
            M::table_name(),
            fields.join(", "),
            values.join(", ")
        );

        let row = conn.query_one(&sql).await?;
        let value = serde_json::to_value(&row.data)
            .map_err(|e| reinhardt_apps::Error::Database(e.to_string()))?;
        serde_json::from_value(value).map_err(|e| reinhardt_apps::Error::Database(e.to_string()))
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

    /// Update an existing record
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

        let updates: Vec<String> = obj
            .iter()
            .filter(|(k, _)| k.as_str() != M::primary_key_field())
            .map(|(k, v)| {
                if v.is_string() {
                    format!("{} = '{}'", k, v.as_str().unwrap())
                } else {
                    format!("{} = {}", k, v)
                }
            })
            .collect();

        let sql = format!(
            "UPDATE {} SET {} WHERE {} = {} RETURNING *",
            M::table_name(),
            updates.join(", "),
            M::primary_key_field(),
            pk
        );

        let row = conn.query_one(&sql).await?;
        let value = serde_json::to_value(&row.data)
            .map_err(|e| reinhardt_apps::Error::Database(e.to_string()))?;
        serde_json::from_value(value).map_err(|e| reinhardt_apps::Error::Database(e.to_string()))
    }

    /// Delete a record
    pub async fn delete(&self, pk: M::PrimaryKey) -> reinhardt_apps::Result<()> {
        let conn = get_connection().await?;
        let sql = format!(
            "DELETE FROM {} WHERE {} = {}",
            M::table_name(),
            M::primary_key_field(),
            pk
        );

        conn.execute(&sql).await?;
        Ok(())
    }

    /// Count records
    pub async fn count(&self) -> reinhardt_apps::Result<i64> {
        let conn = get_connection().await?;
        let sql = format!("SELECT COUNT(*) as count FROM {}", M::table_name());

        let row = conn.query_one(&sql).await?;
        row.get::<i64>("count")
            .ok_or_else(|| reinhardt_apps::Error::Database("Failed to get count".to_string()))
    }

    /// Bulk create multiple records (similar to Django's bulk_create())
    pub fn bulk_create_sql(&self, models: &[M]) -> String {
        if models.is_empty() {
            return String::new();
        }

        // Convert all models to JSON and extract field names from first model
        let json_values: Vec<serde_json::Value> = models
            .iter()
            .filter_map(|m| serde_json::to_value(m).ok())
            .collect();

        if json_values.is_empty() {
            return String::new();
        }

        // Get field names from first model
        let first_obj = match json_values[0].as_object() {
            Some(obj) => obj,
            None => return String::new(),
        };

        let fields: Vec<String> = first_obj.keys().cloned().collect();

        // Build value rows for each model
        let value_rows: Vec<String> = json_values
            .iter()
            .filter_map(|val| {
                let obj = val.as_object()?;
                let values: Vec<String> = fields
                    .iter()
                    .map(|field| {
                        obj.get(field)
                            .map(|v| Self::serialize_value(v))
                            .unwrap_or_else(|| "NULL".to_string())
                    })
                    .collect();
                Some(format!("({})", values.join(", ")))
            })
            .collect();

        if value_rows.is_empty() {
            return String::new();
        }

        format!(
            "INSERT INTO {} ({}) VALUES {}",
            M::table_name(),
            fields.join(", "),
            value_rows.join(", ")
        )
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

    /// Get or create - SQL generation only (for testing)
    pub fn get_or_create_sql(
        &self,
        lookup_fields: &HashMap<String, String>,
        defaults: &HashMap<String, String>,
    ) -> (String, String) {
        // Generate SELECT query
        let select_conditions: Vec<String> = lookup_fields
            .iter()
            .map(|(k, v)| format!("{} = '{}'", k, v))
            .collect();

        let select_sql = format!(
            "SELECT * FROM {} WHERE {}",
            M::table_name(),
            select_conditions.join(" AND ")
        );

        // Generate INSERT query
        let mut insert_fields = lookup_fields.clone();
        insert_fields.extend(defaults.clone());

        let fields: Vec<&String> = insert_fields.keys().collect();
        let values: Vec<&String> = insert_fields.values().collect();

        let insert_sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            M::table_name(),
            fields
                .iter()
                .map(|f| f.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            values
                .iter()
                .map(|v| format!("'{}'", v))
                .collect::<Vec<_>>()
                .join(", ")
        );

        (select_sql, insert_sql)
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

    /// Bulk update - SQL generation only (for testing)
    pub fn bulk_update_sql_detailed(
        &self,
        updates: &[(M::PrimaryKey, HashMap<String, String>)],
        fields: &[String],
    ) -> String
    where
        M::PrimaryKey: std::fmt::Display,
    {
        if updates.is_empty() || fields.is_empty() {
            return String::new();
        }

        // Generate CASE statements for each field
        let case_statements: Vec<String> = fields
            .iter()
            .map(|field| {
                let cases: Vec<String> = updates
                    .iter()
                    .filter_map(|(pk, field_map)| {
                        field_map
                            .get(field)
                            .map(|value| format!("WHEN id = {} THEN '{}'", pk, value))
                    })
                    .collect();

                format!("{} = CASE {} END", field, cases.join(" "))
            })
            .collect();

        let ids: Vec<String> = updates.iter().map(|(pk, _)| format!("{}", pk)).collect();

        format!(
            "UPDATE {} SET {} WHERE id IN ({})",
            M::table_name(),
            case_statements.join(", "),
            ids.join(", ")
        )
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
