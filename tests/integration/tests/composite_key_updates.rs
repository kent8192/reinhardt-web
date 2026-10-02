//! Composite-key updates preserve neighboring rows through owned connections and transactions.

#![cfg(feature = "postgres")]

use reinhardt::db::backends::DatabaseConnection as BackendsConnection;
use reinhardt::db::orm::{DatabaseConnection, DatabaseConnectionLease, Manager, Model};
use reinhardt::model;
use reinhardt_query::prelude::{
	ColumnDef, IntoIden, Order, PostgresQueryBuilder, Query, QueryStatementBuilder,
};
use reinhardt_test::fixtures::postgres_container;
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
	pool: Arc<sqlx::PgPool>,
	_lease: DatabaseConnectionLease,
	_container: ContainerAsync<GenericImage>,
}

#[fixture]
async fn update_fixture(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) -> UpdateFixture {
	let (container, pool, _port, url) = postgres_container.await;
	let owner = BackendsConnection::connect_postgres(&url).await.unwrap();
	let lease = DatabaseConnectionLease::register(owner).unwrap();
	let schema = Query::create_table()
		.table(CompositeUpdateEntries::Table.into_iden())
		.col(
			ColumnDef::new(CompositeUpdateEntries::Tenant)
				.text()
				.not_null(true),
		)
		.col(
			ColumnDef::new(CompositeUpdateEntries::Entry)
				.text()
				.not_null(true),
		)
		.col(
			ColumnDef::new(CompositeUpdateEntries::Body)
				.text()
				.not_null(true),
		)
		.primary_key([
			CompositeUpdateEntries::Tenant,
			CompositeUpdateEntries::Entry,
		])
		.to_string(PostgresQueryBuilder);
	sqlx::query(&schema).execute(pool.as_ref()).await.unwrap();
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
		.to_string(PostgresQueryBuilder);
	sqlx::query(&seed).execute(pool.as_ref()).await.unwrap();
	UpdateFixture {
		connection: lease.handle(),
		pool,
		_lease: lease,
		_container: container,
	}
}

async fn update_entry<M: Model>(connection: &mut DatabaseConnection, model: &M, transaction: bool) {
	let manager = Manager::<M>::new();
	let updated = if transaction {
		connection
			.atomic(async |tx| manager.update_with_conn(tx, model).await)
			.await
	} else {
		manager.update_with_conn(connection, model).await
	}
	.expect("updating a composite key must select exactly one row");
	assert_eq!(
		serde_json::to_value(updated).unwrap(),
		serde_json::to_value(model).unwrap()
	);
}

#[rstest]
#[case::owned(false)]
#[case::atomic(true)]
#[tokio::test]
async fn composite_update_preserves_other_rows(
	#[future] update_fixture: UpdateFixture,
	#[case] transaction: bool,
	#[values(false, true)] aliased: bool,
) {
	// Arrange
	let mut fixture = update_fixture.await;

	// Act
	if aliased {
		let model = AliasedEntry::build()
			.tenant_key("t")
			.entry_key("a")
			.body("new-a")
			.finish();
		update_entry(&mut fixture.connection, &model, transaction).await;
	} else {
		let model = Entry::build().tenant("t").entry("a").body("new-a").finish();
		update_entry(&mut fixture.connection, &model, transaction).await;
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
		.to_string(PostgresQueryBuilder);
	let rows: Vec<(String, String, String)> = sqlx::query_as(&select)
		.fetch_all(fixture.pool.as_ref())
		.await
		.unwrap();
	assert_eq!(
		rows,
		vec![
			("t".to_owned(), "a".to_owned(), "new-a".to_owned()),
			("t".to_owned(), "b".to_owned(), "old-b".to_owned()),
			("u".to_owned(), "a".to_owned(), "other-tenant".to_owned()),
		]
	);
}
