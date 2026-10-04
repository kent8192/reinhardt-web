//! PostgreSQL sequence declarations and schema-only reversible operations.
//!
//! Sequence definitions never include the allocation cursor. Altering START changes
//! the recorded start; only an explicit restart changes the next allocated value.
//!
//! These migration APIs have P0 (native database) parity. For a portable SQL AST,
//! use Reinhardt Query's P2 sequence and identity builders.
//!
//! Independent sequences must be created before columns that use their defaults.
//! Establish column ownership after the table exists. Explicitly named identity
//! sequences remain internal to their columns and are never registered separately.
//!
//! ```
//! use reinhardt_db::migrations::{QualifiedName, SequenceDefinition, SequenceKey,
//!     SequenceOptions, SequenceOperation, IdentityDefinition, IdentityGeneration};
//! let definition = SequenceDefinition::new(
//!     SequenceKey::new("events", "event_numbers"),
//!     QualifiedName::new("event_numbers").with_schema("tenant"),
//! ).with_options(SequenceOptions::new().with_start(10).with_increment(2));
//! let create = SequenceOperation::Create { definition };
//! create.validate()?;
//! assert!(create.to_sql().contains("CREATE SEQUENCE"));
//! let identity = IdentityDefinition::new(IdentityGeneration::Always)
//!     .with_sequence_name(QualifiedName::new("events_sequence_seq"));
//! assert!(identity.to_sql().contains("GENERATED ALWAYS AS IDENTITY"));
//! # Ok::<(), reinhardt_db::migrations::MigrationError>(())
//! ```
//!
//! Register [`SequenceMetadata`] through the model registry during application
//! initialization. Field metadata accepts [`SequenceDefault`] and
//! [`IdentityDefinition`]. For generated models, PostgreSQL supports
//! `#[field(identity_always = true, identity_options(sequence_name = "events_sequence_seq",
//! start = 10, increment = 2, cache = 1))]`; use `identity_by_default = true`
//! to allow ordinary explicit INSERT values. Identity does not imply a primary key.
//!
//! Rollback restores definitions, never consumed numbers or data. Supply complete
//! history to [`super::DatabaseMigrationExecutor::with_migration_history`] for a
//! destructive rollback; missing new-operation snapshots are errors. An explicit
//! [`SequenceOperation::Restart`] needs a reverse target to be reversible.
//! `START WITH` changes the configured start without restarting allocation.
//!
//! Temporary/unlogged sequences, schema moves, role changes, and non-PostgreSQL
//! execution are unsupported. Opaque SQL dependencies require explicit migration
//! ordering; catalog discovery cannot recover a declaration's app/logical identity.
use super::{FieldType, MigrationError, ProjectState, Result, SqlDialect};
use reinhardt_query::{Alias, PostgresQueryBuilder, Query, QueryStatementBuilder, SequenceType};
use serde::{Deserialize, Serialize};

/// Literal identifier components; a dot in a component is not a separator.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct QualifiedName {
	/// Optional schema, resolved by PostgreSQL's search path when absent.
	pub schema: Option<String>,
	/// Relation name.
	pub name: String,
}
impl QualifiedName {
	/// Constructs an unqualified identifier.
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			schema: None,
			name: name.into(),
		}
	}
	/// Sets a literal schema component.
	pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
		self.schema = Some(schema.into());
		self
	}
	/// Quotes each component for PostgreSQL.
	pub fn quoted(&self) -> String {
		let name = quote(&self.name);
		self.schema
			.as_ref()
			.map_or(name.clone(), |schema| format!("{}.{name}", quote(schema)))
	}
	pub(crate) fn validate(&self) -> Result<()> {
		for component in self.schema.iter().chain(std::iter::once(&self.name)) {
			if component.is_empty() || component.contains('\0') {
				return invalid("sequence identifiers must be nonempty and contain no NUL");
			}
		}
		Ok(())
	}
}
fn quote(name: &str) -> String {
	format!("\"{}\"", name.replace('"', "\"\""))
}
fn invalid<T>(message: impl AsRef<str>) -> Result<T> {
	Err(MigrationError::InvalidMigration(message.as_ref().into()))
}

/// Stable app-scoped declaration identity, independent of its physical name.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SequenceKey {
	/// Owning application's label.
	pub app_label: String,
	/// Stable logical declaration name.
	pub logical_name: String,
}
impl SequenceKey {
	/// Constructs a stable declaration identity.
	pub fn new(app_label: impl Into<String>, logical_name: impl Into<String>) -> Self {
		Self {
			app_label: app_label.into(),
			logical_name: logical_name.into(),
		}
	}
}

