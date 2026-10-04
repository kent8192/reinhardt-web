//! Sequence declaration, source, planning, execution, and catalog round trips.
#![cfg(feature = "postgres")]
use reinhardt_db::{
	backends::DatabaseConnection,
	migrations::{
		ColumnDefinition, FieldState, FieldType, IdentityDefinition, IdentityGeneration,
		IdentityOperation, Migration, MigrationAutodetector, MigrationDirection, ModelState,
		Operation, ProjectState, QualifiedName, SequenceBound, SequenceDataType, SequenceDefault,
		SequenceDefinition, SequenceKey, SequenceOperation, SequenceOptions, SequenceOwner,
		introspection::{DatabaseIntrospector, PostgresIntrospector},
		plan_migration_sql_with_states,
		schema_diff::SchemaDiff,
	},
};
use rstest::rstest;
use sqlx::{PgPool, Row};
use testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner};
use testcontainers_modules::postgres::Postgres;

struct PostgresFixture {
	_container: ContainerAsync<Postgres>,
	pool: PgPool,
	connection: DatabaseConnection,
}
async fn postgres(tag: &str) -> PostgresFixture {
	postgres_options(tag, false).await
}
async fn postgres_options(tag: &str, catalog_stats: bool) -> PostgresFixture {
	let image = Postgres::default().with_tag(tag);
	let image = if catalog_stats {
		image.with_cmd(vec![
			"postgres",
			"-c",
			"shared_preload_libraries=pg_stat_statements",
		])
	} else {
		image
	};
	let container = image.start().await.expect("Docker PostgreSQL");
	let host = container.get_host().await.expect("container host");
	let port = container
		.get_host_port_ipv4(5432)
		.await
		.expect("PostgreSQL port");
	let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
	let pool = PgPool::connect(&url).await.expect("SQLx connection");
	let connection = DatabaseConnection::connect_postgres_with_pool_size(&url, Some(1))
		.await
		.expect("Reinhardt connection");
	PostgresFixture {
		_container: container,
		pool,
		connection,
	}
}
fn sequence(key: &str, physical: &str, table: &str) -> SequenceDefinition {
	SequenceDefinition::new(SequenceKey::new("runs", key), QualifiedName::new(physical))
		.with_options(
			SequenceOptions::new()
				.with_data_type(SequenceDataType::BigInteger)
				.with_start(1)
				.with_increment(1)
				.with_min_value(SequenceBound::Default)
				.with_max_value(SequenceBound::Default)
				.with_cache(1),
		)
		.with_owned_by(Some(SequenceOwner::new(QualifiedName::new(table), "seq")))
}
fn six_named_sequences_state() -> ProjectState {
	let mut state = ProjectState::new();
	// Physical definitions from Aidash's frozen schema at d1201622a4110a5d4fb908a15025752f9cae10d2.
	for (table, physical, column) in [
		(
			"run_activations",
			"run_activations_generation_seq",
			"generation",
		),
		("run_inputs", "run_inputs_seq_seq", "seq"),
		(
			"agent_incident_events",
			"agent_incident_events_id_seq",
			"id",
		),
		("atomic_history", "atomic_history_sequence_seq", "sequence"),
		(
			"authorization_peer_mapping_history",
			"authorization_peer_mapping_history_sequence_seq",
			"sequence",
		),
		(
			"semantic_history",
			"semantic_history_sequence_seq",
			"sequence",
		),
	] {
		let definition = sequence(table, physical, table)
			.with_owned_by(Some(SequenceOwner::new(QualifiedName::new(table), column)));
		let default = SequenceDefault::new(definition.key.clone(), definition.name.clone());
		state.add_sequence(definition).expect("valid declaration");
		let mut model = ModelState::new("runs", table);
		model.table_name = table.into();
		model.add_field(
			FieldState::new(column, FieldType::BigInteger, false).with_sequence_default(default),
		);
		state.add_model(model);
	}
	for (table, physical, mode) in [
		("events", "events_sequence_seq", IdentityGeneration::Always),
		(
			"generation_history",
			"generation_history_sequence_seq",
			IdentityGeneration::Always,
		),
		(
			"authorization_decisions",
			"authorization_decisions_sequence_seq",
			IdentityGeneration::Always,
		),
	] {
		let mut model = ModelState::new("runs", table);
		model.table_name = table.into();
		// Identity is deliberately not a primary key.
		model.add_field(
			FieldState::new("sequence", FieldType::BigInteger, false).with_identity(
				IdentityDefinition::new(mode)
					.with_sequence_name(QualifiedName::new(physical))
					.with_options(
						SequenceOptions::new()
							.with_start(1)
							.with_increment(1)
							.with_min_value(SequenceBound::Default)
							.with_max_value(SequenceBound::Default)
							.with_cache(1),
					),
			),
		);
		state.add_model(model);
	}
	state
}
async fn execute(
	fixture: &PostgresFixture,
	migration: &Migration,
	before: &ProjectState,
	after: &ProjectState,
	direction: MigrationDirection,
) {
	let plan =
		plan_migration_sql_with_states(&fixture.connection, migration, before, after, direction)
			.await
			.expect("complete SQL plan");
	for statement in plan.statements {
		if let reinhardt_db::migrations::PlannedStatement::Sql(sql) = statement {
			sqlx::query(&sql)
				.execute(&fixture.pool)
				.await
				.unwrap_or_else(|error| panic!("{sql}: {error}"));
		}
	}
}
fn replay(before: &ProjectState, migration: &Migration) -> ProjectState {
	let mut after = before.clone();
	for operation in &migration.operations {
		operation.state_forwards(&migration.app_label, &mut after);
	}
	after
}

#[rstest]
#[case("16-alpine")]
#[case("18-alpine")]
#[tokio::test]
async fn six_named_sequences_and_three_always_identities_round_trip(#[case] tag: &str) {
	// Arrange
	let fixture = postgres(tag).await;
	let desired = six_named_sequences_state();
	let operations = MigrationAutodetector::new(ProjectState::new(), desired.clone())
		.try_generate_operations()
		.expect("autodetection");
	let migration = operations.into_iter().fold(
		Migration::new("0001_initial", "runs"),
		|migration, operation| migration.add_operation(operation),
	);
	let after = replay(&ProjectState::new(), &migration);
	// Act
	execute(
		&fixture,
		&migration,
		&ProjectState::new(),
		&after,
		MigrationDirection::Forward,
	)
	.await;
	let catalog = PostgresIntrospector::new(fixture.pool.clone())
		.read_schema()
		.await
		.expect("catalog");
	let second = SchemaDiff::with_dialect(
		catalog.clone().into(),
		desired.to_database_schema(),
		reinhardt_db::migrations::SqlDialect::Postgres,
	)
	.try_generate_operations()
	.expect("catalog diff");
	// Assert
	assert_eq!(catalog.sequences.len(), 9);
	assert_eq!(
		catalog
			.sequences
			.iter()
			.filter(|sequence| sequence.identity)
			.count(),
		3
	);
	assert!(
		catalog
			.sequences
			.iter()
			.all(|sequence| sequence.owned_by.is_some())
	);
	assert!(catalog.tables["events"].primary_key.is_empty());
	assert!(second.is_empty(), "unexpected second migration: {second:?}");
	let error = sqlx::query("INSERT INTO events (sequence) VALUES (100)")
		.execute(&fixture.pool)
		.await
		.expect_err("ALWAYS rejects explicit values");
	assert_eq!(
		error
			.as_database_error()
			.and_then(|error| error.code())
			.as_deref(),
		Some("428C9")
	);
	let value: i64 = sqlx::query_scalar("INSERT INTO events DEFAULT VALUES RETURNING sequence")
		.fetch_one(&fixture.pool)
		.await
		.expect("generated identity");
	assert_eq!(value, 1);
	let input: i64 = sqlx::query_scalar("INSERT INTO run_inputs DEFAULT VALUES RETURNING seq")
		.fetch_one(&fixture.pool)
		.await
		.expect("typed sequence default");
	assert_eq!(input, 1);
}

