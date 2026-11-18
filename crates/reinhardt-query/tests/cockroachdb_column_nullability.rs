//! Prepared execution of CockroachDB column alterations against a real server.

use reinhardt_query::prelude::*;
use reinhardt_query::types::{ColumnDef, ColumnType};
use rstest::{fixture, rstest};
use testcontainers::{
	ContainerAsync, GenericImage, ImageExt,
	core::{IntoContainerPort, WaitFor},
	runners::AsyncRunner,
};

#[fixture]
async fn cockroachdb() -> ContainerAsync<GenericImage> {
	GenericImage::new("cockroachdb/cockroach", "v23.1.0")
		.with_exposed_port(26257.tcp())
		.with_wait_for(WaitFor::message_on_stderr("initialized new cluster"))
		.with_cmd([
			"start-single-node",
			"--insecure",
			"--store=type=mem,size=1GiB",
			"--cache=64MiB",
			"--max-sql-memory=64MiB",
		])
		.start()
		.await
		.expect("start CockroachDB")
}

#[rstest]
#[tokio::test]
async fn execute_type_nullability_and_default_changes(
	#[future] cockroachdb: ContainerAsync<GenericImage>,
) {
	// Arrange: the container owns the database and cleans it up on every exit path.
	let container = cockroachdb.await;
	let host = container.get_host().await.expect("container host");
	let port = container.get_host_port_ipv4(26257).await.expect("SQL port");
	let pool = sqlx::postgres::PgPoolOptions::new()
		.max_connections(1)
		.connect(&format!(
			"postgresql://root@{host}:{port}/defaultdb?sslmode=disable"
		))
		.await
		.expect("connect to CockroachDB");
	let builder = CockroachDBQueryBuilder::new();
	let mut create = Query::create_table();
	create.table("users").col(
		ColumnDef::new("age")
			.column_type(ColumnType::SmallInteger)
			.not_null(true),
	);
	let (sql, _) = builder.build_create_table(&create);
	sqlx::query(&sql)
		.execute(&pool)
		.await
		.expect("create table");
	let mut alter = Query::alter_table();
	alter
		.table("users")
		.modify_column(
			ColumnDef::new("age")
				.column_type(ColumnType::Integer)
				.not_null(false)
				.default(7.into()),
		)
		.add_column(
			ColumnDef::new("note")
				.column_type(ColumnType::Text)
				.default("first'; $1 second".into()),
		);

	// Act: each generated pair goes through SQLx's prepared-statement protocol.
	for (sql, values) in builder.build_alter_table_statements(&alter) {
		assert!(
			values.is_empty(),
			"schema expressions cannot use bind variables"
		);
		sqlx::query(&sql)
			.execute(&pool)
			.await
			.expect("execute ALTER statement");
	}

	// Assert: the new type accepts values outside INT2 and nullability/defaults apply.
	let mut insert = Query::insert();
	insert
		.into_table("users")
		.columns(["age"])
		.values_panic([40_000]);
	let (sql, _) = builder.build_insert(&insert);
	sqlx::query(&sql)
		.bind(40_000_i32)
		.execute(&pool)
		.await
		.expect("widened column");
	let mut insert_null = Query::insert();
	insert_null
		.into_table("users")
		.columns(["age"])
		.values_panic([Value::Int(None)]);
	let (sql, _) = builder.build_insert(&insert_null);
	sqlx::query(&sql)
		.execute(&pool)
		.await
		.expect("nullable column");
	let mut insert_default = Query::insert();
	insert_default
		.into_table("users")
		.columns(["note"])
		.values_panic(["explicit"]);
	let (sql, _) = builder.build_insert(&insert_default);
	sqlx::query(&sql)
		.bind("explicit")
		.execute(&pool)
		.await
		.expect("default age");
	let mut select = Query::select();
	select.columns(["age", "note"]).from("users");
	let (sql, _) = builder.build_select(&select);
	let mut rows: Vec<(Option<i64>, String)> = sqlx::query_as(&sql)
		.fetch_all(&pool)
		.await
		.expect("read altered data");
	rows.sort_by_key(|(age, _)| *age);
	assert_eq!(
		rows,
		vec![
			(None, "first'; $1 second".into()),
			(Some(7), "explicit".into()),
			(Some(40_000), "first'; $1 second".into())
		]
	);
}