/// Integer width of a PostgreSQL sequence.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SequenceDataType {
	/// 16-bit signed integer.
	SmallInteger,
	/// 32-bit signed integer.
	Integer,
	/// 64-bit signed integer (the PostgreSQL default).
	#[default]
	BigInteger,
}
impl SequenceDataType {
	fn query(self) -> SequenceType {
		match self {
			Self::SmallInteger => SequenceType::SmallInteger,
			Self::Integer => SequenceType::Integer,
			Self::BigInteger => SequenceType::BigInteger,
		}
	}
	fn bounds(self) -> (i64, i64) {
		match self {
			Self::SmallInteger => (i16::MIN.into(), i16::MAX.into()),
			Self::Integer => (i32::MIN.into(), i32::MAX.into()),
			Self::BigInteger => (i64::MIN, i64::MAX),
		}
	}
	/// Derives the sequence width from an identity column's type.
	pub fn from_field_type(field_type: &FieldType) -> Result<Self> {
		match field_type {
			FieldType::SmallInteger => Ok(Self::SmallInteger),
			FieldType::Integer => Ok(Self::Integer),
			FieldType::BigInteger => Ok(Self::BigInteger),
			_ => invalid("identity columns require smallint, integer, or bigint"),
		}
	}
}

/// Explicit bound reset versus a concrete sequence bound.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SequenceBound {
	/// PostgreSQL's type- and direction-dependent default.
	Default,
	/// A concrete inclusive bound.
	Value(i64),
}

/// Declared sequence options. Absent values retain source-level omission.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SequenceOptions {
	/// Integer width, defaulting to bigint for independent sequences.
	pub data_type: Option<SequenceDataType>,
	/// Allocation increment.
	pub increment: Option<i64>,
	/// Inclusive minimum or an explicit NO MINVALUE.
	pub min_value: Option<SequenceBound>,
	/// Inclusive maximum or an explicit NO MAXVALUE.
	pub max_value: Option<SequenceBound>,
	/// Recorded initial value, not the current allocation position.
	pub start: Option<i64>,
	/// Number of values cached per connection.
	pub cache: Option<i64>,
	/// Whether allocation cycles at the bound.
	pub cycle: Option<bool>,
}
impl SequenceOptions {
	/// Constructs an empty declaration.
	pub fn new() -> Self {
		Self::default()
	}
	/// Sets the integer width.
	pub fn with_data_type(mut self, value: SequenceDataType) -> Self {
		self.data_type = Some(value);
		self
	}
	/// Sets the increment.
	pub fn with_increment(mut self, value: i64) -> Self {
		self.increment = Some(value);
		self
	}
	/// Sets the minimum, including an explicit reset.
	pub fn with_min_value(mut self, value: SequenceBound) -> Self {
		self.min_value = Some(value);
		self
	}
	/// Sets the maximum, including an explicit reset.
	pub fn with_max_value(mut self, value: SequenceBound) -> Self {
		self.max_value = Some(value);
		self
	}
	/// Sets the recorded start.
	pub fn with_start(mut self, value: i64) -> Self {
		self.start = Some(value);
		self
	}
	/// Sets the cache size.
	pub fn with_cache(mut self, value: i64) -> Self {
		self.cache = Some(value);
		self
	}
	/// Sets cycling.
	pub fn with_cycle(mut self, value: bool) -> Self {
		self.cycle = Some(value);
		self
	}
	/// Resolves omitted options without reading or changing the allocation cursor.
	pub fn effective(&self) -> Result<Self> {
		let data_type = self.data_type.unwrap_or_default();
		let (type_min, type_max) = data_type.bounds();
		let increment = self.increment.unwrap_or(1);
		if increment == 0 {
			return invalid("sequence increment must not be zero");
		}
		let min = match self.min_value {
			Some(SequenceBound::Value(value)) => value,
			_ => {
				if increment > 0 {
					1
				} else {
					type_min
				}
			}
		};
		let max = match self.max_value {
			Some(SequenceBound::Value(value)) => value,
			_ => {
				if increment > 0 {
					type_max
				} else {
					-1
				}
			}
		};
		let start = self.start.unwrap_or(if increment > 0 { min } else { max });
		let cache = self.cache.unwrap_or(1);
		if min < type_min
			|| max > type_max
			|| min >= max
			|| start < min
			|| start > max
			|| cache <= 0
		{
			return invalid("invalid sequence bounds, start, or cache");
		}
		Ok(Self {
			data_type: Some(data_type),
			increment: Some(increment),
			min_value: Some(SequenceBound::Value(min)),
			max_value: Some(SequenceBound::Value(max)),
			start: Some(start),
			cache: Some(cache),
			cycle: Some(self.cycle.unwrap_or(false)),
		})
	}
	pub(crate) fn identity_options_sql(&self) -> String {
		let mut parts = Vec::new();
		if let Some(value) = self.increment {
			parts.push(format!("INCREMENT BY {value}"));
		}
		for (name, value) in [("MINVALUE", self.min_value), ("MAXVALUE", self.max_value)] {
			match value {
				Some(SequenceBound::Value(value)) => parts.push(format!("{name} {value}")),
				Some(SequenceBound::Default) => parts.push(format!("NO {name}")),
				None => {}
			}
		}
		if let Some(value) = self.start {
			parts.push(format!("START WITH {value}"));
		}
		if let Some(value) = self.cache {
			parts.push(format!("CACHE {value}"));
		}
		if let Some(value) = self.cycle {
			parts.push(if value { "CYCLE" } else { "NO CYCLE" }.into());
		}
		parts.join(" ")
	}
}

