//! MySQL dialect implementation

use async_trait::async_trait;
use sqlx::{mysql::MySqlRow, Column, MySqlPool, Row as SqlxRow};
use std::sync::Arc;

use crate::{
    backend::DatabaseBackend,
    error::Result,
    types::{DatabaseType, QueryResult, QueryValue, Row},
};

/// MySQL database backend
pub struct MySqlBackend {
    pool: Arc<MySqlPool>,
}

impl MySqlBackend {
    pub fn new(pool: MySqlPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    fn bind_value<'q>(
        query: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
        value: &'q QueryValue,
    ) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
        match value {
            QueryValue::Null => query.bind(None::<i32>),
            QueryValue::Bool(b) => query.bind(b),
            QueryValue::Int(i) => query.bind(i),
            QueryValue::Float(f) => query.bind(f),
            QueryValue::String(s) => query.bind(s),
            QueryValue::Bytes(b) => query.bind(b),
            QueryValue::Timestamp(dt) => query.bind(dt),
        }
    }

    fn convert_row(mysql_row: MySqlRow) -> Result<Row> {
        let mut row = Row::new();
        for column in mysql_row.columns() {
            let column_name = column.name();
            if let Ok(value) = mysql_row.try_get::<bool, _>(column_name) {
                row.insert(column_name.to_string(), QueryValue::Bool(value));
            } else if let Ok(value) = mysql_row.try_get::<i64, _>(column_name) {
                row.insert(column_name.to_string(), QueryValue::Int(value));
            } else if let Ok(value) = mysql_row.try_get::<i32, _>(column_name) {
                row.insert(column_name.to_string(), QueryValue::Int(value as i64));
            } else if let Ok(value) = mysql_row.try_get::<f64, _>(column_name) {
                row.insert(column_name.to_string(), QueryValue::Float(value));
            } else if let Ok(value) = mysql_row.try_get::<String, _>(column_name) {
                row.insert(column_name.to_string(), QueryValue::String(value));
            } else if let Ok(value) = mysql_row.try_get::<Vec<u8>, _>(column_name) {
                row.insert(column_name.to_string(), QueryValue::Bytes(value));
            } else if let Ok(value) = mysql_row.try_get::<chrono::NaiveDateTime, _>(column_name) {
                // MySQL TIMESTAMP/DATETIME without timezone
                row.insert(
                    column_name.to_string(),
                    QueryValue::Timestamp(chrono::DateTime::from_naive_utc_and_offset(
                        value,
                        chrono::Utc,
                    )),
                );
            } else if let Ok(value) =
                mysql_row.try_get::<chrono::DateTime<chrono::Utc>, _>(column_name)
            {
                row.insert(column_name.to_string(), QueryValue::Timestamp(value));
            } else if let Ok(_) = mysql_row.try_get::<Option<i32>, _>(column_name) {
                row.insert(column_name.to_string(), QueryValue::Null);
            }
        }
        Ok(row)
    }
}

#[async_trait]
impl DatabaseBackend for MySqlBackend {
    fn database_type(&self) -> DatabaseType {
        DatabaseType::Mysql
    }

    fn placeholder(&self, _index: usize) -> String {
        "?".to_string()
    }

    fn supports_returning(&self) -> bool {
        false
    }

    fn supports_on_conflict(&self) -> bool {
        false
    }

    async fn execute(&self, sql: &str, params: Vec<QueryValue>) -> Result<QueryResult> {
        let mut query = sqlx::query(sql);
        for param in &params {
            query = Self::bind_value(query, param);
        }
        let result = query.execute(self.pool.as_ref()).await?;
        Ok(QueryResult {
            rows_affected: result.rows_affected(),
        })
    }

    async fn fetch_one(&self, sql: &str, params: Vec<QueryValue>) -> Result<Row> {
        let mut query = sqlx::query(sql);
        for param in &params {
            query = Self::bind_value(query, param);
        }
        let mysql_row = query.fetch_one(self.pool.as_ref()).await?;
        Self::convert_row(mysql_row)
    }

    async fn fetch_all(&self, sql: &str, params: Vec<QueryValue>) -> Result<Vec<Row>> {
        let mut query = sqlx::query(sql);
        for param in &params {
            query = Self::bind_value(query, param);
        }
        let mysql_rows = query.fetch_all(self.pool.as_ref()).await?;
        mysql_rows.into_iter().map(Self::convert_row).collect()
    }

    async fn fetch_optional(&self, sql: &str, params: Vec<QueryValue>) -> Result<Option<Row>> {
        let mut query = sqlx::query(sql);
        for param in &params {
            query = Self::bind_value(query, param);
        }
        let mysql_row = query.fetch_optional(self.pool.as_ref()).await?;
        mysql_row.map(Self::convert_row).transpose()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
