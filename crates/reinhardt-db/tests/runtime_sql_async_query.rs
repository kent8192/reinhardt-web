//! Native backend coverage for the owned AsyncQuery execution path.
#![cfg(all(feature = "orm", feature = "postgres", feature = "mysql"))]

use reinhardt_db::orm::async_query::AsyncQuery;
use reinhardt_db::orm::engine::{Engine, EngineConfig};
use reinhardt_db::orm::expressions::Q;
use reinhardt_db::orm::model::FieldSelector;
use reinhardt_db::orm::query_execution::QueryCompiler;
use reinhardt_db::orm::types::DatabaseDialect;
use reinhardt_db::orm::{Manager, Model};
use rstest::rstest;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use testcontainers::runners::AsyncRunner;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeRow {
	id: Option<i64>,
	name: String,
}

#[derive(Clone)]
struct RuntimeFields;

impl FieldSelector for RuntimeFields {
	fn with_alias(self, _: &str) -> Self {
		self
	}
}

impl Model for RuntimeRow {
	type PrimaryKey = i64;
	type Fields = RuntimeFields;
	type Objects = Manager<Self>;

	fn table_name() -> &'static str {
		"async_runtime_rows"
	}

	fn new_fields() -> Self::Fields {
		RuntimeFields
	}

	fn primary_key(&self) -> Option<Self::PrimaryKey> {
		self.id
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}
}

async fn exercise_queries(url: &str, dialect: DatabaseDialect, insert: &str) {
	// Arrange: one pool connection isolates the fixture from database-specific state.
	sqlx::any::install_default_drivers();
	let engine = Engine::from_config(EngineConfig::new(url).with_pool_size(1, 1))
		.await
		.unwrap();
	engine
		.execute(
			"CREATE TABLE async_runtime_rows (id BIGINT PRIMARY KEY, name VARCHAR(200), age BIGINT)",
		)
		.await
		.unwrap();
	let payload = "quoted' ? $1 model";
	sqlx::query(insert)
		.bind(7_i64)
		.bind(payload)
		.bind(29_i64)
		.execute(engine.pool())
		.await
		.unwrap();
	let query = AsyncQuery::<RuntimeRow>::new(engine, QueryCompiler::new(dialect))
		.filter(Q::new("name", "=", payload))
		.filter(Q::new("age", ">=", "20"))
		.limit(5);
	// Act / Assert: each execution shape preserves text, integer and limit arguments.
	let rows = query.all().await.unwrap();
	assert_eq!(rows.len(), 1);
	assert_eq!(rows[0].try_get::<String, _>("name").unwrap(), payload);
	assert_eq!(rows[0].try_get::<i64, _>("id").unwrap(), 7);
	let first = query.first().await.unwrap().unwrap();
	assert_eq!(first.try_get::<String, _>("name").unwrap(), payload);
	let one = query.one().await.unwrap();
	assert_eq!(one.try_get::<i64, _>("age").unwrap(), 29);
	assert_eq!(query.count().await.unwrap(), 1);
	assert!(query.exists().await.unwrap());
	let absent = query.filter(Q::new("id", "=", "999"));
	assert!(absent.all().await.unwrap().is_empty());
	assert!(absent.first().await.unwrap().is_none());
	assert_eq!(absent.count().await.unwrap(), 0);
	assert!(!absent.exists().await.unwrap());
}

#[rstest]
#[tokio::test]
async fn postgres_async_queries_preserve_generated_arguments() {
	let container = testcontainers_modules::postgres::Postgres::default()
		.start()
		.await
		.unwrap();
	let url = format!(
		"postgres://postgres:postgres@{}:{}/postgres",
		container.get_host().await.unwrap(),
		container.get_host_port_ipv4(5432).await.unwrap()
	);
	exercise_queries(
		&url,
		DatabaseDialect::PostgreSQL,
		"INSERT INTO async_runtime_rows (id, name, age) VALUES ($1, $2, $3)",
	)
	.await;
}

#[rstest]
#[tokio::test]
async fn mysql_async_queries_preserve_generated_arguments() {
	let container = testcontainers_modules::mysql::Mysql::default()
		.start()
		.await
		.unwrap();
	let url = format!(
		"mysql://root@{}:{}/test",
		container.get_host().await.unwrap(),
		container.get_host_port_ipv4(3306).await.unwrap()
	);
	exercise_queries(
		&url,
		DatabaseDialect::MySQL,
		"INSERT INTO async_runtime_rows (id, name, age) VALUES (?, ?, ?)",
	)
	.await;
}
