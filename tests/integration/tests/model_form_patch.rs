//! Generated form patch contracts and conditional persistence.

#![cfg(feature = "sqlite")]

use reinhardt_core::macros::model;
use reinhardt_core::model_form::{ModelFormPatchPayload, PatchValidationError};
use rstest::rstest;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[model(app_label = "patch_test", info = false, form(name = EditRecord, fields(name, active, notes)))]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Record {
	#[field(primary_key = true)]
	id: Option<i64>,
	#[field(editable = false)]
	tenant: i64,
	#[field(min_length = 1, max_length = 64)]
	#[form(trim)]
	name: String,
	#[field(default = true)]
	active: bool,
	#[field(blank = true, max_length = 128)]
	notes: Option<String>,
	#[field(editable = false, max_length = 128)]
	token: String,
	#[field(editable = false)]
	revision: i64,
}

#[model(app_label = "patch_test", info = false, form(name = EditSubset, fields(title)))]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Subset {
	#[field(primary_key = true)]
	id: Option<i64>,
	#[field(max_length = 64)]
	title: String,
	#[field(max_length = 64)]
	untouched: String,
}

#[model(app_label = "patch_test", info = false, form(name = EditNatural, fields(id, label)))]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Natural {
	#[field(primary_key = true, editable = true, max_length = 64)]
	id: String,
	#[field(max_length = 64)]
	label: String,
}

#[rstest]
#[case(json!({"name":" New "}), json!({"name":"New"}))]
#[case(json!({"active":false}), json!({"active":false}))]
#[case(json!({"notes":null}), json!({"notes":null}))]
#[case(json!({"notes":""}), json!({"notes":""}))]
#[case(json!({}), json!({}))]
fn patch_cleaning_retains_only_submitted_values(
	#[case] raw: serde_json::Value,
	#[case] expected: serde_json::Value,
) {
	// Arrange
	let data: EditRecordData = serde_json::from_value(raw).unwrap();
	// Act
	let cleaned = data.clean_and_validate_patch(None).unwrap();
	// Assert
	assert_eq!(serde_json::to_value(cleaned.into_raw()).unwrap(), expected);
}

#[rstest]
fn patch_cleaning_rejects_empty_required_string() {
	let data: EditRecordData = serde_json::from_value(json!({"name":"   "})).unwrap();
	let result = data.clean_and_validate_patch(None);
	assert!(matches!(result, Err(PatchValidationError::Validation(_))));
}

#[model(app_label = "patch_test", info = false, form(name = EditInterval, fields(start, end)))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[form(validate = interval_order)]
struct Interval {
	#[field(primary_key = true)]
	id: Option<i64>,
	start: i64,
	end: i64,
}

fn interval_order<P: reinhardt_core::model_form::ModelFormPolicy>(
	data: &CleanedIntervalModelFormData<P>,
) -> Result<(), reinhardt_core::validators::ValidationErrors> {
	let mut errors = reinhardt_core::validators::ValidationErrors::new();
	if data
		.start()
		.zip(data.end())
		.is_none_or(|(start, end)| start > end)
	{
		errors.add(
			"start",
			reinhardt_core::validators::ValidationError::Custom(
				"start must not exceed end".to_owned(),
			),
		);
	}
	if errors.is_empty() {
		Ok(())
	} else {
		Err(errors)
	}
}

#[rstest]
fn patch_requires_explicit_validator_context() {
	let data: EditIntervalData = serde_json::from_value(json!({"start":2})).unwrap();
	assert!(matches!(
		data.clean_and_validate_patch(None),
		Err(PatchValidationError::ExistingValuesRequired)
	));
}

#[rstest]
fn patch_context_is_not_an_assignment() {
	let data: EditIntervalData = serde_json::from_value(json!({"start":2})).unwrap();
	let context: EditIntervalData = serde_json::from_value(json!({"start":1,"end":5})).unwrap();
	let cleaned = data.clean_and_validate_patch(Some(&context)).unwrap();
	assert_eq!(
		serde_json::to_value(cleaned.into_raw()).unwrap(),
		json!({"start":2})
	);
}

