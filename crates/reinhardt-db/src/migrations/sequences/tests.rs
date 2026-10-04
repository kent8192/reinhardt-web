use super::*;
use crate::migrations::{
	FieldState, FilesystemRepository, Migration, MigrationAutodetector, MigrationRenderOptions,
	ModelState, Operation,
};
use rstest::{fixture, rstest};

#[fixture]
fn declared_state() -> ProjectState {
	let key = SequenceKey::new("runs", "input_sequence");
	let name = QualifiedName::new("run.inputs\"seq");
	let owner = SequenceOwner::new(QualifiedName::new("run_inputs"), "seq");
	let mut state = ProjectState::new();
	state
		.add_sequence(SequenceDefinition::new(key.clone(), name.clone()).with_owned_by(Some(owner)))
		.expect("valid sequence");
	let mut model = ModelState::new("runs", "RunInput");
	model.table_name = "run_inputs".into();
	model.add_field(
		FieldState::new("seq", FieldType::BigInteger, false)
			.with_sequence_default(SequenceDefault::new(key, name)),
	);
	model.add_field(
		FieldState::new("event_no", FieldType::Integer, false).with_identity(
			IdentityDefinition::new(IdentityGeneration::Always)
				.with_sequence_name(QualifiedName::new("events_sequence_seq"))
				.with_options(SequenceOptions::new().with_start(20).with_cache(5)),
		),
	);
	state.add_model(model);
	state
}

#[rstest]
fn declaration_round_trips_json_source_and_replay(declared_state: ProjectState) {
	// Arrange
	let operations = MigrationAutodetector::new(ProjectState::new(), declared_state.clone())
		.try_generate_operations()
		.expect("generate declarations");
	let migration = operations.into_iter().fold(
		Migration::new("0001_initial", "runs"),
		|migration, operation| migration.add_operation(operation),
	);
	// Act
	let json = serde_json::to_string(&migration).expect("serialize");
	let decoded: Migration = serde_json::from_str(&json).expect("deserialize");
	let source = FilesystemRepository::new("/tmp/unused-6506-source")
		.render(
			&migration,
			MigrationRenderOptions {
				include_header: true,
			},
		)
		.expect("render typed source");
	let ast = syn::parse_file(&source).expect("parse generated Rust");
	let loaded = crate::migrations::ast_parser::extract_migration_metadata_strict(
		&ast,
		"runs",
		"0001_initial",
	)
	.expect("load typed source");
	let mut replayed = ProjectState::new();
	for operation in &loaded.operations {
		operation.state_forwards("runs", &mut replayed);
	}
	// Assert
	assert_eq!(decoded.operations, migration.operations);
	assert_eq!(loaded.operations, migration.operations);
	assert!(matches!(
		loaded.operations.first(),
		Some(Operation::Sequence {
			operation: SequenceOperation::Create { .. }
		})
	));
	assert!(matches!(
		loaded.operations.last(),
		Some(Operation::Sequence {
			operation: SequenceOperation::Ownership { .. }
		})
	));
	assert!(
		MigrationAutodetector::new(replayed, declared_state)
			.try_generate_operations()
			.expect("second diff")
			.is_empty()
	);
}

#[rstest]
#[case(SequenceDataType::SmallInteger, 32767, -32768)]
#[case(SequenceDataType::Integer, 2147483647, -2147483648)]
#[case(SequenceDataType::BigInteger, i64::MAX, i64::MIN)]
fn omitted_bounds_resolve_by_width_and_direction(
	#[case] width: SequenceDataType,
	#[case] ascending_max: i64,
	#[case] descending_min: i64,
) {
	// Arrange
	let options = SequenceOptions::new().with_data_type(width);
	// Act
	let ascending = options.effective().expect("ascending options");
	let descending = options
		.with_increment(-1)
		.effective()
		.expect("descending options");
	// Assert
	assert_eq!(
		ascending.max_value,
		Some(SequenceBound::Value(ascending_max))
	);
	assert_eq!(
		descending.min_value,
		Some(SequenceBound::Value(descending_min))
	);
	assert_eq!(descending.start, Some(-1));
}

