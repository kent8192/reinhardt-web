//! PostgreSQL defaults must survive a migration forward/backward/forward cycle.

use reinhardt_db::migrations::{
	ColumnDefinition, FieldType, FilesystemRepository, FilesystemSource, Migration,
	MigrationRepository, MigrationSource, Operation,
};
use reinhardt_query::prelude::*;
use reinhardt_test::fixtures::migrations::{MigrationExecutorFixture, migration_executor};
use rstest::*;

#[derive(Debug, Iden)]
enum Columns {
	Table,
	TableSchema,
	TableName,
	ColumnName,
	ColumnDefault,
	DataType,
	IsNullable,
}

fn migration(name: &str, operations: Vec<Operation>) -> Migration {
	let mut migration = Migration::new(name, "default_reversal");
	migration.operations = operations;
	migration
}

fn sequence(name: &str) -> Operation {
	let (sql, _) =
		PostgresQueryBuilder.build_create_sequence(Query::create_sequence().name(Alias::new(name)));
	let (reverse_sql, _) =
		PostgresQueryBuilder.build_drop_sequence(Query::drop_sequence().name(Alias::new(name)));
	Operation::RunSQL {
		sql,
		reverse_sql: Some(reverse_sql),
	}
}

async fn column_snapshot(pool: &sqlx::PgPool) -> (Option<String>, String, String) {
	let sql = Query::select()
		.columns([
			Columns::ColumnDefault,
			Columns::DataType,
			Columns::IsNullable,
		])
		.from((Alias::new("information_schema"), Columns::Table))
		.and_where(Expr::col(Columns::TableSchema).eq("public"))
		.and_where(Expr::col(Columns::TableName).eq("default_example"))
		.and_where(Expr::col(Columns::ColumnName).eq("id"))
		.to_string(PostgresQueryBuilder);
	sqlx::query_as(&sql).fetch_one(pool).await.unwrap()
}

#[rstest]
#[case::no_default(None, Some("nextval('public.new_sequence'::regclass)"))]
#[case::literal_default(Some("42"), Some("nextval('public.new_sequence'::regclass)"))]
#[case::sequence_default(Some("nextval('public.old_sequence'::regclass)"), None)]
#[tokio::test]
async fn postgres_default_reversal_round_trip(
	#[future] migration_executor: MigrationExecutorFixture,
	#[case] old_default: Option<&str>,
	#[case] new_default: Option<&str>,
	#[values(false, true)] not_null: bool,
) {
	// Arrange: the original sequence is independent of the later default change.
	let (mut executor, _container, pool, _port, _url) = migration_executor.await;
	let mut old_definition = ColumnDefinition::new("id", FieldType::BigInteger);
	old_definition.default = old_default.map(str::to_owned);
	old_definition.not_null = not_null;
	let baseline = migration(
		"0001_baseline",
		vec![
			sequence("old_sequence"),
			Operation::CreateTable {
				name: "default_example".into(),
				columns: vec![old_definition.clone()],
				constraints: vec![],
				without_rowid: None,
				interleave_in_parent: None,
				partition: None,
			},
		],
	);
	let added_sequence = migration("0002_sequence", vec![sequence("new_sequence")]);
	let mut new_definition = old_definition.clone();
	new_definition.default = new_default.map(str::to_owned);
	new_definition.type_definition = FieldType::Integer;
	let alteration = migration(
		"0003_default",
		vec![Operation::AlterColumn {
			table: "default_example".into(),
			column: "id".into(),
			old_definition: Some(old_definition.clone()),
			new_definition,
			mysql_options: None,
		}],
	);
	let directory = tempfile::tempdir().unwrap();
	let mut repository = FilesystemRepository::new(directory.path());
	repository.save(&alteration).await.unwrap();
	let mut loaded = FilesystemSource::new(directory.path())
		.all_migrations()
		.await
		.unwrap();
	assert_eq!(loaded.len(), 1);
	assert_eq!(loaded[0].operations, alteration.operations);
	let alteration = loaded.remove(0);
	executor
		.apply_migrations(std::slice::from_ref(&baseline))
		.await
		.unwrap();
	let before = column_snapshot(pool.as_ref()).await;
	let nullability = if not_null { "NO" } else { "YES" };
	assert_eq!((&before.1[..], &before.2[..]), ("bigint", nullability));

	// Act: undo the altered default before dropping the sequence it used.
	executor
		.apply_migrations(&[added_sequence.clone(), alteration.clone()])
		.await
		.unwrap();
	let after = column_snapshot(pool.as_ref()).await;
	assert_eq!((&after.1[..], &after.2[..]), ("integer", nullability));
	assert_ne!(after.0, before.0);
	executor
		.rollback_migrations(&[added_sequence.clone(), alteration.clone()])
		.await
		.unwrap();

	// Assert: exact catalog state is restored and reapplication reproduces the new state.
	assert_eq!(column_snapshot(pool.as_ref()).await, before);
	executor
		.apply_migrations(&[added_sequence, alteration])
		.await
		.unwrap();
	assert_eq!(column_snapshot(pool.as_ref()).await, after);
}
