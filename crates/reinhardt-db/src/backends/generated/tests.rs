use super::*;
use reinhardt_query::ArrayType;
#[cfg(any(feature = "sqlite", feature = "mysql", feature = "postgres"))]
use reinhardt_query::{Expr, Query, QueryStatementBuilder};
use rstest::rstest;

#[rstest]
#[case(
	Value::BigUnsigned(Some(u64::MAX)),
	"BigUnsigned",
	"unsigned integer exceeds signed 64-bit range"
)]
#[case(Value::Decimal(Some(Box::new("123456789.123456789".parse().unwrap()))), "Decimal", "type requires a native generated-value codec")]
#[case(Value::Array(ArrayType::String, Some(Box::new(vec!["private payload".into()]))), "Array", "type requires a native generated-value codec")]
#[case(
	Value::ChronoDate(None),
	"ChronoDate",
	"type requires a native generated-value codec"
)]
fn legacy_fallback_rejects_loss_with_redacted_position(
	#[case] value: Value,
	#[case] kind: &str,
	#[case] reason: &str,
) {
	// Arrange
	let values = Values(vec![Value::Int(Some(7)), value]);

	// Act
	let error = legacy_values(values, "postgres").unwrap_err();

	// Assert
	assert_eq!(error, super::error("postgres", 2, kind, reason));
}

#[rstest]
#[case::positive(9 * 3600, "2026-10-03T16:02:03.123456789Z")]
#[case::negative(-7 * 3600, "2026-10-05T06:02:03.987654321Z")]
fn legacy_fallback_preserves_zoned_timestamps_and_nulls(
	#[case] offset_seconds: i32,
	#[case] timestamp: &str,
) {
	// Arrange
	let utc = timestamp.parse::<chrono::DateTime<chrono::Utc>>().unwrap();
	let offset = chrono::FixedOffset::east_opt(offset_seconds).unwrap();
	let values = Values(vec![
		Value::Int(Some(7)),
		Value::ChronoDateTimeLocal(Some(Box::new(chrono::DateTime::from_naive_utc_and_offset(
			utc.naive_utc(),
			offset,
		)))),
		Value::ChronoDateTimeWithTimeZone(Some(Box::new(utc.with_timezone(&offset)))),
		Value::ChronoDateTimeLocal(None),
		Value::ChronoDateTimeWithTimeZone(None),
		Value::ChronoDateTimeUtc(Some(Box::new(utc))),
	]);

	// Act
	let params = legacy_values(values, "sqlite").unwrap();

	// Assert
	assert_eq!(
		params,
		vec![
			QueryValue::Int(7),
			QueryValue::Timestamp(utc),
			QueryValue::Timestamp(utc),
			QueryValue::Null,
			QueryValue::Null,
			QueryValue::Timestamp(utc),
		]
	);
}

#[cfg(feature = "sqlite")]
mod raw_only_backend {
	use super::*;
	use crate::backends::{
		DatabaseBackend, DatabaseType, QueryResult, Row, SqliteBackend, TransactionExecutor,
	};
	use reinhardt_query::{ColumnDef, ExprTrait, QueryBuilder, SqliteQueryBuilder};
	use rstest::fixture;

	// This external-style adapter implements only the public raw executor API.
	struct RawOnlyBackend(SqliteBackend);

