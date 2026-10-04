//! Catalog checks completed before any sequence/identity migration DDL.
use super::super::{ColumnDefinition, Migration, MigrationDirection, Operation};
use super::*;
use crate::backends::DatabaseConnection;
use sqlx::Row;
use std::collections::BTreeSet;

fn columns(operation: &Operation) -> Vec<&ColumnDefinition> {
	match operation {
		Operation::CreateTable { columns, .. } => columns.iter().collect(),
		Operation::AddColumn { column, .. } => vec![column],
		Operation::AlterColumn {
			new_definition,
			old_definition,
			..
		} => old_definition
			.iter()
			.chain(std::iter::once(new_definition))
			.collect(),
		Operation::DropColumn { old_definition, .. } => old_definition.iter().collect(),
		_ => Vec::new(),
	}
}
fn typed(operation: &Operation) -> bool {
	matches!(
		operation,
		Operation::Sequence { .. } | Operation::Identity { .. }
	) || columns(operation)
		.iter()
		.any(|column| column.identity.is_some() || column.sequence_default.is_some())
}
fn resolved(name: &QualifiedName, schema: &str) -> QualifiedName {
	name.clone()
		.with_schema(name.schema.as_deref().unwrap_or(schema))
}
fn resolved_state(state: &ProjectState, schema: &str) -> Result<ProjectState> {
	let mut state = state.clone();
	for definition in state.sequences.values_mut() {
		definition.name = resolved(&definition.name, schema);
		if let Some(owner) = &mut definition.owned_by {
			owner.table = resolved(&owner.table, schema);
		}
	}
	for model in state.models.values_mut() {
		model
			.options
			.entry("schema".into())
			.or_insert_with(|| schema.into());
		for field in model.fields.values_mut() {
			if let Some(mut identity) = identity_from_params(&field.params)? {
				identity.sequence_name = identity.sequence_name.map(|name| resolved(&name, schema));
				field.params.insert(
					"identity".into(),
					serde_json::to_string(&identity).expect("identity metadata"),
				);
			}
			if let Some(mut default) = sequence_default_from_params(&field.params)? {
				default.name = resolved(&default.name, schema);
				field.params.insert(
					"sequence_default".into(),
					serde_json::to_string(&default).expect("sequence metadata"),
				);
			}
		}
	}
	state.validate_sequences()?;
	Ok(state)
}

