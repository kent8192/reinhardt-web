//! Composite-key updates preserve neighboring rows and resolve physical column aliases.

#![cfg(feature = "postgres")]

use reinhardt::db::orm::{DatabaseBackend, DatabaseConnection, Manager, Model};
use reinhardt::model;
use reinhardt_query::prelude::{
	ColumnDef, ColumnType, IntoIden, MySqlQueryBuilder, Order, PostgresQueryBuilder, Query,
	QueryStatementBuilder, SqliteQueryBuilder,
};
use reinhardt_test::fixtures::{mysql_container, postgres_container};
use rstest::{fixture, rstest};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

#[model(
	app_label = "composite_update_tests",
	table_name = "composite_update_entries"
)]
#[derive(Serialize, Deserialize)]
struct Entry {
	#[field(primary_key = true, max_length = 64)]
	tenant: String,
	#[field(primary_key = true, max_length = 64)]
	entry: String,
	#[field(max_length = 64)]
	body: String,
}

#[model(
	app_label = "composite_update_tests",
	table_name = "composite_update_entries"
)]
#[derive(Serialize, Deserialize)]
struct AliasedEntry {
	#[field(primary_key = true, max_length = 64, db_column = "tenant")]
	tenant_key: String,
	#[field(primary_key = true, max_length = 64, db_column = "entry")]
	entry_key: String,
	#[field(max_length = 64)]
	body: String,
}

#[derive(Debug, reinhardt_query::prelude::Iden)]
enum CompositeUpdateEntries {
	Table,
	Tenant,
	Entry,
	Body,
}

struct UpdateFixture {
	connection: DatabaseConnection,
	_container: ContainerAsync<GenericImage>,
}

#[fixture]
async fn update_fixture(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) -> UpdateFixture {
	let (container, _pool, _port, url) = postgres_container.await;
	let connection = DatabaseConnection::connect(&url).await.unwrap();
	seed_entries(&connection).await;
	UpdateFixture {
		connection,
		_container: container,
	}
}

#[fixture]
async fn mysql_update_fixture(
	#[future] mysql_container: (
		ContainerAsync<GenericImage>,
		Arc<sqlx::MySqlPool>,
		u16,
		String,
	),
) -> UpdateFixture {
	let (container, _pool, _port, url) = mysql_container.await;
	let connection = DatabaseConnection::connect(&url).await.unwrap();
	seed_entries(&connection).await;
	UpdateFixture {
		connection,
		_container: container,
	}
}

fn render(statement: &impl QueryStatementBuilder, backend: DatabaseBackend) -> String {
	match backend {
		DatabaseBackend::Postgres => statement.to_string(PostgresQueryBuilder),
		DatabaseBackend::MySql => statement.to_string(MySqlQueryBuilder),
		DatabaseBackend::Sqlite => statement.to_string(SqliteQueryBuilder),
	}
}

async fn seed_entries(connection: &DatabaseConnection) {
	let schema = Query::create_table()
		.table(CompositeUpdateEntries::Table.into_iden())
		.col(
			ColumnDef::new(CompositeUpdateEntries::Tenant)
				.column_type(ColumnType::String(Some(64)))
				.not_null(true),
		)
		.col(
			ColumnDef::new(CompositeUpdateEntries::Entry)
				.column_type(ColumnType::String(Some(64)))
				.not_null(true),
		)
		.col(
			ColumnDef::new(CompositeUpdateEntries::Body)
				.column_type(ColumnType::String(Some(64)))
				.not_null(true),
		)
		.primary_key([
			CompositeUpdateEntries::Tenant,
			CompositeUpdateEntries::Entry,
		])
		.to_owned();
	let schema = match connection.backend() {
		DatabaseBackend::Postgres => schema.to_string(PostgresQueryBuilder),
		DatabaseBackend::MySql => schema.to_string(MySqlQueryBuilder),
		DatabaseBackend::Sqlite => schema.to_string(SqliteQueryBuilder),
	};
	connection.execute(&schema, vec![]).await.unwrap();
	let seed = Query::insert()
		.into_table(CompositeUpdateEntries::Table.into_iden())
		.columns([
			CompositeUpdateEntries::Tenant,
			CompositeUpdateEntries::Entry,
			CompositeUpdateEntries::Body,
		])
		.values_panic(["t", "a", "old-a"])
		.values_panic(["t", "b", "old-b"])
		.values_panic(["u", "a", "other-tenant"])
		.to_owned();
	connection
		.execute(&render(&seed, connection.backend()), vec![])
		.await
		.unwrap();
}

async fn update_entry<M: Model>(connection: &DatabaseConnection, model: &M) {
	let updated = Manager::<M>::new()
		.update_with_conn(connection, model)
		.await
		.expect("updating a composite key must select exactly one row");
	assert_eq!(
		serde_json::to_value(updated).unwrap(),
		serde_json::to_value(model).unwrap()
	);
}

#[rstest]
#[tokio::test]
async fn composite_update_preserves_other_rows(
	#[future] update_fixture: UpdateFixture,
	#[values(false, true)] aliased: bool,
) {
	let fixture = update_fixture.await;
	assert_composite_update(&fixture.connection, aliased).await;
}

#[rstest]
#[tokio::test]
async fn mysql_composite_update_reloads_complete_key(
	#[future] mysql_update_fixture: UpdateFixture,
	#[values(false, true)] aliased: bool,
) {
	let fixture = mysql_update_fixture.await;
	assert_composite_update(&fixture.connection, aliased).await;
}

#[rstest]
#[tokio::test]
async fn sqlite_composite_update_preserves_other_rows(#[values(false, true)] aliased: bool) {
	let connection = DatabaseConnection::connect("sqlite::memory:")
		.await
		.unwrap();
	seed_entries(&connection).await;
	assert_composite_update(&connection, aliased).await;
}

async fn assert_composite_update(connection: &DatabaseConnection, aliased: bool) {
	// Act
	if aliased {
		let model = AliasedEntry::build()
			.tenant_key("t")
			.entry_key("a")
			.body("new-a")
			.finish();
		update_entry(connection, &model).await;
	} else {
		let model = Entry::build().tenant("t").entry("a").body("new-a").finish();
		update_entry(connection, &model).await;
	}

	// Assert: neither sharing the tenant nor sharing the entry is enough to match.
	let select = Query::select()
		.from(CompositeUpdateEntries::Table.into_iden())
		.columns([
			CompositeUpdateEntries::Tenant,
			CompositeUpdateEntries::Entry,
			CompositeUpdateEntries::Body,
		])
		.order_by(CompositeUpdateEntries::Tenant, Order::Asc)
		.order_by(CompositeUpdateEntries::Entry, Order::Asc)
		.to_owned();
	let rows: Vec<_> = connection
		.query(&render(&select, connection.backend()), vec![])
		.await
		.unwrap()
		.into_iter()
		.map(|row| row.data)
		.collect();
	assert_eq!(
		rows,
		vec![
			serde_json::json!({"tenant": "t", "entry": "a", "body": "new-a"}),
			serde_json::json!({"tenant": "t", "entry": "b", "body": "old-b"}),
			serde_json::json!({"tenant": "u", "entry": "a", "body": "other-tenant"}),
		]
	);
}
