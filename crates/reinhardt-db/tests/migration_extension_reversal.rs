#![cfg(feature = "migrations")]

use quote::ToTokens;
use reinhardt_db::migrations::operations::{
	OperationStatement, SqlDialect, postgres::CreateExtension,
};
use reinhardt_db::migrations::{
	FilesystemRepository, FilesystemSource, Migration, MigrationError, MigrationRepository,
	MigrationSource, Operation, ProjectState, ast_parser,
};
use rstest::*;

#[fixture]
fn owned_extension() -> Operation {
	CreateExtension::new("hstore")
		.with_schema("public")
		.with_if_not_exists(false)
		.into_operation()
		.unwrap()
}

#[rstest]
fn owned_extension_has_a_typed_inverse(owned_extension: Operation) {
	// Act
	let inverse = owned_extension
		.to_reverse_operation(&ProjectState::new())
		.unwrap()
		.unwrap();
	let sql = owned_extension
		.to_reverse_sql(&SqlDialect::Postgres, &ProjectState::new())
		.unwrap();

	// Assert
	assert_eq!(
		inverse,
		Operation::DropExtension {
			name: "hstore".into(),
			if_exists: false,
			cascade: false,
		}
	);
	assert_eq!(sql, Some(vec!["DROP EXTENSION hstore;".into()]));
}

#[rstest]
#[case(false, false, "DROP EXTENSION \"extension\"\"name\";")]
#[case(true, false, "DROP EXTENSION IF EXISTS \"extension\"\"name\";")]
#[case(false, true, "DROP EXTENSION \"extension\"\"name\" CASCADE;")]
#[case(true, true, "DROP EXTENSION IF EXISTS \"extension\"\"name\" CASCADE;")]
fn drop_extension_sql(#[case] if_exists: bool, #[case] cascade: bool, #[case] expected: &str) {
	let operation = Operation::DropExtension {
		name: "extension\"name".into(),
		if_exists,
		cascade,
	};

	assert_eq!(
		operation.try_to_sql(&SqlDialect::Postgres).unwrap(),
		expected
	);
	let OperationStatement::RawSql(statement) = operation.to_statement() else {
		panic!("extension DDL must use a raw SQL statement");
	};
	assert_eq!(statement, expected);
}

#[rstest]
#[case(SqlDialect::Mysql, "mysql")]
#[case(SqlDialect::Sqlite, "sqlite")]
#[case(SqlDialect::Cockroachdb, "cockroachdb")]
fn drop_extension_rejects_unsupported_backends(#[case] dialect: SqlDialect, #[case] backend: &str) {
	let operation = Operation::DropExtension {
		name: "hstore".into(),
		if_exists: false,
		cascade: false,
	};

	assert_eq!(
		operation.try_to_sql(&dialect).unwrap_err().to_string(),
		format!("PostgreSQL extensions is not supported by the {backend} backend")
	);
}

#[rstest]
fn conditional_creation_rejects_an_unsafe_inverse() {
	let operation = CreateExtension::new("hstore").into_operation().unwrap();
	let expected = "Irreversible migration: Cannot automatically reverse CREATE EXTENSION IF NOT EXISTS hstore: ownership is unknown; use if_not_exists: false for a migration-owned extension";

	assert_eq!(
		operation
			.to_reverse_operation(&ProjectState::new())
			.unwrap_err()
			.to_string(),
		expected
	);
	assert_eq!(
		operation
			.to_reverse_sql(&SqlDialect::Postgres, &ProjectState::new())
			.unwrap_err()
			.to_string(),
		expected
	);
}

#[rstest]
fn legacy_extension_json_preserves_conditional_creation() {
	let extension: CreateExtension =
		serde_json::from_str(r#"{"name":"hstore","schema":"public","version":null}"#).unwrap();

	assert_eq!(
		extension.into_operation().unwrap(),
		Operation::CreateExtension {
			name: "hstore".into(),
			if_not_exists: true,
			schema: Some("public".into()),
		}
	);
}

#[rstest]
#[tokio::test]
async fn extension_operations_round_trip_through_filesystem(owned_extension: Operation) {
	// Arrange
	let inverse = owned_extension
		.to_reverse_operation(&ProjectState::new())
		.unwrap()
		.unwrap();
	let mut migration = Migration::new("0001_extensions", "probe");
	migration.operations = vec![owned_extension, inverse];
	let directory = tempfile::tempdir().unwrap();
	let mut repository = FilesystemRepository::new(directory.path());

	// Act: token rendering, strict AST loading, and normal filesystem loading.
	let tokens = migration.operations[1].to_token_stream();
	let token_source = format!(
		"pub fn migration() -> Migration {{ Migration {{ operations: vec![{tokens}], ..Migration::new(\"0001_extensions\", \"probe\") }} }}"
	);
	let token_ast = syn::parse_file(&token_source).unwrap();
	let parsed =
		ast_parser::extract_migration_metadata(&token_ast, "probe", "0001_extensions").unwrap();
	repository.save(&migration).await.unwrap();
	let restored = repository.get("probe", "0001_extensions").await.unwrap();
	let loaded = FilesystemSource::new(directory.path())
		.all_migrations()
		.await
		.unwrap();

	// Assert
	assert_eq!(parsed.operations, vec![migration.operations[1].clone()]);
	assert_eq!(restored.operations, migration.operations);
	assert_eq!(loaded.len(), 1);
	assert_eq!(loaded[0].operations, migration.operations);
	let serialized = serde_json::to_string(&migration.operations).unwrap();
	assert_eq!(
		serde_json::from_str::<Vec<Operation>>(&serialized).unwrap(),
		migration.operations
	);
}

#[rstest]
fn drop_extension_is_explicitly_irreversible() {
	let operation = reinhardt_db::migrations::operations::postgres::DropExtension::new("hstore")
		.into_operation();
	assert!(matches!(
		operation.to_reverse_sql(&SqlDialect::Postgres, &ProjectState::new()),
		Err(MigrationError::IrreversibleError(message))
			if message == "Cannot automatically reverse DROP EXTENSION hstore: the original schema and version are unknown"
	));
}
