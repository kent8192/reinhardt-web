use crate::{BindError, PreparedQuery, error, value_type};
use reinhardt_query::{Value, Values};
use sqlx::Arguments;
/// Adapt an owned renderer result without changing SQL or argument order.
///
/// Unsupported types and overflowing integer conversions fail before execution.
pub fn prepare_mysql(
	built: (String, Values),
) -> Result<PreparedQuery<sqlx::mysql::MySqlArguments>, BindError> {
	let (sql, values) = built;
	let mut arguments = <sqlx::mysql::MySqlArguments>::default();
	for (offset, value) in values.into_iter().enumerate() {
		let index = offset + 1;
		let kind = value_type(&value);
		let backend = "mysql";
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
			Value::BigUnsigned(v) => add!(v),
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
			Value::ChronoDateTimeLocal(_v) => {
				return Err(fail("type has no supported codec for this backend"));
			}
			#[cfg(feature = "with-chrono")]
			Value::ChronoDateTimeWithTimeZone(_v) => {
				return Err(fail("type has no supported codec for this backend"));
			}
			#[cfg(feature = "with-uuid")]
			Value::Uuid(v) => add!(v.map(|v| *v)),
			#[cfg(feature = "with-json")]
			Value::Json(v) => add!(v.map(|v| sqlx::types::Json(*v))),
			#[cfg(feature = "with-rust_decimal")]
			Value::Decimal(v) => add!(v.map(|v| *v)),
			#[cfg(feature = "with-bigdecimal")]
			Value::BigDecimal(v) => add!(v.map(|v| *v)),
			#[cfg(feature = "pgvector")]
			Value::Vector(_v) => return Err(fail("type has no supported codec for this backend")),
			Value::Array(..) => return Err(fail("arrays require a PostgreSQL native codec")),
			_ => return Err(fail("value codec feature is disabled")),
		}
	}
	Ok(PreparedQuery { sql, arguments })
}
