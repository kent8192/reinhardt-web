//! Native-only argument adaptation (P0: absent on browser WASM).
//!
//! Converts exactly the arguments returned by a typed query renderer into owned
//! SQLx arguments. Consumers retain execution, transaction and row-decoding ownership.
//!
//! Select backends through the consuming crate's existing database features.
//! This crate adds no executor dependencies to `reinhardt-query`.
//!
//! ```
//! # #[cfg(feature = "postgres")] {
//! use reinhardt_query::{Expr, PostgresQueryBuilder, Query, QueryStatementBuilder};
//! use reinhardt_query_sqlx::prepare_postgres;
//! let built = Query::select().expr(Expr::val(42i32)).build(PostgresQueryBuilder);
//! let prepared = prepare_postgres(built).unwrap();
//! let (sql, arguments) = prepared.into_parts();
//! assert_eq!(sql, "SELECT $1");
//! use sqlx::Arguments;
//! assert_eq!(arguments.len(), 1);
//! # }
//! ```

#![cfg(not(target_arch = "wasm32"))]

#[cfg(any(
	feature = "postgres",
	feature = "mysql",
	feature = "sqlite",
	feature = "any"
))]
use reinhardt_query::Value;

/// SQL and its owned arguments, kept together until the consumer executes it.
///
/// This type deliberately does not implement Debug: argument values may be sensitive.
///
/// # API parity
///
/// P0 (native-only): absent on `wasm32` targets.
pub struct PreparedQuery<A> {
	sql: String,
	arguments: A,
}
impl<A> PreparedQuery<A> {
	/// Consume the pair for `sqlx::query_with` or `sqlx::query_as_with`.
	///
	/// # API parity
	///
	/// P0 (native-only): absent on `wasm32` targets.
	pub fn into_parts(self) -> (String, A) {
		(self.sql, self.arguments)
	}
	/// Inspect the generated SQL without displaying argument values.
	///
	/// # API parity
	///
	/// P0 (native-only): absent on `wasm32` targets.
	pub fn sql(&self) -> &str {
		&self.sql
	}
}

/// Redacted argument adaptation failure. Indices are one-based.
///
/// # API parity
///
/// P0 (native-only): absent on `wasm32` targets.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("cannot encode {value_type} argument {index} for {backend}: {reason}")]
#[non_exhaustive]
pub struct BindError {
	/// Backend whose encoder rejected the argument.
	///
	/// P0 (native-only): absent on `wasm32` targets.
	pub backend: &'static str,
	/// One-based position in the renderer's Values sequence.
	///
	/// P0 (native-only): absent on `wasm32` targets.
	pub index: usize,
	/// Value variant, without its contents.
	///
	/// P0 (native-only): absent on `wasm32` targets.
	pub value_type: &'static str,
	/// Technical reason, without its contents.
	///
	/// P0 (native-only): absent on `wasm32` targets.
	pub reason: &'static str,
}
// Other workspace consumers can enable additional Value variants independently.
#[allow(unreachable_patterns)]
#[cfg(any(
	feature = "postgres",
	feature = "mysql",
	feature = "sqlite",
	feature = "any"
))]
fn value_type(value: &Value) -> &'static str {
	match value {
		Value::Bool(..) => "Bool",
		Value::TinyInt(..) => "TinyInt",
		Value::SmallInt(..) => "SmallInt",
		Value::Int(..) => "Int",
		Value::BigInt(..) => "BigInt",
		Value::TinyUnsigned(..) => "TinyUnsigned",
		Value::SmallUnsigned(..) => "SmallUnsigned",
		Value::Unsigned(..) => "Unsigned",
		Value::BigUnsigned(..) => "BigUnsigned",
		Value::Float(..) => "Float",
		Value::Double(..) => "Double",
		Value::Char(..) => "Char",
		Value::String(..) => "String",
		Value::Bytes(..) => "Bytes",
		Value::Array(..) => "Array",
		#[cfg(feature = "with-chrono")]
		Value::ChronoDate(..) => "ChronoDate",
		#[cfg(feature = "with-chrono")]
		Value::ChronoTime(..) => "ChronoTime",
		#[cfg(feature = "with-chrono")]
		Value::ChronoDateTime(..) => "ChronoDateTime",
		#[cfg(feature = "with-chrono")]
		Value::ChronoDateTimeUtc(..) => "ChronoDateTimeUtc",
		#[cfg(feature = "with-chrono")]
		Value::ChronoDateTimeLocal(..) => "ChronoDateTimeLocal",
		#[cfg(feature = "with-chrono")]
		Value::ChronoDateTimeWithTimeZone(..) => "ChronoDateTimeWithTimeZone",
		#[cfg(feature = "with-uuid")]
		Value::Uuid(..) => "Uuid",
		#[cfg(feature = "with-json")]
		Value::Json(..) => "Json",
		#[cfg(feature = "with-rust_decimal")]
		Value::Decimal(..) => "Decimal",
		#[cfg(feature = "with-bigdecimal")]
		Value::BigDecimal(..) => "BigDecimal",
		#[cfg(feature = "pgvector")]
		Value::Vector(..) => "Vector",
		_ => "feature-gated value",
	}
}
#[cfg(any(
	feature = "postgres",
	feature = "mysql",
	feature = "sqlite",
	feature = "any"
))]
fn error(
	backend: &'static str,
	index: usize,
	value_type: &'static str,
	reason: &'static str,
) -> BindError {
	BindError {
		backend,
		index,
		value_type,
		reason,
	}
}

#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "pgvector")]
mod vector;
#[cfg(feature = "postgres")]
pub use postgres::prepare_postgres;
#[cfg(feature = "mysql")]
mod mysql;
#[cfg(feature = "mysql")]
pub use mysql::{prepare_mysql, prepare_mysql_with_text_uuid};
#[cfg(feature = "sqlite")]
mod sqlite;
#[cfg(feature = "sqlite")]
pub use sqlite::{prepare_sqlite, prepare_sqlite_with_text_uuid};
#[cfg(feature = "any")]
mod any;
#[cfg(feature = "any")]
pub use any::{AnyBackend, prepare_any, prepare_any_with_text_codecs};

#[cfg(all(
	test,
	any(
		feature = "postgres",
		feature = "mysql",
		feature = "sqlite",
		feature = "any"
	)
))]
mod tests;
