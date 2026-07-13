//! Database field encoding and decoding contracts.

use std::fmt;

pub use crate::field_domain::{DatabaseStorageKind, FieldDomain, ModelEnumRepr, ModelEnumValue};

/// Canonical owned scalar value at the ORM database boundary.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum DatabaseValue {
	/// SQL null.
	Null,
	/// Boolean value.
	Bool(bool),
	/// 32-bit signed integer value.
	I32(i32),
	/// 64-bit signed integer value.
	I64(i64),
	/// 32-bit floating-point value.
	F32(f32),
	/// 64-bit floating-point value.
	F64(f64),
	/// UTF-8 string value.
	String(String),
	/// Binary byte value.
	Bytes(Vec<u8>),
	/// Native JSON value.
	Json(serde_json::Value),
	/// UUID value.
	Uuid(uuid::Uuid),
	/// Calendar date value.
	Date(chrono::NaiveDate),
	/// Time-of-day value.
	Time(chrono::NaiveTime),
	/// UTC timestamp value.
	DateTime(chrono::DateTime<chrono::Utc>),
}

impl DatabaseValue {
	/// Converts a scalar JSON value into its canonical database representation.
	pub fn try_from_json_value(value: serde_json::Value) -> Result<Self, FieldCodecError> {
		match value {
			serde_json::Value::Null => Ok(Self::Null),
			serde_json::Value::Bool(value) => Ok(Self::Bool(value)),
			serde_json::Value::Number(value) => {
				if let Some(value) = value.as_i64() {
					Ok(Self::I64(value))
				} else if let Some(value) = value.as_f64() {
					Ok(Self::F64(value))
				} else {
					Err(FieldCodecError::Serialization(format!(
						"JSON number {value} cannot be represented as a database scalar"
					)))
				}
			}
			serde_json::Value::String(value) => Ok(Self::String(value)),
			value @ (serde_json::Value::Array(_) | serde_json::Value::Object(_)) => {
				Ok(Self::Json(value))
			}
		}
	}

	/// Converts a database scalar into the legacy model-wide JSON bridge.
	pub fn into_json_value(self) -> Result<serde_json::Value, FieldCodecError> {
		match self {
			Self::Null => Ok(serde_json::Value::Null),
			Self::Bool(value) => Ok(serde_json::Value::Bool(value)),
			Self::I32(value) => Ok(serde_json::Value::Number(value.into())),
			Self::I64(value) => Ok(serde_json::Value::Number(value.into())),
			Self::F32(value) => json_number(f64::from(value)),
			Self::F64(value) => json_number(value),
			Self::String(value) => Ok(serde_json::Value::String(value)),
			Self::Bytes(value) => serde_json::to_value(value)
				.map_err(|error| FieldCodecError::Serialization(error.to_string())),
			Self::Json(value) => Ok(value),
			Self::Uuid(value) => Ok(serde_json::Value::String(value.to_string())),
			Self::Date(value) => Ok(serde_json::Value::String(value.to_string())),
			Self::Time(value) => Ok(serde_json::Value::String(value.to_string())),
			Self::DateTime(value) => Ok(serde_json::Value::String(value.to_rfc3339())),
		}
	}
}

/// Converts a canonical database value directly into the query builder carrier.
pub(crate) fn database_value_to_query_value(value: DatabaseValue) -> reinhardt_query::value::Value {
	use reinhardt_query::value::Value;

	match value {
		DatabaseValue::Null => Value::Int(None),
		DatabaseValue::Bool(value) => Value::Bool(Some(value)),
		DatabaseValue::I32(value) => Value::Int(Some(value)),
		DatabaseValue::I64(value) => Value::BigInt(Some(value)),
		DatabaseValue::F32(value) => Value::Float(Some(value)),
		DatabaseValue::F64(value) => Value::Double(Some(value)),
		DatabaseValue::String(value) => Value::String(Some(Box::new(value))),
		DatabaseValue::Bytes(value) => Value::Bytes(Some(Box::new(value))),
		DatabaseValue::Json(value) => Value::Json(Some(Box::new(value))),
		DatabaseValue::Uuid(value) => Value::Uuid(Some(Box::new(value))),
		DatabaseValue::Date(value) => Value::ChronoDate(Some(Box::new(value))),
		DatabaseValue::Time(value) => Value::ChronoTime(Some(Box::new(value))),
		DatabaseValue::DateTime(value) => Value::ChronoDateTimeUtc(Some(Box::new(value))),
	}
}

fn json_number(value: f64) -> Result<serde_json::Value, FieldCodecError> {
	serde_json::Number::from_f64(value)
		.map(serde_json::Value::Number)
		.ok_or_else(|| {
			FieldCodecError::Serialization(format!(
				"floating-point value {value} cannot be represented as JSON"
			))
		})
}

/// A framework-owned scalar supported by database backends.
pub trait DatabaseScalar: private::Sealed + Clone + Send + Sync + 'static {
	/// Physical storage kind used to read and bind this scalar.
	const STORAGE_KIND: DatabaseStorageKind;
	/// Converts this scalar into the canonical database carrier.
	fn into_database_value(self) -> DatabaseValue;
	/// Extracts this scalar from the canonical database carrier.
	fn from_database_value(value: DatabaseValue) -> Result<Self, FieldCodecError>;
}

