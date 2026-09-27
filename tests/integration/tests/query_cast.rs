//! PostgreSQL execution coverage for quoted CAST type identifiers.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use reinhardt_query::prelude::*;
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sqlx::PgPool;
use testcontainers::{ContainerAsync, GenericImage};

#[rstest]
#[tokio::test]
async fn cast_as_lowercase_text_executes(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, _url) = postgres_container.await;
	let (sql, values) = Query::select()
		.expr(Expr::value(42_i32).cast_as("text"))
		.build(PostgresQueryBuilder);
	assert_eq!(sql, r#"SELECT CAST($1 AS "text")"#);
	assert_eq!(values.0, vec![Value::from(42_i32)]);

	// Act
	let result: String = sqlx::query_scalar(&sql)
		.bind(42_i32)
		.fetch_one(pool.as_ref())
		.await
		.expect("The documented lowercase text type must execute");

	// Assert
	assert_eq!(result, "42");
}

#[rstest]
#[tokio::test]
async fn cast_as_lowercase_timestamptz_executes(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, _url) = postgres_container.await;
	let input = "2026-01-02T03:04:05Z";
	let (sql, values) = Query::select()
		.expr(Expr::value(input).cast_as("timestamptz"))
		.build(PostgresQueryBuilder);
	assert_eq!(sql, r#"SELECT CAST($1 AS "timestamptz")"#);
	assert_eq!(values.0, vec![Value::from(input)]);

	// Act
	let result: DateTime<Utc> = sqlx::query_scalar(&sql)
		.bind(input)
		.fetch_one(pool.as_ref())
		.await
		.expect("The lowercase timestamptz catalog name must execute");

	// Assert
	assert_eq!(result, input.parse::<DateTime<Utc>>().unwrap());
}

#[rstest]
#[tokio::test]
async fn cast_as_preserves_and_escapes_user_defined_type_name(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, _url) = postgres_container.await;
	let type_name = "My\"Type";
	let mut create_type = Query::create_type();
	create_type.name(type_name).as_enum(vec!["active".into()]);
	let (create_sql, _) = PostgresQueryBuilder.build_create_type(&create_type);
	sqlx::query(&create_sql)
		.execute(pool.as_ref())
		.await
		.expect("The isolated database must contain the quoted enum type");
	let (sql, values) = Query::select()
		.expr(Expr::value("active").cast_as(type_name).cast_as("text"))
		.build(PostgresQueryBuilder);
	assert_eq!(sql, r#"SELECT CAST(CAST($1 AS "My""Type") AS "text")"#);
	assert_eq!(values.0, vec![Value::from("active")]);

	// Act
	let result: String = sqlx::query_scalar(&sql)
		.bind("active")
		.fetch_one(pool.as_ref())
		.await
		.expect("Case and embedded quotes must be preserved for user-defined types");

	// Assert
	assert_eq!(result, "active");
}
