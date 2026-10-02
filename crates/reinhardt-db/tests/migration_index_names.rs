//! Regression coverage for generated expression and partial index names (issue #6443).

#![cfg(feature = "migrations")]

use reinhardt_db::migrations::{
	FieldState, FieldType, IndexDefinition, MigrationAutodetector, ModelState, Operation,
	ProjectState, SqlDialect,
};
use rstest::*;

fn create_index(expressions: &[&str], predicate: Option<&str>) -> Operation {
	Operation::CreateIndex {
		table: "events".to_owned(),
		columns: vec!["sequence".to_owned()],
		unique: false,
		index_type: None,
		where_clause: predicate.map(str::to_owned),
		concurrently: false,
		expressions: (!expressions.is_empty()).then(|| {
			expressions
				.iter()
				.map(|expression| (*expression).to_owned())
				.collect()
		}),
		mysql_options: None,
		operator_class: None,
	}
}

fn index_name(operation: &Operation) -> String {
	match operation
		.to_reverse_operation(&ProjectState::new())
		.unwrap()
		.unwrap()
	{
		Operation::DropNamedIndex { name, .. } => name,
		other => panic!("expected DropNamedIndex, got {other:?}"),
	}
}

fn repair_index(
	expressions: &[&str],
	predicate: Option<&str>,
	explicit_name: Option<&str>,
) -> Operation {
	let Operation::CreateIndex {
		table,
		columns,
		unique,
		index_type,
		where_clause,
		concurrently,
		expressions,
		mysql_options,
		operator_class,
	} = create_index(expressions, predicate)
	else {
		panic!("create_index must return CreateIndex");
	};
	Operation::CreateIndexRepair {
		table,
		name: explicit_name.map(str::to_owned),
		columns,
		unique,
		index_type,
		where_clause,
		concurrently,
		expressions,
		mysql_options,
		operator_class,
	}
}

#[rstest]
#[case::expressions(
	create_index(&["(data->>'tenant')", "sequence"], None),
	create_index(&["(data->>'key')", "sequence"], None),
)]
#[case::predicates(
	create_index(&[], Some("sequence > 0")),
	create_index(&[], Some("sequence > 1")),
)]
#[case::expression_predicates(
	create_index(&["LOWER(data::text)"], Some("sequence > 0")),
	create_index(&["LOWER(data::text)"], Some("sequence > 1")),
)]
#[case::expression_order(
	create_index(&["(data->>'tenant')", "sequence"], None),
	create_index(&["sequence", "(data->>'tenant')"], None),
)]
#[case::expression_boundaries(
	create_index(&["'ab'", "'c'"], None),
	create_index(&["'a'", "'bc'"], None),
)]
fn distinct_index_definitions_have_distinct_names(
	#[case] left: Operation,
	#[case] right: Operation,
) {
	// Act
	let left_name = index_name(&left);
	let right_name = index_name(&right);

	// Assert
	assert_ne!(left_name, right_name);
	assert_eq!(index_name(&left), left_name);
	assert_eq!(index_name(&right), right_name);
}

#[rstest]
#[case::tenant(
	create_index(&["(data->>'tenant')", "sequence"], None),
	"CREATE INDEX idx_events_expr_08afe3380f30b13d ON events ((data->>'tenant'), sequence);",
)]
#[case::key(
	create_index(&["(data->>'key')", "sequence"], None),
	"CREATE INDEX idx_events_expr_83bcee3c151bb4f6 ON events ((data->>'key'), sequence);",
)]
#[case::partial(
	create_index(&[], Some("sequence > 0")),
	"CREATE INDEX idx_events_sequence_3faf72889e35dcef ON events (sequence) WHERE sequence > 0;",
)]
#[case::ordinary(create_index(&[], None), "CREATE INDEX idx_events_sequence ON events (sequence);")]
fn generated_names_have_stable_sql(#[case] operation: Operation, #[case] expected: &str) {
	assert_eq!(operation.to_sql(&SqlDialect::Postgres), expected);
}

#[rstest]
#[case::expression(create_index(&["(data->>'tenant')", "sequence"], None))]
#[case::partial(create_index(&[], Some("sequence > 0")))]
fn statement_rendering_uses_the_same_index_name(#[case] operation: Operation) {
	// Arrange
	let expected_name = index_name(&operation);

	// Act
	let sql = operation
		.to_statement()
		.to_sql_string(reinhardt_db::backends::types::DatabaseType::Postgres);

	// Assert
	assert_eq!(
		sql.split_once(" ON ")
			.unwrap()
			.0
			.strip_prefix("CREATE INDEX ")
			.unwrap()
			.trim_matches('"'),
		expected_name
	);
}

