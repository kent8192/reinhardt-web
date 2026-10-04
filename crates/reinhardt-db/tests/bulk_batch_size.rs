//! SQLite bulk-operation tests run in a separate process so their global database
//! initialization cannot affect the manager unit tests.
#![cfg(all(feature = "orm", feature = "sqlite"))]

use reinhardt_db::orm::connection::DatabaseConnection;
use reinhardt_db::orm::manager::{get_connection, reinitialize_database_with_pool_size};
use reinhardt_db::orm::{FieldSelector, Manager, Model};
use rstest::{fixture, rstest};
use serde::{Deserialize, Serialize};
use serial_test::serial;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TestUser {
	id: Option<i64>,
	name: String,
	email: String,
}

#[derive(Debug, Clone)]
struct TestUserFields;

impl FieldSelector for TestUserFields {
	fn with_alias(self, _alias: &str) -> Self {
		self
	}
}

impl Model for TestUser {
	type PrimaryKey = i64;
	type Fields = TestUserFields;
	type Objects = Manager<Self>;

	fn table_name() -> &'static str {
		"test_user"
	}

	fn primary_key(&self) -> Option<Self::PrimaryKey> {
		self.id
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}

	fn primary_key_field() -> &'static str {
		"id"
	}

	fn new_fields() -> Self::Fields {
		TestUserFields
	}
}

#[fixture]
async fn sqlite_bulk_database() -> DatabaseConnection {
	reinitialize_database_with_pool_size("sqlite::memory:", Some(1))
		.await
		.expect("in-memory SQLite must connect");
	let connection = get_connection().await.unwrap();
	connection
		.execute(
			"CREATE TABLE test_user (id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT NOT NULL)",
			vec![],
		)
		.await
		.expect("test table must be created");
	connection
}

#[fixture]
fn bulk_users() -> Vec<TestUser> {
	vec![
		TestUser {
			id: Some(1),
			name: "Alice".into(),
			email: "alice@example.com".into(),
		},
		TestUser {
			id: Some(2),
			name: "Bob".into(),
			email: "bob@example.com".into(),
		},
	]
}

#[rstest]
#[case::create(false)]
#[case::update(true)]
#[serial(sqlx_drivers)]
#[tokio::test]
async fn test_bulk_zero_batch_size_does_not_write(
	#[future] sqlite_bulk_database: DatabaseConnection,
	bulk_users: Vec<TestUser>,
	#[case] update: bool,
) {
	// Arrange
	let connection = sqlite_bulk_database.await;
	let manager = TestUser::objects();

	// Act
	let result = if update {
		manager
			.bulk_update(bulk_users, vec!["name".into()], Some(0))
			.await
			.map(|_| ())
	} else {
		manager
			.bulk_create(bulk_users, Some(0), false, false)
			.await
			.map(|_| ())
	};

	// Assert
	let error = result.expect_err("zero batch size must be rejected");
	assert_eq!(
		error.to_string(),
		"Validation error: batch_size must be greater than zero"
	);
	let row = connection
		.query_one("SELECT COUNT(*) AS count FROM test_user", vec![])
		.await
		.unwrap();
	assert_eq!(row.get::<i64>("count"), Some(0));
}

#[rstest]
#[case::one(Some(1))]
#[case::default(None)]
#[serial(sqlx_drivers)]
#[tokio::test]
async fn test_bulk_create_accepts_valid_batch_sizes(
	#[future] sqlite_bulk_database: DatabaseConnection,
	bulk_users: Vec<TestUser>,
	#[case] batch_size: Option<usize>,
) {
	// Arrange
	let _database = sqlite_bulk_database.await;
	let manager = TestUser::objects();

	// Act
	let created = manager
		.bulk_create(bulk_users.clone(), batch_size, false, false)
		.await
		.expect("valid batch size must create every model");

	// Assert
	assert_eq!(created, bulk_users);
}

#[rstest]
#[case::one(Some(1))]
#[case::default(None)]
#[serial(sqlx_drivers)]
#[tokio::test]
async fn test_bulk_update_accepts_valid_batch_sizes(
	#[future] sqlite_bulk_database: DatabaseConnection,
	bulk_users: Vec<TestUser>,
	#[case] batch_size: Option<usize>,
) {
	// Arrange
	let connection = sqlite_bulk_database.await;
	connection
		.execute(
			"INSERT INTO test_user (id, name, email) VALUES (1, 'Old Alice', 'alice@example.com'), (2, 'Old Bob', 'bob@example.com')",
			vec![],
		)
		.await
		.unwrap();
	let manager = TestUser::objects();

	// Act
	let updated = manager
		.bulk_update(bulk_users.clone(), vec!["name".into()], batch_size)
		.await
		.expect("valid batch size must update every model");

	// Assert
	assert_eq!(updated, 2);
	let rows = connection
		.query("SELECT id, name, email FROM test_user ORDER BY id", vec![])
		.await
		.unwrap();
	let stored: Vec<TestUser> = rows
		.into_iter()
		.map(|row| serde_json::from_value(row.data).unwrap())
		.collect();
	assert_eq!(stored, bulk_users);
}