	#[async_trait::async_trait]
	impl DatabaseBackend for RawOnlyBackend {
		fn database_type(&self) -> DatabaseType {
			self.0.database_type()
		}
		fn placeholder(&self, index: usize) -> String {
			self.0.placeholder(index)
		}
		fn supports_returning(&self) -> bool {
			self.0.supports_returning()
		}
		fn supports_on_conflict(&self) -> bool {
			self.0.supports_on_conflict()
		}
		async fn execute(&self, sql: &str, params: Vec<QueryValue>) -> Result<QueryResult> {
			self.0.execute(sql, params).await
		}
		async fn fetch_one(&self, sql: &str, params: Vec<QueryValue>) -> Result<Row> {
			self.0.fetch_one(sql, params).await
		}
		async fn fetch_all(&self, sql: &str, params: Vec<QueryValue>) -> Result<Vec<Row>> {
			self.0.fetch_all(sql, params).await
		}
		async fn fetch_optional(&self, sql: &str, params: Vec<QueryValue>) -> Result<Option<Row>> {
			self.0.fetch_optional(sql, params).await
		}
		async fn begin(&self) -> Result<Box<dyn TransactionExecutor>> {
			self.0.begin().await
		}
		fn as_any(&self) -> &dyn std::any::Any {
			self
		}
	}

	#[fixture]
	async fn backend() -> RawOnlyBackend {
		let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
		RawOnlyBackend(SqliteBackend::new(pool))
	}

	#[rstest]
	#[case::fixed_positive(false, 9 * 3600, Some("2026-10-03T16:02:03.123456789Z"))]
	#[case::fixed_negative(false, -7 * 3600, Some("2026-10-05T06:02:03.987654321Z"))]
	#[case::local_positive(true, 9 * 3600, Some("2026-10-03T16:02:03.123456789Z"))]
	#[case::local_negative(true, -7 * 3600, Some("2026-10-05T06:02:03.987654321Z"))]
	#[case::fixed_null(false, 0, None)]
	#[case::local_null(true, 0, None)]
	#[tokio::test]
	async fn generated_defaults_preserve_zoned_timestamps(
		#[future] backend: RawOnlyBackend,
		#[case] local: bool,
		#[case] offset_seconds: i32,
		#[case] timestamp: Option<&str>,
	) {
		// Arrange
		let backend = backend.await;
		let utc =
			timestamp.map(|timestamp| timestamp.parse::<chrono::DateTime<chrono::Utc>>().unwrap());
		let offset = chrono::FixedOffset::east_opt(offset_seconds).unwrap();
		let value = if local {
			Value::ChronoDateTimeLocal(utc.map(|utc| {
				Box::new(chrono::DateTime::from_naive_utc_and_offset(
					utc.naive_utc(),
					offset,
				))
			}))
		} else {
			Value::ChronoDateTimeWithTimeZone(utc.map(|utc| Box::new(utc.with_timezone(&offset))))
		};
		let create = Query::create_table()
			.table("timestamp_probe")
			.col(ColumnDef::new("id").integer().primary_key(true))
			.col(ColumnDef::new("timestamp").timestamp())
			.to_owned();
		let (sql, _) = SqliteQueryBuilder.build_create_table(&create);
		backend.execute(&sql, vec![]).await.unwrap();
		let (insert_sql, insert_values) = Query::insert()
			.into_table("timestamp_probe")
			.columns(["id", "timestamp"])
			.values_panic([Value::Int(Some(7)), value.clone()])
			.build(SqliteQueryBuilder);
		let (select_sql, select_values) = Query::select()
			.from("timestamp_probe")
			.columns(["id", "timestamp"])
			.expr_as(Expr::val(value), "bound_timestamp")
			.and_where(Expr::col("id").eq(7))
			.build(SqliteQueryBuilder);

		// Act: each method uses the trait's default generated-value adapter.
		let result = backend
			.__execute_generated(&insert_sql, insert_values)
			.await
			.unwrap();
		let row = backend
			.__fetch_one_generated(&select_sql, select_values.clone())
			.await
			.unwrap();
		let rows = backend
			.__fetch_all_generated(&select_sql, select_values)
			.await
			.unwrap();

		// Assert
		let expected = utc
			.map(|utc| QueryValue::String(utc.to_rfc3339()))
			.unwrap_or(QueryValue::Null);
		assert_eq!(result.rows_affected, 1);
		assert_eq!(row.data["id"], QueryValue::Int(7));
		assert_eq!(row.data["timestamp"], expected);
		assert_eq!(row.data["bound_timestamp"], expected);
		assert_eq!(rows, vec![row]);
	}
}

