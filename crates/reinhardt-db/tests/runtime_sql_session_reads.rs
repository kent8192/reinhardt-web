//! Session DML retains quoted identifiers, canonical values and transaction ownership.
#![cfg(all(feature = "postgres", feature = "mysql", feature = "sqlite"))]

use reinhardt_db::orm::inspection::FieldInfo;
use reinhardt_db::orm::json::Json;
use reinhardt_db::orm::query_types::DbBackend;
use reinhardt_db::orm::session::Session;
use reinhardt_db::orm::{
	DatabaseArrayType, DatabaseStorageKind, DatabaseValue, FieldCodecError, FieldSelector, Manager,
	Model,
};
use reinhardt_query::{
	Expr, ExprTrait, MySqlQueryBuilder, PostgresQueryBuilder, Query, SimpleExpr,
	SqliteQueryBuilder, Value,
};
use reinhardt_query_sqlx::{AnyBackend, prepare_any_with_text_codecs};
use rstest::rstest;
use serde::{Deserialize, Serialize};
use serial_test::serial;
use sqlx::AnyPool;
use std::{
	collections::{BTreeMap, HashMap},
	sync::Arc,
};
use testcontainers::{ImageExt, runners::AsyncRunner};

#[derive(Clone)]
struct Fields;
impl FieldSelector for Fields {
	fn with_alias(self, _alias: &str) -> Self {
		self
	}
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Record {
	id: i64,
	name: String,
	uid: uuid::Uuid,
	enabled: bool,
	amount: rust_decimal::Decimal,
	payload: Json<serde_json::Value>,
	optional_payload: Option<Json<serde_json::Value>>,
	items: Vec<Option<String>>,
	on_date: chrono::NaiveDate,
	on_time: chrono::NaiveTime,
	at: chrono::DateTime<chrono::Utc>,
	bytes: Vec<u8>,
}

impl Model for Record {
	type PrimaryKey = i64;
	type Fields = Fields;
	type Objects = Manager<Self>;
	fn table_name() -> &'static str {
		"session\"quoted`"
	}
	fn new_fields() -> Self::Fields {
		Fields
	}
	fn primary_key(&self) -> Option<i64> {
		Some(self.id)
	}
	fn set_primary_key(&mut self, id: i64) {
		self.id = id;
	}
	fn field_is_none(&self, field: &str) -> bool {
		field == "optional_payload" && self.optional_payload.is_none()
	}
	fn encode_database_fields(&self) -> Result<BTreeMap<String, DatabaseValue>, FieldCodecError> {
		Ok([
			("id", DatabaseValue::I64(self.id)),
			("name", DatabaseValue::String(self.name.clone())),
			("uid", DatabaseValue::Uuid(self.uid)),
			("enabled", DatabaseValue::Bool(self.enabled)),
			("amount", DatabaseValue::Decimal(self.amount)),
			("payload", DatabaseValue::Json(self.payload.0.clone())),
			(
				"optional_payload",
				self.optional_payload
					.as_ref()
					.map_or(DatabaseValue::Null, |value| {
						DatabaseValue::Json(value.0.clone())
					}),
			),
			(
				"items",
				DatabaseValue::Array {
					element_type: DatabaseArrayType::String,
					values: self
						.items
						.iter()
						.map(|value| {
							value.as_ref().map_or(DatabaseValue::Null, |value| {
								DatabaseValue::String(value.clone())
							})
						})
						.collect(),
				},
			),
			("on_date", DatabaseValue::Date(self.on_date)),
			("on_time", DatabaseValue::Time(self.on_time)),
			("at", DatabaseValue::DateTime(self.at)),
			("bytes", DatabaseValue::Bytes(self.bytes.clone())),
		]
		.into_iter()
		.map(|(name, value)| (name.into(), value))
		.collect())
	}
	fn field_metadata() -> Vec<FieldInfo> {
		[
			("id", "BigIntegerField", DatabaseStorageKind::I64),
			("name", "CharField", DatabaseStorageKind::String),
			("uid", "UUIDField", DatabaseStorageKind::Uuid),
			("enabled", "BooleanField", DatabaseStorageKind::Bool),
			("amount", "DecimalField", DatabaseStorageKind::Decimal),
			("payload", "JsonField", DatabaseStorageKind::Json),
			("optional_payload", "JsonField", DatabaseStorageKind::Json),
			("items", "ArrayField", DatabaseStorageKind::Json),
			("on_date", "DateField", DatabaseStorageKind::Date),
			("on_time", "TimeField", DatabaseStorageKind::Time),
			("at", "DateTimeField", DatabaseStorageKind::DateTime),
			("bytes", "BinaryField", DatabaseStorageKind::Bytes),
		]
		.into_iter()
		.map(|(name, field_type, storage)| FieldInfo {
			name: name.into(),
			field_type: field_type.into(),
			storage_kind: Some(storage),
			domain: None,
			nullable: name == "optional_payload",
			primary_key: name == "id",
			unique: false,
			blank: false,
			editable: true,
			default: None,
			db_default: None,
			db_column: match name {
				"id" => Some("pk\"key`".into()),
				"name" => Some("name\"value`".into()),
				_ => None,
			},
			choices: None,
			attributes: HashMap::new(),
		})
		.collect()
	}
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct MappedKey {
	id: i64,
	name: String,
}
impl Model for MappedKey {
	type PrimaryKey = i64;
	type Fields = Fields;
	type Objects = Manager<Self>;
	fn table_name() -> &'static str {
		"mapped_keys"
	}
	fn new_fields() -> Self::Fields {
		Fields
	}
	fn primary_key(&self) -> Option<i64> {
		Some(self.id - 100)
	}
	fn set_primary_key(&mut self, id: i64) {
		self.id = id + 100;
	}
	fn primary_key_database_value(id: &i64) -> Result<DatabaseValue, FieldCodecError> {
		Ok(DatabaseValue::I64(id + 100))
	}
	fn field_metadata() -> Vec<FieldInfo> {
		Record::field_metadata()
			.into_iter()
			.take(2)
			.map(|mut field| {
				field.db_column = None;
				field
			})
			.collect()
	}
}

