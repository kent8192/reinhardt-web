use crate::{BindError, PreparedQuery, error, value_type};
use reinhardt_query::{Value, Values};
use sqlx::Arguments;
/// Adapt an owned renderer result without changing SQL or argument order.
///
/// Unsupported types and overflowing integer conversions fail before execution.
pub fn prepare_sqlite(
	built: (String, Values),
) -> Result<PreparedQuery<sqlx::sqlite::SqliteArguments<'static>>, BindError> {
	prepare_sqlite_impl(built, false)
}

/// Adapt generated arguments for existing UUID text columns (native-only, P0).
///
/// UUID values use canonical hyphenated text, including an explicitly supplied
/// typed NULL. Other values use the same native codecs and checked errors as
/// [`prepare_sqlite`]. SQL and argument order are unchanged.
pub fn prepare_sqlite_with_text_uuid(
	built: (String, Values),
) -> Result<PreparedQuery<sqlx::sqlite::SqliteArguments<'static>>, BindError> {
	prepare_sqlite_impl(built, true)
}

fn prepare_sqlite_impl(
	built: (String, Values),
	_text_uuid: bool,
) -> Result<PreparedQuery<sqlx::sqlite::SqliteArguments<'static>>, BindError> {
	let (sql, values) = built;
	let mut arguments = <sqlx::sqlite::SqliteArguments<'static>>::default();
	for (offset, value) in values.into_iter().enumerate() {
		let index = offset + 1;
		let kind = value_type(&value);
		let backend = "sqlite";
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
			Value::TinyInt(v) => add!(v),
			Value::SmallInt(v) => add!(v),
			Value::Int(v) => add!(v),
			Value::BigInt(v) => add!(v),
			Value::TinyUnsigned(v) => add!(v),
			Value::SmallUnsigned(v) => add!(v),
			Value::Unsigned(v) => add!(v),
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
			Value::ChronoDate(v) => add!(v.map(|v| *v)),
			#[cfg(feature = "with-chrono")]
			Value::ChronoTime(v) => add!(v.map(|v| *v)),
			#[cfg(feature = "with-chrono")]
			Value::ChronoDateTime(v) => add!(v.map(|v| *v)),
			#[cfg(feature = "with-chrono")]
			Value::ChronoDateTimeUtc(v) => add!(v.map(|v| *v)),
			#[cfg(feature = "with-chrono")]
			Value::ChronoDateTimeLocal(v) => add!(v.map(|v| *v)),
			#[cfg(feature = "with-chrono")]
			Value::ChronoDateTimeWithTimeZone(v) => add!(v.map(|v| *v)),
			#[cfg(feature = "with-uuid")]
			Value::Uuid(v) => {
				if _text_uuid {
					add!(v.map(|v| v.to_string()));
				} else {
					add!(v.map(|v| *v));
				}
			}
			#[cfg(feature = "with-json")]
			Value::Json(v) => add!(v.map(|v| sqlx::types::Json(*v))),
			#[cfg(feature = "with-rust_decimal")]
			Value::Decimal(_v) => return Err(fail("type has no supported codec for this backend")),
			#[cfg(feature = "with-bigdecimal")]
			Value::BigDecimal(_v) => return Err(fail("type has no supported codec for this backend")),
			#[cfg(feature = "pgvector")]
			Value::Vector(_v) => return Err(fail("type has no supported codec for this backend")),
			Value::Array(..) => return Err(fail("arrays require a PostgreSQL native codec")),
			_ => return Err(fail("value codec feature is disabled")),
		}
	}
	Ok(PreparedQuery { sql, arguments })
}
