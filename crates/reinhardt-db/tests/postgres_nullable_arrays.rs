//! PostgreSQL row decoding preserves nullable scalar-array elements.

#![cfg(all(feature = "postgres", feature = "orm"))]

use futures::TryStreamExt;
use reinhardt_db::backends::{DatabaseBackend, PostgresBackend, QueryValue};
use reinhardt_db::orm::connection::BackendsConnection;
use reinhardt_db::orm::{
	DatabaseArrayType, DatabaseConnectionLease, DatabaseScalar, DatabaseStorageKind, DatabaseValue,
	FieldCodecError, FieldSelector, Manager, Model, QueryRow,
};
use reinhardt_query::{
	ArrayType, ColumnDef, ColumnType, Expr, ExprTrait, PostgresQueryBuilder, Query,
	QueryStatementBuilder, Value,
};
use rstest::*;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use std::{collections::BTreeMap, sync::Arc};
use testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner};
use testcontainers_modules::postgres::Postgres;

struct ArrayFixture {
	backend: PostgresBackend,
	_container: ContainerAsync<Postgres>,
}

// Component tests own their container directly to avoid a circular publish
// dependency on reinhardt-test (RELEASE_PROCESS.md KI-1/KI-2).
#[fixture]
async fn postgres_arrays() -> ArrayFixture {
	let container = Postgres::default()
		.with_tag("17-alpine")
		.start()
		.await
		.expect("PostgreSQL container should start");
	let host = container.get_host().await.unwrap();
	let port = container.get_host_port_ipv4(5432).await.unwrap();
	let pool = PgPoolOptions::new()
		.max_connections(2)
		.connect(&format!(
			"postgres://postgres:postgres@{host}:{port}/postgres"
		))
		.await
		.expect("PostgreSQL pool should connect");
	ArrayFixture {
		backend: PostgresBackend::new(pool),
		_container: container,
	}
}

#[rstest]
#[tokio::test]
async fn text_array_preserves_null_positions(#[future] postgres_arrays: ArrayFixture) {
	// Arrange
	let fixture = postgres_arrays.await;
	let sql = Query::select()
		.expr_as(
			Expr::value(Value::Array(
				ArrayType::String,
				Some(Box::new(vec![Value::from("kept"), Value::String(None)])),
			))
			.cast_as("_text"),
			"items",
		)
		.to_string(PostgresQueryBuilder);
	let raw: Vec<Option<String>> = sqlx::query_scalar(&sql)
		.fetch_one(fixture.backend.pool())
		.await
		.unwrap();
	assert_eq!(raw, vec![Some("kept".to_owned()), None]);

	// Act
	let row = fixture.backend.fetch_one(&sql, vec![]).await.unwrap();
	let row = QueryRow::from_backend_row(row);

	// Assert
	assert_eq!(row.get::<Vec<Option<String>>>("items"), Some(raw));
}