#[cfg(feature = "postgres")]
#[rstest]
#[case(
	Value::BigUnsigned(Some(u64::MAX)),
	"BigUnsigned",
	"unsigned integer exceeds signed 64-bit range"
)]
#[case(Value::Array(ArrayType::Int, Some(Box::new(vec!["private payload".into()]))), "Array", "array element does not match declared element type")]
#[case(Value::Array(ArrayType::BigUnsigned, Some(Box::new(vec![Value::BigUnsigned(Some(u64::MAX))]))), "Array", "unsigned array element exceeds signed 64-bit range")]
#[case(Value::BigDecimal(Some(Box::new("1e-40000".parse().unwrap()))), "BigDecimal", "decimal exceeds PostgreSQL numeric range")]
#[case(Value::BigDecimal(Some(Box::new("1e131072".parse().unwrap()))), "BigDecimal", "decimal exceeds PostgreSQL numeric range")]
#[case(Value::BigDecimal(Some(Box::new("100e-16386".parse().unwrap()))), "BigDecimal", "decimal exceeds PostgreSQL numeric range")]
#[case(
	Value::Array(ArrayType::BigDecimal, Some(Box::new(vec![Value::from(
		"100e-16386".parse::<sqlx::types::BigDecimal>().unwrap()
	)]))),
	"Array", "decimal exceeds PostgreSQL numeric range"
)]
fn postgres_arguments_reject_loss_before_execution(
	#[case] value: Value,
	#[case] kind: &str,
	#[case] reason: &str,
) {
	// Arrange
	let values = Values(vec![Value::Int(Some(7)), value]);

	// Act
	let error = postgres::arguments(values).err().unwrap();

	// Assert
	assert_eq!(error, super::error("postgres", 2, kind, reason));
}

#[cfg(feature = "sqlite")]
#[rstest]
#[case(
	Value::BigUnsigned(Some(u64::MAX)),
	"BigUnsigned",
	"unsigned integer exceeds signed 64-bit range"
)]
#[case(
	Value::Decimal(None),
	"Decimal",
	"type has no supported codec for this backend"
)]
#[case(
	Value::BigDecimal(None),
	"BigDecimal",
	"type has no supported codec for this backend"
)]
#[case(
	Value::Array(ArrayType::String, None),
	"Array",
	"arrays require a PostgreSQL native codec"
)]
#[case(Value::Double(Some(f64::NAN)), "Double", "NaN would become SQL NULL")]
fn sqlite_arguments_reject_loss_before_execution(
	#[case] value: Value,
	#[case] kind: &str,
	#[case] reason: &str,
) {
	// Arrange
	let values = Values(vec![Value::Int(Some(7)), value]);

	// Act
	let error = sqlite::arguments(values).err().unwrap();

	// Assert
	assert_eq!(error, super::error("sqlite", 2, kind, reason));
}

#[cfg(feature = "mysql")]
#[rstest]
fn mysql_rejects_arrays_and_retains_unsigned_range() {
	// Arrange
	let array = Values(vec![Value::Array(ArrayType::String, None)]);
	let integer = Values(vec![Value::BigUnsigned(Some(u64::MAX))]);

	// Act
	let error = mysql::arguments(array).err().unwrap();
	let arguments = mysql::arguments(integer).unwrap();

	// Assert
	assert_eq!(
		error,
		super::error(
			"mysql",
			1,
			"Array",
			"arrays require a PostgreSQL native codec"
		)
	);
	assert_eq!(sqlx::Arguments::len(&arguments), 1);
}

#[cfg(feature = "mysql")]
#[rstest]
#[case("0e65")]
#[case("0e-65")]
#[case("-0e65")]
#[case("1.2300000000000000000000000000000000")]
#[case("100e-32")]
#[case("1e64")]
#[case("1e-30")]
fn mysql_accepts_equivalent_big_decimal_representations(#[case] input: &str) {
	// Arrange
	let value = Value::BigDecimal(Some(Box::new(input.parse().unwrap())));

	// Act
	let arguments = mysql::arguments(Values(vec![value])).unwrap();

	// Assert
	assert_eq!(sqlx::Arguments::len(&arguments), 1);
}