#[rstest]
fn rename_and_alter_do_not_restart(declared_state: ProjectState) {
	// Arrange
	let mut desired = declared_state.clone();
	let definition = desired.sequences.values_mut().next().expect("sequence");
	definition.name.name = "new_sequence".into();
	definition.options = SequenceOptions::new().with_increment(2).with_cache(10);
	for model in desired.models.values_mut() {
		for field in model.fields.values_mut() {
			if let Some(mut default) =
				sequence_default_from_params(&field.params).expect("metadata")
			{
				default.name.name = "new_sequence".into();
				field.params.insert(
					"sequence_default".into(),
					serde_json::to_string(&default).expect("serialize"),
				);
			}
		}
	}
	// Act
	let operations = MigrationAutodetector::new(declared_state, desired)
		.try_generate_operations()
		.expect("rename and alter");
	let sql: String = operations
		.iter()
		.map(|operation| operation.try_to_sql(&SqlDialect::Postgres).expect("SQL"))
		.collect();
	// Assert
	assert!(sql.contains("RENAME TO \"new_sequence\""));
	assert!(sql.contains("INCREMENT BY 2"));
	assert!(!sql.contains("RESTART"));
}

#[rstest]
fn restart_without_reverse_target_is_irreversible() {
	// Arrange
	let operation = SequenceOperation::Restart {
		name: QualifiedName::new("counter"),
		value: 40,
		reverse_value: None,
	};
	// Act
	let result = operation.reverse();
	// Assert
	assert!(matches!(result, Err(MigrationError::IrreversibleError(_))));
}

#[rstest]
#[case(SqlDialect::Mysql)]
#[case(SqlDialect::Sqlite)]
#[case(SqlDialect::Cockroachdb)]
fn unsupported_backends_reject_new_operations(#[case] dialect: SqlDialect) {
	// Arrange
	let operation = Operation::Sequence {
		operation: SequenceOperation::Create {
			definition: SequenceDefinition::new(
				SequenceKey::new("runs", "counter"),
				QualifiedName::new("counter"),
			),
		},
	};
	// Act
	let result = operation.try_to_sql(&dialect);
	// Assert
	assert!(matches!(
		result,
		Err(MigrationError::UnsupportedBackendFeature { .. })
	));
}

#[rstest]
fn sequence_names_quote_components_independently() {
	// Arrange
	let definition = SequenceDefinition::new(
		SequenceKey::new("runs", "counter"),
		QualifiedName::new("a.b\"c").with_schema("s.ch\"ema"),
	);
	// Act
	let sql = SequenceOperation::Create { definition }.to_sql();
	// Assert
	assert_eq!(sql, "CREATE SEQUENCE \"s.ch\"\"ema\".\"a.b\"\"c\"");
}

#[rstest]
fn owned_deletion_removes_state_without_double_drop(declared_state: ProjectState) {
	// Arrange
	let mut desired = declared_state.clone();
	desired
		.models
		.get_mut(&("runs".into(), "RunInput".into()))
		.unwrap()
		.fields
		.remove("seq");
	desired.sequences.clear();
	// Act
	let operations = MigrationAutodetector::new(declared_state.clone(), desired)
		.try_generate_operations()
		.unwrap();
	let mut replayed = declared_state.clone();
	for operation in &operations {
		operation.state_forwards("runs", &mut replayed);
	}
	let reverse = operations
		.iter()
		.find(
			|operation| matches!(operation, Operation::DropColumn { column, .. } if column == "seq"),
		)
		.unwrap()
		.to_reverse_sql(&SqlDialect::Postgres, &declared_state)
		.unwrap()
		.unwrap();
	// Assert
	assert!(replayed.sequences.is_empty());
	assert!(!operations.iter().any(|operation| matches!(
		operation,
		Operation::Sequence {
			operation: SequenceOperation::Drop { .. }
		}
	)));
	assert!(reverse[0].starts_with("CREATE SEQUENCE"));
	assert!(reverse[1].contains("ADD COLUMN"));
	assert!(reverse[2].contains("OWNED BY"));
}

#[rstest]
fn cross_app_ownership_uses_acyclic_stages(declared_state: ProjectState) {
	// Arrange
	let mut desired = declared_state;
	let definition = desired.sequences.pop_first().unwrap().1;
	let key = SequenceKey::new("sequences", "inputs");
	let default = SequenceDefault::new(key.clone(), definition.name.clone());
	desired.sequences.insert(
		key.clone(),
		SequenceDefinition::new(key, definition.name).with_owned_by(definition.owned_by),
	);
	let model = desired
		.models
		.get_mut(&("runs".into(), "RunInput".into()))
		.unwrap();
	model.fields.get_mut("seq").unwrap().params.insert(
		"sequence_default".into(),
		serde_json::to_string(&default).unwrap(),
	);
	// Act
	let migrations = MigrationAutodetector::new(ProjectState::new(), desired.clone())
		.try_generate_migrations()
		.unwrap();
	let mut graph = crate::migrations::MigrationGraph::new();
	for migration in &migrations {
		graph.add_migration(
			crate::migrations::MigrationKey::new(&migration.app_label, &migration.name),
			migration
				.dependencies
				.iter()
				.map(|(app, name)| crate::migrations::MigrationKey::new(app, name))
				.collect(),
		);
	}
	let order = graph.resolve_execution_order_with_replaces().unwrap();
	let mut state = ProjectState::new();
	for key in &order {
		let migration = migrations
			.iter()
			.find(|migration| migration.app_label == key.app_label && migration.name == key.name)
			.unwrap();
		for operation in &migration.operations {
			operation.state_forwards(&migration.app_label, &mut state);
		}
	}
	// Assert
	assert_eq!(migrations.len(), 3);
	assert_eq!(order[0].app_label, "sequences");
	assert_eq!(order[1].app_label, "runs");
	assert_eq!(order[2].name, "autodetected_ownership");
	state.validate_sequences().unwrap();
	assert!(
		MigrationAutodetector::new(state, desired)
			.try_generate_operations()
			.unwrap()
			.is_empty()
	);
}

