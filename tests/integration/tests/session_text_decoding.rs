//! Session text hydration preserves SQLx Any text and byte representations.
#![cfg(not(all(target_family = "wasm", target_os = "unknown")))]

use reinhardt_db::orm::fields::{BigIntegerField, CharField, Field, TextField};
use reinhardt_db::orm::inspection::FieldInfo;
use reinhardt_db::orm::query_types::DbBackend;
use reinhardt_db::orm::session::{Session, SessionError};
use reinhardt_db::orm::{FieldSelector, Manager, Model, QuerySet};
use reinhardt_test::fixtures::{mysql_container, postgres_container};
use rstest::{fixture, rstest};
use serde::{Deserialize, Serialize};
use serial_test::serial;
use sqlx::{AnyPool, Row, any::AnyPoolOptions};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

#[derive(Clone)]
struct TextFields;

impl FieldSelector for TextFields {
	fn with_alias(self, _alias: &str) -> Self {
		self
	}
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct TextRecord {
	id: i64,
	name: String,
	note: Option<String>,
}

impl Model for TextRecord {
	type PrimaryKey = i64;
	type Fields = TextFields;
	type Objects = Manager<Self>;

	fn table_name() -> &'static str {
		"session_text_decoding"
	}

	fn new_fields() -> Self::Fields {
		TextFields
	}

	fn primary_key(&self) -> Option<i64> {
		Some(self.id)
	}

	fn set_primary_key(&mut self, id: i64) {
		self.id = id;
	}

	fn field_metadata() -> Vec<FieldInfo> {
		let mut id = BigIntegerField::new();
		id.base.primary_key = true;
		id.set_attributes_from_name("id");
		let mut name = CharField::new(255);
		name.set_attributes_from_name("name");
		let mut note = TextField::new();
		note.base.null = true;
		note.set_attributes_from_name("note");
		let mut fields = vec![
			FieldInfo::from_field(&id),
			FieldInfo::from_field(&name),
			FieldInfo::from_field(&note),
		];
		fields[1].db_column = Some("stored_name".into());
		fields
	}
}

struct TextDatabase {
	pool: Arc<AnyPool>,
	// Keep the container alive until all test-owned connections are dropped.
	_container: ContainerAsync<GenericImage>,
}

#[fixture]
async fn mysql_text_database(
	#[future] mysql_container: (
		ContainerAsync<GenericImage>,
		Arc<sqlx::MySqlPool>,
		u16,
		String,
	),
) -> TextDatabase {
	let (container, _native_pool, _port, url) = mysql_container.await;
	sqlx::any::install_default_drivers();
	let pool = Arc::new(
		AnyPoolOptions::new()
			.max_connections(1)
			.connect(&url)
			.await
			.expect("Connect one-connection MySQL Any pool"),
	);
	sqlx::query(
		"CREATE TABLE session_text_decoding (
			id BIGINT PRIMARY KEY,
			stored_name LONGTEXT NOT NULL,
			note TEXT NULL
		) CHARACTER SET utf8mb4",
	)
	.execute(pool.as_ref())
	.await
	.expect("Create isolated text table");
	TextDatabase {
		pool,
		_container: container,
	}
}

#[rstest]
#[serial(sqlx_drivers)]
#[tokio::test]
async fn mysql_session_reads_complete_text(#[future] mysql_text_database: TextDatabase) {
	let database = mysql_text_database.await;
	exercise_session_reads(database.pool.clone(), DbBackend::Mysql).await;
}

#[fixture]
async fn postgres_text_database(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) -> TextDatabase {
	let (container, _native_pool, _port, url) = postgres_container.await;
	sqlx::any::install_default_drivers();
	let pool = Arc::new(
		AnyPoolOptions::new()
			.max_connections(1)
			.connect(&url)
			.await
			.unwrap(),
	);
	sqlx::query("CREATE TABLE session_text_decoding (id BIGINT PRIMARY KEY, stored_name TEXT NOT NULL, note TEXT NULL)")
		.execute(pool.as_ref()).await.unwrap();
	TextDatabase {
		pool,
		_container: container,
	}
}

#[rstest]
#[serial(sqlx_drivers)]
#[tokio::test]
async fn postgres_session_reads_complete_text(#[future] postgres_text_database: TextDatabase) {
	let database = postgres_text_database.await;
	exercise_session_reads(database.pool.clone(), DbBackend::Postgres).await;
}