#[cfg(feature = "postgres")]
#[rstest]
#[case("0e131072")]
#[case("0e-40000")]
#[case("-0e131072")]
#[case(&format!("1.23{}", "0".repeat(40000)))]
#[case("1e131071")]
#[case("1e-16383")]
fn postgres_accepts_equivalent_big_decimal_representations(#[case] input: &str) {
	// Arrange
	let decimal: sqlx::types::BigDecimal = input.parse().unwrap();
	let values = Values(vec![
		decimal.clone().into(),
		Value::Array(
			ArrayType::BigDecimal,
			Some(Box::new(vec![decimal.into(), Value::BigDecimal(None)])),
		),
	]);

	// Act
	let arguments = postgres::arguments(values).unwrap();

	// Assert
	assert_eq!(sqlx::Arguments::len(&arguments), 2);
}

#[cfg(feature = "mysql")]
#[rstest]
#[case(Value::BigDecimal(Some(Box::new("1e65".parse().unwrap()))), "BigDecimal", "decimal exceeds MySQL precision or scale")]
#[case(Value::BigDecimal(Some(Box::new("1e-31".parse().unwrap()))), "BigDecimal", "decimal exceeds MySQL precision or scale")]
#[case(Value::BigDecimal(Some(Box::new("100e-33".parse().unwrap()))), "BigDecimal", "decimal exceeds MySQL precision or scale")]
#[case(
	Value::Double(Some(f64::INFINITY)),
	"Double",
	"non-finite floats are unsupported"
)]
fn mysql_rejects_numeric_values_outside_native_range(
	#[case] value: Value,
	#[case] kind: &str,
	#[case] reason: &str,
) {
	// Arrange
	let values = Values(vec![Value::Int(Some(7)), value]);

	// Act
	let error = mysql::arguments(values).err().unwrap();

	// Assert
	assert_eq!(error, super::error("mysql", 2, kind, reason));
}

#[cfg(any(feature = "postgres", feature = "mysql"))]
fn temporal_value(nanos: u32) -> Value {
	Value::ChronoTime(Some(Box::new(
		chrono::NaiveTime::from_hms_nano_opt(1, 2, 59, nanos).unwrap(),
	)))
}

#[cfg(feature = "postgres")]
#[rstest]
#[case(
	temporal_value(4),
	"ChronoTime",
	"sub-microsecond precision is unsupported"
)]
#[case(Value::Array(ArrayType::ChronoTime, Some(Box::new(vec![temporal_value(4)]))), "Array", "sub-microsecond precision is unsupported")]
#[case(
	temporal_value(1_000_000_000),
	"ChronoTime",
	"leap seconds are unsupported"
)]
fn postgres_rejects_temporal_precision_loss(
	#[case] value: Value,
	#[case] kind: &str,
	#[case] reason: &str,
) {
	// Arrange
	let values = Values(vec![Value::Int(Some(7)), value]);

	// Act
	let error = postgres::arguments(values).err().unwrap();

	// Assert
	assert_eq!(error, super::error("postgres", 2, kind, reason));
}

#[cfg(feature = "mysql")]
fn fixed_offset_datetime_value(nanos: u32) -> Value {
	use chrono::Timelike;

	Value::ChronoDateTimeWithTimeZone(Some(Box::new(
		chrono::DateTime::parse_from_rfc3339("2026-10-04T01:02:59+09:00")
			.unwrap()
			.with_nanosecond(nanos)
			.unwrap(),
	)))
}

