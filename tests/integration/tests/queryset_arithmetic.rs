//! Legacy QuerySet arithmetic retains column identity, grouping, and native binds.

#![cfg(native)]

use std::collections::HashMap;
use std::sync::Arc;

use reinhardt_db::orm::annotation::{
	Annotation, AnnotationValue as AV, Expression, Value as Scalar, When,
};
use reinhardt_db::orm::expressions::{F, Q};
use reinhardt_db::orm::inspection::FieldInfo;
use reinhardt_db::orm::model::FieldSelector;
use reinhardt_db::orm::query::{Filter, FilterOperator, FilterValue, UpdateValue};
use reinhardt_db::orm::query_types::DbBackend;
use reinhardt_db::orm::session::Session;
use reinhardt_db::orm::{Manager, Model, QuerySet};
use reinhardt_query::prelude::{
	MySqlQueryBuilder, PostgresQueryBuilder, QueryBuilder, SqliteQueryBuilder, Values,
};
use reinhardt_query::value::Value;
use reinhardt_test::fixtures::{mysql_container, postgres_container};
use rstest::*;
use serde::{Deserialize, Serialize};
use sqlx::any::AnyPoolOptions;
use testcontainers::{ContainerAsync, GenericImage};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Item {
	id: i64,
}

#[derive(Clone)]
struct ItemFields;

impl FieldSelector for ItemFields {
	fn with_alias(self, _alias: &str) -> Self {
		self
	}
}

impl Model for Item {
	type PrimaryKey = i64;
	type Fields = ItemFields;
	type Objects = Manager<Self>;

	fn table_name() -> &'static str {
		"session_insensitive"
	}

	fn primary_key(&self) -> Option<i64> {
		Some(self.id)
	}

	fn set_primary_key(&mut self, value: i64) {
		self.id = value;
	}

	fn new_fields() -> ItemFields {
		ItemFields
	}

	fn field_metadata() -> Vec<FieldInfo> {
		vec![FieldInfo {
			name: "id".into(),
			field_type: "reinhardt.orm.models.BigIntegerField".into(),
			nullable: false,
			primary_key: true,
			unique: true,
			blank: false,
			editable: true,
			default: None,
			db_default: None,
			db_column: None,
			choices: None,
			attributes: HashMap::new(),
		}]
	}
}

struct ArithmeticDatabase {
	pool: Arc<sqlx::AnyPool>,
	_container: Option<ContainerAsync<GenericImage>>,
}

#[fixture]
async fn arithmetic_database(
	#[default(DbBackend::Sqlite)] backend: DbBackend,
) -> ArithmeticDatabase {
	let (url, container) = match backend {
		DbBackend::Mysql => {
			let (container, _pool, _port, url) = mysql_container().await;
			(url, Some(container))
		}
		DbBackend::Postgres => {
			let (container, _pool, _port, url) = postgres_container().await;
			(url, Some(container))
		}
		DbBackend::Sqlite => ("sqlite::memory:".into(), None),
	};
	sqlx::any::install_default_drivers();
	let database = ArithmeticDatabase {
		pool: Arc::new(
			AnyPoolOptions::new()
				.max_connections(1)
				.connect(&url)
				.await
				.unwrap(),
		),
		_container: container,
	};
	sqlx::query("CREATE TABLE session_insensitive (id BIGINT PRIMARY KEY, name TEXT)")
		.execute(database.pool.as_ref())
		.await
		.unwrap();
	let insert = match backend {
		DbBackend::Postgres => "INSERT INTO session_insensitive (id, name) VALUES ($1, $2)",
		_ => "INSERT INTO session_insensitive (id, name) VALUES (?, ?)",
	};
	sqlx::query(insert)
		.bind(1_i64)
		.bind("quotes' \\ ? $7 雪")
		.execute(database.pool.as_ref())
		.await
		.unwrap();
	database
}

fn field() -> Box<AV> {
	Box::new(AV::Field(F::new("id")))
}

fn number(value: i64) -> Box<AV> {
	Box::new(AV::Value(Scalar::Int(value)))
}

fn nested_identity() -> Expression {
	Expression::Subtract(
		Box::new(AV::Expression(Expression::Divide(
			Box::new(AV::Expression(Expression::Multiply(
				Box::new(AV::Expression(Expression::Add(field(), number(7)))),
				number(2),
			))),
			number(2),
		))),
		number(7),
	)
}

fn case_identity() -> Expression {
	Expression::Case {
		whens: vec![When::new(
			Q::empty(),
			AV::Expression(Expression::Add(field(), number(0))),
		)],
		default: Some(number(0)),
	}
}