#[rstest]
#[case::text(
	"_text",
	ArrayType::String,
	["kept".to_owned(), "tail".to_owned()],
	QueryValue::StringArray,
	QueryValue::NullableStringArray,
	|value: Option<String>| Value::String(value.map(Box::new)),
)]
#[case::varchar(
	"_varchar",
	ArrayType::String,
	["kept".to_owned(), "tail".to_owned()],
	QueryValue::StringArray,
	QueryValue::NullableStringArray,
	|value: Option<String>| Value::String(value.map(Box::new)),
)]
#[case::char(
	"_bpchar",
	ArrayType::String,
	["kept".to_owned(), "tail".to_owned()],
	QueryValue::StringArray,
	QueryValue::NullableStringArray,
	|value: Option<String>| Value::String(value.map(Box::new)),
)]
#[case::integer(
	"_int4",
	ArrayType::Int,
	[i32::MIN, i32::MAX],
	QueryValue::IntArray,
	QueryValue::NullableIntArray,
	Value::Int,
)]
#[case::bigint(
	"_int8",
	ArrayType::BigInt,
	[i64::MIN, i64::MAX],
	QueryValue::BigIntArray,
	QueryValue::NullableBigIntArray,
	Value::BigInt,
)]
#[case::boolean(
	"_bool",
	ArrayType::Bool,
	[true, false],
	QueryValue::BoolArray,
	QueryValue::NullableBoolArray,
	Value::Bool,
)]
#[case::real(
	"_float4",
	ArrayType::Float,
	[1.5_f32, -2.5_f32],
	QueryValue::FloatArray,
	QueryValue::NullableFloatArray,
	Value::Float,
)]
#[case::double(
	"_float8",
	ArrayType::Double,
	[3.5_f64, -4.5_f64],
	QueryValue::DoubleArray,
	QueryValue::NullableDoubleArray,
	Value::Double,
)]
#[case::uuid(
	"_uuid",
	ArrayType::Uuid,
	[uuid::Uuid::nil(), uuid::Uuid::from_u128(1)],
	QueryValue::UuidArray,
	QueryValue::NullableUuidArray,
	|value: Option<uuid::Uuid>| Value::Uuid(value.map(Box::new)),
)]
#[tokio::test]
async fn scalar_arrays_preserve_values_and_null_positions<T>(
	#[future] postgres_arrays: ArrayFixture,
	#[case] postgres_type: &'static str,
	#[case] array_type: ArrayType,
	#[case] values: [T; 2],
	#[case] non_nullable: fn(Vec<T>) -> QueryValue,
	#[case] nullable: fn(Vec<Option<T>>) -> QueryValue,
	#[case] encode: fn(Option<T>) -> Value,
) where
	T: Clone
		+ std::fmt::Debug
		+ PartialEq
		+ serde::Serialize
		+ serde::de::DeserializeOwned
		+ sqlx::Type<sqlx::Postgres>
		+ sqlx::postgres::PgHasArrayType
		+ for<'r> sqlx::Decode<'r, sqlx::Postgres>
		+ Send
		+ Unpin,
{
	// Arrange
	let fixture = postgres_arrays.await;
	let mixed = vec![
		None,
		Some(values[0].clone()),
		None,
		Some(values[1].clone()),
		None,
	];
	let all_null = vec![None, None];
	let non_null = values.iter().cloned().map(Some).collect::<Vec<_>>();
	let shapes = [
		("mixed", Some(mixed.clone()), nullable(mixed)),
		("all_null", Some(all_null.clone()), nullable(all_null)),
		("non_null", Some(non_null), non_nullable(values.to_vec())),
		("empty", Some(vec![]), non_nullable(vec![])),
		("whole_null", None, QueryValue::Null),
	];
	let (bound_sql, _) = Query::select()
		.expr_as(Expr::value(0_i32), "items")
		.build(PostgresQueryBuilder);
	let mut transaction = fixture.backend.begin().await.unwrap();

	for (shape, array_values, expected) in shapes {
		let value = Value::Array(
			array_type.clone(),
			array_values
				.as_ref()
				.map(|values| Box::new(values.iter().cloned().map(encode).collect())),
		);
		let sql = Query::select()
			.expr_as(Expr::value(value).cast_as(postgres_type), "items")
			.to_string(PostgresQueryBuilder);
		let raw: Option<Vec<Option<T>>> = sqlx::query_scalar(&sql)
			.fetch_one(fixture.backend.pool())
			.await
			.unwrap();
		assert_eq!(raw, array_values, "{postgres_type}: {shape} SQLx oracle");

		// Act
		let row = fixture.backend.fetch_one(&sql, vec![]).await.unwrap();
		let all = fixture.backend.fetch_all(&sql, vec![]).await.unwrap();
		let optional = fixture.backend.fetch_optional(&sql, vec![]).await.unwrap();
		let streamed = fixture
			.backend
			.fetch_stream(sql.clone(), vec![], 1)
			.unwrap()
			.try_collect::<Vec<_>>()
			.await
			.unwrap();
		let transaction_row = transaction.fetch_one(&sql, vec![]).await.unwrap();
		let transaction_all = transaction.fetch_all(&sql, vec![]).await.unwrap();
		let transaction_optional = transaction.fetch_optional(&sql, vec![]).await.unwrap();
		let transaction_streamed = transaction
			.fetch_stream(sql, vec![], 1)
			.unwrap()
			.try_collect::<Vec<_>>()
			.await
			.unwrap();

		// Assert
		assert_eq!(
			row.data.get("items"),
			Some(&expected),
			"{postgres_type}: {shape}"
		);
		assert_eq!(all, vec![row.clone()]);
		assert_eq!(optional, Some(row.clone()));
		assert_eq!(streamed, vec![row.clone()]);
		assert_eq!(transaction_row, row);
		assert_eq!(transaction_all, vec![row.clone()]);
		assert_eq!(transaction_optional, Some(row.clone()));
		assert_eq!(transaction_streamed, vec![row.clone()]);
		assert_eq!(
			QueryRow::from_backend_row(row).get::<Option<Vec<Option<T>>>>("items"),
			Some(array_values.clone()),
			"{postgres_type}: {shape} ORM values",
		);
		let serialized = serde_json::to_string(&expected).unwrap();
		assert_eq!(
			serde_json::from_str::<QueryValue>(&serialized).unwrap(),
			expected
		);

		if let Some(values) = array_values {
			// Nullable carriers also bind empty and non-NULL arrays with their type intact.
			let bound_value = nullable(values);
			let rebound = fixture
				.backend
				.fetch_one(&bound_sql, vec![bound_value.clone()])
				.await
				.unwrap();
			let transaction_rebound = transaction
				.fetch_one(&bound_sql, vec![bound_value])
				.await
				.unwrap();
			assert_eq!(rebound.data.get("items"), Some(&expected));
			assert_eq!(transaction_rebound, rebound);
		}
	}
	transaction.rollback().await.unwrap();
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct NullableArrayModel<T> {
	id: i64,
	items: Vec<Option<T>>,
}

#[derive(Clone)]
struct NullableArrayFields;

impl FieldSelector for NullableArrayFields {
	fn with_alias(self, _alias: &str) -> Self {
		self
	}
}

impl<T> Model for NullableArrayModel<T>
where
	T: DatabaseScalar + Clone + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
	type PrimaryKey = i64;
	type Fields = NullableArrayFields;
	type Objects = Manager<Self>;

	fn table_name() -> &'static str {
		"nullable_array_models"
	}

	fn primary_key(&self) -> Option<Self::PrimaryKey> {
		Some(self.id)
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = value;
	}

	fn new_fields() -> Self::Fields {
		NullableArrayFields
	}

	fn encode_database_fields(&self) -> Result<BTreeMap<String, DatabaseValue>, FieldCodecError> {
		let element_type = match T::STORAGE_KIND {
			DatabaseStorageKind::String => DatabaseArrayType::String,
			DatabaseStorageKind::I32 => DatabaseArrayType::I32,
			DatabaseStorageKind::I64 => DatabaseArrayType::I64,
			DatabaseStorageKind::Bool => DatabaseArrayType::Bool,
			DatabaseStorageKind::F32 => DatabaseArrayType::F32,
			DatabaseStorageKind::F64 => DatabaseArrayType::F64,
			DatabaseStorageKind::Uuid => DatabaseArrayType::Uuid,
			_ => panic!("test model requires a supported scalar array type"),
		};
		Ok(BTreeMap::from([
			("id".to_owned(), DatabaseValue::I64(self.id)),
			(
				"items".to_owned(),
				DatabaseValue::Array {
					element_type,
					values: self
						.items
						.iter()
						.cloned()
						.map(|value| value.map_or(DatabaseValue::Null, T::into_database_value))
						.collect(),
				},
			),
		]))
	}
}

