use crate::{BindError, PreparedQuery, error, value_type};
use reinhardt_query::{Value, Values};
use sqlx::Arguments;
/// Adapt an owned renderer result without changing SQL or argument order.
///
/// Unsupported types and overflowing integer conversions fail before execution.
pub fn prepare_postgres(
	built: (String, Values),
) -> Result<PreparedQuery<sqlx::postgres::PgArguments>, BindError> {
	let (sql, values) = built;
	let mut arguments = <sqlx::postgres::PgArguments>::default();
	for (offset, value) in values.into_iter().enumerate() {
		let index = offset + 1;
		let kind = value_type(&value);
		let backend = "postgres";
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
			Value::Uuid(v) => add!(v.map(|v| *v)),
			#[cfg(feature = "with-json")]
			Value::Json(v) => add!(v.map(|v| sqlx::types::Json(*v))),
			#[cfg(feature = "with-rust_decimal")]
			Value::Decimal(v) => add!(v.map(|v| *v)),
			#[cfg(feature = "with-bigdecimal")]
			Value::BigDecimal(v) => add!(v.map(|v| *v)),
			#[cfg(feature = "pgvector")]
			Value::Vector(v) => add!(
				v.map(|v| crate::vector::VectorArgument::new(*v))
					.transpose()
					.map_err(fail)?
			),
			Value::Array(ty, values) => add_array(&mut arguments, ty, values.map(|v| *v), index)?,
			_ => return Err(fail("value codec feature is disabled")),
		}
	}
	Ok(PreparedQuery { sql, arguments })
}
fn add_array(
	arguments: &mut sqlx::postgres::PgArguments,
	ty: reinhardt_query::ArrayType,
	values: Option<Vec<Value>>,
	index: usize,
) -> Result<(), BindError> {
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
						.collect::<Result<Vec<_>, BindError>>()
				})
				.transpose()?;
			arguments
				.add(converted)
				.map_err(|_| fail("SQLx encoding failed"))?;
		}};
	}
	// Query features may be enabled by another workspace consumer.
	#[allow(unreachable_patterns)]
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
		#[cfg(feature = "with-chrono")]
		ArrayType::ChronoDate => array!(ChronoDate, |v: Option<Box<_>>| Ok(v.map(|v| *v))),
		#[cfg(feature = "with-chrono")]
		ArrayType::ChronoTime => array!(ChronoTime, |v: Option<Box<_>>| Ok(v.map(|v| *v))),
		#[cfg(feature = "with-chrono")]
		ArrayType::ChronoDateTime => array!(ChronoDateTime, |v: Option<Box<_>>| Ok(v.map(|v| *v))),
		#[cfg(feature = "with-chrono")]
		ArrayType::ChronoDateTimeUtc => array!(ChronoDateTimeUtc, |v: Option<Box<_>>| Ok(v.map(|v| *v))),
		#[cfg(feature = "with-chrono")]
		ArrayType::ChronoDateTimeLocal => {
			array!(ChronoDateTimeLocal, |v: Option<Box<_>>| Ok(v.map(|v| *v)))
		}
		#[cfg(feature = "with-chrono")]
		ArrayType::ChronoDateTimeWithTimeZone => {
			array!(ChronoDateTimeWithTimeZone, |v: Option<Box<_>>| Ok(
				v.map(|v| *v)
			))
		}
		#[cfg(feature = "with-uuid")]
		ArrayType::Uuid => array!(Uuid, |v: Option<Box<_>>| Ok(v.map(|v| *v))),
		#[cfg(feature = "with-json")]
		ArrayType::Json => array!(Json, |v: Option<Box<_>>| Ok(
			v.map(|v| sqlx::types::Json(*v))
		)),
		#[cfg(feature = "with-rust_decimal")]
		ArrayType::Decimal => array!(Decimal, |v: Option<Box<_>>| Ok(v.map(|v| *v))),
		#[cfg(feature = "with-bigdecimal")]
		ArrayType::BigDecimal => array!(BigDecimal, |v: Option<Box<_>>| Ok(v.map(|v| *v))),
		_ => return Err(fail("array codec feature is disabled")),
	}
	Ok(())
}