#[cfg(feature = "mysql")]
#[rstest]
fn mysql_accepts_fixed_offset_datetime_and_null() {
	// Arrange
	let values = Values(vec![
		fixed_offset_datetime_value(123_456_000),
		Value::ChronoDateTimeWithTimeZone(None),
	]);

	// Act
	let arguments = mysql::arguments(values).unwrap();

	// Assert
	assert_eq!(sqlx::Arguments::len(&arguments), 2);
}

#[cfg(feature = "mysql")]
#[rstest]
#[case(
	temporal_value(4),
	"ChronoTime",
	"sub-microsecond precision is unsupported"
)]
#[case(
	temporal_value(1_000_000_000),
	"ChronoTime",
	"leap seconds are unsupported"
)]
#[case(
	fixed_offset_datetime_value(4),
	"ChronoDateTimeWithTimeZone",
	"sub-microsecond precision is unsupported"
)]
#[case(
	fixed_offset_datetime_value(1_000_000_000),
	"ChronoDateTimeWithTimeZone",
	"leap seconds are unsupported"
)]
fn mysql_rejects_temporal_precision_loss(
	#[case] value: Value,
	#[case] kind: &str,
	#[case] reason: &str,
) {
	// Arrange
	let values = Values(vec![Value::Int(Some(7)), value]);

	// Act
	let error = mysql::arguments(values).err().unwrap();

	// Assert
	assert_eq!(error, super::error("mysql", 2, kind, reason));
}

#[cfg(feature = "sqlite")]
#[rstest]
#[tokio::test]
async fn sqlite_codecs_round_trip_typed_values_and_nulls() {
	use reinhardt_query::SqliteQueryBuilder;
	use sqlx::Row;

	// Arrange
	let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
	let date = chrono::NaiveDate::from_ymd_opt(2026, 10, 4).unwrap();
	let time = chrono::NaiveTime::from_hms_nano_opt(1, 2, 3, 123_456_789).unwrap();
	let datetime = date.and_time(time);
	let uuid = uuid::Uuid::from_u128(42);
	let json = serde_json::json!({"quoted": "\"comma,slash\\", "array": [1, null]});
	let mut statement = Query::select();
	for value in [
		Value::BigUnsigned(Some(i64::MAX as u64)),
		date.into(),
		time.into(),
		datetime.into(),
		uuid.into(),
		json.clone().into(),
		Value::String(None),
		Value::Bytes(Some(Box::new(vec![0, 255]))),
		Value::ChronoDateTimeLocal(None),
		Value::ChronoDateTimeWithTimeZone(None),
	] {
		statement.expr(Expr::val(value));
	}
	let (sql, values) = statement.build(SqliteQueryBuilder);

	// Act
	let arguments = sqlite::arguments(values).unwrap();
	let row = sqlx::query_with(&sql, arguments)
		.fetch_one(&pool)
		.await
		.unwrap();

	// Assert
	assert_eq!(row.get::<i64, _>(0), i64::MAX);
	assert_eq!(row.get::<chrono::NaiveDate, _>(1), date);
	assert_eq!(row.get::<chrono::NaiveTime, _>(2), time);
	assert_eq!(row.get::<chrono::NaiveDateTime, _>(3), datetime);
	assert_eq!(row.get::<String, _>(4), uuid.to_string());
	assert_eq!(
		row.get::<sqlx::types::Json<serde_json::Value>, _>(5).0,
		json
	);
	assert_eq!(row.get::<Option<String>, _>(6), None);
	assert_eq!(row.get::<Vec<u8>, _>(7), vec![0, 255]);
	assert_eq!(row.get::<Option<chrono::DateTime<chrono::Utc>>, _>(8), None);
	assert_eq!(row.get::<Option<chrono::DateTime<chrono::Utc>>, _>(9), None);
}

