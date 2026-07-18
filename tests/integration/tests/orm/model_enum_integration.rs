//! Model enum persistence integration tests.

use reinhardt::db::associations::ForeignKeyField;
use reinhardt::db::orm::manager::{get_connection, reinitialize_database};
use reinhardt::db::orm::query_types::DbBackend;
use reinhardt::db::orm::session::Session;
use reinhardt::db::orm::{DatabaseField, FieldCodecContext, FieldCodecError, Model};
use reinhardt::{ModelEnum, model};
use reinhardt_db::field_domain::{ModelEnumRepr, ModelEnumValue};
use reinhardt_db::migrations::{MigrationAutodetector, Operation, ProjectState, SqlDialect};
use reinhardt_test::fixtures::{postgres_container, testcontainers::mysql_container};
use rstest::rstest;
use serde::{Deserialize, Serialize};
use serial_test::serial;
use sqlx::{AnyPool, MySqlPool, PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

#[derive(ModelEnum, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[model_enum(repr = "string")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Status {
	#[model_enum(value = "queued")]
	Queued,
	#[model_enum(value = "running")]
	Running,
	#[model_enum(value = "550e8400-e29b-41d4-a716-446655440000")]
	UuidShaped,
}

#[derive(ModelEnum, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[model_enum(repr = "i32")]
enum Priority {
	#[model_enum(value = 10)]
	Low,
	#[model_enum(value = 20)]
	Normal,
	#[model_enum(value = 30)]
	High,
}

#[model(app_label = "jobs", table_name = "cross_backend_enum_jobs")]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct CrossBackendEnumJob {
	#[field(primary_key = true)]
	id: Option<i64>,
	#[field(db_column = "job_status", max_length = 40)]
	status: Status,
	#[field(db_column = "job_priority")]
	priority: Priority,
	#[field(db_column = "fallback_status", max_length = 40, null = true)]
	fallback: Option<Status>,
}

#[model(app_label = "jobs", table_name = "async_jobs")]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct AsyncJob {
	#[field(primary_key = true)]
	id: Option<i64>,
	#[field(db_column = "job_status", max_length = 40)]
	status: Status,
	#[field(db_column = "fallback_status", max_length = 40, null = true)]
	fallback: Option<Status>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct RejectingStatus(String);

impl DatabaseField for RejectingStatus {
	type Storage = String;

	fn encode_database(&self) -> Result<Self::Storage, FieldCodecError> {
		Err(FieldCodecError::Serialization(
			"rejected query value".to_owned(),
		))
	}

	fn decode_database(
		value: Self::Storage,
		_context: &FieldCodecContext,
	) -> Result<Self, FieldCodecError> {
		Ok(Self(value))
	}
}

#[model(app_label = "jobs", table_name = "codec_jobs")]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct CodecJob {
	#[field(primary_key = true)]
	id: Option<i64>,
	#[field(max_length = 40)]
	status: RejectingStatus,
}

#[model(app_label = "jobs", table_name = "byte_records")]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct ByteRecord {
	#[field(primary_key = true)]
	id: Option<i64>,
	payload: Vec<u8>,
}

#[model(app_label = "jobs", table_name = "custom_key_jobs")]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct CustomKeyJob {
	#[field(primary_key = true, db_column = "job_key")]
	key: Option<i64>,
	#[field(max_length = 64)]
	name: String,
}

#[model(app_label = "jobs", table_name = "custom_key_job_attempts")]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct CustomKeyJobAttempt {
	#[field(primary_key = true)]
	id: Option<i64>,
	#[rel(foreign_key)]
	job: ForeignKeyField<CustomKeyJob>,
}

#[model(app_label = "jobs", table_name = "text_key_records")]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct TextKeyRecord {
	#[field(primary_key = true, max_length = 40)]
	key: Option<String>,
	#[field(max_length = 64)]
	name: String,
}

#[model(app_label = "jobs", table_name = "i32_key_records")]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct I32KeyRecord {
	#[field(primary_key = true)]
	id: Option<i32>,
	#[field(max_length = 64)]
	name: String,
}

