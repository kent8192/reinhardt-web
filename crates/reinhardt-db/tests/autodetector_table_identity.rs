//! Regression coverage for unchanged equal-shaped tables (issue #6472).

#![cfg(feature = "migrations")]

use reinhardt_db::migrations::model_registry::{FieldMetadata, ModelMetadata};
use reinhardt_db::migrations::{
	ColumnDefinition, FieldType, Migration, MigrationAutodetector, Operation, ProjectState,
};
use rstest::*;

const APP: &str = "marketplace";
const MODELS: [(&str, &str); 4] = [
	("DistributionAudience", "marketplace_audiences"),
	("RedistributionConsent", "marketplace_consents"),
	("CompatibilityGate", "marketplace_gate"),
	("IdempotentRequest", "marketplace_requests"),
];

#[fixture]
fn equal_shaped_tables() -> ProjectState {
	let mut state = ProjectState::new();
	for (model_name, table_name) in MODELS {
		let mut metadata = ModelMetadata::new(APP, model_name, table_name);
		metadata.add_field(
			"key".to_owned(),
			FieldMetadata::new(FieldType::Text).with_param("primary_key", "true"),
		);
		metadata.add_field("document".to_owned(), FieldMetadata::new(FieldType::Json));
		state.add_model(metadata.to_model_state());
	}
	state
}

fn replay_initial(state: &ProjectState) -> ProjectState {
	let operations = MigrationAutodetector::new(ProjectState::new(), state.clone())
		.try_generate_operations()
		.expect("initial migration generation must succeed");
	let mut tables = operations
		.iter()
		.map(|operation| match operation {
			Operation::CreateTable { name, .. } => name.as_str(),
			other => panic!("initial migration must contain only CreateTable: {other:?}"),
		})
		.collect::<Vec<_>>();
	tables.sort_unstable();
	assert_eq!(
		tables,
		MODELS.map(|(_, table)| table),
		"initial migration must create all four distinct tables"
	);
	let migration = operations.into_iter().fold(
		Migration::new("0001_initial", APP),
		|migration, operation| migration.add_operation(operation),
	);
	ProjectState::from_migrations(&[migration])
}

fn migration_operations(detector: &MigrationAutodetector) -> Vec<Operation> {
	let mut migrations = detector
		.try_generate_migrations()
		.expect("migration generation must succeed");
	assert_eq!(migrations.len(), 1, "unexpected migrations: {migrations:?}");
	let migration = migrations.pop().expect("one migration must exist");
	assert_eq!(migration.app_label, APP);
	migration.operations
}

#[rstest]
fn unchanged_tables_have_no_model_renames(equal_shaped_tables: ProjectState) {
	// Arrange
	let previous = replay_initial(&equal_shaped_tables);
	let detector = MigrationAutodetector::new(previous, equal_shaped_tables);

	// Act
	let changes = detector
		.try_detect_changes()
		.expect("unchanged model detection must succeed");

	// Assert
	assert_eq!(
		changes.renamed_models,
		Vec::<(String, String, String)>::new()
	);
	assert!(changes.moved_models.is_empty());
	assert!(changes.created_models.is_empty());
	assert!(changes.deleted_models.is_empty());
}

#[rstest]
fn unchanged_tables_round_trip_to_empty_diff(equal_shaped_tables: ProjectState) {
	// Arrange
	let previous = replay_initial(&equal_shaped_tables);
	assert_eq!(
		previous
			.models
			.keys()
			.map(|(_, model)| model.as_str())
			.collect::<Vec<_>>(),
		vec!["Audiences", "Consents", "Gate", "Requests"]
	);
	let detector = MigrationAutodetector::new(previous, equal_shaped_tables);

	// Act
	let operations = detector
		.try_generate_operations()
		.expect("unchanged model detection must succeed");
	let migrations = detector
		.try_generate_migrations()
		.expect("unchanged models must not generate migrations");

	// Assert
	assert_eq!(operations, Vec::<Operation>::new());
	assert_eq!(migrations.len(), 0, "unexpected migrations: {migrations:?}");
}

#[rstest]
fn unchanged_tables_in_multiple_apps_keep_their_owners(mut equal_shaped_tables: ProjectState) {
	// Arrange
	let mut model = equal_shaped_tables
		.remove_model(APP, "IdempotentRequest")
		.expect("request model must exist");
	model.app_label = "auditing".to_owned();
	equal_shaped_tables.add_model(model);
	let initial = MigrationAutodetector::new(ProjectState::new(), equal_shaped_tables.clone())
		.try_generate_migrations()
		.expect("initial migration generation must succeed");
	let previous = ProjectState::from_migrations(&initial);
	let detector = MigrationAutodetector::new(previous, equal_shaped_tables);

	// Act
	let changes = detector
		.try_detect_changes()
		.expect("unchanged model detection must succeed");
	let operations = detector
		.try_generate_operations()
		.expect("unchanged model detection must succeed");

	// Assert
	assert!(changes.moved_models.is_empty());
	assert!(changes.renamed_models.is_empty());
	assert!(changes.created_models.is_empty());
	assert!(changes.deleted_models.is_empty());
	assert_eq!(operations, Vec::<Operation>::new());
}