#[cfg(feature = "sqlite")]
#[rstest]
#[case::fixed_positive(false, 9 * 3600, "2026-10-03T16:02:03.123456789Z")]
#[case::fixed_negative(false, -7 * 3600, "2026-10-05T06:02:03.987654321Z")]
#[case::local_positive(true, 9 * 3600, "2026-10-03T16:02:03.123456789Z")]
#[case::local_negative(true, -7 * 3600, "2026-10-05T06:02:03.987654321Z")]
#[tokio::test]
async fn sqlite_generated_datetime_filters_match_legacy_utc_storage(
	#[case] local: bool,
	#[case] offset_seconds: i32,
	#[case] timestamp: &str,
) {
	use crate::backends::{DatabaseBackend, QueryValue, SqliteBackend};
	use reinhardt_query::{ColumnDef, ExprTrait, QueryBuilder, SqliteQueryBuilder};
	use sqlx::Row;

	// Arrange
	let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
	let backend = SqliteBackend::new(pool);
	let utc = timestamp.parse::<chrono::DateTime<chrono::Utc>>().unwrap();
	let offset = chrono::FixedOffset::east_opt(offset_seconds).unwrap();
	let value = if local {
		// An explicit Local offset makes this independent of the host timezone.
		Value::ChronoDateTimeLocal(Some(Box::new(chrono::DateTime::from_naive_utc_and_offset(
			utc.naive_utc(),
			offset,
		))))
	} else {
		Value::ChronoDateTimeWithTimeZone(Some(Box::new(utc.with_timezone(&offset))))
	};
	let create = Query::create_table()
		.table("timestamp_probe")
		.col(ColumnDef::new("id").integer().primary_key(true))
		.col(ColumnDef::new("timestamp").timestamp())
		.to_owned();
	let (sql, _) = SqliteQueryBuilder.build_create_table(&create);
	backend.execute(&sql, vec![]).await.unwrap();
	let (sql, _) = Query::insert()
		.into_table("timestamp_probe")
		.columns(["id", "timestamp"])
		.values_panic([Value::Int(Some(7)), Value::from(utc)])
		.build(SqliteQueryBuilder);
	// Raw Timestamp binding is the UTC storage path used by Manager saves.
	backend
		.execute(&sql, vec![QueryValue::Int(7), QueryValue::Timestamp(utc)])
		.await
		.unwrap();
	let (sql, values) = Query::select()
		.column("id")
		.from("timestamp_probe")
		.and_where(Expr::col("timestamp").eq(value.clone()))
		.build(SqliteQueryBuilder);
	let (encoded_sql, encoded_values) = Query::select()
		.expr(Expr::val(value))
		.build(SqliteQueryBuilder);

	// Act
	let rows = backend.__fetch_all_generated(&sql, values).await.unwrap();
	let encoded = sqlx::query_with(&encoded_sql, sqlite::arguments(encoded_values).unwrap())
		.fetch_one(backend.pool())
		.await
		.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	assert_eq!(rows[0].get::<i64>("id").unwrap(), 7);
	assert_eq!(encoded.get::<String, _>(0), utc.to_rfc3339());
}

#[cfg(all(feature = "sqlite", feature = "orm"))]
#[rstest]
#[tokio::test]
async fn generated_fetch_errors_propagate_through_orm_connection() {
	use crate::orm::connection::DatabaseConnection;
	use reinhardt_query::SqliteQueryBuilder;

	// Arrange
	let db = DatabaseConnection::connect_sqlite("sqlite::memory:")
		.await
		.unwrap();
	let (sql, values) = Query::select()
		.expr(Expr::val(Value::BigUnsigned(Some(u64::MAX))))
		.build(SqliteQueryBuilder);

	// Act
	let one_error = db
		.query_one_generated(&sql, values.clone())
		.await
		.err()
		.unwrap();
	let all_error = db.query_generated(&sql, values).await.err().unwrap();

	// Assert
	let expected = super::error(
		"sqlite",
		1,
		"BigUnsigned",
		"unsigned integer exceeds signed 64-bit range",
	);
	assert_eq!(one_error.to_string(), expected.to_string());
	assert_eq!(all_error.to_string(), expected.to_string());
}

