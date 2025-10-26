//! # Query Compilation and Execution
//!
//! Compile and execute queries against the database.
//!
//! This module is inspired by SQLAlchemy's query execution patterns
//! Copyright 2005-2025 SQLAlchemy authors and contributors
//! Licensed under MIT License. See THIRD-PARTY-NOTICES for details.

use crate::Model;
use crate::engine::Engine;
use crate::expressions::Q;
use crate::types::DatabaseDialect;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;

/// Query compiler - converts query structures to SQL
#[derive(Debug, Clone)]
pub struct QueryCompiler {
    dialect: DatabaseDialect,
}

impl QueryCompiler {
    /// Create a new query compiler
    pub fn new(dialect: DatabaseDialect) -> Self {
        Self { dialect }
    }
    /// Compile a SELECT query
    ///
    pub fn compile_select<T: Model>(
        &self,
        table: &str,
        columns: &[&str],
        where_clause: Option<&Q>,
        order_by: &[&str],
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> String {
        let cols = if columns.is_empty() {
            "*".to_string()
        } else {
            columns.join(", ")
        };

        let mut sql = format!("SELECT {} FROM {}", cols, table);

        if let Some(q) = where_clause {
            sql.push_str(&format!(" WHERE {}", q.to_sql()));
        }

        if !order_by.is_empty() {
            sql.push_str(&format!(" ORDER BY {}", order_by.join(", ")));
        }

        if let Some(lim) = limit {
            sql.push_str(&format!(" LIMIT {}", lim));
        }

        if let Some(off) = offset {
            sql.push_str(&format!(" OFFSET {}", off));
        }

        sql
    }
    /// Compile an INSERT query
    ///
    pub fn compile_insert<T: Model>(
        &self,
        table: &str,
        columns: &[&str],
        values: &[&str],
    ) -> String {
        format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table,
            columns.join(", "),
            values.join(", ")
        )
    }
    /// Compile an UPDATE query
    ///
    pub fn compile_update<T: Model>(
        &self,
        table: &str,
        updates: &[(&str, &str)],
        where_clause: Option<&Q>,
    ) -> String {
        let sets: Vec<String> = updates
            .iter()
            .map(|(k, v)| format!("{} = {}", k, v))
            .collect();

        let mut sql = format!("UPDATE {} SET {}", table, sets.join(", "));

        if let Some(q) = where_clause {
            sql.push_str(&format!(" WHERE {}", q.to_sql()));
        }

        sql
    }
    /// Compile a DELETE query
    ///
    pub fn compile_delete<T: Model>(&self, table: &str, where_clause: Option<&Q>) -> String {
        let mut sql = format!("DELETE FROM {}", table);

        if let Some(q) = where_clause {
            sql.push_str(&format!(" WHERE {}", q.to_sql()));
        }

        sql
    }
    /// Get the current dialect
    ///
    pub fn dialect(&self) -> DatabaseDialect {
        self.dialect
    }
}

/// Executable query - compiled query ready to execute
pub struct ExecutableQuery<T: Model> {
    sql: String,
    engine: Option<Engine>,
    _phantom: PhantomData<T>,
}