#[rstest]
#[case(SqlDialect::Postgres)]
#[case(SqlDialect::Sqlite)]
#[case(SqlDialect::Mysql)]
#[case(SqlDialect::Cockroachdb)]
fn create_and_rollback_use_the_same_index_name(
	#[case] dialect: SqlDialect,
	#[values(None, Some("sequence > 0"))] predicate: Option<&str>,
) {
	// Arrange
	let operation = create_index(&["(data->>'tenant')", "sequence"], predicate);
	let name = index_name(&operation);
	let where_sql = predicate
		.filter(|_| !matches!(dialect, SqlDialect::Mysql))
		.map_or_else(String::new, |predicate| format!(" WHERE {predicate}"));
	let mysql = matches!(dialect, SqlDialect::Mysql);
	let on_table = if mysql { " ON `events`" } else { "" };
	let table = if mysql { "`events`" } else { "events" };
	let forward_name = if mysql {
		format!("`{name}`")
	} else {
		name.clone()
	};
	let reverse_name = if mysql {
		format!("`{name}`")
	} else {
		format!("\"{name}\"")
	};

	// Act
	let sql = operation.to_sql(&dialect);
	let reverse = operation
		.to_reverse_sql(&dialect, &ProjectState::new())
		.unwrap()
		.unwrap();
	let reverse_operation = operation
		.to_reverse_operation(&ProjectState::new())
		.unwrap()
		.unwrap();

	// Assert
	assert_eq!(
		sql,
		format!("CREATE INDEX {forward_name} ON {table} ((data->>'tenant'), sequence){where_sql};")
	);
	assert_eq!(
		reverse,
		vec![format!("DROP INDEX {reverse_name}{on_table};")]
	);
	assert_eq!(
		reverse_operation.to_sql(&dialect),
		format!("DROP INDEX {forward_name}{on_table};")
	);
}

#[rstest]
fn empty_expressions_preserve_ordinary_index_names() {
	// Arrange
	let mut operation = create_index(&[], None);
	if let Operation::CreateIndex { expressions, .. } = &mut operation {
		*expressions = Some(Vec::new());
	}

	// Act
	let sql = operation.to_sql(&SqlDialect::Postgres);
	let name = index_name(&operation);

	// Assert
	assert_eq!(
		sql,
		"CREATE INDEX idx_events_sequence ON events (sequence);"
	);
	assert_eq!(name, "idx_events_sequence");
}

#[rstest]
fn expression_index_names_ignore_unused_columns() {
	// Arrange
	let left = create_index(&["LOWER(data::text)"], None);
	let mut right = left.clone();
	if let Operation::CreateIndex { columns, .. } = &mut right {
		*columns = vec!["unused".to_owned()];
	}

	// Assert
	assert_eq!(
		left.to_sql(&SqlDialect::Postgres),
		right.to_sql(&SqlDialect::Postgres)
	);
	assert_eq!(index_name(&left), index_name(&right));
}

#[rstest]
#[case::ascii("t".repeat(60), 63)]
#[case::unicode(format!("a{}", "表".repeat(20)), 61)]
fn long_index_names_retain_distinct_hashes(#[case] table: String, #[case] expected_bytes: usize) {
	// Arrange
	let mut left = create_index(&["(data->>'tenant')", "sequence"], None);
	let mut right = create_index(&["(data->>'key')", "sequence"], None);
	for operation in [&mut left, &mut right] {
		if let Operation::CreateIndex {
			table: index_table, ..
		} = operation
		{
			*index_table = table.clone();
		}
	}

	// Act
	let left_name = index_name(&left);
	let right_name = index_name(&right);

	// Assert
	assert_eq!(left_name.len(), expected_bytes);
	assert_eq!(right_name.len(), expected_bytes);
	assert_ne!(left_name, right_name);
}

#[rstest]
fn long_table_names_with_common_prefixes_have_distinct_index_names() {
	// Arrange
	let mut left = create_index(&["LOWER(data::text)"], None);
	let mut right = left.clone();
	for (operation, suffix) in [(&mut left, "a"), (&mut right, "b")] {
		if let Operation::CreateIndex { table, .. } = operation {
			*table = format!("{}{suffix}", "t".repeat(59));
		}
	}

	// Act
	let left_name = index_name(&left);
	let right_name = index_name(&right);

	// Assert
	assert_eq!(left_name.len(), 63);
	assert_eq!(right_name.len(), 63);
	assert_ne!(left_name, right_name);
}

