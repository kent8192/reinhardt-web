//! PostgreSQL catalog sequence metadata. Allocation state is deliberately excluded.
#[cfg(feature = "postgres")]
use super::*;
use crate::migrations::{QualifiedName, SequenceOptions, SequenceOwner};

#[cfg(feature = "postgres")]
use crate::migrations::{IdentityDefinition, IdentityGeneration, SequenceBound, SequenceDataType};

/// Catalog definition and ownership of a PostgreSQL sequence.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct SequenceInfo {
	/// Literal schema and sequence components.
	pub name: QualifiedName,
	/// Effective options from pg_sequence, without last_value/is_called.
	pub options: SequenceOptions,
	/// Automatic or internal column dependency.
	pub owned_by: Option<SequenceOwner>,
	/// Whether PostgreSQL owns this as an identity column's internal sequence.
	pub identity: bool,
	/// PostgreSQL role that owns the sequence.
	pub role: String,
	/// Whether this is a permanent, supported sequence definition.
	pub supported: bool,
}

#[cfg(feature = "postgres")]
impl PostgresIntrospector {
	/// Reads sequence definitions from catalogs without inspecting allocation cursors.
	pub async fn read_sequences(&self) -> Result<Vec<SequenceInfo>> {
		use sqlx::Row;
		let rows = sqlx::query(r#"
            SELECT ns.nspname AS schema_name, seq.relname AS sequence_name,
                   format_type(s.seqtypid, NULL) AS data_type,
                   s.seqstart, s.seqincrement, s.seqmin, s.seqmax, s.seqcache, s.seqcycle,
                   dep.deptype::text AS dependency_type, tns.nspname AS table_schema,
                   tbl.relname AS table_name, attr.attname AS column_name,
                   pg_get_userbyid(seq.relowner) AS role_name, seq.relpersistence::text AS persistence
            FROM pg_sequence s
            JOIN pg_class seq ON seq.oid = s.seqrelid
            JOIN pg_namespace ns ON ns.oid = seq.relnamespace
            LEFT JOIN pg_depend dep ON dep.classid = 'pg_class'::regclass
                AND dep.objid = seq.oid AND dep.refclassid = 'pg_class'::regclass
                AND dep.deptype IN ('a', 'i') AND dep.refobjsubid > 0
            LEFT JOIN pg_class tbl ON tbl.oid = dep.refobjid
            LEFT JOIN pg_namespace tns ON tns.oid = tbl.relnamespace
            LEFT JOIN pg_attribute attr ON attr.attrelid = tbl.oid AND attr.attnum = dep.refobjsubid
            WHERE ns.nspname NOT LIKE 'pg_%' AND ns.nspname <> 'information_schema'
            ORDER BY ns.nspname, seq.relname
        "#).fetch_all(&self.pool).await.map_err(|error| MigrationError::IntrospectionError(error.to_string()))?;
		rows.into_iter()
			.map(|row| {
				let get_error =
					|error: sqlx::Error| MigrationError::IntrospectionError(error.to_string());
				let data_type: String = row.try_get("data_type").map_err(get_error)?;
				let width = match data_type.as_str() {
					"smallint" => SequenceDataType::SmallInteger,
					"integer" => SequenceDataType::Integer,
					"bigint" => SequenceDataType::BigInteger,
					_ => {
						return Err(MigrationError::IntrospectionError(format!(
							"unsupported sequence type: {data_type}"
						)));
					}
				};
				let table: Option<String> = row.try_get("table_name").map_err(get_error)?;
				let schema: String = row.try_get("schema_name").map_err(get_error)?;
				let dependency: Option<String> =
					row.try_get("dependency_type").map_err(get_error)?;
				let owned_by = table
					.map(|table| -> Result<_> {
						let table_schema: String =
							row.try_get("table_schema").map_err(get_error)?;
						let column: String = row.try_get("column_name").map_err(get_error)?;
						Ok(SequenceOwner::new(
							QualifiedName::new(table).with_schema(table_schema),
							column,
						))
					})
					.transpose()?;
				Ok(SequenceInfo {
					name: QualifiedName::new(
						row.try_get::<String, _>("sequence_name")
							.map_err(get_error)?,
					)
					.with_schema(schema),
					options: SequenceOptions::new()
						.with_data_type(width)
						.with_start(row.try_get("seqstart").map_err(get_error)?)
						.with_increment(row.try_get("seqincrement").map_err(get_error)?)
						.with_min_value(SequenceBound::Value(
							row.try_get("seqmin").map_err(get_error)?,
						))
						.with_max_value(SequenceBound::Value(
							row.try_get("seqmax").map_err(get_error)?,
						))
						.with_cache(row.try_get("seqcache").map_err(get_error)?)
						.with_cycle(row.try_get("seqcycle").map_err(get_error)?),
					owned_by,
					identity: dependency.as_deref() == Some("i"),
					role: row.try_get("role_name").map_err(get_error)?,
					supported: row.try_get::<String, _>("persistence").map_err(get_error)? == "p",
				})
			})
			.collect()
	}
	pub(super) fn column_identity(
		&self,
		schema: &str,
		catalog: &IdentitySequenceCatalog<'_>,
		table: &str,
		column: &str,
		generation: &str,
	) -> Result<IdentityDefinition> {
		let sequence = catalog.get(&(schema, table, column)).ok_or_else(|| {
			MigrationError::IntrospectionError(format!(
				"identity sequence not found for {schema}.{table}.{column}"
			))
		})?;
		if !sequence.supported {
			return Err(MigrationError::IntrospectionError(
				"unsupported identity sequence persistence".into(),
			));
		}
		let mode = match generation {
			"ALWAYS" => IdentityGeneration::Always,
			"BY DEFAULT" => IdentityGeneration::ByDefault,
			_ => {
				return Err(MigrationError::IntrospectionError(
					"unknown identity generation mode".into(),
				));
			}
		};
		Ok(IdentityDefinition::new(mode)
			.with_sequence_name(sequence.name.clone())
			.with_options(sequence.options.clone()))
	}
	pub(super) async fn default_sequence(
		&self,
		table: &str,
		column: &str,
	) -> Result<Option<QualifiedName>> {
		use sqlx::Row;
		// Dependency OIDs identify the sequence; expression equality restricts this
		// to a plain nextval default rather than guessing through arbitrary SQL.
		let row = sqlx::query(r#"
            SELECT sns.nspname AS schema_name, seq.relname AS sequence_name
            FROM pg_attrdef def
            JOIN pg_class tbl ON tbl.oid = def.adrelid
            JOIN pg_namespace tns ON tns.oid = tbl.relnamespace
            JOIN pg_attribute attr ON attr.attrelid = tbl.oid AND attr.attnum = def.adnum
            JOIN pg_depend dep ON dep.classid = 'pg_attrdef'::regclass AND dep.objid = def.oid
                AND dep.refclassid = 'pg_class'::regclass
            JOIN pg_class seq ON seq.oid = dep.refobjid AND seq.relkind = 'S'
            JOIN pg_namespace sns ON sns.oid = seq.relnamespace
            WHERE tns.nspname = current_schema() AND tbl.relname = $1 AND attr.attname = $2
                AND pg_get_expr(def.adbin, def.adrelid) = format('nextval(%L::regclass)', seq.oid::regclass::text)
        "#).bind(table).bind(column).fetch_optional(&self.pool).await.map_err(|error| MigrationError::IntrospectionError(error.to_string()))?;
		row.map(|row| -> Result<_> {
			Ok(QualifiedName::new(
				row.try_get::<String, _>("sequence_name")
					.map_err(|error| MigrationError::IntrospectionError(error.to_string()))?,
			)
			.with_schema(
				row.try_get::<String, _>("schema_name")
					.map_err(|error| MigrationError::IntrospectionError(error.to_string()))?,
			))
		})
		.transpose()
	}
}

#[cfg(feature = "postgres")]
pub(super) type IdentitySequenceCatalog<'a> =
	std::collections::HashMap<(&'a str, &'a str, &'a str), &'a SequenceInfo>;

#[cfg(feature = "postgres")]
pub(super) fn identity_catalog(sequences: &[SequenceInfo]) -> IdentitySequenceCatalog<'_> {
	sequences
		.iter()
		.filter(|sequence| sequence.identity)
		.filter_map(|sequence| {
			let owner = sequence.owned_by.as_ref()?;
			Some((
				(
					owner.table.schema.as_deref()?,
					owner.table.name.as_str(),
					owner.column.as_str(),
				),
				sequence,
			))
		})
		.collect()
}

#[cfg(all(test, feature = "postgres"))]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	#[tokio::test]
	async fn catalog_indexes_identity_sequences_by_qualified_owner() {
		// Arrange
		let sequences: Vec<_> = ["alpha", "beta"]
			.into_iter()
			.map(|schema| SequenceInfo {
				name: QualifiedName::new("events_n_seq").with_schema(schema),
				options: SequenceOptions::new().with_cache(if schema == "alpha" { 2 } else { 3 }),
				owned_by: Some(SequenceOwner::new(
					QualifiedName::new("events").with_schema(schema),
					"n",
				)),
				identity: true,
				role: "postgres".into(),
				supported: true,
			})
			.collect();
		let catalog = identity_catalog(&sequences);
		let introspector = PostgresIntrospector::new(
			sqlx::postgres::PgPoolOptions::new()
				.connect_lazy("postgres://unused/unused")
				.unwrap(),
		);
		// Act
		let identity = introspector
			.column_identity("beta", &catalog, "events", "n", "BY DEFAULT")
			.unwrap();
		// Assert
		assert_eq!(
			identity.sequence_name.unwrap().schema.as_deref(),
			Some("beta")
		);
		assert_eq!(identity.options.cache, Some(3));
		assert!(
			introspector
				.column_identity("missing", &catalog, "events", "n", "ALWAYS")
				.is_err()
		);
	}
}
