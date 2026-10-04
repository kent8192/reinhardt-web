//! Session reads retain quoted identifiers, canonical keys and Any projections.
#![cfg(all(feature = "postgres", feature = "mysql", feature = "sqlite"))]

use reinhardt_db::orm::inspection::FieldInfo;
use reinhardt_db::orm::json::Json;
use reinhardt_db::orm::query_types::DbBackend;
use reinhardt_db::orm::session::Session;
use reinhardt_db::orm::{
	DatabaseStorageKind, DatabaseValue, FieldCodecError, FieldSelector, Manager, Model,
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
use std::{collections::HashMap, sync::Arc};
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
	let mut session = Session::new(Arc::new(pool), backend).await.unwrap();
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
	assert_eq!(listed, vec![expected, second]);
	assert_eq!(session.get::<Record>(-1).await.unwrap(), None);
	assert_eq!(
		session.get::<MappedKey>(5).await.unwrap(),
		Some(MappedKey {
			id: 105,
			name: "codec".into()
		})
	);
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
