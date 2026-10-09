//! Native unsigned composite-key binding and lookup regressions.

#![cfg(native)]

use std::collections::HashMap;
use std::sync::Arc;

use reinhardt_core::exception::DatabaseErrorKind;
use reinhardt_db::orm::composite_pk::{CompositePrimaryKey, PkValue};
use reinhardt_db::orm::connection::{
	BackendsConnection, DatabaseBackend, DatabaseConnection, DatabaseConnectionLease, OrmExecutor,
};
use reinhardt_db::orm::execution::convert_values;
use reinhardt_db::orm::{DatabaseValue, FieldSelector, Manager, Model, QuerySet};
use reinhardt_query::prelude::{
	ColumnDef, Iden, IntoIden, MySqlQueryBuilder, PostgresQueryBuilder, Query,
	QueryStatementBuilder, SqliteQueryBuilder, Value,
};
use reinhardt_test::fixtures::{mysql_container, postgres_container};
use rstest::{fixture, rstest};
use serde::{Deserialize, Serialize};
use testcontainers::{ContainerAsync, GenericImage};

#[derive(Debug, Iden)]
enum CompositeItems {
	Table,
	Id,
	Kind,
}

#[derive(Clone)]
struct ItemFields;

impl FieldSelector for ItemFields {
	fn with_alias(self, _alias: &str) -> Self {
		self
	}
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct SignedItem {
	id: i64,
	kind: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct UnsignedItem {
	id: u64,
	kind: String,
}

macro_rules! item_model {
	($model:ty, $key:ty) => {
		impl Model for $model {
			type PrimaryKey = $key;
			type Fields = ItemFields;
			type Objects = Manager<Self>;

			fn table_name() -> &'static str {
				"composite_items"
			}

			fn new_fields() -> Self::Fields {
				ItemFields
			}

			fn primary_key(&self) -> Option<Self::PrimaryKey> {
				Some(self.id)
			}

			fn set_primary_key(&mut self, value: Self::PrimaryKey) {
				self.id = value;
			}

			fn composite_primary_key() -> Option<CompositePrimaryKey> {
				Some(
					CompositePrimaryKey::new(vec!["id".to_owned(), "kind".to_owned()])
						.expect("two distinct key fields"),
				)
			}
		}
	};
}

item_model!(SignedItem, i64);
item_model!(UnsignedItem, u64);

struct DatabaseFixture {
	connection: DatabaseConnection,
	_lease: DatabaseConnectionLease,
}

const UNSIGNED_KEYS: [u64; 6] = [
	0,
	42,
	i64::MAX as u64,
	i64::MAX as u64 + 1,
	u64::MAX - 1,
	u64::MAX,
];

async fn prepare_database(owner: BackendsConnection, unsigned: bool) -> DatabaseFixture {
	let lease = DatabaseConnectionLease::register(owner).expect("register database owner");
	let connection = lease.handle();
	let id_column = ColumnDef::new(CompositeItems::Id);
	let mut table = Query::create_table();
	table
		.table(CompositeItems::Table.into_iden())
		.col(if unsigned {
			id_column.custom("BIGINT UNSIGNED").not_null(true)
		} else {
			id_column.big_integer().not_null(true)
		})
		.col(
			ColumnDef::new(CompositeItems::Kind)
				.string_len(32)
				.not_null(true),
		)
		.primary_key([CompositeItems::Id, CompositeItems::Kind]);
	let sql = match connection.backend() {
		DatabaseBackend::Postgres => table.to_string(PostgresQueryBuilder),
		DatabaseBackend::MySql => table.to_string(MySqlQueryBuilder),
		DatabaseBackend::Sqlite => table.to_string(SqliteQueryBuilder),
	};
	connection
		.execute(&sql, vec![])
		.await
		.expect("create composite table");

	let keys = if unsigned {
		UNSIGNED_KEYS
			.into_iter()
			.map(Value::from)
			.collect::<Vec<_>>()
	} else {
		[i64::MIN, -1, 0, 42, i64::MAX]
			.into_iter()
			.map(Value::from)
			.collect()
	};
	for id in keys {
		let mut insert = Query::insert();
		insert
			.into_table(CompositeItems::Table.into_iden())
			.columns([CompositeItems::Id, CompositeItems::Kind])
			.values_panic([id, Value::from("matching")]);
		let (sql, values) = match connection.backend() {
			DatabaseBackend::Postgres => insert.build(PostgresQueryBuilder),
			DatabaseBackend::MySql => insert.build(MySqlQueryBuilder),
			DatabaseBackend::Sqlite => insert.build(SqliteQueryBuilder),
		};
		connection
			.execute(&sql, convert_values(values))
			.await
			.expect("seed exact keys");
	}
	DatabaseFixture {
		connection,
		_lease: lease,
	}
}

#[fixture]
async fn sqlite_database() -> DatabaseFixture {
	let owner = BackendsConnection::connect_sqlite("sqlite::memory:")
		.await
		.expect("connect to disposable SQLite database");
	prepare_database(owner, false).await
}

fn primary_key(value: PkValue) -> HashMap<String, PkValue> {
	HashMap::from([
		("id".to_owned(), value),
		("kind".to_owned(), PkValue::String("matching".to_owned())),
	])
}

async fn assert_signed_lookup<E: OrmExecutor>(executor: &mut E, value: PkValue, expected: i64) {
	// Act
	let item = QuerySet::<SignedItem>::new()
		.get_composite_with_db(executor, &primary_key(value))
		.await
		.expect("representable key should address the original row");
	// Assert
	assert_eq!(
		item,
		SignedItem {
			id: expected,
			kind: "matching".to_owned()
		}
	);
}

async fn assert_rejected<E: OrmExecutor>(executor: &mut E, value: u64, expected_message: &str) {
	// Act
	let error = QuerySet::<SignedItem>::new()
		.get_composite_with_db(executor, &primary_key(PkValue::Uint(value)))
		.await
		.expect_err("overflow must not address the wrapped or clamped signed key");
	// Assert
	assert_eq!(error.database_kind(), Some(DatabaseErrorKind::Type));
	assert_eq!(
		error
			.database_error()
			.expect("structured database error")
			.message(),
		expected_message,
	);
}

#[rstest]
#[case::first_overflow(i64::MAX as u64 + 1)]
#[case::maximum(u64::MAX)]
#[tokio::test]
async fn sqlite_unsigned_overflow_never_matches_a_signed_key(
	#[future] sqlite_database: DatabaseFixture,
	#[case] value: u64,
	#[values(false, true)] transaction: bool,
) {
	// Arrange
	let mut database = sqlite_database.await;
	let expected_message = "cannot encode BigUnsigned argument 1 for sqlite: unsigned integer exceeds signed 64-bit range";

	// Act
	if transaction {
		database
			.connection
			.atomic(async |executor| {
				assert_rejected(executor, value, expected_message).await;
				Ok::<_, reinhardt_core::exception::Error>(())
			})
			.await
			.expect("checked rejection leaves the transaction usable");
	} else {
		assert_rejected(&mut database.connection, value, expected_message).await;
	}
}

#[rstest]
#[case::zero(0)]
#[case::ordinary(42)]
#[case::signed_maximum(i64::MAX as u64)]
#[tokio::test]
async fn sqlite_unsigned_keys_within_range_keep_their_value(
	#[future] sqlite_database: DatabaseFixture,
	#[case] value: u64,
	#[values(false, true)] transaction: bool,
) {
	// Arrange
	let mut database = sqlite_database.await;
	let expected = i64::try_from(value).expect("representable test key");

	// Act
	if transaction {
		database
			.connection
			.atomic(async |executor| {
				assert_signed_lookup(executor, PkValue::Uint(value), expected).await;
				Ok::<_, reinhardt_core::exception::Error>(())
			})
			.await
			.expect("transaction lookup succeeds");
	} else {
		assert_signed_lookup(&mut database.connection, PkValue::Uint(value), expected).await;
	}
}

#[rstest]
#[tokio::test]
async fn sqlite_signed_and_native_composite_keys_keep_their_codecs(
	#[future] sqlite_database: DatabaseFixture,
) {
	// Arrange
	let mut database = sqlite_database.await;

	// Act
	assert_signed_lookup(&mut database.connection, PkValue::Int(-1), -1).await;
	assert_signed_lookup(
		&mut database.connection,
		PkValue::Database {
			value: DatabaseValue::I64(42),
		},
		42,
	)
	.await;
}

#[rstest]
#[tokio::test]
async fn postgres_unsigned_keys_are_checked_before_execution(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
	#[values(false, true)] transaction: bool,
) {
	// Arrange
	let (_container, _pool, _port, url) = postgres_container.await;
	let owner = BackendsConnection::connect_postgres(&url)
		.await
		.expect("native PostgreSQL connection");
	let mut database = prepare_database(owner, false).await;
	let expected_message = "cannot encode BigUnsigned argument 1 for postgres: unsigned integer exceeds signed 64-bit range";

	// Act
	async fn check(executor: &mut impl OrmExecutor, expected_message: &str) {
		for value in [i64::MAX as u64 + 1, u64::MAX] {
			assert_rejected(executor, value, expected_message).await;
		}
		for value in [0, 42, i64::MAX] {
			assert_signed_lookup(executor, PkValue::Uint(value as u64), value).await;
		}
	}
	if transaction {
		database
			.connection
			.atomic(async |executor| {
				check(executor, expected_message).await;
				Ok::<_, reinhardt_core::exception::Error>(())
			})
			.await
			.expect("bind rejection does not abort the PostgreSQL transaction");
	} else {
		check(&mut database.connection, expected_message).await;
	}
}

#[rstest]
#[tokio::test]
async fn mysql_unsigned_composite_keys_bind_and_decode_the_full_range(
	#[future] mysql_container: (
		ContainerAsync<GenericImage>,
		Arc<sqlx::MySqlPool>,
		u16,
		String,
	),
	#[values(false, true)] transaction: bool,
) {
	// Arrange
	let (_container, _pool, _port, url) = mysql_container.await;
	let owner = BackendsConnection::connect_mysql(&url)
		.await
		.expect("native MySQL connection");
	let mut database = prepare_database(owner, true).await;

	// Act
	async fn check(executor: &mut impl OrmExecutor) {
		for value in UNSIGNED_KEYS {
			let item = QuerySet::<UnsignedItem>::new()
				.get_composite_with_db(executor, &primary_key(PkValue::Uint(value)))
				.await
				.expect("MySQL must preserve every unsigned key bit");
			assert_eq!(
				item,
				UnsignedItem {
					id: value,
					kind: "matching".to_owned()
				}
			);
		}
	}
	if transaction {
		database
			.connection
			.atomic(async |executor| {
				check(executor).await;
				Ok::<_, reinhardt_core::exception::Error>(())
			})
			.await
			.expect("unsigned transaction lookup succeeds");
	} else {
		check(&mut database.connection).await;
	}
}