#[rstest]
#[tokio::test]
async fn sequence_alter_rename_and_reverse_preserve_allocation() {
	// Arrange
	let fixture = postgres("16-alpine").await;
	let definition = SequenceDefinition::new(
		SequenceKey::new("runs", "counter"),
		QualifiedName::new("counter.a\"b"),
	);
	let before = ProjectState::new();
	let initial = Migration::new("0001", "runs").add_operation(Operation::Sequence {
		operation: SequenceOperation::Create {
			definition: definition.clone(),
		},
	});
	let original = replay(&before, &initial);
	execute(
		&fixture,
		&initial,
		&before,
		&original,
		MigrationDirection::Forward,
	)
	.await;
	let first: i64 = sqlx::query_scalar("SELECT nextval('\"counter.a\"\"b\"'::regclass)")
		.fetch_one(&fixture.pool)
		.await
		.expect("first allocation");
	let renamed = QualifiedName::new("renamed");
	let mut old = definition.clone();
	old.name = renamed.clone();
	let new = old
		.clone()
		.with_options(SequenceOptions::new().with_increment(2).with_cache(1));
	let change = Migration::new("0002", "runs")
		.add_operation(Operation::Sequence {
			operation: SequenceOperation::Rename {
				key: definition.key,
				old: definition.name,
				new: renamed,
			},
		})
		.add_operation(Operation::Sequence {
			operation: SequenceOperation::Alter { old, new },
		});
	let changed = replay(&original, &change);
	// Act
	execute(
		&fixture,
		&change,
		&original,
		&changed,
		MigrationDirection::Forward,
	)
	.await;
	let next: i64 = sqlx::query_scalar("SELECT nextval('renamed')")
		.fetch_one(&fixture.pool)
		.await
		.expect("next allocation");
	execute(
		&fixture,
		&change,
		&original,
		&changed,
		MigrationDirection::Backward,
	)
	.await;
	let reversed: i64 = sqlx::query_scalar("SELECT nextval('\"counter.a\"\"b\"'::regclass)")
		.fetch_one(&fixture.pool)
		.await
		.expect("reverse allocation");
	// Assert
	assert_eq!((first, next, reversed), (1, 3, 4));
}

#[rstest]
#[tokio::test]
async fn identity_mode_and_options_change_without_replacing_the_sequence() {
	// Arrange
	let fixture = postgres("16-alpine").await;
	let old = IdentityDefinition::new(IdentityGeneration::Always)
		.with_sequence_name(QualifiedName::new("identity_counter"));
	let initial = Migration::new("0001", "runs").add_operation(Operation::CreateTable {
		name: "identity_values".into(),
		columns: vec![
			ColumnDefinition::new("number", FieldType::BigInteger)
				.with_not_null(true)
				.with_identity(Some(old.clone())),
		],
		constraints: Vec::new(),
		without_rowid: None,
		interleave_in_parent: None,
		partition: None,
	});
	let original = replay(&ProjectState::new(), &initial);
	execute(
		&fixture,
		&initial,
		&ProjectState::new(),
		&original,
		MigrationDirection::Forward,
	)
	.await;
	let first: i64 =
		sqlx::query_scalar("INSERT INTO identity_values DEFAULT VALUES RETURNING number")
			.fetch_one(&fixture.pool)
			.await
			.expect("identity allocation");
	let oid: i64 = sqlx::query_scalar("SELECT 'identity_counter'::regclass::oid::bigint")
		.fetch_one(&fixture.pool)
		.await
		.expect("identity OID");
	let new = IdentityDefinition::new(IdentityGeneration::ByDefault)
		.with_sequence_name(QualifiedName::new("identity_counter"))
		.with_options(SequenceOptions::new().with_increment(3));
	let migration = Migration::new("0002", "runs").add_operation(Operation::Identity {
		operation: IdentityOperation::new(
			QualifiedName::new("identity_values"),
			"number",
			FieldType::BigInteger,
			Some(old),
			Some(new),
		),
	});
	let changed = replay(&original, &migration);
	// Act
	execute(
		&fixture,
		&migration,
		&original,
		&changed,
		MigrationDirection::Forward,
	)
	.await;
	sqlx::query("INSERT INTO identity_values (number) VALUES (100)")
		.execute(&fixture.pool)
		.await
		.expect("BY DEFAULT explicit value");
	let next: i64 =
		sqlx::query_scalar("INSERT INTO identity_values DEFAULT VALUES RETURNING number")
			.fetch_one(&fixture.pool)
			.await
			.expect("preserved position");
	let actual_oid: i64 = sqlx::query_scalar("SELECT 'identity_counter'::regclass::oid::bigint")
		.fetch_one(&fixture.pool)
		.await
		.expect("preserved OID");
	execute(
		&fixture,
		&migration,
		&original,
		&changed,
		MigrationDirection::Backward,
	)
	.await;
	let reversed: i64 =
		sqlx::query_scalar("INSERT INTO identity_values DEFAULT VALUES RETURNING number")
			.fetch_one(&fixture.pool)
			.await
			.expect("reversed options");
	// Assert
	assert_eq!((first, next, reversed), (1, 4, 5));
	assert_eq!(actual_oid, oid);
	let rows = sqlx::query("SELECT number FROM identity_values ORDER BY number")
		.fetch_all(&fixture.pool)
		.await
		.expect("rows retained");
	assert_eq!(
		rows.iter()
			.map(|row| row.get::<i64, _>("number"))
			.collect::<Vec<_>>(),
		vec![1, 4, 5, 100]
	);
}

