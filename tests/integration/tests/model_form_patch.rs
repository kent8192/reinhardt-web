//! Generated form patch contracts and conditional persistence.

#![cfg(feature = "sqlite")]

use reinhardt_core::macros::model;
use reinhardt_core::model_form::{ModelFormPatchPayload, PatchValidationError};
use reinhardt_query::prelude::{
	ColumnDef, Expr, ExprTrait, IntoIden, Order, Query, QueryBuilderTrait, QueryStatementBuilder,
	SqliteQueryBuilder, Value,
};
#[cfg(all(feature = "postgres", feature = "mysql"))]
use reinhardt_query::prelude::{MySqlQueryBuilder, PostgresQueryBuilder};
use reinhardt_test::fixtures::{TestDatabase, TestDatabaseBuilder, test_database};
#[cfg(all(feature = "postgres", feature = "mysql"))]
use reinhardt_test::fixtures::{mysql_container, postgres_container};
use rstest::{fixture, rstest};
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

#[derive(Debug, reinhardt_query::Iden)]
enum PatchTestRecord {
	Table,
	Id,
	Tenant,
	Name,
	Active,
	Notes,
	Token,
	Revision,
}

struct EmptyMigrations;

impl reinhardt_db::migrations::MigrationProvider for EmptyMigrations {
	fn migrations() -> Vec<reinhardt_db::migrations::Migration> {
		Vec::new()
	}
}

struct PatchDatabase {
	connection: DatabaseConnection,
	_lease: DatabaseConnectionLease,
	database: TestDatabase,
}

async fn seed_patch_database<B: QueryBuilderTrait + Clone>(
	connection: &DatabaseConnection,
	builder: B,
) {
	let schema = Query::create_table()
		.table(PatchTestRecord::Table.into_iden())
		.col(
			ColumnDef::new(PatchTestRecord::Id)
				.big_integer()
				.primary_key(true),
		)
		.col(
			ColumnDef::new(PatchTestRecord::Tenant)
				.big_integer()
				.not_null(true),
		)
		.col(
			ColumnDef::new(PatchTestRecord::Name)
				.string_len(64)
				.not_null(true)
				.unique(true),
		)
		.col(
			ColumnDef::new(PatchTestRecord::Active)
				// Preserve the declared type used by SQLite's boolean decoder.
				.custom("BOOLEAN")
				.not_null(true),
		)
		.col(ColumnDef::new(PatchTestRecord::Notes).string_len(128))
		.col(
			ColumnDef::new(PatchTestRecord::Token)
				.string_len(128)
				.not_null(true),
		)
		.col(
			ColumnDef::new(PatchTestRecord::Revision)
				.big_integer()
				.not_null(true),
		)
		.to_string(builder.clone());
	connection.execute(&schema, vec![]).await.unwrap();
	let seed = Query::insert()
		.into_table(PatchTestRecord::Table.into_iden())
		.columns([
			PatchTestRecord::Id,
			PatchTestRecord::Tenant,
			PatchTestRecord::Name,
			PatchTestRecord::Active,
			PatchTestRecord::Notes,
			PatchTestRecord::Token,
			PatchTestRecord::Revision,
		])
		.values_panic([
			Value::from(1_i64),
			7_i64.into(),
			"Old".into(),
			true.into(),
			"note".into(),
			"secret".into(),
			Value::from(1_i64),
		])
		.values_panic([
			Value::from(2_i64),
			8_i64.into(),
			"Other".into(),
			true.into(),
			Value::String(None),
			"other_secret".into(),
			Value::from(1_i64),
		])
		.to_string(builder);
	connection.execute(&seed, vec![]).await.unwrap();
}

fn select_records(id: Option<i64>) -> String {
	let mut query = Query::select();
	query
		.expr(Expr::asterisk())
		.from(PatchTestRecord::Table.into_iden())
		.order_by(PatchTestRecord::Id, Order::Asc);
	if let Some(id) = id {
		query.and_where(Expr::col(PatchTestRecord::Id).eq(id));
	}
	query.to_string(SqliteQueryBuilder)
}

#[fixture]
async fn patch_database(test_database: TestDatabaseBuilder) -> PatchDatabase {
	let database = test_database
		.sqlite()
		.migrations::<EmptyMigrations>()
		.build()
		.await
		.unwrap();
	let lease = DatabaseConnectionLease::register(database.connection().clone()).unwrap();
	let connection = lease.handle();
	seed_patch_database(&connection, SqliteQueryBuilder).await;
	PatchDatabase {
		connection,
		_lease: lease,
		database,
	}
}

