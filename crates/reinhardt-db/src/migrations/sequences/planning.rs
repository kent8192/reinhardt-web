//! Declaration validation and dependency-aware sequence operation generation.
use super::super::{ColumnDefinition, Operation};
use super::*;
use std::collections::{BTreeMap, BTreeSet};

impl ProjectState {
	/// Registers an independent sequence in a manually constructed project state.
	pub fn add_sequence(&mut self, definition: SequenceDefinition) -> Result<()> {
		definition.validate()?;
		if self.sequences.contains_key(&definition.key) {
			return invalid("duplicate sequence declaration identity");
		}
		self.sequences.insert(definition.key.clone(), definition);
		Ok(())
	}
	/// Validates declarations, column generation, and typed sequence dependencies.
	pub fn validate_sequences(&self) -> Result<()> {
		let mut relations = BTreeSet::new();
		for model in self.models.values() {
			let schema = model.options.get("schema").cloned();
			relations.insert((schema.clone(), model.table_name.clone()));
			for index in &model.indexes {
				relations.insert((schema.clone(), index.name.clone()));
			}
		}
		for model in self.models.values() {
			for (name, field) in &model.fields {
				identity_from_params(&field.params)?;
				sequence_default_from_params(&field.params)?;
				let column = ColumnDefinition::from_field_state(name, field);
				column.validate_generation()?;
				if let Some(default) = &column.sequence_default {
					let Some(sequence) = self.sequences.get(&default.key) else {
						return invalid("typed sequence default references an unknown declaration");
					};
					if sequence.name != default.name {
						return invalid(
							"typed sequence default's physical name does not match its declaration",
						);
					}
				}
				if let Some(identity) = &column.identity
					&& let Some(name) = &identity.sequence_name
					&& !relations.insert((name.schema.clone(), name.name.clone()))
				{
					return invalid(
						"identity sequence name conflicts with a relation in the same schema",
					);
				}
			}
		}
		for sequence in self.sequences.values() {
			sequence.validate()?;
			if !relations.insert((sequence.name.schema.clone(), sequence.name.name.clone())) {
				return invalid("sequence name conflicts with a relation in the same schema");
			}
			if let Some(owner) = &sequence.owned_by {
				// App-scoped states retain external sequence declarations as read-only
				// dependency context; global validation checks their owning models.
				if let Some(model) = self.find_model_by_table(&owner.table.name) {
					if !model.fields.contains_key(&owner.column) {
						return invalid("sequence ownership references an unknown column");
					}
				} else if self.sequence_scope.is_none() {
					return invalid("sequence ownership references an unknown table");
				}
			}
		}
		Ok(())
	}
}

fn scoped(state: &ProjectState, key: &SequenceKey) -> bool {
	state
		.sequence_scope
		.as_ref()
		.is_none_or(|app| &key.app_label == app)
}
fn owner_deleted(owner: &SequenceOwner, operations: &[Operation]) -> bool {
	operations.iter().any(|operation| match operation {
		Operation::DropTable { name } => name == &owner.table.name,
		Operation::DropColumn { table, column, .. } => {
			table == &owner.table.name && column == &owner.column
		}
		_ => false,
	})
}

