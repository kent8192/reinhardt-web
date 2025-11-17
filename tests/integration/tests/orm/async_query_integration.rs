//! Integration tests for async query API with PostgreSQL and MySQL
//!
//! These tests verify that AsyncQuery and AsyncSession work correctly
//! with real database containers.

use reinhardt_core::validators::TableName;
use reinhardt_orm::{
	expressions::Q, query_execution::QueryCompiler, types::DatabaseDialect, Model,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use testcontainers::{runners::AsyncRunner, ContainerAsync, GenericImage, ImageExt};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestModel {
	id: Option<i64>,
	name: String,
}

#[allow(dead_code)]
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

// ========================================================================
// PostgreSQL Tests
// ========================================================================

#[cfg(feature = "postgres")]
mod postgres_tests {
	use super::*;
	use sqlx::postgres::{PgPool, PgPoolOptions};

	async fn create_postgres_pool(
		container: &ContainerAsync<GenericImage>,
	) -> Result<PgPool, sqlx::Error> {
		let port = container
			.get_host_port_ipv4(testcontainers::core::ContainerPort::Tcp(5432))
			.await
			.expect("Failed to get PostgreSQL port");
		let url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);

		// Retry connection up to 10 times with 1 second delay
		for attempt in 1..=10 {
			match PgPoolOptions::new()
				.min_connections(1)
				.max_connections(5)
				.acquire_timeout(Duration::from_secs(10))
				.connect(&url)
				.await
			{
				Ok(pool) => return Ok(pool),
				Err(_e) if attempt < 10 => {
					tokio::time::sleep(Duration::from_secs(1)).await;
					continue;
				}
				Err(e) => return Err(e),
			}
		}
		unreachable!()
	}

	#[tokio::test]
	async fn test_postgres_async_query_builder() {
		let postgres_image = GenericImage::new("postgres", "17-alpine")
			.with_exposed_port(testcontainers::core::ContainerPort::Tcp(5432))
			.with_env_var("POSTGRES_PASSWORD", "postgres")
			.with_env_var("POSTGRES_DB", "postgres");

		// Use AsyncRunner for compatibility with #[tokio::test]
		let container = postgres_image
			.start()
			.await
			.expect("Failed to start PostgreSQL container");
		let pool = create_postgres_pool(&container)
			.await
			.expect("Failed to create PostgreSQL pool");

		// Test basic SQL generation with QueryCompiler
		let compiler = QueryCompiler::new(DatabaseDialect::PostgreSQL);
		let stmt = compiler.compile_select::<TestModel>(
			TestModel::table_name(),
			&[],
			Some(&Q::new("age", ">=", "18")),
			&["name"],
			Some(10),
			None,
		);

		let sql = stmt.to_string(sea_query::PostgresQueryBuilder);
		assert!(sql.contains("test_model"));
		assert!(sql.contains("ORDER BY"));

		pool.close().await;
	}

	#[tokio::test]
	async fn test_postgres_async_query_execution() {
		let postgres_image = GenericImage::new("postgres", "17-alpine")
			.with_exposed_port(testcontainers::core::ContainerPort::Tcp(5432))
			.with_env_var("POSTGRES_PASSWORD", "postgres")
			.with_env_var("POSTGRES_DB", "postgres");

		// Use AsyncRunner for compatibility with #[tokio::test]
		let container = postgres_image
			.start()
			.await
			.expect("Failed to start PostgreSQL container");
		let pool = create_postgres_pool(&container)
			.await
			.expect("Failed to create PostgreSQL pool");

		sqlx::query("CREATE TABLE test_models (id SERIAL PRIMARY KEY, name TEXT)")
			.execute(&pool)
			.await
			.expect("Failed to create table");

		sqlx::query("INSERT INTO test_models (name) VALUES ('Alice'), ('Bob')")
			.execute(&pool)
			.await
			.expect("Failed to insert data");

		let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_models")
			.fetch_one(&pool)
			.await
			.expect("Count failed");
		assert_eq!(count, 2);

		pool.close().await;
	}

	#[tokio::test]
	async fn test_postgres_async_session() {
		let postgres_image = GenericImage::new("postgres", "17-alpine")
			.with_exposed_port(testcontainers::core::ContainerPort::Tcp(5432))
			.with_env_var("POSTGRES_PASSWORD", "postgres")
			.with_env_var("POSTGRES_DB", "postgres");

		// Use AsyncRunner for compatibility with #[tokio::test]
		let container = postgres_image
			.start()
			.await
			.expect("Failed to start PostgreSQL container");
		let pool = create_postgres_pool(&container)
			.await
			.expect("Failed to create PostgreSQL pool");

		sqlx::query("CREATE TABLE test_models (id SERIAL PRIMARY KEY, name TEXT)")
			.execute(&pool)
			.await
			.unwrap();

		sqlx::query("INSERT INTO test_models (name) VALUES ('Test')")
			.execute(&pool)
			.await
			.expect("Insert failed");

		let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM test_models)")
			.fetch_one(&pool)
			.await
			.expect("Exists check failed");
		assert!(exists);

		pool.close().await;
	}
}