#[cfg(feature = "postgres")]
#[rstest]
#[tokio::test]
async fn postgres_round_trips_normalized_big_decimal_scalars_and_arrays() {
	use sqlx::Row;
	use testcontainers::{ImageExt, runners::AsyncRunner};
	use testcontainers_modules::postgres::Postgres;

	// Arrange
	let container = Postgres::default()
		.with_tag("16-alpine")
		.start()
		.await
		.unwrap();
	let url = format!(
		"postgres://postgres:postgres@{}:{}/postgres",
		container.get_host().await.unwrap(),
		container.get_host_port_ipv4(5432).await.unwrap()
	);
	let pool = sqlx::PgPool::connect(&url).await.unwrap();
	let inputs = [
		"0e131072".to_owned(),
		"0e-40000".to_owned(),
		"-0e131072".to_owned(),
		format!("1.23{}", "0".repeat(40000)),
	];
	for input in inputs {
		let decimal: sqlx::types::BigDecimal = input.parse().unwrap();
		let array = Value::Array(
			ArrayType::BigDecimal,
			Some(Box::new(vec![
				decimal.clone().into(),
				Value::BigDecimal(None),
			])),
		);
		let (sql, values) = Query::select()
			.expr(Expr::val(decimal.clone()))
			.expr(Expr::val(array))
			.expr(Expr::val(Value::BigDecimal(None)))
			.build(reinhardt_query::PostgresQueryBuilder);

		// Act
		let row = sqlx::query_with(&sql, postgres::arguments(values).unwrap())
			.fetch_one(&pool)
			.await
			.unwrap();

		// Assert
		assert_eq!(row.get::<sqlx::types::BigDecimal, _>(0), decimal);
		assert_eq!(
			row.get::<Vec<Option<sqlx::types::BigDecimal>>, _>(1),
			vec![Some(decimal), None]
		);
		assert_eq!(row.get::<Option<sqlx::types::BigDecimal>, _>(2), None);
	}
}