#[rstest]
#[case::audience("DistributionAudience", "marketplace_audiences")]
#[case::consent("RedistributionConsent", "marketplace_consents")]
#[case::gate("CompatibilityGate", "marketplace_gate")]
#[case::request("IdempotentRequest", "marketplace_requests")]
fn field_change_targets_its_physical_table(
	mut equal_shaped_tables: ProjectState,
	#[case] model_name: &str,
	#[case] table_name: &str,
) {
	// Arrange
	let previous = replay_initial(&equal_shaped_tables);
	equal_shaped_tables
		.models
		.get_mut(&(APP.to_owned(), model_name.to_owned()))
		.expect("model must exist")
		.fields
		.get_mut("document")
		.expect("document field must exist")
		.nullable = true;
	let detector = MigrationAutodetector::new(previous, equal_shaped_tables);

	// Act
	let operations = detector
		.try_generate_operations()
		.expect("field change detection must succeed");
	let migration_operations = migration_operations(&detector);

	// Assert
	assert_eq!(operations.len(), 1, "unexpected operations: {operations:?}");
	let Operation::AlterColumn {
		table,
		column,
		new_definition,
		..
	} = &operations[0]
	else {
		panic!("expected only AlterColumn, got {operations:?}");
	};
	assert_eq!(table, table_name);
	assert_eq!(column, "document");
	assert_eq!(
		new_definition,
		&ColumnDefinition::new("document", FieldType::Json)
	);
	assert_eq!(migration_operations, operations);
}

#[rstest]
#[case::audience("DistributionAudience", "marketplace_audiences")]
#[case::consent("RedistributionConsent", "marketplace_consents")]
#[case::gate("CompatibilityGate", "marketplace_gate")]
#[case::request("IdempotentRequest", "marketplace_requests")]
fn actual_table_rename_preserves_unchanged_peers(
	mut equal_shaped_tables: ProjectState,
	#[case] model_name: &str,
	#[case] table_name: &str,
) {
	// Arrange
	let previous = replay_initial(&equal_shaped_tables);
	let new_table = format!("{table_name}_renamed");
	equal_shaped_tables
		.models
		.get_mut(&(APP.to_owned(), model_name.to_owned()))
		.expect("model must exist")
		.table_name = new_table.clone();
	let detector = MigrationAutodetector::new(previous, equal_shaped_tables);

	// Act
	let operations = migration_operations(&detector);

	// Assert
	assert_eq!(
		operations,
		vec![Operation::RenameTable {
			old_name: table_name.to_owned(),
			new_name: new_table,
		}]
	);
}

#[rstest]
#[case::audience("DistributionAudience", "marketplace_audiences")]
#[case::consent("RedistributionConsent", "marketplace_consents")]
#[case::gate("CompatibilityGate", "marketplace_gate")]
#[case::request("IdempotentRequest", "marketplace_requests")]
fn removed_table_drops_only_its_physical_table(
	mut equal_shaped_tables: ProjectState,
	#[case] model_name: &str,
	#[case] table_name: &str,
) {
	// Arrange
	let previous = replay_initial(&equal_shaped_tables);
	equal_shaped_tables
		.remove_model(APP, model_name)
		.expect("model to remove must exist");
	let detector = MigrationAutodetector::new(previous, equal_shaped_tables);

	// Act
	let operations = detector
		.try_generate_operations()
		.expect("table deletion detection must succeed");
	let migration_operations = migration_operations(&detector);

	// Assert
	assert_eq!(
		operations,
		vec![Operation::DropTable {
			name: table_name.to_owned(),
		}]
	);
	assert_eq!(migration_operations, operations);
}

#[rstest]
fn cross_app_move_preserves_unchanged_peers(mut equal_shaped_tables: ProjectState) {
	// Arrange
	let previous = replay_initial(&equal_shaped_tables);
	let mut moved = equal_shaped_tables
		.models
		.remove(&(APP.to_owned(), "IdempotentRequest".to_owned()))
		.expect("request model must exist");
	moved.app_label = "auditing".to_owned();
	equal_shaped_tables.add_model(moved);
	let detector = MigrationAutodetector::new(previous, equal_shaped_tables);

	// Act
	let changes = detector
		.try_detect_changes()
		.expect("cross-app move detection must succeed");

	// Assert
	assert_eq!(
		changes.moved_models,
		vec![(
			APP.to_owned(),
			"Requests".to_owned(),
			"auditing".to_owned(),
			"IdempotentRequest".to_owned(),
			false,
			None,
			None,
		)]
	);
	assert!(changes.renamed_models.is_empty());
	assert!(changes.created_models.is_empty());
	assert!(changes.deleted_models.is_empty());
}
