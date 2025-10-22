//! Query builder with dialect support

use std::sync::Arc;

use sea_query::{Alias, Asterisk, Expr, ExprTrait, Query, Value};

use crate::{
    backend::DatabaseBackend,
    error::Result,
    types::{QueryResult, QueryValue, Row},
};

/// Convert QueryValue to SeaQuery Value
fn query_value_to_sea_value(qv: &QueryValue) -> Value {
    match qv {
        QueryValue::Null => Value::Bool(None),
        QueryValue::Bool(b) => Value::Bool(Some(*b)),
        QueryValue::Int(i) => Value::BigInt(Some(*i)),
        QueryValue::Float(f) => Value::Double(Some(*f)),
        QueryValue::String(s) => Value::String(Some(s.clone().into())),
        QueryValue::Bytes(b) => Value::Bytes(Some(b.clone().into())),
        // Convert timestamp to string representation for now
        QueryValue::Timestamp(dt) => Value::String(Some(dt.to_rfc3339().into())),
    }
}

/// INSERT query builder
pub struct InsertBuilder {
    backend: Arc<dyn DatabaseBackend>,
    table: String,
    columns: Vec<String>,
    values: Vec<QueryValue>,
    returning: Option<Vec<String>>,
}

impl InsertBuilder {
    pub fn new(backend: Arc<dyn DatabaseBackend>, table: impl Into<String>) -> Self {
        Self {
            backend,
            table: table.into(),
            columns: Vec::new(),
            values: Vec::new(),
            returning: None,
        }
    }

    pub fn value(mut self, column: impl Into<String>, value: impl Into<QueryValue>) -> Self {
        self.columns.push(column.into());
        self.values.push(value.into());
        self
    }

    pub fn returning(mut self, columns: Vec<&str>) -> Self {
        if self.backend.supports_returning() {
            self.returning = Some(columns.iter().map(|s| (*s).to_owned()).collect());
        }
        self
    }

    pub fn build(&self) -> (String, Vec<QueryValue>) {
        use crate::types::DatabaseType;
        use sea_query::{MysqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder};

        let mut stmt = Query::insert()
            .into_table(Alias::new(&self.table))
            .to_owned();

        // Add columns
        let column_refs: Vec<Alias> = self.columns.iter().map(|c| Alias::new(c)).collect();
        stmt.columns(column_refs);

        // Add values
        if !self.values.is_empty() {
            let sea_values: Vec<Expr> = self
                .values
                .iter()
                .map(|v| Expr::val(query_value_to_sea_value(v)))
                .collect();
            stmt.values(sea_values).unwrap();
        }

        // Add RETURNING clause if supported
        if let Some(ref cols) = self.returning {
            for col in cols {
                stmt.returning(Query::returning().column(Alias::new(col)));
            }
        }

        // Build SQL based on database type
        let sql = match self.backend.database_type() {
            DatabaseType::Postgres => stmt.to_string(PostgresQueryBuilder),
            DatabaseType::Mysql => stmt.to_string(MysqlQueryBuilder),
            DatabaseType::Sqlite => stmt.to_string(SqliteQueryBuilder),
        };

        (sql, self.values.clone())
    }

    pub async fn execute(&self) -> Result<QueryResult> {
        let (sql, params) = self.build();
        self.backend.execute(&sql, params).await
    }

    pub async fn fetch_one(&self) -> Result<Row> {
        let (sql, params) = self.build();
        self.backend.fetch_one(&sql, params).await
    }
}

/// UPDATE query builder
pub struct UpdateBuilder {
    backend: Arc<dyn DatabaseBackend>,
    table: String,
    sets: Vec<(String, QueryValue)>,
    wheres: Vec<(String, String, QueryValue)>,
}

impl UpdateBuilder {
    pub fn new(backend: Arc<dyn DatabaseBackend>, table: impl Into<String>) -> Self {
        Self {
            backend,
            table: table.into(),
            sets: Vec::new(),
            wheres: Vec::new(),
        }
    }

    pub fn set(mut self, column: impl Into<String>, value: impl Into<QueryValue>) -> Self {
        self.sets.push((column.into(), value.into()));
        self
    }

    pub fn set_now(mut self, column: impl Into<String>) -> Self {
        self.sets
            .push((column.into(), QueryValue::String("__NOW__".to_string())));
        self
    }

    pub fn where_eq(mut self, column: impl Into<String>, value: impl Into<QueryValue>) -> Self {
        self.wheres
            .push((column.into(), "=".to_string(), value.into()));
        self
    }