fn quoted(backend: DbBackend, name: &str) -> String {
	match backend {
		DbBackend::Mysql => format!("`{}`", name.replace('`', "``")),
		_ => format!("\"{}\"", name.replace('"', "\"\"")),
	}
}

fn fixture_value(
	backend: DbBackend,
	value: Value,
	postgres_type: Option<&'static str>,
) -> SimpleExpr {
	let expression = Expr::val(value);
	if backend == DbBackend::Postgres
		&& let Some(name) = postgres_type
	{
		return expression.cast_as(name);
	}
	expression.into_simple_expr()
}

async fn exercise_reads(pool: AnyPool, backend: DbBackend) {
	// Arrange: test-owned schema and arguments provide an independent stored-value control.
	let (uid, amount, json, items, date, time, timestamp, bytes) = match backend {
		DbBackend::Postgres => (
			"UUID",
			"NUMERIC(30,12)",
			"JSONB",
			"TEXT[]",
			"DATE",
			"TIME(6)",
			"TIMESTAMPTZ",
			"BYTEA",
		),
		DbBackend::Mysql => (
			"CHAR(36)",
			"DECIMAL(30,12)",
			"JSON",
			"TEXT",
			"DATE",
			"TIME(6)",
			"DATETIME(6)",
			"BLOB",
		),
		DbBackend::Sqlite => (
			"TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "TEXT", "BLOB",
		),
	};
	let table = quoted(backend, Record::table_name());
	let key = quoted(backend, "pk\"key`");
	let name = quoted(backend, "name\"value`");
	let schema = format!(
		"CREATE TABLE {table} ({key} BIGINT PRIMARY KEY, {name} TEXT, uid {uid}, enabled BOOLEAN, amount {amount}, payload {json}, optional_payload {json}, items {items}, on_date {date}, on_time {time}, at {timestamp}, bytes {bytes})"
	);
	sqlx::query(&schema).execute(&pool).await.unwrap();
	sqlx::query("CREATE TABLE mapped_keys (id BIGINT PRIMARY KEY, name TEXT)")
		.execute(&pool)
		.await
		.unwrap();
	let expected = Record {
		id: 1_i64 << 40,
		name: format!("{}雪", "bound' ? $43 ".repeat(2_000)),
		uid: uuid::Uuid::parse_str("12345678-1234-5678-9abc-def012345678").unwrap(),
		enabled: true,
		amount: rust_decimal::Decimal::from_str_exact("123456789012.123456789012").unwrap(),
		payload: Json(serde_json::json!("scalar' ? $99")),
		optional_payload: Some(Json(serde_json::Value::Null)),
		items: vec![Some("alpha".into()), None],
		on_date: chrono::NaiveDate::from_ymd_opt(2026, 10, 4).unwrap(),
		on_time: chrono::NaiveTime::from_hms_micro_opt(5, 6, 7, 123_456).unwrap(),
		at: chrono::DateTime::parse_from_rfc3339("2026-10-04T05:06:07.123456Z")
			.unwrap()
			.with_timezone(&chrono::Utc),
		bytes: vec![0, 1, 127, 255],
	};
	let second = Record {
		id: expected.id + 1,
		optional_payload: None,
		..expected.clone()
	};
	for record in [&expected, &second] {
		let fields = Record::field_metadata();
		let optional = record
			.optional_payload
			.as_ref()
			.map_or(Value::String(None), |value| value.0.to_string().into());
		let values = [
			fixture_value(backend, record.id.into(), None),
			fixture_value(backend, record.name.clone().into(), None),
			fixture_value(backend, record.uid.to_string().into(), Some("uuid")),
			fixture_value(backend, record.enabled.into(), None),
			fixture_value(backend, record.amount.to_string().into(), Some("numeric")),
			fixture_value(backend, record.payload.0.to_string().into(), Some("jsonb")),
			fixture_value(backend, optional, Some("jsonb")),
			fixture_value(
				backend,
				if backend == DbBackend::Postgres {
					"{\"alpha\",NULL}".into()
				} else {
					"[\"alpha\",null]".into()
				},
				Some("_text"),
			),
			fixture_value(backend, record.on_date.to_string().into(), Some("date")),
			fixture_value(backend, record.on_time.to_string().into(), Some("time")),
			fixture_value(
				backend,
				if backend == DbBackend::Mysql {
					record.at.naive_utc().to_string().into()
				} else {
					record.at.to_rfc3339().into()
				},
				Some("timestamptz"),
			),
			fixture_value(
				backend,
				Value::Bytes(Some(Box::new(record.bytes.clone()))),
				None,
			),
		];
		let mut statement = Query::insert();
		statement
			.into_table(Record::table_name())
			.columns(
				fields
					.iter()
					.map(|field| reinhardt_query::Alias::new(field.db_column_name())),
			)
			.values_expr(values.into())
			.unwrap();
		let (built, adapter) = match backend {
			DbBackend::Postgres => (
				PostgresQueryBuilder
					.build_insert_checked(&statement)
					.unwrap(),
				AnyBackend::Postgres,
			),
			DbBackend::Mysql => (
				MySqlQueryBuilder.build_insert_checked(&statement).unwrap(),
				AnyBackend::MySql,
			),
			DbBackend::Sqlite => (
				SqliteQueryBuilder.build_insert_checked(&statement).unwrap(),
				AnyBackend::Sqlite,
			),
		};
		let (sql, arguments) = prepare_any_with_text_codecs(built, adapter)
			.unwrap()
			.into_parts();
		sqlx::query_with(&sql, arguments)
			.execute(&pool)
			.await
			.unwrap();
	}
	let mut insert = Query::insert();
	insert
		.into_table("mapped_keys")
		.columns(["id", "name"])
		.values_panic([Value::from(105_i64), "codec".into()]);
	let (built, adapter) = match backend {
		DbBackend::Postgres => (
			PostgresQueryBuilder.build_insert_checked(&insert).unwrap(),
			AnyBackend::Postgres,
		),
		DbBackend::Mysql => (
			MySqlQueryBuilder.build_insert_checked(&insert).unwrap(),
			AnyBackend::MySql,
		),
		DbBackend::Sqlite => (
			SqliteQueryBuilder.build_insert_checked(&insert).unwrap(),
			AnyBackend::Sqlite,
		),
	};
	let (sql, arguments) = prepare_any_with_text_codecs(built, adapter)
		.unwrap()
		.into_parts();
	sqlx::query_with(&sql, arguments)
		.execute(&pool)
		.await
		.unwrap();
	let pool = Arc::new(pool);
	let mut session = Session::new(pool.clone(), backend).await.unwrap();
	// Act / Assert: direct reads and identity hits retain all projected types.
	assert_eq!(
		session.get::<Record>(expected.id).await.unwrap(),
		Some(expected.clone())
	);
	assert_eq!(
		session.get::<Record>(expected.id).await.unwrap(),
		Some(expected.clone())
	);
	assert_eq!(
		session.get::<Record>(second.id).await.unwrap(),
		Some(second.clone())
	);
	assert_eq!(
		session.get::<Record>(second.id).await.unwrap(),
		Some(second.clone())
	);
	let mut all = session.list_all::<Record>().await.unwrap();
	all.sort_by_key(|record| record.id);
	assert_eq!(all, vec![expected.clone(), second.clone()]);
	let mut listed = session
		.list(&reinhardt_db::orm::QuerySet::<Record>::new())
		.await
		.unwrap();
	listed.sort_by_key(|record| record.id);
	assert_eq!(listed, vec![expected.clone(), second]);
	assert_eq!(session.get::<Record>(-1).await.unwrap(), None);
	assert_eq!(
		session.get::<MappedKey>(5).await.unwrap(),
		Some(MappedKey {
			id: 105,
			name: "codec".into()
		})
	);
	exercise_writes(pool, backend, expected).await;
}