/// Optional ownership of an independent sequence by a physical column.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequenceOwner {
	/// Physical table, with optional schema.
	pub table: QualifiedName,
	/// Physical column (after db_column mapping).
	pub column: String,
}
impl SequenceOwner {
	/// Constructs a column ownership declaration.
	pub fn new(table: QualifiedName, column: impl Into<String>) -> Self {
		Self {
			table,
			column: column.into(),
		}
	}
}

/// Independent sequence declaration. Identity sequences are not registered here.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequenceDefinition {
	/// Stable declaration identity.
	pub key: SequenceKey,
	/// Physical sequence identifier.
	pub name: QualifiedName,
	/// Source-level options.
	pub options: SequenceOptions,
	/// Optional lifetime dependency on a column.
	pub owned_by: Option<SequenceOwner>,
}
/// App-level sequence metadata, registered independently of models.
pub type SequenceMetadata = SequenceDefinition;
impl SequenceDefinition {
	/// Constructs an independent declaration with default options.
	pub fn new(key: SequenceKey, name: QualifiedName) -> Self {
		Self {
			key,
			name,
			options: SequenceOptions::new(),
			owned_by: None,
		}
	}
	/// Sets sequence options.
	pub fn with_options(mut self, options: SequenceOptions) -> Self {
		self.options = options;
		self
	}
	/// Sets or removes column ownership.
	pub fn with_owned_by(mut self, owner: Option<SequenceOwner>) -> Self {
		self.owned_by = owner;
		self
	}
	/// Validates identifiers, options, and same-schema ownership.
	pub fn validate(&self) -> Result<()> {
		self.name.validate()?;
		if self.key.app_label.is_empty() || self.key.logical_name.is_empty() {
			return invalid("sequence declaration identity must be nonempty");
		}
		self.options.effective()?;
		if let Some(owner) = &self.owned_by {
			owner.table.validate()?;
			if owner.column.is_empty() || owner.column.contains('\0') {
				return invalid("invalid sequence ownership column");
			}
			if self
				.name
				.schema
				.as_ref()
				.zip(owner.table.schema.as_ref())
				.is_some_and(|(sequence, table)| sequence != table)
			{
				return invalid(
					"OWNED BY must use the sequence's schema; qualify both objects consistently",
				);
			}
		}
		Ok(())
	}
	fn create_sql(&self) -> String {
		let mut statement = Query::create_sequence();
		statement.name(Alias::new(&self.name.name));
		if let Some(schema) = &self.name.schema {
			statement.schema(Alias::new(schema));
		}
		let options = &self.options;
		if let Some(value) = options.data_type {
			statement.as_type(value.query());
		}
		if let Some(value) = options.increment {
			statement.increment(value);
		}
		if let Some(value) = options.min_value {
			statement.min_value(match value {
				SequenceBound::Default => None,
				SequenceBound::Value(value) => Some(value),
			});
		}
		if let Some(value) = options.max_value {
			statement.max_value(match value {
				SequenceBound::Default => None,
				SequenceBound::Value(value) => Some(value),
			});
		}
		if let Some(value) = options.start {
			statement.start(value);
		}
		if let Some(value) = options.cache {
			statement.cache(value);
		}
		if let Some(value) = options.cycle {
			statement.cycle(value);
		}
		statement.to_string(PostgresQueryBuilder)
	}
}

/// Typed default with a stable sequence dependency and a physical SQL reference.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequenceDefault {
	/// Stable declaration dependency.
	pub key: SequenceKey,
	/// Physical relation referenced by regclass.
	pub name: QualifiedName,
}
impl SequenceDefault {
	/// Constructs a typed nextval default.
	pub fn new(key: SequenceKey, name: QualifiedName) -> Self {
		Self { key, name }
	}
	/// Renders a properly quoted PostgreSQL regclass constant.
	pub fn to_sql(&self) -> String {
		format!(
			"nextval('{}'::regclass)",
			self.name.quoted().replace('\'', "''")
		)
	}
}

