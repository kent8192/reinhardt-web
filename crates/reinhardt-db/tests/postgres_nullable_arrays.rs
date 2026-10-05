//! PostgreSQL row decoding preserves nullable scalar-array elements.

#![cfg(all(feature = "postgres", feature = "orm"))]

use futures::TryStreamExt;
use reinhardt_db::backends::{DatabaseBackend, PostgresBackend, QueryValue};
use reinhardt_db::orm::QueryRow;
use reinhardt_query::{
	ArrayType, Expr, ExprTrait, PostgresQueryBuilder, Query, QueryStatementBuilder, Value,
};
use rstest::*;
use sqlx::postgres::PgPoolOptions;
use testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner};
use testcontainers_modules::postgres::Postgres;

struct ArrayFixture {
	backend: PostgresBackend,
	#[cfg(feature = "migrations")]
	database_url: String,
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
	let database_url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
	let pool = PgPoolOptions::new()
		.max_connections(2)
		.connect(&database_url)
		.await
		.expect("PostgreSQL pool should connect");
	ArrayFixture {
		backend: PostgresBackend::new(pool),
		#[cfg(feature = "migrations")]
		database_url,
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

#[cfg(feature = "migrations")]
mod derived_models {
	use super::*;
	use reinhardt_db::orm::connection::BackendsConnection;
	use reinhardt_db::orm::custom_manager::CustomManager;
	use reinhardt_db::orm::expressions::{FieldRef, GeneratedModelField};
	use reinhardt_db::orm::{
		DatabaseConnectionLease, DatabaseField, DatabaseScalar, Manager, Model,
	};
	use reinhardt_query::{ColumnDef, ColumnType};
	use serde::{Deserialize, Serialize};
	use std::{marker::PhantomData, sync::Arc};

	trait NullableArrayRecord<T>: Model<PrimaryKey = i64> {
		fn new(id: i64, items: Vec<Option<T>>) -> Self;
		fn items(&self) -> &[Option<T>];
		fn id_field() -> FieldRef<Self, i64, GeneratedModelField>;
		fn items_field() -> FieldRef<Self, Vec<Option<T>>, GeneratedModelField>;
	}

	macro_rules! nullable_array_model {
		($name:ident, $element:ty, $table:literal) => {
			#[derive(
				reinhardt_core::macros::Model, Debug, Clone, PartialEq, Serialize, Deserialize,
			)]
			#[model(app_label = "nullable_arrays", table_name = $table)]
			struct $name {
				#[field(primary_key = true)]
				id: i64,
				items: Vec<Option<$element>>,
			}

			impl NullableArrayRecord<$element> for $name {
				fn new(id: i64, items: Vec<Option<$element>>) -> Self {
					Self { id, items }
				}
				fn items(&self) -> &[Option<$element>] {
					&self.items
				}
				fn id_field() -> FieldRef<Self, i64, GeneratedModelField> {
					Self::field_id()
				}
				fn items_field() -> FieldRef<Self, Vec<Option<$element>>, GeneratedModelField> {
					Self::field_items()
				}
			}
		};
	}

	nullable_array_model!(StringArrayModel, String, "nullable_string_arrays");
	nullable_array_model!(IntArrayModel, i32, "nullable_int_arrays");
	nullable_array_model!(BigIntArrayModel, i64, "nullable_bigint_arrays");
	nullable_array_model!(BoolArrayModel, bool, "nullable_bool_arrays");
	nullable_array_model!(FloatArrayModel, f32, "nullable_float_arrays");
	nullable_array_model!(DoubleArrayModel, f64, "nullable_double_arrays");
	nullable_array_model!(UuidArrayModel, uuid::Uuid, "nullable_uuid_arrays");

	type FloatArrayConversions<T> = (
		fn(Vec<T>) -> QueryValue,
		fn(Vec<Option<T>>) -> QueryValue,
		fn(T) -> f64,
		&'static str,
	);
	const REAL_CONVERSIONS: FloatArrayConversions<f32> = (
		QueryValue::FloatArray,
		QueryValue::NullableFloatArray,
		f64::from,
		"f32",
	);
	const DOUBLE_CONVERSIONS: FloatArrayConversions<f64> = (
		QueryValue::DoubleArray,
		QueryValue::NullableDoubleArray,
		std::convert::identity,
		"f64",
	);