impl<T: Model> ExecutableQuery<T> {
    /// Create a new executable query
    pub fn new(sql: impl Into<String>) -> Self {
        Self {
            sql: sql.into(),
            engine: None,
            _phantom: PhantomData,
        }
    }
    /// Bind an engine to this query
    pub fn with_engine(mut self, engine: Engine) -> Self {
        self.engine = Some(engine);
        self
    }
    /// Get the SQL string
    ///
    pub fn sql(&self) -> &str {
        &self.sql
    }
    /// Execute the query and return affected rows
    ///
    pub async fn execute(&self) -> Result<u64, sqlx::Error> {
        match &self.engine {
            Some(engine) => engine.execute(&self.sql).await,
            None => Err(sqlx::Error::Configuration(
                "No engine bound to query".into(),
            )),
        }
    }
    /// Execute the query and fetch all results
    ///
    pub async fn fetch_all(&self) -> Result<Vec<sqlx::any::AnyRow>, sqlx::Error>
    where
        T: DeserializeOwned,
    {
        match &self.engine {
            Some(engine) => engine.fetch_all(&self.sql).await,
            None => Err(sqlx::Error::Configuration(
                "No engine bound to query".into(),
            )),
        }
    }
    /// Execute the query and fetch one result
    ///
    pub async fn fetch_one(&self) -> Result<sqlx::any::AnyRow, sqlx::Error>
    where
        T: DeserializeOwned,
    {
        match &self.engine {
            Some(engine) => engine.fetch_one(&self.sql).await,
            None => Err(sqlx::Error::Configuration(
                "No engine bound to query".into(),
            )),
        }
    }
    /// Execute the query and fetch optional result
    ///
    pub async fn fetch_optional(&self) -> Result<Option<sqlx::any::AnyRow>, sqlx::Error>
    where
        T: DeserializeOwned,
    {
        match &self.engine {
            Some(engine) => engine.fetch_optional(&self.sql).await,
            None => Err(sqlx::Error::Configuration(
                "No engine bound to query".into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reinhardt_validators::TableName;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestModel {
        id: Option<i64>,
        name: String,
    }

    const TEST_MODEL_TABLE: TableName = TableName::new_const("test_model");

    impl Model for TestModel {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            TEST_MODEL_TABLE.as_str()
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    #[test]
    fn test_compile_select() {
        let compiler = QueryCompiler::new(DatabaseDialect::SQLite);
        let sql = compiler.compile_select::<TestModel>(
            "test_models",
            &["id", "name"],
            None,
            &[],
            None,
            None,
        );

        assert_eq!(sql, "SELECT id, name FROM test_models");
    }

    #[test]
    fn test_compile_select_with_where() {
        let compiler = QueryCompiler::new(DatabaseDialect::SQLite);
        let q = Q::new("age", ">=", "18");
        let sql =
            compiler.compile_select::<TestModel>("test_models", &[], Some(&q), &[], None, None);

        assert!(sql.contains("WHERE age >= 18"));
    }

    #[test]
    fn test_compile_select_with_limit_offset() {
        let compiler = QueryCompiler::new(DatabaseDialect::SQLite);
        let sql = compiler.compile_select::<TestModel>(
            "test_models",
            &[],
            None,
            &["id"],
            Some(10),
            Some(20),
        );

        assert!(sql.contains("LIMIT 10"));
        assert!(sql.contains("OFFSET 20"));
        assert!(sql.contains("ORDER BY id"));
    }

    #[test]
    fn test_compile_insert() {
        let compiler = QueryCompiler::new(DatabaseDialect::SQLite);
        let sql =
            compiler.compile_insert::<TestModel>("test_models", &["id", "name"], &["1", "'Alice'"]);

        assert_eq!(
            sql,
            "INSERT INTO test_models (id, name) VALUES (1, 'Alice')"
        );
    }

    #[test]
    fn test_compile_update() {
        let compiler = QueryCompiler::new(DatabaseDialect::SQLite);
        let q = Q::new("id", "=", "1");
        let sql = compiler.compile_update::<TestModel>(
            "test_models",
            &[("name", "'Bob'"), ("age", "25")],
            Some(&q),
        );

        assert!(sql.contains("UPDATE test_models"));
        assert!(sql.contains("SET name = 'Bob', age = 25"));
        assert!(sql.contains("WHERE id = 1"));
    }

    #[test]
    fn test_compile_delete() {
        let compiler = QueryCompiler::new(DatabaseDialect::SQLite);
        let q = Q::new("active", "=", "0");
        let sql = compiler.compile_delete::<TestModel>("test_models", Some(&q));

        assert_eq!(sql, "DELETE FROM test_models WHERE active = 0");
    }

    #[test]
    fn test_executable_query() {
        let query = ExecutableQuery::<TestModel>::new("SELECT * FROM test_models");
        assert_eq!(query.sql(), "SELECT * FROM test_models");
    }
}