/// PostgreSQL identity generation mode.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentityGeneration {
	/// Explicit values require OVERRIDING SYSTEM VALUE.
	Always,
	/// Explicit values override generated values.
	ByDefault,
}
impl IdentityGeneration {
	fn sql(self) -> &'static str {
		match self {
			Self::Always => "ALWAYS",
			Self::ByDefault => "BY DEFAULT",
		}
	}
}

/// Column-owned identity sequence definition.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityDefinition {
	/// Generation mode.
	pub generation: IdentityGeneration,
	/// Optional explicit name of the column-owned sequence.
	pub sequence_name: Option<QualifiedName>,
	/// Sequence options; width is derived from the column.
	pub options: SequenceOptions,
}
impl IdentityDefinition {
	/// Constructs an identity definition.
	pub fn new(generation: IdentityGeneration) -> Self {
		Self {
			generation,
			sequence_name: None,
			options: SequenceOptions::new(),
		}
	}
	/// Sets an explicit identity sequence name.
	pub fn with_sequence_name(mut self, name: QualifiedName) -> Self {
		self.sequence_name = Some(name);
		self
	}
	/// Sets sequence options.
	pub fn with_options(mut self, options: SequenceOptions) -> Self {
		self.options = options;
		self
	}
	/// Validates an identity column and derives its effective sequence width.
	pub fn effective_options(&self, field_type: &FieldType) -> Result<SequenceOptions> {
		let width = SequenceDataType::from_field_type(field_type)?;
		if self.options.data_type.is_some_and(|value| value != width) {
			return invalid("identity sequence width must match its column");
		}
		if let Some(name) = &self.sequence_name {
			name.validate()?;
		}
		self.options.clone().with_data_type(width).effective()
	}
	fn query_definition(&self) -> reinhardt_query::IdentityDef {
		let mut identity = reinhardt_query::IdentityDef::new(match self.generation {
			IdentityGeneration::Always => reinhardt_query::IdentityGeneration::Always,
			IdentityGeneration::ByDefault => reinhardt_query::IdentityGeneration::ByDefault,
		});
		if let Some(name) = &self.sequence_name {
			identity = identity.sequence_name(Alias::new(&name.name));
			if let Some(schema) = &name.schema {
				identity = identity.sequence_schema(Alias::new(schema));
			}
		}
		if let Some(value) = self.options.increment {
			identity = identity.increment(value);
		}
		if let Some(value) = self.options.start {
			identity = identity.start(value);
		}
		if let Some(value) = self.options.cache {
			identity = identity.cache(value);
		}
		if let Some(value) = self.options.cycle {
			identity = identity.cycle(value);
		}
		for (minimum, bound) in [
			(true, self.options.min_value),
			(false, self.options.max_value),
		] {
			if let Some(bound) = bound {
				let value = match bound {
					SequenceBound::Default => None,
					SequenceBound::Value(value) => Some(value),
				};
				identity = if minimum {
					identity.min_value(value)
				} else {
					identity.max_value(value)
				};
			}
		}
		identity
	}
	/// Renders the column identity clause.
	pub fn to_sql(&self) -> String {
		let mut options = self.options.identity_options_sql();
		if let Some(name) = &self.sequence_name {
			if !options.is_empty() {
				options.push(' ');
			}
			options.push_str(&format!("SEQUENCE NAME {}", name.quoted()));
		}
		format!(
			"GENERATED {} AS IDENTITY{}",
			self.generation.sql(),
			if options.is_empty() {
				String::new()
			} else {
				format!(" ({options})")
			}
		)
	}
}