	#[rstest]
	#[case::string(PhantomData::<StringArrayModel>, ColumnType::Text, ["kept".to_owned(), "quote'tail".to_owned()])]
	#[case::int(PhantomData::<IntArrayModel>, ColumnType::Integer, [i32::MIN, i32::MAX])]
	#[case::bigint(PhantomData::<BigIntArrayModel>, ColumnType::BigInteger, [i64::MIN, i64::MAX])]
	#[case::bool(PhantomData::<BoolArrayModel>, ColumnType::Boolean, [true, false])]
	#[case::float(PhantomData::<FloatArrayModel>, ColumnType::Float, [1.5_f32, -2.5_f32])]
	#[case::double(PhantomData::<DoubleArrayModel>, ColumnType::Double, [3.5_f64, -4.5_f64])]
	#[case::uuid(PhantomData::<UuidArrayModel>, ColumnType::Uuid, [uuid::Uuid::nil(), uuid::Uuid::from_u128(1)])]
	#[tokio::test]
	#[serial_test::serial(nullable_array_default_database)]
	async fn bulk_updates_preserve_nullable_array_elements<M, T>(
		#[future] postgres_arrays: ArrayFixture,
		#[case] _model: PhantomData<M>,
		#[case] column_type: ColumnType,
		#[case] values: [T; 2],
	) where
		M: NullableArrayRecord<T> + std::fmt::Debug + PartialEq,
		T: Clone
			+ std::fmt::Debug
			+ PartialEq
			+ sqlx::Type<sqlx::Postgres>
			+ sqlx::postgres::PgHasArrayType
			+ for<'r> sqlx::Decode<'r, sqlx::Postgres>
			+ Send
			+ Unpin,
	{
		// Arrange
		let fixture = postgres_arrays.await;
		let schema = Query::create_table()
			.table(M::table_name())
			.col(ColumnDef::new("id").big_integer().primary_key(true))
			.col(ColumnDef::new("items").array(column_type))
			.to_string(PostgresQueryBuilder);
		fixture.backend.execute(&schema, vec![]).await.unwrap();
		let owner = BackendsConnection::new(Arc::new(PostgresBackend::new(
			fixture.backend.pool().clone(),
		)));
		let lease = DatabaseConnectionLease::register(owner).unwrap();
		let mut connection = lease.handle();
		let _registration =
			reinhardt_db::orm::manager::install_scoped_database(&fixture.database_url)
				.await
				.unwrap();
		let manager = Manager::<M>::new();
		let shapes = [
			vec![None, None],
			vec![
				None,
				Some(values[0].clone()),
				None,
				Some(values[1].clone()),
				None,
			],
			vec![],
			values.iter().cloned().map(Some).collect(),
		];
		for id in 1..=4 {
			manager
				.create_with_conn(&mut connection, &M::new(id, vec![]))
				.await
				.unwrap();
		}

		for use_global_connection in [false, true] {
			let updated_models = shapes
				.iter()
				.enumerate()
				.map(|(index, _)| {
					let offset = usize::from(use_global_connection);
					M::new(
						index as i64 + 1,
						shapes[(index + offset) % shapes.len()].clone(),
					)
				})
				.collect::<Vec<_>>();

			// Act: both public bulk-update paths render CASE array literals.
			let count = if use_global_connection {
				manager
					.bulk_update(updated_models.clone(), vec!["items".to_owned()], Some(2))
					.await
			} else {
				manager
					.bulk_update_with_conn(
						&mut connection,
						updated_models.clone(),
						vec!["items".to_owned()],
						Some(2),
					)
					.await
			}
			.expect("typed nullable-array bulk update should succeed");

			// Assert: typed SQLx reads observe every shape after each bulk update.
			assert_eq!(count, updated_models.len());
			for model in updated_models {
				let sql = Query::select()
					.column("items")
					.from(M::table_name())
					.and_where(Expr::col("id").eq(model.primary_key().unwrap()))
					.to_string(PostgresQueryBuilder);
				let stored: Vec<Option<T>> = sqlx::query_scalar(&sql)
					.fetch_one(fixture.backend.pool())
					.await
					.unwrap();
				assert_eq!(stored, model.items());
			}
		}
	}

