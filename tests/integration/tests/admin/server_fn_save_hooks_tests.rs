//! Integration tests for asynchronous `ModelAdmin` save hooks.

use super::server_fn_helpers::{TEST_CSRF_TOKEN, make_auth_user, make_staff_request};
use reinhardt_admin::adapters::MutationRequest;
use reinhardt_admin::core::{
	AdminChange, AdminDatabase, AdminDatabaseKey, AdminError, AdminFormMode, AdminRecord,
	AdminResult, AdminSaveContext, AdminSite, AdminSiteKey, AdminUser, ImportFormat, ModelAdmin,
};
use reinhardt_admin::server::{create_record, import_data, update_inline_edits, update_record};
use reinhardt_admin::types::{InlineEditError, InlineEditMutation, InlineEditRequest};
use reinhardt_db::backends::connection::DatabaseConnection as BackendsConnection;
use reinhardt_db::backends::dialect::PostgresBackend;
use reinhardt_db::migrations::{FieldMetadata, FieldType, ModelMetadata, global_registry};
use reinhardt_db::orm::DatabaseConnectionLease;
use reinhardt_di::KeyedDepends;
use reinhardt_pages::server_fn::ServerFnErrorKind;
use reinhardt_test::fixtures::shared_postgres::shared_db_pool;
use rstest::*;
use serde_json::{Value, json};
use serial_test::serial;
use sqlx::Executor;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

const APP_LABEL: &str = "admin_save_hooks";
const MODEL_NAME: &str = "SaveHookAccount";
const TABLE_NAME: &str = "admin_save_hook_accounts";
/// Stored name of an account whose sessions cannot be ended.
const UNREVOKABLE_NAME: &str = "unrevokable";
const REJECTION: &str = "sessions could not be ended";
/// Name that new accounts may not use.
const RESERVED_NAME: &str = "reserved";
/// Name whose creation the hook denies as a permission failure.
const FORBIDDEN_NAME: &str = "forbidden";

struct RegistryGuard;

impl Drop for RegistryGuard {
	fn drop(&mut self) {
		global_registry().remove_model(APP_LABEL, MODEL_NAME);
	}
}

/// One observed save hook invocation.
#[derive(Debug, Clone, PartialEq)]
struct HookCall {
	hook: &'static str,
	mode: AdminFormMode,
	pk: Option<String>,
	stored_is_active: Option<Value>,
	changed_fields: Vec<String>,
	is_active_transition: Option<(bool, bool)>,
}

impl HookCall {
	fn update(
		hook: &'static str,
		pk: &str,
		stored_is_active: bool,
		changed_fields: &[&str],
		is_active_transition: Option<(bool, bool)>,
	) -> Self {
		Self {
			hook,
			mode: AdminFormMode::Update,
			pk: Some(pk.to_owned()),
			stored_is_active: Some(json!(stored_is_active)),
			changed_fields: changed_fields
				.iter()
				.map(|&field| field.to_owned())
				.collect(),
			is_active_transition,
		}
	}
}

#[derive(Clone, Default)]
struct HookRecorder(Arc<Mutex<Vec<HookCall>>>);

impl HookRecorder {
	fn record(&self, hook: &'static str, ctx: &AdminSaveContext<'_>, change: &AdminChange) {
		let call = HookCall {
			hook,
			mode: ctx.mode(),
			pk: ctx.pk().map(str::to_owned),
			stored_is_active: ctx
				.stored()
				.and_then(|stored| stored.get("is_active").cloned()),
			changed_fields: change.changed_fields().map(str::to_owned).collect(),
			is_active_transition: change.transition::<bool>("is_active"),
		};
		self.0
			.lock()
			.unwrap_or_else(PoisonError::into_inner)
			.push(call);
	}

	fn calls(&self) -> Vec<HookCall> {
		self.0
			.lock()
			.unwrap_or_else(PoisonError::into_inner)
			.clone()
	}
}

/// Admin that must end an account's sessions before reactivating it.
struct SaveHookAdmin {
	recorder: HookRecorder,
}

#[async_trait::async_trait]
impl ModelAdmin for SaveHookAdmin {
	fn model_name(&self) -> &str {
		MODEL_NAME
	}

	fn table_name(&self) -> &str {
		TABLE_NAME
	}

	fn list_display(&self) -> Vec<&str> {
		vec!["id", "name", "is_active"]
	}

	fn list_editable(&self) -> Vec<&str> {
		vec!["name", "is_active"]
	}

	fn fields(&self) -> Option<Vec<&str>> {
		Some(vec!["name", "is_active"])
	}