    pub fn build(&self) -> (String, Vec<QueryValue>) {
        use crate::types::DatabaseType;
        use sea_query::{MysqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder};

        let mut stmt = Query::update().table(Alias::new(&self.table)).to_owned();

        // Add SET clauses
        for (col, val) in &self.sets {
            if let QueryValue::String(s) = val {
                if s == "__NOW__" {
                    stmt.value(Alias::new(col), Expr::cust("NOW()"));
                    continue;
                }
            }
            stmt.value(Alias::new(col), query_value_to_sea_value(val));
        }

        // Add WHERE clauses
        for (col, op, val) in &self.wheres {
            if op == "=" {
                stmt.and_where(
                    Expr::col(Alias::new(col)).eq(Expr::val(query_value_to_sea_value(val))),
                );
            }
        }

        // Build SQL based on database type
        let sql = match self.backend.database_type() {
            DatabaseType::Postgres => stmt.to_string(PostgresQueryBuilder),
            DatabaseType::Mysql => stmt.to_string(MysqlQueryBuilder),
            DatabaseType::Sqlite => stmt.to_string(SqliteQueryBuilder),
        };

        // Preserve parameter order: first SET values, then WHERE values
        let mut params = Vec::new();
        for (_, val) in &self.sets {
            if !matches!(val, QueryValue::String(s) if s == "__NOW__") {
                params.push(val.clone());
            }
        }
        for (_, _, val) in &self.wheres {
            params.push(val.clone());
        }

        (sql, params)
    }

    pub async fn execute(&self) -> Result<QueryResult> {
        let (sql, params) = self.build();
        self.backend.execute(&sql, params).await
    }
}

/// SELECT query builder
pub struct SelectBuilder {
    backend: Arc<dyn DatabaseBackend>,
    columns: Vec<String>,
    table: String,
    wheres: Vec<(String, String, QueryValue)>,
    limit: Option<i64>,
}

impl SelectBuilder {
    pub fn new(backend: Arc<dyn DatabaseBackend>) -> Self {
        Self {
            backend,
            columns: vec!["*".to_string()],
            table: String::new(),
            wheres: Vec::new(),
            limit: None,
        }
    }

    pub fn columns(mut self, columns: Vec<&str>) -> Self {
        self.columns = columns.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn from(mut self, table: impl Into<String>) -> Self {
        self.table = table.into();
        self
    }

    pub fn where_eq(mut self, column: impl Into<String>, value: impl Into<QueryValue>) -> Self {
        self.wheres
            .push((column.into(), "=".to_string(), value.into()));
        self
    }

    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn build(&self) -> (String, Vec<QueryValue>) {
        use crate::types::DatabaseType;
        use sea_query::{MysqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder};

        let mut stmt = Query::select().from(Alias::new(&self.table)).to_owned();

        // Add columns
        if self.columns == vec!["*".to_string()] {
            stmt.column(Asterisk);
        } else {
            for col in &self.columns {
                stmt.column(Alias::new(col));
            }
        }

        // Add WHERE clauses
        for (col, op, val) in &self.wheres {
            if op == "=" {
                stmt.and_where(
                    Expr::col(Alias::new(col)).eq(Expr::val(query_value_to_sea_value(val))),
                );
            }
        }

        // Add LIMIT
        if let Some(limit) = self.limit {
            stmt.limit(limit as u64);
        }

        // Build SQL
        let sql = match self.backend.database_type() {
            DatabaseType::Postgres => stmt.to_string(PostgresQueryBuilder),
            DatabaseType::Mysql => stmt.to_string(MysqlQueryBuilder),
            DatabaseType::Sqlite => stmt.to_string(SqliteQueryBuilder),
        };

        // Collect parameters
        let params: Vec<QueryValue> = self.wheres.iter().map(|(_, _, val)| val.clone()).collect();

        (sql, params)
    }

    pub async fn fetch_all(&self) -> Result<Vec<Row>> {
        let (sql, params) = self.build();
        self.backend.fetch_all(&sql, params).await
    }

    pub async fn fetch_one(&self) -> Result<Row> {
        let (sql, params) = self.build();
        self.backend.fetch_one(&sql, params).await
    }
}

/// DELETE query builder
pub struct DeleteBuilder {
    backend: Arc<dyn DatabaseBackend>,
    table: String,
    wheres: Vec<(String, String, QueryValue)>,
}

impl DeleteBuilder {
    pub fn new(backend: Arc<dyn DatabaseBackend>, table: impl Into<String>) -> Self {
        Self {
            backend,
            table: table.into(),
            wheres: Vec::new(),
        }
    }

    pub fn where_eq(mut self, column: impl Into<String>, value: impl Into<QueryValue>) -> Self {
        self.wheres
            .push((column.into(), "=".to_string(), value.into()));
        self
    }