#[rstest]
#[case::string(ColumnType::Text, ["kept".to_owned(), "tail".to_owned()])]
#[case::int(ColumnType::Integer, [i32::MIN, i32::MAX])]
#[case::bigint(ColumnType::BigInteger, [i64::MIN, i64::MAX])]
#[case::bool(ColumnType::Boolean, [true, false])]
#[case::float(ColumnType::Float, [1.5_f32, -2.5_f32])]
#[case::double(ColumnType::Double, [3.5_f64, -4.5_f64])]
#[case::uuid(ColumnType::Uuid, [uuid::Uuid::nil(), uuid::Uuid::from_u128(1)])]
#[tokio::test]
async fn manager_writes_preserve_nullable_array_elements<T>(
	#[future] postgres_arrays: ArrayFixture,
	#[case] column_type: ColumnType,
	#[case] values: [T; 2],
) where
	T: DatabaseScalar
		+ Clone
		+ std::fmt::Debug
		+ PartialEq
		+ Serialize
		+ for<'de> Deserialize<'de>
		+ sqlx::Type<sqlx::Postgres>
		+ sqlx::postgres::PgHasArrayType
		+ for<'r> sqlx::Decode<'r, sqlx::Postgres>
		+ Send
		+ Sync
		+ Unpin
		+ 'static,
{
	// Arrange
	let fixture = postgres_arrays.await;
	let schema = Query::create_table()
		.table(NullableArrayModel::<T>::table_name())
		.col(ColumnDef::new("id").big_integer().primary_key(true))
		.col(ColumnDef::new("items").array(column_type))
		.to_string(PostgresQueryBuilder);
	fixture.backend.execute(&schema, vec![]).await.unwrap();
	let owner = BackendsConnection::new(Arc::new(PostgresBackend::new(
		fixture.backend.pool().clone(),
	)));
	let lease = DatabaseConnectionLease::register(owner).unwrap();
	let mut connection = lease.handle();
	let manager = Manager::<NullableArrayModel<T>>::new();
	let shapes = [
		vec![
			None,
			Some(values[0].clone()),
			None,
			Some(values[1].clone()),
			None,
		],
		vec![None, None],
		values.iter().cloned().map(Some).collect(),
		vec![],
	];
	let mut transaction = fixture.backend.begin().await.unwrap();
	let mut updated_models = Vec::new();

	for (index, items) in shapes.iter().enumerate() {
		let model = NullableArrayModel {
			id: index as i64 + 1,
			items: items.clone(),
		};

		// Act
		let created = manager
			.create_with_conn(&mut connection, &model)
			.await
			.unwrap();
		let transaction_model = NullableArrayModel {
			id: model.id + 100,
			items: model.items.clone(),
		};
		let transaction_created = manager
			.insert_with_executor(transaction.as_mut(), &transaction_model)
			.await
			.unwrap();
		let updated_model = NullableArrayModel {
			id: model.id,
			items: shapes[(index + 1) % shapes.len()].clone(),
		};
		let updated = manager
			.update_with_conn(&mut connection, &updated_model)
			.await
			.unwrap();
		let transaction_updated_model = NullableArrayModel {
			id: transaction_model.id,
			items: updated_model.items.clone(),
		};
		let transaction_updated = manager
			.save_with_executor(transaction.as_mut(), &transaction_updated_model)
			.await
			.unwrap();

		// Assert
		assert_eq!(created, model);
		assert_eq!(transaction_created, transaction_model);
		assert_eq!(updated, updated_model);
		assert_eq!(transaction_updated, transaction_updated_model);
		updated_models.extend([updated_model, transaction_updated_model]);
	}
	transaction.commit().await.unwrap();

	for model in updated_models {
		let sql = Query::select()
			.column("items")
			.from(NullableArrayModel::<T>::table_name())
			.and_where(Expr::col("id").eq(model.id))
			.to_string(PostgresQueryBuilder);
		let stored: Vec<Option<T>> = sqlx::query_scalar(&sql)
			.fetch_one(fixture.backend.pool())
			.await
			.unwrap();
		assert_eq!(stored, model.items, "SQLx oracle for model {}", model.id);
	}
}

