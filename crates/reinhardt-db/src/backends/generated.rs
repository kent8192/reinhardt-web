//! Argument codecs for renderer-generated SQL, separate from the public raw API.
//!
//! Values retain their type and order until native SQLx encoding. Encoding errors
//! contain only the backend, type, one-based argument index, and a fixed reason.
//! BigDecimal values use a normalized representation for range checks and encoding.

use reinhardt_query::{Value, Values};

use super::{
	DatabaseError, DatabaseErrorKind, Result,
	types::{DatabaseType, QueryValue},
};

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
		#[cfg(feature = "pgvector")]
		Value::Vector(..) => "Vector",
	}
}

fn error(
	backend: &str,
	index: usize,
	kind: &str,
	reason: &str,
) -> reinhardt_core::exception::Error {
	DatabaseError::new(
		DatabaseErrorKind::Type,
		format!("cannot encode {kind} argument {index} for {backend}: {reason}"),
	)
	.into()
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
				Value::Int(v) => v.map(QueryValue::Int32),
				Value::BigInt(v) => v.map(QueryValue::Int),
				Value::TinyUnsigned(v) => v.map(|v| QueryValue::Int(i64::from(v))),
				Value::SmallUnsigned(v) => v.map(|v| QueryValue::Int(i64::from(v))),
				Value::Unsigned(v) => v.map(|v| QueryValue::Int(i64::from(v))),
				Value::BigUnsigned(v) => v.map(QueryValue::Uint),
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
				Value::Array(ty, values) => {
					Some(legacy_array(ty, values.map(|v| *v), backend, offset + 1)?)
				}
				Value::Json(v) => Some(QueryValue::Json(v)),
				Value::ChronoDateTime(v) => v.map(|v| QueryValue::NaiveTimestamp(*v)),
				#[cfg(feature = "pgvector")]
				Value::Vector(v) => Some(QueryValue::Vector(v.map(|v| *v))),
				_ => return Err(fail("type requires a native generated-value codec")),
			};
			Ok(converted.unwrap_or(QueryValue::Null))
		})
		.collect()
}