/// Explicit sequence operation with complete schema snapshots.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
// Inline snapshots keep authored migration source and the strict AST loader simple.
// These schema operations are constructed during migration planning, outside hot paths.
#[allow(clippy::large_enum_variant)]
pub enum SequenceOperation {
	/// Creates an independent sequence (ownership must be a later operation).
	Create {
		/// Complete declaration.
		definition: SequenceDefinition,
	},
	/// Alters the complete definition without restarting allocation.
	Alter {
		/// Definition before alteration.
		old: SequenceDefinition,
		/// Desired complete definition.
		new: SequenceDefinition,
	},
	/// Drops with RESTRICT, retaining the definition for schema rollback.
	Drop {
		/// Definition before dropping.
		definition: SequenceDefinition,
	},
	/// Renames within one schema, preserving the allocation position.
	Rename {
		/// Stable declaration identity.
		key: SequenceKey,
		/// Original identifier.
		old: QualifiedName,
		/// New identifier.
		new: QualifiedName,
	},
	/// Changes the sequence's column lifetime dependency.
	Ownership {
		/// Stable declaration identity.
		key: SequenceKey,
		/// Sequence identifier.
		name: QualifiedName,
		/// Previous ownership.
		old: Option<SequenceOwner>,
		/// Desired ownership.
		new: Option<SequenceOwner>,
	},
	/// Explicitly maps a declaration identity without touching its physical object.
	RenameDeclaration {
		/// Previous app and logical name.
		old: SequenceKey,
		/// New app and logical name.
		new: SequenceKey,
	},
	/// Explicitly changes allocation position. Never emitted by autodetection.
	Restart {
		/// Physical sequence identifier.
		name: QualifiedName,
		/// Next allocated value.
		value: i64,
		/// Explicit reverse target; absence makes rollback irreversible.
		reverse_value: Option<i64>,
	},
}
impl SequenceOperation {
	/// Validates the operation before SQL planning.
	pub fn validate(&self) -> Result<()> {
		match self {
			Self::Create { definition } | Self::Drop { definition } => definition.validate(),
			Self::Alter { old, new } => {
				old.validate()?;
				new.validate()?;
				if old.key != new.key || old.name != new.name || old.owned_by != new.owned_by {
					return invalid(
						"AlterSequence cannot rename, move schemas, or change ownership",
					);
				}
				Ok(())
			}
			Self::Rename { old, new, .. } => {
				old.validate()?;
				new.validate()?;
				if old.schema != new.schema {
					return invalid(
						"moving a sequence between schemas is outside the supported migration contract",
					);
				}
				Ok(())
			}
			Self::Ownership { key, name, new, .. } => {
				SequenceDefinition::new(key.clone(), name.clone())
					.with_owned_by(new.clone())
					.validate()
			}
			Self::RenameDeclaration { old, new } => {
				if [old, new]
					.iter()
					.any(|key| key.app_label.is_empty() || key.logical_name.is_empty())
				{
					return invalid("sequence declaration identity must be nonempty");
				}
				Ok(())
			}
			Self::Restart { name, .. } => name.validate(),
		}
	}
	/// Generates PostgreSQL SQL through the structured query builders.
	pub fn to_sql(&self) -> String {
		match self {
			Self::Create { definition } => definition.create_sql(),
			Self::Drop { definition } => {
				let mut statement = Query::drop_sequence();
				statement.name(Alias::new(&definition.name.name)).restrict();
				if let Some(schema) = &definition.name.schema {
					statement.schema(Alias::new(schema));
				}
				statement.to_string(PostgresQueryBuilder)
			}
			Self::Alter { new, .. } => {
				let options = new.options.effective().expect("validated sequence options");
				let mut statement = Query::alter_sequence();
				statement.name(Alias::new(&new.name.name));
				if let Some(schema) = &new.name.schema {
					statement.schema(Alias::new(schema));
				}
				statement
					.as_type(options.data_type.expect("effective type").query())
					.increment_by(options.increment.expect("effective increment"));
				if let Some(SequenceBound::Value(value)) = options.min_value {
					statement.min_value(value);
				}
				if let Some(SequenceBound::Value(value)) = options.max_value {
					statement.max_value(value);
				}
				statement
					.start(options.start.expect("effective start"))
					.cache(options.cache.expect("effective cache"));
				if options.cycle == Some(true) {
					statement.cycle();
				} else {
					statement.no_cycle();
				}
				statement.to_string(PostgresQueryBuilder)
			}
			Self::Rename { old, new, .. } => {
				let mut statement = Query::alter_sequence();
				statement
					.name(Alias::new(&old.name))
					.rename_to(Alias::new(&new.name));
				if let Some(schema) = &old.schema {
					statement.schema(Alias::new(schema));
				}
				statement.to_string(PostgresQueryBuilder)
			}
			Self::Ownership { name, new, .. } => {
				let mut statement = Query::alter_sequence();
				statement.name(Alias::new(&name.name));
				if let Some(schema) = &name.schema {
					statement.schema(Alias::new(schema));
				}
				match new {
					None => {
						statement.owned_by_none();
					}
					Some(owner) => {
						if let Some(schema) = &owner.table.schema {
							statement.owned_by_schema_column(
								Alias::new(schema),
								Alias::new(&owner.table.name),
								Alias::new(&owner.column),
							);
						} else {
							statement.owned_by_column(
								Alias::new(&owner.table.name),
								Alias::new(&owner.column),
							);
						}
					}
				}
				statement.to_string(PostgresQueryBuilder)
			}
			Self::RenameDeclaration { .. } => "-- Rename sequence declaration identity".into(),
			Self::Restart { name, value, .. } => {
				let mut statement = Query::alter_sequence();
				statement.name(Alias::new(&name.name)).restart(Some(*value));
				if let Some(schema) = &name.schema {
					statement.schema(Alias::new(schema));
				}
				statement.to_string(PostgresQueryBuilder)
			}
		}
	}
	/// Produces the schema-only inverse, or rejects an unspecified restart target.
	pub fn reverse(&self) -> Result<Self> {
		Ok(match self {
			Self::Create { definition } => Self::Drop {
				definition: definition.clone(),
			},
			Self::Drop { definition } => Self::Create {
				definition: definition.clone(),
			},
			Self::Alter { old, new } => Self::Alter {
				old: new.clone(),
				new: old.clone(),
			},
			Self::Rename { key, old, new } => Self::Rename {
				key: key.clone(),
				old: new.clone(),
				new: old.clone(),
			},
			Self::Ownership {
				key,
				name,
				old,
				new,
			} => Self::Ownership {
				key: key.clone(),
				name: name.clone(),
				old: new.clone(),
				new: old.clone(),
			},
			Self::RenameDeclaration { old, new } => Self::RenameDeclaration {
				old: new.clone(),
				new: old.clone(),
			},
			Self::Restart {
				name,
				value,
				reverse_value,
			} => Self::Restart {
				name: name.clone(),
				value: reverse_value.ok_or_else(|| {
					MigrationError::IrreversibleError(
						"RestartSequence requires an explicit reverse target".into(),
					)
				})?,
				reverse_value: Some(*value),
			},
		})
	}
	pub(crate) fn state_forwards(&self, state: &mut ProjectState) {
		match self {
			Self::Create { definition }
			| Self::Alter {
				new: definition, ..
			} => {
				state
					.sequences
					.insert(definition.key.clone(), definition.clone());
			}
			Self::Drop { definition } => {
				state.sequences.remove(&definition.key);
			}
			Self::Rename { key, new, .. } => {
				if let Some(definition) = state.sequences.get_mut(key) {
					definition.name = new.clone();
				}
				for model in state.models.values_mut() {
					for field in model.fields.values_mut() {
						if let Some(mut default) = sequence_default_from_params(&field.params)
							.expect("validated sequence default")
							&& &default.key == key
						{
							default.name = new.clone();
							field.params.insert(
								"sequence_default".into(),
								serde_json::to_string(&default).expect("serializable default"),
							);
						}
					}
				}
			}
			Self::Ownership { key, new, .. } => {
				if let Some(definition) = state.sequences.get_mut(key) {
					definition.owned_by = new.clone();
				}
			}
			Self::RenameDeclaration { old, new } => {
				if let Some(mut definition) = state.sequences.remove(old) {
					definition.key = new.clone();
					state.sequences.insert(new.clone(), definition);
					for model in state.models.values_mut() {
						for field in model.fields.values_mut() {
							if let Some(mut default) = sequence_default_from_params(&field.params)
								.expect("validated sequence default")
								&& &default.key == old
							{
								default.key = new.clone();
								field.params.insert(
									"sequence_default".into(),
									serde_json::to_string(&default).expect("serializable default"),
								);
							}
						}
					}
				}
			}
			Self::Restart { .. } => {}
		}
	}
}