fn keyed(id: i64) -> reinhardt_db::orm::QuerySet<Record> {
	use reinhardt_db::orm::query::{Filter, FilterOperator, FilterValue};
	reinhardt_db::orm::QuerySet::new().filter(Filter::new(
		"pk\"key`",
		FilterOperator::Eq,
		FilterValue::Integer(id),
	))
}

async fn exercise_writes(pool: Arc<AnyPool>, backend: DbBackend, original: Record) {
	// Arrange: use one caller-owned transaction for reads, locks and every write.
	let mut writer = Session::new(pool.clone(), backend).await.unwrap();
	let reader = Session::new(pool.clone(), backend).await.unwrap();
	let mut transaction = pool.begin().await.unwrap();
	let inserted = Record {
		id: original.id + 2,
		name: "Insert' $50 ? %_\\雪 Tail42 12.5 True".into(),
		items: vec![Some("quote\" slash\\ dollar$8 comma,".into()), None],
		optional_payload: None,
		..original.clone()
	};
	// Act / Assert: verify stored values through a separate reader on the same connection.
	writer.add_new(inserted.clone()).await.unwrap();
	writer
		.flush_with_connection(&mut transaction)
		.await
		.unwrap();
	assert_eq!(
		reader
			.list_with_connection(&keyed(inserted.id), &mut transaction)
			.await
			.unwrap(),
		vec![inserted.clone()]
	);
	if backend != DbBackend::Sqlite {
		assert_eq!(
			reader
				.list_with_connection_for_update(&keyed(inserted.id), &mut transaction)
				.await
				.unwrap(),
			vec![inserted.clone()]
		);
	}
	assert_insensitive_matches(&reader, &mut transaction, &inserted).await;
	assert_range_matches(&reader, &mut transaction, &inserted).await;
	assert_expression_matches(&reader, &mut transaction, backend, &inserted).await;
	if backend == DbBackend::Mysql {
		// This connection belongs to a disposable container; its guard owns cleanup.
		sqlx::query("SET SESSION sql_mode = 'NO_BACKSLASH_ESCAPES'")
			.execute(&mut *transaction)
			.await
			.unwrap();
		assert_insensitive_matches(&reader, &mut transaction, &inserted).await;
		assert_range_matches(&reader, &mut transaction, &inserted).await;
		assert_expression_matches(&reader, &mut transaction, backend, &inserted).await;
		sqlx::query("SET SESSION sql_mode = ''")
			.execute(&mut *transaction)
			.await
			.unwrap();
		assert_expression_matches(&reader, &mut transaction, backend, &inserted).await;
	}
	let updated = Record {
		name: "update' $70 ?".into(),
		enabled: false,
		payload: Json(serde_json::json!(["changed' $99", null])),
		optional_payload: Some(Json(serde_json::Value::Null)),
		..inserted
	};
	writer.add(updated.clone()).await.unwrap();
	writer
		.flush_with_connection(&mut transaction)
		.await
		.unwrap();
	assert_eq!(
		reader
			.list_with_connection(&keyed(updated.id), &mut transaction)
			.await
			.unwrap(),
		vec![updated.clone()]
	);
	writer.delete(updated.clone()).await.unwrap();
	writer
		.flush_with_connection(&mut transaction)
		.await
		.unwrap();
	assert!(
		reader
			.list_with_connection(&keyed(updated.id), &mut transaction)
			.await
			.unwrap()
			.is_empty()
	);
	let rolled_back = Record {
		id: original.id + 3,
		..original
	};
	writer.add_new(rolled_back.clone()).await.unwrap();
	writer
		.flush_with_connection(&mut transaction)
		.await
		.unwrap();
	assert_eq!(
		reader
			.list_with_connection(&keyed(rolled_back.id), &mut transaction)
			.await
			.unwrap(),
		vec![rolled_back.clone()]
	);
	transaction.rollback().await.unwrap();
	assert!(
		reader
			.list(&keyed(rolled_back.id))
			.await
			.unwrap()
			.is_empty()
	);
}

