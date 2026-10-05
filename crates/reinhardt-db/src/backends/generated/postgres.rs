use super::{error, value_type};
use crate::backends::error::Result;
use reinhardt_query::{Value, Values};
use sqlx::Arguments;
pub(in crate::backends) fn arguments(values: Values) -> Result<sqlx::postgres::PgArguments> {
	let mut arguments = <sqlx::postgres::PgArguments>::default();
	for (offset, value) in values.into_iter().enumerate() {
		let index = offset + 1;
		let kind = value_type(&value);
		let backend = "postgres";
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
			Value::ChronoDate(v) => add!(v.map(|v| *v)),
			Value::ChronoTime(v) => add!(v.map(|v| *v)),
			Value::ChronoDateTime(v) => add!(v.map(|v| *v)),
			Value::ChronoDateTimeUtc(v) => add!(v.map(|v| *v)),
			Value::ChronoDateTimeLocal(v) => add!(v.map(|v| *v)),
			Value::ChronoDateTimeWithTimeZone(v) => add!(v.map(|v| *v)),
			Value::Uuid(v) => add!(v.map(|v| *v)),
			Value::Json(v) => add!(v.map(|v| sqlx::types::Json(*v))),
			Value::Decimal(v) => add!(v.map(|v| *v)),
			Value::BigDecimal(v) => add!(v.map(|v| v.normalized())),
			Value::Array(ty, values) => add_array(&mut arguments, ty, values.map(|v| *v), index)?,
		}
	}
	Ok(arguments)
}
fn add_array(
	arguments: &mut sqlx::postgres::PgArguments,
	ty: reinhardt_query::ArrayType,
	values: Option<Vec<Value>>,
	index: usize,
) -> Result<()> {
	use reinhardt_query::ArrayType;
	let fail = |reason| error("postgres", index, "Array", reason);
	macro_rules! array {
		($variant:ident, $convert:expr) => {{
			let converted = values
				.map(|values| {
					values
						.into_iter()
						.map(|value| match value {
							// The canonical untyped NULL uses Int(None); the array
							// declaration supplies its SQLx element type.
							Value::Int(None) => Ok(None),
							Value::$variant(v) => ($convert)(v),
							_ => Err(fail("array element does not match declared element type")),
						})
						.collect::<Result<Vec<_>>>()
				})
				.transpose()?;
			arguments
				.add(converted)
				.map_err(|_| fail("SQLx encoding failed"))?;
		}};
	}
	match ty {
		ArrayType::Bool => array!(Bool, |v: Option<_>| Ok(v)),
		ArrayType::TinyInt => array!(TinyInt, |v: Option<_>| Ok(v.map(i16::from))),
		ArrayType::SmallInt => array!(SmallInt, |v: Option<_>| Ok(v)),
		ArrayType::Int => array!(Int, |v: Option<_>| Ok(v)),
		ArrayType::BigInt => array!(BigInt, |v: Option<_>| Ok(v)),
		ArrayType::TinyUnsigned => array!(TinyUnsigned, |v: Option<_>| Ok(v.map(i16::from))),
		ArrayType::SmallUnsigned => array!(SmallUnsigned, |v: Option<_>| Ok(v.map(i32::from))),
		ArrayType::Unsigned => array!(Unsigned, |v: Option<_>| Ok(v.map(i64::from))),
		ArrayType::BigUnsigned => array!(BigUnsigned, |v: Option<_>| v
			.map(i64::try_from)
			.transpose()
			.map_err(|_| fail("unsigned array element exceeds signed 64-bit range"))),
		ArrayType::Float => array!(Float, |v: Option<_>| Ok(v)),
		ArrayType::Double => array!(Double, |v: Option<_>| Ok(v)),
		ArrayType::Char => array!(Char, |v: Option<char>| Ok(v.map(|c| c.to_string()))),
		ArrayType::String => array!(String, |v: Option<Box<String>>| Ok(v.map(|v| *v))),
		ArrayType::Bytes => array!(Bytes, |v: Option<Box<Vec<u8>>>| Ok(v.map(|v| *v))),
		ArrayType::ChronoDate => array!(ChronoDate, |v: Option<Box<_>>| Ok(v.map(|v| *v))),
		ArrayType::ChronoTime => array!(ChronoTime, |v: Option<Box<_>>| Ok(v.map(|v| *v))),
		ArrayType::ChronoDateTime => array!(ChronoDateTime, |v: Option<Box<_>>| Ok(v.map(|v| *v))),
		ArrayType::ChronoDateTimeUtc => {
			array!(ChronoDateTimeUtc, |v: Option<Box<_>>| Ok(v.map(|v| *v)))
		}
		ArrayType::ChronoDateTimeLocal => {
			array!(ChronoDateTimeLocal, |v: Option<Box<_>>| Ok(v.map(|v| *v)))
		}
		ArrayType::ChronoDateTimeWithTimeZone => {
			array!(ChronoDateTimeWithTimeZone, |v: Option<Box<_>>| Ok(
				v.map(|v| *v)
			))
		}
		ArrayType::Uuid => array!(Uuid, |v: Option<Box<_>>| Ok(v.map(|v| *v))),
		ArrayType::Json => array!(Json, |v: Option<Box<_>>| Ok(
			v.map(|v| JsonElement(sqlx::types::Json(*v)))
		)),
		ArrayType::Jsonb => array!(Json, |v: Option<Box<_>>| Ok(
			v.map(|v| sqlx::types::Json(*v))
		)),
		ArrayType::Decimal => array!(Decimal, |v: Option<Box<_>>| Ok(v.map(|v| *v))),
		ArrayType::BigDecimal => array!(BigDecimal, |v: Option<Box<sqlx::types::BigDecimal>>| Ok(
			v.map(|v| v.normalized())
		)),
	}
	Ok(())
}

// SQLx's Json codec defaults to JSONB. Declare JSON for JSON arrays while
// retaining its native encoder, which adapts the wire prefix to the resolved type.
struct JsonElement(sqlx::types::Json<serde_json::Value>);

impl sqlx::Type<sqlx::Postgres> for JsonElement {
	fn type_info() -> sqlx::postgres::PgTypeInfo {
		sqlx::postgres::PgTypeInfo::with_name("JSON")
	}
}

impl sqlx::postgres::PgHasArrayType for JsonElement {
	fn array_type_info() -> sqlx::postgres::PgTypeInfo {
		sqlx::postgres::PgTypeInfo::with_name("_json")
	}
}

impl sqlx::Encode<'_, sqlx::Postgres> for JsonElement {
	fn encode_by_ref(
		&self,
		buffer: &mut sqlx::postgres::PgArgumentBuffer,
	) -> std::result::Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
		sqlx::Encode::<sqlx::Postgres>::encode_by_ref(&self.0, buffer)
	}
}