/// Encoding contract for a typed model field.
pub trait DatabaseField:
	Clone + serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static
{
	/// Scalar type used at the database boundary.
	type Storage: DatabaseScalar;
	/// Largest enum string value accepted by this field, measured in characters.
	const MAX_STRING_VALUE_CHARS: Option<usize> = None;

	/// Encodes the typed field as its database storage value.
	fn encode_database(&self) -> Result<Self::Storage, FieldCodecError>;
	/// Decodes the database storage value as the typed field.
	fn decode_database(
		value: Self::Storage,
		context: &FieldCodecContext,
	) -> Result<Self, FieldCodecError>;
	/// Returns structured constraints associated with the field.
	fn domain() -> Option<FieldDomain> {
		None
	}
}

/// Metadata contract implemented by model enums.
pub trait ModelEnum: DatabaseField {
	/// Persistent scalar representation used by the enum.
	const REPR: ModelEnumRepr;
	/// Canonical persistent values declared by the enum.
	const VALUES: &'static [ModelEnumValueRef];
}

/// Allocation-free persistent value emitted by the model enum derive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelEnumValueRef {
	/// A borrowed static string enum value.
	String(&'static str),
	/// A 32-bit signed integer enum value.
	I32(i32),
}

/// Location of a field being decoded from a database row.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FieldCodecContext {
	/// Model name used in codec diagnostics.
	pub model: String,
	/// Rust field name used in codec diagnostics.
	pub field: String,
	/// Resolved database column name used in codec diagnostics.
	pub column: String,
}

impl FieldCodecContext {
	/// Creates codec context for a model field and resolved database column.
	pub fn new(
		model: impl Into<String>,
		field: impl Into<String>,
		column: impl Into<String>,
	) -> Self {
		Self {
			model: model.into(),
			field: field.into(),
			column: column.into(),
		}
	}
}

/// Failure to encode or decode a typed database field.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FieldCodecError {
	/// The database carrier does not match a field's storage type.
	TypeMismatch {
		/// Storage kind required by the field.
		expected: DatabaseStorageKind,
		/// Carrier received from the database boundary.
		actual: DatabaseValue,
	},
	/// A stored scalar is not a declared value of a model enum.
	InvalidEnumValue {
		/// Model field and database column being decoded.
		context: FieldCodecContext,
		/// Persistent representation used by the enum.
		repr: ModelEnumRepr,
		/// Rejected persistent value.
		value: ModelEnumValue,
	},
	/// Serde or JSON bridge conversion failed.
	Serialization(String),
}

impl FieldCodecError {
	/// Creates an invalid-enum error for a decoded database value.
	pub fn invalid_enum(
		context: FieldCodecContext,
		repr: ModelEnumRepr,
		value: ModelEnumValue,
	) -> Self {
		Self::InvalidEnumValue {
			context,
			repr,
			value,
		}
	}
}

impl fmt::Display for FieldCodecError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::TypeMismatch { expected, actual } => {
				write!(
					formatter,
					"expected {expected:?} database value, got {actual:?}"
				)
			}
			Self::InvalidEnumValue { context, value, .. } => write!(
				formatter,
				"invalid enum value '{}' for {}.{} from database column '{}'",
				EnumValueDisplay(value),
				context.model,
				context.field,
				context.column
			),
			Self::Serialization(message) => {
				write!(formatter, "field serialization failed: {message}")
			}
		}
	}
}

impl std::error::Error for FieldCodecError {}

struct EnumValueDisplay<'a>(&'a ModelEnumValue);

impl fmt::Display for EnumValueDisplay<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self.0 {
			ModelEnumValue::String(value) => formatter.write_str(value),
			ModelEnumValue::I32(value) => value.fmt(formatter),
		}
	}
}

macro_rules! impl_scalar_field {
	($type:ty, $kind:ident, $variant:ident) => {
		impl private::Sealed for $type {}

		impl DatabaseScalar for $type {
			const STORAGE_KIND: DatabaseStorageKind = DatabaseStorageKind::$kind;

			fn into_database_value(self) -> DatabaseValue {
				DatabaseValue::$variant(self)
			}

			fn from_database_value(value: DatabaseValue) -> Result<Self, FieldCodecError> {
				match value {
					DatabaseValue::$variant(value) => Ok(value),
					actual => Err(FieldCodecError::TypeMismatch {
						expected: Self::STORAGE_KIND,
						actual,
					}),
				}
			}
		}

		impl DatabaseField for $type {
			type Storage = Self;

			fn encode_database(&self) -> Result<Self::Storage, FieldCodecError> {
				Ok(self.clone())
			}

			fn decode_database(
				value: Self::Storage,
				_context: &FieldCodecContext,
			) -> Result<Self, FieldCodecError> {
				Ok(value)
			}
		}

		impl From<$type> for DatabaseValue {
			fn from(value: $type) -> Self {
				Self::$variant(value)
			}
		}
	};
}