#[rstest]
fn patch_checks_merged_candidate() {
	let data: EditIntervalData = serde_json::from_value(json!({"start":6})).unwrap();
	let context: EditIntervalData = serde_json::from_value(json!({"start":1,"end":5})).unwrap();
	assert!(matches!(
		data.clean_and_validate_patch(Some(&context)),
		Err(PatchValidationError::Validation(_))
	));
}

#[rstest]
fn empty_patch_is_a_native_error() {
	assert!(matches!(
		EditRecord::validate_patch(EditRecordData::default()),
		Err(reinhardt_forms::model_form::PatchError::EmptyPatch)
	));
}

use reinhardt_db::orm::{DatabaseConnection, DatabaseConnectionLease, Model};

struct PatchDatabase {
	connection: DatabaseConnection,
	_lease: DatabaseConnectionLease,
	_directory: tempfile::TempDir,
}

async fn patch_database() -> PatchDatabase {
	let directory = tempfile::Builder::new()
		.prefix("reinhardt-patch-")
		.tempdir_in("/tmp")
		.unwrap();
	let url = format!(
		"sqlite://{}",
		directory.path().join("patch.sqlite").display()
	);
	let owner = reinhardt_db::backends::DatabaseConnection::connect_sqlite(&url)
		.await
		.unwrap();
	let lease = DatabaseConnectionLease::register(owner).unwrap();
	let connection = lease.handle();
	connection.execute("CREATE TABLE patch_test_record (id INTEGER PRIMARY KEY, tenant INTEGER NOT NULL, name TEXT NOT NULL UNIQUE, active BOOLEAN NOT NULL, notes TEXT, token TEXT NOT NULL, revision BIGINT NOT NULL)", vec![]).await.unwrap();
	connection.execute("INSERT INTO patch_test_record VALUES (1, 7, 'Old', true, 'note', 'secret', 1), (2, 8, 'Other', true, NULL, 'other_secret', 1)", vec![]).await.unwrap();
	PatchDatabase {
		connection,
		_lease: lease,
		_directory: directory,
	}
}

#[rstest]
#[tokio::test]
async fn patch_preserves_scope_and_unrelated_fields() {
	let mut db = patch_database().await;
	let payload: EditRecordData =
		serde_json::from_value(json!({"name":" New ","active":false,"notes":null})).unwrap();
	let patch = EditRecord::validate_patch(payload).unwrap();
	let result = patch
		.apply_to(
			Record::objects().filter(Record::field_tenant().eq(7)),
			1_i64,
			&mut db.connection,
		)
		.await
		.unwrap();
	assert_eq!(result.rows_affected, 1);
	let rows = db
		.connection
		.query("SELECT * FROM patch_test_record ORDER BY id", vec![])
		.await
		.unwrap();
	assert_eq!(rows.len(), 2);
	assert_eq!(rows[0].get::<String>("name"), Some("New".to_owned()));
	assert_eq!(rows[0].get::<String>("token"), Some("secret".to_owned()));
	assert_eq!(rows[0].get::<Option<String>>("notes"), Some(None));
	assert_eq!(rows[1].get::<String>("name"), Some("Other".to_owned()));
}

#[rstest]
#[tokio::test]
async fn patch_cannot_escape_tenant_scope() {
	let mut db = patch_database().await;
	let payload: EditRecordData = serde_json::from_value(json!({"name":" New "})).unwrap();
	let patch = EditRecord::validate_patch(payload).unwrap();
	let result = patch
		.apply_to(
			Record::objects().filter(Record::field_tenant().eq(7)),
			2_i64,
			&mut db.connection,
		)
		.await
		.unwrap();
	assert_eq!(result.rows_affected, 0);
	let rows = db
		.connection
		.query("SELECT name FROM patch_test_record ORDER BY id", vec![])
		.await
		.unwrap();
	assert_eq!(
		rows.iter()
			.map(|row| row.get::<String>("name").unwrap())
			.collect::<Vec<_>>(),
		["Old", "Other"]
	);
}

#[rstest]
#[case(r#"{"tenant":7}"#)]
#[case(r#"{"token":"stolen"}"#)]
#[case(r#"{"id":1}"#)]
#[case(r#"{"unknown":1}"#)]
#[case(r#"{"active":"false"}"#)]
#[case(r#"{"name":"a","name":"b"}"#)]
fn patch_wire_boundary_rejects_invalid_inputs(#[case] raw: &str) {
	assert!(serde_json::from_str::<EditRecordData>(raw).is_err());
}