#[rstest]
fn explicit_logical_rename_maps_defaults_and_preserves_physical_identity(
	declared_state: ProjectState,
) {
	// Arrange
	let mut state = declared_state;
	let old = state.sequences.first_key_value().unwrap().0.clone();
	let new = SequenceKey::new("runs", "renamed_inputs");
	let physical = state.sequences[&old].name.clone();
	let operation = SequenceOperation::RenameDeclaration {
		old: old.clone(),
		new: new.clone(),
	};
	// Act
	operation.state_forwards(&mut state);
	state.validate_sequences().unwrap();
	operation.reverse().unwrap().state_forwards(&mut state);
	// Assert
	assert_eq!(state.sequences[&old].name, physical);
	assert!(!state.sequences.contains_key(&new));
}

#[rstest]
fn identity_name_collides_with_later_table(declared_state: ProjectState) {
	// Arrange
	let mut state = declared_state;
	let mut model = ModelState::new("z", "Later");
	model.table_name = "events_sequence_seq".into();
	state.add_model(model);
	// Act / Assert
	assert!(state.validate_sequences().is_err());
}

#[rstest]
fn every_lifecycle_operation_round_trips_typed_source_and_json() {
	// Arrange
	let old = SequenceDefinition::new(
		SequenceKey::new("runs", "numbers"),
		QualifiedName::new("numbers").with_schema("tenant.dot"),
	)
	.with_options(
		SequenceOptions::new()
			.with_increment(-2)
			.with_start(-10)
			.with_min_value(SequenceBound::Value(i64::MIN))
			.with_max_value(SequenceBound::Default),
	);
	let new = old
		.clone()
		.with_options(old.options.clone().with_cache(5).with_cycle(true));
	let owner = SequenceOwner::new(QualifiedName::new("events").with_schema("tenant.dot"), "n");
	let mut migration = Migration::new("0002_lifecycle", "runs");
	for operation in [
		SequenceOperation::Create {
			definition: old.clone(),
		},
		SequenceOperation::Alter {
			old: old.clone(),
			new,
		},
		SequenceOperation::Rename {
			key: old.key.clone(),
			old: old.name.clone(),
			new: QualifiedName::new("renamed").with_schema("tenant.dot"),
		},
		SequenceOperation::Ownership {
			key: old.key.clone(),
			name: old.name.clone(),
			old: None,
			new: Some(owner),
		},
		SequenceOperation::Restart {
			name: old.name.clone(),
			value: -10,
			reverse_value: Some(-20),
		},
		SequenceOperation::RenameDeclaration {
			old: old.key.clone(),
			new: SequenceKey::new("runs", "renamed"),
		},
		SequenceOperation::Drop { definition: old },
	] {
		migration = migration.add_operation(Operation::Sequence { operation });
	}
	let identity = IdentityDefinition::new(IdentityGeneration::Always)
		.with_sequence_name(QualifiedName::new("internal"))
		.with_options(SequenceOptions::new().with_start(5).with_cache(1));
	for (old, new) in [
		(None, Some(identity.clone())),
		(
			Some(identity.clone()),
			Some(
				IdentityDefinition::new(IdentityGeneration::ByDefault)
					.with_sequence_name(QualifiedName::new("internal")),
			),
		),
		(Some(identity), None),
	] {
		migration = migration.add_operation(Operation::Identity {
			operation: IdentityOperation::new(
				QualifiedName::new("events"),
				"n",
				FieldType::BigInteger,
				old,
				new,
			),
		});
	}
	// Act
	let json: Migration =
		serde_json::from_str(&serde_json::to_string(&migration).unwrap()).unwrap();
	let source = FilesystemRepository::new("/tmp/unused-6506-source")
		.render(
			&migration,
			MigrationRenderOptions {
				include_header: true,
			},
		)
		.unwrap();
	let ast = syn::parse_file(&source).unwrap();
	let loaded = crate::migrations::ast_parser::extract_migration_metadata_strict(
		&ast,
		"runs",
		"0002_lifecycle",
	)
	.unwrap();
	// Assert
	assert_eq!(json.operations, migration.operations);
	assert_eq!(loaded.operations, migration.operations);
}