    pub fn where_in(mut self, column: impl Into<String> + Clone, values: Vec<QueryValue>) -> Self {
        for value in values {
            self.wheres
                .push((column.clone().into(), "IN".to_string(), value));
        }
        self
    }

    pub fn build(&self) -> (String, Vec<QueryValue>) {
        use crate::types::DatabaseType;
        use sea_query::{MysqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder};

        let mut stmt = Query::delete()
            .from_table(Alias::new(&self.table))
            .to_owned();

        // Add WHERE clauses
        for (col, op, val) in &self.wheres {
            match op.as_str() {
                "=" => {
                    stmt.and_where(
                        Expr::col(Alias::new(col)).eq(Expr::val(query_value_to_sea_value(val))),
                    );
                }
                "IN" => {
                    stmt.and_where(
                        Expr::col(Alias::new(col))
                            .is_in([Expr::val(query_value_to_sea_value(val))]),
                    );
                }
                _ => {}
            }
        }

        // Build SQL
        let sql = match self.backend.database_type() {
            DatabaseType::Postgres => stmt.to_string(PostgresQueryBuilder),
            DatabaseType::Mysql => stmt.to_string(MysqlQueryBuilder),
            DatabaseType::Sqlite => stmt.to_string(SqliteQueryBuilder),
        };

        // Collect parameters
        let params: Vec<QueryValue> = self.wheres.iter().map(|(_, _, val)| val.clone()).collect();

        (sql, params)
    }

    pub async fn execute(&self) -> Result<QueryResult> {
        let (sql, params) = self.build();
        self.backend.execute(&sql, params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::DatabaseBackend;
    use crate::types::{DatabaseType, QueryResult, QueryValue, Row};

    struct MockBackend;

    #[async_trait::async_trait]
    impl DatabaseBackend for MockBackend {
        fn database_type(&self) -> DatabaseType {
            DatabaseType::Postgres
        }

        fn placeholder(&self, index: usize) -> String {
            format!("${}", index)
        }

        fn supports_returning(&self) -> bool {
            true
        }

        fn supports_on_conflict(&self) -> bool {
            true
        }

        async fn execute(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<QueryResult> {
            Ok(QueryResult { rows_affected: 1 })
        }

        async fn fetch_one(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<Row> {
            Ok(Row::new())
        }

        async fn fetch_all(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<Vec<Row>> {
            Ok(Vec::new())
        }

        async fn fetch_optional(
            &self,
            _sql: &str,
            _params: Vec<QueryValue>,
        ) -> Result<Option<Row>> {
            Ok(None)
        }
    }

    #[test]
    fn test_delete_builder_basic() {
        let backend = Arc::new(MockBackend);
        let builder = DeleteBuilder::new(backend, "users");
        let (sql, params) = builder.build();

        // SeaQuery uses quotes for identifiers
        assert_eq!(sql, "DELETE FROM \"users\"");
        assert!(params.is_empty());
    }

    #[test]
    fn test_delete_builder_where_eq() {
        let backend = Arc::new(MockBackend);
        let builder = DeleteBuilder::new(backend, "users").where_eq("id", QueryValue::Int(1));
        let (sql, params) = builder.build();

        // SeaQuery embeds values directly in SQL when using to_string()
        assert_eq!(sql, "DELETE FROM \"users\" WHERE \"id\" = 1");
        assert_eq!(params.len(), 1);
        assert!(matches!(params[0], QueryValue::Int(1)));
    }

    #[test]
    fn test_delete_builder_where_in() {
        let backend = Arc::new(MockBackend);
        let builder = DeleteBuilder::new(backend, "users")
            .where_in("id", vec![QueryValue::Int(1), QueryValue::Int(2)]);
        let (sql, params) = builder.build();

        // SeaQuery embeds values directly in SQL when using to_string()
        assert_eq!(
            sql,
            "DELETE FROM \"users\" WHERE \"id\" IN (1) AND \"id\" IN (2)"
        );
        assert_eq!(params.len(), 2);
        assert!(matches!(params[0], QueryValue::Int(1)));
        assert!(matches!(params[1], QueryValue::Int(2)));
    }

    #[test]
    fn test_delete_builder_multiple_conditions() {
        let backend = Arc::new(MockBackend);
        let builder = DeleteBuilder::new(backend, "users")
            .where_eq("status", QueryValue::String("inactive".to_string()))
            .where_eq("age", QueryValue::Int(18));
        let (sql, params) = builder.build();

        // SeaQuery embeds values directly in SQL when using to_string()
        assert_eq!(
            sql,
            "DELETE FROM \"users\" WHERE \"status\" = 'inactive' AND \"age\" = 18"
        );
        assert_eq!(params.len(), 2);
    }
}