#[rstest]
#[tokio::test]
async fn contextual_patch_checks_target_identity() {
	let mut db = patch_database().await;
	let existing = Record {
		id: Some(1),
		tenant: 7,
		name: "Old".to_owned(),
		active: true,
		notes: None,
		token: "secret".to_owned(),
		revision: 1,
	};
	let payload: EditRecordData = serde_json::from_value(json!({"name":"New"})).unwrap();
	let patch = EditRecord::validate_patch_with_existing(payload, &existing).unwrap();
	let result = patch
		.apply_to(Record::objects().all(), 2_i64, &mut db.connection)
		.await;
	assert!(matches!(
		result,
		Err(reinhardt_forms::model_form::PatchError::TargetMismatch)
	));
}

#[derive(Default)]
struct RecordingExecutor {
	writes: Vec<(String, Vec<reinhardt_db::orm::connection::QueryValue>)>,
}

#[async_trait::async_trait]
impl reinhardt_db::orm::OrmExecutor for RecordingExecutor {
	fn backend(&self) -> reinhardt_db::orm::connection::DatabaseBackend {
		reinhardt_db::orm::connection::DatabaseBackend::Postgres
	}
	async fn execute(
		&mut self,
		sql: &str,
		params: Vec<reinhardt_db::orm::connection::QueryValue>,
	) -> reinhardt_core::exception::Result<reinhardt_db::orm::connection::QueryResult> {
		self.writes.push((sql.to_owned(), params));
		Ok(reinhardt_db::orm::connection::QueryResult {
			rows_affected: 1,
			last_insert_id: None,
		})
	}
	async fn fetch_optional(
		&mut self,
		_: &str,
		_: Vec<reinhardt_db::orm::connection::QueryValue>,
	) -> reinhardt_core::exception::Result<Option<reinhardt_db::orm::connection::Row>> {
		panic!("patch must not read")
	}
	async fn fetch_one(
		&mut self,
		_: &str,
		_: Vec<reinhardt_db::orm::connection::QueryValue>,
	) -> reinhardt_core::exception::Result<reinhardt_db::orm::connection::Row> {
		panic!("patch must not read")
	}
	async fn fetch_all(
		&mut self,
		_: &str,
		_: Vec<reinhardt_db::orm::connection::QueryValue>,
	) -> reinhardt_core::exception::Result<Vec<reinhardt_db::orm::connection::Row>> {
		panic!("patch must not read")
	}
}

#[rstest]
#[tokio::test]
async fn patch_sql_ands_resource_key_with_complete_scope() {
	use reinhardt_db::orm::connection::QueryValue;
	let mut executor = RecordingExecutor::default();
	let payload: EditRecordData = serde_json::from_value(json!({"name":"New"})).unwrap();
	let patch = EditRecord::validate_patch(payload).unwrap();
	let scope = Record::objects().filter(
		Record::field_tenant()
			.eq(7)
			.or(Record::field_tenant().eq(9)),
	);
	patch.apply_to(scope, 1_i64, &mut executor).await.unwrap();
	assert_eq!(executor.writes, vec![(
        "UPDATE \"patch_test_record\" SET \"name\" = $1 WHERE (\"id\" = $2 AND (\"tenant\" = $3 OR \"tenant\" = $4))".to_owned(),
        vec![QueryValue::String("New".to_owned()), QueryValue::Int(1), QueryValue::Int(7), QueryValue::Int(9)],
    )]);
}

#[rstest]
fn create_defaults_cannot_become_patch_assignments() {
	use reinhardt_core::model_form::ModelFormValidatingPayload;
	let data: EditRecordData = serde_json::from_value(json!({"name":"New"})).unwrap();
	let raw = data.clean_and_validate().unwrap().into_raw();
	assert!(matches!(
		EditRecord::validate_patch(raw),
		Err(reinhardt_forms::model_form::PatchError::Validation(
			PatchValidationError::DefaultedValues
		))
	));
}

