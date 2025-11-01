//! Database backend abstraction

use async_trait::async_trait;

use crate::{
	error::Result,
	types::{DatabaseType, QueryResult, QueryValue, Row},
};

/// Core database backend trait
#[async_trait]
pub trait DatabaseBackend: Send + Sync {
	/// Returns the database type
	fn database_type(&self) -> DatabaseType;

	/// Generates a placeholder for the given parameter index (1-based)
	fn placeholder(&self, index: usize) -> String;

	/// Returns whether the database supports RETURNING clause
	fn supports_returning(&self) -> bool;

	/// Returns whether the database supports ON CONFLICT clause
	fn supports_on_conflict(&self) -> bool;

	/// Executes a query that modifies the database
	async fn execute(&self, sql: &str, params: Vec<QueryValue>) -> Result<QueryResult>;

	/// Fetches a single row from the database
	async fn fetch_one(&self, sql: &str, params: Vec<QueryValue>) -> Result<Row>;

	/// Fetches all matching rows from the database
	async fn fetch_all(&self, sql: &str, params: Vec<QueryValue>) -> Result<Vec<Row>>;

	/// Fetches an optional single row from the database
	async fn fetch_optional(&self, sql: &str, params: Vec<QueryValue>) -> Result<Option<Row>>;

	/// Returns self as &dyn std::any::Any for downcasting
	fn as_any(&self) -> &dyn std::any::Any;
}
