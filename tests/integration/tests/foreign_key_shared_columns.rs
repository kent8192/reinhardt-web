//! Foreign keys whose key also carries columns shared with the referenced
//! model, as in a tenant-scoped `(tenant, task_id) -> tasks(tenant, id)` key.

// The model macro emits feature cfgs not declared by this standalone integration-test crate.
#![allow(unexpected_cfgs)]

use std::sync::Arc;

use reinhardt_core::macros::model;
use reinhardt_db::associations::ForeignKeyField;
use reinhardt_db::backends::DatabaseConnection;
use reinhardt_db::migrations::executor::DatabaseMigrationExecutor;
use reinhardt_db::migrations::{
	Constraint, ConstraintDefinition, ForeignKeyAction, ForeignKeyConstraintInfo,
	MigrationAutodetector, Operation, ProjectState,
};
use reinhardt_db::orm::Model;
use reinhardt_query::prelude::{Alias, PostgresQueryBuilder, Query, QueryStatementBuilder, Value};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use testcontainers::{ContainerAsync, GenericImage};

const APP: &str = "shared_fk_test";
const CONSTRAINT: &str = "shared_fk_artifacts_task_fkey";

#[model(
	app_label = "shared_fk_test",
	table_name = "shared_fk_tasks",
	constraints = [unique(fields = ["tenant", "id"], name = "shared_fk_tasks_tenant_id_key")]
)]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct SharedTask {
	#[field(primary_key = true)]
	id: i64,
	#[field(max_length = 64, db_column = "tenant_name")]
	tenant: String,
}

#[model(app_label = "shared_fk_test", table_name = "shared_fk_artifacts")]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct SharedArtifact {
	#[field(primary_key = true)]
	id: i64,
	#[field(max_length = 64, db_column = "owner_tenant")]
	tenant: String,
	#[rel(
		foreign_key,
		shared_fields = ["tenant"],
		db_constraint = "shared_fk_artifacts_task_fkey",
		db_index = false,
		on_delete = Cascade,
		on_update = Restrict
	)]
	task: ForeignKeyField<SharedTask>,
}

fn expected_constraint() -> ConstraintDefinition {
	ConstraintDefinition {
		name: CONSTRAINT.to_string(),
		constraint_type: "foreign_key".to_string(),
		fields: vec!["owner_tenant".to_string(), "task_id".to_string()],
		expression: None,
		foreign_key_info: Some(ForeignKeyConstraintInfo {
			referenced_table: "shared_fk_tasks".to_string(),
			referenced_columns: vec!["tenant_name".to_string(), "id".to_string()],
			on_delete: ForeignKeyAction::Cascade,
			on_update: ForeignKeyAction::Restrict,
		}),
	}
}

#[fixture]
fn model_state() -> ProjectState {
	let registry = ProjectState::from_global_registry();
	let mut state = ProjectState::new();
	for name in ["SharedTask", "SharedArtifact"] {
		state.add_model(registry.get_model(APP, name).unwrap().clone());
	}
	state
}

#[rstest]
fn shared_columns_lead_the_foreign_key_constraint(model_state: ProjectState) {
	// Arrange
	let artifact = model_state.get_model(APP, "SharedArtifact").unwrap();

	// Act
	let foreign_keys = artifact
		.constraints
		.iter()
		.filter(|constraint| constraint.constraint_type == "foreign_key")
		.cloned()
		.collect::<Vec<_>>();

	// Assert
	assert_eq!(foreign_keys, vec![expected_constraint()]);
	assert!(artifact.indexes.is_empty());
	assert_eq!(
		SharedArtifact::constraint_fields(CONSTRAINT),
		Some(vec!["tenant", "task_id"])
	);
}

#[rstest]
fn a_typed_constraint_recorded_in_migration_state_satisfies_the_model(model_state: ProjectState) {
	// Arrange
	let mut recorded = model_state.clone();
	recorded
		.get_model_mut(APP, "SharedArtifact")
		.unwrap()
		.constraints
		.retain(|constraint| constraint.name != CONSTRAINT);
	let operation = Operation::AddConstraintDefinition {
		table: "shared_fk_artifacts".to_string(),
		constraint: Constraint::ForeignKey {
			name: CONSTRAINT.to_string(),
			columns: vec!["owner_tenant".to_string(), "task_id".to_string()],
			referenced_table: "shared_fk_tasks".to_string(),
			referenced_columns: vec!["tenant_name".to_string(), "id".to_string()],
			on_delete: ForeignKeyAction::Cascade,
			on_update: ForeignKeyAction::Restrict,
			deferrable: None,
		},
	};

	// Act
	operation.state_forwards(APP, &mut recorded);
	let changes = MigrationAutodetector::new(recorded, model_state).detect_changes();

	// Assert
	assert_eq!(changes.added_constraints, Vec::new());
	assert_eq!(changes.removed_constraints, Vec::new());
}

#[rstest]
#[tokio::test]
async fn generated_shared_column_foreign_key_enforces_the_whole_key(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	model_state: ProjectState,
) {
	// Arrange
	let (_container, pool, _port, url) = postgres_container.await;
	let connection = DatabaseConnection::connect_postgres(&url).await.unwrap();
	let mut executor = DatabaseMigrationExecutor::new(connection);
	let migrations = MigrationAutodetector::new(ProjectState::new(), model_state)
		.try_generate_migrations()
		.unwrap();
	let applied = executor.apply_migrations(&migrations).await.unwrap();
	assert_eq!(applied.failed, None);
	let insert_task = Query::insert()
		.into_table(Alias::new("shared_fk_tasks"))
		.columns([Alias::new("id"), Alias::new("tenant_name")])
		.values_panic([Value::from(1_i64), Value::from("acme")])
		.to_string(PostgresQueryBuilder);
	sqlx::query(&insert_task)
		.execute(pool.as_ref())
		.await
		.unwrap();
	let insert_artifact = |id: i64, tenant: &str| {
		Query::insert()
			.into_table(Alias::new("shared_fk_artifacts"))
			.columns([
				Alias::new("id"),
				Alias::new("owner_tenant"),
				Alias::new("task_id"),
			])
			.values_panic([Value::from(id), Value::from(tenant), Value::from(1_i64)])
			.to_string(PostgresQueryBuilder)
	};

	// Act
	let same_tenant = sqlx::query(&insert_artifact(1, "acme"))
		.execute(pool.as_ref())
		.await;
	let other_tenant = sqlx::query(&insert_artifact(2, "globex"))
		.execute(pool.as_ref())
		.await;

	// Assert
	assert_eq!(same_tenant.unwrap().rows_affected(), 1);
	let error = other_tenant.unwrap_err();
	let database_error = error.as_database_error().unwrap();
	assert_eq!(database_error.code().as_deref(), Some("23503"));
	assert_eq!(database_error.constraint(), Some(CONSTRAINT));

	let rollback = executor.rollback_migrations(&migrations).await.unwrap();
	assert_eq!(rollback.failed, None);
}