#[rstest]
#[tokio::test]
async fn patch_accepts_model_reference_and_optional_key() {
	let mut db = patch_database().await;
	let existing = Record {
		id: Some(1),
		tenant: 7,
		name: "Old".to_owned(),
		active: true,
		notes: None,
		token: "secret".to_owned(),
		revision: 1,
	};
	for use_reference in [true, false] {
		let payload = serde_json::from_value(json!({"active":false})).unwrap();
		let patch = EditRecord::validate_patch_with_existing(payload, &existing).unwrap();
		let scope = Record::objects().filter(Record::field_tenant().eq(7));
		let outcome = if use_reference {
			patch
				.apply_to(scope, &existing, &mut db.connection)
				.await
				.unwrap()
		} else {
			patch
				.apply_to(scope, Some(1_i64), &mut db.connection)
				.await
				.unwrap()
		};
		// SQLite reports matched rows, including the same-value second update.
		assert_eq!(outcome.rows_affected, 1);
	}
}

#[rstest]
#[tokio::test]
async fn patch_preserves_typed_constraint_error_and_rolls_back() {
	use reinhardt_forms::model_form::PatchError;
	let db = patch_database().await;
	let result: Result<(), PatchError> = db
		.connection
		.atomic(async |transaction| {
			let patch = EditRecord::validate_patch(
				serde_json::from_value(json!({"active":false})).unwrap(),
			)?;
			patch
				.apply_to(Record::objects().all(), 1_i64, transaction)
				.await?;
			let patch = EditRecord::validate_patch(
				serde_json::from_value(json!({"name":"Other"})).unwrap(),
			)?;
			patch
				.apply_to(Record::objects().all(), 1_i64, transaction)
				.await?;
			Ok(())
		})
		.await;
	let Err(PatchError::Database(error)) = result else {
		panic!("expected database error")
	};
	assert_eq!(
		error.database_kind(),
		Some(reinhardt_core::exception::DatabaseErrorKind::UniqueViolation)
	);
	let rows = db
		.connection
		.query(
			"SELECT name, active FROM patch_test_record WHERE id = 1",
			vec![],
		)
		.await
		.unwrap();
	assert_eq!(rows[0].get::<String>("name"), Some("Old".to_owned()));
	assert_eq!(rows[0].get::<bool>("active"), Some(true));
}

#[rstest]
#[tokio::test]
async fn patch_validation_error_rolls_back_transaction() {
	use reinhardt_forms::model_form::PatchError;
	let db = patch_database().await;
	let result: Result<(), PatchError> = db
		.connection
		.atomic(async |transaction| {
			EditRecord::validate_patch(serde_json::from_value(json!({"active":false})).unwrap())?
				.apply_to(Record::objects().all(), 1_i64, transaction)
				.await?;
			EditRecord::validate_patch(serde_json::from_value(json!({"name":" "})).unwrap())?;
			Ok(())
		})
		.await;
	assert!(matches!(
		result,
		Err(PatchError::Validation(PatchValidationError::Validation(_)))
	));
	let rows = db
		.connection
		.query("SELECT active FROM patch_test_record WHERE id = 1", vec![])
		.await
		.unwrap();
	assert_eq!(rows[0].get::<bool>("active"), Some(true));
}