fn legacy_array(
	ty: reinhardt_query::ArrayType,
	values: Option<Vec<Value>>,
	backend: &str,
	index: usize,
) -> Result<QueryValue> {
	use super::types::array_query_value;
	use reinhardt_query::ArrayType;
	let fail = || {
		error(
			backend,
			index,
			"Array",
			"array element does not match declared element type",
		)
	};
	macro_rules! array {
		($variant:ident, $convert:expr, $full:ident, $nullable:ident) => {{
			let values = values
				.map(|values| {
					values
						.into_iter()
						.map(|value| match value {
							Value::Int(None) => Ok(None),
							Value::$variant(value) => Ok(value.map($convert)),
							_ => Err(fail()),
						})
						.collect::<Result<Vec<_>>>()
				})
				.transpose()?;
			Ok(array_query_value(
				values,
				QueryValue::$full,
				QueryValue::$nullable,
			))
		}};
	}
	match ty {
		ArrayType::String => array!(String, |value| *value, StringArray, NullableStringArray),
		ArrayType::Int => array!(Int, |value| value, IntArray, NullableIntArray),
		ArrayType::BigInt => array!(BigInt, |value| value, BigIntArray, NullableBigIntArray),
		ArrayType::Bool => array!(Bool, |value| value, BoolArray, NullableBoolArray),
		ArrayType::Float => array!(Float, |value| value, FloatArray, NullableFloatArray),
		ArrayType::Double => array!(Double, |value| value, DoubleArray, NullableDoubleArray),
		ArrayType::Uuid => array!(Uuid, |value| *value, UuidArray, NullableUuidArray),
		_ => Err(error(
			backend,
			index,
			"Array",
			"type requires a native generated-value codec",
		)),
	}
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

fn compatibility_error(
	backend: DatabaseType,
	index: usize,
	kind: &str,
	reason: &str,
) -> DatabaseError {
	let backend = match backend {
		DatabaseType::Postgres => "postgres/custom",
		DatabaseType::Mysql => "mysql/custom",
		DatabaseType::Sqlite => "sqlite/custom",
	};
	DatabaseError::new(
		DatabaseErrorKind::Type,
		format!("cannot encode {kind} argument {index} for {backend}: {reason}"),
	)
}

/// Bridge generated values into an existing custom backend's raw parameter API.
/// Native SQLx implementations bypass this bridge and preserve their full codecs.
pub(crate) fn compatibility_arguments(
	built: (String, Values),
	backend: DatabaseType,
) -> Result<(String, Vec<QueryValue>)> {
	let (sql, values) = built;
	let mut arguments = Vec::with_capacity(values.0.len());
	for (offset, value) in values.into_iter().enumerate() {
		let index = offset + 1;
		let unsupported = |kind| {
			compatibility_error(
				backend,
				index,
				kind,
				"custom backend must implement generated execution for this value type",
			)
		};
		let argument = match value {
			Value::Json(value) => QueryValue::Json(value),
			#[cfg(feature = "pgvector")]
			Value::Vector(value) => QueryValue::Vector(value.map(|value| *value)),
			Value::Array(..) => return Err(unsupported("Array").into()),
			Value::ChronoDate(..) => return Err(unsupported("ChronoDate").into()),
			Value::ChronoTime(..) => return Err(unsupported("ChronoTime").into()),
			Value::Decimal(..) => return Err(unsupported("Decimal").into()),
			Value::BigDecimal(..) => return Err(unsupported("BigDecimal").into()),
			value if value.is_null() => QueryValue::Null,
			Value::Bool(Some(value)) => QueryValue::Bool(value),
			Value::TinyInt(Some(value)) => QueryValue::Int(i64::from(value)),
			Value::SmallInt(Some(value)) => QueryValue::Int(i64::from(value)),
			Value::Int(Some(value)) => QueryValue::Int32(value),
			Value::BigInt(Some(value)) => QueryValue::Int(value),
			Value::TinyUnsigned(Some(value)) => QueryValue::Int(i64::from(value)),
			Value::SmallUnsigned(Some(value)) => QueryValue::Int(i64::from(value)),
			Value::Unsigned(Some(value)) => QueryValue::Int(i64::from(value)),
			Value::BigUnsigned(Some(value)) => QueryValue::Uint(value),
			Value::Float(Some(value)) => QueryValue::Float(f64::from(value)),
			Value::Double(Some(value)) => QueryValue::Float(value),
			Value::Char(Some(value)) => QueryValue::String(value.to_string()),
			Value::String(Some(value)) => QueryValue::String(*value),
			Value::Bytes(Some(value)) => QueryValue::Bytes(*value),
			Value::ChronoDateTimeUtc(Some(value)) => QueryValue::Timestamp(*value),
			Value::ChronoDateTimeLocal(Some(value)) => {
				QueryValue::Timestamp(value.with_timezone(&chrono::Utc))
			}
			Value::ChronoDateTimeWithTimeZone(Some(value)) => {
				QueryValue::Timestamp(value.with_timezone(&chrono::Utc))
			}
			Value::ChronoDateTime(Some(value)) => QueryValue::NaiveTimestamp(*value),
			Value::Uuid(Some(value)) => QueryValue::Uuid(*value),
			// NULL variants are handled by is_null() above. Keeping this catch-all
			// avoids feature-unified variants becoming a debug/string fallback.
			_ => return Err(unsupported("Value").into()),
		};
		arguments.push(argument);
	}
	Ok((sql, arguments))
}

/// Build an unnamed query after removing potentially incompatible named statements.
#[cfg(feature = "postgres")]
pub(crate) async fn uncached_postgres_query<'q>(
	connection: &mut sqlx::PgConnection,
	sql: &'q str,
	arguments: sqlx::postgres::PgArguments,
) -> std::result::Result<
	sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
	sqlx::Error,
> {
	use sqlx::Connection;
	if connection.cached_statements_size() > 0 {
		connection.clear_cached_statements().await?;
	}
	// Workaround: https://github.com/kent8192/reinhardt-web/issues/6533
	// SQLx 0.8 reuses SQL-only cache entries before checking parameter types or
	// persistence. Clear existing entries above and keep generated queries unnamed.
	// Remove this bypass after a validated driver keys or invalidates by argument type.
	Ok(sqlx::query_with(sql, arguments).persistent(false))
}

#[cfg(test)]
mod compatibility_tests {
	use super::*;
	use crate::backends::backend::DatabaseBackend;
	use crate::backends::types::{QueryResult, Row, RowStream, TransactionExecutor};
	use futures::StreamExt;
	use rstest::rstest;
	use std::sync::{Arc, Mutex};

	type CapturedCalls = Arc<Mutex<Vec<(String, Vec<QueryValue>)>>>;

	struct CustomBackend {
		calls: CapturedCalls,
	}
	struct CustomTransaction(CustomBackend);

