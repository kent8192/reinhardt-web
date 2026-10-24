//! PostgreSQL execution regressions for primary-key changes in AlterColumn.
#![cfg(all(feature = "migrations", feature = "postgres"))]

use reinhardt_db::backends::DatabaseConnection;
use reinhardt_db::migrations::{
	ColumnDefinition, FieldType, Migration, Operation, executor::DatabaseMigrationExecutor,
};
use rstest::{fixture, rstest};
use testcontainers::{ContainerAsync, runners::AsyncRunner};
use testcontainers_modules::postgres::Postgres;

#[fixture]
async fn database() -> (ContainerAsync<Postgres>, DatabaseConnection) {
	let container = Postgres::default().start().await.unwrap();
	let host = container.get_host().await.unwrap();
	let port = container.get_host_port_ipv4(5432).await.unwrap();
	let connection = DatabaseConnection::connect_postgres(&format!(
		"postgres://postgres:postgres@{host}:{port}/postgres"
	))
	.await
	.unwrap();
	(container, connection)
}

#[rstest]
#[case::promotion(false)]
#[case::demotion(true)]
#[tokio::test]
async fn alter_column_primary_key_round_trip(
	#[future] database: (ContainerAsync<Postgres>, DatabaseConnection),
	#[case] initially_primary: bool,
	#[values(
		"events",
		"events_with_a_long_table_name_that_leaves_no_room_for_pk_suffix"
	)]
	table: &str,
) {
	// Arrange: a populated table and definitions that exercise PK-implied NOT NULL.
	let (_container, connection) = database.await;
	let pool = connection.into_postgres().unwrap();
	let quoted_table = pg_escape::quote_identifier(table);
	let original =
		ColumnDefinition::new("id", FieldType::Integer).with_primary_key(initially_primary);
	let mut executor = DatabaseMigrationExecutor::new(connection.clone());
	let initial =
		Migration::new("0001_create", "primary_key_test").add_operation(Operation::CreateTable {
			name: table.to_owned(),
			columns: vec![original.clone()],
			constraints: vec![],
			without_rowid: None,
			interleave_in_parent: None,
			partition: None,
		});
	executor.apply_migrations(&[initial]).await.unwrap();
	sqlx::query(&format!("INSERT INTO {quoted_table} (id) VALUES (1)"))
		.execute(&pool)
		.await
		.unwrap();
	let change =
		Migration::new("0002_change", "primary_key_test").add_operation(Operation::AlterColumn {
			table: table.to_owned(),
			column: "id".to_owned(),
			old_definition: Some(original.clone()),
			new_definition: original.with_primary_key(!initially_primary),
			mysql_options: None,
		});

	// Act: migrate and inspect the actual constraints and column nullability.
	executor
		.apply_migrations(std::slice::from_ref(&change))
		.await
		.unwrap();
	let schema = "SELECT EXISTS (SELECT 1 FROM pg_constraint \
		WHERE conrelid = $1::regclass AND contype = 'p'), \
		is_nullable = 'NO' FROM information_schema.columns \
		WHERE table_name = $2 AND column_name = 'id'";
	let after: (bool, bool) = sqlx::query_as(schema)
		.bind(quoted_table.as_ref())
		.bind(table)
		.fetch_one(&pool)
		.await
		.unwrap();

	// Assert: both constraints follow the new definition.
	assert_eq!(after, (!initially_primary, !initially_primary));
	let null_insert = sqlx::query(&format!("INSERT INTO {quoted_table} (id) VALUES (NULL)"))
		.execute(&pool)
		.await;
	assert_eq!(null_insert.is_ok(), initially_primary);
	// Restore valid data so rollback can reintroduce a primary key.
	sqlx::query(&format!("DELETE FROM {quoted_table} WHERE id IS NULL"))
		.execute(&pool)
		.await
		.unwrap();

	// Act: rollback must drop a promoted PK before relaxing NOT NULL.
	executor.rollback_migrations(&[change]).await.unwrap();

	// Assert: the original schema and data are restored.
	let restored: (bool, bool) = sqlx::query_as(schema)
		.bind(quoted_table.as_ref())
		.bind(table)
		.fetch_one(&pool)
		.await
		.unwrap();
	assert_eq!(restored, (initially_primary, initially_primary));
	let ids: Vec<i32> = sqlx::query_scalar(&format!("SELECT id FROM {quoted_table}"))
		.fetch_all(&pool)
		.await
		.unwrap();
	assert_eq!(ids, vec![1]);
}

#[rstest]
#[tokio::test]
async fn autodetected_primary_key_replacement_round_trip(
	#[future] database: (ContainerAsync<Postgres>, DatabaseConnection),
) {
	use reinhardt_db::migrations::{FieldState, MigrationAutodetector, ModelState, ProjectState};

	// Arrange: alphabetical field order would try to promote a_id first.
	let (_container, connection) = database.await;
	let pool = connection.into_postgres().unwrap();
	let mut before = ModelState::new("primary_key_test", "Event");
	before.table_name = "events".to_owned();
	let mut after = before.clone();
	for name in ["a_id", "z_id"] {
		let mut field = FieldState::new(name, FieldType::Integer, false);
		field
			.params
			.insert("primary_key".to_owned(), (name == "z_id").to_string());
		before.add_field(field.clone());
		field
			.params
			.insert("primary_key".to_owned(), (name == "a_id").to_string());
		after.add_field(field);
	}
	let initial =
		Migration::new("0001_create", "primary_key_test").add_operation(Operation::CreateTable {
			name: "events".to_owned(),
			columns: before
				.fields
				.iter()
				.map(|(name, field)| ColumnDefinition::from_field_state(name, field))
				.collect(),
			constraints: vec![],
			without_rowid: None,
			interleave_in_parent: None,
			partition: None,
		});
	let mut from = ProjectState::new();
	from.add_model(before);
	let mut to = ProjectState::new();
	to.add_model(after);
	let migrations = MigrationAutodetector::new(from, to).generate_migrations();
	assert_eq!(migrations.len(), 1);
	let mut executor = DatabaseMigrationExecutor::new(connection.clone());
	executor.apply_migrations(&[initial]).await.unwrap();
	sqlx::query("INSERT INTO events (a_id, z_id) VALUES (1, 2)")
		.execute(&pool)
		.await
		.unwrap();
	let primary_columns = "SELECT a.attname::text FROM pg_constraint c \
		JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = ANY(c.conkey) \
		WHERE c.conrelid = 'events'::regclass AND c.contype = 'p'";

	// Act
	executor.apply_migrations(&migrations).await.unwrap();

	// Assert: the new column actually owns the sole primary-key constraint.
	let columns: Vec<String> = sqlx::query_scalar(primary_columns)
		.fetch_all(&pool)
		.await
		.unwrap();
	assert_eq!(columns, ["a_id"]);

	// Act
	executor.rollback_migrations(&migrations).await.unwrap();

	// Assert: reverse operation order safely restores the old key and data.
	let columns: Vec<String> = sqlx::query_scalar(primary_columns)
		.fetch_all(&pool)
		.await
		.unwrap();
	assert_eq!(columns, ["z_id"]);
	let row: (i32, i32) = sqlx::query_as("SELECT a_id, z_id FROM events")
		.fetch_one(&pool)
		.await
		.unwrap();
	assert_eq!(row, (1, 2));
}