async fn assert_expression_matches(
	reader: &Session,
	connection: &mut sqlx::AnyConnection,
	backend: DbBackend,
	expected: &Record,
) {
	use reinhardt_db::orm::annotation::{AnnotationValue as AV, Expression, Value as Scalar};
	use reinhardt_db::orm::expressions::F;
	use reinhardt_db::orm::query::{Filter, FilterOperator, FilterValue, UpdateValue};
	// Arrange: nested AST operations must preserve arithmetic scope and quoted columns.
	let field = || Box::new(AV::Field(F::new("pk\"key`")));
	let number = |value| Box::new(AV::Value(Scalar::Int(value)));
	let nested = || {
		Expression::Divide(
			Box::new(AV::Expression(Expression::Multiply(
				Box::new(AV::Expression(Expression::Add(field(), number(7)))),
				number(2),
			))),
			number(2),
		)
	};
	for (expression, matched) in [
		(Expression::Add(field(), number(0)), true),
		(Expression::Subtract(field(), number(0)), true),
		(Expression::Multiply(field(), number(1)), true),
		(Expression::Divide(field(), number(1)), true),
		(
			Expression::Subtract(Box::new(AV::Expression(nested())), number(7)),
			true,
		),
		(Expression::Add(field(), number(1)), false),
		(
			Expression::Coalesce(vec![AV::Value(Scalar::Null), *field()]),
			true,
		),
	] {
		// Act / Assert: the same transaction connection executes generated SQL/Values.
		let query = keyed(expected.id).filter(Filter::new(
			"pk\"key`",
			FilterOperator::Eq,
			FilterValue::Expression(expression),
		));
		let records = reader
			.list_with_connection(&query, connection)
			.await
			.unwrap();
		assert_eq!(
			records,
			if matched {
				vec![expected.clone()]
			} else {
				vec![]
			}
		);
	}
	let text_query = keyed(expected.id).filter(Filter::new(
		"name\"value`",
		FilterOperator::Eq,
		FilterValue::Expression(Expression::Coalesce(vec![
			AV::Value(Scalar::Null),
			AV::Value(Scalar::String(expected.name.clone())),
		])),
	));
	assert_eq!(
		reader
			.list_with_connection(&text_query, connection)
			.await
			.unwrap(),
		vec![expected.clone()]
	);

	// Logical UPDATE fields resolve to physical db_column names before lowering.
	for name in [
		format!("{} updated' ? $89\\雪", expected.name),
		expected.name.clone(),
	] {
		let updates = HashMap::from([
			(
				"id".into(),
				UpdateValue::Expression(Expression::Divide(
					Box::new(AV::Expression(Expression::Multiply(
						Box::new(AV::Field(F::new("id"))),
						number(2),
					))),
					number(2),
				)),
			),
			(
				"name".into(),
				UpdateValue::Expression(Expression::Coalesce(vec![
					AV::Value(Scalar::Null),
					AV::Value(Scalar::String(name.clone())),
				])),
			),
		]);
		let statement = keyed(expected.id).update_query(&updates).unwrap();
		let (built, adapter) = match backend {
			DbBackend::Postgres => (
				PostgresQueryBuilder
					.build_update_checked(&statement)
					.unwrap(),
				AnyBackend::Postgres,
			),
			DbBackend::Mysql => (
				MySqlQueryBuilder.build_update_checked(&statement).unwrap(),
				AnyBackend::MySql,
			),
			DbBackend::Sqlite => (
				SqliteQueryBuilder.build_update_checked(&statement).unwrap(),
				AnyBackend::Sqlite,
			),
		};
		assert!(!built.0.contains(&name));
		let (sql, arguments) = prepare_any_with_text_codecs(built, adapter)
			.unwrap()
			.into_parts();
		let result = sqlx::query_with(&sql, arguments)
			.execute(&mut *connection)
			.await
			.unwrap();
		assert_eq!(result.rows_affected(), 1);
		assert_eq!(
			reader
				.list_with_connection(&keyed(expected.id), connection)
				.await
				.unwrap(),
			vec![Record {
				name,
				..expected.clone()
			}]
		);
	}
}

