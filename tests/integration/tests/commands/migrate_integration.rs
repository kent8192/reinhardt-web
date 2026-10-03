//! Real migrate command lifecycle against persisted migration files and PostgreSQL.

use super::fixtures::MigrateCommandFixture;
use reinhardt_commands::{BaseCommand, MigrateCommand};
use reinhardt_db::migrations::MigrationSource;
use reinhardt_test::fixtures::{TestMigrationSource, postgres_container};
use rstest::rstest;
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

/// Test: MigrateCommand name and description
///
/// Category: Happy Path
/// Verifies that the command has correct metadata.
#[rstest]
fn test_migrate_command_metadata() {
	let command = MigrateCommand;

	assert_eq!(
		command.name(),
		"migrate",
		"Command name should be 'migrate'"
	);
	assert!(
		!command.description().is_empty(),
		"Command should have a description"
	);
	assert!(
		command.description().contains("migration"),
		"Description should mention 'migration'"
	);
}

/// Test: MigrateCommand arguments and options
///
/// Category: Happy Path
/// Verifies that the command defines expected arguments and options.
#[rstest]
fn test_migrate_command_arguments_and_options() {
	let command = MigrateCommand;

	let arguments = command.arguments();
	assert!(
		arguments.len() >= 2,
		"Should have at least 2 arguments (app, migration)"
	);

	let options = command.options();
	let option_names: Vec<&str> = options.iter().map(|o| o.long.as_str()).collect();

	assert!(option_names.contains(&"fake"), "Should have --fake option");
	assert!(
		option_names.contains(&"fake-initial"),
		"Should have --fake-initial option"
	);
	assert!(
		option_names.contains(&"database"),
		"Should have --database option"
	);
}

#[rstest]
#[tokio::test]
async fn test_migrate_empty_directory_is_a_noop(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = postgres_container.await;
	let fixture = MigrateCommandFixture::new();
	let (_directory, context) = fixture.command_context(&url).await;
	let before: Vec<String> = sqlx::query_scalar("SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name")
        .fetch_all(pool.as_ref()).await.unwrap();
	// Act
	MigrateCommand.execute(&context).await.unwrap();
	MigrateCommand.execute(&context).await.unwrap();
	// Assert
	let after: Vec<String> = sqlx::query_scalar("SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name")
        .fetch_all(pool.as_ref()).await.unwrap();
	assert_eq!(after, before);
}

/// Pending migrations change schema/history exactly once, including on rerun.
#[rstest]
#[case::one_migration(1)]
#[case::multiple_migrations(3)]
#[tokio::test]
async fn test_migrate_applies_once_and_reruns_without_changes(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	#[case] count: usize,
) {
	// Arrange
	let (_container, pool, _port, url) = postgres_container.await;
	let mut fixture = MigrateCommandFixture::new();
	for index in 0..count {
		fixture.add_create_table_migration(
			"audit",
			&format!("000{index}_table"),
			&format!("audit_table_{index}"),
		);
	}
	let (_directory, context) = fixture.command_context(&url).await;
	let schema_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public' AND table_name LIKE 'audit_table_%'")
        .fetch_one(pool.as_ref()).await.unwrap();
	assert_eq!(schema_before, 0);

	// Act
	MigrateCommand
		.execute(&context)
		.await
		.expect("initial migrate must succeed");
	let history_before: Vec<(String, String, chrono::NaiveDateTime)> =
		sqlx::query_as("SELECT app, name, applied FROM reinhardt_migrations ORDER BY app, name")
			.fetch_all(pool.as_ref())
			.await
			.unwrap();
	MigrateCommand
		.execute(&context)
		.await
		.expect("rerun must succeed");

	// Assert: full history equality also detects rewriting the application timestamp.
	let history_after: Vec<(String, String, chrono::NaiveDateTime)> =
		sqlx::query_as("SELECT app, name, applied FROM reinhardt_migrations ORDER BY app, name")
			.fetch_all(pool.as_ref())
			.await
			.unwrap();
	let expected: Vec<_> = (0..count)
		.map(|index| ("audit".to_owned(), format!("000{index}_table")))
		.collect();
	assert_eq!(
		history_before
			.iter()
			.map(|(app, name, _)| (app.clone(), name.clone()))
			.collect::<Vec<_>>(),
		expected
	);
	assert_eq!(history_after, history_before);
	for index in 0..count {
		// Writing a row verifies the migrated columns and constraints, not merely a table name.
		sqlx::query(&format!(
			"INSERT INTO audit_table_{index} (name) VALUES ('persisted')"
		))
		.execute(pool.as_ref())
		.await
		.unwrap();
		let names: Vec<String> =
			sqlx::query_scalar(&format!("SELECT name FROM audit_table_{index}"))
				.fetch_all(pool.as_ref())
				.await
				.unwrap();
		assert_eq!(names, ["persisted"]);
	}
}

#[rstest]
#[tokio::test]
async fn test_migrate_invalid_database_url() {
	// Arrange
	let fixture = MigrateCommandFixture::new();
	let (_directory, context) = fixture.command_context("invalid://not-a-valid-url").await;
	// Act
	let error = MigrateCommand.execute(&context).await.unwrap_err();
	// Assert
	assert_eq!(
		error.to_string(),
		"Execution error: Unsupported database URL scheme."
	);
}