#[cfg(feature = "sqlite")]
#[rstest]
#[case::string(QueryValue::NullableStringArray(vec![None, Some("kept".to_owned()), None]), serde_json::json!([null, "kept", null]))]
#[case::int(QueryValue::NullableIntArray(vec![None, Some(i32::MAX), None]), serde_json::json!([null, i32::MAX, null]))]
#[case::bigint(QueryValue::NullableBigIntArray(vec![None, Some(i64::MAX), None]), serde_json::json!([null, i64::MAX, null]))]
#[case::bool(QueryValue::NullableBoolArray(vec![None, Some(false), None]), serde_json::json!([null, false, null]))]
#[case::float(QueryValue::NullableFloatArray(vec![None, Some(1.5), None]), serde_json::json!([null, 1.5, null]))]
#[case::double(QueryValue::NullableDoubleArray(vec![None, Some(-2.5), None]), serde_json::json!([null, -2.5, null]))]
#[case::uuid(QueryValue::NullableUuidArray(vec![None, Some(uuid::Uuid::nil()), None]), serde_json::json!([null, "00000000-0000-0000-0000-000000000000", null]))]
#[tokio::test]
async fn nullable_arrays_bind_as_json_on_sqlite(
	#[case] value: QueryValue,
	#[case] expected: serde_json::Value,
) {
	// Arrange
	let pool = sqlx::sqlite::SqlitePoolOptions::new()
		.max_connections(1)
		.connect("sqlite::memory:")
		.await
		.unwrap();
	let backend = reinhardt_db::backends::SqliteBackend::new(pool);
	let (sql, _) = Query::select()
		.expr_as(Expr::value(0_i32), "items")
		.build(reinhardt_query::SqliteQueryBuilder);
	let mut transaction = backend.begin().await.unwrap();

	// Act
	let transaction_row = transaction
		.fetch_one(&sql, vec![value.clone()])
		.await
		.unwrap();
	transaction.rollback().await.unwrap();
	let row = backend.fetch_one(&sql, vec![value]).await.unwrap();

	// Assert
	assert_eq!(transaction_row, row);
	let json = row.get::<String>("items").unwrap();
	assert_eq!(
		serde_json::from_str::<serde_json::Value>(&json).unwrap(),
		expected
	);
}