#[rstest]
#[case(false)]
#[case(true)]
#[tokio::test]
async fn owned_sequence_implicit_deletion_and_schema_rollback(#[case] drop_table: bool) {
	// Arrange
	let fixture = postgres("16-alpine").await;
	let before = six_named_sequences_state();
	let initial = MigrationAutodetector::new(ProjectState::new(), before.clone())
		.try_generate_operations()
		.unwrap()
		.into_iter()
		.fold(
			Migration::new("0001_initial", "runs"),
			|migration, operation| migration.add_operation(operation),
		);
	let before = replay(&ProjectState::new(), &initial);
	execute(
		&fixture,
		&initial,
		&ProjectState::new(),
		&before,
		MigrationDirection::Forward,
	)
	.await;
	let mut desired = before.clone();
	if drop_table {
		desired.models.remove(&("runs".into(), "run_inputs".into()));
	} else {
		desired
			.models
			.get_mut(&("runs".into(), "run_inputs".into()))
			.unwrap()
			.fields
			.remove("seq");
	}
	desired
		.sequences
		.remove(&SequenceKey::new("runs", "run_inputs"));
	let operations = MigrationAutodetector::new(before.clone(), desired)
		.try_generate_operations()
		.unwrap();
	assert!(!operations.iter().any(|operation| matches!(
		operation,
		Operation::Sequence {
			operation: SequenceOperation::Drop { .. }
		}
	)));
	let deletion = operations.into_iter().fold(
		Migration::new("0002_delete", "runs"),
		|migration, operation| migration.add_operation(operation),
	);
	let after = replay(&before, &deletion);
	// Act
	execute(
		&fixture,
		&deletion,
		&before,
		&after,
		MigrationDirection::Forward,
	)
	.await;
	let missing: Option<String> =
		sqlx::query_scalar("SELECT to_regclass('run_inputs_seq_seq')::text")
			.fetch_one(&fixture.pool)
			.await
			.unwrap();
	execute(
		&fixture,
		&deletion,
		&before,
		&after,
		MigrationDirection::Backward,
	)
	.await;
	let catalog = PostgresIntrospector::new(fixture.pool.clone())
		.read_schema()
		.await
		.unwrap();
	// Assert
	assert!(missing.is_none());
	let restored = catalog
		.sequences
		.iter()
		.find(|sequence| sequence.name.name == "run_inputs_seq_seq")
		.unwrap();
	assert_eq!(restored.owned_by.as_ref().unwrap().column, "seq");
	assert!(
		SchemaDiff::with_dialect(
			catalog.into(),
			before.to_database_schema(),
			reinhardt_db::migrations::SqlDialect::Postgres
		)
		.try_generate_operations()
		.unwrap()
		.is_empty()
	);
}

#[rstest]
#[tokio::test]
async fn unknown_sequence_dependency_rejects_whole_plan_before_ddl() {
	// Arrange
	let fixture = postgres("16-alpine").await;
	let before = six_named_sequences_state();
	let initial = MigrationAutodetector::new(ProjectState::new(), before.clone())
		.try_generate_operations()
		.unwrap()
		.into_iter()
		.fold(
			Migration::new("0001_initial", "runs"),
			|migration, operation| migration.add_operation(operation),
		);
	let before = replay(&ProjectState::new(), &initial);
	execute(
		&fixture,
		&initial,
		&ProjectState::new(),
		&before,
		MigrationDirection::Forward,
	)
	.await;
	sqlx::query("CREATE TABLE unmanaged (n bigint DEFAULT nextval('run_inputs_seq_seq'))")
		.execute(&fixture.pool)
		.await
		.unwrap();
	let deletion = Migration::new("0002_delete", "runs").add_operation(Operation::DropTable {
		name: "run_inputs".into(),
	});
	let after = replay(&before, &deletion);
	// Act
	let result = plan_migration_sql_with_states(
		&fixture.connection,
		&deletion,
		&before,
		&after,
		MigrationDirection::Forward,
	)
	.await;
	// Assert
	assert!(
		result
			.unwrap_err()
			.to_string()
			.contains("unaccounted dependency")
	);
	let exists: Option<String> = sqlx::query_scalar("SELECT to_regclass('run_inputs')::text")
		.fetch_one(&fixture.pool)
		.await
		.unwrap();
	assert_eq!(exists.as_deref(), Some("run_inputs"));
}

#[rstest]
#[case(false)]
#[case(true)]
#[tokio::test]
async fn executor_restores_owned_sequence_from_complete_history_with_one_connection(
	#[case] replacement: bool,
) {
	// Arrange
	let fixture = postgres("16-alpine").await;
	let desired = six_named_sequences_state();
	let initial = MigrationAutodetector::new(ProjectState::new(), desired)
		.try_generate_operations()
		.unwrap()
		.into_iter()
		.fold(
			Migration::new("0001_initial", "runs"),
			|migration, operation| migration.add_operation(operation),
		);
	let before = replay(&ProjectState::new(), &initial);
	let deletion = Migration::new("0002_delete", "runs")
		.add_dependency("runs", "0001_initial")
		.add_operation(Operation::DropColumn {
			table: "run_inputs".into(),
			column: "seq".into(),
			old_definition: Some(ColumnDefinition::from_field_state(
				"seq",
				&before.models[&("runs".into(), "run_inputs".into())].fields["seq"],
			)),
		});
	let mut squash = initial.clone();
	squash.name = "0001_squashed".into();
	squash.replaces = vec![("runs".into(), "0001_initial".into())];
	let mut pending =
		Migration::new("0001_pending", "pending").add_operation(Operation::Sequence {
			operation: SequenceOperation::Create {
				definition: SequenceDefinition::new(
					SequenceKey::new("pending", "ghost"),
					QualifiedName::new("ghost"),
				)
				.with_owned_by(Some(SequenceOwner::new(
					QualifiedName::new("run_inputs"),
					"seq",
				))),
			},
		});
	// Every known file is supplied, including a pending independent app and
	// both sides of the replacement. Only recorded migrations may be replayed.
	pending.dependencies = vec![("runs".into(), "0001_initial".into())];
	let mut executor =
		reinhardt_db::migrations::DatabaseMigrationExecutor::new(fixture.connection.clone())
			.with_migration_history(vec![
				initial.clone(),
				squash.clone(),
				pending,
				deletion.clone(),
			]);
	// Act
	executor
		.apply_migrations(&[if replacement { squash } else { initial }, deletion.clone()])
		.await
		.unwrap();
	executor.rollback_migrations(&[deletion]).await.unwrap();
	// Assert
	let value: i64 = sqlx::query_scalar("INSERT INTO run_inputs DEFAULT VALUES RETURNING seq")
		.fetch_one(&fixture.pool)
		.await
		.unwrap();
	assert_eq!(value, 1);
	let recorder =
		reinhardt_db::migrations::DatabaseMigrationRecorder::new(fixture.connection.clone());
	assert!(
		recorder
			.is_applied(
				"runs",
				if replacement {
					"0001_squashed"
				} else {
					"0001_initial"
				}
			)
			.await
			.unwrap()
	);
	assert!(
		!PostgresIntrospector::new(fixture.pool.clone())
			.read_sequences()
			.await
			.unwrap()
			.iter()
			.any(|sequence| sequence.name.name == "ghost")
	);
	assert!(!recorder.is_applied("runs", "0002_delete").await.unwrap());
}