#[rstest]
fn opaque_history_cannot_restore_owned_sequence(declared_state: ProjectState) {
	// Arrange
	let mut state = declared_state;
	state.has_opaque_schema_operations = true;
	// Act / Assert
	assert!(owned_sequence_restore_sql(&state, "run_inputs", None, &SqlDialect::Postgres).is_err());
}

#[rstest]
fn physical_renames_replay_references_and_avoid_redundant_ownership(declared_state: ProjectState) {
	// Arrange
	let mut desired = declared_state.clone();
	let key = desired.sequences.first_key_value().unwrap().0.clone();
	let rename = SequenceOperation::Rename {
		key: key.clone(),
		old: desired.sequences[&key].name.clone(),
		new: QualifiedName::new("renamed_sequence"),
	};
	let operations = [
		Operation::Sequence { operation: rename },
		Operation::RenameTable {
			old_name: "run_inputs".into(),
			new_name: "renamed_inputs".into(),
		},
		Operation::RenameColumn {
			table: "renamed_inputs".into(),
			old_name: "seq".into(),
			new_name: "renamed_seq".into(),
		},
	];
	// Act
	for operation in &operations {
		operation.state_forwards("runs", &mut desired);
	}
	desired.validate_sequences().unwrap();
	let generated = MigrationAutodetector::new(declared_state.clone(), desired.clone())
		.try_generate_operations()
		.unwrap();
	let mut alternate = declared_state;
	alternate.apply_migration_operations(&operations, "runs");
	alternate.validate_sequences().unwrap();
	// Assert
	assert_eq!(alternate.sequences, desired.sequences);
	assert!(!generated.iter().any(|operation| matches!(
		operation,
		Operation::Sequence {
			operation: SequenceOperation::Ownership { .. }
		}
	)));
	assert_eq!(
		desired.sequences[&key].owned_by.as_ref().unwrap().column,
		"renamed_seq"
	);
	assert_eq!(
		desired.sequences[&key]
			.owned_by
			.as_ref()
			.unwrap()
			.table
			.name,
		"renamed_inputs"
	);
}

#[rstest]
fn mutually_referencing_apps_create_sequences_before_either_table() {
	// Arrange
	let mut desired = ProjectState::new();
	for (app, provider) in [("left", "right"), ("right", "left")] {
		let key = SequenceKey::new(app, "numbers");
		desired
			.add_sequence(
				SequenceDefinition::new(key, QualifiedName::new(format!("{app}_seq")))
					.with_owned_by(Some(SequenceOwner::new(
						QualifiedName::new(format!("{provider}_values")),
						"n",
					))),
			)
			.unwrap();
		let mut model = ModelState::new(app, "Values");
		model.table_name = format!("{app}_values");
		model.add_field(
			FieldState::new("n", FieldType::BigInteger, false).with_sequence_default(
				SequenceDefault::new(
					SequenceKey::new(provider, "numbers"),
					QualifiedName::new(format!("{provider}_seq")),
				),
			),
		);
		desired.add_model(model);
	}
	// Act
	let migrations = MigrationAutodetector::new(ProjectState::new(), desired.clone())
		.try_generate_migrations()
		.unwrap();
	let mut graph = crate::migrations::MigrationGraph::new();
	for migration in &migrations {
		graph.add_migration(
			crate::migrations::MigrationKey::new(&migration.app_label, &migration.name),
			migration
				.dependencies
				.iter()
				.map(|(app, name)| crate::migrations::MigrationKey::new(app, name))
				.collect(),
		);
	}
	let order = graph.resolve_execution_order_with_replaces().unwrap();
	let mut replayed = ProjectState::new();
	for key in &order {
		let migration = migrations
			.iter()
			.find(|migration| migration.app_label == key.app_label && migration.name == key.name)
			.unwrap();
		for operation in &migration.operations {
			operation.state_forwards(&migration.app_label, &mut replayed);
		}
		replayed.validate_sequences().unwrap();
	}
	// Assert
	assert_eq!(migrations.len(), 6);
	assert!(
		order
			.iter()
			.take(2)
			.all(|key| key.name == "autodetected_sequences")
	);
	assert!(
		MigrationAutodetector::new(replayed, desired)
			.try_generate_operations()
			.unwrap()
			.is_empty()
	);
}