async fn assert_range_matches(
	reader: &Session,
	connection: &mut sqlx::AnyConnection,
	expected: &Record,
) {
	use reinhardt_db::orm::query::{Filter, FilterOperator, FilterValue};
	// Arrange / Act / Assert: inclusive boundaries use the same caller-owned transaction.
	for (lower, upper, matched) in [
		(expected.id, expected.id, true),
		(expected.id - 1, expected.id, true),
		(expected.id, expected.id + 1, true),
		(expected.id + 1, expected.id + 2, false),
	] {
		let query = keyed(expected.id).filter(Filter::new(
			"pk\"key`",
			FilterOperator::Range,
			FilterValue::Range(
				Box::new(FilterValue::Typed(Ok(DatabaseValue::I64(lower)))),
				Box::new(FilterValue::Integer(upper)),
			),
		));
		let records = reader
			.list_with_connection(&query, connection)
			.await
			.unwrap();
		let expected_rows = if matched {
			vec![expected.clone()]
		} else {
			vec![]
		};
		assert_eq!(records, expected_rows, "range: {lower}..={upper}");
	}
}

async fn assert_insensitive_matches(
	reader: &Session,
	connection: &mut sqlx::AnyConnection,
	expected: &Record,
) {
	use reinhardt_db::orm::query::{Filter, FilterOperator, FilterValue};
	// Arrange: test both ordinary and canonical values against a quoted text column.
	let cases = [
		(
			FilterOperator::IContains,
			FilterValue::String("%_\\雪".into()),
		),
		(
			FilterOperator::IStartsWith,
			FilterValue::String("INSERT' $50 ? %_\\雪".into()),
		),
		(
			FilterOperator::IEndsWith,
			FilterValue::String("TRUE".into()),
		),
		(
			FilterOperator::IExact,
			FilterValue::String(expected.name.to_ascii_uppercase()),
		),
		(
			FilterOperator::IContains,
			FilterValue::Typed(Ok(DatabaseValue::String("TAIL42".into()))),
		),
		(FilterOperator::IContains, FilterValue::Integer(42)),
		(FilterOperator::IContains, FilterValue::Float(12.5)),
		(FilterOperator::IEndsWith, FilterValue::Boolean(true)),
		(FilterOperator::IContains, FilterValue::Null),
	];
	for (operator, value) in cases {
		let query = keyed(expected.id).filter(Filter::new("name\"value`", operator.clone(), value));
		// Act
		let records = reader
			.list_with_connection(&query, connection)
			.await
			.unwrap();
		// Assert
		assert_eq!(records, vec![expected.clone()], "operator: {operator:?}");
	}
	// A wildcard in user input must not broaden matching to the stored text.
	for (operator, value) in [
		(FilterOperator::IContains, "%_\\missing"),
		(FilterOperator::IExact, "%"),
	] {
		let query = keyed(expected.id).filter(Filter::new(
			"name\"value`",
			operator,
			FilterValue::String(value.into()),
		));
		assert!(
			reader
				.list_with_connection(&query, connection)
				.await
				.unwrap()
				.is_empty()
		);
	}
}