/// Appends operations to the same path used by flat and per-app generation.
pub(crate) fn augment_operations(
	from: &ProjectState,
	to: &ProjectState,
	by_app: &mut BTreeMap<String, Vec<Operation>>,
) {
	// PostgreSQL tracks ownership by object OID through table/column renames.
	// Compare against the resulting names, avoiding a redundant OWNED BY whose
	// inverse would refer to the old table before its reverse rename.
	let original_from = from;
	let mut renamed_from = from.clone();
	for operation in by_app.values().flatten() {
		match operation {
			Operation::RenameTable { old_name, new_name } => {
				rename_owned_table(&mut renamed_from, old_name, new_name)
			}
			Operation::MoveModel {
				rename_table: true,
				old_table_name: Some(old),
				new_table_name: Some(new),
				..
			} => rename_owned_table(&mut renamed_from, old, new),
			_ => {}
		}
	}
	for operation in by_app.values().flatten() {
		if let Operation::RenameColumn {
			table,
			old_name,
			new_name,
		} = operation
		{
			rename_owned_column(&mut renamed_from, table, old_name, new_name);
		}
	}
	let from = &renamed_from;
	for (key, desired) in &to.sequences {
		if !scoped(to, key) {
			continue;
		}
		let operations = by_app.entry(key.app_label.clone()).or_default();
		match from.sequences.get(key) {
			None => {
				let mut definition = desired.clone();
				definition.owned_by = None;
				operations.insert(
					0,
					Operation::Sequence {
						operation: SequenceOperation::Create { definition },
					},
				);
				if desired.owned_by.is_some() {
					operations.push(Operation::Sequence {
						operation: SequenceOperation::Ownership {
							key: key.clone(),
							name: desired.name.clone(),
							old: None,
							new: desired.owned_by.clone(),
						},
					});
				}
			}
			Some(previous) => {
				if previous.name != desired.name {
					operations.insert(
						0,
						Operation::Sequence {
							operation: SequenceOperation::Rename {
								key: key.clone(),
								old: previous.name.clone(),
								new: desired.name.clone(),
							},
						},
					);
				}
				if previous
					.options
					.effective()
					.expect("validated previous options")
					!= desired
						.options
						.effective()
						.expect("validated desired options")
				{
					let mut old = previous.clone();
					old.name = desired.name.clone();
					old.owned_by = if desired.owned_by.is_none() {
						None
					} else {
						previous.owned_by.clone()
					};
					let mut new = desired.clone();
					new.owned_by = old.owned_by.clone();
					operations.push(Operation::Sequence {
						operation: SequenceOperation::Alter { old, new },
					});
				}
				if previous.owned_by != desired.owned_by {
					let operation = Operation::Sequence {
						operation: SequenceOperation::Ownership {
							key: key.clone(),
							name: if desired.owned_by.is_none() {
								previous.name.clone()
							} else {
								desired.name.clone()
							},
							old: if desired.owned_by.is_none() {
								original_from.sequences[key].owned_by.clone()
							} else {
								previous.owned_by.clone()
							},
							new: desired.owned_by.clone(),
						},
					};
					if desired.owned_by.is_none() {
						operations.insert(0, operation);
					} else {
						operations.push(operation);
					}
				}
			}
		}
	}
	let deletion_operations: Vec<_> = by_app.values().flatten().cloned().collect();
	for (key, previous) in &from.sequences {
		if !scoped(to, key) || to.sequences.contains_key(key) {
			continue;
		}
		let operations = by_app.entry(key.app_label.clone()).or_default();
		if !previous
			.owned_by
			.as_ref()
			.is_some_and(|owner| owner_deleted(owner, &deletion_operations))
		{
			operations.push(Operation::Sequence {
				operation: SequenceOperation::Drop {
					definition: previous.clone(),
				},
			});
		}
	}
	for (app, operations) in by_app.iter_mut() {
		// A sequence rename updates default OID references automatically. Keep
		// old column snapshots at the already-renamed physical name so rollback
		// does not reference the old name before reversing the sequence rename.
		operations.retain_mut(|operation| {
			if let Operation::AlterColumn {
				old_definition: Some(old),
				new_definition: new,
				..
			} = operation && let (Some(old_default), Some(new_default)) =
				(&mut old.sequence_default, &new.sequence_default)
				&& old_default.key == new_default.key
				&& let (Some(previous), Some(desired)) = (
					from.sequences.get(&old_default.key),
					to.sequences.get(&new_default.key),
				) && previous.name != desired.name
				&& old_default.name == previous.name
				&& new_default.name == desired.name
			{
				old_default.name = new_default.name.clone();
				return old != new;
			}
			true
		});
		rewrite_identity_changes(operations, to, app);
	}
	by_app.retain(|_, operations| !operations.is_empty());
}

