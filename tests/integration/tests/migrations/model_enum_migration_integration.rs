//! Integration tests for native model-enum migration metadata.

use reinhardt_db::field_domain::{FieldDomain, ModelEnumRepr, ModelEnumValue};
use reinhardt_db::migrations::{
	ColumnDefinition, FieldMetadata, FieldType, MigrationAutodetector, ModelMetadata, Operation,
	ProjectState,
};
use reinhardt_db::orm::DatabaseField;
use reinhardt_macros::{ModelEnum, model};
use serde::{Deserialize, Serialize};

#[derive(ModelEnum, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[model_enum(repr = "string")]
enum MigrationStatus {
	#[model_enum(value = "queued")]
	Queued,
	#[model_enum(value = "running")]
	Running,
}

#[derive(ModelEnum, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[model_enum(repr = "string")]
enum ReorderedMigrationStatus {
	#[model_enum(value = "running")]
	Running,
	#[model_enum(value = "queued")]
	Queued,
}

#[model(
	app_label = "model_enum_migrations",
	table_name = "model_enum_migration_jobs"
)]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct MigrationJob {
	#[field(primary_key = true)]
	id: Option<i64>,
	#[field(db_column = "job_status", max_length = 32)]
	status: MigrationStatus,
}

#[model(
	app_label = "model_enum_migrations",
	table_name = "model_enum_migration_jobs_reordered"
)]
#[derive(Clone, Debug, Serialize, Deserialize)]
// This fixture is registered through its generated constructor and inspected via migration metadata.
#[allow(dead_code)]
struct ReorderedMigrationJob {
	#[field(primary_key = true)]
	id: Option<i64>,
	#[field(db_column = "job_status", max_length = 32)]
	status: ReorderedMigrationStatus,
}

#[model(
	app_label = "model_enum_migrations",
	table_name = "model_enum_optional_jobs"
)]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct OptionalMigrationJob {
	#[field(primary_key = true)]
	id: Option<i64>,
	#[field(db_column = "optional_status", max_length = 32)]
	status: Option<MigrationStatus>,
}

fn string_domain(values: &[&str]) -> FieldDomain {
	FieldDomain::Enum {
		repr: ModelEnumRepr::String,
		values: values
			.iter()
			.map(|value| ModelEnumValue::String((*value).to_string()))
			.collect(),
	}
}

fn project_state_with_domain(values: &[&str]) -> ProjectState {
	let mut metadata = ModelMetadata::new("model_enum_state", "Job", "model_enum_state_jobs");
	metadata.add_field(
		"status".to_string(),
		FieldMetadata::new(FieldType::VarChar(32))
			.with_param("db_column", "job_status")
			.with_domain(string_domain(values)),
	);

	let mut state = ProjectState::new();
	state.add_model(metadata.to_model_state());
	state
}

#[test]
fn enum_domain_order_is_canonical_for_autodetection() {
	let from_state = project_state_with_domain(&["queued", "running"]);
	let to_state = project_state_with_domain(&["running", "queued"]);

	let operations = MigrationAutodetector::new(from_state, to_state).generate_operations();

	assert_eq!(operations, Vec::<Operation>::new());
}

#[test]
fn macro_registered_variant_reorder_is_a_migration_noop() {
	let registry = reinhardt_db::migrations::global_registry();
	let from_model = registry
		.get_model("model_enum_migrations", "MigrationJob")
		.expect("MigrationJob should be registered")
		.to_model_state();
	let mut to_model = registry
		.get_model("model_enum_migrations", "ReorderedMigrationJob")
		.expect("ReorderedMigrationJob should be registered")
		.to_model_state();

	to_model.name = from_model.name.clone();
	to_model.table_name = from_model.table_name.clone();
	let constraint_name = from_model
		.constraints
		.iter()
		.find(|constraint| constraint.constraint_type == "enum_domain")
		.expect("source enum-domain constraint should exist")
		.name
		.clone();
	to_model
		.constraints
		.iter_mut()
		.find(|constraint| constraint.constraint_type == "enum_domain")
		.expect("reordered enum-domain constraint should exist")
		.name = constraint_name;

	let mut from_state = ProjectState::new();
	from_state.add_model(from_model);
	let mut to_state = ProjectState::new();
	to_state.add_model(to_model);

	let operations = MigrationAutodetector::new(from_state, to_state).generate_operations();

	assert_eq!(operations, Vec::<Operation>::new());
}