#[rstest]
#[case::partial_writers(false, "New")]
#[case::legacy_rotation_control(true, "Old")]
#[tokio::test]
async fn two_connections_preserve_disjoint_patches_after_stale_reads(
	#[case] legacy_rotation: bool,
	#[case] expected_name: &str,
) {
	// Both independent connection pools read the old state before either writer
	// proceeds. A channel orders commits deterministically without sleeps.
	let mut db = patch_database().await;
	let url = format!(
		"sqlite://{}",
		db._directory.path().join("patch.sqlite").display()
	);
	let owner = reinhardt_db::backends::DatabaseConnection::connect_sqlite(&url)
		.await
		.unwrap();
	let lease = DatabaseConnectionLease::register(owner).unwrap();
	let mut second = lease.handle();
	let first_snapshot = Record::objects()
		.filter(Record::field_id().eq(1_i64))
		.first_with_db(&mut db.connection)
		.await
		.unwrap()
		.unwrap();
	let mut second_snapshot = Record::objects()
		.filter(Record::field_id().eq(1_i64))
		.first_with_db(&mut second)
		.await
		.unwrap()
		.unwrap();
	assert_eq!(first_snapshot.name, second_snapshot.name);
	assert_eq!(second_snapshot.token, "secret");
	let (sent, received) = tokio::sync::oneshot::channel();
	let edit = async {
		let patch = EditRecord::validate_patch_with_existing(
			serde_json::from_value(json!({"name":"New"})).unwrap(),
			&first_snapshot,
		)
		.unwrap();
		assert_eq!(
			patch
				.apply_to(
					Record::objects().filter(Record::field_tenant().eq(7)),
					&first_snapshot,
					&mut db.connection
				)
				.await
				.unwrap()
				.rows_affected,
			1
		);
		sent.send(()).unwrap();
	};
	let rotate = async {
		received.await.unwrap();
		if legacy_rotation {
			second_snapshot.token = "rotated".to_owned();
			Record::objects()
				.update_with_conn(&mut second, &second_snapshot)
				.await
				.unwrap();
		} else {
			assert_eq!(
				Record::objects()
					.filter(Record::field_tenant().eq(7))
					.filter(Record::field_id().eq(1_i64))
					.update_fields_with_conn(
						&mut second,
						[Record::field_token().assign("rotated".to_owned())]
					)
					.await
					.unwrap(),
				1
			);
		}
	};
	tokio::time::timeout(std::time::Duration::from_secs(15), async {
		tokio::join!(edit, rotate);
	})
	.await
	.expect("bounded writer schedule");
	let rows = second
		.query(
			"SELECT name, token FROM patch_test_record WHERE id = 1",
			vec![],
		)
		.await
		.unwrap();
	assert_eq!(
		rows[0].get::<String>("name").as_deref(),
		Some(expected_name)
	);
	assert_eq!(rows[0].get::<String>("token"), Some("rotated".to_owned()));
}

#[model(app_label = "patch_test", info = false, form(name = EditRenamed, fields(value)))]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Renamed {
	#[field(primary_key = true, db_column = "resource_key")]
	id: i64,
	#[field(db_column = "stored_value")]
	value: i64,
}

#[rstest]
#[tokio::test]
async fn patch_uses_typed_columns_and_preserves_zero() {
	use reinhardt_db::orm::connection::QueryValue;
	let mut executor = RecordingExecutor::default();
	let patch =
		EditRenamed::validate_patch(serde_json::from_value(json!({"value":0})).unwrap()).unwrap();
	patch
		.apply_to(Renamed::objects().all(), 4_i64, &mut executor)
		.await
		.unwrap();
	assert_eq!(
		executor.writes,
		vec![(
			"UPDATE \"patch_test_renamed\" SET \"stored_value\" = $1 WHERE \"resource_key\" = $2"
				.to_owned(),
			vec![QueryValue::Int(0), QueryValue::Int(4)],
		)]
	);
}

#[rstest]
#[tokio::test]
async fn caller_predicate_controls_same_field_conflicts() {
	let mut db = patch_database().await;
	for (name, expected) in [("Winner", 1), ("Stale", 0)] {
		let patch =
			EditRecord::validate_patch(serde_json::from_value(json!({"name":name})).unwrap())
				.unwrap();
		let scope = Record::objects()
			.filter(Record::field_tenant().eq(7))
			.filter(Record::field_name().eq("Old"));
		assert_eq!(
			patch
				.apply_to(scope, 1_i64, &mut db.connection)
				.await
				.unwrap()
				.rows_affected,
			expected
		);
	}
	let rows = db
		.connection
		.query("SELECT name FROM patch_test_record WHERE id = 1", vec![])
		.await
		.unwrap();
	assert_eq!(rows[0].get::<String>("name").as_deref(), Some("Winner"));
}

fn forbidden_create_default() -> String {
	panic!("patch validation must never evaluate a create default")
}

#[model(app_label = "patch_test", info = false, form(name = EditDefaults, fields(title, note)))]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Defaults {
	#[field(primary_key = true)]
	id: Option<i64>,
	#[field(max_length = 64)]
	title: String,
	#[field(max_length = 64, blank = true, default = forbidden_create_default())]
	#[form(trim)]
	note: String,
}