async fn exercise_session_reads(pool: Arc<AnyPool>, backend: DbBackend) {
	// Arrange: independent bytes-to-UTF-8 controls also protect against truncation.
	let names = [
		"stored' ? $55 日本語 🦀".to_owned(),
		"長い文章' ? $55 🦀".repeat(12_000),
		String::new(),
	];
	let mut expected = Vec::new();
	for (index, name) in names.into_iter().enumerate() {
		let id = i64::try_from(index).unwrap() + 1;
		let note = match index {
			0 => Some("本文' ? $55 🦀".repeat(3_000)),
			1 => None,
			_ => Some(String::new()),
		};
		sqlx::query(match backend {
			DbBackend::Postgres => {
				"INSERT INTO session_text_decoding (id, stored_name, note) VALUES ($1, $2, $3)"
			}
			_ => "INSERT INTO session_text_decoding (id, stored_name, note) VALUES (?, ?, ?)",
		})
		.bind(id)
		.bind(&name)
		.bind(&note)
		.execute(pool.as_ref())
		.await
		.unwrap();
		let row = sqlx::query(match backend {
			DbBackend::Postgres => {
				"SELECT stored_name, note FROM session_text_decoding WHERE id = $1"
			}
			_ => "SELECT stored_name, note FROM session_text_decoding WHERE id = ?",
		})
		.bind(id)
		.fetch_one(pool.as_ref())
		.await
		.unwrap();
		if backend == DbBackend::Mysql {
			let bytes: Vec<u8> = row.try_get("stored_name").unwrap();
			assert_eq!(String::from_utf8(bytes).unwrap(), name);
			assert!(row.try_get::<String, _>("stored_name").is_err());
			let note_bytes: Option<Vec<u8>> = row.try_get("note").unwrap();
			assert_eq!(
				note_bytes.map(|bytes| String::from_utf8(bytes).unwrap()),
				note
			);
		} else {
			assert_eq!(row.try_get::<String, _>("stored_name").unwrap(), name);
			assert_eq!(row.try_get::<Option<String>, _>("note").unwrap(), note);
		}
		expected.push(TextRecord { id, name, note });
	}
	let mut session = Session::new(pool.clone(), backend).await.unwrap();

	// Act
	let mut listed = session.list(&QuerySet::<TextRecord>::new()).await.unwrap();
	let mut all = session.list_all::<TextRecord>().await.unwrap();
	let mut found = Vec::new();
	for record in &expected {
		found.push(session.get::<TextRecord>(record.id).await.unwrap());
	}

	// Assert
	listed.sort_by_key(|record| record.id);
	all.sort_by_key(|record| record.id);
	assert_eq!(listed, expected);
	assert_eq!(all, expected);
	assert_eq!(
		found,
		expected.iter().cloned().map(Some).collect::<Vec<_>>()
	);
	assert_eq!(session.get::<TextRecord>(1).await.unwrap(), found[0]);
}

#[rstest]
#[serial(sqlx_drivers)]
#[tokio::test]
async fn session_rejects_invalid_utf8_with_field_context(
	#[future] mysql_text_database: TextDatabase,
) {
	// Arrange: BLOB input models the byte representation without server conversion.
	let database = mysql_text_database.await;
	sqlx::query("ALTER TABLE session_text_decoding MODIFY stored_name BLOB NOT NULL")
		.execute(database.pool.as_ref())
		.await
		.unwrap();
	sqlx::query("INSERT INTO session_text_decoding (id, stored_name) VALUES (?, ?)")
		.bind(1_i64)
		.bind(vec![0xff_u8, 0xfe])
		.execute(database.pool.as_ref())
		.await
		.unwrap();
	let mut session = Session::new(database.pool.clone(), DbBackend::Mysql)
		.await
		.unwrap();

	// Act
	let list_error = session
		.list(&QuerySet::<TextRecord>::new())
		.await
		.unwrap_err();
	let get_error = session.get::<TextRecord>(1).await.unwrap_err();

	// Assert
	for error in [list_error, get_error] {
		let SessionError::SerializationError(message) = error else {
			panic!("Expected a text serialization error: {error}");
		};
		assert!(
			message.contains("table `session_text_decoding`"),
			"{message}"
		);
		assert!(message.contains("field `name`"), "{message}");
		assert!(message.contains("column `stored_name`"), "{message}");
		assert!(message.contains("invalid UTF-8"), "{message}");
	}
}