#[test]
fn enum_domain_value_replacement_recreates_the_constraint() {
	let from_state = project_state_with_domain(&["queued", "running"]);
	let to_state = project_state_with_domain(&["queued", "executing"]);

	let operations = MigrationAutodetector::new(from_state, to_state).generate_operations();

	assert_eq!(operations.len(), 2, "operations = {operations:?}");
	assert!(matches!(
		&operations[0],
		Operation::DropConstraint {
			table,
			constraint_name,
		} if table == "model_enum_state_jobs"
			&& constraint_name == "model_enum_state_jobs_job_status_model_enum_check"
	));
	assert!(
		matches!(
			&operations[1],
			Operation::AddConstraintDefinition {
				table,
				constraint: reinhardt_db::migrations::Constraint::EnumDomain {
					name,
					column,
					domain,
				},
			} if table == "model_enum_state_jobs"
				&& name == "model_enum_state_jobs_job_status_model_enum_check"
				&& column == "job_status"
				&& domain == &string_domain(&["executing", "queued"])
		),
		"operations = {operations:?}"
	);
}

#[test]
fn model_macro_registers_domain_for_the_resolved_database_column() {
	let _job = MigrationJob {
		id: None,
		status: MigrationStatus::Queued,
	};
	let metadata = reinhardt_db::migrations::global_registry()
		.get_model("model_enum_migrations", "MigrationJob")
		.expect("MigrationJob should be registered");

	let field = metadata
		.fields
		.get("job_status")
		.expect("resolved database column should be registered");
	assert_eq!(field.domain, <MigrationStatus as DatabaseField>::domain());

	let state = metadata.to_model_state();
	let state_field = state
		.fields
		.get("job_status")
		.expect("resolved database column should reach migration state");
	assert_eq!(
		state_field.domain,
		<MigrationStatus as DatabaseField>::domain()
	);
	assert_eq!(
		state.constraints[0].name,
		"model_enum_migration_jobs_job_status_model_enum_check"
	);
}

#[test]
fn optional_model_enum_registers_the_inner_domain() {
	let _job = OptionalMigrationJob {
		id: None,
		status: None,
	};
	let metadata = reinhardt_db::migrations::global_registry()
		.get_model("model_enum_migrations", "OptionalMigrationJob")
		.expect("OptionalMigrationJob should be registered");
	let field = metadata
		.fields
		.get("optional_status")
		.expect("resolved optional database column should be registered");

	assert_eq!(field.domain, <MigrationStatus as DatabaseField>::domain());
	assert_eq!(
		metadata.to_model_state().constraints[0].name,
		"model_enum_optional_jobs_optional_status_model_enum_check"
	);
}

#[test]
fn column_definition_domain_survives_serialization() {
	let column = ColumnDefinition::new("job_status", FieldType::VarChar(32))
		.with_domain(string_domain(&["queued", "running"]));

	let serialized = serde_json::to_string(&column).expect("column should serialize");
	let restored: ColumnDefinition =
		serde_json::from_str(&serialized).expect("column should deserialize");

	assert_eq!(restored, column);

	let mut legacy = serde_json::to_value(&column).expect("column should serialize as a value");
	legacy
		.as_object_mut()
		.expect("column serialization should be an object")
		.remove("domain");
	let restored: ColumnDefinition =
		serde_json::from_value(legacy).expect("legacy column should deserialize");
	assert_eq!(restored.domain, None);
}
