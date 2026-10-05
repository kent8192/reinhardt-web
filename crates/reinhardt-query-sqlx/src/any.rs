use crate::{BindError, PreparedQuery, error, value_type};
use reinhardt_query::{Value, Values};
use sqlx::Arguments;
/// Explicit backend for the SQLx Any compatibility codecs.
///
/// # API parity
///
/// P0 (native-only): absent on `wasm32` targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AnyBackend {
	/// PostgreSQL protocol, including compatible CockroachDB callers.
	///
	/// P0 (native-only): absent on `wasm32` targets.
	Postgres,
	/// MySQL protocol.
	///
	/// P0 (native-only): absent on `wasm32` targets.
	MySql,
	/// SQLite protocol.
	///
	/// P0 (native-only): absent on `wasm32` targets.
	Sqlite,
}
impl AnyBackend {
	fn name(self) -> &'static str {
		match self {
			Self::Postgres => "postgres/any",
			Self::MySql => "mysql/any",
			Self::Sqlite => "sqlite/any",
		}
	}
}
/// Adapt an owned renderer result without changing SQL or argument order.
///
/// Unsupported types and overflowing integer conversions fail before execution.
///
/// # API parity
///
/// P0 (native-only): absent on `wasm32` targets.
pub fn prepare_any(
	built: (String, Values),
	backend: AnyBackend,
) -> Result<PreparedQuery<sqlx::any::AnyArguments<'static>>, BindError> {
	prepare_any_impl(built, backend, false)
}

/// Explicit compatibility encoding for schemas that store complex values as text.
///
/// UUID, JSON, decimals and chrono values use their documented textual encodings.
/// PostgreSQL callers must provide typed casts when targeting native columns;
/// this function does not change SQL. Arrays and vectors remain unsupported.
///
/// # API parity
///
/// P0 (native-only): absent on `wasm32` targets.
pub fn prepare_any_with_text_codecs(
	built: (String, Values),
	backend: AnyBackend,
) -> Result<PreparedQuery<sqlx::any::AnyArguments<'static>>, BindError> {
	prepare_any_impl(built, backend, true)
}
fn prepare_any_impl(
	built: (String, Values),
	backend: AnyBackend,
	_text_codecs: bool,
) -> Result<PreparedQuery<sqlx::any::AnyArguments<'static>>, BindError> {
	let (sql, values) = built;
	let mut arguments = <sqlx::any::AnyArguments<'static>>::default();
	for (offset, value) in values.into_iter().enumerate() {
		let index = offset + 1;
		let kind = value_type(&value);
		let backend = backend.name();
		let fail = |reason| error(backend, index, kind, reason);
		macro_rules! add {
			($value:expr) => {
				arguments
					.add($value)
					.map_err(|_| fail("SQLx encoding failed"))?
			};
		}
		// Feature unification may expose query values whose codec is disabled here.
		#[allow(unreachable_patterns)]
		match value {
			Value::Bool(v) => add!(v),
			Value::TinyInt(v) => add!(v.map(i16::from)),
			Value::SmallInt(v) => add!(v),
			Value::Int(v) => add!(v),
			Value::BigInt(v) => add!(v),
			Value::TinyUnsigned(v) => add!(v.map(i16::from)),
			Value::SmallUnsigned(v) => add!(v.map(i32::from)),
			Value::Unsigned(v) => add!(v.map(i64::from)),
			Value::BigUnsigned(v) => add!(
				v.map(i64::try_from)
					.transpose()
					.map_err(|_| fail("unsigned integer exceeds signed 64-bit range"))?
			),
			Value::Float(v) => add!(v),
			Value::Double(v) => add!(v),
			Value::Char(v) => add!(v.map(|c| c.to_string())),
			Value::String(v) => add!(v.map(|v| *v)),
			Value::Bytes(v) => add!(v.map(|v| *v)),
			#[cfg(feature = "with-chrono")]
			Value::ChronoDate(v) => {
				if !_text_codecs {
					return Err(fail(
						"complex values require explicit text compatibility codecs",
					));
				}
				add!(v.map(|v| v.to_string()))
			}
			#[cfg(feature = "with-chrono")]
			Value::ChronoTime(v) => {
				if !_text_codecs {
					return Err(fail(
						"complex values require explicit text compatibility codecs",
					));
				}
				add!(v.map(|v| v.to_string()))
			}
			#[cfg(feature = "with-chrono")]
			Value::ChronoDateTime(v) => {
				if !_text_codecs {
					return Err(fail(
						"complex values require explicit text compatibility codecs",
					));
				}
				add!(v.map(|v| v.to_string()))
			}
			#[cfg(feature = "with-chrono")]
			Value::ChronoDateTimeUtc(v) => {
				if !_text_codecs {
					return Err(fail(
						"complex values require explicit text compatibility codecs",
					));
				}
				add!(v.map(|v| v.to_rfc3339()))
			}
			#[cfg(feature = "with-chrono")]
			Value::ChronoDateTimeLocal(v) => {
				if !_text_codecs {
					return Err(fail(
						"complex values require explicit text compatibility codecs",
					));
				}
				add!(v.map(|v| v.to_rfc3339()))
			}
			#[cfg(feature = "with-chrono")]
			Value::ChronoDateTimeWithTimeZone(v) => {
				if !_text_codecs {
					return Err(fail(
						"complex values require explicit text compatibility codecs",
					));
				}
				add!(v.map(|v| v.to_rfc3339()))
			}
			#[cfg(feature = "with-uuid")]
			Value::Uuid(v) => {
				if !_text_codecs {
					return Err(fail(
						"complex values require explicit text compatibility codecs",
					));
				}
				add!(v.map(|v| v.to_string()))
			}
			#[cfg(feature = "with-json")]
			Value::Json(v) => {
				if !_text_codecs {
					return Err(fail(
						"complex values require explicit text compatibility codecs",
					));
				}
				add!(v.map(|v| v.to_string()))
			}
			#[cfg(feature = "with-rust_decimal")]
			Value::Decimal(v) => {
				if !_text_codecs {
					return Err(fail(
						"complex values require explicit text compatibility codecs",
					));
				}
				add!(v.map(|v| v.to_string()))
			}
			#[cfg(feature = "with-bigdecimal")]
			Value::BigDecimal(v) => {
				if !_text_codecs {
					return Err(fail(
						"complex values require explicit text compatibility codecs",
					));
				}
				add!(v.map(|v| v.to_string()))
			}
			#[cfg(feature = "pgvector")]
			Value::Vector(_v) => return Err(fail("type has no supported codec for this backend")),
			Value::Array(..) => return Err(fail("arrays require a PostgreSQL native codec")),
			_ => return Err(fail("value codec feature is disabled")),
		}
	}
	Ok(PreparedQuery { sql, arguments })
}