fn rewrite_identity_changes(operations: &mut Vec<Operation>, to: &ProjectState, app: &str) {
	let mut rewritten = Vec::new();
	for operation in operations.drain(..) {
		if let Operation::AlterColumn {
			table,
			column,
			old_definition: Some(old),
			new_definition: new,
			..
		} = &operation
		{
			let old_identity = old.identity.clone();
			let mut new_identity = new.identity.clone();
			if let (Some(old), Some(new)) = (&old_identity, &mut new_identity)
				&& new.sequence_name.is_none()
			{
				new.sequence_name = old.sequence_name.clone();
			}
			if old_identity.is_some() || new_identity.is_some() {
				let equivalent = match (&old_identity, &new_identity) {
					(Some(old_identity), Some(new_identity)) => {
						old_identity.generation == new_identity.generation
							&& old_identity.sequence_name == new_identity.sequence_name
							&& old_identity
								.effective_options(&old.type_definition)
								.expect("validated identity")
								== new_identity
									.effective_options(&new.type_definition)
									.expect("validated identity")
					}
					_ => false,
				};
				let mut table_name = QualifiedName::new(table);
				if let Some(schema) = to
					.models
					.values()
					.find(|model| model.app_label == app && model.table_name == *table)
					.and_then(|model| model.options.get("schema"))
				{
					table_name.schema = Some(schema.clone());
				}
				let mut transition = IdentityOperation::new(
					table_name,
					column,
					if new_identity.is_none() {
						old.type_definition.clone()
					} else {
						new.type_definition.clone()
					},
					old_identity.clone(),
					new_identity.clone(),
				);
				if old_identity.is_some()
					&& new_identity.is_some()
					&& old.type_definition != new.type_definition
				{
					transition = transition.with_old_field_type(old.type_definition.clone());
				}
				let mut old_plain = old.clone();
				let mut new_plain = new.clone();
				old_plain.identity = None;
				new_plain.identity = None;
				old_plain.auto_increment = false;
				new_plain.auto_increment = false;
				if !equivalent && new_identity.is_none() {
					rewritten.push(Operation::Identity {
						operation: transition.clone(),
					});
				}
				if old_plain != new_plain {
					// The generic transition changes type/default/nullability first.
					// Keep the identity snapshot at the mode present at this stage.
					let staged_identity = if new_identity.is_none() {
						None
					} else {
						old_identity.clone()
					};
					old_plain.identity = staged_identity.clone();
					new_plain.identity = staged_identity.map(|identity| {
						retarget_identity_width(
							&identity,
							&old.type_definition,
							&new.type_definition,
						)
					});
					let mut column_operation = operation.clone();
					if let Operation::AlterColumn {
						old_definition,
						new_definition,
						..
					} = &mut column_operation
					{
						*old_definition = Some(old_plain);
						*new_definition = new_plain;
					}
					rewritten.push(column_operation);
				}
				if !equivalent && new_identity.is_some() {
					rewritten.push(Operation::Identity {
						operation: transition,
					});
				}
				continue;
			}
		}
		rewritten.push(operation);
	}
	*operations = rewritten;
}

/// Keeps sequence creation ahead of defaults and OWNED BY after column creation.
pub(crate) fn order_operations(operations: &mut Vec<Operation>) {
	let mut before = Vec::new();
	let mut middle = Vec::new();
	let mut after = Vec::new();
	for operation in operations.drain(..) {
		match &operation {
			Operation::Sequence {
				operation:
					SequenceOperation::Create { .. }
					| SequenceOperation::Rename { .. }
					| SequenceOperation::Ownership { new: None, .. },
			} => before.push(operation),
			Operation::Sequence {
				operation:
					SequenceOperation::Drop { .. } | SequenceOperation::Ownership { new: Some(_), .. },
			} => after.push(operation),
			_ => middle.push(operation),
		}
	}
	before.extend(middle);
	before.extend(after);
	// Relation names are shared by tables and sequences. Keep the stable order
	// unless a target must first be vacated, and move its consumers with it.
	let mut remaining: Vec<_> = before.into_iter().enumerate().collect();
	let mut ordered = Vec::new();
	while !remaining.is_empty() {
		let ready = remaining.iter().position(|(index, candidate)| {
			!remaining
				.iter()
				.any(|(other_index, other)| index != other_index && must_precede(other, candidate))
		});
		let Some(ready) = ready else {
			// Cyclic renames need explicit temporary names; preflight rejects the
			// occupied target before executing any statement.
			ordered.extend(remaining.into_iter().map(|(_, operation)| operation));
			break;
		};
		ordered.push(remaining.remove(ready).1);
	}
	*operations = ordered;
}

pub(super) fn acquisition(operation: &Operation) -> Option<QualifiedName> {
	match operation {
		Operation::Sequence {
			operation: SequenceOperation::Create { definition },
		} => Some(definition.name.clone()),
		Operation::Sequence {
			operation: SequenceOperation::Rename { new, .. },
		} => Some(new.clone()),
		Operation::CreateTable { name, .. } => Some(QualifiedName::new(name)),
		Operation::RenameTable { new_name, .. } => Some(QualifiedName::new(new_name)),
		_ => None,
	}
}

