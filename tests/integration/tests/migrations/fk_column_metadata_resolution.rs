//! Integration tests for foreign-key column metadata resolution.
//!
//! Covers regressions for issues #4430 and #4431.
//!
//! - #4430: `ColumnDefinition::from_field_state` must resolve the column
//!   `type_definition` of a `ForeignKeyField<T>` `_id` column from the
//!   target model's primary key registered in the global `ModelRegistry`
//!   (the `#[model]` macro emits a `FieldType::Uuid` placeholder).
//! - #4431: The same path must honor the `not_null` parameter emitted by
//!   the macro for non-`Option` foreign-key fields.
//!
//! These tests interact with `global_registry()`, which is process-wide
//! shared state with no public clear hook. To stay safe under
//! parallel execution and against future tests reusing the same model
//! names, every test allocates uniquely-suffixed app / model / table
//! identifiers via `fresh_ids()` (UUID v4). See #4434 review (HYA).
//!
//! Fixtures Used: none (pure registry + `ColumnDefinition` manipulation).

use reinhardt_db::migrations::model_registry::{FieldMetadata, ModelMetadata, global_registry};
use reinhardt_db::migrations::{ColumnDefinition, FieldState, FieldType};
use rstest::rstest;
use uuid::Uuid;

/// Generate a triple of unique `(app, model, table)` identifiers for a
/// single test, so registrations into the process-wide `global_registry()`
/// can never collide across tests or repeat runs.
fn fresh_ids(base: &str) -> (String, String, String) {
	let suffix = Uuid::new_v4().simple().to_string();
	(
		format!("{base}_app_{suffix}"),
		format!("FkMetaTarget_{base}_{suffix}"),
		format!("{base}_target_{suffix}"),
	)
}

/// Register a minimal target model with the given primary-key
/// `FieldType` into the global registry.
fn register_target_model(app: &str, model: &str, table: &str, pk_type: FieldType) {
	let mut metadata = ModelMetadata::new(app, model, table);
	metadata.add_field(
		"id".to_string(),
		FieldMetadata::new(pk_type)
			.with_param("primary_key", "true")
			.with_param("not_null", "true")
			.with_param("null", "false")
			.with_param("auto_increment", "true"),
	);
	global_registry().register_model(metadata);
}

/// Build a `FieldState` shaped exactly like the `_id` column emitted by
/// the `#[model]` macro for a `ForeignKeyField<T>`.
fn fk_id_field_state(column_name: &str, fk_target_model: &str, nullable: bool) -> FieldState {
	let mut field_state = FieldState::new(column_name, FieldType::Uuid, nullable);
	field_state
		.params
		.insert("fk_target".to_string(), fk_target_model.to_string());
	field_state
		.params
		.insert("not_null".to_string(), (!nullable).to_string());
	field_state
		.params
		.insert("db_index".to_string(), "true".to_string());
	field_state
}

#[rstest]
#[case::big_integer_pk("bigint", FieldType::BigInteger)]
#[case::integer_pk("int", FieldType::Integer)]
fn fk_column_type_resolves_from_target_model_pk(#[case] base: &str, #[case] pk_type: FieldType) {
	// Arrange — register the target model with the requested PK type and
	// build an FK `_id` `FieldState` that mirrors what the `#[model]`
	// macro emits (placeholder `FieldType::Uuid`, `fk_target` param).
	let (app, model, table) = fresh_ids(base);
	register_target_model(&app, &model, &table, pk_type.clone());
	let fk_field = fk_id_field_state("target_id", &model, /* nullable */ false);

	// Act
	let column = ColumnDefinition::from_field_state("target_id".to_string(), &fk_field);

	// Assert — the resolved column type matches the target's PK type, not
	// the macro-emitted `FieldType::Uuid` placeholder.
	assert_eq!(
		column.type_definition, pk_type,
		"FK column type must be resolved from the target model's PK in \
		 the global registry, not from the macro-emitted placeholder \
		 (issue #4430)."
	);
}

#[rstest]
fn fk_column_type_preserves_uuid_when_target_pk_is_uuid() {
	// Arrange — Uuid PK target. Even though the macro placeholder is
	// also `Uuid`, the resolution path must not regress for Uuid PKs.
	let (app, model, table) = fresh_ids("uuid");
	register_target_model(&app, &model, &table, FieldType::Uuid);
	let fk_field = fk_id_field_state("target_id", &model, /* nullable */ false);

	// Act
	let column = ColumnDefinition::from_field_state("target_id".to_string(), &fk_field);

	// Assert
	assert_eq!(
		column.type_definition,
		FieldType::Uuid,
		"FK column type must remain Uuid when the target's PK is Uuid."
	);
}

#[rstest]
#[case::non_optional_is_not_null("not_null", false, true)]
#[case::optional_is_nullable("nullable", true, false)]
fn fk_column_not_null_reflects_macro_emitted_param(
	#[case] base: &str,
	#[case] nullable: bool,
	#[case] expected_not_null: bool,
) {
	// Arrange — register a target with BigInteger PK and construct an
	// FK field state whose `not_null` param mirrors the macro contract:
	// non-`Option` ForeignKeyField -> `not_null = "true"`,
	// `Option<ForeignKeyField>` -> `not_null = "false"`.
	let (app, model, table) = fresh_ids(base);
	register_target_model(&app, &model, &table, FieldType::BigInteger);
	let fk_field = fk_id_field_state("target_id", &model, nullable);

	// Act
	let column = ColumnDefinition::from_field_state("target_id".to_string(), &fk_field);

	// Assert
	assert_eq!(
		column.not_null, expected_not_null,
		"FK column `not_null` must reflect the macro-emitted `not_null` \
		 param (issue #4431). nullable={nullable}",
	);
}

#[rstest]
fn fk_column_falls_back_to_placeholder_when_target_unregistered() {
	// Arrange — build an FK `_id` `FieldState` whose target is not
	// registered. The resolver must not panic; it should leave the
	// macro-emitted placeholder type in place so the downstream caller
	// can surface a clearer error.
	let orphan_model = format!("UnregisteredFkMetaTarget_{}", Uuid::new_v4().simple());
	let fk_field = fk_id_field_state("orphan_id", &orphan_model, /* nullable */ false);

	// Act
	let column = ColumnDefinition::from_field_state("orphan_id".to_string(), &fk_field);

	// Assert
	assert_eq!(
		column.type_definition,
		FieldType::Uuid,
		"Unregistered FK targets must fall back to the placeholder type \
		 rather than panic."
	);
	assert!(
		column.not_null,
		"`not_null` must still come from the field state's params even \
		 when the FK target is unregistered."
	);
}
