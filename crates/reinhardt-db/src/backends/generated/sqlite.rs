use super::{error, value_type};
use crate::backends::error::Result;
use reinhardt_query::{Value, Values};
use sqlx::Arguments;
pub(in crate::backends) fn arguments(
	values: Values,
) -> Result<sqlx::sqlite::SqliteArguments<'static>> {
	let mut arguments = <sqlx::sqlite::SqliteArguments<'static>>::default();
	for (offset, value) in values.into_iter().enumerate() {
		let index = offset + 1;
		let kind = value_type(&value);
		let backend = "sqlite";
		let fail = |reason| error(backend, index, kind, reason);
		if matches!(&value, Value::Float(Some(v)) if v.is_nan())
			|| matches!(&value, Value::Double(Some(v)) if v.is_nan())
		{
			return Err(fail("NaN would become SQL NULL"));
		}
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
			Value::ChronoDate(v) => add!(v.map(|v| *v)),
			Value::ChronoTime(v) => add!(v.map(|v| *v)),
			Value::ChronoDateTime(v) => add!(v.map(|v| *v)),
			Value::ChronoDateTimeUtc(v) => add!(v.map(|v| *v)),
			Value::ChronoDateTimeLocal(v) => add!(v.map(|v| *v)),
			Value::ChronoDateTimeWithTimeZone(v) => add!(v.map(|v| *v)),
			Value::Uuid(v) => {
				add!(v.map(|v| v.to_string()));
			}
			Value::Json(v) => add!(v.map(|v| sqlx::types::Json(*v))),
			Value::Decimal(_v) => return Err(fail("type has no supported codec for this backend")),
			Value::BigDecimal(_v) => {
				return Err(fail("type has no supported codec for this backend"));
			}
			Value::Array(..) => return Err(fail("arrays require a PostgreSQL native codec")),
		}
	}
	Ok(arguments)
}