	async fn before_save(
		&self,
		ctx: &AdminSaveContext<'_>,
		change: &AdminChange,
	) -> AdminResult<()> {
		self.recorder.record("before_save", ctx, change);
		let unrevokable = ctx
			.stored()
			.and_then(|stored| stored.get("name"))
			.is_some_and(|name| name == UNREVOKABLE_NAME);
		if change.transition::<bool>("is_active") == Some((false, true)) && unrevokable {
			return Err(AdminError::ValidationError(REJECTION.to_owned()));
		}
		if ctx.mode() == AdminFormMode::Create
			&& change.new_value("name") == Some(&json!(RESERVED_NAME))
		{
			return Err(AdminError::ValidationError("name is reserved".to_owned()));
		}
		if ctx.mode() == AdminFormMode::Create
			&& change.new_value("name") == Some(&json!(FORBIDDEN_NAME))
		{
			return Err(AdminError::PermissionDenied(
				"account creation is not allowed".to_owned(),
			));
		}
		Ok(())
	}

	async fn after_save(&self, ctx: &AdminSaveContext<'_>, change: &AdminChange) {
		self.recorder.record("after_save", ctx, change);
	}

	async fn has_view_permission(&self, _user: &dyn AdminUser) -> bool {
		true
	}

	async fn has_add_permission(&self, _user: &dyn AdminUser) -> bool {
		true
	}

	async fn has_change_permission(&self, _user: &dyn AdminUser) -> bool {
		true
	}
}

struct SaveHookContext {
	site: KeyedDepends<AdminSiteKey, AdminSite>,
	db: KeyedDepends<AdminDatabaseKey, AdminDatabase>,
	pool: sqlx::PgPool,
	recorder: HookRecorder,
	_connection_lease: DatabaseConnectionLease,
	_registry: RegistryGuard,
}

impl SaveHookContext {
	async fn seed(&self, name: &str, is_active: bool) {
		sqlx::query(&format!(
			"INSERT INTO {TABLE_NAME} (name, is_active) VALUES ($1, $2)"
		))
		.bind(name)
		.bind(is_active)
		.execute(&self.pool)
		.await
		.expect("save hook fixture row should be inserted");
	}

	async fn row_count(&self) -> i64 {
		sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {TABLE_NAME}"))
			.fetch_one(&self.pool)
			.await
			.expect("save hook row count should be readable")
	}

	async fn stored(&self, id: &str) -> HashMap<String, Value> {
		self.db
			.get::<AdminRecord>(TABLE_NAME, "id", id)
			.await
			.expect("save hook record lookup should succeed")
			.expect("save hook record should exist")
	}
}

#[fixture]
async fn save_hook_context(#[future] shared_db_pool: (sqlx::PgPool, String)) -> SaveHookContext {
	let (pool, _) = shared_db_pool.await;
	pool.execute(format!("DROP TABLE IF EXISTS {TABLE_NAME}").as_str())
		.await
		.expect("save hook table should be removed");
	pool.execute(
		format!(
			"CREATE TABLE {TABLE_NAME} (\
				id SERIAL PRIMARY KEY, \
				name VARCHAR(255) NOT NULL, \
				is_active BOOLEAN NOT NULL\
			)"
		)
		.as_str(),
	)
	.await
	.expect("save hook table should be created");

	let mut metadata = ModelMetadata::new(APP_LABEL, MODEL_NAME, TABLE_NAME);
	metadata.add_field("id".to_owned(), FieldMetadata::new(FieldType::Integer));
	metadata.add_field(
		"name".to_owned(),
		FieldMetadata::new(FieldType::VarChar(255)),
	);
	metadata.add_field(
		"is_active".to_owned(),
		FieldMetadata::new(FieldType::Boolean),
	);
	global_registry().register_model(metadata);

	let backend = Arc::new(PostgresBackend::new(pool.clone()));
	let connection_lease = DatabaseConnectionLease::register(BackendsConnection::new(backend))
		.expect("save hook connection should register");
	let mut connection = connection_lease.handle();
	super::server_fn_helpers::setup_admin_history_schema(&mut connection).await;
	let db = AdminDatabase::new(connection_lease.handle());
	let recorder = HookRecorder::default();
	let site = AdminSite::new("Save hook test site");
	site.register(
		MODEL_NAME,
		SaveHookAdmin {
			recorder: recorder.clone(),
		},
	)
	.expect("save hook admin should register");

	SaveHookContext {
		site: KeyedDepends::from_value(site),
		db: KeyedDepends::from_value(db),
		pool,
		recorder,
		_connection_lease: connection_lease,
		_registry: RegistryGuard,
	}
}

fn mutation_request(data: &[(&str, Value)]) -> MutationRequest {
	MutationRequest {
		csrf_token: TEST_CSRF_TOKEN.to_owned(),
		data: data
			.iter()
			.map(|(field, value)| ((*field).to_owned(), value.clone()))
			.collect(),
	}
}

