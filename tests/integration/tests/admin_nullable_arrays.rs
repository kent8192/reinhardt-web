//! Admin updates preserve nullable ORM array fields through PostgreSQL binding.

use reinhardt_admin::core::database::AdminDatabase;
use reinhardt_db::backends::{DatabaseBackend, DatabaseConnection, PostgresBackend};
use reinhardt_db::orm::{DatabaseConnectionLease, Manager, Model};
use reinhardt_query::{
	ColumnDef, ColumnType, Expr, ExprTrait, PostgresQueryBuilder, Query, QueryStatementBuilder,
};
use rstest::{fixture, rstest};
use serde::{Deserialize, Serialize};
use sqlx::{Row, postgres::PgPoolOptions};
use std::sync::Arc;
use testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner};
use testcontainers_modules::postgres::Postgres;

#[derive(reinhardt::Model, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[model(
	app_label = "admin_nullable_arrays",
	table_name = "admin_nullable_arrays"
)]
struct NullableArrays {
	#[field(primary_key = true)]
	id: i64,
	strings: Option<Vec<Option<String>>>,
	integers: Option<Vec<Option<i32>>>,
	big_integers: Option<Vec<Option<i64>>>,
	booleans: Option<Vec<Option<bool>>>,
	floats: Option<Vec<Option<f32>>>,
	doubles: Option<Vec<Option<f64>>>,
	uuids: Option<Vec<Option<uuid::Uuid>>>,
}

struct AdminArrayFixture {
	admin: AdminDatabase,
	backend: PostgresBackend,
	_lease: DatabaseConnectionLease,
	_container: ContainerAsync<Postgres>,
}

#[fixture]
async fn admin_arrays() -> AdminArrayFixture {
	let container = Postgres::default()
		.with_tag("17-alpine")
		.start()
		.await
		.unwrap();
	let host = container.get_host().await.unwrap();
	let port = container.get_host_port_ipv4(5432).await.unwrap();
	let pool = PgPoolOptions::new()
		.max_connections(2)
		.connect(&format!(
			"postgres://postgres:postgres@{host}:{port}/postgres"
		))
		.await
		.unwrap();
	let backend = PostgresBackend::new(pool);
	let lease = DatabaseConnectionLease::register(DatabaseConnection::new(Arc::new(
		PostgresBackend::new(backend.pool().clone()),
	)))
	.unwrap();
	AdminArrayFixture {
		admin: AdminDatabase::new(lease.handle()),
		backend,
		_lease: lease,
		_container: container,
	}
}

#[derive(Clone, Copy)]
enum Shape {
	WholeNull,
	Empty,
	AllNull,
	Mixed,
}

fn elements<T>(shape: Shape, value: T) -> Option<Vec<Option<T>>> {
	match shape {
		Shape::WholeNull => None,
		Shape::Empty => Some(vec![]),
		Shape::AllNull => Some(vec![None, None]),
		Shape::Mixed => Some(vec![None, Some(value), None]),
	}
}

#[rstest]
#[case::whole_null(Shape::WholeNull)]
#[case::empty(Shape::Empty)]
#[case::all_null(Shape::AllNull)]
#[case::mixed(Shape::Mixed)]
#[tokio::test]
async fn admin_updates_round_trip_nullable_array_fields(
	#[future] admin_arrays: AdminArrayFixture,
	#[case] shape: Shape,
) {
	// Arrange: the real derive registers migration types used by the admin editor.
	let fixture = admin_arrays.await;
	let mut schema = Query::create_table();
	schema
		.table(NullableArrays::table_name())
		.col(ColumnDef::new("id").big_integer().primary_key(true));
	for (name, element_type) in [
		("strings", ColumnType::Text),
		("integers", ColumnType::Integer),
		("big_integers", ColumnType::BigInteger),
		("booleans", ColumnType::Boolean),
		("floats", ColumnType::Float),
		("doubles", ColumnType::Double),
		("uuids", ColumnType::Uuid),
	] {
		schema.col(ColumnDef::new(name).array(element_type));
	}
	fixture
		.backend
		.execute(&schema.to_string(PostgresQueryBuilder), vec![])
		.await
		.unwrap();
	let expected = NullableArrays {
		id: 1,
		strings: elements(shape, "quote'tail".to_owned()),
		integers: elements(shape, i32::MAX),
		big_integers: elements(shape, i64::MAX),
		booleans: elements(shape, false),
		floats: elements(shape, 1.5),
		doubles: elements(shape, -2.5),
		uuids: elements(shape, uuid::Uuid::nil()),
	};
	let mut connection = *fixture.admin.connection();
	Manager::<NullableArrays>::new()
		.create_with_conn(&mut connection, &expected)
		.await
		.unwrap();

	// Act: submit the fetched field values unchanged, as the admin editor does.
	let mut data = fixture
		.admin
		.get::<NullableArrays>(NullableArrays::table_name(), "id", "1")
		.await
		.unwrap()
		.unwrap();
	data.remove("id");
	let json = serde_json::to_value(&expected).unwrap();
	for (field, value) in &data {
		assert_eq!(value, &json[field], "{field}");
	}
	assert_eq!(data.len(), 7);
	let affected = fixture
		.admin
		.update::<NullableArrays>(NullableArrays::table_name(), "id", "1", data)
		.await
		.expect("admin must retain nullable array elements");
	let invalid = fixture
		.admin
		.update::<NullableArrays>(
			NullableArrays::table_name(),
			"id",
			"1",
			std::collections::HashMap::from([(
				"integers".to_owned(),
				serde_json::json!([null, "invalid"]),
			)]),
		)
		.await
		.expect_err("non-NULL elements must still be type checked");
	assert!(matches!(
		invalid,
		reinhardt_admin::types::AdminError::ValidationError(_)
	));

	// Assert: native SQLx decoders prove that binding preserved every array shape.
	assert_eq!(affected, 1);
	let sql = Query::select()
		.column(reinhardt_query::ColumnRef::Asterisk)
		.from(NullableArrays::table_name())
		.and_where(Expr::col("id").eq(1_i64))
		.to_string(PostgresQueryBuilder);
	let row = sqlx::query(&sql)
		.fetch_one(fixture.backend.pool())
		.await
		.unwrap();
	macro_rules! assert_field {
		($name:ident, $element:ty) => {
			assert_eq!(
				row.try_get::<Option<Vec<Option<$element>>>, _>(stringify!($name))
					.unwrap(),
				expected.$name
			);
		};
	}
	assert_field!(strings, String);
	assert_field!(integers, i32);
	assert_field!(big_integers, i64);
	assert_field!(booleans, bool);
	assert_field!(floats, f32);
	assert_field!(doubles, f64);
	assert_field!(uuids, uuid::Uuid);
}
