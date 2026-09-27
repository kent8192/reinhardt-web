//! Regression coverage for generated foreign-key identifiers (issue #6404).

#![cfg(feature = "migrations")]

use reinhardt_db::migrations::autodetector::ForeignKeyInfo;
use reinhardt_db::migrations::model_registry::{FieldMetadata, ManyToManyMetadata, ModelMetadata};
use reinhardt_db::migrations::{
	Constraint, FieldState, FieldType, ForeignKeyAction, MigrationAutodetector, ModelState,
	Operation, ProjectState,
};
use rstest::*;

const TABLE: &str = "registry_agent_resource_refs";
const SOURCE: &str = "related_registry_agent_resource_source_id";
const TARGET: &str = "related_registry_agent_resource_target_id";

#[fixture]
fn foreign_key() -> ForeignKeyInfo {
	ForeignKeyInfo {
		referenced_table: "registry_agent_resources".to_owned(),
		referenced_column: "id".to_owned(),
		on_delete: ForeignKeyAction::Cascade,
		on_update: ForeignKeyAction::NoAction,
	}
}

#[fixture]
fn long_foreign_keys(foreign_key: ForeignKeyInfo) -> ProjectState {
	let mut resource = ModelMetadata::new("registry", "Resource", "registry_agent_resources");
	resource.add_field(
		"id".to_owned(),
		FieldMetadata::new(FieldType::Integer).with_param("primary_key", "true"),
	);
	let mut refs = ModelMetadata::new("registry", "ResourceRef", TABLE);
	refs.add_field(
		"id".to_owned(),
		FieldMetadata::new(FieldType::Integer).with_param("primary_key", "true"),
	);
	for column in [SOURCE, TARGET] {
		refs.add_field(
			column.to_owned(),
			FieldMetadata::new(FieldType::Integer).with_foreign_key(foreign_key.clone()),
		);
	}
	let mut state = ProjectState::new();
	state.add_model(resource.to_model_state());
	state.add_model(refs.to_model_state());
	state
}

#[rstest]
#[case::short("posts".to_owned(), "author_id".to_owned())]
#[case::exact_limit("t".repeat(30), "c".repeat(29))]
#[case::unicode_at_limit("表".repeat(10), format!("{}ab", "列".repeat(9)))]
fn preserves_foreign_key_names_within_limit(
	foreign_key: ForeignKeyInfo,
	#[case] table: String,
	#[case] column: String,
) {
	// Arrange
	let mut model = ModelState::new("registry", "ResourceRef");
	model.table_name = table.clone();
	model.add_field(FieldState::with_foreign_key(
		&column,
		FieldType::Integer,
		false,
		foreign_key,
	));

	// Act
	model.add_foreign_key_constraint_from_field(&column);

	// Assert
	assert_eq!(model.constraints.len(), 1);
	assert_eq!(model.constraints[0].name, format!("fk_{table}_{column}"));
}

#[rstest]
#[case::one_byte_over_limit("t".repeat(30), "c".repeat(30), 63)]
#[case::unicode_boundary("表".repeat(16), "参照先".repeat(3), 62)]
fn bounds_foreign_key_names_at_utf8_boundaries(
	foreign_key: ForeignKeyInfo,
	#[case] table: String,
	#[case] column: String,
	#[case] expected_bytes: usize,
) {
	// Arrange
	let mut model = ModelState::new("registry", "ResourceRef");
	model.table_name = table;
	model.add_field(FieldState::with_foreign_key(
		&column,
		FieldType::Integer,
		false,
		foreign_key,
	));

	// Act
	model.add_foreign_key_constraint_from_field(&column);

	// Assert
	assert_eq!(model.constraints.len(), 1);
	assert_eq!(model.constraints[0].name.len(), expected_bytes);
}

#[rstest]
fn generated_foreign_keys_remain_distinct_after_postgres_truncation(
	long_foreign_keys: ProjectState,
) {
	// Arrange
	let detector = MigrationAutodetector::new(ProjectState::new(), long_foreign_keys);

	// Act
	let migrations = detector.try_generate_migrations().unwrap();

	// Assert
	let mut names: Vec<_> = migrations
		.iter()
		.flat_map(|migration| &migration.operations)
		.filter_map(|operation| match operation {
			Operation::CreateTable {
				name, constraints, ..
			} if name == TABLE => Some(constraints),
			_ => None,
		})
		.flatten()
		.filter_map(|constraint| match constraint {
			Constraint::ForeignKey { name, .. } => Some(name.clone()),
			_ => None,
		})
		.collect();
	names.sort();
	assert_eq!(
		names,
		vec![
			"fk_registry_agent_resource_refs_related_regist_74dadaa1f5257db3",
			"fk_registry_agent_resource_refs_related_regist_9d7b7fa9ada59fef",
		]
	);
}

#[rstest]
fn generated_foreign_key_names_survive_migration_replay(long_foreign_keys: ProjectState) {
	// Arrange
	let migrations = MigrationAutodetector::new(ProjectState::new(), long_foreign_keys.clone())
		.try_generate_migrations()
		.unwrap();
	let replayed = ProjectState::from_migrations(&migrations);

	// Act
	let repeated = MigrationAutodetector::new(replayed, long_foreign_keys)
		.try_generate_migrations()
		.unwrap();

	// Assert
	assert_eq!(repeated.len(), 0);
}

#[rstest]
#[case::operations(false)]
#[case::migrations(true)]
fn generated_many_to_many_foreign_key_names_are_bounded(#[case] migrations: bool) {
	// Arrange
	let table = "registry_agent_resources_with_long_names";
	let mut resource = ModelState::new("registry", "Resource");
	resource.table_name = table.to_owned();
	resource.add_field(FieldState::new("id", FieldType::Integer, false));
	// The two generation entry points consume different M2M representations.
	if migrations {
		resource
			.many_to_many_fields
			.push(ManyToManyMetadata::new("refs", "Resource"));
	} else {
		resource.add_field(FieldState::new(
			"refs",
			FieldType::ManyToMany {
				to: "registry.Resource".to_owned(),
				through: None,
			},
			false,
		));
	}
	let mut state = ProjectState::new();
	state.add_model(resource);
	let detector = MigrationAutodetector::new(ProjectState::new(), state);

	// Act
	let operations = if migrations {
		detector
			.try_generate_migrations()
			.unwrap()
			.into_iter()
			.flat_map(|migration| migration.operations)
			.collect()
	} else {
		detector.generate_operations()
	};

	// Assert
	let names: Vec<_> = operations
		.iter()
		.filter_map(|operation| match operation {
			Operation::CreateTable {
				name, constraints, ..
			} if name == &format!("{table}_refs") => Some(constraints),
			_ => None,
		})
		.flatten()
		.filter_map(|constraint| match constraint {
			Constraint::ForeignKey { name, .. } => Some(name),
			_ => None,
		})
		.collect();
	assert_eq!(names.len(), 2);
	assert_eq!(
		names.iter().map(|name| name.len()).collect::<Vec<_>>(),
		vec![63, 63]
	);
	assert_ne!(names[0], names[1]);
}