#[cfg(feature = "mysql")]
#[rstest]
#[tokio::test]
async fn mysql_codecs_round_trip_native_values() {
	use crate::backends::{DatabaseBackend, MySqlBackend};
	use reinhardt_query::{ColumnDef, MySqlQueryBuilder, QueryBuilder};
	use sqlx::Row;
	use testcontainers::{GenericImage, ImageExt, core::WaitFor, runners::AsyncRunner};

	// Arrange
	let container = GenericImage::new("mysql", "8.0")
		.with_exposed_port(testcontainers::core::ContainerPort::Tcp(3306))
		// The initialization server uses port 0; wait for the TCP server.
		.with_wait_for(WaitFor::message_on_stderr("port: 3306"))
		.with_env_var("MYSQL_ROOT_PASSWORD", "test")
		.with_env_var("MYSQL_DATABASE", "test")
		.start()
		.await
		.unwrap();
	let url = format!(
		"mysql://root:test@{}:{}/test",
		container.get_host().await.unwrap(),
		container.get_host_port_ipv4(3306).await.unwrap()
	);
	let backend = MySqlBackend::new(sqlx::MySqlPool::connect(&url).await.unwrap());
	let create = Query::create_table()
		.table("bind_probe")
		.col(ColumnDef::new("decimal").decimal(28, 15))
		.col(ColumnDef::new("big_decimal").decimal(60, 30))
		.to_owned();
	let (sql, _) = MySqlQueryBuilder.build_create_table(&create);
	backend.execute(&sql, vec![]).await.unwrap();
	let decimal: rust_decimal::Decimal = "1234567890123.123456789012345".parse().unwrap();
	let big_decimal: sqlx::types::BigDecimal =
		"12345678901234567890123456789.12345678901234567890123456789"
			.parse()
			.unwrap();
	let statement = Query::insert()
		.into_table("bind_probe")
		.columns(["decimal", "big_decimal"])
		.values_panic([Value::from(decimal), big_decimal.clone().into()])
		.to_owned();
	let (sql, values) = statement.build(MySqlQueryBuilder);
	let datetimes = [
		(
			"2026-10-04T01:02:03.123456+09:00",
			"2026-10-03T16:02:03.123456Z",
		),
		(
			"2026-10-04T23:02:03.654321-07:00",
			"2026-10-05T06:02:03.654321Z",
		),
		(
			"2026-10-04T12:02:03.000001+00:00",
			"2026-10-04T12:02:03.000001Z",
		),
	]
	.map(|(input, expected)| {
		(
			chrono::DateTime::parse_from_rfc3339(input).unwrap(),
			expected.parse::<chrono::DateTime<chrono::Utc>>().unwrap(),
		)
	});

	// Act
	let result = backend.__execute_generated(&sql, values).await.unwrap();
	let (sql, _) = Query::select()
		.from("bind_probe")
		.columns(["decimal", "big_decimal"])
		.build(MySqlQueryBuilder);
	let row = sqlx::query(&sql).fetch_one(backend.pool()).await.unwrap();
	let (sql, values) = Query::select()
		.expr(Expr::val(Value::BigUnsigned(Some(u64::MAX))))
		.build(MySqlQueryBuilder);
	let unsigned_row = sqlx::query_with(&sql, mysql::arguments(values).unwrap())
		.fetch_one(backend.pool())
		.await
		.unwrap();
	let mut datetime_rows = Vec::with_capacity(datetimes.len());
	for (datetime, _) in &datetimes {
		let (sql, values) = Query::select()
			.expr(Expr::val(*datetime))
			.expr(Expr::val(datetime.with_timezone(&chrono::Utc)))
			.expr(Expr::val(datetime.with_timezone(&chrono::Local)))
			.expr(Expr::val(Value::ChronoDateTimeWithTimeZone(None)))
			.build(MySqlQueryBuilder);
		let row = sqlx::query_with(&sql, mysql::arguments(values).unwrap())
			.fetch_one(backend.pool())
			.await
			.unwrap();
		datetime_rows.push(row);
	}

	// Assert
	assert_eq!(result.rows_affected, 1);
	assert_eq!(row.get::<rust_decimal::Decimal, _>("decimal"), decimal);
	assert_eq!(
		row.get::<sqlx::types::BigDecimal, _>("big_decimal"),
		big_decimal
	);
	assert_eq!(unsigned_row.get::<u64, _>(0), u64::MAX);
	for (row, (_, expected)) in datetime_rows.iter().zip(datetimes) {
		assert_eq!(row.get::<chrono::DateTime<chrono::Utc>, _>(0), expected);
		assert_eq!(row.get::<chrono::DateTime<chrono::Utc>, _>(1), expected);
		assert_eq!(row.get::<chrono::DateTime<chrono::Utc>, _>(2), expected);
		assert_eq!(row.get::<Option<chrono::DateTime<chrono::Utc>>, _>(3), None);
	}

	for (input, expected) in [
		("0e65", "0"),
		("0e-65", "0"),
		("-0e65", "0"),
		("1.2300000000000000000000000000000000", "1.23"),
		("100e-32", "1e-30"),
	] {
		// Arrange
		let decimal: sqlx::types::BigDecimal = input.parse().unwrap();
		let expected: sqlx::types::BigDecimal = expected.parse().unwrap();
		let (sql, values) = Query::update()
			.table("bind_probe")
			.value("big_decimal", decimal)
			.build(MySqlQueryBuilder);

		// Act
		backend.__execute_generated(&sql, values).await.unwrap();
		let (sql, _) = Query::select()
			.from("bind_probe")
			.column("big_decimal")
			.build(MySqlQueryBuilder);
		let row = sqlx::query(&sql).fetch_one(backend.pool()).await.unwrap();

		// Assert
		assert_eq!(row.get::<sqlx::types::BigDecimal, _>(0), expected);
	}
}