/// Rejects every new sequence/identity feature on non-PostgreSQL backends.
pub(crate) fn validate_backend(dialect: &SqlDialect) -> Result<()> {
	let backend = match dialect {
		SqlDialect::Postgres => return Ok(()),
		SqlDialect::Mysql => "mysql",
		SqlDialect::Sqlite => "sqlite",
		SqlDialect::Cockroachdb => "cockroachdb",
	};
	Err(MigrationError::UnsupportedBackendFeature {
		feature: "PostgreSQL sequences and identity definitions",
		backend,
	})
}

/// Adds, alters, or removes a column-owned identity without rewriting old rows.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityOperation {
	/// Physical table identifier.
	pub table: QualifiedName,
	/// Physical column identifier.
	pub column: String,
	/// Column width used to resolve sequence defaults.
	pub field_type: FieldType,
	/// Previous column width, when a type change precedes this transition.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub old_field_type: Option<FieldType>,
	/// Complete previous identity definition, absent when adding identity.
	pub old: Option<IdentityDefinition>,
	/// Complete desired definition, absent when dropping identity.
	pub new: Option<IdentityDefinition>,
}
impl IdentityOperation {
	/// Constructs a reversible identity transition.
	pub fn new(
		table: QualifiedName,
		column: impl Into<String>,
		field_type: FieldType,
		old: Option<IdentityDefinition>,
		new: Option<IdentityDefinition>,
	) -> Self {
		let mut new = new;
		if let (Some(old), Some(new)) = (&old, &mut new)
			&& new.sequence_name.is_none()
		{
			new.sequence_name = old.sequence_name.clone();
		}
		Self {
			table,
			column: column.into(),
			field_type,
			old_field_type: None,
			old,
			new,
		}
	}
	/// Retains the original width for validation and schema rollback.
	pub fn with_old_field_type(mut self, field_type: FieldType) -> Self {
		self.old_field_type = Some(field_type);
		self
	}
	/// Validates the transition before generating SQL.
	pub fn validate(&self) -> Result<()> {
		self.table.validate()?;
		if self.column.is_empty()
			|| self.column.contains('\0')
			|| (self.old.is_none() && self.new.is_none())
		{
			return invalid("invalid identity transition");
		}
		if let Some(identity) = &self.old {
			identity.effective_options(self.old_field_type.as_ref().unwrap_or(&self.field_type))?;
		}
		if let Some(identity) = &self.new {
			identity.effective_options(&self.field_type)?;
		}
		if let (Some(old), Some(new)) = (&self.old, &self.new) {
			if old.sequence_name != new.sequence_name
				&& (old.sequence_name.is_none() || new.sequence_name.is_none())
			{
				return invalid("identity sequence rename requires both physical names");
			}
			if let (Some(old_name), Some(new_name)) = (&old.sequence_name, &new.sequence_name)
				&& old_name.schema != new_name.schema
			{
				return invalid("identity sequence cannot move schemas");
			}
		}
		Ok(())
	}
	/// Generates identity DDL; option changes preserve the allocation cursor.
	pub fn to_sql(&self) -> String {
		let mut statement = Query::alter_table();
		match &self.table.schema {
			Some(schema) => {
				statement.table((Alias::new(schema), Alias::new(&self.table.name)));
			}
			None => {
				statement.table(Alias::new(&self.table.name));
			}
		}
		match (&self.old, &self.new) {
			(None, Some(new)) => {
				statement.add_identity(Alias::new(&self.column), new.query_definition());
			}
			(Some(_), None) => {
				statement.drop_identity(Alias::new(&self.column));
			}
			(Some(old), Some(new)) => {
				let options = new
					.effective_options(&self.field_type)
					.expect("validated identity options");
				let effective = IdentityDefinition::new(new.generation).with_options(options);
				statement.set_identity(Alias::new(&self.column), effective.query_definition());
				let mut sql = statement.to_string(PostgresQueryBuilder);
				if old.sequence_name != new.sequence_name {
					let old_name = old.sequence_name.as_ref().expect("validated rename source");
					let new_name = new.sequence_name.as_ref().expect("validated rename target");
					let mut rename = Query::alter_sequence();
					rename
						.name(Alias::new(&old_name.name))
						.rename_to(Alias::new(&new_name.name));
					if let Some(schema) = &old_name.schema {
						rename.schema(Alias::new(schema));
					}
					sql.push_str("; ");
					sql.push_str(&rename.to_string(PostgresQueryBuilder));
				}
				return sql;
			}
			(None, None) => unreachable!("identity transition validated before planning"),
		}
		statement.to_string(PostgresQueryBuilder)
	}
	/// Reverses the schema definition without recovering consumed values.
	pub fn reverse(&self) -> Self {
		Self {
			field_type: self
				.old_field_type
				.clone()
				.unwrap_or_else(|| self.field_type.clone()),
			old_field_type: self
				.old_field_type
				.as_ref()
				.map(|_| self.field_type.clone()),
			old: self.new.clone(),
			new: self.old.clone(),
			..self.clone()
		}
	}
	pub(crate) fn state_forwards(&self, state: &mut ProjectState) {
		if let Some(model) = state
			.models
			.values_mut()
			.find(|model| model_matches(model, &self.table))
			&& let Some(field) = model.fields.get_mut(&self.column)
		{
			field.params.remove("identity_always");
			field.params.remove("identity_by_default");
			match &self.new {
				Some(identity) => {
					field.params.insert(
						"identity".into(),
						serde_json::to_string(identity).expect("serializable identity"),
					);
				}
				None => {
					field.params.remove("identity");
					field.params.insert("auto_increment".into(), "false".into());
				}
			}
		}
	}
}