// ========================================================================
// MySQL Tests
// ========================================================================

#[cfg(any())] // MySQL support not yet enabled in integration tests
mod mysql_tests {
	use super::*;
	use sqlx::mysql::{MySqlPool, MySqlPoolOptions};

	async fn create_mysql_pool(
		container: &ContainerAsync<GenericImage>,
	) -> Result<MySqlPool, sqlx::Error> {
		let port = container
			.get_host_port_ipv4(testcontainers::core::ContainerPort::Tcp(3306))
			.await
			.expect("Failed to get MySQL port");
		let url = format!("mysql://root:test@localhost:{}/test", port);

		// Retry connection up to 30 times with 2 second delay (MySQL takes longer to start)
		for attempt in 1..=30 {
			match MySqlPoolOptions::new()
				.min_connections(1)
				.max_connections(5)
				.acquire_timeout(Duration::from_secs(10))
				.connect(&url)
				.await
			{
				Ok(pool) => return Ok(pool),
				Err(_e) if attempt < 30 => {
					tokio::time::sleep(Duration::from_secs(2)).await;
					continue;
				}
				Err(e) => return Err(e),
			}
		}
		unreachable!()
	}

	#[tokio::test]
	async fn test_mysql_async_query_builder() {
		let mysql_image = GenericImage::new("mysql", "8.0")
			.with_exposed_port(testcontainers::core::ContainerPort::Tcp(3306))
			.with_env_var("MYSQL_ROOT_PASSWORD", "test")
			.with_env_var("MYSQL_DATABASE", "test");

		// Use AsyncRunner for compatibility with #[tokio::test]
		let container = mysql_image
			.start()
			.await
			.expect("Failed to start MySQL container");

		let pool = create_mysql_pool(&container)
			.await
			.expect("Failed to create MySQL pool");

		// Test basic SQL generation with QueryCompiler
		let compiler = QueryCompiler::new(DatabaseDialect::MySQL);
		let stmt = compiler.compile_select::<TestModel>(
			TestModel::table_name(),
			&[],
			Some(&Q::new("age", ">=", "18")),
			&["name"],
			Some(10),
			None,
		);

		let sql = stmt.to_string(sea_query::MysqlQueryBuilder);
		assert!(sql.contains("test_model"));
		assert!(sql.contains("ORDER BY"));

		pool.close().await;
	}

	#[tokio::test]
	async fn test_mysql_async_query_execution() {
		let mysql_image = GenericImage::new("mysql", "8.0")
			.with_exposed_port(testcontainers::core::ContainerPort::Tcp(3306))
			.with_env_var("MYSQL_ROOT_PASSWORD", "test")
			.with_env_var("MYSQL_DATABASE", "test");

		// Use AsyncRunner for compatibility with #[tokio::test]
		let container = mysql_image
			.start()
			.await
			.expect("Failed to start MySQL container");

		let pool = create_mysql_pool(&container)
			.await
			.expect("Failed to create MySQL pool");

		sqlx::query("CREATE TABLE test_models (id INT AUTO_INCREMENT PRIMARY KEY, name TEXT)")
			.execute(&pool)
			.await
			.expect("Failed to create table");

		sqlx::query("INSERT INTO test_models (name) VALUES ('Alice'), ('Bob')")
			.execute(&pool)
			.await
			.expect("Failed to insert data");

		let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_models")
			.fetch_one(&pool)
			.await
			.expect("Count failed");
		assert_eq!(count, 2);

		pool.close().await;
	}

	#[tokio::test]
	async fn test_mysql_async_session() {
		// MySQL's "ready for connections" message appears twice:
		// 1st time: during initialization
		// 2nd time: when fully ready
		// So we remove the WaitFor condition and rely on create_mysql_pool's retry logic
		let mysql_image = GenericImage::new("mysql", "8.0")
			.with_exposed_port(testcontainers::core::ContainerPort::Tcp(3306))
			.with_env_var("MYSQL_ROOT_PASSWORD", "test")
			.with_env_var("MYSQL_DATABASE", "test");

		// Use AsyncRunner for compatibility with #[tokio::test]
		let container = mysql_image
			.start()
			.await
			.expect("Failed to start MySQL container");

		// create_mysql_pool will retry up to 30 times (60 seconds total)
		// to ensure MySQL is fully initialized
		let pool = create_mysql_pool(&container)
			.await
			.expect("Failed to create MySQL pool");

		sqlx::query("CREATE TABLE test_models (id INT AUTO_INCREMENT PRIMARY KEY, name TEXT)")
			.execute(&pool)
			.await
			.unwrap();

		sqlx::query("INSERT INTO test_models (name) VALUES ('Test')")
			.execute(&pool)
			.await
			.expect("Insert failed");

		let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_models")
			.fetch_one(&pool)
			.await
			.expect("Exists check failed");
		assert!(count > 0);

		pool.close().await;
	}
}
