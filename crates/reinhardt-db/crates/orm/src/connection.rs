//! Database connection management

use async_trait::async_trait;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseBackend {
    Postgres,
    MySql,
    Sqlite,
}

pub struct QueryRow {
    pub data: serde_json::Value,
}

impl QueryRow {
    pub fn new(data: serde_json::Value) -> Self {
        Self { data }
    }

    pub fn get<T>(&self, _key: &str) -> Option<T> {
        None
    }
}

#[async_trait]
pub trait DatabaseExecutor: Send + Sync {
    async fn execute(&self, sql: &str) -> Result<u64, anyhow::Error>;
    async fn query(&self, sql: &str) -> Result<Vec<QueryRow>, anyhow::Error>;
}

#[derive(Clone)]
pub struct DatabaseConnection {
    backend: DatabaseBackend,
}

impl DatabaseConnection {
    pub fn new(backend: DatabaseBackend) -> Self {
        Self { backend }
    }

    pub async fn connect(_url: &str) -> Result<Self, anyhow::Error> {
        Ok(Self::new(DatabaseBackend::Postgres))
    }

    pub fn backend(&self) -> DatabaseBackend {
        self.backend
    }

    pub async fn query_one(&self, _sql: &str) -> Result<QueryRow, anyhow::Error> {
        Ok(QueryRow::new(serde_json::Value::Null))
    }

    pub async fn query_optional(&self, _sql: &str) -> Result<Option<QueryRow>, anyhow::Error> {
        Ok(None)
    }

    pub async fn execute(&self, _sql: &str) -> Result<u64, anyhow::Error> {
        Ok(0)
    }

    pub async fn query(&self, _sql: &str) -> Result<Vec<QueryRow>, anyhow::Error> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl DatabaseExecutor for DatabaseConnection {
    async fn execute(&self, _sql: &str) -> Result<u64, anyhow::Error> {
        Ok(0)
    }

    async fn query(&self, _sql: &str) -> Result<Vec<QueryRow>, anyhow::Error> {
        Ok(Vec::new())
    }
}