pub(crate) fn model_matches(model: &super::ModelState, table: &QualifiedName) -> bool {
	model.table_name == table.name && model.options.get("schema") == table.schema.as_ref()
}

pub(crate) fn retarget_identity_width(
	identity: &IdentityDefinition,
	old_type: &FieldType,
	new_type: &FieldType,
) -> IdentityDefinition {
	let mut identity = identity.clone();
	if old_type != new_type {
		let old_defaults = SequenceOptions::new()
			.with_data_type(SequenceDataType::from_field_type(old_type).expect("integer identity"))
			.with_increment(identity.options.increment.unwrap_or(1))
			.effective()
			.expect("default options");
		let new_defaults = SequenceOptions::new()
			.with_data_type(SequenceDataType::from_field_type(new_type).expect("integer identity"))
			.with_increment(identity.options.increment.unwrap_or(1))
			.effective()
			.expect("default options");
		if identity.options.data_type.is_some() {
			identity.options.data_type = new_defaults.data_type;
		}
		if identity.options.min_value == old_defaults.min_value {
			identity.options.min_value = new_defaults.min_value;
		}
		if identity.options.max_value == old_defaults.max_value {
			identity.options.max_value = new_defaults.max_value;
		}
	}
	identity
}

pub(crate) fn identity_from_params(
	params: &std::collections::HashMap<String, String>,
) -> Result<Option<IdentityDefinition>> {
	let always = params
		.get("identity_always")
		.is_some_and(|value| value == "true");
	let by_default = params
		.get("identity_by_default")
		.is_some_and(|value| value == "true");
	if always && by_default {
		return invalid("identity_always and identity_by_default are mutually exclusive");
	}
	if let Some(value) = params.get("identity") {
		let identity: IdentityDefinition = serde_json::from_str(value).map_err(|error| {
			MigrationError::InvalidMigration(format!("invalid identity metadata: {error}"))
		})?;
		if (always && identity.generation != IdentityGeneration::Always)
			|| (by_default && identity.generation != IdentityGeneration::ByDefault)
		{
			return invalid("conflicting identity generation modes");
		}
		return Ok(Some(identity));
	}
	let mut identity = if always {
		Some(IdentityDefinition::new(IdentityGeneration::Always))
	} else if by_default {
		Some(IdentityDefinition::new(IdentityGeneration::ByDefault))
	} else {
		None
	};
	if let Some(value) = params.get("identity_options") {
		let Some(identity) = identity.as_mut() else {
			return invalid("identity_options requires an explicit identity generation mode");
		};
		let mut options: serde_json::Value = serde_json::from_str(value).map_err(|error| {
			MigrationError::InvalidMigration(format!("invalid identity options: {error}"))
		})?;
		let object = options.as_object_mut().ok_or_else(|| {
			MigrationError::InvalidMigration("identity options must be an object".into())
		})?;
		let name = object.remove("sequence_name");
		let schema = object.remove("sequence_schema");
		if let Some(name) = name {
			let name = name.as_str().ok_or_else(|| {
				MigrationError::InvalidMigration("identity sequence name must be a string".into())
			})?;
			let mut qualified = QualifiedName::new(name);
			if let Some(schema) = schema {
				qualified = qualified.with_schema(schema.as_str().ok_or_else(|| {
					MigrationError::InvalidMigration("identity schema must be a string".into())
				})?);
			}
			identity.sequence_name = Some(qualified);
		} else if schema.is_some() {
			return invalid("identity schema requires an explicit sequence name");
		}
		identity.options = serde_json::from_value(options).map_err(|error| {
			MigrationError::InvalidMigration(format!("invalid identity options: {error}"))
		})?;
	}
	Ok(identity)
}
pub(crate) fn sequence_default_from_params(
	params: &std::collections::HashMap<String, String>,
) -> Result<Option<SequenceDefault>> {
	params
		.get("sequence_default")
		.map(|value| {
			serde_json::from_str(value).map_err(|error| {
				MigrationError::InvalidMigration(format!(
					"invalid sequence default metadata: {error}"
				))
			})
		})
		.transpose()
}
pub(crate) fn write_column_params(
	column: &super::ColumnDefinition,
	params: &mut std::collections::HashMap<String, String>,
) {
	if let Some(identity) = &column.identity {
		params.insert(
			"identity".into(),
			serde_json::to_string(identity).expect("serializable identity"),
		);
	}
	if let Some(default) = &column.sequence_default {
		params.insert(
			"sequence_default".into(),
			serde_json::to_string(default).expect("serializable sequence default"),
		);
	}
}

mod to_tokens;

mod planning;
pub(crate) use planning::{
	augment_operations, order_operations, owned_sequence_restore_sql, remove_owned_sequences,
	rename_owned_column, rename_owned_table, staged_migrations, validate_state_transition,
};

#[cfg(test)]
mod tests;

#[cfg(feature = "postgres")]
mod preflight;
#[cfg(feature = "postgres")]
pub(crate) use preflight::preflight;
