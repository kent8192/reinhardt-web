//! Proc-macro inspection and migration registration preserve physical identity metadata.
// The model macro emits target/backend cfgs belonging to its consuming crate.
#![allow(unexpected_cfgs)]
use reinhardt_core::macros::model;
use reinhardt_db::{
	migrations::{
		ColumnDefinition, MigrationAutodetector, Operation, ProjectState, SqlDialect,
		model_registry::global_registry,
	},
	orm::Model,
};
use rstest::rstest;
use serde::{Deserialize, Serialize};

// This model is registered for metadata assertions rather than instantiated.
#[allow(dead_code)]
#[model(
	app_label = "sequence_metadata",
	table_name = "sequence_metadata_events"
)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct SequenceMetadataEvent {
	#[field(primary_key = true)]
	id: i64,
	#[field(db_column = "event_no", identity_always = true, identity_options(sequence_name = "event_numbers", start = -10, increment = -2, cache = 5, no_max_value = true))]
	sequence: i64,
}

#[rstest]
fn non_primary_identity_retains_options_and_db_column_in_registered_state() {
	// Arrange
	let inspection = SequenceMetadataEvent::field_metadata()
		.into_iter()
		.find(|field| field.name == "sequence")
		.unwrap();
	let registration = global_registry()
		.get_model("sequence_metadata", "SequenceMetadataEvent")
		.unwrap();
	// Act
	let model = registration.to_model_state();
	let field = model.fields.get("event_no").unwrap();
	let column = ColumnDefinition::from_field_state("event_no", field);
	let mut state = ProjectState::new();
	state.add_model(model);
	let operations = MigrationAutodetector::new(ProjectState::new(), state)
		.try_generate_operations()
		.unwrap();
	let sql = operations
		.iter()
		.find(|operation| matches!(operation, Operation::CreateTable { .. }))
		.unwrap()
		.try_to_sql(&SqlDialect::Postgres)
		.unwrap();
	// Assert
	assert_eq!(inspection.db_column.as_deref(), Some("event_no"));
	assert!(!column.primary_key);
	let identity = column.identity.unwrap();
	assert_eq!(identity.sequence_name.unwrap().name, "event_numbers");
	assert_eq!(identity.options.start, Some(-10));
	assert_eq!(identity.options.increment, Some(-2));
	assert_eq!(identity.options.cache, Some(5));
	assert!(
		sql.contains("event_no BIGINT GENERATED ALWAYS AS IDENTITY"),
		"{sql}"
	);
	assert!(!sql.contains("\"sequence\""));
}