	#[async_trait::async_trait]
	impl TransactionExecutor for CustomTransaction {
		async fn execute(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<QueryResult> {
			self.0.execute(sql, params).await
		}
		async fn fetch_one(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Row> {
			self.0.fetch_one(sql, params).await
		}
		async fn fetch_all(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Vec<Row>> {
			self.0.fetch_all(sql, params).await
		}
		async fn fetch_optional(
			&mut self,
			sql: &str,
			params: Vec<QueryValue>,
		) -> Result<Option<Row>> {
			self.0.fetch_optional(sql, params).await
		}
		fn fetch_stream<'a>(
			&'a mut self,
			sql: String,
			params: Vec<QueryValue>,
			chunk_size: usize,
		) -> Result<RowStream<'a>> {
			self.0.fetch_stream(sql, params, chunk_size)
		}
		async fn commit(self: Box<Self>) -> Result<()> {
			Ok(())
		}
		async fn rollback(self: Box<Self>) -> Result<()> {
			Ok(())
		}
	}

	#[async_trait::async_trait]
	impl DatabaseBackend for CustomBackend {
		fn database_type(&self) -> DatabaseType {
			DatabaseType::Postgres
		}
		fn placeholder(&self, index: usize) -> String {
			format!("${index}")
		}
		fn supports_returning(&self) -> bool {
			true
		}
		fn supports_on_conflict(&self) -> bool {
			true
		}
		async fn execute(&self, sql: &str, params: Vec<QueryValue>) -> Result<QueryResult> {
			self.calls.lock().unwrap().push((sql.to_owned(), params));
			Ok(QueryResult {
				rows_affected: 1,
				last_insert_id: Some(7),
			})
		}
		async fn fetch_one(&self, sql: &str, params: Vec<QueryValue>) -> Result<Row> {
			self.calls.lock().unwrap().push((sql.to_owned(), params));
			let mut row = Row::new();
			row.insert("id".to_owned(), QueryValue::Int(7));
			Ok(row)
		}
		async fn fetch_all(&self, sql: &str, params: Vec<QueryValue>) -> Result<Vec<Row>> {
			Ok(vec![self.fetch_one(sql, params).await?])
		}
		async fn fetch_optional(&self, sql: &str, params: Vec<QueryValue>) -> Result<Option<Row>> {
			Ok(Some(self.fetch_one(sql, params).await?))
		}
		async fn begin(&self) -> Result<Box<dyn TransactionExecutor>> {
			Ok(Box::new(CustomTransaction(CustomBackend {
				calls: self.calls.clone(),
			})))
		}
		fn fetch_stream<'a>(
			&'a self,
			sql: String,
			params: Vec<QueryValue>,
			_chunk_size: usize,
		) -> Result<RowStream<'a>> {
			self.calls.lock().unwrap().push((sql, params));
			let mut row = Row::new();
			row.insert("id".to_owned(), QueryValue::Int(7));
			Ok(Box::pin(futures::stream::once(async move { Ok(row) })))
		}
		fn as_any(&self) -> &dyn std::any::Any {
			self
		}
	}

	#[rstest]
	#[case(0)]
	#[case(i64::MAX as u64 + 1)]
	#[case(u64::MAX)]
	fn custom_backend_bridge_preserves_unsigned_values(#[case] value: u64) {
		// Arrange
		let built = (
			"SELECT ?".to_owned(),
			Values(vec![Value::BigUnsigned(Some(value))]),
		);
		// Act
		let (sql, arguments) = compatibility_arguments(built, DatabaseType::Mysql).unwrap();
		// Assert
		assert_eq!(sql, "SELECT ?");
		assert_eq!(arguments, vec![QueryValue::Uint(value)]);
	}

	#[rstest]
	#[tokio::test]
	async fn existing_custom_backend_works_without_implementing_new_generated_methods() {
		// Arrange: this provider implements only the pre-existing required raw methods.
		let calls: CapturedCalls = Arc::new(Mutex::new(Vec::new()));
		let connection = crate::backends::DatabaseConnection::new(Arc::new(CustomBackend {
			calls: calls.clone(),
		}));
		let sql = "SELECT $1 AS id, $2 AS name";
		let built = || {
			(
				sql.to_owned(),
				Values(vec![
					Value::Int(Some(7)),
					Value::String(Some(Box::new("quoted' payload".to_owned()))),
				]),
			)
		};
		// Act
		let result = connection.execute_generated(built(), None).await.unwrap();
		let one = connection.fetch_one_generated(built(), None).await.unwrap();
		let all = connection.fetch_all_generated(built(), None).await.unwrap();
		let optional = connection
			.fetch_optional_generated(built(), None)
			.await
			.unwrap();
		let failed = connection
			.execute_generated(
				(
					"private SQL".to_owned(),
					Values(vec![Value::ChronoDate(None)]),
				),
				None,
			)
			.await;
		// Assert: metadata, row shapes and exact pairs reach the original raw seam.
		assert_eq!(result.rows_affected, 1);
		assert_eq!(result.last_insert_id, Some(7));
		assert_eq!(one.get::<i64>("id").unwrap(), 7);
		assert_eq!(all, [one.clone()]);
		assert_eq!(optional, Some(one));
		assert_eq!(
			failed.unwrap_err().database_kind(),
			Some(DatabaseErrorKind::Type)
		);
		assert_eq!(
			*calls.lock().unwrap(),
			vec![
				(
					sql.to_owned(),
					vec![
						QueryValue::Int32(7),
						QueryValue::String("quoted' payload".to_owned())
					]
				);
				4
			]
		);
	}

	#[rstest]
	#[tokio::test]
	async fn existing_custom_transaction_and_stream_defaults_preserve_raw_parameters() {
		// Arrange: no generated methods are implemented by this legacy provider.
		let calls: CapturedCalls = Arc::new(Mutex::new(Vec::new()));
		let connection = crate::backends::DatabaseConnection::new(Arc::new(CustomBackend {
			calls: calls.clone(),
		}));
		let sql = "SELECT $1 AS id";
		let built = || (sql.to_owned(), Values(vec![Value::Int(Some(7))]));
		let mut transaction = connection.begin().await.unwrap();
		// Act
		let result = transaction.execute_generated(built(), None).await.unwrap();
		let row = transaction
			.fetch_one_generated(built(), None)
			.await
			.unwrap();
		assert_eq!(
			transaction
				.fetch_all_generated(built(), None)
				.await
				.unwrap(),
			vec![row.clone()]
		);
		assert_eq!(
			transaction
				.fetch_optional_generated(built(), None)
				.await
				.unwrap(),
			Some(row.clone())
		);
		{
			let mut stream = transaction
				.fetch_stream_generated(built(), 1, None)
				.unwrap();
			assert_eq!(stream.next().await.unwrap().unwrap(), row);
			assert!(stream.next().await.is_none());
		}
		let error = transaction
			.execute_generated(
				(
					"private SQL".to_owned(),
					Values(vec![Value::ChronoDate(None)]),
				),
				None,
			)
			.await
			.unwrap_err();
		transaction.rollback().await.unwrap();
		{
			let mut stream = connection.fetch_stream_generated(built(), 1, None).unwrap();
			assert_eq!(
				stream
					.next()
					.await
					.unwrap()
					.unwrap()
					.get::<i64>("id")
					.unwrap(),
				7
			);
		}
		// Assert: failed encoding never reaches the raw provider.
		assert_eq!(result.rows_affected, 1);
		assert_eq!(result.last_insert_id, Some(7));
		assert_eq!(error.database_kind(), Some(DatabaseErrorKind::Type));
		assert_eq!(
			*calls.lock().unwrap(),
			vec![(sql.to_owned(), vec![QueryValue::Int32(7)]); 6]
		);
	}

	#[rstest]
	fn custom_bridge_preserves_pair_order_and_unsigned_values() {
		// Arrange
		let sql = "SELECT ? AS amount, ? AS name";
		let built = (
			sql.to_owned(),
			Values(vec![
				Value::Int(Some(7)),
				Value::String(Some(Box::new("quoted' payload".to_owned()))),
			]),
		);
		// Act
		let (actual, values) = compatibility_arguments(built, DatabaseType::Sqlite).unwrap();
		// Assert
		assert_eq!(actual, sql);
		assert_eq!(
			values,
			[
				QueryValue::Int32(7),
				QueryValue::String("quoted' payload".to_owned())
			]
		);
		let unsigned = compatibility_arguments(
			(
				"private SQL".to_owned(),
				Values(vec![
					Value::String(Some(Box::new("private payload".to_owned()))),
					Value::BigUnsigned(Some(u64::MAX)),
				]),
			),
			DatabaseType::Mysql,
		)
		.unwrap();
		assert_eq!(
			unsigned,
			(
				"private SQL".into(),
				vec![
					QueryValue::String("private payload".into()),
					QueryValue::Uint(u64::MAX)
				]
			)
		);
	}

	#[rstest]
	#[case(
		Value::Decimal(Some(Box::new(rust_decimal::Decimal::new(12345, 4)))),
		"Decimal"
	)]
	#[case(Value::Array(reinhardt_query::ArrayType::Int, Some(Box::new(vec![Value::Int(Some(1))]))), "Array")]
	fn custom_bridge_rejects_unrepresentable_types_without_stringifying(
		#[case] value: Value,
		#[case] kind: &str,
	) {
		// Act
		let error = compatibility_arguments(
			("private SQL".to_owned(), Values(vec![value])),
			DatabaseType::Postgres,
		)
		.unwrap_err();
		// Assert
		assert_eq!(error.database_kind(), Some(DatabaseErrorKind::Type));
		let message = error.to_string();
		assert!(message.contains(&format!("{kind} argument 1 for postgres/custom")));
		assert!(!message.contains("private SQL"));
		assert!(!message.contains("12345"));
	}
}