#[rstest]
fn replay_and_removal_preserve_each_index_name() {
	// Arrange
	let operations = vec![
		create_index(&["(data->>'tenant')", "sequence"], None),
		create_index(&["(data->>'key')", "sequence"], None),
		create_index(&[], Some("sequence > 0")),
		create_index(&[], Some("sequence > 1")),
	];
	let mut model = ModelState::new("app", "Event");
	model.table_name = "events".to_owned();
	model.add_field(FieldState::new("sequence", FieldType::BigInteger, false));
	model.add_field(FieldState::new("data", FieldType::Jsonb, false));
	let mut replayed = ProjectState::new();
	replayed.add_model(model);
	let without_indexes = replayed.clone();
	let expected = vec![
		"idx_events_expr_08afe3380f30b13d",
		"idx_events_expr_83bcee3c151bb4f6",
		"idx_events_sequence_3faf72889e35dcef",
		"idx_events_sequence_95cc28f21275a2db",
	];

	// Act
	replayed.apply_migration_operations(&operations, "app");
	let mut directly_replayed = without_indexes.clone();
	for operation in &operations {
		operation.state_forwards("app", &mut directly_replayed);
	}
	let direct_names: Vec<_> = directly_replayed
		.find_model_by_table("events")
		.unwrap()
		.indexes
		.iter()
		.map(|index| index.name.as_str())
		.collect();
	let replayed_names: Vec<_> = replayed
		.find_model_by_table("events")
		.unwrap()
		.indexes
		.iter()
		.map(|index| index.name.clone())
		.collect();
	let drops = MigrationAutodetector::new(replayed, without_indexes).generate_operations();
	let mut dropped_names: Vec<_> = drops
		.iter()
		.map(|operation| match operation {
			Operation::DropNamedIndex { name, .. } => name.as_str(),
			other => panic!("expected DropNamedIndex, got {other:?}"),
		})
		.collect();
	dropped_names.sort();

	// Assert
	assert_eq!(replayed_names, expected);
	assert_eq!(direct_names, expected);
	assert_eq!(dropped_names, expected);
}

#[rstest]
#[case::explicit(Some("idx_events_expr"))]
#[case::generated(None)]
fn repair_operations_preserve_explicit_or_generated_names(#[case] explicit_name: Option<&str>) {
	// Arrange
	let operation = create_index(&["(data->>'tenant')", "sequence"], Some("sequence > 0"));
	let state = ProjectState::new();
	let repair = repair_index(
		&["(data->>'tenant')", "sequence"],
		Some("sequence > 0"),
		explicit_name,
	);
	let expected_name = explicit_name
		.map(str::to_owned)
		.unwrap_or_else(|| index_name(&operation));

	// Act
	let sql = repair.to_sql(&SqlDialect::Postgres);
	let reverse = repair
		.to_reverse_sql(&SqlDialect::Postgres, &state)
		.unwrap();
	let reverse_operation = repair.to_reverse_operation(&state).unwrap();
	let mut replayed = ProjectState::new();
	let mut model = ModelState::new("app", "Event");
	model.table_name = "events".to_owned();
	replayed.add_model(model);
	repair.state_forwards("app", &mut replayed);

	// Assert
	assert_eq!(
		sql,
		format!(
			"CREATE INDEX {expected_name} ON events ((data->>'tenant'), sequence) WHERE sequence > 0;"
		)
	);
	assert_eq!(
		replayed.find_model_by_table("events").unwrap().indexes[0].name,
		expected_name
	);
	assert!(reverse.is_none());
	assert!(reverse_operation.is_none());
}