	#[rstest]
	#[case::real(PhantomData::<FloatArrayModel>, ColumnType::Float, [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1.5], REAL_CONVERSIONS)]
	#[case::double(PhantomData::<DoubleArrayModel>, ColumnType::Double, [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.5], DOUBLE_CONVERSIONS)]
	#[tokio::test]
	async fn non_finite_arrays_reject_derived_model_hydration<M, T>(
		#[future] postgres_arrays: ArrayFixture,
		#[case] _model: PhantomData<M>,
		#[case] column_type: ColumnType,
		#[case] values: [T; 4],
		#[case] conversions: FloatArrayConversions<T>,
	) where
		M: NullableArrayRecord<T> + std::fmt::Debug,
		T: Clone
			+ std::fmt::Debug
			+ serde::de::DeserializeOwned
			+ sqlx::Type<sqlx::Postgres>
			+ sqlx::postgres::PgHasArrayType
			+ for<'r> sqlx::Decode<'r, sqlx::Postgres>
			+ Send
			+ Unpin,
	{
		// Arrange
		let fixture = postgres_arrays.await;
		let (non_nullable, nullable, to_f64, expected_type) = conversions;
		let schema = Query::create_table()
			.table(M::table_name())
			.col(ColumnDef::new("id").big_integer().primary_key(true))
			.col(ColumnDef::new("items").array(column_type))
			.to_string(PostgresQueryBuilder);
		fixture.backend.execute(&schema, vec![]).await.unwrap();
		let mut transaction = fixture.backend.begin().await.unwrap();
		for has_null in [false, true] {
			let id = i64::from(has_null) + 1;
			let items = if has_null {
				nullable(vec![
					Some(values[0].clone()),
					None,
					Some(values[1].clone()),
					Some(values[2].clone()),
					Some(values[3].clone()),
				])
			} else {
				non_nullable(values.to_vec())
			};
			let (sql, _) = Query::insert()
				.into_table(M::table_name())
				.columns(["id", "items"])
				.values(vec![Value::from(id), Value::from(0_i32)])
				.unwrap()
				.build(PostgresQueryBuilder);
			fixture
				.backend
				.execute(&sql, vec![QueryValue::Int(id), items])
				.await
				.unwrap();
			let sql = Query::select()
				.column("items")
				.from(M::table_name())
				.and_where(Expr::col("id").eq(id))
				.to_string(PostgresQueryBuilder);
			let raw: Vec<Option<T>> = sqlx::query_scalar(&sql)
				.fetch_one(fixture.backend.pool())
				.await
				.unwrap();

			// Assert: PostgreSQL stores non-finite values separately from NULL.
			let offset = usize::from(has_null);
			assert!(to_f64(raw[0].clone().unwrap()).is_nan());
			assert_eq!(to_f64(raw[1 + offset].clone().unwrap()), f64::INFINITY);
			assert_eq!(to_f64(raw[2 + offset].clone().unwrap()), f64::NEG_INFINITY);
			assert_eq!(to_f64(raw[3 + offset].clone().unwrap()), 1.5);
			if has_null {
				assert!(raw[1].is_none());
			}

			// Act
			let row = fixture.backend.fetch_one(&sql, vec![]).await.unwrap();
			let query_row = QueryRow::from_backend_row(row);
			let result = Manager::<M>::new()
				.all()
				.filter(M::id_field().eq(id))
				.all_with_executor(transaction.as_mut())
				.await;

			// Assert
			assert_eq!(query_row.data["items"][0], "NaN");
			assert!(query_row.get::<Vec<Option<T>>>("items").is_none());
			let error = result.expect_err(
				"non-finite JSON hydration must fail instead of returning NULL elements",
			);
			assert_eq!(
				error.kind(),
				reinhardt_core::exception::DatabaseErrorKind::Serialization
			);
			assert!(
				error
					.to_string()
					.contains(&format!("expected {expected_type}")),
				"{error}"
			);
		}
		transaction.rollback().await.unwrap();
	}

	#[rstest]
	#[case::string(PhantomData::<StringArrayModel>, ColumnType::Text, ["kept".to_owned(), "tail".to_owned()])]
	#[case::int(PhantomData::<IntArrayModel>, ColumnType::Integer, [i32::MIN, i32::MAX])]
	#[case::bigint(PhantomData::<BigIntArrayModel>, ColumnType::BigInteger, [i64::MIN, i64::MAX])]
	#[case::bool(PhantomData::<BoolArrayModel>, ColumnType::Boolean, [true, false])]
	#[case::float(PhantomData::<FloatArrayModel>, ColumnType::Float, [1.5_f32, -2.5_f32])]
	#[case::double(PhantomData::<DoubleArrayModel>, ColumnType::Double, [3.5_f64, -4.5_f64])]
	#[case::uuid(PhantomData::<UuidArrayModel>, ColumnType::Uuid, [uuid::Uuid::nil(), uuid::Uuid::from_u128(1)])]
	#[tokio::test]
	async fn derived_models_preserve_nullable_array_elements<M, T>(
		#[future] postgres_arrays: ArrayFixture,
		#[case] _model: PhantomData<M>,
		#[case] column_type: ColumnType,
		#[case] values: [T; 2],
	) where
		M: NullableArrayRecord<T> + std::fmt::Debug + PartialEq,
		Vec<Option<T>>: DatabaseField<Storage = Vec<Option<T>>> + DatabaseScalar,
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
			.table(M::table_name())
			.col(ColumnDef::new("id").big_integer().primary_key(true))
			.col(ColumnDef::new("items").array(column_type))
			.to_string(PostgresQueryBuilder);
		fixture.backend.execute(&schema, vec![]).await.unwrap();
		let owner = BackendsConnection::new(Arc::new(PostgresBackend::new(
			fixture.backend.pool().clone(),
		)));
		let lease = DatabaseConnectionLease::register(owner).unwrap();
		let mut connection = lease.handle();
		let manager = Manager::<M>::new();
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
		let mut stored_models = Vec::new();