fn inline_request(updates: &[(&str, &str, Value)]) -> InlineEditRequest {
	InlineEditRequest {
		csrf_token: TEST_CSRF_TOKEN.to_owned(),
		updates: updates
			.iter()
			.map(|(object_id, field, value)| InlineEditMutation {
				object_id: (*object_id).to_owned(),
				changes: HashMap::from([((*field).to_owned(), value.clone())]),
				json_fields: Vec::new(),
			})
			.collect(),
	}
}

#[rstest]
#[tokio::test]
#[serial(admin_save_hooks)]
async fn update_rejected_by_before_save_returns_validation_error_and_keeps_row(
	#[future] save_hook_context: SaveHookContext,
) {
	// Arrange
	let context = save_hook_context.await;
	context.seed(UNREVOKABLE_NAME, false).await;

	// Act
	let error = update_record(
		MODEL_NAME.to_owned(),
		"1".to_owned(),
		mutation_request(&[("is_active", json!(true))]),
		context.site.clone(),
		context.db.clone(),
		make_staff_request(),
		make_auth_user(),
	)
	.await
	.expect_err("before_save must reject the reactivation");

	// Assert
	assert_eq!(error.kind(), ServerFnErrorKind::Validation);
	assert_eq!(error.status(), Some(422));
	assert_eq!(
		error
			.field_errors()
			.iter()
			.map(|error| (error.field(), error.message()))
			.collect::<Vec<_>>(),
		vec![("_all", REJECTION)]
	);
	assert_eq!(
		context.stored("1").await.get("is_active"),
		Some(&json!(false))
	);
	assert_eq!(
		context.recorder.calls(),
		vec![HookCall::update(
			"before_save",
			"1",
			false,
			&["is_active"],
			Some((false, true)),
		)]
	);
}

#[rstest]
#[tokio::test]
#[serial(admin_save_hooks)]
async fn update_runs_before_and_after_save_with_stored_record_and_transition(
	#[future] save_hook_context: SaveHookContext,
) {
	// Arrange
	let context = save_hook_context.await;
	context.seed("alice", false).await;

	// Act
	update_record(
		MODEL_NAME.to_owned(),
		"1".to_owned(),
		mutation_request(&[("name", json!("alice")), ("is_active", json!(true))]),
		context.site.clone(),
		context.db.clone(),
		make_staff_request(),
		make_auth_user(),
	)
	.await
	.expect("before_save must permit the reactivation");

	// Assert
	assert_eq!(
		context.stored("1").await.get("is_active"),
		Some(&json!(true))
	);
	assert_eq!(
		context.recorder.calls(),
		vec![
			HookCall::update(
				"before_save",
				"1",
				false,
				&["is_active"],
				Some((false, true)),
			),
			HookCall::update(
				"after_save",
				"1",
				false,
				&["is_active"],
				Some((false, true)),
			),
		]
	);
}

#[rstest]
#[tokio::test]
#[serial(admin_save_hooks)]
async fn create_runs_hooks_without_stored_record_and_reports_new_pk_after_save(
	#[future] save_hook_context: SaveHookContext,
) {
	// Arrange
	let context = save_hook_context.await;
	let created_call = |hook, pk: Option<&str>| HookCall {
		hook,
		mode: AdminFormMode::Create,
		pk: pk.map(str::to_owned),
		stored_is_active: None,
		changed_fields: vec!["is_active".to_owned(), "name".to_owned()],
		is_active_transition: None,
	};

	// Act
	create_record(
		MODEL_NAME.to_owned(),
		mutation_request(&[("name", json!("bob")), ("is_active", json!(true))]),
		context.site.clone(),
		context.db.clone(),
		make_staff_request(),
		make_auth_user(),
	)
	.await
	.expect("create should succeed");

	// Assert
	assert_eq!(context.stored("1").await.get("name"), Some(&json!("bob")));
	assert_eq!(
		context.recorder.calls(),
		vec![
			created_call("before_save", None),
			created_call("after_save", Some("1")),
		]
	);
}

#[rstest]
#[tokio::test]
#[serial(admin_save_hooks)]
async fn import_runs_hooks_per_record_and_counts_rejected_records_as_failed(
	#[future] save_hook_context: SaveHookContext,
) {
	// Arrange
	let context = save_hook_context.await;
	let created_call = |hook, pk: Option<&str>| HookCall {
		hook,
		mode: AdminFormMode::Create,
		pk: pk.map(str::to_owned),
		stored_is_active: None,
		changed_fields: vec!["is_active".to_owned(), "name".to_owned()],
		is_active_transition: None,
	};
	let records = serde_json::to_vec(&json!([
		{"name": "dave", "is_active": true},
		{"name": RESERVED_NAME, "is_active": false},
	]))
	.expect("import fixture should serialize");

	// Act
	let response = import_data(
		MODEL_NAME.to_owned(),
		ImportFormat::JSON,
		records,
		context.site.clone(),
		context.db.clone(),
		make_staff_request(),
		make_auth_user(),
	)
	.await
	.expect("record-local hook rejections must return statistics");

	// Assert
	assert_eq!(response.imported, 1);
	assert_eq!(response.failed, 1);
	assert_eq!(
		response.errors,
		Some(vec!["Record 2: import failed".to_owned()])
	);
	assert_eq!(context.stored("1").await.get("name"), Some(&json!("dave")));
	assert_eq!(context.row_count().await, 1);
	assert_eq!(
		context.recorder.calls(),
		vec![
			created_call("before_save", None),
			created_call("after_save", Some("1")),
			created_call("before_save", None),
		]
	);
}

