//! PostgreSQL identity column declarations.
//!
//! Identity AST construction and SQL rendering have P2 native/WASM parity.
//! Execution requires a PostgreSQL database; no driver is part of this module.
//!
//! ```
//! use reinhardt_query::prelude::*;
//! let mut table = Query::create_table();
//! table.table("events").col(ColumnDef::new("number").big_integer().not_null(true)
//!     .identity(IdentityDef::new(IdentityGeneration::Always)
//!         .sequence_name("events_number_seq").start(10).cache(1)));
//! let sql = PostgresQueryBuilder.build_create_table_checked(&table)?.0;
//! assert!(sql.contains("GENERATED ALWAYS AS IDENTITY"));
//! # Ok::<(), reinhardt_query::QueryBuildError>(())
//! ```
use super::{DynIden, IntoIden, SequenceDef};

/// Identity value generation mode.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityGeneration {
	/// Explicit values require OVERRIDING SYSTEM VALUE.
	Always,
	/// Explicit values override generated values.
	ByDefault,
}

/// Structured column-owned identity definition.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct IdentityDef {
	pub(crate) generation: IdentityGeneration,
	pub(crate) name: Option<DynIden>,
	pub(crate) schema: Option<DynIden>,
	pub(crate) options: SequenceDef,
}
impl IdentityDef {
	/// Constructs an identity column declaration with default sequence options.
	pub fn new(generation: IdentityGeneration) -> Self {
		Self {
			generation,
			name: None,
			schema: None,
			options: SequenceDef::new(""),
		}
	}
	/// Sets a literal sequence name.
	pub fn sequence_name(mut self, name: impl IntoIden) -> Self {
		self.name = Some(name.into_iden());
		self
	}
	/// Sets a literal schema for the explicit sequence name.
	pub fn sequence_schema(mut self, schema: impl IntoIden) -> Self {
		self.schema = Some(schema.into_iden());
		self
	}
	/// Sets the increment.
	pub fn increment(mut self, value: i64) -> Self {
		self.options.increment = Some(value);
		self
	}
	/// Sets the minimum, or an explicit NO MINVALUE.
	pub fn min_value(mut self, value: Option<i64>) -> Self {
		self.options.min_value = Some(value);
		self
	}
	/// Sets the maximum, or an explicit NO MAXVALUE.
	pub fn max_value(mut self, value: Option<i64>) -> Self {
		self.options.max_value = Some(value);
		self
	}
	/// Sets the recorded start without implicitly restarting existing sequences.
	pub fn start(mut self, value: i64) -> Self {
		self.options.start = Some(value);
		self
	}
	/// Sets the cache size.
	pub fn cache(mut self, value: i64) -> Self {
		self.options.cache = Some(value);
		self
	}
	/// Enables or disables cycling.
	pub fn cycle(mut self, value: bool) -> Self {
		self.options.cycle = Some(value);
		self
	}
}