impl_scalar_field!(bool, Bool, Bool);
impl_scalar_field!(i32, I32, I32);
impl_scalar_field!(i64, I64, I64);
impl_scalar_field!(f32, F32, F32);
impl_scalar_field!(f64, F64, F64);
impl_scalar_field!(String, String, String);
impl_scalar_field!(Vec<u8>, Bytes, Bytes);
impl_scalar_field!(serde_json::Value, Json, Json);
impl_scalar_field!(uuid::Uuid, Uuid, Uuid);
impl_scalar_field!(chrono::NaiveDate, Date, Date);
impl_scalar_field!(chrono::NaiveTime, Time, Time);
impl_scalar_field!(chrono::DateTime<chrono::Utc>, DateTime, DateTime);

impl<S: DatabaseScalar> private::Sealed for Option<S> {}

impl<S: DatabaseScalar> DatabaseScalar for Option<S> {
	const STORAGE_KIND: DatabaseStorageKind = S::STORAGE_KIND;

	fn into_database_value(self) -> DatabaseValue {
		self.map_or(DatabaseValue::Null, DatabaseScalar::into_database_value)
	}

	fn from_database_value(value: DatabaseValue) -> Result<Self, FieldCodecError> {
		match value {
			DatabaseValue::Null => Ok(None),
			value => S::from_database_value(value).map(Some),
		}
	}
}

impl<T: DatabaseField> DatabaseField for Option<T> {
	type Storage = Option<T::Storage>;
	const MAX_STRING_VALUE_CHARS: Option<usize> = T::MAX_STRING_VALUE_CHARS;

	fn encode_database(&self) -> Result<Self::Storage, FieldCodecError> {
		self.as_ref()
			.map(DatabaseField::encode_database)
			.transpose()
	}

	fn decode_database(
		value: Self::Storage,
		context: &FieldCodecContext,
	) -> Result<Self, FieldCodecError> {
		value
			.map(|value| T::decode_database(value, context))
			.transpose()
	}

	fn domain() -> Option<FieldDomain> {
		T::domain()
	}
}

impl<T> DatabaseField for super::Json<T>
where
	T: Clone + serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static,
{
	type Storage = serde_json::Value;

	fn encode_database(&self) -> Result<Self::Storage, FieldCodecError> {
		self.to_json_value()
			.map_err(|error| FieldCodecError::Serialization(error.to_string()))
	}

	fn decode_database(
		value: Self::Storage,
		_context: &FieldCodecContext,
	) -> Result<Self, FieldCodecError> {
		Self::from_json_value(value)
			.map_err(|error| FieldCodecError::Serialization(error.to_string()))
	}
}

mod private {
	pub trait Sealed {}
}

#[cfg(test)]
mod tests {
	use super::{
		DatabaseField, DatabaseValue, FieldCodecContext, FieldCodecError, ModelEnumRepr,
		ModelEnumValue,
	};

	#[test]
	fn string_database_field_round_trips() {
		let encoded = "queued".to_string().encode_database().unwrap();
		assert_eq!(encoded, "queued");
		assert_eq!(
			String::decode_database(encoded, &FieldCodecContext::new("Job", "status", "status"))
				.unwrap(),
			"queued"
		);
	}

	#[test]
	fn optional_database_field_preserves_null() {
		let encoded = Option::<i32>::None.encode_database().unwrap();
		assert_eq!(encoded, None);
		assert_eq!(
			Option::<i32>::decode_database(
				encoded,
				&FieldCodecContext::new("Job", "priority", "priority")
			)
			.unwrap(),
			None
		);
	}

	#[test]
	fn invalid_enum_error_names_the_field_and_column() {
		let error = FieldCodecError::invalid_enum(
			FieldCodecContext::new("AsyncJob", "status", "job_status"),
			ModelEnumRepr::String,
			ModelEnumValue::String("unknown".to_string()),
		);
		assert_eq!(
			error.to_string(),
			"invalid enum value 'unknown' for AsyncJob.status from database column 'job_status'"
		);
	}

	#[test]
	fn database_value_keeps_i32_width() {
		assert_eq!(DatabaseValue::from(7_i32), DatabaseValue::I32(7));
	}

	#[test]
	fn uuid_shaped_database_string_binds_as_string() {
		let value = super::database_value_to_query_value(DatabaseValue::String(
			"550e8400-e29b-41d4-a716-446655440000".to_string(),
		));
		assert!(matches!(
			value,
			reinhardt_query::value::Value::String(Some(text))
				if text.as_ref() == "550e8400-e29b-41d4-a716-446655440000"
		));
	}

	#[test]
	fn database_bytes_bind_as_bytes() {
		let value =
			super::database_value_to_query_value(DatabaseValue::Bytes(vec![0, 1, 127, 255]));
		assert!(matches!(
			value,
			reinhardt_query::value::Value::Bytes(Some(bytes))
				if bytes.as_ref() == &[0, 1, 127, 255]
		));
	}
}