#[rstest]
#[tokio::test]
#[serial(admin_save_hooks)]
async fn import_aborts_with_hook_permission_error_instead_of_counting_failed_record(
	#[future] save_hook_context: SaveHookContext,
) {
	// Arrange
	let context = save_hook_context.await;
	let records = serde_json::to_vec(&json!([
		{"name": "dave", "is_active": true},
		{"name": FORBIDDEN_NAME, "is_active": true},
		{"name": "erin", "is_active": true},
	]))
	.expect("import fixture should serialize");

	// Act
	let error = import_data(
		MODEL_NAME.to_owned(),
		ImportFormat::JSON,
		records,
		context.site.clone(),
		context.db.clone(),
		make_staff_request(),
		make_auth_user(),
	)
	.await
	.expect_err("a hook permission denial must abort the import");

	// Assert
	assert_eq!(error.kind(), ServerFnErrorKind::Server);
	assert_eq!(error.status(), Some(403));
	assert_eq!(error.user_message(), "account creation is not allowed");
	assert_eq!(context.row_count().await, 1);
	assert_eq!(context.stored("1").await.get("name"), Some(&json!("dave")));
}

#[rstest]
#[tokio::test]
#[serial(admin_save_hooks)]
async fn inline_edit_rejected_row_rolls_back_batch_and_reports_row_error(
	#[future] save_hook_context: SaveHookContext,
) {
	// Arrange
	let context = save_hook_context.await;
	context.seed("alice", false).await;
	context.seed(UNREVOKABLE_NAME, false).await;

	// Act
	let response = update_inline_edits(
		MODEL_NAME.to_owned(),
		inline_request(&[
			("1", "is_active", json!(true)),
			("2", "is_active", json!(true)),
		]),
		context.site.clone(),
		context.db.clone(),
		make_staff_request(),
		make_auth_user(),
	)
	.await
	.expect("rejected rows are returned as typed errors");

	// Assert
	assert_eq!(response.updated, 0);
	assert_eq!(response.outcomes, vec![]);
	assert_eq!(
		response.errors,
		vec![InlineEditError {
			object_id: "2".to_owned(),
			field: None,
			message: REJECTION.to_owned(),
		}]
	);
	assert_eq!(
		context.stored("1").await.get("is_active"),
		Some(&json!(false))
	);
	assert_eq!(
		context.stored("2").await.get("is_active"),
		Some(&json!(false))
	);
	assert_eq!(
		context.recorder.calls(),
		vec![
			HookCall::update(
				"before_save",
				"1",
				false,
				&["is_active"],
				Some((false, true)),
			),
			HookCall::update(
				"before_save",
				"2",
				false,
				&["is_active"],
				Some((false, true)),
			),
		]
	);
}

#[rstest]
#[tokio::test]
#[serial(admin_save_hooks)]
async fn inline_edit_runs_before_save_per_row_and_after_save_after_commit(
	#[future] save_hook_context: SaveHookContext,
) {
	// Arrange
	let context = save_hook_context.await;
	context.seed("alice", false).await;
	context.seed("carol", true).await;

	// Act
	let response = update_inline_edits(
		MODEL_NAME.to_owned(),
		inline_request(&[
			("1", "is_active", json!(true)),
			("2", "name", json!("caroline")),
		]),
		context.site.clone(),
		context.db.clone(),
		make_staff_request(),
		make_auth_user(),
	)
	.await
	.expect("permitted inline edits should commit");

	// Assert
	assert_eq!(response.updated, 2);
	assert_eq!(response.errors, vec![]);
	assert_eq!(
		context.stored("1").await.get("is_active"),
		Some(&json!(true))
	);
	assert_eq!(
		context.stored("2").await.get("name"),
		Some(&json!("caroline"))
	);
	assert_eq!(
		context.recorder.calls(),
		vec![
			HookCall::update(
				"before_save",
				"1",
				false,
				&["is_active"],
				Some((false, true)),
			),
			HookCall::update("before_save", "2", true, &["name"], None),
			HookCall::update(
				"after_save",
				"1",
				false,
				&["is_active"],
				Some((false, true)),
			),
			HookCall::update("after_save", "2", true, &["name"], None),
		]
	);
}