#[rstest]
#[case(json!({"title":"New"}), json!({"title":"New"}))]
#[case(json!({"note":"  "}), json!({"note":""}))]
fn patch_never_evaluates_create_defaults(
	#[case] raw: serde_json::Value,
	#[case] expected: serde_json::Value,
) {
	let data: EditDefaultsData = serde_json::from_value(raw).unwrap();
	let cleaned = data.clean_and_validate_patch(None).unwrap();
	assert_eq!(serde_json::to_value(cleaned.into_raw()).unwrap(), expected);
}

#[rstest]
fn context_without_a_key_is_rejected() {
	let existing = Record {
		id: None,
		tenant: 7,
		name: "Old".to_owned(),
		active: true,
		notes: None,
		token: "secret".to_owned(),
		revision: 1,
	};
	let result = EditRecord::validate_patch_with_existing(
		serde_json::from_value(json!({"name":"New"})).unwrap(),
		&existing,
	);
	assert!(matches!(
		result,
		Err(reinhardt_forms::PatchError::MissingSnapshotKey)
	));
}

#[rstest]
#[tokio::test]
async fn invalid_or_empty_input_cannot_reach_executor() {
	let mut executor = RecordingExecutor::default();
	for raw in [json!({}), json!({"name":" "})] {
		let result = async {
			EditRecord::validate_patch(serde_json::from_value(raw).unwrap())?
				.apply_to(Record::objects().all().none(), 1_i64, &mut executor)
				.await
		}
		.await;
		assert!(result.is_err());
	}
	assert_eq!(executor.writes, []);
}

#[cfg(all(feature = "postgres", feature = "mysql"))]
#[rstest]
#[case::postgres(true)]
#[case::mysql(false)]
#[tokio::test]
async fn live_backend_counts_and_version_predicates(#[case] postgres: bool) {
	use std::time::Duration;
	use testcontainers::{
		GenericImage, ImageExt,
		core::{IntoContainerPort, WaitFor},
		runners::AsyncRunner,
	};
	let (image, tag, port, ready) = if postgres {
		(
			"postgres",
			"16-alpine",
			5432,
			"database system is ready to accept connections",
		)
	} else {
		("mysql", "8.0", 3306, "port: 3306  MySQL Community Server")
	};
	let container = GenericImage::new(image, tag)
		.with_exposed_port(port.tcp())
		.with_wait_for(WaitFor::message_on_stderr(ready))
		.with_env_var("POSTGRES_HOST_AUTH_METHOD", "trust")
		.with_env_var("MYSQL_ROOT_PASSWORD", "test")
		.with_env_var("MYSQL_DATABASE", "patch_test")
		.with_startup_timeout(Duration::from_secs(120))
		.start()
		.await
		.unwrap();
	let host = container.get_host().await.unwrap();
	let port = container.get_host_port_ipv4(port).await.unwrap();
	let url = if postgres {
		format!("postgres://postgres@{host}:{port}/postgres?sslmode=disable")
	} else {
		format!("mysql://root:test@{host}:{port}/patch_test")
	};
	let owner = if postgres {
		reinhardt_db::backends::DatabaseConnection::connect_postgres(&url).await
	} else {
		reinhardt_db::backends::DatabaseConnection::connect_mysql(&url).await
	}
	.unwrap();
	let lease = DatabaseConnectionLease::register(owner).unwrap();
	let mut connection = lease.handle();
	connection.execute("CREATE TABLE patch_test_record (id BIGINT PRIMARY KEY, tenant BIGINT NOT NULL, name VARCHAR(64) NOT NULL UNIQUE, active BOOLEAN NOT NULL, notes VARCHAR(128), token VARCHAR(128) NOT NULL, revision BIGINT NOT NULL)", vec![]).await.unwrap();
	connection
		.execute(
			"INSERT INTO patch_test_record VALUES (1, 7, 'Old', true, NULL, 'secret', 1)",
			vec![],
		)
		.await
		.unwrap();
	// Changed, identical, nonexistent, and out-of-scope writes retain the
	// configured driver's exact count semantics on each live backend.
	for (id, tenant, expected) in [(1, 7, 1), (1, 7, 1), (99, 7, 0), (1, 8, 0)] {
		let patch =
			EditRecord::validate_patch(serde_json::from_value(json!({"name":"New"})).unwrap())
				.unwrap();
		assert_eq!(
			patch
				.apply_to(
					Record::objects().filter(Record::field_tenant().eq(tenant)),
					id,
					&mut connection
				)
				.await
				.unwrap()
				.rows_affected,
			expected
		);
	}
	// A version check and advancement belong to the caller's transaction.
	let result: Result<(), reinhardt_forms::PatchError> = connection
		.atomic(async |transaction| {
			let scope = Record::objects()
				.filter(Record::field_tenant().eq(7))
				.filter(Record::field_revision().eq(1));
			let patch = EditRecord::validate_patch(
				serde_json::from_value(json!({"name":"Winner"})).unwrap(),
			)?;
			assert_eq!(
				patch
					.apply_to(scope, 1_i64, transaction)
					.await?
					.rows_affected,
				1
			);
			Record::objects()
				.filter(Record::field_id().eq(1_i64))
				.filter(Record::field_tenant().eq(7))
				.update_fields_with_conn(transaction, [Record::field_revision().assign(2)])
				.await?;
			Ok(())
		})
		.await;
	result.unwrap();
	let patch =
		EditRecord::validate_patch(serde_json::from_value(json!({"name":"Stale"})).unwrap())
			.unwrap();
	assert_eq!(
		patch
			.apply_to(
				Record::objects()
					.filter(Record::field_revision().eq(1))
					.filter(Record::field_tenant().eq(7)),
				1_i64,
				&mut connection
			)
			.await
			.unwrap()
			.rows_affected,
		0
	);
	let final_record = Record::objects()
		.filter(Record::field_id().eq(1_i64))
		.first_with_db(&mut connection)
		.await
		.unwrap()
		.unwrap();
	assert_eq!(final_record.name, "Winner");
	assert_eq!(final_record.revision, 2);
	assert_eq!(final_record.token, "secret");
}