#[rstest]
#[tokio::test]
async fn irreversible_restart_keeps_recorder_applied() {
	// Arrange
	let fixture = postgres("16-alpine").await;
	let definition = SequenceDefinition::new(
		SequenceKey::new("runs", "restart"),
		QualifiedName::new("restart_counter"),
	);
	let initial = Migration::new("0001_sequence", "runs").add_operation(Operation::Sequence {
		operation: SequenceOperation::Create { definition },
	});
	let restart = Migration::new("0002_restart", "runs")
		.add_dependency("runs", "0001_sequence")
		.add_operation(Operation::Sequence {
			operation: SequenceOperation::Restart {
				name: QualifiedName::new("restart_counter"),
				value: 99,
				reverse_value: None,
			},
		});
	let mut executor =
		reinhardt_db::migrations::DatabaseMigrationExecutor::new(fixture.connection.clone())
			.with_migration_history(vec![initial.clone(), restart.clone()]);
	executor
		.apply_migrations(&[initial, restart.clone()])
		.await
		.unwrap();
	// Act
	let result = executor.rollback_migrations(&[restart]).await;
	// Assert
	assert!(
		result
			.unwrap_err()
			.to_string()
			.contains("explicit reverse target")
	);
	let recorder =
		reinhardt_db::migrations::DatabaseMigrationRecorder::new(fixture.connection.clone());
	assert!(recorder.is_applied("runs", "0002_restart").await.unwrap());
	let next: i64 = sqlx::query_scalar("SELECT nextval('restart_counter')")
		.fetch_one(&fixture.pool)
		.await
		.unwrap();
	assert_eq!(next, 99);
}

#[rstest]
#[tokio::test]
async fn literal_schema_and_descending_sequence_catalog_round_trip() {
	// Arrange
	let fixture = postgres("16-alpine").await;
	let sql = reinhardt_query::Query::create_schema()
		.name("tenant.dot")
		.to_string(reinhardt_query::PostgresQueryBuilder);
	use reinhardt_query::QueryStatementBuilder;
	sqlx::query(&sql).execute(&fixture.pool).await.unwrap();
	let name = QualifiedName::new("a.b\"c").with_schema("tenant.dot");
	let key = SequenceKey::new("runs", "descending");
	let definition = SequenceDefinition::new(key.clone(), name.clone()).with_options(
		SequenceOptions::new()
			.with_data_type(SequenceDataType::SmallInteger)
			.with_increment(-2),
	);
	let mut desired = ProjectState::new();
	desired.add_sequence(definition).unwrap();
	let mut model = ModelState::new("runs", "Numbers");
	model.table_name = "literal_numbers".into();
	model.add_field(
		FieldState::new("n", FieldType::SmallInteger, false)
			.with_sequence_default(SequenceDefault::new(key, name)),
	);
	desired.add_model(model);
	let migration = MigrationAutodetector::new(ProjectState::new(), desired.clone())
		.try_generate_operations()
		.unwrap()
		.into_iter()
		.fold(
			Migration::new("0001_literal", "runs"),
			|migration, operation| migration.add_operation(operation),
		);
	let after = replay(&ProjectState::new(), &migration);
	// Act
	execute(
		&fixture,
		&migration,
		&ProjectState::new(),
		&after,
		MigrationDirection::Forward,
	)
	.await;
	let first: i16 = sqlx::query_scalar("INSERT INTO literal_numbers DEFAULT VALUES RETURNING n")
		.fetch_one(&fixture.pool)
		.await
		.unwrap();
	let second: i16 = sqlx::query_scalar("INSERT INTO literal_numbers DEFAULT VALUES RETURNING n")
		.fetch_one(&fixture.pool)
		.await
		.unwrap();
	let catalog = PostgresIntrospector::new(fixture.pool.clone())
		.read_schema()
		.await
		.unwrap();
	// Assert
	assert_eq!((first, second), (-1, -3));
	assert!(
		SchemaDiff::with_dialect(
			catalog.into(),
			desired.to_database_schema(),
			reinhardt_db::migrations::SqlDialect::Postgres
		)
		.try_generate_operations()
		.unwrap()
		.is_empty()
	);
}

#[rstest]
#[tokio::test]
async fn adding_identity_keeps_existing_rows_and_uses_explicit_start() {
	// Arrange
	let fixture = postgres("16-alpine").await;
	let mut before = ProjectState::new();
	let mut model = ModelState::new("runs", "Populated");
	model.table_name = "populated_values".into();
	model.add_field(FieldState::new("n", FieldType::BigInteger, false));
	before.add_model(model);
	sqlx::query("CREATE TABLE populated_values (n bigint NOT NULL)")
		.execute(&fixture.pool)
		.await
		.unwrap();
	sqlx::query("INSERT INTO populated_values (n) VALUES (100)")
		.execute(&fixture.pool)
		.await
		.unwrap();
	let migration = Migration::new("0002_identity", "runs").add_operation(Operation::Identity {
		operation: IdentityOperation::new(
			QualifiedName::new("populated_values"),
			"n",
			FieldType::BigInteger,
			None,
			Some(
				IdentityDefinition::new(IdentityGeneration::Always)
					.with_options(SequenceOptions::new().with_start(5)),
			),
		),
	});
	let after = replay(&before, &migration);
	// Act
	execute(
		&fixture,
		&migration,
		&before,
		&after,
		MigrationDirection::Forward,
	)
	.await;
	let value: i64 = sqlx::query_scalar("INSERT INTO populated_values DEFAULT VALUES RETURNING n")
		.fetch_one(&fixture.pool)
		.await
		.unwrap();
	let rows: Vec<i64> = sqlx::query_scalar("SELECT n FROM populated_values ORDER BY n")
		.fetch_all(&fixture.pool)
		.await
		.unwrap();
	// Assert
	assert_eq!(value, 5);
	assert_eq!(rows, vec![5, 100]);
}

#[rstest]
#[tokio::test]
async fn invalid_restart_rejects_before_any_ddl() {
	// Arrange
	let fixture = postgres("16-alpine").await;
	sqlx::query("CREATE SEQUENCE bounded_seq MINVALUE 1 MAXVALUE 10")
		.execute(&fixture.pool)
		.await
		.unwrap();
	let migration = Migration::new("0001_invalid_restart", "runs")
		.add_operation(Operation::CreateTable {
			name: "must_not_exist".into(),
			columns: vec![ColumnDefinition::new("n", FieldType::BigInteger)],
			constraints: Vec::new(),
			without_rowid: None,
			interleave_in_parent: None,
			partition: None,
		})
		.add_operation(Operation::Sequence {
			operation: SequenceOperation::Restart {
				name: QualifiedName::new("bounded_seq"),
				value: 11,
				reverse_value: None,
			},
		});
	// Act
	let result = plan_migration_sql_with_states(
		&fixture.connection,
		&migration,
		&ProjectState::new(),
		&replay(&ProjectState::new(), &migration),
		MigrationDirection::Forward,
	)
	.await;
	let table: Option<String> = sqlx::query_scalar("SELECT to_regclass('must_not_exist')::text")
		.fetch_one(&fixture.pool)
		.await
		.unwrap();
	// Assert
	assert!(
		result
			.unwrap_err()
			.to_string()
			.contains("outside sequence bounds")
	);
	assert_eq!(table, None);
}

