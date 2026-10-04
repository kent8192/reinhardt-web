//! Owned generated arguments and checked compatibility for custom backends.

use super::error::{DatabaseError, DatabaseErrorKind, Result};
use super::types::{DatabaseType, QueryValue};
use reinhardt_query::{Value, Values};

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
			Value::BigUnsigned(Some(value)) => {
				QueryValue::Int(i64::try_from(value).map_err(|_| {
					compatibility_error(
						backend,
						index,
						"BigUnsigned",
						"unsigned integer exceeds signed 64-bit range",
					)
				})?)
			}
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

pub(crate) fn binding_error(error: reinhardt_query_sqlx::BindError) -> DatabaseError {
	DatabaseError::new(DatabaseErrorKind::Type, error.to_string()).with_source(error)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::backends::backend::DatabaseBackend;
	use crate::backends::types::{QueryResult, Row, TransactionExecutor};
	use rstest::rstest;
	use std::sync::{Arc, Mutex};

	type CapturedCalls = Arc<Mutex<Vec<(String, Vec<QueryValue>)>>>;

	struct CustomBackend {
		calls: CapturedCalls,
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
			panic!("custom generated-query fixture does not start transactions")
		}
		fn as_any(&self) -> &dyn std::any::Any {
			self
		}
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
					Values(vec![Value::BigUnsigned(Some(u64::MAX))]),
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
	fn custom_bridge_preserves_pair_order_and_rejects_unsigned_overflow() {
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
		let error = compatibility_arguments(
			(
				"private SQL".to_owned(),
				Values(vec![
					Value::String(Some(Box::new("private payload".to_owned()))),
					Value::BigUnsigned(Some(u64::MAX)),
				]),
			),
			DatabaseType::Mysql,
		)
		.unwrap_err();
		let message = error.to_string();
		assert!(message.contains("BigUnsigned argument 2 for mysql/custom"));
		assert!(!message.contains("private"));
		assert!(!message.contains(&u64::MAX.to_string()));
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
