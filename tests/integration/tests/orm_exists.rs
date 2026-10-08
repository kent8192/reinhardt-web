//! Native backend regression coverage for SELECT EXISTS execution and decoding.

#![cfg(native)]

use reinhardt::model;
use reinhardt_db::orm::connection::{DatabaseBackend, DatabaseConnection, DatabaseConnectionLease};
use reinhardt_db::orm::execution::{QueryExecution, SelectExecution};
use reinhardt_query::prelude::{Alias, Expr, ExprTrait, Query};
use reinhardt_test::fixtures::{mysql_container, postgres_container};
use rstest::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use testcontainers::{ContainerAsync, GenericImage};

#[model(app_label = "exists_regression", table_name = "exists_items")]
#[derive(Serialize, Deserialize)]
struct Item {
	#[field(primary_key = true)]
	id: i64,
}

struct ExistsDatabase {
	connection: DatabaseConnection,
	_lease: DatabaseConnectionLease,
	_container: Option<ContainerAsync<GenericImage>>,
}

#[fixture]
async fn exists_database(
	#[default(DatabaseBackend::Sqlite)] backend: DatabaseBackend,
	#[default(false)] populated: bool,
) -> ExistsDatabase {
	let (connection, container) = match backend {
		DatabaseBackend::Postgres => {
			let (container, _pool, _port, url) = postgres_container().await;
			let connection =
				reinhardt_db::backends::connection::DatabaseConnection::connect_postgres(&url)
					.await
					.expect("PostgreSQL ORM connection must succeed");
			(connection, Some(container))
		}
		DatabaseBackend::MySql => {
			let (container, _pool, _port, url) = mysql_container().await;
			let connection =
				reinhardt_db::backends::connection::DatabaseConnection::connect_mysql(&url)
					.await
					.expect("MySQL ORM connection must succeed");
			(connection, Some(container))
		}
		DatabaseBackend::Sqlite => (
			reinhardt_db::backends::connection::DatabaseConnection::connect("sqlite::memory:")
				.await
				.unwrap(),
			None,
		),
	};
	let lease = DatabaseConnectionLease::register(connection).unwrap();
	let database = ExistsDatabase {
		connection: lease.handle(),
		_lease: lease,
		_container: container,
	};
	database
		.connection
		.execute("CREATE TABLE exists_items (id BIGINT NOT NULL)", vec![])
		.await
		.expect("EXISTS test table must be created");
	if populated {
		database
			.connection
			.execute("INSERT INTO exists_items (id) VALUES (1)", vec![])
			.await
			.expect("EXISTS test row must be inserted");
	}
	database
}

#[rstest]
#[case::sqlite_empty(DatabaseBackend::Sqlite, false)]
#[case::sqlite_populated(DatabaseBackend::Sqlite, true)]
#[case::mysql_empty(DatabaseBackend::MySql, false)]
#[case::mysql_populated(DatabaseBackend::MySql, true)]
#[case::postgres_empty(DatabaseBackend::Postgres, false)]
#[case::postgres_populated(DatabaseBackend::Postgres, true)]
#[tokio::test]
async fn exists_async_decodes_native_results(
	#[case] backend: DatabaseBackend,
	#[case] populated: bool,
	#[with(backend, populated)]
	#[future]
	exists_database: ExistsDatabase,
) {
	// Arrange
	let mut database = exists_database.await;
	let row = database
		.connection
		.query_one("SELECT EXISTS(SELECT 1 FROM exists_items)", vec![])
		.await
		.expect("Native EXISTS query must succeed");
	let expected_native = match backend {
		DatabaseBackend::Postgres => json!(populated),
		DatabaseBackend::Sqlite | DatabaseBackend::MySql => json!(i64::from(populated)),
	};
	assert_eq!(
		row.data.as_object().unwrap().values().next(),
		Some(&expected_native),
		"EXISTS decoding must support the existing backend row representation"
	);

	// Act
	let mut results = Vec::new();
	for id in [None, Some(1_i64), Some(2_i64)] {
		let mut statement = Query::select();
		statement
			.column(Alias::new("id"))
			.from(Alias::new("exists_items"));
		if let Some(id) = id {
			statement.and_where(Expr::col(Alias::new("id")).eq(id));
		}
		let execution = SelectExecution::<Item>::new(statement.to_owned());
		results.push(
			execution
				.exists_async(&mut database.connection)
				.await
				.expect("ORM EXISTS must decode the native result"),
		);
	}

	// Assert
	assert_eq!(results, vec![populated, populated, false]);
}