#[rstest]
#[case("CREATE UNLOGGED SEQUENCE unmanaged_seq", "permanent non-identity")]
#[case(
	"CREATE TABLE internal_owner (n bigint GENERATED ALWAYS AS IDENTITY (SEQUENCE NAME unmanaged_seq))",
	"permanent non-identity"
)]
#[tokio::test]
async fn independent_operations_reject_unsupported_catalog_sequences(
	#[case] setup: &str,
	#[case] message: &str,
) {
	// Arrange
	let fixture = postgres("16-alpine").await;
	sqlx::query(setup).execute(&fixture.pool).await.unwrap();
	let definition = SequenceDefinition::new(
		SequenceKey::new("runs", "managed"),
		QualifiedName::new("unmanaged_seq"),
	);
	let migration = Migration::new("0001_drop", "runs").add_operation(Operation::Sequence {
		operation: SequenceOperation::Drop { definition },
	});
	// Act
	let result = plan_migration_sql_with_states(
		&fixture.connection,
		&migration,
		&ProjectState::new(),
		&ProjectState::new(),
		MigrationDirection::Forward,
	)
	.await;
	let relation: Option<String> = sqlx::query_scalar("SELECT to_regclass('unmanaged_seq')::text")
		.fetch_one(&fixture.pool)
		.await
		.unwrap();
	// Assert
	assert!(result.unwrap_err().to_string().contains(message));
	assert!(relation.is_some());
}

#[rstest]
#[tokio::test]
async fn ownership_rejects_different_catalog_roles() {
	// Arrange
	let fixture = postgres("16-alpine").await;
	for sql in [
		"CREATE ROLE other_owner",
		"CREATE TABLE owned_values (n bigint NOT NULL)",
		"CREATE SEQUENCE owner_seq",
		"ALTER SEQUENCE owner_seq OWNER TO other_owner",
	] {
		sqlx::query(sql).execute(&fixture.pool).await.unwrap();
	}
	let mut before = ProjectState::new();
	let mut model = ModelState::new("runs", "Owned");
	model.table_name = "owned_values".into();
	model.add_field(FieldState::new("n", FieldType::BigInteger, false));
	before.add_model(model);
	let key = SequenceKey::new("runs", "owned");
	let name = QualifiedName::new("owner_seq");
	before
		.add_sequence(SequenceDefinition::new(key.clone(), name.clone()))
		.unwrap();
	let migration = Migration::new("0002_ownership", "runs").add_operation(Operation::Sequence {
		operation: SequenceOperation::Ownership {
			key,
			name,
			old: None,
			new: Some(SequenceOwner::new(QualifiedName::new("owned_values"), "n")),
		},
	});
	// Act
	let result = plan_migration_sql_with_states(
		&fixture.connection,
		&migration,
		&before,
		&replay(&before, &migration),
		MigrationDirection::Forward,
	)
	.await;
	// Assert
	assert!(
		result
			.unwrap_err()
			.to_string()
			.contains("same PostgreSQL role")
	);
}

#[rstest]
#[tokio::test]
async fn removing_default_then_adding_identity_uses_staged_column_snapshot() {
	// Arrange
	let fixture = postgres("16-alpine").await;
	let mut before = ProjectState::new();
	let mut model = ModelState::new("runs", "Defaulted");
	model.table_name = "defaulted_values".into();
	let mut defaulted = FieldState::new("n", FieldType::BigInteger, false);
	defaulted.params.insert("default".into(), "42".into());
	model.add_field(defaulted);
	before.add_model(model);
	sqlx::query("CREATE TABLE defaulted_values (n bigint NOT NULL DEFAULT 42)")
		.execute(&fixture.pool)
		.await
		.unwrap();
	let mut desired = before.clone();
	let field = desired
		.models
		.values_mut()
		.next()
		.unwrap()
		.fields
		.get_mut("n")
		.unwrap();
	field.params.remove("default");
	*field = field.clone().with_identity(
		IdentityDefinition::new(IdentityGeneration::Always)
			.with_options(SequenceOptions::new().with_start(5)),
	);
	let migration = MigrationAutodetector::new(before.clone(), desired.clone())
		.try_generate_operations()
		.unwrap()
		.into_iter()
		.fold(
			Migration::new("0002_identity", "runs"),
			|migration, operation| migration.add_operation(operation),
		);
	let after = replay(&before, &migration);
	// Act
	execute(
		&fixture,
		&migration,
		&before,
		&after,
		MigrationDirection::Forward,
	)
	.await;
	let value: i64 = sqlx::query_scalar("INSERT INTO defaulted_values DEFAULT VALUES RETURNING n")
		.fetch_one(&fixture.pool)
		.await
		.unwrap();
	// Assert
	assert_eq!(migration.operations.len(), 2);
	assert_eq!(value, 5);
	assert!(
		MigrationAutodetector::new(after, desired)
			.try_generate_operations()
			.unwrap()
			.is_empty()
	);
}

#[rstest]
#[case(false)]
#[case(true)]
#[tokio::test]
async fn renamed_owner_and_sequence_reverse_without_recreating_allocation(#[case] detach: bool) {
	// Arrange
	let fixture = postgres("16-alpine").await;
	let key = SequenceKey::new("runs", "owned");
	let name = QualifiedName::new("original_seq");
	let mut before = ProjectState::new();
	before
		.add_sequence(
			SequenceDefinition::new(key.clone(), name.clone()).with_owned_by(Some(
				SequenceOwner::new(QualifiedName::new("original_values"), "n"),
			)),
		)
		.unwrap();
	let mut model = ModelState::new("runs", "Values");
	model.table_name = "original_values".into();
	model.add_field(
		FieldState::new("n", FieldType::BigInteger, false)
			.with_sequence_default(SequenceDefault::new(key.clone(), name.clone())),
	);
	before.add_model(model);
	let initial = MigrationAutodetector::new(ProjectState::new(), before.clone())
		.try_generate_operations()
		.unwrap()
		.into_iter()
		.fold(
			Migration::new("0001_initial", "runs"),
			|migration, operation| migration.add_operation(operation),
		);
	execute(
		&fixture,
		&initial,
		&ProjectState::new(),
		&replay(&ProjectState::new(), &initial),
		MigrationDirection::Forward,
	)
	.await;
	let first: i64 = sqlx::query_scalar("INSERT INTO original_values DEFAULT VALUES RETURNING n")
		.fetch_one(&fixture.pool)
		.await
		.unwrap();
	let mut desired = before.clone();
	for operation in [
		Operation::Sequence {
			operation: SequenceOperation::Rename {
				key,
				old: name,
				new: QualifiedName::new("renamed_seq"),
			},
		},
		Operation::RenameTable {
			old_name: "original_values".into(),
			new_name: "renamed_values".into(),
		},
		Operation::RenameColumn {
			table: "renamed_values".into(),
			old_name: "n".into(),
			new_name: "value_no".into(),
		},
	] {
		operation.state_forwards("runs", &mut desired);
	}
	if detach {
		let definition = desired.sequences.values_mut().next().unwrap();
		definition.owned_by = None;
		definition.options = SequenceOptions::new().with_cache(1).with_start(10);
	}
	let migration = MigrationAutodetector::new(before.clone(), desired)
		.try_generate_operations()
		.unwrap()
		.into_iter()
		.fold(
			Migration::new("0002_renames", "runs"),
			|migration, operation| migration.add_operation(operation),
		);
	let after = replay(&before, &migration);
	// Act
	execute(
		&fixture,
		&migration,
		&before,
		&after,
		MigrationDirection::Forward,
	)
	.await;
	let second: i64 =
		sqlx::query_scalar("INSERT INTO renamed_values DEFAULT VALUES RETURNING value_no")
			.fetch_one(&fixture.pool)
			.await
			.unwrap();
	execute(
		&fixture,
		&migration,
		&before,
		&after,
		MigrationDirection::Backward,
	)
	.await;
	let third: i64 = sqlx::query_scalar("INSERT INTO original_values DEFAULT VALUES RETURNING n")
		.fetch_one(&fixture.pool)
		.await
		.unwrap();
	let sequences = PostgresIntrospector::new(fixture.pool.clone())
		.read_sequences()
		.await
		.unwrap();
	// Assert
	assert_eq!((first, second, third), (1, 2, 3));
	assert_eq!(sequences.len(), 1);
	assert_eq!(sequences[0].name.name, "original_seq");
	assert_eq!(
		sequences[0].owned_by.as_ref().unwrap().table.name,
		"original_values"
	);
	assert_eq!(sequences[0].owned_by.as_ref().unwrap().column, "n");
}