fn sqlite_database_url() -> (tempfile::NamedTempFile, String) {
	let database = tempfile::NamedTempFile::new().expect("temporary SQLite file should be created");
	let url = format!("sqlite:{}", database.path().display());
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

fn cross_backend_enum_table_sql(dialect: &SqlDialect, with_constraints: bool) -> String {
	let metadata = reinhardt_db::migrations::global_registry()
		.get_model("jobs", "CrossBackendEnumJob")
		.expect("cross-backend model enum metadata should be registered");
	let mut target = ProjectState::new();
	target.add_model(metadata.to_model_state());
	let mut operation = MigrationAutodetector::new(ProjectState::new(), target)
		.generate_operations()
		.into_iter()
		.find(|operation| matches!(operation, Operation::CreateTable { .. }))
		.expect("model enum migration should create its table");
	if !with_constraints {
		let Operation::CreateTable { constraints, .. } = &mut operation else {
			panic!("model schema should compile to CREATE TABLE");
		};
		constraints.clear();
	}
	operation.to_sql(dialect)
}

fn codec_error_in_chain<'a>(
	error: &'a (dyn std::error::Error + 'static),
) -> Option<&'a FieldCodecError> {
	let mut current = Some(error);
	while let Some(source) = current {
		if let Some(codec) = source.downcast_ref::<FieldCodecError>() {
			return Some(codec);
		}
		current = source.source();
	}
	None
}

