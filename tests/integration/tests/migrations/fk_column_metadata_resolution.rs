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
//! shared state, so they use uniquely-named apps and models to avoid
//! cross-test interference under parallel execution.
//!
//! Fixtures Used: none (pure registry + `ColumnDefinition` manipulation).

use reinhardt_db::migrations::model_registry::{FieldMetadata, ModelMetadata, global_registry};
use reinhardt_db::migrations::{ColumnDefinition, FieldState, FieldType};
use rstest::rstest;

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
		.insert("null".to_string(), nullable.to_string());
	field_state
		.params
		.insert("not_null".to_string(), (!nullable).to_string());
	field_state
		.params
		.insert("db_index".to_string(), "true".to_string());
	field_state
}

#[rstest]
#[case::big_integer_pk(
	"fk_meta_bigint_app",
	"FkMetaBigIntTarget",
	"fk_meta_bigint_target",
	FieldType::BigInteger
)]
#[case::integer_pk(
	"fk_meta_int_app",
	"FkMetaIntTarget",
	"fk_meta_int_target",
	FieldType::Integer
)]
fn fk_column_type_resolves_from_target_model_pk(
	#[case] app: &str,
	#[case] model: &str,
	#[case] table: &str,
	#[case] pk_type: FieldType,
) {
	// Arrange — register the target model with the requested PK type and
	// build an FK `_id` `FieldState` that mirrors what the `#[model]`
	// macro emits (placeholder `FieldType::Uuid`, `fk_target` param).
	register_target_model(app, model, table, pk_type.clone());
	let fk_field = fk_id_field_state("target_id", model, /* nullable */ false);

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
	register_target_model(
		"fk_meta_uuid_app",
		"FkMetaUuidTarget",
		"fk_meta_uuid_target",
		FieldType::Uuid,
	);
	let fk_field = fk_id_field_state("target_id", "FkMetaUuidTarget", /* nullable */ false);

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
#[case::non_optional_is_not_null(false, true)]
#[case::optional_is_nullable(true, false)]
fn fk_column_not_null_reflects_macro_emitted_param(
	#[case] nullable: bool,
	#[case] expected_not_null: bool,
) {
	// Arrange — register a target with BigInteger PK and construct an
	// FK field state whose `not_null` param mirrors the macro contract:
	// non-`Option` ForeignKeyField -> `not_null = "true"`,
	// `Option<ForeignKeyField>` -> `not_null = "false"`.
	let app = if nullable {
		"fk_meta_nullable_app"
	} else {
		"fk_meta_not_null_app"
	};
	let model = if nullable {
		"FkMetaNullableTarget"
	} else {
		"FkMetaNotNullTarget"
	};
	let table = if nullable {
		"fk_meta_nullable_target"
	} else {
		"fk_meta_not_null_target"
	};
	register_target_model(app, model, table, FieldType::BigInteger);
	let fk_field = fk_id_field_state("target_id", model, nullable);

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
	let fk_field = fk_id_field_state(
		"orphan_id",
		"UnregisteredFkMetaTargetUniqueName",
		/* nullable */ false,
	);

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
