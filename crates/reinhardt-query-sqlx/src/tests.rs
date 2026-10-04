//! Argument conversion regression coverage; SQL literals here are test inputs.
use super::*;
#[cfg(feature = "postgres")]
use reinhardt_query::ArrayType;
use reinhardt_query::Values;
#[cfg(any(feature = "postgres", feature = "sqlite"))]
use rstest::fixture;
use rstest::rstest;
use sqlx::Arguments;

#[cfg(any(feature = "postgres", feature = "any"))]
fn failure<A>(result: Result<PreparedQuery<A>, BindError>) -> BindError {
	match result {
		Err(error) => error,
		Ok(_) => panic!("encoding unexpectedly succeeded"),
	}
}
#[cfg(any(feature = "postgres", feature = "sqlite"))]
#[fixture]
fn scalar_values() -> Values {
	Values(vec![
		Value::Bool(Some(true)),
		Value::Int(Some(42)),
		Value::String(Some(Box::new("'quoted ? $1".into()))),
		Value::Bytes(Some(Box::new(vec![0, 255]))),
		Value::BigInt(None),
	])
}
#[cfg(feature = "postgres")]
#[rstest]
fn postgres_preserves_sql_and_all_supplied_values(scalar_values: Values) {
	// Arrange: even an explicit NULL argument must retain its renderer-supplied position.
	let sql = "SELECT $1, $2, $3, $4, $5";
	// Act
	let prepared = prepare_postgres((sql.into(), scalar_values)).unwrap();
	let (actual, arguments) = prepared.into_parts();
	// Assert
	assert_eq!(actual, sql);
	assert_eq!(arguments.len(), 5);
}
#[cfg(feature = "postgres")]
#[rstest]
#[case(
	Value::BigUnsigned(Some(u64::MAX)),
	"BigUnsigned",
	"unsigned integer exceeds signed 64-bit range"
)]
#[case(Value::Array(ArrayType::Int, Some(Box::new(vec![Value::String(Some(Box::new("secret".into())))]))), "Array", "array element does not match declared element type")]
fn postgres_rejects_invalid_values_without_contents(
	#[case] value: Value,
	#[case] kind: &'static str,
	#[case] reason: &'static str,
) {
	let error = failure(prepare_postgres((
		"SELECT $1, $2".into(),
		Values(vec![Value::Int(Some(1)), value]),
	)));
	assert_eq!(
		error,
		BindError {
			backend: "postgres",
			index: 2,
			value_type: kind,
			reason
		}
	);
	assert_eq!(
		error.to_string(),
		format!("cannot encode {kind} argument 2 for postgres: {reason}")
	);
}
#[cfg(feature = "postgres")]
#[rstest]
#[case(vec![])]
#[case(vec![Value::Int(Some(42)), Value::Int(None)])]
fn postgres_accepts_empty_and_nullable_typed_arrays(#[case] values: Vec<Value>) {
	let prepared = prepare_postgres((
		"SELECT $1".into(),
		Values(vec![Value::Array(ArrayType::Int, Some(Box::new(values)))]),
	))
	.unwrap();
	assert_eq!(prepared.into_parts().1.len(), 1);
}
#[cfg(feature = "mysql")]
#[rstest]
fn mysql_retains_full_unsigned_range() {
	let prepared = prepare_mysql((
		"SELECT ?".into(),
		Values(vec![Value::BigUnsigned(Some(u64::MAX))]),
	))
	.unwrap();
	assert_eq!(prepared.into_parts().1.len(), 1);
}
#[cfg(feature = "sqlite")]
#[rstest]
#[tokio::test]
async fn sqlite_round_trip_preserves_order_null_and_bytes(scalar_values: Values) {
	// Arrange: the connection owns its in-memory database through RAII.
	use sqlx::Connection;
	let mut connection = sqlx::SqliteConnection::connect("sqlite::memory:")
		.await
		.unwrap();
	let prepared = prepare_sqlite(("SELECT ?, ?, ?, ?, ?".into(), scalar_values)).unwrap();
	let (sql, arguments) = prepared.into_parts();
	// Act
	let row: (bool, i32, String, Vec<u8>, Option<i64>) = sqlx::query_as_with(&sql, arguments)
		.fetch_one(&mut connection)
		.await
		.unwrap();
	// Assert
	assert_eq!(row, (true, 42, "'quoted ? $1".into(), vec![0, 255], None));
}