async fn assert_cross_backend_model_enum_behavior(url: &str, dialect: SqlDialect) {
	reinitialize_database(url)
		.await
		.expect("ORM connection should initialize");
	let connection = get_connection()
		.await
		.expect("ORM connection should be available");
	connection
		.execute(&cross_backend_enum_table_sql(&dialect, true), vec![])
		.await
		.expect("model enum table should be created from model metadata");
	if matches!(dialect, SqlDialect::Sqlite) {
		let sqlite_master = connection
			.query_one(
				"SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'cross_backend_enum_jobs'",
				vec![],
			)
			.await
			.expect("created SQLite schema should be readable")
			.get::<String>("sql")
			.expect("SQLite schema SQL should be text");
		assert!(
			sqlite_master.contains(
				"CHECK (\"job_status\" IN ('550e8400-e29b-41d4-a716-446655440000', 'queued', 'running'))"
			),
			"{sqlite_master}"
		);
		assert!(
			sqlite_master.contains("CHECK (\"job_priority\" IN (10, 20, 30))"),
			"{sqlite_master}"
		);
	}

	let queued = CrossBackendEnumJob::objects()
		.create(&CrossBackendEnumJob {
			id: None,
			status: Status::Queued,
			priority: Priority::Low,
			fallback: None,
		})
		.await
		.expect("string and i32 enum values should be saved");
	let running = CrossBackendEnumJob::objects()
		.create(&CrossBackendEnumJob {
			id: None,
			status: Status::Running,
			priority: Priority::High,
			fallback: Some(Status::Running),
		})
		.await
		.expect("nullable enum Some value should be saved");

	let queued_hydrated = CrossBackendEnumJob::objects()
		.get(queued.id.expect("queued job should have an id"))
		.get()
		.await
		.expect("string and i32 enum values should hydrate");
	assert_eq!(queued_hydrated.status, Status::Queued);
	assert_eq!(queued_hydrated.priority, Priority::Low);
	assert_eq!(queued_hydrated.fallback, None);
	let running_hydrated = CrossBackendEnumJob::objects()
		.get(running.id.expect("running job should have an id"))
		.get()
		.await
		.expect("nullable enum Some value should hydrate");
	assert_eq!(running_hydrated.status, Status::Running);
	assert_eq!(running_hydrated.priority, Priority::High);
	assert_eq!(running_hydrated.fallback, Some(Status::Running));

	let string_eq = CrossBackendEnumJob::objects()
		.filter(CrossBackendEnumJob::field_status().eq(Status::Queued))
		.all()
		.await
		.expect("string enum equality filter should execute");
	assert_eq!(string_eq.len(), 1);
	assert_eq!(string_eq[0].id, queued.id);
	let i32_eq = CrossBackendEnumJob::objects()
		.filter(CrossBackendEnumJob::field_priority().eq(Priority::High))
		.all()
		.await
		.expect("i32 enum equality filter should execute");
	assert_eq!(i32_eq.len(), 1);
	assert_eq!(i32_eq[0].id, running.id);
	let string_members = CrossBackendEnumJob::objects()
		.filter(CrossBackendEnumJob::field_status().is_in([Status::Queued, Status::Running]))
		.all()
		.await
		.expect("string enum membership filter should execute");
	assert_eq!(string_members.len(), 2);
	let i32_members = CrossBackendEnumJob::objects()
		.filter(CrossBackendEnumJob::field_priority().is_in([Priority::Low, Priority::Normal]))
		.all()
		.await
		.expect("i32 enum membership filter should execute");
	assert_eq!(i32_members.len(), 1);
	assert_eq!(i32_members[0].id, queued.id);

	let updated = CrossBackendEnumJob::objects()
		.filter(CrossBackendEnumJob::field_id().eq(running.id))
		.update_fields([
			CrossBackendEnumJob::field_status().assign(Status::Queued),
			CrossBackendEnumJob::field_priority().assign(Priority::Normal),
			CrossBackendEnumJob::field_fallback().assign(Some(Status::Queued)),
		])
		.await
		.expect("typed enum partial update should execute");
	assert_eq!(updated, 1);
	let updated_job = CrossBackendEnumJob::objects()
		.get(running.id.expect("running job should retain its id"))
		.get()
		.await
		.expect("updated enum values should hydrate");
	assert_eq!(updated_job.status, Status::Queued);
	assert_eq!(updated_job.priority, Priority::Normal);
	assert_eq!(updated_job.fallback, Some(Status::Queued));

	let invalid_string = connection
		.execute(
			"INSERT INTO cross_backend_enum_jobs (job_status, job_priority, fallback_status) VALUES ('unknown', 10, NULL)",
			vec![],
		)
		.await;
	assert!(
		invalid_string.is_err(),
		"generated string enum constraint should reject an invalid direct insert"
	);
	let invalid_i32 = connection
		.execute(
			"INSERT INTO cross_backend_enum_jobs (job_status, job_priority, fallback_status) VALUES ('queued', 999, NULL)",
			vec![],
		)
		.await;
	assert!(
		invalid_i32.is_err(),
		"generated i32 enum constraint should reject an invalid direct insert"
	);

	connection
		.execute("DROP TABLE cross_backend_enum_jobs", vec![])
		.await
		.expect("constrained model enum table should be dropped");
	connection
		.execute(&cross_backend_enum_table_sql(&dialect, false), vec![])
		.await
		.expect("legacy model enum table should be created without checks");
	connection
		.execute(
			"INSERT INTO cross_backend_enum_jobs (job_status, job_priority, fallback_status) VALUES ('unknown', 10, NULL)",
			vec![],
		)
		.await
		.expect("legacy invalid enum value should bypass database checks");

	sqlx::any::install_default_drivers();
	let session_pool = Arc::new(
		AnyPool::connect(url)
			.await
			.expect("legacy model enum session pool should connect"),
	);
	let backend = match dialect {
		SqlDialect::Postgres => DbBackend::Postgres,
		SqlDialect::Mysql => DbBackend::Mysql,
		SqlDialect::Sqlite => DbBackend::Sqlite,
		other => panic!("unsupported model enum test dialect: {other:?}"),
	};
	let mut session = Session::new(session_pool, backend)
		.await
		.expect("legacy model enum session should initialize");
	let hydration_error = session
		.get::<CrossBackendEnumJob>(1)
		.await
		.expect_err("legacy invalid enum value should fail typed hydration");
	let codec_error = codec_error_in_chain(&hydration_error)
		.expect("hydration error should preserve its field codec source");
	assert!(matches!(
		codec_error,
		FieldCodecError::InvalidEnumValue {
			context,
			repr: ModelEnumRepr::String,
			value: ModelEnumValue::String(value),
		} if context.model == "CrossBackendEnumJob"
			&& context.field == "status"
			&& context.column == "job_status"
			&& value == "unknown"
	));
}

#[rstest]
#[tokio::test]
#[serial(model_enum_database)]
async fn sqlite_model_enum_cross_backend_contract() {
	let (_database, url) = sqlite_database_url();
	assert_cross_backend_model_enum_behavior(&url, SqlDialect::Sqlite).await;
}

#[rstest]
#[tokio::test]
#[serial(model_enum_database)]
async fn postgres_model_enum_cross_backend_contract(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;
	assert_cross_backend_model_enum_behavior(&url, SqlDialect::Postgres).await;
}

