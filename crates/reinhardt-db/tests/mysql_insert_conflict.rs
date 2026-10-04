//! Native MySQL coverage for conflict clauses that must fail before execution.

#![cfg(all(feature = "backends", feature = "mysql"))]
#![cfg(not(all(target_family = "wasm", target_os = "unknown")))]

use std::sync::Arc;
use std::time::Duration;

use reinhardt_db::backends::query_builder::OnConflictClause;
use reinhardt_db::backends::{
	DatabaseBackend, DatabaseError, InsertBuilder, MySqlBackend, QueryValue,
};
use rstest::{fixture, rstest};
use testcontainers::{ContainerAsync, ImageExt, core::logs::LogFrame, runners::AsyncRunner};
use testcontainers_modules::mysql::Mysql;

#[fixture]
async fn mysql_backend() -> (ContainerAsync<Mysql>, Arc<MySqlBackend>) {
	let container = Mysql::default()
		.with_startup_timeout(Duration::from_secs(120))
		.with_log_consumer(|frame: &LogFrame| eprint!("{}", String::from_utf8_lossy(frame.bytes())))
		.start()
		.await
		.unwrap();
	let host = container.get_host().await.unwrap();
	let port = container.get_host_port_ipv4(3306).await.unwrap();
	let pool = sqlx::MySqlPool::connect(&format!("mysql://root@{host}:{port}/test"))
		.await
		.unwrap();
	(container, Arc::new(MySqlBackend::new(pool)))
}

#[rstest]
#[tokio::test]
async fn unsupported_conflict_preserves_existing_mysql_row(
	#[future] mysql_backend: (ContainerAsync<Mysql>, Arc<MySqlBackend>),
) {
	// Arrange: the container owns the disposable database for this test.
	let (_container, backend) = mysql_backend.await;
	backend
		.execute(
			"CREATE TABLE options (id BIGINT PRIMARY KEY, name TEXT)",
			vec![],
		)
		.await
		.unwrap();
	InsertBuilder::new(backend.clone(), "options")
		.value("id", QueryValue::Int(1))
		.value("name", QueryValue::String("original".into()))
		.execute()
		.await
		.unwrap();

	// Act / Assert: unsupported clauses must report errors and preserve the row.
	for (clause, message) in [
		(
			OnConflictClause::constraint("ignored_constraint").do_update(vec!["name"]),
			"MySQL does not support named conflict targets",
		),
		(
			OnConflictClause::columns(vec!["id"])
				.do_update(vec!["name"])
				.where_clause("1 = 0"),
			"MySQL does not support conditional ON DUPLICATE KEY UPDATE",
		),
		(
			OnConflictClause::constraint("ignored_constraint")
				.do_update(vec!["name"])
				.where_clause("1 = 0"),
			"MySQL does not support named conflict targets",
		),
	] {
		let error = InsertBuilder::new(backend.clone(), "options")
			.value("id", QueryValue::Int(1))
			.value("name", QueryValue::String("replacement".into()))
			.on_conflict(clause)
			.execute()
			.await
			.unwrap_err();
		assert_eq!(error, DatabaseError::NotSupported(message.into()));
		let row = backend
			.fetch_one("SELECT name FROM options WHERE id = 1", vec![])
			.await
			.unwrap();
		assert_eq!(row.get::<String>("name").unwrap(), "original");
	}

	// Supported update / ignore: preserve the native MySQL behavior.
	for (clause, name) in [
		(OnConflictClause::any().do_update(vec!["name"]), "updated"),
		(OnConflictClause::any().do_nothing(), "ignored"),
	] {
		InsertBuilder::new(backend.clone(), "options")
			.value("id", QueryValue::Int(1))
			.value("name", QueryValue::String(name.into()))
			.on_conflict(clause)
			.execute()
			.await
			.unwrap();
		let row = backend
			.fetch_one("SELECT name FROM options WHERE id = 1", vec![])
			.await
			.unwrap();
		assert_eq!(row.get::<String>("name").unwrap(), "updated");
	}
}