#[cfg(all(feature = "sqlite", feature = "with-uuid"))]
#[rstest]
#[tokio::test]
async fn sqlite_text_uuid_mode_preserves_legacy_columns_and_native_default() {
	use sqlx::Connection;
	// Arrange
	let uuid = sqlx::types::Uuid::parse_str("12345678-1234-5678-9abc-def012345678").unwrap();
	let mut connection = sqlx::SqliteConnection::connect("sqlite::memory:")
		.await
		.unwrap();
	let value = Value::Uuid(Some(Box::new(uuid)));
	let text_sql = "SELECT typeof(?), ?, ?";
	let (sql, arguments) = prepare_sqlite_with_text_uuid((
		text_sql.into(),
		Values(vec![value.clone(), value.clone(), Value::Uuid(None)]),
	))
	.unwrap()
	.into_parts();
	assert_eq!(sql, text_sql);
	assert_eq!(arguments.len(), 3);
	// Act
	let text: (String, String, Option<String>) = sqlx::query_as_with(&sql, arguments)
		.fetch_one(&mut connection)
		.await
		.unwrap();
	let (sql, arguments) = prepare_sqlite((
		"SELECT typeof(?), ?".into(),
		Values(vec![value.clone(), value]),
	))
	.unwrap()
	.into_parts();
	let native: (String, Vec<u8>) = sqlx::query_as_with(&sql, arguments)
		.fetch_one(&mut connection)
		.await
		.unwrap();
	// Assert: choosing text mode does not change the ordinary binary UUID codec.
	assert_eq!(text, ("text".into(), uuid.to_string(), None));
	assert_eq!(native, ("blob".into(), uuid.as_bytes().to_vec()));
}

#[cfg(all(feature = "mysql", feature = "with-uuid"))]
#[rstest]
#[tokio::test]
async fn mysql_text_uuid_mode_preserves_legacy_columns_and_native_default() {
	use testcontainers::runners::AsyncRunner;
	// Arrange: container and pool own cleanup across both execution paths.
	let container = testcontainers_modules::mysql::Mysql::default()
		.start()
		.await
		.unwrap();
	let url = format!(
		"mysql://root@{}:{}/test",
		container.get_host().await.unwrap(),
		container.get_host_port_ipv4(3306).await.unwrap()
	);
	let pool = sqlx::MySqlPool::connect(&url).await.unwrap();
	let uuid = sqlx::types::Uuid::parse_str("12345678-1234-5678-9abc-def012345678").unwrap();
	let value = Value::Uuid(Some(Box::new(uuid)));
	let text_sql = "SELECT ?, ?";
	let (sql, arguments) = prepare_mysql_with_text_uuid((
		text_sql.into(),
		Values(vec![value.clone(), Value::Uuid(None)]),
	))
	.unwrap()
	.into_parts();
	assert_eq!(sql, text_sql);
	assert_eq!(arguments.len(), 2);
	// Act
	let text: (String, Option<String>) = sqlx::query_as_with(&sql, arguments)
		.fetch_one(&pool)
		.await
		.unwrap();
	let (sql, arguments) = prepare_mysql(("SELECT ?".into(), Values(vec![value])))
		.unwrap()
		.into_parts();
	let native: (Vec<u8>,) = sqlx::query_as_with(&sql, arguments)
		.fetch_one(&pool)
		.await
		.unwrap();
	// Assert
	assert_eq!(text, (uuid.to_string(), None));
	assert_eq!(native.0, uuid.as_bytes());
}
#[cfg(feature = "any")]
#[rstest]
#[case(AnyBackend::Postgres, "postgres/any")]
#[case(AnyBackend::MySql, "mysql/any")]
#[case(AnyBackend::Sqlite, "sqlite/any")]
fn any_rejects_unsigned_overflow(#[case] backend: AnyBackend, #[case] name: &'static str) {
	let error = failure(prepare_any(
		(
			"SELECT ?".into(),
			Values(vec![Value::BigUnsigned(Some(u64::MAX))]),
		),
		backend,
	));
	assert_eq!(
		error,
		BindError {
			backend: name,
			index: 1,
			value_type: "BigUnsigned",
			reason: "unsigned integer exceeds signed 64-bit range"
		}
	);
}
#[cfg(all(feature = "any", feature = "with-json"))]
#[rstest]
fn any_requires_explicit_complex_text_codec() {
	let value = Value::Json(Some(Box::new(
		sqlx::types::Json::<sqlx::types::JsonValue>::default().0,
	)));
	let error = failure(prepare_any(
		("SELECT ?".into(), Values(vec![value.clone()])),
		AnyBackend::Sqlite,
	));
	assert_eq!(
		error,
		BindError {
			backend: "sqlite/any",
			index: 1,
			value_type: "Json",
			reason: "complex values require explicit text compatibility codecs"
		}
	);
	let prepared =
		prepare_any_with_text_codecs(("SELECT ?".into(), Values(vec![value])), AnyBackend::Sqlite)
			.unwrap();
	assert_eq!(prepared.into_parts().1.len(), 1);
}