#[rstest]
fn model_partial_indexes_round_trip_without_migration_drift(
	#[values("idx_events_sequence", "positive_events")] name: &str,
	#[values(false, true)] existing_table: bool,
	#[values(false, true)] direct_replay: bool,
) {
	// Arrange
	let mut model = ModelState::new("app", "Events");
	model.table_name = "events".to_owned();
	model.add_field(FieldState::new("sequence", FieldType::BigInteger, false));
	let mut source = ProjectState::new();
	if existing_table {
		source.add_model(model.clone());
	}
	let mut index = IndexDefinition::new(name, vec!["sequence".to_owned()], false);
	index.where_clause = Some("sequence > 0".to_owned());
	model.indexes.push(index.clone());
	let mut target = ProjectState::new();
	target.add_model(model);
	let operations =
		MigrationAutodetector::new(source.clone(), target.clone()).generate_operations();

	// Act
	let mut replayed = source;
	if direct_replay {
		for operation in &operations {
			operation.state_forwards("app", &mut replayed);
		}
	} else {
		replayed.apply_migration_operations(&operations, "app");
	}
	let next_operations =
		MigrationAutodetector::new(replayed.clone(), target).generate_operations();

	// Assert
	assert_eq!(next_operations, Vec::<Operation>::new());
	assert_eq!(
		replayed.find_model_by_table("events").unwrap().indexes,
		vec![index]
	);
	let create = operations.last().unwrap();
	assert_eq!(index_name(create), name);
	let drop = create.to_reverse_operation(&replayed).unwrap().unwrap();
	assert_eq!(
		create
			.to_reverse_sql(&SqlDialect::Postgres, &replayed)
			.unwrap()
			.unwrap(),
		vec![format!("DROP INDEX {name};")]
	);
	let recreate = drop.to_reverse_operation(&replayed).unwrap().unwrap();
	assert_eq!(recreate, *create);
	assert_eq!(
		recreate.to_reverse_operation(&replayed).unwrap(),
		Some(drop)
	);
}

#[rstest]
fn model_partial_index_replacement_and_removal_preserve_rollback(
	#[values(None, Some("sequence > 1"))] predicate: Option<&str>,
) {
	// Arrange
	let mut model = ModelState::new("app", "Events");
	model.table_name = "events".to_owned();
	model.add_field(FieldState::new("sequence", FieldType::BigInteger, false));
	let mut index = IndexDefinition::new("idx_events_sequence", vec!["sequence".to_owned()], false);
	index.where_clause = Some("sequence > 0".to_owned());
	model.indexes.push(index);
	let mut source = ProjectState::new();
	source.add_model(model.clone());
	model.indexes.clear();
	if let Some(predicate) = predicate {
		let mut index =
			IndexDefinition::new("idx_events_sequence", vec!["sequence".to_owned()], false);
		index.where_clause = Some(predicate.to_owned());
		model.indexes.push(index);
	}
	let mut target = ProjectState::new();
	target.add_model(model);

	// Act
	let operations =
		MigrationAutodetector::new(source.clone(), target.clone()).generate_operations();
	let mut replayed = source.clone();
	replayed.apply_migration_operations(&operations, "app");
	let rollback: Vec<_> = operations
		.iter()
		.rev()
		.map(|operation| operation.to_reverse_operation(&source).unwrap().unwrap())
		.collect();
	let mut rolled_back = replayed.clone();
	rolled_back.apply_migration_operations(&rollback, "app");

	// Assert
	assert_eq!(operations.len(), if predicate.is_some() { 2 } else { 1 });
	assert_eq!(
		MigrationAutodetector::new(replayed, target).generate_operations(),
		Vec::<Operation>::new()
	);
	assert_eq!(
		MigrationAutodetector::new(rolled_back, source).generate_operations(),
		Vec::<Operation>::new()
	);
	for (operation, reverse) in operations.iter().zip(rollback.iter().rev()) {
		assert_eq!(
			reverse.to_reverse_operation(&ProjectState::new()).unwrap(),
			Some(operation.clone())
		);
	}
}

#[cfg(all(feature = "postgres", feature = "backends"))]
mod postgres {
	use super::*;
	use reinhardt_db::backends::connection::DatabaseConnection;
	use reinhardt_db::migrations::ColumnDefinition;
	use testcontainers::{ContainerAsync, runners::AsyncRunner};
	use testcontainers_modules::postgres::Postgres;

	#[fixture]
	async fn database() -> (ContainerAsync<Postgres>, DatabaseConnection) {
		let container = Postgres::default().start().await.unwrap();
		let url = format!(
			"postgres://postgres:postgres@{}:{}/postgres",
			container.get_host().await.unwrap(),
			container.get_host_port_ipv4(5432).await.unwrap(),
		);
		let connection = DatabaseConnection::connect_postgres(&url).await.unwrap();
		(container, connection)
	}

