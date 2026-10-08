//! Configured PostgreSQL containers compose with migration URL helpers.

use reinhardt_db::migrations::{
	ColumnDefinition, FieldType, Migration, MigrationProvider, Operation,
};
use reinhardt_query::prelude::{Expr, PostgresQueryBuilder, Query, QueryStatementBuilder};
use reinhardt_test::PostgresContainerConfig;
use reinhardt_test::prelude::{apply_postgres_migrations_from, start_postgres_container};
use rstest::rstest;

struct ConfigMigrations;

impl MigrationProvider for ConfigMigrations {
	fn migrations() -> Vec<Migration> {
		let mut migration = Migration::new("0001_initial", "config_test");
		migration.operations.push(Operation::CreateTable {
			name: "config_probe".into(),
			columns: vec![ColumnDefinition::new("id", FieldType::Integer)],
			constraints: vec![],
			without_rowid: None,
			interleave_in_parent: None,
			partition: None,
		});
		vec![migration]
	}
}

#[rstest]
#[tokio::test]
async fn postgres_container_migration_provider_uses_configured_url() {
	// Arrange
	let (_container, pool, _port, url) = start_postgres_container(
		PostgresContainerConfig::default()
			.user("migration_user")
			.password("migration_password")
			.database("migration_db"),
	)
	.await;
	let identity_query = Query::select()
		.expr_as(Expr::cust("current_database()"), "database")
		.to_string(PostgresQueryBuilder);
	let table_query = Query::select()
		.expr(Expr::cust("COUNT(*)"))
		.from("config_probe")
		.to_string(PostgresQueryBuilder);

	// Act
	let database = apply_postgres_migrations_from::<ConfigMigrations>(&url)
		.await
		.unwrap();
	let identity = database.query_one(&identity_query, vec![]).await.unwrap();
	let rows: i64 = sqlx::query_scalar(&table_query)
		.fetch_one(pool.as_ref())
		.await
		.unwrap();
	let handle = *database;
	drop(database);
	let expired = handle.query_one(&identity_query, vec![]).await.unwrap_err();

	// Assert
	assert_eq!(
		identity.get::<String>("database"),
		Some("migration_db".into())
	);
	assert_eq!(rows, 0);
	assert_eq!(
		expired.database_error().unwrap().kind(),
		reinhardt_core::exception::DatabaseErrorKind::ConnectionHandleExpired
	);
}