struct PatchConnections {
	second: DatabaseConnection,
	_second_lease: DatabaseConnectionLease,
	first: PatchDatabase,
}

#[fixture]
async fn patch_connections(#[future] patch_database: PatchDatabase) -> PatchConnections {
	let first = patch_database.await;
	let owner = reinhardt_db::backends::DatabaseConnection::connect_sqlite(first.database.url())
		.await
		.unwrap();
	let lease = DatabaseConnectionLease::register(owner).unwrap();
	PatchConnections {
		first,
		second: lease.handle(),
		_second_lease: lease,
	}
}

#[rstest]
#[tokio::test]
async fn patch_preserves_scope_and_unrelated_fields(#[future] patch_database: PatchDatabase) {
	let mut db = patch_database.await;
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
		.query(&select_records(None), vec![])
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
async fn patch_cannot_escape_tenant_scope(#[future] patch_database: PatchDatabase) {
	let mut db = patch_database.await;
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
		.query(&select_records(None), vec![])
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
async fn contextual_patch_checks_target_identity(#[future] patch_database: PatchDatabase) {
	let mut db = patch_database.await;
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
async fn patch_accepts_model_reference_and_optional_key(#[future] patch_database: PatchDatabase) {
	let mut db = patch_database.await;
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
async fn patch_preserves_typed_constraint_error_and_rolls_back(
	#[future] patch_database: PatchDatabase,
) {
	use reinhardt_forms::model_form::PatchError;
	let db = patch_database.await;
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
		.query(&select_records(Some(1)), vec![])
		.await
		.unwrap();
	assert_eq!(rows[0].get::<String>("name"), Some("Old".to_owned()));
	assert_eq!(rows[0].get::<bool>("active"), Some(true));
}

#[rstest]
#[tokio::test]
async fn patch_validation_error_rolls_back_transaction(#[future] patch_database: PatchDatabase) {
	use reinhardt_forms::model_form::PatchError;
	let db = patch_database.await;
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
		.query(&select_records(Some(1)), vec![])
		.await
		.unwrap();
	assert_eq!(rows[0].get::<bool>("active"), Some(true));
}

#[rstest]
#[case::partial_writers(false, "New")]
#[case::legacy_rotation_control(true, "Old")]
#[tokio::test]
async fn two_connections_preserve_disjoint_patches_after_stale_reads(
	#[future] patch_connections: PatchConnections,
	#[case] legacy_rotation: bool,
	#[case] expected_name: &str,
) {
	// Both independent connection pools read the old state before either writer
	// proceeds. A channel orders commits deterministically without sleeps.
	let mut connections = patch_connections.await;
	let db = &mut connections.first;
	let second = &mut connections.second;
	let first_snapshot = Record::objects()
		.filter(Record::field_id().eq(1_i64))
		.first_with_db(&mut db.connection)
		.await
		.unwrap()
		.unwrap();
	let mut second_snapshot = Record::objects()
		.filter(Record::field_id().eq(1_i64))
		.first_with_db(second)
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
				.update_with_conn(second, &second_snapshot)
				.await
				.unwrap();
		} else {
			assert_eq!(
				Record::objects()
					.filter(Record::field_tenant().eq(7))
					.filter(Record::field_id().eq(1_i64))
					.update_fields_with_conn(
						second,
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
		.query(&select_records(Some(1)), vec![])
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
async fn caller_predicate_controls_same_field_conflicts(#[future] patch_database: PatchDatabase) {
	let mut db = patch_database.await;
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
		.query(&select_records(Some(1)), vec![])
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
struct BackendPatchDatabase {
	connection: DatabaseConnection,
	_lease: DatabaseConnectionLease,
	_container: testcontainers::ContainerAsync<testcontainers::GenericImage>,
}

#[cfg(all(feature = "postgres", feature = "mysql"))]
#[fixture]
async fn postgres_patch_database(
	#[future] postgres_container: (
		testcontainers::ContainerAsync<testcontainers::GenericImage>,
		std::sync::Arc<sqlx::PgPool>,
		u16,
		String,
	),
) -> BackendPatchDatabase {
	let (container, _pool, _port, url) = postgres_container.await;
	let owner = reinhardt_db::backends::DatabaseConnection::connect_postgres(&url)
		.await
		.unwrap();
	let lease = DatabaseConnectionLease::register(owner).unwrap();
	let connection = lease.handle();
	seed_patch_database(&connection, PostgresQueryBuilder).await;
	BackendPatchDatabase {
		connection,
		_lease: lease,
		_container: container,
	}
}

#[cfg(all(feature = "postgres", feature = "mysql"))]
#[fixture]
async fn mysql_patch_database(
	#[future] mysql_container: (
		testcontainers::ContainerAsync<testcontainers::GenericImage>,
		std::sync::Arc<sqlx::MySqlPool>,
		u16,
		String,
	),
) -> BackendPatchDatabase {
	let (container, _pool, _port, url) = mysql_container.await;
	let owner = reinhardt_db::backends::DatabaseConnection::connect_mysql(&url)
		.await
		.unwrap();
	let lease = DatabaseConnectionLease::register(owner).unwrap();
	let connection = lease.handle();
	seed_patch_database(&connection, MySqlQueryBuilder).await;
	BackendPatchDatabase {
		connection,
		_lease: lease,
		_container: container,
	}
}

#[cfg(all(feature = "postgres", feature = "mysql"))]
#[rstest]
#[case::postgres(postgres_patch_database::default())]
#[case::mysql(mysql_patch_database::default())]
#[tokio::test]
async fn live_backend_counts_and_version_predicates(
	#[case]
	#[future]
	database: BackendPatchDatabase,
) {
	let mut database = database.await;
	let connection = &mut database.connection;
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
					connection
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
				connection
			)
			.await
			.unwrap()
			.rows_affected,
		0
	);
	let final_record = Record::objects()
		.filter(Record::field_id().eq(1_i64))
		.first_with_db(connection)
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

#[derive(Debug, reinhardt_query::Iden)]
enum PatchTestCluster {
	Table,
	Id,
	OrganizationId,
	Name,
	ApiUrl,
	IsActive,
}

#[fixture]
async fn cluster_database(#[future] patch_database: PatchDatabase) -> PatchDatabase {
	let db = patch_database.await;
	let schema = Query::create_table()
		.table(PatchTestCluster::Table.into_iden())
		.col(
			ColumnDef::new(PatchTestCluster::Id)
				.integer()
				.primary_key(true),
		)
		.col(
			ColumnDef::new(PatchTestCluster::OrganizationId)
				.big_integer()
				.not_null(true),
		)
		.col(
			ColumnDef::new(PatchTestCluster::Name)
				.string_len(64)
				.not_null(true),
		)
		.col(
			ColumnDef::new(PatchTestCluster::ApiUrl)
				.string_len(2048)
				.not_null(true),
		)
		.col(
			ColumnDef::new(PatchTestCluster::IsActive)
				// Preserve the declared type used by SQLite's boolean decoder.
				.custom("BOOLEAN")
				.not_null(true),
		)
		.to_string(SqliteQueryBuilder);
	db.connection.execute(&schema, vec![]).await.unwrap();
	let seed = Query::insert()
		.into_table(PatchTestCluster::Table.into_iden())
		.columns([
			PatchTestCluster::Id,
			PatchTestCluster::OrganizationId,
			PatchTestCluster::Name,
			PatchTestCluster::ApiUrl,
			PatchTestCluster::IsActive,
		])
		.values_panic([
			Value::from(1_i64),
			7_i64.into(),
			"Old".into(),
			"https://old.example".into(),
			true.into(),
		])
		.to_string(SqliteQueryBuilder);
	db.connection.execute(&seed, vec![]).await.unwrap();
	db
}

#[rstest]
#[tokio::test]
async fn cloud_consumer_keeps_rbac_scope_validation_and_response_refresh(
	#[future] cluster_database: PatchDatabase,
) {
	let mut db = cluster_database.await;
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
	assert!(!response.is_active);
	assert_eq!(response.organization_id, 7);
}

#[rstest]
#[tokio::test]
async fn grouped_scope_cannot_expand_target_on_database(#[future] patch_database: PatchDatabase) {
	let mut db = patch_database.await;
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
		.query(&select_records(None), vec![])
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
#[case::without_snapshot(false)]
#[case::with_snapshot(true)]
#[tokio::test]
async fn named_subset_contract_does_not_reference_unselected_model_fields(
	#[case] with_snapshot: bool,
) {
	use reinhardt_db::orm::connection::QueryValue;
	let mut executor = RecordingExecutor::default();
	let data = serde_json::from_value(json!({"title": "New"})).unwrap();
	let existing = Subset {
		id: Some(4),
		title: "Old".to_owned(),
		untouched: "Preserved".to_owned(),
	};
	let patch = if with_snapshot {
		EditSubset::validate_patch_with_existing(data, &existing)
	} else {
		EditSubset::validate_patch(data)
	}
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