#[rstest]
#[serial(sqlx_drivers)]
#[tokio::test]
async fn postgres_session_reads() {
	sqlx::any::install_default_drivers();
	let container = testcontainers_modules::postgres::Postgres::default()
		.start()
		.await
		.unwrap();
	let port = container.get_host_port_ipv4(5432).await.unwrap();
	let pool = sqlx::any::AnyPoolOptions::new()
		.max_connections(1)
		.connect(&format!(
			"postgres://postgres:postgres@127.0.0.1:{port}/postgres"
		))
		.await
		.unwrap();
	exercise_reads(pool, DbBackend::Postgres).await;
}

#[rstest]
#[serial(sqlx_drivers)]
#[tokio::test]
async fn mysql_session_reads() {
	sqlx::any::install_default_drivers();
	let container = testcontainers_modules::mysql::Mysql::default()
		.with_startup_timeout(std::time::Duration::from_secs(180))
		.start()
		.await
		.unwrap();
	let port = container.get_host_port_ipv4(3306).await.unwrap();
	let pool = sqlx::any::AnyPoolOptions::new()
		.max_connections(1)
		.connect(&format!("mysql://root@127.0.0.1:{port}/test"))
		.await
		.unwrap();
	exercise_reads(pool, DbBackend::Mysql).await;
}

#[rstest]
#[serial(sqlx_drivers)]
#[tokio::test]
async fn sqlite_session_reads() {
	sqlx::any::install_default_drivers();
	let pool = sqlx::any::AnyPoolOptions::new()
		.max_connections(1)
		.connect("sqlite::memory:")
		.await
		.unwrap();
	exercise_reads(pool, DbBackend::Sqlite).await;
}
