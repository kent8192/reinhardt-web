//! Native backend builders preserve bound values and predicate grouping.
#![cfg(all(feature = "postgres", feature = "mysql", feature = "sqlite"))]

use reinhardt_db::backends::query_builder::DeleteBuilder;
use reinhardt_db::backends::{
	DatabaseConnection, DatabaseType, QueryValue, SelectBuilder, UpdateBuilder,
};
use rstest::rstest;
use testcontainers::{ImageExt, runners::AsyncRunner};

async fn exercise_builders(connection: DatabaseConnection) {
	// Arrange: all fixture SQL and the native owner belong to this isolated test.
	let backend = connection.backend();
	let (timestamp, array) = match backend.database_type() {
		DatabaseType::Postgres => ("TIMESTAMP", "TEXT[]"),
		DatabaseType::Mysql => ("DATETIME", "TEXT"),
		DatabaseType::Sqlite => ("TEXT", "TEXT"),
	};
	connection.execute(&format!("CREATE TABLE builder_values (id BIGINT PRIMARY KEY, name TEXT, touched {timestamp}, items {array})"), vec![]).await.unwrap();
	connection
		.execute(
			"INSERT INTO builder_values (id,name) VALUES (1,'one'),(2,'two'),(3,'three')",
			vec![],
		)
		.await
		.unwrap();
	let name = "quoted' ? $42";
	let items = vec!["array' ? $43".to_owned()];
	// Act / Assert: timestamp and NULL expressions cannot shift native arguments.
	let updated = UpdateBuilder::new(backend.clone(), "builder_values")
		.set("name", name)
		.set_now("touched")
		.set("items", QueryValue::StringArray(items.clone()))
		.where_eq("id", 1_i64)
		.execute()
		.await
		.unwrap();
	assert_eq!(updated.rows_affected, 1);
	let touched = connection
		.fetch_one(
			"SELECT COUNT(*) AS count FROM builder_values WHERE id = 1 AND touched IS NOT NULL",
			vec![],
		)
		.await
		.unwrap();
	assert_eq!(touched.get::<i64>("count").unwrap(), 1);
	let row = SelectBuilder::new(backend.clone())
		.from("builder_values")
		.where_eq("id", 1_i64)
		.fetch_one()
		.await
		.unwrap();
	assert_eq!(row.get::<String>("name").unwrap(), name);
	if backend.database_type() == DatabaseType::Postgres {
		assert_eq!(row.data.get("items"), Some(&QueryValue::StringArray(items)));
	} else {
		assert_eq!(
			serde_json::from_str::<Vec<String>>(&row.get::<String>("items").unwrap()).unwrap(),
			items
		);
	}
	let null_update = UpdateBuilder::new(backend.clone(), "builder_values")
		.set("name", QueryValue::Null)
		.where_eq("id", 2_i64)
		.execute()
		.await
		.unwrap();
	assert_eq!(null_update.rows_affected, 1);
	let null_row = connection
		.fetch_one(
			"SELECT COUNT(*) AS count FROM builder_values WHERE id = 2 AND name IS NULL",
			vec![],
		)
		.await
		.unwrap();
	assert_eq!(null_row.get::<i64>("count").unwrap(), 1);
	let limited = SelectBuilder::new(backend.clone())
		.columns(vec!["id", "name"])
		.from("builder_values")
		.where_eq("name", name)
		.limit(1)
		.fetch_all()
		.await
		.unwrap();
	assert_eq!(limited.len(), 1);
	assert_eq!(limited[0].get::<i64>("id").unwrap(), 1);
	assert!(
		SelectBuilder::new(backend.clone())
			.from("builder_values")
			.limit(0)
			.fetch_all()
			.await
			.unwrap()
			.is_empty()
	);
	let empty = DeleteBuilder::new(backend.clone(), "builder_values")
		.where_in("id", vec![])
		.execute()
		.await
		.unwrap();
	assert_eq!(empty.rows_affected, 0);
	let deleted = DeleteBuilder::new(backend.clone(), "builder_values")
		.where_in("id", vec![1_i64.into(), 2_i64.into()])
		.execute()
		.await
		.unwrap();
	assert_eq!(deleted.rows_affected, 2);
	let retained = SelectBuilder::new(backend.clone())
		.from("builder_values")
		.fetch_all()
		.await
		.unwrap();
	assert_eq!(retained.len(), 1);
	assert_eq!(retained[0].get::<i64>("id").unwrap(), 3);
	let separate_groups = DeleteBuilder::new(backend, "builder_values")
		.where_in("id", vec![3_i64.into(), 4_i64.into()])
		.where_in("id", vec![4_i64.into()])
		.execute()
		.await
		.unwrap();
	assert_eq!(separate_groups.rows_affected, 0);
}

#[rstest]
#[tokio::test]
async fn sqlite_native_backend_builder_contracts() {
	exercise_builders(
		DatabaseConnection::connect_sqlite("sqlite::memory:")
			.await
			.unwrap(),
	)
	.await;
}

#[rstest]
#[tokio::test]
async fn postgres_native_backend_builder_contracts() {
	let container = testcontainers_modules::postgres::Postgres::default()
		.start()
		.await
		.unwrap();
	let port = container.get_host_port_ipv4(5432).await.unwrap();
	let connection = DatabaseConnection::connect_postgres(&format!(
		"postgres://postgres:postgres@127.0.0.1:{port}/postgres"
	))
	.await
	.unwrap();
	exercise_builders(connection).await;
}

#[rstest]
#[tokio::test]
async fn mysql_native_backend_builder_contracts() {
	let container = testcontainers_modules::mysql::Mysql::default()
		.with_startup_timeout(std::time::Duration::from_secs(180))
		.start()
		.await
		.unwrap();
	let port = container.get_host_port_ipv4(3306).await.unwrap();
	let connection =
		DatabaseConnection::connect_mysql(&format!("mysql://root@127.0.0.1:{port}/test"))
			.await
			.unwrap();
	exercise_builders(connection).await;
}
