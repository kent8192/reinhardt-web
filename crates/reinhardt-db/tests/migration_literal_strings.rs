#![cfg(feature = "migrations")]

use reinhardt_db::migrations::ast_parser::extract_migration_metadata_strict;
use reinhardt_db::migrations::{
	ColumnDefinition, Constraint, FieldType, FilesystemSource, MigrationError, MigrationSource,
	Operation,
};
use rstest::rstest;

fn migration_source(conversion: &str) -> String {
	r##"// reinhardt-migration-source: 1
use reinhardt::db::migrations::prelude::*;

pub fn migration() -> Migration {
	Migration::new("0001_probe", "probe")
		.add_operation(Operation::RunSQL {
			sql: r#"SELECT '🦀'"#STRING_SUFFIX,
			reverse_sql: Some("SELECT 'line\nbreak'"STRING_SUFFIX),
		})
		.add_operation(Operation::CreateTable {
			name: "probe_records"STRING_SUFFIX,
			columns: vec![
				ColumnDefinition::new("id", FieldType::Integer),
				ColumnDefinition::new("label", FieldType::VarChar(64)),
			],
			constraints: vec![Constraint::Unique {
				name: "probe_records_label_key"STRING_SUFFIX,
				columns: vec!["id"STRING_SUFFIX, "label"STRING_SUFFIX],
			}],
			without_rowid: None,
			interleave_in_parent: None,
			partition: None,
		})
		.add_operation(Operation::CreateIndex {
			table: "probe_records"STRING_SUFFIX,
			columns: vec!["label"STRING_SUFFIX],
			unique: false,
			index_type: None,
			where_clause: Some("label IS NOT NULL"STRING_SUFFIX),
			concurrently: false,
			expressions: Some(vec!["LOWER(label)"STRING_SUFFIX]),
			mysql_options: None,
			operator_class: None,
		})
		.atomic(true)
}
"##
	.replace("STRING_SUFFIX", conversion)
}

fn expected_operations() -> Vec<Operation> {
	vec![
		Operation::RunSQL {
			sql: "SELECT '🦀'".into(),
			reverse_sql: Some("SELECT 'line\nbreak'".into()),
		},
		Operation::CreateTable {
			name: "probe_records".into(),
			columns: vec![
				ColumnDefinition::new("id", FieldType::Integer),
				ColumnDefinition::new("label", FieldType::VarChar(64)),
			],
			constraints: vec![Constraint::Unique {
				name: "probe_records_label_key".into(),
				columns: vec!["id".into(), "label".into()],
			}],
			without_rowid: None,
			interleave_in_parent: None,
			partition: None,
		},
		Operation::CreateIndex {
			table: "probe_records".into(),
			columns: vec!["label".into()],
			unique: false,
			index_type: None,
			where_clause: Some("label IS NOT NULL".into()),
			concurrently: false,
			expressions: Some(vec!["LOWER(label)".into()]),
			mysql_options: None,
			operator_class: None,
		},
	]
}

#[rstest]
#[case::literal("")]
#[case::to_string(".to_string()")]
#[case::into(".into()")]
#[case::to_owned(".to_owned()")]
#[case::to_string_empty_turbofish(".to_string::<>()")]
#[case::into_empty_turbofish(".into::<>()")]
#[case::to_owned_empty_turbofish(".to_owned::<>()")]
fn strict_parser_preserves_literal_strings(#[case] conversion: &str) {
	// Arrange
	let ast = syn::parse_file(&migration_source(conversion)).unwrap();

	// Act
	let migration = extract_migration_metadata_strict(&ast, "probe", "0001_probe").unwrap();

	// Assert
	assert_eq!(migration.operations, expected_operations());
	assert!(migration.atomic);
}

#[rstest]
#[case::to_string(".to_string()")]
#[case::into(".into()")]
#[case::to_owned(".to_owned()")]
#[case::to_string_empty_turbofish(".to_string::<>()")]
#[case::into_empty_turbofish(".into::<>()")]
#[case::to_owned_empty_turbofish(".to_owned::<>()")]
#[tokio::test]
async fn filesystem_source_loads_literal_string_conversions(#[case] conversion: &str) {
	// Arrange
	let directory = tempfile::tempdir_in("/tmp").unwrap();
	let app_directory = directory.path().join("probe");
	std::fs::create_dir(&app_directory).unwrap();
	std::fs::write(
		app_directory.join("0001_probe.rs"),
		migration_source(conversion),
	)
	.unwrap();
	let source = FilesystemSource::new(directory.path());

	// Act
	let migrations = source.all_migrations().await.unwrap();

	// Assert
	assert_eq!(migrations.len(), 1);
	assert_eq!(migrations[0].app_label, "probe");
	assert_eq!(migrations[0].name, "0001_probe");
	assert_eq!(migrations[0].operations, expected_operations());
	assert!(migrations[0].atomic);
}

#[rstest]
#[case::variable("sql.into()")]
#[case::function("build_sql().into()")]
#[case::format_macro(r#"format!("SELECT {}", 1).into()"#)]
#[case::non_string_literal("1.into()")]
#[case::non_string_to_string("1.to_string()")]
#[case::other_method(r#""SELECT 1".trim().into()"#)]
#[case::into_argument(r#""SELECT 1".into(1)"#)]
#[case::into_type_argument(r#""SELECT 1".into::<String>()"#)]
#[case::to_owned_argument(r#""SELECT 1".to_owned(1)"#)]
#[case::to_owned_type_argument(r#""SELECT 1".to_owned::<String>()"#)]
#[case::to_string_argument(r#""SELECT 1".to_string(1)"#)]
#[case::to_string_type_argument(r#""SELECT 1".to_string::<String>()"#)]
fn strict_parser_rejects_unsupported_string_expressions(#[case] expression: &str) {
	// Arrange
	let source = format!(
		r#"pub fn migration() -> Migration {{
			Migration::new("0001_probe", "probe")
				.add_operation(Operation::RunSQL {{ sql: {expression}, reverse_sql: None }})
		}}"#,
	);
	let ast = syn::parse_file(&source).unwrap();

	// Act
	let error = extract_migration_metadata_strict(&ast, "probe", "0001_probe").unwrap_err();

	// Assert
	assert!(matches!(
		error,
		MigrationError::InvalidMigration(message)
			if message == "operations[0].RunSQL.sql is unsupported or malformed"
	));
}