pub(crate) async fn preflight(
	connection: &DatabaseConnection,
	migration: &Migration,
	before: &ProjectState,
	direction: MigrationDirection,
) -> Result<()> {
	let owned_deletion = migration
		.operations
		.iter()
		.any(|operation| match operation {
			Operation::DropTable { name } => before.sequences.values().any(|definition| {
				definition
					.owned_by
					.as_ref()
					.is_some_and(|owner| &owner.table.name == name)
			}),
			Operation::DropColumn { table, column, .. } => {
				before.sequences.values().any(|definition| {
					definition
						.owned_by
						.as_ref()
						.is_some_and(|owner| &owner.table.name == table && &owner.column == column)
				})
			}
			_ => false,
		});
	if !migration.operations.iter().any(typed) && !owned_deletion {
		return Ok(());
	}
	validate_backend(&super::super::sql_plan::migration_sql_dialect(connection))?;
	let pool = connection.into_postgres().ok_or_else(|| {
		MigrationError::InvalidMigration(
			"PostgreSQL sequence planning requires a PostgreSQL connection".into(),
		)
	})?;
	let error = |error: sqlx::Error| MigrationError::IntrospectionError(error.to_string());
	let (schema, current_role): (String, String) =
		sqlx::query_as("SELECT current_schema(), current_user::text")
			.fetch_one(&pool)
			.await
			.map_err(error)?;
	let mut current = before.clone();
	let mut planned = Vec::new();
	if direction == MigrationDirection::Forward {
		for operation in &migration.operations {
			if let Operation::Identity {
				operation: identity,
			} = operation
			{
				let field = current
					.models
					.values()
					.find(|model| {
						model.table_name == identity.table.name
							&& model
								.options
								.get("schema")
								.map(String::as_str)
								.unwrap_or(&schema) == identity
								.table
								.schema
								.as_deref()
								.unwrap_or(&schema)
					})
					.and_then(|model| model.fields.get(&identity.column))
					.ok_or_else(|| {
						MigrationError::InvalidMigration(
							"identity transition requires an existing column snapshot".into(),
						)
					})?;
				let column = ColumnDefinition::from_field_state(&identity.column, field);
				column.validate_generation()?;
				let expected = identity.old.as_ref().map(|old| {
					retarget_identity_width(
						old,
						identity
							.old_field_type
							.as_ref()
							.unwrap_or(&identity.field_type),
						&identity.field_type,
					)
				});
				let mut observed = column.identity.clone();
				if let Some(identity) = &mut observed {
					identity.sequence_name = identity
						.sequence_name
						.as_ref()
						.map(|name| resolved(name, &schema));
				}
				let mut expected = expected;
				if let Some(identity) = &mut expected {
					identity.sequence_name = identity
						.sequence_name
						.as_ref()
						.map(|name| resolved(name, &schema));
				}
				if observed != expected
					|| column.type_definition != identity.field_type
					|| !column.not_null
					|| column.default.is_some()
					|| column.sequence_default.is_some()
					|| column.generated.is_some()
				{
					return invalid(
						"identity transition conflicts with the column snapshot; remove incompatible defaults explicitly first",
					);
				}
			}
			if let Operation::Sequence {
				operation: SequenceOperation::RenameDeclaration { old, new },
			} = operation && (!current.sequences.contains_key(old)
				|| current.sequences.contains_key(new))
			{
				return invalid(
					"logical sequence rename requires an existing source and an unused target identity",
				);
			}
			let mut normalized_operation = operation.clone();
			if let Operation::Identity { operation } = &mut normalized_operation {
				operation.table.schema = current
					.models
					.values()
					.find(|model| {
						model.table_name == operation.table.name
							&& model
								.options
								.get("schema")
								.map(String::as_str)
								.unwrap_or(&schema) == operation
								.table
								.schema
								.as_deref()
								.unwrap_or(&schema)
					})
					.and_then(|model| model.options.get("schema"))
					.cloned();
			}
			normalized_operation.state_forwards(&migration.app_label, &mut current);
			planned.push(operation.clone());
		}
	} else {
		// Reverse snapshots are reconstructed from historical pre-operation state.
		// The caller validates reversibility before executing this completed plan.
		for operation in migration.operations.iter().rev() {
			if let Some(operation) = operation.to_reverse_operation(before)? {
				planned.push(operation);
			}
		}
	}
	let _normalized = resolved_state(&current, &schema)?;
	let mut created = BTreeSet::new();
	let mut deleting = Vec::new();
	for operation in &planned {
		match operation {
			Operation::CreateTable { name, columns, .. } => {
				created.insert(QualifiedName::new(name).with_schema(&schema));
				for column in columns {
					if let Some(name) = column
						.identity
						.as_ref()
						.and_then(|identity| identity.sequence_name.as_ref())
					{
						created.insert(resolved(name, &schema));
					}
				}
			}
			Operation::AddColumn { column, .. } => {
				if let Some(name) = column
					.identity
					.as_ref()
					.and_then(|identity| identity.sequence_name.as_ref())
				{
					created.insert(resolved(name, &schema));
				}
			}
			Operation::Sequence {
				operation: SequenceOperation::Create { definition },
			} => {
				created.insert(resolved(&definition.name, &schema));
			}
			Operation::Sequence {
				operation: SequenceOperation::Drop { definition },
			} => deleting.push(resolved(&definition.name, &schema)),
			Operation::Sequence {
				operation: SequenceOperation::Rename { new, .. },
			} => {
				created.insert(resolved(new, &schema));
			}
			Operation::Identity { operation }
				if operation
					.old
					.as_ref()
					.and_then(|old| old.sequence_name.as_ref())
					!= operation
						.new
						.as_ref()
						.and_then(|new| new.sequence_name.as_ref()) =>
			{
				if let Some(name) = operation
					.new
					.as_ref()
					.and_then(|identity| identity.sequence_name.as_ref())
				{
					created.insert(resolved(name, &schema));
				}
			}
			_ => {}
		}
	}
	if direction == MigrationDirection::Backward {
		for definition in before.sequences.values() {
			if definition.owned_by.as_ref().is_some_and(|owner| {
				migration
					.operations
					.iter()
					.any(|operation| releases(operation, &owner.table.name, &owner.column))
			}) {
				created.insert(resolved(&definition.name, &schema));
				planned.push(Operation::Sequence {
					operation: SequenceOperation::Ownership {
						key: definition.key.clone(),
						name: definition.name.clone(),
						old: None,
						new: definition.owned_by.clone(),
					},
				});
			}
		}
	}
	let restored_ownership: Vec<_> = planned
		.iter()
		.filter_map(|operation| {
			if let Operation::Sequence {
				operation: SequenceOperation::Create { definition },
			} = operation
			{
				definition
					.owned_by
					.as_ref()
					.map(|owner| Operation::Sequence {
						operation: SequenceOperation::Ownership {
							key: definition.key.clone(),
							name: definition.name.clone(),
							old: None,
							new: Some(owner.clone()),
						},
					})
			} else {
				None
			}
		})
		.collect();
	planned.extend(restored_ownership);
	let renames: std::collections::BTreeMap<_, _> = planned
		.iter()
		.filter_map(|operation| {
			if let Operation::Sequence {
				operation: SequenceOperation::Rename { old, new, .. },
			} = operation
			{
				Some((resolved(new, &schema), resolved(old, &schema)))
			} else if let Operation::RenameTable { old_name, new_name } = operation {
				Some((
					QualifiedName::new(new_name).with_schema(&schema),
					QualifiedName::new(old_name).with_schema(&schema),
				))
			} else {
				None
			}
		})
		.collect();
	let mut existing_sequences = BTreeSet::new();
	for operation in &planned {
		if let Operation::Sequence { operation } = operation {
			let name = match operation {
				SequenceOperation::Alter { old, .. }
				| SequenceOperation::Drop { definition: old } => Some(&old.name),
				SequenceOperation::Rename { old, .. } => Some(old),
				SequenceOperation::Ownership { name, .. }
				| SequenceOperation::Restart { name, .. } => Some(name),
				_ => None,
			};
			if let Some(name) = name {
				existing_sequences.insert(resolved(name, &schema));
			}
		}
	}
	for target in existing_sequences {
		let name = renames.get(&target).unwrap_or(&target);
		let metadata: Option<(String, String, bool)> = sqlx::query_as("SELECT c.relkind::text, c.relpersistence::text, EXISTS(SELECT 1 FROM pg_depend d WHERE d.classid='pg_class'::regclass AND d.objid=c.oid AND d.deptype='i') FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname=$1 AND c.relname=$2").bind(name.schema.as_ref().expect("resolved schema")).bind(&name.name).fetch_optional(&pool).await.map_err(error)?;
		if metadata.is_none() && !created.contains(&target) {
			return invalid("sequence operation references an unknown relation");
		}
		if let Some((kind, persistence, identity)) = metadata
			&& (kind != "S" || persistence != "p" || identity)
		{
			return invalid(
				"independent sequence operations require a permanent non-identity sequence",
			);
		}
	}

	for operation in &planned {
		if let Operation::Sequence {
			operation: SequenceOperation::Restart { name, value, .. },
		} = operation
		{
			let name = resolved(name, &schema);
			let bounds = if let Some(definition) = current
				.sequences
				.values()
				.find(|definition| resolved(&definition.name, &schema) == name)
			{
				let options = definition.options.effective()?;
				match (options.min_value, options.max_value) {
					(Some(SequenceBound::Value(min)), Some(SequenceBound::Value(max))) => {
						Some((min, max))
					}
					_ => None,
				}
			} else {
				let lookup = renames.get(&name).unwrap_or(&name);
				sqlx::query_as::<_, (i64, i64)>("SELECT s.seqmin, s.seqmax FROM pg_sequence s JOIN pg_class c ON c.oid=s.seqrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname=$1 AND c.relname=$2")
					.bind(lookup.schema.as_ref().expect("resolved schema")).bind(&lookup.name).fetch_optional(&pool).await.map_err(error)?
			};
			let Some((min, max)) = bounds else {
				return invalid("restart requires a known independent sequence");
			};
			if !(min..=max).contains(value) {
				return invalid("restart target is outside sequence bounds");
			}
		}
	}
	for name in &created {
		let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname=$1 AND c.relname=$2)").bind(name.schema.as_ref().expect("resolved schema")).bind(&name.name).fetch_one(&pool).await.map_err(error)?;
		if exists {
			return invalid(format!(
				"planned relation {} already exists; unmanaged objects cannot be adopted implicitly",
				name.quoted()
			));
		}
	}
	for operation in &planned {
		if let Operation::Sequence {
			operation: SequenceOperation::Ownership {
				name,
				new: Some(owner),
				..
			},
		} = operation
		{
			let name = resolved(name, &schema);
			let table = resolved(&owner.table, &schema);
			if name.schema != table.schema {
				return invalid("OWNED BY requires sequence and table in the same resolved schema");
			}
			let mut roles = Vec::new();
			for target in [&name, &table] {
				let object = renames.get(target).unwrap_or(target);
				let role: Option<String> = sqlx::query_scalar("SELECT pg_get_userbyid(c.relowner) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname=$1 AND c.relname=$2").bind(object.schema.as_ref().expect("resolved schema")).bind(&object.name).fetch_optional(&pool).await.map_err(error)?;
				roles.push(match role {
					Some(role) => role,
					None if created.contains(target) => current_role.clone(),
					None => return invalid("OWNED BY references an unknown relation"),
				});
			}
			if roles[0] != roles[1] {
				return invalid(
					"OWNED BY requires sequence and table owned by the same PostgreSQL role",
				);
			}
		}
	}
	for definition in before.sequences.values() {
		if definition.owned_by.as_ref().is_some_and(|owner| {
			planned
				.iter()
				.any(|operation| releases(operation, &owner.table.name, &owner.column))
		}) {
			deleting.push(resolved(&definition.name, &schema));
		}
	}
	for name in deleting {
		let rows = sqlx::query(r#"SELECT tn.nspname AS table_schema, t.relname AS table_name, a.attname AS column_name FROM pg_depend d
            JOIN pg_class s ON s.oid=d.refobjid JOIN pg_namespace n ON n.oid=s.relnamespace
            LEFT JOIN pg_attrdef ad ON d.classid='pg_attrdef'::regclass AND ad.oid=d.objid
            LEFT JOIN pg_class t ON t.oid=ad.adrelid LEFT JOIN pg_namespace tn ON tn.oid=t.relnamespace LEFT JOIN pg_attribute a ON a.attrelid=ad.adrelid AND a.attnum=ad.adnum
            WHERE d.refclassid='pg_class'::regclass AND n.nspname=$1 AND s.relname=$2 AND d.deptype='n'"#)
            .bind(name.schema.as_ref().expect("resolved schema")).bind(&name.name).fetch_all(&pool).await.map_err(error)?;
		for row in rows {
			let table_schema: Option<String> = row.try_get("table_schema").map_err(error)?;
			let table: Option<String> = row.try_get("table_name").map_err(error)?;
			let column: Option<String> = row.try_get("column_name").map_err(error)?;
			if !table_schema.zip(table.zip(column)).is_some_and(
				|(table_schema, (table, column))| {
					table_schema == schema
						&& planned
							.iter()
							.any(|operation| releases(operation, &table, &column))
				},
			) {
				return invalid(format!(
					"cannot drop {}: an unaccounted dependency exists; provide an explicit dependency migration",
					name.quoted()
				));
			}
		}
	}
	Ok(())
}
fn releases(operation: &Operation, table_name: &str, column_name: &str) -> bool {
	match operation {
		Operation::DropTable { name } => name == table_name,
		Operation::DropColumn { table, column, .. } => table == table_name && column == column_name,
		Operation::AlterColumn {
			table,
			column,
			old_definition,
			new_definition,
			..
		} => {
			table == table_name
				&& column == column_name
				&& old_definition.as_ref().is_some_and(|old| {
					old.default != new_definition.default
						|| old.sequence_default != new_definition.sequence_default
				})
		}
		_ => false,
	}
}