#[cfg(feature = "postgres")]
#[rstest]
#[tokio::test]
async fn postgres_native_round_trip() {
	// Arrange: container and pool guards clean up even on assertion failure.
	use testcontainers::runners::AsyncRunner;
	use testcontainers_modules::postgres::Postgres;
	let container = Postgres::default().start().await.unwrap();
	let url = format!(
		"postgres://postgres:postgres@{}:{}/postgres",
		container.get_host().await.unwrap(),
		container.get_host_port_ipv4(5432).await.unwrap()
	);
	let pool = sqlx::PgPool::connect(&url).await.unwrap();
	let values = Values(vec![
		Value::Int(Some(42)),
		Value::String(Some(Box::new("'quoted ? $1".into()))),
		Value::Bytes(Some(Box::new(vec![0, 255]))),
		Value::Array(
			ArrayType::Int,
			Some(Box::new(vec![Value::Int(Some(7)), Value::Int(None)])),
		),
	]);
	let (sql, arguments) = prepare_postgres(("SELECT $1, $2, $3, $4".into(), values))
		.unwrap()
		.into_parts();
	// Act
	let row: (i32, String, Vec<u8>, Vec<Option<i32>>) = sqlx::query_as_with(&sql, arguments)
		.fetch_one(&pool)
		.await
		.unwrap();
	// Assert
	assert_eq!(
		row,
		(42, "'quoted ? $1".into(), vec![0, 255], vec![Some(7), None])
	);
}

#[cfg(feature = "pgvector")]
#[rstest]
#[case(vec![])]
#[case(vec![1.0; 16_001])]
#[case(vec![f32::NAN])]
#[case(vec![f32::INFINITY])]
fn postgres_rejects_invalid_vectors_before_execution(#[case] values: Vec<f32>) {
	// Arrange
	let values = Values(vec![Value::Vector(Some(Box::new(values)))]);
	// Act
	let error = failure(prepare_postgres(("private SQL".into(), values)));
	// Assert
	assert_eq!(error.index, 1);
	assert_eq!(error.value_type, "Vector");
	assert_eq!(error.backend, "postgres");
	assert!(!error.to_string().contains("private SQL"));
}

#[cfg(feature = "pgvector")]
#[rstest]
#[tokio::test]
async fn postgres_vector_codec_round_trips_native_extension_and_null() {
	use testcontainers::{
		GenericImage, ImageExt,
		core::{IntoContainerPort, WaitFor},
		runners::AsyncRunner,
	};
	// Arrange: container and pool own database/extension resources through RAII.
	let container = GenericImage::new("pgvector/pgvector", "pg16")
		.with_exposed_port(5432.tcp())
		.with_wait_for(WaitFor::message_on_stderr(
			"database system is ready to accept connections",
		))
		.with_env_var("POSTGRES_PASSWORD", "postgres")
		.start()
		.await
		.unwrap();
	let url = format!(
		"postgres://postgres:postgres@{}:{}/postgres",
		container.get_host().await.unwrap(),
		container.get_host_port_ipv4(5432).await.unwrap()
	);
	let pool = sqlx::PgPool::connect(&url).await.unwrap();
	sqlx::query("CREATE EXTENSION vector")
		.execute(&pool)
		.await
		.unwrap();
	let sql = "SELECT $1::vector::text, ($2::vector IS NULL)";
	let (actual_sql, arguments) = prepare_postgres((
		sql.into(),
		Values(vec![
			Value::Vector(Some(Box::new(vec![1.0, -2.5, 3.25]))),
			Value::Vector(None),
		]),
	))
	.unwrap()
	.into_parts();
	// Act: native PostgreSQL decodes the binary vector, then renders its text.
	let row: (String, bool) = sqlx::query_as_with(&actual_sql, arguments)
		.fetch_one(&pool)
		.await
		.unwrap();
	// Assert
	assert_eq!(actual_sql, sql);
	assert_eq!(row, ("[1,-2.5,3.25]".into(), true));
}

#[cfg(feature = "mysql")]
#[rstest]
#[tokio::test]
async fn mysql_native_round_trip() {
	// Arrange
	use testcontainers::runners::AsyncRunner;
	use testcontainers_modules::mysql::Mysql;
	let container = Mysql::default().start().await.unwrap();
	let url = format!(
		"mysql://root@{}:{}/test",
		container.get_host().await.unwrap(),
		container.get_host_port_ipv4(3306).await.unwrap()
	);
	let pool = sqlx::MySqlPool::connect(&url).await.unwrap();
	let values = Values(vec![
		Value::BigUnsigned(Some(u64::MAX)),
		Value::String(Some(Box::new("'quoted ? $1".into()))),
		Value::Bytes(Some(Box::new(vec![0, 255]))),
		Value::BigInt(None),
	]);
	let (sql, arguments) = prepare_mysql(("SELECT ?, ?, ?, ?".into(), values))
		.unwrap()
		.into_parts();
	// Act
	let row: (u64, String, Vec<u8>, Option<i64>) = sqlx::query_as_with(&sql, arguments)
		.fetch_one(&pool)
		.await
		.unwrap();
	// Assert
	assert_eq!(row, (u64::MAX, "'quoted ? $1".into(), vec![0, 255], None));
}