pub(super) fn release(operation: &Operation) -> Option<QualifiedName> {
	match operation {
		Operation::Sequence {
			operation: SequenceOperation::Drop { definition },
		} => Some(definition.name.clone()),
		Operation::Sequence {
			operation: SequenceOperation::Rename { old, .. },
		} => Some(old.clone()),
		Operation::DropTable { name } => Some(QualifiedName::new(name)),
		Operation::RenameTable { old_name, .. } => Some(QualifiedName::new(old_name)),
		_ => None,
	}
}

fn same_relation(left: &QualifiedName, right: &QualifiedName) -> bool {
	left.name == right.name
		&& (left.schema == right.schema || left.schema.is_none() || right.schema.is_none())
}

fn must_precede(before: &Operation, after: &Operation) -> bool {
	if let (Some(released), Some(acquired)) = (release(before), acquisition(after))
		&& same_relation(&released, &acquired)
	{
		return true;
	}
	let Some(acquired) = acquisition(before) else {
		return false;
	};
	match after {
		Operation::Sequence { operation } => match operation {
			SequenceOperation::Alter { old, .. } => same_relation(&acquired, &old.name),
			SequenceOperation::Ownership { name, .. } | SequenceOperation::Restart { name, .. } => {
				same_relation(&acquired, name)
			}
			_ => false,
		},
		Operation::CreateTable { columns, .. } => columns.iter().any(|column| {
			column
				.sequence_default
				.as_ref()
				.is_some_and(|default| same_relation(&acquired, &default.name))
		}),
		Operation::AddColumn { table, column, .. }
		| Operation::AlterColumn {
			table,
			new_definition: column,
			..
		} => {
			same_relation(&acquired, &QualifiedName::new(table))
				|| column
					.sequence_default
					.as_ref()
					.is_some_and(|default| same_relation(&acquired, &default.name))
		}
		Operation::RenameColumn { table, .. }
		| Operation::Identity {
			operation: IdentityOperation {
				table: QualifiedName { name: table, .. },
				..
			},
		} => acquired.name == *table,
		_ => false,
	}
}

pub(crate) fn validate_state_transition(from: &ProjectState, to: &ProjectState) -> Result<()> {
	for (key, desired) in &to.sequences {
		if !scoped(to, key) {
			continue;
		}
		if let Some(previous) = from.sequences.get(key) {
			if previous.name.schema != desired.name.schema {
				return invalid(
					"sequence schema moves require an explicit unsupported-operation decision",
				);
			}
		} else if from
			.sequences
			.iter()
			.any(|(old_key, previous)| old_key != key && previous.name == desired.name)
		{
			return invalid(
				"logical sequence identity changed for the same physical object; provide an explicit state migration instead of guessing a rename",
			);
		}
	}
	Ok(())
}

/// Tracks automatic ownership references without renaming the sequence itself.
pub(crate) fn rename_owned_table(state: &mut ProjectState, old: &str, new: &str) {
	for definition in state.sequences.values_mut() {
		if let Some(owner) = &mut definition.owned_by
			&& owner.table.name == old
		{
			owner.table.name = new.into();
		}
	}
}
pub(crate) fn rename_owned_column(state: &mut ProjectState, table: &str, old: &str, new: &str) {
	for definition in state.sequences.values_mut() {
		if let Some(owner) = &mut definition.owned_by
			&& owner.table.name == table
			&& owner.column == old
		{
			owner.column = new.into();
		}
	}
}

/// Mirrors PostgreSQL's automatic deletion of owned sequences.
pub(crate) fn remove_owned_sequences(state: &mut ProjectState, table: &str, column: Option<&str>) {
	state.sequences.retain(|_, definition| {
		!definition.owned_by.as_ref().is_some_and(|owner| {
			owner.table.name == table && column.is_none_or(|column| owner.column == column)
		})
	});
}

/// Recreates implicitly deleted schema objects around their owning column/table.
pub(crate) fn owned_sequence_restore_sql(
	state: &ProjectState,
	table: &str,
	column: Option<&str>,
	dialect: &SqlDialect,
) -> Result<(Vec<String>, Vec<String>)> {
	let definitions: Vec<_> = state
		.sequences
		.values()
		.filter(|definition| {
			definition.owned_by.as_ref().is_some_and(|owner| {
				owner.table.name == table && column.is_none_or(|column| owner.column == column)
			})
		})
		.collect();
	if !definitions.is_empty() {
		validate_backend(dialect)?;
		if state.has_opaque_schema_operations {
			return invalid(
				"cannot restore owned sequences from history containing opaque schema operations",
			);
		}
	}
	let mut before = Vec::new();
	let mut after = Vec::new();
	for definition in definitions {
		definition.validate()?;
		let mut independent = definition.clone();
		independent.owned_by = None;
		before.push(
			SequenceOperation::Create {
				definition: independent,
			}
			.to_sql(),
		);
		after.push(
			SequenceOperation::Ownership {
				key: definition.key.clone(),
				name: definition.name.clone(),
				old: None,
				new: definition.owned_by.clone(),
			}
			.to_sql(),
		);
	}
	Ok((before, after))
}

