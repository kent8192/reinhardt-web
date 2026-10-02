//! Regression coverage for generated expression and partial index names (issue #6443).

#![cfg(feature = "migrations")]

use reinhardt_db::migrations::{
	FieldState, FieldType, MigrationAutodetector, ModelState, Operation, ProjectState, SqlDialect,
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
		sql.split_once(" ON ").unwrap().0,
		format!("CREATE INDEX \"{expected_name}\"")
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
	let on_table = if matches!(dialect, SqlDialect::Mysql) {
		" ON events"
	} else {
		""
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
		format!("CREATE INDEX {name} ON events ((data->>'tenant'), sequence){where_sql};")
	);
	assert_eq!(reverse, vec![format!("DROP INDEX {name}{on_table};")]);
	assert_eq!(reverse_operation.to_sql(&dialect), reverse[0]);
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
	model.add_field(FieldState::new("data", FieldType::JsonBinary, false));
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
	assert_eq!(dropped_names, expected);
}

#[rstest]
#[case::explicit(Some("idx_events_expr"))]
#[case::generated(None)]
fn repair_operations_preserve_explicit_or_generated_names(#[case] explicit_name: Option<&str>) {
	// Arrange
	let operation = create_index(&["(data->>'tenant')", "sequence"], Some("sequence > 0"));
	let state = ProjectState::new();
	let drop = operation.to_reverse_operation(&state).unwrap().unwrap();
	let mut repair = drop.to_reverse_operation(&state).unwrap().unwrap();
	if let Operation::CreateIndexRepair { name, .. } = &mut repair {
		*name = explicit_name.map(str::to_owned);
	} else {
		panic!("expected CreateIndexRepair, got {repair:?}");
	}
	let expected_name = explicit_name
		.map(str::to_owned)
		.unwrap_or_else(|| index_name(&operation));

	// Act
	let sql = repair.to_sql(&SqlDialect::Postgres);
	let reverse = repair
		.to_reverse_sql(&SqlDialect::Postgres, &state)
		.unwrap()
		.unwrap();

	// Assert
	assert_eq!(
		sql,
		format!(
			"CREATE INDEX {expected_name} ON events ((data->>'tenant'), sequence) WHERE sequence > 0;"
		)
	);
	assert_eq!(index_name(&repair), expected_name);
	assert_eq!(reverse, vec![format!("DROP INDEX {expected_name};")]);
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
	async fn postgres_creates_and_reverses_distinct_indexes(
		#[future] database: (ContainerAsync<Postgres>, DatabaseConnection),
	) {
		// Arrange
		let (_container, connection) = database.await;
		let table = Operation::CreateTable {
			name: "events".to_owned(),
			columns: vec![
				ColumnDefinition::new("data", FieldType::JsonBinary),
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