#[rstest]
#[tokio::test]
async fn incremental_executor_applies_and_reverses_identity_lifecycle() {
	// Arrange
	let fixture = postgres("16-alpine").await;
	let plain = ColumnDefinition::new("n", FieldType::Integer).with_not_null(true);
	let always = IdentityDefinition::new(IdentityGeneration::Always)
		.with_options(SequenceOptions::new().with_start(20));
	let by_default = IdentityDefinition::new(IdentityGeneration::ByDefault)
		.with_options(SequenceOptions::new().with_start(20).with_cache(3));
	let initial = Migration::new("0001_initial", "events").add_operation(Operation::CreateTable {
		name: "events".into(),
		columns: vec![plain],
		constraints: vec![],
		without_rowid: None,
		interleave_in_parent: None,
		partition: None,
	});
	let add = Migration::new("0002_add", "events")
		.add_dependency("events", "0001_initial")
		.add_operation(Operation::Identity {
			operation: IdentityOperation::new(
				QualifiedName::new("events"),
				"n",
				FieldType::Integer,
				None,
				Some(always.clone()),
			),
		});
	let alter = Migration::new("0003_alter", "events")
		.add_dependency("events", "0002_add")
		.add_operation(Operation::Identity {
			operation: IdentityOperation::new(
				QualifiedName::new("events"),
				"n",
				FieldType::Integer,
				Some(always),
				Some(by_default.clone()),
			),
		});
	let drop = Migration::new("0004_drop", "events")
		.add_dependency("events", "0003_alter")
		.add_operation(Operation::Identity {
			operation: IdentityOperation::new(
				QualifiedName::new("events"),
				"n",
				FieldType::Integer,
				Some(by_default),
				None,
			),
		});
	let mut executor =
		reinhardt_db::migrations::DatabaseMigrationExecutor::new(fixture.connection.clone());
	// Act
	for migration in [&initial, &add, &alter, &drop] {
		executor
			.apply_migrations(std::slice::from_ref(migration))
			.await
			.unwrap();
	}
	let introspector = PostgresIntrospector::new(fixture.pool.clone());
	assert!(
		introspector
			.read_table("events")
			.await
			.unwrap()
			.unwrap()
			.columns["n"]
			.identity
			.is_none()
	);
	executor
		.rollback_migrations(std::slice::from_ref(&drop))
		.await
		.unwrap();
	let restored = introspector
		.read_table("events")
		.await
		.unwrap()
		.unwrap()
		.columns["n"]
		.identity
		.clone()
		.unwrap();
	executor
		.rollback_migrations(std::slice::from_ref(&alter))
		.await
		.unwrap();
	let previous = introspector
		.read_table("events")
		.await
		.unwrap()
		.unwrap()
		.columns["n"]
		.identity
		.clone()
		.unwrap();
	executor
		.rollback_migrations(std::slice::from_ref(&add))
		.await
		.unwrap();
	// Assert
	assert_eq!(restored.generation, IdentityGeneration::ByDefault);
	assert_eq!(restored.options.cache, Some(3));
	assert_eq!(previous.generation, IdentityGeneration::Always);
	assert_eq!(previous.options.cache, Some(1));
	assert!(
		introspector
			.read_table("events")
			.await
			.unwrap()
			.unwrap()
			.columns["n"]
			.identity
			.is_none()
	);
	let recorder =
		reinhardt_db::migrations::DatabaseMigrationRecorder::new(fixture.connection.clone());
	assert_eq!(recorder.get_applied_migrations().await.unwrap().len(), 1);
}

#[rstest]
#[case(false)]
#[case(true)]
#[tokio::test]
async fn executor_identity_width_change_restores_sequence_bounds(#[case] explicit: bool) {
	// Arrange
	let fixture = postgres("16-alpine").await;
	let mut old = ProjectState::new();
	let mut model = ModelState::new("events", "Event");
	model.table_name = "events".into();
	let options = if explicit {
		SequenceOptions::new().with_data_type(SequenceDataType::Integer)
	} else {
		SequenceOptions::new()
	};
	model.add_field(
		FieldState::new("n", FieldType::Integer, false).with_identity(
			IdentityDefinition::new(IdentityGeneration::Always).with_options(options),
		),
	);
	old.add_model(model);
	let initial = MigrationAutodetector::new(ProjectState::new(), old.clone())
		.try_generate_operations()
		.unwrap()
		.into_iter()
		.fold(
			Migration::new("0001_initial", "events"),
			|migration, operation| migration.add_operation(operation),
		);
	let mut new = old.clone();
	let options = if explicit {
		SequenceOptions::new().with_data_type(SequenceDataType::BigInteger)
	} else {
		SequenceOptions::new()
	};
	new.models.values_mut().next().unwrap().add_field(
		FieldState::new("n", FieldType::BigInteger, false).with_identity(
			IdentityDefinition::new(IdentityGeneration::Always).with_options(options),
		),
	);
	let change = MigrationAutodetector::new(old, new)
		.try_generate_operations()
		.unwrap()
		.into_iter()
		.fold(
			Migration::new("0002_width", "events").add_dependency("events", "0001_initial"),
			|migration, operation| migration.add_operation(operation),
		);
	let mut executor =
		reinhardt_db::migrations::DatabaseMigrationExecutor::new(fixture.connection.clone());
	executor.apply_migrations(&[initial]).await.unwrap();
	// Act
	executor
		.apply_migrations(std::slice::from_ref(&change))
		.await
		.unwrap();
	let introspector = PostgresIntrospector::new(fixture.pool.clone());
	let widened = introspector
		.read_table("events")
		.await
		.unwrap()
		.unwrap()
		.columns["n"]
		.clone();
	executor.rollback_migrations(&[change]).await.unwrap();
	let restored = introspector
		.read_table("events")
		.await
		.unwrap()
		.unwrap()
		.columns["n"]
		.clone();
	// Assert
	assert_eq!(widened.column_type, FieldType::BigInteger);
	assert_eq!(
		widened.identity.unwrap().options.max_value,
		Some(SequenceBound::Value(i64::MAX))
	);
	assert_eq!(restored.column_type, FieldType::Integer);
	assert_eq!(
		restored.identity.unwrap().options.max_value,
		Some(SequenceBound::Value(i64::from(i32::MAX)))
	);
}