async fn assert_native_arithmetic(database: &ArithmeticDatabase, backend: DbBackend) {
	// Arrange: each arithmetic node must retain its grouping and typed constants.
	let reader = Session::new(database.pool.clone(), backend).await.unwrap();
	for (expression, expected) in [
		(Expression::Add(field(), number(0)), vec![Item { id: 1 }]),
		(
			Expression::Subtract(field(), number(0)),
			vec![Item { id: 1 }],
		),
		(
			Expression::Multiply(field(), number(1)),
			vec![Item { id: 1 }],
		),
		(Expression::Divide(field(), number(1)), vec![Item { id: 1 }]),
		(nested_identity(), vec![Item { id: 1 }]),
		(case_identity(), vec![Item { id: 1 }]),
		(Expression::Add(field(), number(1)), vec![]),
		(
			Expression::Coalesce(vec![AV::Value(Scalar::Null), *field()]),
			vec![Item { id: 1 }],
		),
	] {
		let queryset = QuerySet::<Item>::new().filter(Filter::new(
			"id",
			FilterOperator::Eq,
			FilterValue::Expression(expression),
		));
		// Act / Assert: Session executes the native backend's generated SELECT.
		assert_eq!(reader.list(&queryset).await.unwrap(), expected);
	}
	let text = QuerySet::<Item>::new().filter(Filter::new(
		"name",
		FilterOperator::Eq,
		FilterValue::Expression(Expression::Coalesce(vec![
			AV::Value(Scalar::Null),
			AV::Value(Scalar::String("quotes' \\ ? $7 雪".into())),
		])),
	));
	assert_eq!(reader.list(&text).await.unwrap(), vec![Item { id: 1 }]);
	for expression in [Expression::Add(field(), number(0)), case_identity()] {
		let annotated = QuerySet::<Item>::new()
			.annotate(Annotation::new("identity", AV::Expression(expression)));
		assert_eq!(reader.list(&annotated).await.unwrap(), vec![Item { id: 1 }]);
	}

	for (expression, operands) in [
		(Expression::Add(field(), number(0)), vec![0_i64]),
		(Expression::Subtract(field(), number(0)), vec![0_i64]),
		(Expression::Multiply(field(), number(1)), vec![1_i64]),
		(Expression::Divide(field(), number(1)), vec![1_i64]),
		(nested_identity(), vec![7_i64, 2, 2, 7]),
		(case_identity(), vec![0_i64]),
	] {
		let queryset = QuerySet::<Item>::new().filter(Filter::new(
			"id",
			FilterOperator::Eq,
			FilterValue::Integer(1),
		));
		let statement = queryset.update_query(&HashMap::from([(
			"id".into(),
			UpdateValue::Expression(expression),
		)]));
		let (sql, values) = match backend {
			DbBackend::Postgres => PostgresQueryBuilder.build_update(&statement),
			DbBackend::Mysql => MySqlQueryBuilder.build_update(&statement),
			DbBackend::Sqlite => SqliteQueryBuilder.build_update(&statement),
		};
		let expected = operands
			.into_iter()
			.chain([1_i64])
			.map(Into::into)
			.collect();
		assert_eq!(values, Values(expected));
		let mut query = sqlx::query(&sql);
		for value in values.0 {
			let Value::BigInt(Some(value)) = value else {
				panic!("arithmetic binds must remain i64")
			};
			query = query.bind(value);
		}
		// Act / Assert: bind each generated argument once, including the WHERE key.
		assert_eq!(
			query
				.execute(database.pool.as_ref())
				.await
				.unwrap()
				.rows_affected(),
			1
		);
		assert_eq!(reader.list(&queryset).await.unwrap(), vec![Item { id: 1 }]);
	}
}

