//! Native Session regressions for portable escaped case-insensitive lookups.
#![cfg(all(
	feature = "orm",
	feature = "postgres",
	feature = "mysql",
	feature = "sqlite",
	not(all(target_family = "wasm", target_os = "unknown"))
))]

use reinhardt_db::orm::inspection::FieldInfo;
use reinhardt_db::orm::query_types::DbBackend;
use reinhardt_db::orm::session::Session;
use reinhardt_db::orm::{
	FieldSelector, Filter, FilterOperator, FilterValue, Manager, Model, QuerySet,
};
use rstest::{fixture, rstest};
use serde::{Deserialize, Serialize};
use sqlx::{AnyPool, any::AnyPoolOptions};
use std::{collections::HashMap, sync::Arc, time::Duration};
use testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner};
use testcontainers_modules::{mysql::Mysql, postgres::Postgres};

#[derive(Clone)]
struct Fields;

impl FieldSelector for Fields {
	fn with_alias(self, _alias: &str) -> Self {
		self
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Record {
	id: i64,
	#[serde(rename = "na\"me`")]
	name: String,
}

impl Model for Record {
	type PrimaryKey = i64;
	type Fields = Fields;
	type Objects = Manager<Self>;

	fn table_name() -> &'static str {
		"case\"lookup`"
	}
	fn new_fields() -> Self::Fields {
		Fields
	}
	fn primary_key(&self) -> Option<i64> {
		Some(self.id)
	}
	fn set_primary_key(&mut self, id: i64) {
		self.id = id;
	}
	fn field_metadata() -> Vec<FieldInfo> {
		[("id", "BigIntegerField"), ("na\"me`", "CharField")]
			.into_iter()
			.map(|(name, field_type)| FieldInfo {
				name: name.into(),
				field_type: field_type.into(),
				nullable: false,
				primary_key: name == "id",
				unique: false,
				blank: false,
				editable: true,
				default: None,
				db_default: None,
				db_column: None,
				choices: None,
				attributes: HashMap::new(),
				domain: None,
				storage_kind: None,
			})
			.collect()
	}
}

async fn connect(url: &str) -> Arc<AnyPool> {
	sqlx::any::install_default_drivers();
	Arc::new(
		AnyPoolOptions::new()
			.max_connections(1)
			.connect(url)
			.await
			.unwrap(),
	)
}

#[fixture]
async fn sqlite_pool() -> Arc<AnyPool> {
	connect("sqlite::memory:").await
}

#[fixture]
async fn postgres_pool() -> (ContainerAsync<Postgres>, Arc<AnyPool>) {
	let container = Postgres::default()
		.with_tag("16-alpine")
		.start()
		.await
		.unwrap();
	let port = container.get_host_port_ipv4(5432).await.unwrap();
	let host = container.get_host().await.unwrap();
	let pool = connect(&format!(
		"postgres://postgres:postgres@{host}:{port}/postgres"
	))
	.await;
	(container, pool)
}

#[fixture]
async fn mysql_pool() -> (ContainerAsync<Mysql>, Arc<AnyPool>) {
	// First-time initialization can exceed the default minute on loaded Docker hosts.
	let container = Mysql::default()
		.with_tag("8.0")
		.with_startup_timeout(Duration::from_secs(180))
		.start()
		.await
		.unwrap();
	let port = container.get_host_port_ipv4(3306).await.unwrap();
	let host = container.get_host().await.unwrap();
	let pool = connect(&format!("mysql://root@{host}:{port}/test")).await;
	(container, pool)
}

async fn exercise_lookups(pool: Arc<AnyPool>, backend: DbBackend) {
	// Arrange: native schemas include both identifier quotes and literal pattern data.
	let (create, insert) = match backend {
		DbBackend::Mysql => (
			"CREATE TABLE `case\"lookup``` (`id` BIGINT PRIMARY KEY, `na\"me``` VARCHAR(255) COLLATE utf8mb4_0900_as_cs)",
			"INSERT INTO `case\"lookup``` (`id`, `na\"me```) VALUES (?, ?)",
		),
		DbBackend::Postgres => (
			r#"CREATE TABLE "case""lookup`" ("id" BIGINT PRIMARY KEY, "na""me`" TEXT)"#,
			r#"INSERT INTO "case""lookup`" ("id", "na""me`") VALUES ($1, $2)"#,
		),
		DbBackend::Sqlite => (
			r#"CREATE TABLE "case""lookup`" ("id" BIGINT PRIMARY KEY, "na""me`" TEXT)"#,
			r#"INSERT INTO "case""lookup`" ("id", "na""me`") VALUES (?, ?)"#,
		),
	};
	sqlx::query(create).execute(pool.as_ref()).await.unwrap();
	for (index, name) in [
		r"stored' ? $55:%_\",
		"prefix stored suffix",
		"%",
		r"stored' ? $55:XX\",
		"42",
		"true",
		"3.5",
	]
	.into_iter()
	.enumerate()
	{
		sqlx::query(insert)
			.bind(i64::try_from(index + 1).unwrap())
			.bind(name)
			.execute(pool.as_ref())
			.await
			.unwrap();
	}
	let session = Session::new(pool.clone(), backend).await.unwrap();
	let cases = [
		(FilterOperator::IContains, "STORED", vec![1, 2, 4]),
		(FilterOperator::IStartsWith, "STORED", vec![1, 4]),
		(FilterOperator::IEndsWith, r"%_\", vec![1]),
		(FilterOperator::IExact, r"STORED' ? $55:%_\", vec![1]),
		(FilterOperator::IContains, r"%_\", vec![1]),
		(FilterOperator::IStartsWith, r"STORED' ? $55:%_\", vec![1]),
		(FilterOperator::IExact, "%", vec![3]),
	];
	for (operator, pattern, expected) in cases {
		let query = QuerySet::<Record>::new().filter(Filter::new(
			"na\"me`",
			operator,
			FilterValue::String(pattern.into()),
		));
		// Act
		let records = session.list(&query).await.unwrap();
		let mut ids: Vec<_> = records.iter().map(|record| record.id).collect();
		ids.sort_unstable();
		// Assert: wildcard input cannot broaden the matching rows.
		assert_eq!(ids, expected, "{backend:?}, pattern {pattern:?}");
	}
	for operator in [
		FilterOperator::IContains,
		FilterOperator::IStartsWith,
		FilterOperator::IEndsWith,
	] {
		for (value, expected) in [
			(FilterValue::Integer(42), 5),
			(FilterValue::Int(42), 5),
			(FilterValue::Boolean(true), 6),
			(FilterValue::Bool(true), 6),
			(FilterValue::Float(3.5), 7),
		] {
			let query =
				QuerySet::<Record>::new().filter(Filter::new("na\"me`", operator.clone(), value));
			let records = session.list(&query).await.unwrap();
			assert_eq!(
				records.iter().map(|record| record.id).collect::<Vec<_>>(),
				vec![expected]
			);
		}
		let query =
			QuerySet::<Record>::new().filter(Filter::new("na\"me`", operator, FilterValue::Null));
		let records = session.list(&query).await.unwrap();
		let mut ids: Vec<_> = records.iter().map(|record| record.id).collect();
		ids.sort_unstable();
		assert_eq!(ids, vec![1, 2, 3, 4, 5, 6, 7]);
	}
	// Reset the schema between MySQL modes; fixture guards own cleanup on failure.
	sqlx::query(match backend {
		DbBackend::Mysql => "DROP TABLE `case\"lookup```",
		_ => r#"DROP TABLE "case""lookup`""#,
	})
	.execute(pool.as_ref())
	.await
	.unwrap();
}

#[rstest]
#[tokio::test]
async fn sqlite_case_insensitive_filters(#[future] sqlite_pool: Arc<AnyPool>) {
	exercise_lookups(sqlite_pool.await, DbBackend::Sqlite).await;
}

#[rstest]
#[tokio::test]
async fn postgres_case_insensitive_filters(
	#[future] postgres_pool: (ContainerAsync<Postgres>, Arc<AnyPool>),
) {
	let (_container, pool) = postgres_pool.await;
	exercise_lookups(pool, DbBackend::Postgres).await;
}

#[rstest]
#[tokio::test]
async fn mysql_case_insensitive_filters(
	#[future] mysql_pool: (ContainerAsync<Mysql>, Arc<AnyPool>),
) {
	let (_container, pool) = mysql_pool.await;
	for mode in ["", "NO_BACKSLASH_ESCAPES"] {
		// The one-connection pool keeps this session mode for every lookup.
		sqlx::query("SET SESSION sql_mode = ?")
			.bind(mode)
			.execute(pool.as_ref())
			.await
			.unwrap();
		exercise_lookups(pool.clone(), DbBackend::Mysql).await;
	}
}