#[model(app_label = "patch_test", info = false, form(name = EditCluster, fields(name, api_url, is_active)))]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Cluster {
	#[field(primary_key = true)]
	id: Option<i64>,
	#[field(editable = false)]
	organization_id: i64,
	#[field(min_length = 1, max_length = 64)]
	#[form(trim)]
	name: String,
	#[field(url = true, max_length = 2048)]
	#[form(trim)]
	api_url: String,
	is_active: bool,
}

async fn cloud_edit(
	data: EditClusterData,
	can_edit: bool,
	organization: i64,
	id: i64,
	connection: &mut DatabaseConnection,
) -> Result<Option<Cluster>, reinhardt_forms::PatchError> {
	if !can_edit {
		return Ok(None);
	}
	let scope = Cluster::objects().filter(Cluster::field_organization_id().eq(organization));
	let outcome = EditCluster::validate_patch(data)?
		.apply_to(scope, id, connection)
		.await?;
	if outcome.rows_affected == 0 {
		return Ok(None);
	}
	Ok(Cluster::objects()
		.filter(Cluster::field_organization_id().eq(organization))
		.filter(Cluster::field_id().eq(id))
		.first_with_db(connection)
		.await?)
}

#[rstest]
#[tokio::test]
async fn cloud_consumer_keeps_rbac_scope_validation_and_response_refresh() {
	let mut db = patch_database().await;
	db.connection.execute("CREATE TABLE patch_test_cluster (id INTEGER PRIMARY KEY, organization_id BIGINT NOT NULL, name TEXT NOT NULL, api_url TEXT NOT NULL, is_active BOOLEAN NOT NULL)", vec![]).await.unwrap();
	db.connection
		.execute(
			"INSERT INTO patch_test_cluster VALUES (1, 7, 'Old', 'https://old.example', true)",
			vec![],
		)
		.await
		.unwrap();
	let payload = || {
		serde_json::from_value(
			json!({"name":" New ", "api_url":" https://api.example ", "is_active":false}),
		)
		.unwrap()
	};
	assert!(
		cloud_edit(payload(), false, 7, 1, &mut db.connection)
			.await
			.unwrap()
			.is_none()
	);
	assert!(
		cloud_edit(payload(), true, 8, 1, &mut db.connection)
			.await
			.unwrap()
			.is_none()
	);
	let before = Cluster::objects()
		.all()
		.first_with_db(&mut db.connection)
		.await
		.unwrap()
		.unwrap();
	assert_eq!(before.name, "Old");
	let invalid = serde_json::from_value(json!({"api_url":"not a url"})).unwrap();
	assert!(matches!(
		cloud_edit(invalid, true, 7, 1, &mut db.connection).await,
		Err(reinhardt_forms::PatchError::Validation(
			PatchValidationError::Validation(_)
		))
	));
	let response = cloud_edit(payload(), true, 7, 1, &mut db.connection)
		.await
		.unwrap()
		.unwrap();
	assert_eq!(response.name, "New");
	assert_eq!(response.api_url, "https://api.example");
	assert_eq!(response.is_active, false);
	assert_eq!(response.organization_id, 7);
}

