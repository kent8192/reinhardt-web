//! Portable database error classification across supported SQL backends.

use reinhardt_core::exception::Error;
use reinhardt_db::DatabaseErrorKind;
use reinhardt_db::orm::connection::DatabaseConnection;
#[cfg(feature = "postgres")]
use reinhardt_test::fixtures::postgres_container;
use rstest::rstest;
#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "postgres")]
use std::sync::Arc;
#[cfg(feature = "postgres")]
use testcontainers::{ContainerAsync, GenericImage};

async fn assert_execute_kind(
	connection: &DatabaseConnection,
	sql: &str,
	expected: DatabaseErrorKind,
) {
	let Err(error) = connection.execute(sql, vec![]).await else {
		panic!("the invalid statement must fail with {expected:?}");
	};

	assert_database_kind(error, expected);
}

fn assert_database_kind(error: Error, expected: DatabaseErrorKind) {
	assert_eq!(error.database_kind(), Some(expected));
}

async fn create_portable_schema(connection: &DatabaseConnection) {
	connection
		.execute(
			"CREATE TABLE error_kind_parents (id BIGINT PRIMARY KEY)",
			vec![],
		)
		.await
		.expect("the parent table must be created");
	connection
		.execute(
			"CREATE TABLE error_kind_records (\
			 id BIGINT PRIMARY KEY, \
			 unique_value VARCHAR(255) NOT NULL UNIQUE, \
			 parent_id BIGINT NOT NULL REFERENCES error_kind_parents(id), \
			 required_value VARCHAR(255) NOT NULL, \
			 quantity INTEGER NOT NULL CHECK (quantity > 0)\
			 )",
			vec![],
		)
		.await
		.expect("the record table must be created");
	connection
		.execute("INSERT INTO error_kind_parents (id) VALUES (1)", vec![])
		.await
		.expect("the parent row must be inserted");
	connection
		.execute(
			"INSERT INTO error_kind_records \
			 (id, unique_value, parent_id, required_value, quantity) \
			 VALUES (1, 'duplicate', 1, 'present', 1)",
			vec![],
		)
		.await
		.expect("the baseline row must be inserted");
}

async fn assert_portable_constraints(connection: &DatabaseConnection) {
	assert_execute_kind(
		connection,
		"INSERT INTO error_kind_records \
		 (id, unique_value, parent_id, required_value, quantity) \
		 VALUES (2, 'duplicate', 1, 'present', 1)",
		DatabaseErrorKind::UniqueViolation,
	)
	.await;
	assert_execute_kind(
		connection,
		"INSERT INTO error_kind_records \
		 (id, unique_value, parent_id, required_value, quantity) \
		 VALUES (3, 'foreign-key', 999, 'present', 1)",
		DatabaseErrorKind::ForeignKeyViolation,
	)
	.await;
	assert_execute_kind(
		connection,
		"INSERT INTO error_kind_records \
		 (id, unique_value, parent_id, required_value, quantity) \
		 VALUES (4, 'not-null', 1, NULL, 1)",
		DatabaseErrorKind::NotNullViolation,
	)
	.await;
	assert_execute_kind(
		connection,
		"INSERT INTO error_kind_records \
		 (id, unique_value, parent_id, required_value, quantity) \
		 VALUES (5, 'check', 1, 'present', 0)",
		DatabaseErrorKind::CheckViolation,
	)
	.await;
}

#[cfg(feature = "postgres")]
#[rstest]
#[tokio::test]
async fn postgres_constraint_errors_have_portable_kinds(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	// Arrange
	let (_container, _pool, _port, url) = postgres_container.await;
	let connection = DatabaseConnection::connect(&url)
		.await
		.expect("the PostgreSQL fixture must accept framework connections");
	create_portable_schema(&connection).await;

	// Act and assert
	assert_portable_constraints(&connection).await;
}

#[rstest]
#[tokio::test]
async fn sqlite_constraint_errors_have_portable_kinds() {
	// Arrange
	let connection = DatabaseConnection::connect("sqlite::memory:")
		.await
		.expect("the in-memory SQLite database must connect");
	connection
		.execute("PRAGMA foreign_keys = ON", vec![])
		.await
		.expect("SQLite foreign-key enforcement must be enabled");
	create_portable_schema(&connection).await;

	// Act and assert
	assert_portable_constraints(&connection).await;
}

#[cfg(feature = "mysql")]
#[rstest]
#[tokio::test]
async fn mysql_constraint_errors_have_portable_kinds() {
	use reinhardt_test::{MySqlContainer, TestDatabase};

	// Arrange
	let container = MySqlContainer::new().await;
	container
		.wait_ready()
		.await
		.expect("the MySQL container must become ready");
	let connection = DatabaseConnection::connect(&container.connection_url())
		.await
		.expect("the MySQL fixture must accept framework connections");
	connection
		.execute(
			"CREATE TABLE error_kind_parents (id BIGINT PRIMARY KEY) ENGINE=InnoDB",
			vec![],
		)
		.await
		.expect("the MySQL parent table must be created");
	connection
		.execute(
			"CREATE TABLE error_kind_records (\
			 id BIGINT PRIMARY KEY, \
			 unique_value VARCHAR(255) NOT NULL UNIQUE, \
			 parent_id BIGINT NOT NULL, \
			 required_value VARCHAR(255) NOT NULL, \
			 quantity INTEGER NOT NULL CHECK (quantity > 0), \
			 CONSTRAINT error_kind_records_parent_fk \
			 FOREIGN KEY (parent_id) REFERENCES error_kind_parents(id)\
			 ) ENGINE=InnoDB",
			vec![],
		)
		.await
		.expect("the MySQL record table must be created");
	connection
		.execute("INSERT INTO error_kind_parents (id) VALUES (1)", vec![])
		.await
		.expect("the MySQL parent row must be inserted");
	connection
		.execute(
			"INSERT INTO error_kind_records \
			 (id, unique_value, parent_id, required_value, quantity) \
			 VALUES (1, 'duplicate', 1, 'present', 1)",
			vec![],
		)
		.await
		.expect("the MySQL baseline row must be inserted");

	// Act and assert
	assert_portable_constraints(&connection).await;
}

#[cfg(feature = "postgres")]
#[rstest]
#[tokio::test]
async fn refused_postgres_connection_is_classified_as_connection() {
	// Arrange
	let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
		.expect("a local ephemeral port must be available");
	let address = listener
		.local_addr()
		.expect("the bound listener must have a local address");
	drop(listener);
	let url = format!(
		"postgres://postgres@{}:{}/postgres?connect_timeout=1",
		address.ip(),
		address.port()
	);

	// Act
	let result = DatabaseConnection::connect(&url).await;

	// Assert
	let Err(error) = result else {
		panic!("a closed local endpoint must refuse the framework connection");
	};
	assert_database_kind(error, DatabaseErrorKind::Connection);
}