#[rstest]
#[tokio::test]
#[serial(model_enum_database)]
async fn mysql_model_enum_cross_backend_contract(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<MySqlPool>, u16, String),
) {
	let (_container, _pool, _port, url) = mysql_container.await;
	assert_cross_backend_model_enum_behavior(&url, SqlDialect::Mysql).await;
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
			"CREATE TABLE async_jobs (id INTEGER PRIMARY KEY AUTOINCREMENT, job_status VARCHAR(40) NOT NULL, fallback_status VARCHAR(40))",
			vec![],
		)
		.await
		.expect("async_jobs table should be created");

	let job = AsyncJob {
		id: None,
		status: Status::UuidShaped,
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
	assert_eq!(raw_status, "550e8400-e29b-41d4-a716-446655440000");
	assert_eq!(created.status, Status::UuidShaped);
	assert_eq!(created.fallback, Some(Status::Running));

	let hydrated = AsyncJob::objects()
		.get(created.id.expect("created model should have an id"))
		.get()
		.await
		.expect("enum-backed model should hydrate");
	assert_eq!(hydrated.status, Status::UuidShaped);
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
			"CREATE TABLE async_jobs (id INTEGER PRIMARY KEY AUTOINCREMENT, job_status VARCHAR(40) NOT NULL, fallback_status VARCHAR(40))",
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
async fn typed_model_enum_filters_lists_and_assignments_use_persistent_values() {
	let (_database, url) = sqlite_database_url();
	reinitialize_database(&url)
		.await
		.expect("SQLite ORM connection should initialize");
	let connection = get_connection()
		.await
		.expect("SQLite ORM connection should be available");
	connection
		.execute(
			"CREATE TABLE async_jobs (id INTEGER PRIMARY KEY AUTOINCREMENT, job_status VARCHAR(40) NOT NULL, fallback_status VARCHAR(40))",
			vec![],
		)
		.await
		.expect("async_jobs table should be created");

	let queued = AsyncJob::objects()
		.create(&AsyncJob {
			id: None,
			status: Status::Queued,
			fallback: None,
		})
		.await
		.expect("queued job should be created");
	let running = AsyncJob::objects()
		.create(&AsyncJob {
			id: None,
			status: Status::Running,
			fallback: None,
		})
		.await
		.expect("running job should be created");

	let queued_rows = AsyncJob::objects()
		.filter(AsyncJob::field_status().eq(Status::Queued))
		.all()
		.await
		.expect("typed enum equality filter should execute");
	assert_eq!(queued_rows.len(), 1);
	assert_eq!(queued_rows[0].id, queued.id);

	let matching_rows = AsyncJob::objects()
		.filter(AsyncJob::field_status().is_in([Status::Queued, Status::Running]))
		.all()
		.await
		.expect("typed enum list filter should execute");
	assert_eq!(matching_rows.len(), 2);

	let updated = AsyncJob::objects()
		.filter(AsyncJob::field_id().eq(running.id))
		.update_fields([AsyncJob::field_status().assign(Status::Queued)])
		.await
		.expect("typed enum assignment should execute");
	assert_eq!(updated, 1);
	let raw_status = connection
		.query(
			"SELECT job_status FROM async_jobs WHERE id = ?",
			vec![running.id.expect("running job should have an id").into()],
		)
		.await
		.expect("updated status should be readable")
		.into_iter()
		.next()
		.and_then(|row| row.get::<String>("job_status"))
		.expect("job_status should be a string");
	assert_eq!(raw_status, "queued");
}

#[tokio::test]
#[serial(model_enum_database)]
async fn typed_codec_errors_surface_before_filter_or_update_execution() {
	let (_database, url) = sqlite_database_url();
	let mut connection = reinhardt::db::orm::connection::DatabaseConnection::connect(&url)
		.await
		.expect("SQLite connection should initialize");
	connection
		.execute(
			"CREATE TABLE owners (id INTEGER PRIMARY KEY, name VARCHAR(40) NOT NULL)",
			vec![],
		)
		.await
		.expect("owners table should be created");
	connection
		.execute(
			"CREATE TABLE codec_jobs (id INTEGER PRIMARY KEY, status VARCHAR(40) NOT NULL, owner_id INTEGER)",
			vec![],
		)
		.await
		.expect("codec_jobs table should be created");
	connection
		.execute(
			"INSERT INTO codec_jobs (id, status) VALUES (1, 'queued')",
			vec![],
		)
		.await
		.expect("codec job should be inserted");
	connection
		.execute(
			"CREATE TRIGGER reject_codec_job_update BEFORE UPDATE ON codec_jobs BEGIN SELECT RAISE(FAIL, 'SQL update executed'); END",
			vec![],
		)
		.await
		.expect("update rejection trigger should be created");
	connection
		.execute(
			"CREATE TRIGGER reject_codec_job_delete BEFORE DELETE ON codec_jobs BEGIN SELECT RAISE(FAIL, 'SQL delete executed'); END",
			vec![],
		)
		.await
		.expect("delete rejection trigger should be created");

	let filter_error = CodecJob::objects()
		.filter(CodecJob::field_status().eq(RejectingStatus("queued".to_owned())))
		.all_with_db(&mut connection)
		.await
		.expect_err("filter codec error should surface before SQL execution");
	let filter_source =
		std::error::Error::source(&filter_error).expect("filter codec source should be preserved");
	assert!(filter_source.downcast_ref::<FieldCodecError>().is_some());

	let select_related_error = CodecJob::objects()
		.select_related(&["owner"])
		.filter(CodecJob::field_status().eq(RejectingStatus("queued".to_owned())))
		.all_with_db(&mut connection)
		.await
		.expect_err("select-related codec error should surface before SQL execution");
	let select_related_source = std::error::Error::source(&select_related_error)
		.expect("select-related codec source should be preserved");
	assert!(
		select_related_source
			.downcast_ref::<FieldCodecError>()
			.is_some()
	);

	let delete_error = CodecJob::objects()
		.filter(CodecJob::field_status().eq(RejectingStatus("queued".to_owned())))
		.delete_sql()
		.expect_err("delete codec error should surface before SQL execution");
	let delete_source =
		std::error::Error::source(&delete_error).expect("delete codec source should be preserved");
	assert!(delete_source.downcast_ref::<FieldCodecError>().is_some());

	let update_error = CodecJob::objects()
		.filter(CodecJob::field_id().eq(Some(1)))
		.update_fields_with_conn(
			&mut connection,
			[CodecJob::field_status().assign(RejectingStatus("running".to_owned()))],
		)
		.await
		.expect_err("update codec error should surface before SQL execution");
	let update_source =
		std::error::Error::source(&update_error).expect("update codec source should be preserved");
	assert!(update_source.downcast_ref::<FieldCodecError>().is_some());
	let raw_status = connection
		.query("SELECT status FROM codec_jobs WHERE id = 1", vec![])
		.await
		.expect("codec job status should remain readable")
		.into_iter()
		.next()
		.and_then(|row| row.get::<String>("status"))
		.expect("status should be a string");
	assert_eq!(raw_status, "queued");
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
			"CREATE TABLE async_jobs (id INTEGER PRIMARY KEY AUTOINCREMENT, job_status VARCHAR(40) NOT NULL, fallback_status VARCHAR(40))",
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
		"CREATE TABLE async_jobs (id INTEGER PRIMARY KEY AUTOINCREMENT, job_status VARCHAR(40) NOT NULL, fallback_status VARCHAR(40))",
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
			status: Status::UuidShaped,
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
	assert_eq!(
		raw.get::<String, _>("job_status"),
		"550e8400-e29b-41d4-a716-446655440000"
	);
	assert_eq!(raw.get::<Option<String>, _>("fallback_status"), None);

	let mut reader = Session::new(pool, DbBackend::Sqlite)
		.await
		.expect("reader session should initialize");
	let hydrated = reader
		.get::<AsyncJob>(1)
		.await
		.expect("session hydration should succeed")
		.expect("enum-backed model should exist");
	assert_eq!(hydrated.status, Status::UuidShaped);
	assert_eq!(hydrated.fallback, None);
}

#[tokio::test]
#[serial(model_enum_database)]
async fn session_invalid_enum_error_preserves_codec_source() {
	let (_database, url) = sqlite_database_url();
	let pool = sqlite_session_pool(&url).await;
	sqlx::query(
		"CREATE TABLE async_jobs (id INTEGER PRIMARY KEY AUTOINCREMENT, job_status VARCHAR(40) NOT NULL, fallback_status VARCHAR(40))",
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
async fn manager_binds_database_bytes_without_json_reinterpretation() {
	let (_database, url) = sqlite_database_url();
	reinitialize_database(&url)
		.await
		.expect("SQLite ORM connection should initialize");
	let connection = get_connection()
		.await
		.expect("SQLite ORM connection should be available");
	connection
		.execute(
			"CREATE TABLE byte_records (id INTEGER PRIMARY KEY AUTOINCREMENT, payload BLOB NOT NULL)",
			vec![],
		)
		.await
		.expect("byte_records table should be created");
	let payload = vec![0, 1, 127, 255];

	let created = ByteRecord::objects()
		.create(&ByteRecord {
			id: None,
			payload: payload.clone(),
		})
		.await
		.expect("manager should bind byte payload");
	let row = connection
		.query_one(
			"SELECT typeof(payload) AS storage_type, hex(payload) AS payload_hex FROM byte_records WHERE id = 1",
			vec![],
		)
		.await
		.expect("raw byte payload should be readable");
	assert_eq!(row.get::<String>("storage_type").as_deref(), Some("blob"));
	assert_eq!(
		row.get::<String>("payload_hex").as_deref(),
		Some("00017FFF")
	);
	assert_eq!(created.payload, payload);
}

#[tokio::test]
#[serial(model_enum_database)]
async fn session_binds_database_bytes_without_json_reinterpretation() {
	let (_database, url) = sqlite_database_url();
	let pool = sqlite_session_pool(&url).await;
	sqlx::query(
		"CREATE TABLE byte_records (id INTEGER PRIMARY KEY AUTOINCREMENT, payload BLOB NOT NULL)",
	)
	.execute(pool.as_ref())
	.await
	.expect("byte_records table should be created");
	let payload = vec![0, 1, 127, 255];
	let mut session = Session::new(pool.clone(), DbBackend::Sqlite)
		.await
		.expect("session should initialize");
	session
		.add(ByteRecord {
			id: None,
			payload: payload.clone(),
		})
		.await
		.expect("session should track byte payload");
	session
		.flush()
		.await
		.expect("session should bind byte payload");

	let row = sqlx::query("SELECT payload FROM byte_records WHERE id = 1")
		.fetch_one(pool.as_ref())
		.await
		.expect("raw byte payload should be readable");
	assert_eq!(row.get::<Vec<u8>, _>("payload"), payload);
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

#[tokio::test]
#[serial(model_enum_database)]
async fn foreign_key_loader_uses_target_primary_key_database_column() {
	let (_database, url) = sqlite_database_url();
	reinitialize_database(&url)
		.await
		.expect("SQLite ORM connection should initialize");
	let mut connection = get_connection()
		.await
		.expect("SQLite ORM connection should be available");
	connection
		.execute(
			"CREATE TABLE custom_key_jobs (job_key INTEGER PRIMARY KEY, name VARCHAR(64) NOT NULL)",
			vec![],
		)
		.await
		.expect("custom_key_jobs table should be created");
	connection
		.execute(
			"INSERT INTO custom_key_jobs (job_key, name) VALUES (41, 'related job')",
			vec![],
		)
		.await
		.expect("related custom key row should be inserted");
	let attempt = CustomKeyJobAttempt::build().job(41_i64).finish();

	let related = attempt
		.job(&mut connection)
		.await
		.expect("foreign-key loader should use the target database column")
		.expect("related custom key job should exist");

	assert_eq!(related.key, Some(41));
	assert_eq!(related.name, "related job");
}

#[tokio::test]
#[serial(model_enum_database)]
async fn uuid_shaped_text_primary_key_stays_text_for_manager_writes() {
	let (_database, url) = sqlite_database_url();
	reinitialize_database(&url)
		.await
		.expect("SQLite ORM connection should initialize");
	let connection = get_connection()
		.await
		.expect("SQLite ORM connection should be available");
	connection
		.execute(
			"CREATE TABLE text_key_records (key TEXT PRIMARY KEY, name VARCHAR(64) NOT NULL)",
			vec![],
		)
		.await
		.expect("text_key_records table should be created");
	let manager_key = "550e8400-e29b-41d4-a716-446655440000";
	let bulk_key = "550e8400-e29b-41d4-a716-446655440001";
	for (key, name) in [(manager_key, "manager before"), (bulk_key, "bulk before")] {
		connection
			.execute(
				&format!("INSERT INTO text_key_records (key, name) VALUES ('{key}', '{name}')"),
				vec![],
			)
			.await
			.expect("text primary-key row should be inserted");
	}

	TextKeyRecord::objects()
		.update(&TextKeyRecord {
			key: Some(manager_key.to_string()),
			name: "manager after".to_string(),
		})
		.await
		.expect("manager update should target a UUID-shaped text primary key");
	TextKeyRecord::objects()
		.bulk_update(
			vec![TextKeyRecord {
				key: Some(bulk_key.to_string()),
				name: "bulk after".to_string(),
			}],
			vec!["name".to_string()],
			None,
		)
		.await
		.expect("bulk update should target a UUID-shaped text primary key");

	let rows = connection
		.query(
			"SELECT key, name, typeof(key) AS key_type FROM text_key_records ORDER BY key",
			vec![],
		)
		.await
		.expect("updated rows should be readable");
	assert_eq!(rows.len(), 2);
	assert_eq!(rows[0].get::<String>("key").as_deref(), Some(manager_key));
	assert_eq!(
		rows[0].get::<String>("name").as_deref(),
		Some("manager after")
	);
	assert_eq!(rows[0].get::<String>("key_type").as_deref(), Some("text"));
	assert_eq!(rows[1].get::<String>("key").as_deref(), Some(bulk_key));
	assert_eq!(rows[1].get::<String>("name").as_deref(), Some("bulk after"));
	assert_eq!(rows[1].get::<String>("key_type").as_deref(), Some("text"));
}

#[tokio::test]
#[serial(model_enum_database)]
async fn uuid_shaped_text_primary_key_stays_text_for_session_delete() {
	let (_database, url) = sqlite_database_url();
	let pool = sqlite_session_pool(&url).await;
	sqlx::query("CREATE TABLE text_key_records (key TEXT PRIMARY KEY, name VARCHAR(64) NOT NULL)")
		.execute(pool.as_ref())
		.await
		.expect("text_key_records table should be created");
	let delete_key = "550e8400-e29b-41d4-a716-446655440002";
	sqlx::query("INSERT INTO text_key_records (key, name) VALUES (?, 'delete me')")
		.bind(delete_key)
		.execute(pool.as_ref())
		.await
		.expect("text primary-key row should be inserted");

	let mut session = Session::new(pool.clone(), DbBackend::Sqlite)
		.await
		.expect("session should initialize");
	session
		.delete(TextKeyRecord {
			key: Some(delete_key.to_string()),
			name: "delete me".to_string(),
		})
		.await
		.expect("session should track the text primary key for deletion");
	session
		.flush()
		.await
		.expect("session should delete by the canonical text primary key");
	let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM text_key_records")
		.fetch_one(pool.as_ref())
		.await
		.expect("remaining row count should be readable");
	assert_eq!(remaining, 0);
}

#[tokio::test]
#[serial(model_enum_database)]
async fn manager_omits_zero_i32_primary_key_for_autogeneration() {
	let (_database, url) = sqlite_database_url();
	reinitialize_database(&url)
		.await
		.expect("SQLite ORM connection should initialize");
	let connection = get_connection()
		.await
		.expect("SQLite ORM connection should be available");
	connection
		.execute(
			"CREATE TABLE i32_key_records (id INTEGER PRIMARY KEY AUTOINCREMENT, name VARCHAR(64) NOT NULL)",
			vec![],
		)
		.await
		.expect("i32_key_records table should be created");

	let created = I32KeyRecord::objects()
		.create(&I32KeyRecord {
			id: Some(0),
			name: "generated".to_string(),
		})
		.await
		.expect("zero i32 primary key should use database generation");

	assert_eq!(created.id, Some(1));
	let row = connection
		.query_one("SELECT id, name FROM i32_key_records", vec![])
		.await
		.expect("generated row should be readable");
	assert_eq!(row.get::<i32>("id"), Some(1));
	assert_eq!(row.get::<String>("name").as_deref(), Some("generated"));
}