async fn assert_case_conditions_bind(database: &ArithmeticDatabase, backend: DbBackend) {
	// Arrange: CASE predicates must compare runtime text with the stored name as data.
	let reader = Session::new(database.pool.clone(), backend).await.unwrap();
	let stored = "quotes' \\ ? $7 雪";
	for (condition, expected) in [
		(Q::new("name", "=", "missing"), vec![]),
		(Q::new("name", "=", "x' OR 1=1 OR 'x"), vec![]),
		(Q::new("name", "=", stored), vec![Item { id: 1 }]),
		(Q::new("name", "!=", stored), vec![]),
		(
			Q::new("name", "IN", format!("(missing, {stored})")),
			vec![Item { id: 1 }],
		),
		(Q::empty(), vec![Item { id: 1 }]),
		(Q::from_raw_sql("1 = 1"), vec![Item { id: 1 }]),
	] {
		let queryset = QuerySet::<Item>::new().filter(Filter::new(
			"id",
			FilterOperator::Eq,
			FilterValue::Expression(Expression::Case {
				// Typed arithmetic keeps the THEN column backend-quoted, as in case_identity().
				whens: vec![When::new(
					condition.clone(),
					AV::Expression(Expression::Add(field(), number(0))),
				)],
				default: Some(number(0)),
			}),
		));
		// Act / Assert: compare the complete result, including the rejected payload.
		assert_eq!(
			reader.list(&queryset).await.unwrap(),
			expected,
			"CASE condition: {condition:?}"
		);
	}

	// CASE result text in UPDATE binds too; the second pass restores the fixture.
	for name in [format!("{stored} case"), stored.to_owned()] {
		let statement = QuerySet::<Item>::new()
			.filter(Filter::new(
				"id",
				FilterOperator::Eq,
				FilterValue::Integer(1),
			))
			.update_query(&HashMap::from([(
				"name".into(),
				UpdateValue::Expression(Expression::Case {
					whens: vec![When::new(
						Q::new("id", "=", "1"),
						AV::Value(Scalar::String(name.clone())),
					)],
					default: None,
				}),
			)]));
		let (sql, values) = match backend {
			DbBackend::Postgres => PostgresQueryBuilder.build_update(&statement),
			DbBackend::Mysql => MySqlQueryBuilder.build_update(&statement),
			DbBackend::Sqlite => SqliteQueryBuilder.build_update(&statement),
		};
		let texts = values
			.0
			.iter()
			.filter(|value| matches!(value, Value::String(_)))
			.cloned()
			.collect::<Vec<_>>();
		assert_eq!(texts, vec![Value::from(name.clone())], "{sql}");
		let mut query = sqlx::query(&sql);
		for value in values.0 {
			query = match value {
				Value::BigInt(Some(value)) => query.bind(value),
				Value::String(Some(value)) => query.bind(*value),
				other => panic!("unexpected CASE argument: {other:?}"),
			};
		}
		// Act
		let affected = query
			.execute(database.pool.as_ref())
			.await
			.unwrap()
			.rows_affected();
		// Assert: the stored text equals the bound result exactly.
		assert_eq!(affected, 1);
		let renamed = QuerySet::<Item>::new().filter(Filter::new(
			"name",
			FilterOperator::Eq,
			FilterValue::String(name.clone()),
		));
		assert_eq!(reader.list(&renamed).await.unwrap(), vec![Item { id: 1 }]);
	}
}

#[rstest]
#[case::mysql(DbBackend::Mysql)]
#[case::postgres(DbBackend::Postgres)]
#[case::sqlite(DbBackend::Sqlite)]
#[tokio::test]
async fn legacy_case_conditions_execute_with_bound_runtime_values(
	#[case] backend: DbBackend,
	#[with(backend)]
	#[future]
	arithmetic_database: ArithmeticDatabase,
) {
	// Arrange: the fixture owns the disposable database and its single-connection pool.
	let database = arithmetic_database.await;
	// Act / Assert
	assert_case_conditions_bind(&database, backend).await;
	if backend == DbBackend::Mysql {
		// Fixed modes affect only this RAII-owned container and connection.
		for mode in [
			"SET SESSION sql_mode = 'NO_BACKSLASH_ESCAPES'",
			"SET SESSION sql_mode = ''",
		] {
			sqlx::query(mode)
				.execute(database.pool.as_ref())
				.await
				.unwrap();
			assert_case_conditions_bind(&database, backend).await;
		}
	}
}

#[rstest]
#[case::mysql(DbBackend::Mysql)]
#[case::postgres(DbBackend::Postgres)]
#[case::sqlite(DbBackend::Sqlite)]
#[tokio::test]
async fn legacy_arithmetic_executes_with_native_quotes_and_binds(
	#[case] backend: DbBackend,
	#[with(backend)]
	#[future]
	arithmetic_database: ArithmeticDatabase,
) {
	// Arrange: the fixture owns the disposable database and its single-connection pool.
	let database = arithmetic_database.await;
	let control: (i64,) = sqlx::query_as("SELECT id FROM session_insensitive WHERE id = (id + 0)")
		.fetch_one(database.pool.as_ref())
		.await
		.unwrap();
	assert_eq!(control.0, 1);
	// Act / Assert
	assert_native_arithmetic(&database, backend).await;
	if backend == DbBackend::Mysql {
		let mode: (String,) = sqlx::query_as("SELECT @@SESSION.sql_mode")
			.fetch_one(database.pool.as_ref())
			.await
			.unwrap();
		assert!(mode.0.contains("STRICT_"));
		// Fixed modes affect only this RAII-owned container and connection.
		for mode in [
			"SET SESSION sql_mode = 'NO_BACKSLASH_ESCAPES'",
			"SET SESSION sql_mode = ''",
		] {
			sqlx::query(mode)
				.execute(database.pool.as_ref())
				.await
				.unwrap();
			assert_native_arithmetic(&database, backend).await;
		}
	}
}
