//! Model enum persistence integration tests.

use reinhardt::db::orm::manager::{get_connection, reinitialize_database};
use reinhardt::db::orm::query_types::DbBackend;
use reinhardt::db::orm::session::Session;
use reinhardt::db::orm::{FieldCodecError, Model};
use reinhardt::{ModelEnum, model};
use serde::{Deserialize, Serialize};
use serial_test::serial;
use sqlx::{AnyPool, Row};
use std::sync::Arc;

#[derive(ModelEnum, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[model_enum(repr = "string")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Status {
	#[model_enum(value = "queued")]
	Queued,
	#[model_enum(value = "running")]
	Running,
}

#[model(app_label = "jobs", table_name = "async_jobs")]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct AsyncJob {
	#[field(primary_key = true)]
	id: Option<i64>,
	#[field(db_column = "job_status", max_length = 16)]
	status: Status,
	#[field(db_column = "fallback_status", max_length = 16, null = true)]
	fallback: Option<Status>,
}

#[model(app_label = "jobs", table_name = "custom_key_jobs")]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct CustomKeyJob {
	#[field(primary_key = true, db_column = "job_key")]
	key: Option<i64>,
	#[field(max_length = 64)]
	name: String,
}

fn sqlite_database_url() -> (tempfile::NamedTempFile, String) {
	let database = tempfile::NamedTempFile::new().expect("temporary SQLite file should be created");
	let url = format!("sqlite://{}", database.path().display());
	(database, url)
}

async fn sqlite_session_pool(url: &str) -> Arc<AnyPool> {
	sqlx::any::install_default_drivers();
	Arc::new(
		AnyPool::connect(url)
			.await
			.expect("SQLite session pool should connect"),
	)
}

#[tokio::test]
#[serial(model_enum_database)]
async fn model_enum_uses_database_value_independently_of_serde() {
	let (_database, url) = sqlite_database_url();
	reinitialize_database(&url)
		.await
		.expect("SQLite ORM connection should initialize");
	let connection = get_connection()
		.await
		.expect("SQLite ORM connection should be available");
	connection
		.execute(
			"CREATE TABLE async_jobs (id INTEGER PRIMARY KEY AUTOINCREMENT, job_status VARCHAR(16) NOT NULL, fallback_status VARCHAR(16))",
			vec![],
		)
		.await
		.expect("async_jobs table should be created");

	let job = AsyncJob {
		id: None,
		status: Status::Queued,
		fallback: Some(Status::Running),
	};
	let created = AsyncJob::objects()
		.create(&job)
		.await
		.expect("enum-backed model should be saved");

	let raw_status = connection
		.query("SELECT job_status FROM async_jobs", vec![])
		.await
		.expect("raw status should be readable")
		.into_iter()
		.next()
		.and_then(|row| row.get::<String>("job_status"))
		.expect("job_status should be a string");
	assert_eq!(raw_status, "queued");
	assert_eq!(created.status, Status::Queued);
	assert_eq!(created.fallback, Some(Status::Running));

	let hydrated = AsyncJob::objects()
		.get(created.id.expect("created model should have an id"))
		.get()
		.await
		.expect("enum-backed model should hydrate");
	assert_eq!(hydrated.status, Status::Queued);
	assert_eq!(hydrated.fallback, Some(Status::Running));
}

#[tokio::test]
#[serial(model_enum_database)]
async fn nullable_model_enum_round_trips_sql_null() {
	let (_database, url) = sqlite_database_url();
	reinitialize_database(&url)
		.await
		.expect("SQLite ORM connection should initialize");
	let connection = get_connection()
		.await
		.expect("SQLite ORM connection should be available");
	connection
		.execute(
			"CREATE TABLE async_jobs (id INTEGER PRIMARY KEY AUTOINCREMENT, job_status VARCHAR(16) NOT NULL, fallback_status VARCHAR(16))",
			vec![],
		)
		.await
		.expect("async_jobs table should be created");

	let created = AsyncJob::objects()
		.create(&AsyncJob {
			id: None,
			status: Status::Queued,
			fallback: None,
		})
		.await
		.expect("nullable enum-backed model should be saved");
	assert_eq!(created.fallback, None);

	let hydrated = AsyncJob::objects()
		.get(created.id.expect("created model should have an id"))
		.get()
		.await
		.expect("nullable enum-backed model should hydrate");
	assert_eq!(hydrated.fallback, None);
}

#[tokio::test]
#[serial(model_enum_database)]
async fn invalid_legacy_database_value_reports_model_field_column_and_value() {
	let (_database, url) = sqlite_database_url();
	reinitialize_database(&url)
		.await
		.expect("SQLite ORM connection should initialize");
	let connection = get_connection()
		.await
		.expect("SQLite ORM connection should be available");
	connection
		.execute(
			"CREATE TABLE async_jobs (id INTEGER PRIMARY KEY AUTOINCREMENT, job_status VARCHAR(16) NOT NULL, fallback_status VARCHAR(16))",
			vec![],
		)
		.await
		.expect("legacy async_jobs table should be created");
	connection
		.execute(
			"INSERT INTO async_jobs (job_status) VALUES ('unknown')",
			vec![],
		)
		.await
		.expect("legacy enum value should be inserted");

	let error = AsyncJob::objects()
		.all()
		.all()
		.await
		.expect_err("invalid enum value should fail hydration")
		.to_string();
	assert!(
		error.contains("AsyncJob.status"),
		"unexpected error: {error}"
	);
	assert!(error.contains("job_status"), "unexpected error: {error}");
	assert!(error.contains("unknown"), "unexpected error: {error}");
}