#[rstest]
#[tokio::test]
async fn catalog_identity_changes_keep_implicit_physical_names() {
	// Arrange
	let fixture = postgres("16-alpine").await;
	let mut target = ProjectState::new();
	let mut model = ModelState::new("events", "Event");
	model.table_name = "events".into();
	model.add_field(
		FieldState::new("n", FieldType::Integer, false)
			.with_identity(IdentityDefinition::new(IdentityGeneration::Always)),
	);
	target.add_model(model);
	let initial = MigrationAutodetector::new(ProjectState::new(), target.clone())
		.try_generate_operations()
		.unwrap()
		.into_iter()
		.fold(
			Migration::new("0001_initial", "events"),
			|migration, operation| migration.add_operation(operation),
		);
	let mut executor =
		reinhardt_db::migrations::DatabaseMigrationExecutor::new(fixture.connection.clone());
	executor.apply_migrations(&[initial]).await.unwrap();
	let introspector = PostgresIntrospector::new(fixture.pool.clone());
	let catalog = introspector.read_schema().await.unwrap();
	let original_name = catalog.tables["events"].columns["n"]
		.identity
		.as_ref()
		.unwrap()
		.sequence_name
		.clone();
	target.models.values_mut().next().unwrap().add_field(
		FieldState::new("n", FieldType::Integer, false).with_identity(
			IdentityDefinition::new(IdentityGeneration::ByDefault)
				.with_options(SequenceOptions::new().with_cache(4)),
		),
	);
	let change = SchemaDiff::with_dialect(
		catalog.into(),
		target.to_database_schema(),
		reinhardt_db::migrations::SqlDialect::Postgres,
	)
	.try_generate_operations()
	.unwrap()
	.into_iter()
	.fold(
		Migration::new("0002_mode", "events").add_dependency("events", "0001_initial"),
		|migration, operation| migration.add_operation(operation),
	);
	// Catalog-derived migrations use the observed definition as their pre-state.
	let mut history =
		Migration::new("0001_initial", "events").add_operation(Operation::CreateTable {
			name: "events".into(),
			columns: vec![
				ColumnDefinition::new("n", FieldType::Integer)
					.with_not_null(true)
					.with_identity(
						introspector
							.read_table("events")
							.await
							.unwrap()
							.unwrap()
							.columns["n"]
							.identity
							.clone(),
					),
			],
			constraints: vec![],
			without_rowid: None,
			interleave_in_parent: None,
			partition: None,
		});
	history.state_only = true;
	let mut executor =
		reinhardt_db::migrations::DatabaseMigrationExecutor::new(fixture.connection.clone())
			.with_migration_history(vec![history]);
	// Act
	executor.apply_migrations(&[change]).await.unwrap();
	let changed = introspector.read_schema().await.unwrap();
	let second = SchemaDiff::with_dialect(
		changed.clone().into(),
		target.to_database_schema(),
		reinhardt_db::migrations::SqlDialect::Postgres,
	)
	.try_generate_operations()
	.unwrap();
	// Assert
	assert_eq!(
		changed.tables["events"].columns["n"]
			.identity
			.as_ref()
			.unwrap()
			.sequence_name,
		original_name
	);
	assert!(second.is_empty());
}

#[rstest]
#[case(false)]
#[case(true)]
#[tokio::test]
async fn executor_reuses_vacated_relation_names_for_sequences(#[case] table: bool) {
	// Arrange
	let fixture = postgres("16-alpine").await;
	let mut old = ProjectState::new();
	old.add_sequence(SequenceDefinition::new(
		SequenceKey::new("events", "counter"),
		QualifiedName::new("counter"),
	))
	.unwrap();
	if table {
		let mut model = ModelState::new("events", "Event");
		model.table_name = "events".into();
		model.add_field(FieldState::new("n", FieldType::Integer, false));
		old.add_model(model);
	} else {
		old.add_sequence(SequenceDefinition::new(
			SequenceKey::new("events", "obsolete"),
			QualifiedName::new("events"),
		))
		.unwrap();
	}
	let initial = MigrationAutodetector::new(ProjectState::new(), old.clone())
		.try_generate_operations()
		.unwrap()
		.into_iter()
		.fold(
			Migration::new("0001_initial", "events"),
			|migration, operation| migration.add_operation(operation),
		);
	let mut new = old.clone();
	new.sequences
		.get_mut(&SequenceKey::new("events", "counter"))
		.unwrap()
		.name = QualifiedName::new("events");
	if table {
		new.models.values_mut().next().unwrap().table_name = "archived_events".into();
	} else {
		new.sequences
			.remove(&SequenceKey::new("events", "obsolete"));
	}
	let change = MigrationAutodetector::new(old, new)
		.try_generate_operations()
		.unwrap()
		.into_iter()
		.fold(
			Migration::new("0002_reuse", "events").add_dependency("events", "0001_initial"),
			|migration, operation| migration.add_operation(operation),
		);
	let mut executor =
		reinhardt_db::migrations::DatabaseMigrationExecutor::new(fixture.connection.clone());
	executor.apply_migrations(&[initial]).await.unwrap();
	// Act
	executor
		.apply_migrations(std::slice::from_ref(&change))
		.await
		.unwrap();
	let renamed: i64 = sqlx::query_scalar("SELECT nextval('events')")
		.fetch_one(&fixture.pool)
		.await
		.unwrap();
	executor.rollback_migrations(&[change]).await.unwrap();
	let restored: i64 = sqlx::query_scalar("SELECT nextval('counter')")
		.fetch_one(&fixture.pool)
		.await
		.unwrap();
	// Assert
	assert_eq!(renamed, 1);
	assert_eq!(restored, 2);
	let kind: String =
		sqlx::query_scalar("SELECT relkind::text FROM pg_class WHERE oid = 'events'::regclass")
			.fetch_one(&fixture.pool)
			.await
			.unwrap();
	assert_eq!(kind, if table { "r" } else { "S" });
}

