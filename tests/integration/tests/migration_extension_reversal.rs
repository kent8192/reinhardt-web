//! Native extension rollback must restore owned extensions and preserve shared ones.

use reinhardt_db::backends::DatabaseConnection;
use reinhardt_db::migrations::{DatabaseMigrationRecorder, Migration, MigrationError, Operation};
use reinhardt_query::prelude::*;
use reinhardt_test::fixtures::migrations::{MigrationExecutorFixture, migration_executor};
use rstest::*;

#[derive(Debug, Iden)]
enum Extensions {
	#[iden = "pg_extension"]
	Table,
	Extname,
	Extversion,
	Extnamespace,
}

#[derive(Debug, Iden)]
enum Namespaces {
	#[iden = "pg_namespace"]
	Table,
	Oid,
	Nspname,
}

fn extension_migration(if_not_exists: bool) -> Migration {
	let mut migration = Migration::new("0001_extension", "extension_reversal");
	migration.operations = vec![Operation::CreateExtension {
		name: "hstore".into(),
		if_not_exists,
		schema: Some("public".into()),
	}];
	migration
}

async fn extension_snapshot(pool: &sqlx::PgPool) -> Option<(String, String)> {
	let sql = Query::select()
		.column((Extensions::Table, Extensions::Extversion))
		.column((Namespaces::Table, Namespaces::Nspname))
		.from(Extensions::Table.into_iden())
		.join(
			JoinType::InnerJoin,
			Namespaces::Table.into_iden(),
			Expr::col((Extensions::Table, Extensions::Extnamespace))
				.equals((Namespaces::Table, Namespaces::Oid)),
		)
		.and_where(Expr::col((Extensions::Table, Extensions::Extname)).eq("hstore"))
		.to_string(PostgresQueryBuilder);
	sqlx::query_as(&sql).fetch_optional(pool).await.unwrap()
}

#[rstest]
#[tokio::test]
async fn postgres_owned_extension_round_trip(
	#[future] migration_executor: MigrationExecutorFixture,
) {
	// Arrange
	let (mut executor, _container, pool, _port, url) = migration_executor.await;
	let migration = extension_migration(false);
	let recorder =
		DatabaseMigrationRecorder::new(DatabaseConnection::connect_postgres(&url).await.unwrap());
	assert_eq!(extension_snapshot(pool.as_ref()).await, None);

	// Act
	executor
		.apply_migrations(std::slice::from_ref(&migration))
		.await
		.unwrap();
	let created = extension_snapshot(pool.as_ref()).await.unwrap();
	assert_eq!(created.1, "public");
	assert!(
		recorder
			.is_applied(&migration.app_label, &migration.name)
			.await
			.unwrap()
	);
	executor
		.rollback_migrations(std::slice::from_ref(&migration))
		.await
		.unwrap();

	// Assert
	assert_eq!(extension_snapshot(pool.as_ref()).await, None);
	assert!(
		!recorder
			.is_applied(&migration.app_label, &migration.name)
			.await
			.unwrap()
	);
	executor
		.apply_migrations(std::slice::from_ref(&migration))
		.await
		.unwrap();
	assert_eq!(extension_snapshot(pool.as_ref()).await, Some(created));
}

#[rstest]
#[case::already_installed(true)]
#[case::fresh_database(false)]
#[tokio::test]
async fn postgres_conditional_extension_rollback_preserves_schema_and_history(
	#[future] migration_executor: MigrationExecutorFixture,
	#[case] preexisting: bool,
) {
	// Arrange
	let (mut executor, _container, pool, _port, url) = migration_executor.await;
	if preexisting {
		let mut provision = extension_migration(false);
		provision.name = "0000_provision".into();
		executor.apply_migrations(&[provision]).await.unwrap();
	}
	let migration = extension_migration(true);
	executor
		.apply_migrations(std::slice::from_ref(&migration))
		.await
		.unwrap();
	let before = extension_snapshot(pool.as_ref()).await;
	assert_eq!(before.as_ref().unwrap().1, "public");
	let recorder =
		DatabaseMigrationRecorder::new(DatabaseConnection::connect_postgres(&url).await.unwrap());
	let history = recorder.get_applied_migrations().await.unwrap();

	// Act
	let error = executor
		.rollback_migrations(std::slice::from_ref(&migration))
		.await
		.unwrap_err();

	// Assert: even a fresh conditional creation has no persisted ownership proof.
	assert!(matches!(error, MigrationError::IrreversibleError(message)
		if message == "Cannot automatically reverse CREATE EXTENSION IF NOT EXISTS hstore: ownership is unknown; use if_not_exists: false for a migration-owned extension"));
	assert_eq!(extension_snapshot(pool.as_ref()).await, before);
	assert_eq!(recorder.get_applied_migrations().await.unwrap(), history);
}