#[tokio::test]
#[serial(model_enum_database)]
async fn session_model_enum_round_trip_uses_database_codecs() {
	let (_database, url) = sqlite_database_url();
	let pool = sqlite_session_pool(&url).await;
	sqlx::query(
		"CREATE TABLE async_jobs (id INTEGER PRIMARY KEY AUTOINCREMENT, job_status VARCHAR(16) NOT NULL, fallback_status VARCHAR(16))",
	)
	.execute(pool.as_ref())
	.await
	.expect("async_jobs table should be created");

	let mut writer = Session::new(pool.clone(), DbBackend::Sqlite)
		.await
		.expect("writer session should initialize");
	writer
		.add(AsyncJob {
			id: None,
			status: Status::Queued,
			fallback: None,
		})
		.await
		.expect("session should track enum-backed model");
	writer
		.flush()
		.await
		.expect("session should flush enum-backed model");

	let raw = sqlx::query("SELECT job_status, fallback_status FROM async_jobs WHERE id = 1")
		.fetch_one(pool.as_ref())
		.await
		.expect("raw enum columns should be readable");
	assert_eq!(raw.get::<String, _>("job_status"), "queued");
	assert_eq!(raw.get::<Option<String>, _>("fallback_status"), None);

	let mut reader = Session::new(pool, DbBackend::Sqlite)
		.await
		.expect("reader session should initialize");
	let hydrated = reader
		.get::<AsyncJob>(1)
		.await
		.expect("session hydration should succeed")
		.expect("enum-backed model should exist");
	assert_eq!(hydrated.status, Status::Queued);
	assert_eq!(hydrated.fallback, None);
}

#[tokio::test]
#[serial(model_enum_database)]
async fn session_invalid_enum_error_preserves_codec_source() {
	let (_database, url) = sqlite_database_url();
	let pool = sqlite_session_pool(&url).await;
	sqlx::query(
		"CREATE TABLE async_jobs (id INTEGER PRIMARY KEY AUTOINCREMENT, job_status VARCHAR(16) NOT NULL, fallback_status VARCHAR(16))",
	)
	.execute(pool.as_ref())
	.await
	.expect("async_jobs table should be created");
	sqlx::query("INSERT INTO async_jobs (job_status) VALUES ('unknown')")
		.execute(pool.as_ref())
		.await
		.expect("legacy enum value should be inserted");

	let mut session = Session::new(pool, DbBackend::Sqlite)
		.await
		.expect("session should initialize");
	let error = session
		.get::<AsyncJob>(1)
		.await
		.expect_err("invalid enum value should fail session hydration");
	let source = std::error::Error::source(&error).expect("codec source should be preserved");
	assert!(source.downcast_ref::<FieldCodecError>().is_some());
	let message = error.to_string();
	assert!(
		message.contains("AsyncJob.status"),
		"unexpected error: {message}"
	);
	assert!(
		message.contains("job_status"),
		"unexpected error: {message}"
	);
	assert!(message.contains("unknown"), "unexpected error: {message}");
}

#[tokio::test]
#[serial(model_enum_database)]
async fn session_uses_custom_primary_key_field_and_database_column() {
	let (_database, url) = sqlite_database_url();
	let pool = sqlite_session_pool(&url).await;
	sqlx::query(
		"CREATE TABLE custom_key_jobs (job_key INTEGER PRIMARY KEY, name VARCHAR(64) NOT NULL)",
	)
	.execute(pool.as_ref())
	.await
	.expect("custom_key_jobs table should be created");
	sqlx::query("INSERT INTO custom_key_jobs (job_key, name) VALUES (41, 'before')")
		.execute(pool.as_ref())
		.await
		.expect("existing custom key row should be inserted");

	let expected = CustomKeyJob {
		key: Some(41),
		name: "custom key".to_string(),
	};
	let mut writer = Session::new(pool.clone(), DbBackend::Sqlite)
		.await
		.expect("writer session should initialize");
	writer
		.add(expected.clone())
		.await
		.expect("session should track custom primary key model");
	writer
		.flush()
		.await
		.expect("session should flush custom primary key model");

	let mut reader = Session::new(pool, DbBackend::Sqlite)
		.await
		.expect("reader session should initialize");
	let hydrated = reader
		.get::<CustomKeyJob>(41)
		.await
		.expect("custom primary key lookup should succeed")
		.expect("custom primary key model should exist");
	assert_eq!(hydrated, expected);
}