#[rstest]
#[tokio::test]
async fn embedded_sequence_ownership_matches_catalog_and_column_lifetime() {
	// Arrange
	let fixture = postgres("16-alpine").await;
	let definition = SequenceDefinition::new(
		SequenceKey::new("events", "counter"),
		QualifiedName::new("counter"),
	)
	.with_owned_by(Some(SequenceOwner::new(QualifiedName::new("events"), "n")));
	let initial = Migration::new("0001_initial", "events")
		.add_operation(Operation::CreateTable {
			name: "events".into(),
			columns: vec![ColumnDefinition::new("n", FieldType::Integer).with_not_null(true)],
			constraints: vec![],
			without_rowid: None,
			interleave_in_parent: None,
			partition: None,
		})
		.add_operation(Operation::Sequence {
			operation: SequenceOperation::Create { definition },
		});
	let drop = Migration::new("0002_drop", "events")
		.add_dependency("events", "0001_initial")
		.add_operation(Operation::DropColumn {
			table: "events".into(),
			column: "n".into(),
			old_definition: Some(
				ColumnDefinition::new("n", FieldType::Integer).with_not_null(true),
			),
		});
	let mut executor =
		reinhardt_db::migrations::DatabaseMigrationExecutor::new(fixture.connection.clone());
	// Act
	executor.apply_migrations(&[initial]).await.unwrap();
	let sequences = PostgresIntrospector::new(fixture.pool.clone())
		.read_sequences()
		.await
		.unwrap();
	executor.apply_migrations(&[drop]).await.unwrap();
	let remaining = PostgresIntrospector::new(fixture.pool.clone())
		.read_sequences()
		.await
		.unwrap();
	// Assert
	assert_eq!(sequences[0].owned_by.as_ref().unwrap().table.name, "events");
	assert_eq!(sequences[0].owned_by.as_ref().unwrap().column, "n");
	assert!(
		!remaining
			.iter()
			.any(|sequence| sequence.name.name == "counter")
	);
}

#[rstest]
#[case(false)]
#[case(true)]
#[tokio::test]
async fn schema_inspection_reads_the_sequence_catalog_once(#[case] selected: bool) {
	// Arrange
	let fixture = postgres_options("16-alpine", true).await;
	// Server statistics measure catalog queries across both introspector pools.
	sqlx::query("CREATE EXTENSION pg_stat_statements")
		.execute(&fixture.pool)
		.await
		.unwrap();
	let columns = (0..16)
		.map(|n| {
			ColumnDefinition::new(format!("n{n}"), FieldType::Integer)
				.with_not_null(true)
				.with_identity(Some(IdentityDefinition::new(IdentityGeneration::Always)))
		})
		.collect();
	let initial = Migration::new("0001_initial", "events").add_operation(Operation::CreateTable {
		name: "events".into(),
		columns,
		constraints: vec![],
		without_rowid: None,
		interleave_in_parent: None,
		partition: None,
	});
	let mut executor =
		reinhardt_db::migrations::DatabaseMigrationExecutor::new(fixture.connection.clone());
	executor.apply_migrations(&[initial]).await.unwrap();
	sqlx::query("SELECT pg_stat_statements_reset()")
		.execute(&fixture.pool)
		.await
		.unwrap();
	let mut options = reinhardt_db::migrations::introspection::InspectDbOptions::default();
	if selected {
		options.tables = vec!["events".into()];
	}
	// Act
	let schema =
		reinhardt_db::migrations::introspection::inspect_database(&fixture.connection, &options)
			.await
			.unwrap();
	let calls: i64 = sqlx::query_scalar("SELECT calls::bigint FROM pg_stat_statements WHERE query LIKE '%FROM pg_sequence s%' AND query NOT LIKE '%pg_stat_statements%'").fetch_one(&fixture.pool).await.unwrap();
	// Assert
	assert_eq!(schema.tables["events"].columns.len(), 16);
	assert_eq!(
		schema.tables["events"]
			.columns
			.values()
			.filter(|column| column.identity.is_some())
			.count(),
		16
	);
	assert_eq!(calls, 1);
}

#[rstest]
#[tokio::test]
async fn qualified_identity_preflight_and_rollback_use_the_requested_schema() {
	use reinhardt_query::prelude::{
		Alias, ColumnDef, ColumnType, PostgresQueryBuilder, Query, QueryStatementBuilder,
	};
	// Arrange
	let fixture = postgres("16-alpine").await;
	let mut before = ProjectState::new();
	for (schema, cache) in [("alpha", 2), ("beta", 4)] {
		let sql = Query::create_schema()
			.name(Alias::new(schema))
			.to_string(PostgresQueryBuilder);
		sqlx::query(&sql).execute(&fixture.pool).await.unwrap();
		let sql = Query::create_table()
			.table((Alias::new(schema), Alias::new("events")))
			.col(
				ColumnDef::new("n")
					.column_type(ColumnType::BigInteger)
					.not_null(true),
			)
			.to_string(PostgresQueryBuilder);
		sqlx::query(&sql).execute(&fixture.pool).await.unwrap();
		let identity = IdentityDefinition::new(IdentityGeneration::Always)
			.with_options(SequenceOptions::new().with_cache(cache));
		let sql = IdentityOperation::new(
			QualifiedName::new("events").with_schema(schema),
			"n",
			FieldType::BigInteger,
			None,
			Some(identity.clone()),
		)
		.to_sql();
		sqlx::query(&sql).execute(&fixture.pool).await.unwrap();
		let mut model = ModelState::new(schema, "Event");
		model.table_name = "events".into();
		model.options.insert("schema".into(), schema.into());
		model.add_field(FieldState::new("n", FieldType::BigInteger, false).with_identity(identity));
		before.add_model(model);
	}
	let migration = Migration::new("0002_generation", "beta").add_operation(Operation::Identity {
		operation: IdentityOperation::new(
			QualifiedName::new("events").with_schema("beta"),
			"n",
			FieldType::BigInteger,
			Some(
				IdentityDefinition::new(IdentityGeneration::Always)
					.with_options(SequenceOptions::new().with_cache(4)),
			),
			Some(
				IdentityDefinition::new(IdentityGeneration::ByDefault)
					.with_options(SequenceOptions::new().with_cache(8)),
			),
		),
	});
	let after = replay(&before, &migration);
	let catalog_query = "SELECT n.nspname::text, a.attidentity::text, s.seqcache FROM pg_attribute a JOIN pg_class t ON t.oid=a.attrelid JOIN pg_namespace n ON n.oid=t.relnamespace JOIN pg_depend d ON d.refobjid=t.oid AND d.refobjsubid=a.attnum AND d.deptype='i' JOIN pg_sequence s ON s.seqrelid=d.objid WHERE n.nspname IN ('alpha', 'beta') AND t.relname='events' AND a.attname='n' ORDER BY n.nspname";
	// Act
	execute(
		&fixture,
		&migration,
		&before,
		&after,
		MigrationDirection::Forward,
	)
	.await;
	let changed: Vec<(String, String, i64)> = sqlx::query_as(catalog_query)
		.fetch_all(&fixture.pool)
		.await
		.unwrap();
	execute(
		&fixture,
		&migration,
		&before,
		&after,
		MigrationDirection::Backward,
	)
	.await;
	let restored: Vec<(String, String, i64)> = sqlx::query_as(catalog_query)
		.fetch_all(&fixture.pool)
		.await
		.unwrap();
	// Assert
	assert_eq!(
		changed,
		vec![
			("alpha".into(), "a".into(), 2),
			("beta".into(), "d".into(), 8)
		]
	);
	assert_eq!(
		restored,
		vec![
			("alpha".into(), "a".into(), 2),
			("beta".into(), "a".into(), 4)
		]
	);
	assert_eq!(
		after.models[&("alpha".into(), "Event".into())],
		before.models[&("alpha".into(), "Event".into())]
	);
}
