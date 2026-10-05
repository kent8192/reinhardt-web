//! Argument codecs for renderer-generated SQL, separate from the public raw API.
//!
//! Values retain their type and order until native SQLx encoding. Encoding errors
//! contain only the backend, type, one-based argument index, and a fixed reason.
//! BigDecimal values use a normalized representation for range checks and encoding.

use reinhardt_query::{Value, Values};

use super::{DatabaseError, Result, types::QueryValue};

#[cfg(feature = "mysql")]
pub(super) mod mysql;
#[cfg(feature = "postgres")]
pub(super) mod postgres;
#[cfg(feature = "sqlite")]
pub(super) mod sqlite;

pub(super) fn value_type(value: &Value) -> &'static str {
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
		Value::ChronoDate(..) => "ChronoDate",
		Value::ChronoTime(..) => "ChronoTime",
		Value::ChronoDateTime(..) => "ChronoDateTime",
		Value::ChronoDateTimeUtc(..) => "ChronoDateTimeUtc",
		Value::ChronoDateTimeLocal(..) => "ChronoDateTimeLocal",
		Value::ChronoDateTimeWithTimeZone(..) => "ChronoDateTimeWithTimeZone",
		Value::Uuid(..) => "Uuid",
		Value::Json(..) => "Json",
		Value::Decimal(..) => "Decimal",
		Value::BigDecimal(..) => "BigDecimal",
		Value::Array(..) => "Array",
	}
}

fn error(backend: &str, index: usize, kind: &str, reason: &str) -> DatabaseError {
	DatabaseError::TypeError(format!(
		"cannot encode {kind} argument {index} for {backend}: {reason}"
	))
}

/// Compatibility fallback for external backends implementing only the raw API.
///
/// Native backends bypass this adapter. A legacy backend can receive only values
/// representable without content loss in QueryValue; other values fail closed.
pub(crate) fn legacy_values(values: Values, backend: &str) -> Result<Vec<QueryValue>> {
	values
		.into_iter()
		.enumerate()
		.map(|(offset, value)| {
			let kind = value_type(&value);
			let fail = |reason| error(backend, offset + 1, kind, reason);
			let converted = match value {
				Value::Bool(v) => v.map(QueryValue::Bool),
				Value::TinyInt(v) => v.map(|v| QueryValue::Int(i64::from(v))),
				Value::SmallInt(v) => v.map(|v| QueryValue::Int(i64::from(v))),
				Value::Int(v) => v.map(|v| QueryValue::Int(i64::from(v))),
				Value::BigInt(v) => v.map(QueryValue::Int),
				Value::TinyUnsigned(v) => v.map(|v| QueryValue::Int(i64::from(v))),
				Value::SmallUnsigned(v) => v.map(|v| QueryValue::Int(i64::from(v))),
				Value::Unsigned(v) => v.map(|v| QueryValue::Int(i64::from(v))),
				Value::BigUnsigned(v) => v
					.map(|v| i64::try_from(v).map(QueryValue::Int))
					.transpose()
					.map_err(|_| fail("unsigned integer exceeds signed 64-bit range"))?,
				Value::Float(v) => v.map(|v| QueryValue::Float(f64::from(v))),
				Value::Double(v) => v.map(QueryValue::Float),
				Value::Char(v) => v.map(|v| QueryValue::String(v.to_string())),
				Value::String(v) => v.map(|v| QueryValue::String(*v)),
				Value::Bytes(v) => v.map(|v| QueryValue::Bytes(*v)),
				Value::ChronoDateTimeUtc(v) => v.map(|v| QueryValue::Timestamp(*v)),
				Value::ChronoDateTimeLocal(v) => {
					v.map(|v| QueryValue::Timestamp(v.with_timezone(&chrono::Utc)))
				}
				Value::ChronoDateTimeWithTimeZone(v) => {
					v.map(|v| QueryValue::Timestamp(v.with_timezone(&chrono::Utc)))
				}
				Value::Uuid(v) => v.map(|v| QueryValue::Uuid(*v)),
				_ => return Err(fail("type requires a native generated-value codec")),
			};
			Ok(converted.unwrap_or(QueryValue::Null))
		})
		.collect()
}

pub(super) fn backend_name(backend: super::types::DatabaseType) -> &'static str {
	match backend {
		super::types::DatabaseType::Postgres => "postgres",
		super::types::DatabaseType::Mysql => "mysql",
		super::types::DatabaseType::Sqlite => "sqlite",
	}
}

/// Check native numeric and temporal limits before SQLx encoding, including
/// each element of a typed array. SQLx's encoders alone do not enforce every
/// server precision limit.
#[cfg(any(feature = "postgres", feature = "mysql"))]
fn validate_value(value: &Value, backend: &str, index: usize) -> Result<()> {
	if let Some(reason) = codec_loss(value, backend) {
		return Err(error(backend, index, value_type(value), reason));
	}
	Ok(())
}

#[cfg(any(feature = "postgres", feature = "mysql"))]
fn codec_loss(value: &Value, backend: &str) -> Option<&'static str> {
	use chrono::Timelike;
	let nanos = match value {
		Value::ChronoTime(Some(v)) => v.nanosecond(),
		Value::ChronoDateTime(Some(v)) => v.nanosecond(),
		Value::ChronoDateTimeUtc(Some(v)) => v.nanosecond(),
		Value::ChronoDateTimeLocal(Some(v)) => v.nanosecond(),
		Value::ChronoDateTimeWithTimeZone(Some(v)) => v.nanosecond(),
		Value::Array(_, Some(values)) => {
			return values.iter().find_map(|value| codec_loss(value, backend));
		}
		Value::BigDecimal(Some(decimal)) => {
			// Zero and redundant trailing digits do not enlarge the numeric value.
			let decimal = decimal.normalized();
			let scale = decimal.fractional_digit_count();
			let fractional = u64::try_from(scale).unwrap_or(0);
			let integer = if scale < 0 {
				decimal.digits().saturating_add(scale.unsigned_abs())
			} else {
				decimal.digits().saturating_sub(fractional)
			};
			return match backend {
				// PostgreSQL NUMERIC: 131072 integral and 16383 fractional digits.
				"postgres" if integer > 131_072 || fractional > 16_383 => {
					Some("decimal exceeds PostgreSQL numeric range")
				}
				// MySQL DECIMAL: at most 65 total digits and a scale of 30.
				"mysql" if integer.saturating_add(fractional) > 65 || fractional > 30 => {
					Some("decimal exceeds MySQL precision or scale")
				}
				_ => None,
			};
		}
		Value::Float(Some(value)) if backend == "mysql" && !value.is_finite() => {
			return Some("non-finite floats are unsupported");
		}
		Value::Double(Some(value)) if backend == "mysql" && !value.is_finite() => {
			return Some("non-finite floats are unsupported");
		}
		_ => return None,
	};
	if nanos >= 1_000_000_000 {
		Some("leap seconds are unsupported")
	} else if nanos % 1_000 != 0 {
		Some("sub-microsecond precision is unsupported")
	} else {
		None
	}
}

#[cfg(test)]
mod tests;