/// Defers cross-app ownership until both the sequence and its owner exist.
pub(crate) fn staged_migrations(
	to: &ProjectState,
	by_app: BTreeMap<String, Vec<Operation>>,
) -> Vec<super::super::Migration> {
	use super::super::Migration;
	let mut creations = Vec::new();
	let mut bases = Vec::new();
	let mut deferred = Vec::new();
	for (app, operations) in by_app {
		let mut base = Migration::new("autodetected", &app);
		let mut ownership = Migration::new("autodetected_ownership", &app);
		for operation in operations {
			let defer = matches!(&operation, Operation::Sequence { operation: SequenceOperation::Ownership { new: Some(owner), .. } }
                if to.find_model_by_table(&owner.table.name).is_some_and(|model| model.app_label != app));
			if defer {
				ownership = ownership.add_operation(operation);
			} else {
				base = base.add_operation(operation);
			}
		}
		let needs_creation_stage = base.operations.iter().any(|operation| {
			matches!(
				operation,
				Operation::Sequence {
					operation: SequenceOperation::Create { .. }
				}
			)
		})
			&& !super::super::MigrationAutodetector::foreign_key_provider_apps(
				to,
				&base.operations,
				&app,
			)
			.is_empty();
		if needs_creation_stage {
			let mut creation = Migration::new("autodetected_sequences", &app);
			base.operations.retain(|operation| {
				if matches!(
					operation,
					Operation::Sequence {
						operation: SequenceOperation::Create { .. }
					}
				) {
					creation.operations.push(operation.clone());
					false
				} else {
					true
				}
			});
			creations.push(creation);
		}
		if !base.operations.is_empty() {
			bases.push(base);
		}
		if !ownership.operations.is_empty() {
			deferred.push(ownership);
		}
	}
	let base_apps: BTreeSet<_> = bases
		.iter()
		.map(|migration| migration.app_label.clone())
		.collect();
	let creation_apps: BTreeSet<_> = creations
		.iter()
		.map(|migration| migration.app_label.clone())
		.collect();
	for migration in &mut bases {
		let mut dependencies = BTreeSet::new();
		if creation_apps.contains(&migration.app_label) {
			dependencies.insert((migration.app_label.clone(), "autodetected_sequences".into()));
		}
		let mut other_operations = migration.operations.clone();
		for operation in &mut other_operations {
			let columns: Vec<&mut ColumnDefinition> = match operation {
				Operation::CreateTable { columns, .. } => columns.iter_mut().collect(),
				Operation::AddColumn { column, .. } => vec![column],
				Operation::AlterColumn { new_definition, .. } => vec![new_definition],
				_ => Vec::new(),
			};
			for column in columns {
				if let Some(default) = column.sequence_default.take() {
					let provider = default.key.app_label;
					if provider != migration.app_label {
						if creation_apps.contains(&provider) {
							dependencies.insert((provider, "autodetected_sequences".into()));
						} else if base_apps.contains(&provider) {
							dependencies.insert((provider, "autodetected".into()));
						}
					}
				}
			}
		}
		for provider in super::super::MigrationAutodetector::foreign_key_provider_apps(
			to,
			&other_operations,
			&migration.app_label,
		) {
			if base_apps.contains(&provider) {
				dependencies.insert((provider, "autodetected".into()));
			}
		}
		migration.dependencies.extend(dependencies);
	}

	for migration in &mut deferred {
		if base_apps.contains(&migration.app_label) {
			migration
				.dependencies
				.push((migration.app_label.clone(), "autodetected".into()));
		} else if creation_apps.contains(&migration.app_label) {
			migration
				.dependencies
				.push((migration.app_label.clone(), "autodetected_sequences".into()));
		}
		for provider in super::super::MigrationAutodetector::foreign_key_provider_apps(
			to,
			&migration.operations,
			&migration.app_label,
		) {
			if base_apps.contains(&provider) {
				migration
					.dependencies
					.push((provider, "autodetected".into()));
			}
		}
	}
	creations.extend(bases);
	creations.extend(deferred);
	creations
}