#[rstest]
#[tokio::test]
async fn test_migrate_connection_failure() {
	// Arrange
	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
	let url = format!(
		"postgres://postgres:postgres@{}/nonexistent?connect_timeout=1",
		listener.local_addr().unwrap()
	);
	let fixture = MigrateCommandFixture::new();
	let (_directory, context) = fixture.command_context(&url).await;
	let mut peers = tokio::task::JoinSet::new();
	peers.spawn(async move {
		loop {
			let (stream, _) = listener.accept().await.unwrap();
			drop(stream);
		}
	});
	// Act
	let error = tokio::time::timeout(
		std::time::Duration::from_secs(10),
		MigrateCommand.execute(&context),
	)
	.await
	.expect("connection failure should be bounded")
	.unwrap_err();
	// Assert
	assert_eq!(
		error.to_string(),
		"Execution error: Failed to connect to PostgreSQL database."
	);
}

#[rstest]
#[tokio::test]
async fn test_migrate_plan_preserves_schema(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	// Arrange
	let (_container, pool, _port, url) = postgres_container.await;
	let mut fixture = MigrateCommandFixture::new();
	fixture.add_create_table_migration("plan_test", "0001_initial", "test_table");
	let (_directory, mut context) = fixture.command_context(&url).await;
	context.set_option("plan".into(), "true".into());
	let before: Vec<String> = sqlx::query_scalar("SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name")
        .fetch_all(pool.as_ref()).await.unwrap();

	// Act
	MigrateCommand
		.execute(&context)
		.await
		.expect("plan must succeed");

	// Assert: neither migration history nor application schema may be created.
	let after: Vec<String> = sqlx::query_scalar("SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name")
        .fetch_all(pool.as_ref()).await.unwrap();
	assert_eq!(after, before);
}

#[rstest]
#[case::all_pending(false)]
#[case::partially_applied(true)]
#[tokio::test]
async fn test_migrate_fake_preserves_schema(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	#[case] partially_applied: bool,
) {
	// Arrange
	let (_container, pool, _port, url) = postgres_container.await;
	let mut fixture = MigrateCommandFixture::new();
	fixture.add_create_table_migration("audit_fake", "0001_initial", "audit_existing");
	let (_first_directory, first_context) = fixture.command_context(&url).await;
	if partially_applied {
		MigrateCommand.execute(&first_context).await.unwrap();
	}
	fixture.add_create_table_migration("audit_fake", "0002_pending", "audit_pending");
	let (_directory, mut context) = fixture.command_context(&url).await;
	context.set_option("fake".into(), "true".into());
	let tables_before: Vec<String> = sqlx::query_scalar("SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name LIKE 'audit_%' ORDER BY table_name")
        .fetch_all(pool.as_ref()).await.unwrap();
	assert_eq!(
		tables_before,
		if partially_applied {
			vec!["audit_existing"]
		} else {
			vec![]
		}
	);

	// Act
	MigrateCommand
		.execute(&context)
		.await
		.expect("fake mode must succeed");

	// Assert
	let history: Vec<(String, String, chrono::NaiveDateTime)> =
		sqlx::query_as("SELECT app, name, applied FROM reinhardt_migrations ORDER BY app, name")
			.fetch_all(pool.as_ref())
			.await
			.unwrap();
	assert_eq!(
		history
			.iter()
			.map(|(app, name, _)| (app.as_str(), name.as_str()))
			.collect::<Vec<_>>(),
		[
			("audit_fake", "0001_initial"),
			("audit_fake", "0002_pending")
		]
	);
	let tables_after: Vec<String> = sqlx::query_scalar("SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name LIKE 'audit_%' ORDER BY table_name")
        .fetch_all(pool.as_ref()).await.unwrap();
	assert_eq!(tables_after, tables_before);
	MigrateCommand
		.execute(&context)
		.await
		.expect("fake rerun must succeed");
	let after_rerun: Vec<(String, String, chrono::NaiveDateTime)> =
		sqlx::query_as("SELECT app, name, applied FROM reinhardt_migrations ORDER BY app, name")
			.fetch_all(pool.as_ref())
			.await
			.unwrap();
	assert_eq!(after_rerun, history);
}

#[rstest]
#[tokio::test]
async fn test_migrate_target_rolls_back_later_migrations(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Arrange
	let mut fixture = MigrateCommandFixture::new();
	fixture.add_create_table_migration("target_test", "0001_initial", "table_1");
	fixture.add_create_table_migration("target_test", "0002_add_table", "table_2");
	fixture.add_create_table_migration("target_test", "0003_add_more", "table_3");
	let mut migrations = fixture.migrations.all_migrations().await.unwrap();
	migrations[1]
		.dependencies
		.push(("target_test".into(), "0001_initial".into()));
	migrations[2]
		.dependencies
		.push(("target_test".into(), "0002_add_table".into()));
	fixture.migrations = TestMigrationSource::with_migrations(migrations);
	let (_directory, context) = fixture.command_context(&url).await;
	MigrateCommand
		.execute(&context)
		.await
		.expect("all migrations must apply");
	let before: Vec<String> = sqlx::query_scalar("SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name LIKE 'table_%' ORDER BY table_name")
		.fetch_all(pool.as_ref()).await.unwrap();
	assert_eq!(before, ["table_1", "table_2", "table_3"]);

	// Act
	let mut target = context.clone();
	target.add_arg("target_test".into());
	target.add_arg("0002_add_table".into());
	MigrateCommand
		.execute(&target)
		.await
		.expect("rollback must succeed");

	// Assert
	let after: Vec<String> = sqlx::query_scalar("SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name LIKE 'table_%' ORDER BY table_name")
		.fetch_all(pool.as_ref()).await.unwrap();
	assert_eq!(after, ["table_1", "table_2"]);
	let history: Vec<String> = sqlx::query_scalar(
		"SELECT name FROM reinhardt_migrations WHERE app = 'target_test' ORDER BY name",
	)
	.fetch_all(pool.as_ref())
	.await
	.unwrap();
	assert_eq!(history, ["0001_initial", "0002_add_table"]);
}
