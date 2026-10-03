//! PostgreSQL bytea literal execution and native binding regression coverage.

use reinhardt_query::{
	ColumnDef, Expr, IntoIden, PostgresQueryBuilder, Query, QueryStatementBuilder, SimpleExpr,
};
use rstest::{fixture, rstest};
use sqlx::{Connection, PgConnection};
use testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner};
use testcontainers_modules::postgres::Postgres;

// Keep this component fixture independent of Reinhardt test crates to avoid
// introducing a circular publish dependency (RELEASE_PROCESS.md KI-1/KI-2).
#[fixture]
async fn postgres_bytea() -> (ContainerAsync<Postgres>, PgConnection) {
	let container = Postgres::default()
		.with_tag("17-alpine")
		.start()
		.await
		.expect("PostgreSQL 17 container should start");
	let host = container.get_host().await.unwrap();
	let port = container.get_host_port_ipv4(5432).await.unwrap();
	let connection = PgConnection::connect(&format!(
		"postgres://postgres:postgres@{host}:{port}/postgres"
	))
	.await
	.expect("PostgreSQL connection should succeed");
	(container, connection)
}

#[rstest]
#[case::reported_bytes(Some(vec![0x00, 0x01, 0xff]))]
#[case::empty(Some(vec![]))]
#[case::sql_sensitive_bytes(Some(vec![0x00, 0x27, 0x5c, 0x80, 0xff]))]
#[case::all_byte_values(Some((0..=255).collect()))]
#[case::null(None)]
#[tokio::test]
async fn postgres_byte_literals_round_trip_like_bound_parameters(
	#[future] postgres_bytea: (ContainerAsync<Postgres>, PgConnection),
	#[case] bytes: Option<Vec<u8>>,
	#[values(false, true)] nested: bool,
) {
	// Arrange
	let (_container, mut connection) = postgres_bytea.await;
	let create = Query::create_table()
		.table("byte_literal_round_trip")
		.col(ColumnDef::new("value").blob())
		.to_string(PostgresQueryBuilder);
	sqlx::query(&create).execute(&mut connection).await.unwrap();
	let value = Expr::value(bytes.clone()).into_simple_expr();
	let expression = if nested {
		SimpleExpr::CustomWithExpr("(?)".into(), vec![value])
	} else {
		value
	};
	let select = Query::select().expr(expression).to_owned();
	let insert = Query::insert()
		.into_table("byte_literal_round_trip")
		.columns(["value"])
		.from_subquery(select.clone())
		.returning_col("value")
		.to_owned();

	for setting in ["on", "off"] {
		let configure = Query::select()
			.expr(SimpleExpr::FunctionCall(
				"set_config".into_iden(),
				vec![
					Expr::value("standard_conforming_strings").into(),
					Expr::value(setting).into(),
					Expr::value(false).into(),
				],
			))
			.to_string(PostgresQueryBuilder);
		sqlx::query(&configure)
			.execute(&mut connection)
			.await
			.unwrap();

		// Act
		let inlined =
			sqlx::query_scalar::<_, Option<Vec<u8>>>(&insert.to_string(PostgresQueryBuilder))
				.fetch_one(&mut connection)
				.await
				.expect("Inlined bytea insert should succeed");
		let (bound_sql, _) = insert.build(PostgresQueryBuilder);
		let mut bound_query = sqlx::query_scalar::<_, Option<Vec<u8>>>(&bound_sql);
		if let Some(bytes) = bytes.as_deref() {
			bound_query = bound_query.bind(bytes);
		}
		let bound = bound_query
			.fetch_one(&mut connection)
			.await
			.expect("Prepared bytea insert should succeed");
		let standalone = if bytes.is_some() {
			Some(
				sqlx::query_scalar::<_, Vec<u8>>(&select.to_string(PostgresQueryBuilder))
					.fetch_one(&mut connection)
					.await
					.expect("Standalone byte expression should have bytea type"),
			)
		} else {
			None
		};

		// Assert
		assert_eq!(inlined, bytes, "Inlined bytes with setting {setting}");
		assert_eq!(bound, bytes, "Bound bytes with setting {setting}");
		assert_eq!(standalone, bytes, "Standalone bytes with setting {setting}");
	}
}
