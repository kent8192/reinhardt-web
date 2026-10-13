//! Attacker-selected projection and join field names must stay identifiers.
//!
//! These tests execute the public `QuerySet` entry points against SQLite so the
//! regression covers the statement that actually reaches a database.
#![cfg(all(feature = "backends", feature = "orm", feature = "sqlite"))]

use reinhardt_core::exception::DatabaseErrorKind;
use reinhardt_db::backends::DatabaseConnection as BackendsConnection;
use reinhardt_db::orm::connection::{DatabaseConnection, DatabaseConnectionLease};
use reinhardt_db::orm::{FieldSelector, Manager, Model, QuerySet};
use rstest::{fixture, rstest};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tempfile::TempDir;

#[derive(Clone)]
struct NoFields;

impl FieldSelector for NoFields {
	fn with_alias(self, _alias: &str) -> Self {
		self
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Account {
	id: Option<i64>,
	name: String,
	secret: String,
}

impl Model for Account {
	type PrimaryKey = i64;
	type Fields = NoFields;
	type Objects = Manager<Self>;

	fn table_name() -> &'static str {
		"accounts"
	}

	fn new_fields() -> Self::Fields {
		NoFields
	}

	fn primary_key(&self) -> Option<Self::PrimaryKey> {
		self.id
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Membership {
	id: Option<i64>,
	account_id: i64,
}

impl Model for Membership {
	type PrimaryKey = i64;
	type Fields = NoFields;
	type Objects = Manager<Self>;

	fn table_name() -> &'static str {
		"memberships"
	}

	fn new_fields() -> Self::Fields {
		NoFields
	}

	fn primary_key(&self) -> Option<Self::PrimaryKey> {
		self.id
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}
}

/// Owns the SQLite database file and the registered ORM connection.
///
/// Field order is drop order: the lease closes the connection before the
/// temporary directory removes the database file, even when a test panics.
struct SqliteDatabase {
	connection: DatabaseConnection,
	_lease: DatabaseConnectionLease,
	_directory: TempDir,
}

#[fixture]
async fn sqlite_database() -> SqliteDatabase {
	let directory = tempfile::Builder::new()
		.prefix("reinhardt-identifier-injection-")
		.tempdir_in("/tmp")
		.expect("SQLite temporary directory should be created under /tmp");
	let database_path = directory.path().join("identifiers.sqlite");
	let owner =
		BackendsConnection::connect_sqlite(&format!("sqlite:///{}", database_path.display()))
			.await
			.expect("SQLite connection should open");
	let lease = DatabaseConnectionLease::register(owner).expect("SQLite lease should register");
	let connection = lease.handle();
	for statement in [
		"CREATE TABLE accounts (id INTEGER PRIMARY KEY, name TEXT NOT NULL, secret TEXT NOT NULL)",
		"CREATE TABLE memberships (id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL)",
		"INSERT INTO accounts (id, name, secret) VALUES (1, 'alice', 'alice-secret'), (2, 'bob', 'bob-secret')",
		"INSERT INTO memberships (id, account_id) VALUES (10, 1)",
	] {
		connection
			.execute(statement, Vec::new())
			.await
			.expect("SQLite fixture statement should execute");
	}
	SqliteDatabase {
		connection,
		_lease: lease,
		_directory: directory,
	}
}

#[rstest]
#[tokio::test]
async fn projection_executes_only_named_columns(#[future] sqlite_database: SqliteDatabase) {
	// Arrange
	let database = sqlite_database.await;
	let mut connection = database.connection;
	let queryset = QuerySet::<Account>::new()
		.values(&["id", "name"])
		.order_by(&["id"]);

	// Act
	let rows = queryset
		.rows_with_db(&mut connection)
		.await
		.expect("field projection should execute");

	// Assert
	let rows = rows.into_iter().map(|row| row.data).collect::<Vec<_>>();
	assert_eq!(
		rows,
		vec![
			json!({"id": 1, "name": "alice"}),
			json!({"id": 2, "name": "bob"}),
		]
	);
}

#[rstest]
#[case::scalar_subquery("(SELECT secret FROM accounts WHERE id = 2)")]
#[case::aggregate("COUNT(*)")]
#[tokio::test]
async fn projection_rejects_sql_expressions_before_execution(
	#[future] sqlite_database: SqliteDatabase,
	#[case] projection: &str,
) {
	// Arrange
	let database = sqlite_database.await;
	let mut connection = database.connection;
	let queryset = QuerySet::<Account>::new().values(&["name", projection]);

	// Act
	let error = queryset
		.rows_with_db(&mut connection)
		.await
		.expect_err("expression-shaped projection must not execute");

	// Assert
	assert_eq!(error.database_kind(), Some(DatabaseErrorKind::Unsupported));
}

#[rstest]
#[tokio::test]
async fn identifier_join_executes_column_equality(#[future] sqlite_database: SqliteDatabase) {
	// Arrange
	let database = sqlite_database.await;
	let mut connection = database.connection;
	let queryset = QuerySet::<Account>::new()
		.inner_join::<Membership>("id", "account_id")
		.values(&["name"])
		.order_by(&["name"]);

	// Act
	let rows = queryset
		.rows_with_db(&mut connection)
		.await
		.expect("identifier join should execute");

	// Assert
	let rows = rows.into_iter().map(|row| row.data).collect::<Vec<_>>();
	assert_eq!(rows, vec![json!({"name": "alice"})]);
}

#[rstest]
#[tokio::test]
async fn identifier_join_field_cannot_widen_on_predicate(
	#[future] sqlite_database: SqliteDatabase,
) {
	// Arrange: as raw SQL this field would turn the ON clause into a tautology
	// and join every account to the membership row.
	let database = sqlite_database.await;
	let mut connection = database.connection;
	let queryset = QuerySet::<Account>::new()
		.inner_join::<Membership>("id OR 1 = 1 OR accounts.id", "account_id")
		.values(&["name"]);

	// Act
	let error = queryset
		.rows_with_db(&mut connection)
		.await
		.expect_err("a quoted unknown join column must not execute as a predicate");

	// Assert
	assert_eq!(error.database_kind(), Some(DatabaseErrorKind::Query));
}