#[rstest]
#[tokio::test]
async fn grouped_scope_cannot_expand_target_on_database() {
	let mut db = patch_database().await;
	let scope = Record::objects().filter(
		Record::field_tenant()
			.eq(7)
			.or(Record::field_tenant().eq(8)),
	);
	let patch =
		EditRecord::validate_patch(serde_json::from_value(json!({"active":false})).unwrap())
			.unwrap();
	assert_eq!(
		patch
			.apply_to(scope, 1_i64, &mut db.connection)
			.await
			.unwrap()
			.rows_affected,
		1
	);
	let rows = db
		.connection
		.query("SELECT active FROM patch_test_record ORDER BY id", vec![])
		.await
		.unwrap();
	assert_eq!(
		rows.iter()
			.map(|row| row.get::<bool>("active").unwrap())
			.collect::<Vec<_>>(),
		[false, true]
	);
}

#[rstest]
#[tokio::test]
async fn named_subset_contract_does_not_reference_unselected_model_fields() {
	use reinhardt_db::orm::connection::QueryValue;
	let mut executor = RecordingExecutor::default();
	let patch =
		EditSubset::validate_patch(serde_json::from_value(json!({"title": "New"})).unwrap())
			.unwrap();
	patch
		.apply_to(Subset::objects().all(), 4_i64, &mut executor)
		.await
		.unwrap();
	assert_eq!(
		executor.writes,
		vec![(
			"UPDATE \"patch_test_subset\" SET \"title\" = $1 WHERE \"id\" = $2".to_owned(),
			vec![QueryValue::String("New".to_owned()), QueryValue::Int(4)],
		)]
	);
}

#[rstest]
fn assigned_primary_key_is_rejected_by_patch_validation() {
	let payload: EditNaturalData =
		serde_json::from_value(json!({"id": "external", "label": "New"})).unwrap();
	assert!(matches!(
		EditNatural::validate_patch(payload),
		Err(reinhardt_forms::PatchError::Validation(
			PatchValidationError::Validation(_)
		))
	));
}

#[rstest]
#[tokio::test]
#[should_panic(expected = "Primary key value must be provided")]
async fn optional_target_preserves_existing_key_precondition() {
	let mut executor = RecordingExecutor::default();
	let patch =
		EditRecord::validate_patch(serde_json::from_value(json!({"active":false})).unwrap())
			.unwrap();
	let _ = patch
		.apply_to(Record::objects().all(), None::<i64>, &mut executor)
		.await;
}

#[rstest]
#[tokio::test]
#[should_panic(expected = "Model instance must have a primary key set")]
async fn model_target_preserves_existing_key_precondition() {
	let mut executor = RecordingExecutor::default();
	let existing = Record {
		id: None,
		tenant: 7,
		name: "Old".to_owned(),
		active: true,
		notes: None,
		token: "secret".to_owned(),
		revision: 1,
	};
	let patch =
		EditRecord::validate_patch(serde_json::from_value(json!({"active":false})).unwrap())
			.unwrap();
	let _ = patch
		.apply_to(Record::objects().all(), &existing, &mut executor)
		.await;
}
