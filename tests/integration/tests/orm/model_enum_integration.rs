//! Model enum persistence integration tests.

use reinhardt::db::orm::Model;
use reinhardt::db::orm::manager::{get_connection, reinitialize_database};
use reinhardt::{ModelEnum, model};
use serde::{Deserialize, Serialize};
use serial_test::serial;

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
}

fn sqlite_database_url() -> (tempfile::NamedTempFile, String) {
	let database = tempfile::NamedTempFile::new().expect("temporary SQLite file should be created");
	let url = format!("sqlite://{}", database.path().display());
	(database, url)
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
			"CREATE TABLE async_jobs (id INTEGER PRIMARY KEY AUTOINCREMENT, job_status VARCHAR(16) NOT NULL)",
			vec![],
		)
		.await
		.expect("async_jobs table should be created");

	let job = AsyncJob {
		id: None,
		status: Status::Queued,
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

	let hydrated = AsyncJob::objects()
		.get(created.id.expect("created model should have an id"))
		.get()
		.await
		.expect("enum-backed model should hydrate");
	assert_eq!(hydrated.status, Status::Queued);
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
			"CREATE TABLE async_jobs (id INTEGER PRIMARY KEY AUTOINCREMENT, job_status VARCHAR(16) NOT NULL)",
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
