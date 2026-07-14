//! Portable database error classification across supported SQL backends.

use reinhardt_core::exception::Error;
use reinhardt_db::DatabaseErrorKind;
use reinhardt_db::orm::connection::{DatabaseBackend, DatabaseConnection};
use reinhardt_query::prelude::{
	ColumnDef, Expr, Iden, MySqlQueryBuilder, PostgresQueryBuilder, Query, QueryStatementBuilder,
	QueryStatementWriter, SqliteQueryBuilder, Value,
};
#[cfg(feature = "postgres")]
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "postgres")]
use std::sync::Arc;
#[cfg(feature = "postgres")]
use testcontainers::{ContainerAsync, GenericImage};

#[derive(Iden)]
enum ErrorKindParents {
	Table,
	Id,
}

#[derive(Iden)]
enum ErrorKindRecords {
	Table,
	Id,
	UniqueValue,
	ParentId,
	RequiredValue,
	Quantity,
}

const PORTABLE_CONSTRAINT_KINDS: [DatabaseErrorKind; 4] = [
	DatabaseErrorKind::UniqueViolation,
	DatabaseErrorKind::ForeignKeyViolation,
	DatabaseErrorKind::NotNullViolation,
	DatabaseErrorKind::CheckViolation,
];

fn sql_for_backend(statement: &impl QueryStatementWriter, backend: DatabaseBackend) -> String {
	match backend {
		DatabaseBackend::Postgres => statement.to_string(PostgresQueryBuilder),
		DatabaseBackend::MySql => statement.to_string(MySqlQueryBuilder),
		DatabaseBackend::Sqlite => statement.to_string(SqliteQueryBuilder),
	}
}

fn parent_insert_sql(backend: DatabaseBackend) -> String {
	let mut statement = Query::insert();
	statement
		.into_table(ErrorKindParents::Table)
		.columns([ErrorKindParents::Id])
		.values_panic([Value::BigInt(Some(1))]);

	sql_for_backend(&statement, backend)
}

fn record_insert_sql(
	backend: DatabaseBackend,
	id: i64,
	unique_value: &str,
	parent_id: i64,
	required_value: Option<&str>,
	quantity: i32,
) -> String {
	let mut statement = Query::insert();
	statement
		.into_table(ErrorKindRecords::Table)
		.columns([
			ErrorKindRecords::Id,
			ErrorKindRecords::UniqueValue,
			ErrorKindRecords::ParentId,
			ErrorKindRecords::RequiredValue,
			ErrorKindRecords::Quantity,
		])
		.values_panic([
			Value::BigInt(Some(id)),
			Value::String(Some(Box::new(unique_value.to_owned()))),
			Value::BigInt(Some(parent_id)),
			Value::String(required_value.map(|value| Box::new(value.to_owned()))),
			Value::Int(Some(quantity)),
		]);

	sql_for_backend(&statement, backend)
}

async fn execute_error_kind(connection: &DatabaseConnection, sql: &str) -> DatabaseErrorKind {
	let Err(error) = connection.execute(sql, vec![]).await else {
		panic!("the invalid statement must fail");
	};

	error
		.database_kind()
		.expect("the execution failure must be classified as a database error")
}

fn assert_database_kind(error: Error, expected: DatabaseErrorKind) {
	assert_eq!(error.database_kind(), Some(expected));
}

async fn create_portable_schema(connection: &DatabaseConnection) {
	let backend = connection.backend();
	let mut parent_table = Query::create_table();
	parent_table.table(ErrorKindParents::Table).col(
		ColumnDef::new(ErrorKindParents::Id)
			.big_integer()
			.primary_key(true),
	);
	let mut record_table = Query::create_table();
	record_table
		.table(ErrorKindRecords::Table)
		.col(
			ColumnDef::new(ErrorKindRecords::Id)
				.big_integer()
				.primary_key(true),
		)
		.col(
			ColumnDef::new(ErrorKindRecords::UniqueValue)
				.string_len(255)
				.not_null(true),
		)
		.col(
			ColumnDef::new(ErrorKindRecords::ParentId)
				.big_integer()
				.not_null(true),
		)
		.col(
			ColumnDef::new(ErrorKindRecords::RequiredValue)
				.string_len(255)
				.not_null(true),
		)
		.col(
			ColumnDef::new(ErrorKindRecords::Quantity)
				.integer()
				.not_null(true)
				.check(Expr::col(ErrorKindRecords::Quantity).gt(0)),
		)
		.unique([ErrorKindRecords::UniqueValue])
		.foreign_key(
			[ErrorKindRecords::ParentId],
			ErrorKindParents::Table,
			[ErrorKindParents::Id],
			None,
			None,
		);

	connection
		.execute(&sql_for_backend(&parent_table, backend), vec![])
		.await
		.expect("the parent table must be created");
	connection
		.execute(&sql_for_backend(&record_table, backend), vec![])
		.await
		.expect("the record table must be created");
	connection
		.execute(&parent_insert_sql(backend), vec![])
		.await
		.expect("the parent row must be inserted");
	connection
		.execute(
			&record_insert_sql(backend, 1, "duplicate", 1, Some("present"), 1),
			vec![],
		)
		.await
		.expect("the baseline row must be inserted");
}

async fn portable_constraint_kinds(connection: &DatabaseConnection) -> [DatabaseErrorKind; 4] {
	let backend = connection.backend();
	let unique = record_insert_sql(backend, 2, "duplicate", 1, Some("present"), 1);
	let foreign_key = record_insert_sql(backend, 3, "foreign-key", 999, Some("present"), 1);
	let not_null = record_insert_sql(backend, 4, "not-null", 1, None, 1);
	let check = record_insert_sql(backend, 5, "check", 1, Some("present"), 0);

	[
		execute_error_kind(connection, &unique).await,
		execute_error_kind(connection, &foreign_key).await,
		execute_error_kind(connection, &not_null).await,
		execute_error_kind(connection, &check).await,
	]
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

	// Act
	let kinds = portable_constraint_kinds(&connection).await;

	// Assert
	assert_eq!(kinds, PORTABLE_CONSTRAINT_KINDS);
}

#[cfg(feature = "sqlite")]
#[rstest]
#[tokio::test]
async fn sqlite_constraint_errors_have_portable_kinds() {
	// Arrange
	let connection = DatabaseConnection::connect("sqlite::memory:")
		.await
		.expect("the in-memory SQLite database must connect");
	// reinhardt-query has no builder for this backend session directive.
	connection
		.execute("PRAGMA foreign_keys = ON", vec![])
		.await
		.expect("SQLite foreign-key enforcement must be enabled");
	create_portable_schema(&connection).await;

	// Act
	let kinds = portable_constraint_kinds(&connection).await;

	// Assert
	assert_eq!(kinds, PORTABLE_CONSTRAINT_KINDS);
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
	create_portable_schema(&connection).await;

	// Act
	let kinds = portable_constraint_kinds(&connection).await;

	// Assert
	assert_eq!(kinds, PORTABLE_CONSTRAINT_KINDS);
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
