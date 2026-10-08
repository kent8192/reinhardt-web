use super::{error, value_type};
use crate::backends::error::Result;
use reinhardt_query::{Value, Values};
use sqlx::Arguments;
pub(in crate::backends) fn arguments(values: Values) -> Result<sqlx::mysql::MySqlArguments> {
	let mut arguments = <sqlx::mysql::MySqlArguments>::default();
	for (offset, value) in values.into_iter().enumerate() {
		let index = offset + 1;
		let kind = value_type(&value);
		let backend = "mysql";
		let fail = |reason| error(backend, index, kind, reason);
		super::validate_value(&value, backend, index)?;
		macro_rules! add {
			($value:expr) => {
				arguments
					.add($value)
					.map_err(|_| fail("SQLx encoding failed"))?
			};
		}
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
			Value::ChronoDate(v) => add!(v.map(|v| *v)),
			Value::ChronoTime(v) => add!(v.map(|v| *v)),
			Value::ChronoDateTime(v) => add!(v.map(|v| *v)),
			Value::ChronoDateTimeUtc(v) => add!(v.map(|v| *v)),
			Value::ChronoDateTimeLocal(v) => add!(v.map(|v| *v)),
			Value::ChronoDateTimeWithTimeZone(v) => {
				add!(v.map(|v| v.with_timezone(&chrono::Utc)));
			}
			Value::Uuid(v) => {
				add!(v.map(|v| v.to_string()));
			}
			Value::Json(v) => add!(v.map(|v| sqlx::types::Json(*v))),
			Value::Decimal(v) => add!(v.map(|v| *v)),
			Value::BigDecimal(v) => add!(v.map(|v| v.normalized())),
			#[cfg(feature = "pgvector")]
			Value::Vector(..) => return Err(fail("vectors require a PostgreSQL native codec")),
			Value::Array(..) => return Err(fail("arrays require a PostgreSQL native codec")),
		}
	}
	Ok(arguments)
}