		for (index, items) in shapes.iter().enumerate() {
			let id = index as i64 + 1;
			let model = M::new(id, items.clone());
			let transaction_model = M::new(id + 100, items.clone());
			let updated_items = shapes[(index + 1) % shapes.len()].clone();
			let updated_model = M::new(id, updated_items.clone());
			let transaction_updated_model = M::new(id + 100, updated_items.clone());

			// Act: derived codecs drive both Manager conversion and row hydration.
			let created = manager
				.create_with_conn(&mut connection, &model)
				.await
				.unwrap();
			let transaction_created = manager
				.insert_with_executor(transaction.as_mut(), &transaction_model)
				.await
				.unwrap();
			let updated = manager
				.update_with_conn(&mut connection, &updated_model)
				.await
				.unwrap();
			let transaction_updated = manager
				.save_with_executor(transaction.as_mut(), &transaction_updated_model)
				.await
				.unwrap();

			// Assert
			assert_eq!(created, model);
			assert_eq!(transaction_created, transaction_model);
			assert_eq!(updated, updated_model);
			assert_eq!(transaction_updated, transaction_updated_model);

			// Act: typed QuerySet filters use the shared execution conversion.
			let matching = Manager::<M>::new()
				.all()
				.filter(M::id_field().eq(id + 100))
				.filter(M::items_field().eq(updated_items.clone()))
				.all_with_executor(transaction.as_mut())
				.await
				.unwrap();

			// Assert
			assert_eq!(matching, vec![transaction_updated_model.clone()]);

			// Act: both upsert insert and update bind arrays through bound_sql.
			let (
				(upsert_created, was_created),
				(upsert_updated, was_updated_created),
				(get_or_created, was_get_created),
			) = connection
				.atomic_write(async |upsert_transaction| {
					let created = Manager::<M>::new()
						.update_or_create()
						.lookup(M::id_field(), id + 200)
						.set(M::items_field(), items.clone())
						.execute_with(upsert_transaction)
						.await?;
					let updated = Manager::<M>::new()
						.update_or_create()
						.lookup(M::id_field(), id + 200)
						.set(M::items_field(), updated_items.clone())
						.execute_with(upsert_transaction)
						.await?;
					let get_or_created = Manager::<M>::new()
						.get_or_create()
						.lookup(M::id_field(), id + 300)
						.default(M::items_field(), items.clone())
						.execute_with(upsert_transaction)
						.await?;
					Ok::<_, reinhardt_core::exception::Error>((created, updated, get_or_created))
				})
				.await
				.unwrap();

			// Assert
			assert!(was_created);
			assert!(!was_updated_created);
			assert!(was_get_created);
			assert_eq!(upsert_created, M::new(id + 200, items.clone()));
			assert_eq!(upsert_updated, M::new(id + 200, updated_items));
			assert_eq!(get_or_created, M::new(id + 300, items.clone()));
			stored_models.extend([
				updated_model,
				transaction_updated_model,
				upsert_updated,
				get_or_created,
			]);
		}
		transaction.commit().await.unwrap();

		// Assert: an independent typed SQLx read observes every committed array.
		for model in stored_models {
			let id = model.primary_key().unwrap();
			let sql = Query::select()
				.column("items")
				.from(M::table_name())
				.and_where(Expr::col("id").eq(id))
				.to_string(PostgresQueryBuilder);
			let stored: Vec<Option<T>> = sqlx::query_scalar(&sql)
				.fetch_one(fixture.backend.pool())
				.await
				.unwrap();
			assert_eq!(stored, model.items(), "SQLx oracle for model {id}");
		}
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