	#[rstest]
	#[tokio::test]
	async fn postgres_model_partial_index_keeps_declared_name_and_reversibility(
		#[future] database: (ContainerAsync<Postgres>, DatabaseConnection),
	) {
		// Arrange
		let (_container, connection) = database.await;
		let mut model = ModelState::new("app", "Events");
		model.table_name = "events".to_owned();
		model.add_field(FieldState::new("sequence", FieldType::BigInteger, false));
		let mut index =
			IndexDefinition::new("idx_events_sequence", vec!["sequence".to_owned()], false);
		index.where_clause = Some("sequence > 0".to_owned());
		model.indexes.push(index);
		let mut target = ProjectState::new();
		target.add_model(model);
		let operations =
			MigrationAutodetector::new(ProjectState::new(), target.clone()).generate_operations();
		let create = operations.last().unwrap();
		let drop = create.to_reverse_operation(&target).unwrap().unwrap();
		let recreate = drop.to_reverse_operation(&target).unwrap().unwrap();

		// Act
		let mut affected_rows = Vec::new();
		for operation in &operations {
			let result = connection
				.execute(
					&operation.try_to_sql(&SqlDialect::Postgres).unwrap(),
					Vec::new(),
				)
				.await
				.unwrap();
			affected_rows.push(result.rows_affected);
		}
		for sql in create
			.to_reverse_sql(&SqlDialect::Postgres, &target)
			.unwrap()
			.unwrap()
		{
			let result = connection.execute(&sql, Vec::new()).await.unwrap();
			affected_rows.push(result.rows_affected);
		}
		for operation in [&recreate, &drop] {
			let result = connection
				.execute(
					&operation.try_to_sql(&SqlDialect::Postgres).unwrap(),
					Vec::new(),
				)
				.await
				.unwrap();
			affected_rows.push(result.rows_affected);
		}

		// Assert
		assert_eq!(operations.len(), 2);
		assert_eq!(index_name(create), "idx_events_sequence");
		assert_eq!(recreate, *create);
		assert_eq!(affected_rows, vec![0; 5]);
	}

	#[rstest]
	#[tokio::test]
	async fn postgres_creates_and_reverses_distinct_indexes(
		#[future] database: (ContainerAsync<Postgres>, DatabaseConnection),
	) {
		// Arrange
		let (_container, connection) = database.await;
		let table = Operation::CreateTable {
			name: "events".to_owned(),
			columns: vec![
				ColumnDefinition::new("data", FieldType::Jsonb),
				ColumnDefinition::new("sequence", FieldType::BigInteger),
			],
			constraints: Vec::new(),
			without_rowid: None,
			partition: None,
			interleave_in_parent: None,
		};
		connection
			.execute(&table.to_sql(&SqlDialect::Postgres), Vec::new())
			.await
			.unwrap();
		let indexes = vec![
			create_index(&["(data->>'tenant')", "sequence"], None),
			create_index(&["(data->>'key')", "sequence"], None),
			create_index(&[], Some("sequence > 0")),
			create_index(&[], Some("sequence > 1")),
		];
		let state = ProjectState::new();

		// Upgrade an already-applied legacy expression index before automatic rollback.
		let legacy_index = repair_index(
			&["(data->>'tenant')", "sequence"],
			None,
			Some("idx_events_expr"),
		);
		connection
			.execute(&legacy_index.to_sql(&SqlDialect::Postgres), Vec::new())
			.await
			.unwrap();
		let rename = Operation::RunSQL {
			sql: format!(
				"ALTER INDEX idx_events_expr RENAME TO {};",
				index_name(&indexes[0])
			),
			reverse_sql: None,
		};
		connection
			.execute(&rename.to_sql(&SqlDialect::Postgres), Vec::new())
			.await
			.unwrap();
		for sql in indexes[0]
			.to_reverse_sql(&SqlDialect::Postgres, &state)
			.unwrap()
			.unwrap()
		{
			connection.execute(&sql, Vec::new()).await.unwrap();
		}

		// CREATE
		for index in &indexes {
			let result = connection
				.execute(&index.to_sql(&SqlDialect::Postgres), Vec::new())
				.await
				.unwrap();
			assert_eq!(result.rows_affected, 0);
		}

		// DELETE
		for index in &indexes {
			for sql in index
				.to_reverse_sql(&SqlDialect::Postgres, &state)
				.unwrap()
				.unwrap()
			{
				let result = connection.execute(&sql, Vec::new()).await.unwrap();
				assert_eq!(result.rows_affected, 0);
			}
		}

		// CREATE: both rollback APIs must target the same physical names.
		for index in &indexes {
			let result = connection
				.execute(&index.to_sql(&SqlDialect::Postgres), Vec::new())
				.await
				.unwrap();
			assert_eq!(result.rows_affected, 0);
			let drop = index.to_reverse_operation(&state).unwrap().unwrap();
			let result = connection
				.execute(&drop.to_sql(&SqlDialect::Postgres), Vec::new())
				.await
				.unwrap();
			assert_eq!(result.rows_affected, 0);
		}
	}
}
