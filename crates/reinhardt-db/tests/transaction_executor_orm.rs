use async_trait::async_trait;
use reinhardt_db::backends::error::{DatabaseError, Result as BackendResult};
use reinhardt_db::backends::types::{
	DatabaseType, QueryResult, QueryValue, Row, TransactionExecutor,
};
#[cfg(feature = "sqlite")]
use reinhardt_db::orm::DatabaseConnection;
use reinhardt_db::orm::events::{EventRegistry, EventResult, MapperEvents, set_active_registry};
use reinhardt_db::orm::model::FieldSelector;
use reinhardt_db::orm::{Filter, FilterOperator, FilterValue, Manager, Model, QuerySet};
use rstest::rstest;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serial_test::serial;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
struct RecordedQuery {
	operation: &'static str,
	sql: String,
	params: Vec<QueryValue>,
}

type RecordingState = (
	RecordingExecutor,
	Arc<Mutex<Vec<String>>>,
	Arc<Mutex<Vec<RecordedQuery>>>,
);

#[derive(Clone)]
struct RecordingExecutor {
	backend: DatabaseType,
	operations: Arc<Mutex<Vec<String>>>,
	queries: Arc<Mutex<Vec<RecordedQuery>>>,
	lookup_rows: usize,
	model_id: i64,
	model_name: String,
	generated_id: Option<QueryValue>,
}

impl RecordingExecutor {
	fn model_row(&self) -> Row {
		let mut row = Row::new();
		row.insert("id".to_string(), QueryValue::Int(self.model_id));
		row.insert(
			"name".to_string(),
			QueryValue::String(self.model_name.clone()),
		);
		row
	}

	fn record(&self, operation: &'static str) {
		self.operations
			.lock()
			.expect("operations mutex should not be poisoned")
			.push(operation.to_string());
	}

	fn record_query(&self, operation: &'static str, sql: &str, params: &[QueryValue]) {
		self.record(operation);
		self.queries
			.lock()
			.expect("queries mutex should not be poisoned")
			.push(RecordedQuery {
				operation,
				sql: sql.to_string(),
				params: params.to_vec(),
			});
	}
}

#[async_trait]
impl TransactionExecutor for RecordingExecutor {
	fn backend(&self) -> DatabaseType {
		self.backend
	}

	async fn execute(&mut self, sql: &str, params: Vec<QueryValue>) -> BackendResult<QueryResult> {
		let operation = if sql.to_ascii_uppercase().starts_with("UPDATE")
			|| sql.to_ascii_uppercase().starts_with("INSERT")
		{
			"save"
		} else {
			"delete"
		};
		self.record_query(operation, sql, &params);
		Ok(QueryResult { rows_affected: 1 })
	}

	async fn fetch_one(&mut self, sql: &str, params: Vec<QueryValue>) -> BackendResult<Row> {
		if sql.to_ascii_uppercase().contains("COUNT") {
			self.record_query("count", sql, &params);
			let mut row = Row::new();
			row.insert("count".to_string(), QueryValue::Int(1));
			Ok(row)
		} else if sql.to_ascii_uppercase().contains("LAST_INSERT_ID") {
			let is_reset = sql.to_ascii_uppercase().contains("LAST_INSERT_ID(0)");
			let operation = if is_reset {
				"reset_generated_id"
			} else {
				"generated_id"
			};
			self.record_query(operation, sql, &params);
			let mut row = Row::new();
			let generated_id = if is_reset {
				Some(QueryValue::Int(0))
			} else {
				self.generated_id.clone()
			};
			if let Some(generated_id) = generated_id {
				row.insert("generated_id".to_string(), generated_id);
			}
			Ok(row)
		} else {
			let operation = if sql.to_ascii_uppercase().starts_with("SELECT") {
				"reload"
			} else {
				"save"
			};
			self.record_query(operation, sql, &params);
			Ok(self.model_row())
		}
	}

	async fn fetch_all(&mut self, sql: &str, params: Vec<QueryValue>) -> BackendResult<Vec<Row>> {
		self.record_query("fetch_optional", sql, &params);
		let row_count = if sql.to_ascii_uppercase().contains("LIMIT") {
			self.lookup_rows.min(2)
		} else {
			self.lookup_rows
		};
		Ok((0..row_count).map(|_| self.model_row()).collect())
	}

	async fn fetch_optional(
		&mut self,
		_sql: &str,
		_params: Vec<QueryValue>,
	) -> BackendResult<Option<Row>> {
		Ok(Some(self.model_row()))
	}

	async fn commit(self: Box<Self>) -> BackendResult<()> {
		Ok(())
	}

	async fn rollback(self: Box<Self>) -> BackendResult<()> {
		Ok(())
	}
}

fn recording_executor(backend: DatabaseType, lookup_rows: usize) -> RecordingState {
	let operations = Arc::new(Mutex::new(Vec::new()));
	let queries = Arc::new(Mutex::new(Vec::new()));
	(
		RecordingExecutor {
			backend,
			operations: operations.clone(),
			queries: queries.clone(),
			lookup_rows,
			model_id: 1,
			model_name: "updated".to_string(),
			generated_id: Some(QueryValue::Int(42)),
		},
		operations,
		queries,
	)
}

struct RecordingMapperEvents {
	operations: Arc<Mutex<Vec<String>>>,
	veto: bool,
}

impl RecordingMapperEvents {
	fn record(&self, event: &'static str) -> EventResult {
		self.operations
			.lock()
			.expect("operations mutex should not be poisoned")
			.push(event.to_string());
		if self.veto {
			EventResult::Veto
		} else {
			EventResult::Continue
		}
	}
}

#[async_trait]
impl MapperEvents for RecordingMapperEvents {
	async fn before_insert(&self, _instance_id: &str, _values: &JsonValue) -> EventResult {
		self.record("before_insert")
	}

	async fn after_insert(&self, _instance_id: &str) -> EventResult {
		self.record("after_insert")
	}

	async fn before_update(&self, _instance_id: &str, _values: &JsonValue) -> EventResult {
		self.record("before_update")
	}

	async fn after_update(&self, _instance_id: &str) -> EventResult {
		self.record("after_update")
	}

	async fn before_delete(&self, _instance_id: &str) -> EventResult {
		self.record("before_delete")
	}

	async fn after_delete(&self, _instance_id: &str) -> EventResult {
		self.record("after_delete")
	}
}

#[derive(Clone)]
struct WidgetFields;

impl FieldSelector for WidgetFields {
	fn with_alias(self, _alias: &str) -> Self {
		self
	}
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Widget {
	id: Option<i64>,
	name: String,
}

impl Model for Widget {
	type Fields = WidgetFields;
	type Objects = Manager<Self>;
	type PrimaryKey = i64;

	fn table_name() -> &'static str {
		"widgets"
	}

	fn new_fields() -> Self::Fields {
		WidgetFields
	}

	fn primary_key(&self) -> Option<Self::PrimaryKey> {
		self.id
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct RenamedPrimaryKeyWidget {
	id: Option<i64>,
	name: String,
}

impl Model for RenamedPrimaryKeyWidget {
	type Fields = WidgetFields;
	type Objects = Manager<Self>;
	type PrimaryKey = i64;

	fn table_name() -> &'static str {
		"renamed_primary_key_widgets"
	}

	fn new_fields() -> Self::Fields {
		WidgetFields
	}

	fn primary_key(&self) -> Option<Self::PrimaryKey> {
		self.id
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}

	fn primary_key_column() -> &'static str {
		"widget_id"
	}
}

#[cfg(feature = "sqlite")]
#[tokio::test]
async fn connection_queryset_count_ignores_slice_and_one_caps_at_two() {
	let directory = tempfile::Builder::new()
		.prefix("reinhardt-db-queryset-")
		.tempdir_in("/tmp")
		.expect("temporary database directory should be created");
	let database_url = format!(
		"sqlite:///{}",
		directory.path().join("queryset.sqlite").display()
	);
	let connection = DatabaseConnection::connect_sqlite(&database_url)
		.await
		.expect("SQLite connection should open");
	connection
		.execute(
			"CREATE TABLE widgets (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
			vec![],
		)
		.await
		.expect("widget table should be created");
	for id in 1..=4 {
		connection
			.execute(
				"INSERT INTO widgets (id, name) VALUES (?, ?)",
				vec![
					QueryValue::Int(id),
					QueryValue::String("visible".to_owned()),
				],
			)
			.await
			.expect("widget row should be inserted");
	}

	let queryset = QuerySet::<Widget>::new()
		.filter(Filter::new(
			"name",
			FilterOperator::Eq,
			FilterValue::String("visible".to_owned()),
		))
		.offset(1)
		.limit(1);
	let count = queryset
		.count_with_db(&connection)
		.await
		.expect("connection-backed count should succeed");
	let strict = queryset
		.one_with_db(&connection)
		.await
		.expect("existing stricter limit should be preserved");
	let capped = QuerySet::<Widget>::new()
		.one_with_db(&connection)
		.await
		.expect("unbounded detail query should cap at two rows");

	assert_eq!(count, 4);
	assert_eq!(strict.len(), 1);
	assert_eq!(capped.len(), 2);
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct StringKeyWidget {
	id: Option<String>,
	name: String,
}

impl Model for StringKeyWidget {
	type Fields = WidgetFields;
	type Objects = Manager<Self>;
	type PrimaryKey = String;

	fn table_name() -> &'static str {
		"string_key_widgets"
	}

	fn new_fields() -> Self::Fields {
		WidgetFields
	}

	fn primary_key(&self) -> Option<Self::PrimaryKey> {
		self.id.clone()
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}
}

#[tokio::test]
async fn orm_operations_use_the_caller_owned_transaction_executor() {
	let (mut executor, operations, _queries) = recording_executor(DatabaseType::Postgres, 1);
	let queryset = QuerySet::<Widget>::new();
	let mut widget = Widget {
		id: Some(1),
		name: "updated".to_string(),
	};

	let count = queryset
		.count_with_executor(&mut executor)
		.await
		.expect("count should use the transaction executor");
	let matches = queryset
		.one_with_executor(&mut executor)
		.await
		.expect("lookup should use the transaction executor");
	widget
		.save_with_executor(&mut executor)
		.await
		.expect("save should use the transaction executor");
	widget
		.delete_with_executor(&mut executor)
		.await
		.expect("delete should use the transaction executor");

	assert_eq!(count, 1);
	assert_eq!(matches, vec![widget]);
	assert_eq!(
		operations
			.lock()
			.expect("operations mutex should not be poisoned")
			.as_slice(),
		["count", "fetch_optional", "save", "delete"]
	);
}

#[tokio::test]
async fn all_and_one_with_executor_preserve_the_two_row_lookup_boundary() {
	let (mut executor, _operations, _queries) = recording_executor(DatabaseType::Postgres, 3);
	let queryset = QuerySet::<Widget>::new();

	let all = queryset
		.all_with_executor(&mut executor)
		.await
		.expect("all should return every executor row");
	let one = queryset
		.one_with_executor(&mut executor)
		.await
		.expect("one should preserve up to two executor rows");

	assert_eq!(all.len(), 3);
	assert_eq!(one.len(), 2);
}

#[rstest]
#[case(
	DatabaseType::Postgres,
	r#"SELECT * FROM "widgets" WHERE "name" = $1"#,
	r#"UPDATE "widgets" SET "name" = $1 WHERE "id" = $2 RETURNING "id", "name""#
)]
#[case(
	DatabaseType::Mysql,
	"SELECT * FROM `widgets` WHERE `name` = ?",
	"UPDATE `widgets` SET `name` = ? WHERE `id` = ?"
)]
#[case(
	DatabaseType::Sqlite,
	r#"SELECT * FROM "widgets" WHERE "name" = ?"#,
	r#"UPDATE "widgets" SET "name" = ? WHERE "id" = ? RETURNING "id", "name""#
)]
#[tokio::test]
async fn executor_backend_controls_sql_placeholders_and_bound_values(
	#[case] backend: DatabaseType,
	#[case] expected_read_sql: &str,
	#[case] expected_write_sql: &str,
) {
	let (mut executor, _operations, queries) = recording_executor(backend, 1);
	let queryset = QuerySet::<Widget>::new().filter(Filter::new(
		"name",
		FilterOperator::Eq,
		FilterValue::String("needle".to_string()),
	));
	let mut widget = Widget {
		id: Some(7),
		name: "updated".to_string(),
	};

	queryset
		.all_with_executor(&mut executor)
		.await
		.expect("read should use the executor backend");
	widget
		.save_with_executor(&mut executor)
		.await
		.expect("write should use the executor backend");

	let mut expected_queries = vec![
		RecordedQuery {
			operation: "fetch_optional",
			sql: expected_read_sql.to_string(),
			params: vec![QueryValue::String("needle".to_string())],
		},
		RecordedQuery {
			operation: "save",
			sql: expected_write_sql.to_string(),
			params: vec![
				QueryValue::String("updated".to_string()),
				QueryValue::Int(7),
			],
		},
	];
	if backend == DatabaseType::Mysql {
		expected_queries.push(RecordedQuery {
			operation: "reload",
			sql: "SELECT * FROM `widgets` WHERE `id` = ?".to_string(),
			params: vec![QueryValue::Int(7)],
		});
	}
	assert_eq!(
		queries
			.lock()
			.expect("queries mutex should not be poisoned")
			.as_slice(),
		expected_queries
	);
}

#[tokio::test]
async fn mysql_insert_uses_generated_id_and_reloads_on_the_same_executor() {
	let (mut executor, _operations, queries) = recording_executor(DatabaseType::Mysql, 1);
	executor.model_id = 42;
	executor.model_name = "created".to_string();
	let mut widget = Widget {
		id: None,
		name: "created".to_string(),
	};

	widget
		.save_with_executor(&mut executor)
		.await
		.expect("MySQL insert should reload the generated primary key");

	assert_eq!(
		widget,
		Widget {
			id: Some(42),
			name: "created".to_string(),
		}
	);
	assert_eq!(
		queries
			.lock()
			.expect("queries mutex should not be poisoned")
			.as_slice(),
		[
			RecordedQuery {
				operation: "reset_generated_id",
				sql: "SELECT LAST_INSERT_ID(0) AS generated_id".to_string(),
				params: vec![],
			},
			RecordedQuery {
				operation: "save",
				sql: "INSERT INTO `widgets` (`name`) VALUES (?)".to_string(),
				params: vec![QueryValue::String("created".to_string())],
			},
			RecordedQuery {
				operation: "generated_id",
				sql: "SELECT CAST(LAST_INSERT_ID() AS SIGNED) AS generated_id".to_string(),
				params: vec![],
			},
			RecordedQuery {
				operation: "reload",
				sql: "SELECT * FROM `widgets` WHERE `id` = ?".to_string(),
				params: vec![QueryValue::Int(42)],
			},
		]
	);
}

#[tokio::test]
async fn mysql_insert_without_an_integer_generated_id_returns_an_error() {
	let (mut executor, _operations, queries) = recording_executor(DatabaseType::Mysql, 1);
	executor.generated_id = Some(QueryValue::Int(0));
	let mut widget = StringKeyWidget {
		id: None,
		name: "created".to_string(),
	};

	let error = widget
		.save_with_executor(&mut executor)
		.await
		.expect_err("MySQL insert should reject unsupported generated primary keys");

	assert_eq!(
		error,
		reinhardt_db::backends::error::DatabaseError::NotSupported(
			"MySQL executor inserts without an explicit primary key require an auto-increment integer primary key"
				.to_string(),
		)
	);
	assert_eq!(widget.id, None);
	assert_eq!(
		queries
			.lock()
			.expect("queries mutex should not be poisoned")
			.as_slice(),
		[
			RecordedQuery {
				operation: "reset_generated_id",
				sql: "SELECT LAST_INSERT_ID(0) AS generated_id".to_string(),
				params: vec![],
			},
			RecordedQuery {
				operation: "save",
				sql: "INSERT INTO `string_key_widgets` (`name`) VALUES (?)".to_string(),
				params: vec![QueryValue::String("created".to_string())],
			},
			RecordedQuery {
				operation: "generated_id",
				sql: "SELECT CAST(LAST_INSERT_ID() AS SIGNED) AS generated_id".to_string(),
				params: vec![],
			},
		]
	);
}

#[rstest]
#[case::missing(
	None,
	DatabaseError::ColumnNotFound("generated_id".to_string())
)]
#[case::noninteger(
	Some(QueryValue::String("not-an-integer".to_string())),
	DatabaseError::TypeError("Cannot convert String(\"not-an-integer\") to i64".to_string())
)]
#[tokio::test]
async fn mysql_insert_rejects_invalid_generated_id_responses_without_reloading(
	#[case] generated_id: Option<QueryValue>,
	#[case] expected_error: DatabaseError,
) {
	let (mut executor, _operations, queries) = recording_executor(DatabaseType::Mysql, 1);
	executor.generated_id = generated_id;
	let mut widget = Widget {
		id: None,
		name: "created".to_string(),
	};

	let error = widget
		.save_with_executor(&mut executor)
		.await
		.expect_err("invalid generated ID responses should return an error");

	assert_eq!(error, expected_error);
	assert_eq!(widget.id, None);
	assert_eq!(
		queries
			.lock()
			.expect("queries mutex should not be poisoned")
			.as_slice(),
		[
			RecordedQuery {
				operation: "reset_generated_id",
				sql: "SELECT LAST_INSERT_ID(0) AS generated_id".to_string(),
				params: vec![],
			},
			RecordedQuery {
				operation: "save",
				sql: "INSERT INTO `widgets` (`name`) VALUES (?)".to_string(),
				params: vec![QueryValue::String("created".to_string())],
			},
			RecordedQuery {
				operation: "generated_id",
				sql: "SELECT CAST(LAST_INSERT_ID() AS SIGNED) AS generated_id".to_string(),
				params: vec![],
			},
		]
	);
}

#[tokio::test]
async fn mysql_save_with_an_existing_primary_key_remains_an_update() {
	let (mut executor, _operations, queries) = recording_executor(DatabaseType::Mysql, 1);
	executor.model_id = 7;
	let mut widget = Widget {
		id: Some(7),
		name: "updated".to_string(),
	};

	widget
		.save_with_executor(&mut executor)
		.await
		.expect("an existing primary key should use the update path");

	assert_eq!(widget.id, Some(7));
	assert_eq!(
		queries
			.lock()
			.expect("queries mutex should not be poisoned")
			.as_slice(),
		[
			RecordedQuery {
				operation: "save",
				sql: "UPDATE `widgets` SET `name` = ? WHERE `id` = ?".to_string(),
				params: vec![
					QueryValue::String("updated".to_string()),
					QueryValue::Int(7),
				],
			},
			RecordedQuery {
				operation: "reload",
				sql: "SELECT * FROM `widgets` WHERE `id` = ?".to_string(),
				params: vec![QueryValue::Int(7)],
			},
		]
	);
}

#[tokio::test]
async fn executor_writes_use_the_physical_primary_key_column() {
	let (mut executor, _operations, queries) = recording_executor(DatabaseType::Postgres, 1);
	let mut widget = RenamedPrimaryKeyWidget {
		id: Some(7),
		name: "updated".to_string(),
	};

	widget
		.save_with_executor(&mut executor)
		.await
		.expect("update should use the physical primary key column");
	widget
		.delete_with_executor(&mut executor)
		.await
		.expect("delete should use the physical primary key column");

	assert_eq!(
		queries
			.lock()
			.expect("queries mutex should not be poisoned")
			.as_slice(),
		[
			RecordedQuery {
				operation: "save",
				sql: r#"UPDATE "renamed_primary_key_widgets" SET "name" = $1 WHERE "widget_id" = $2 RETURNING "id", "name""#.to_string(),
				params: vec![QueryValue::String("updated".to_string()), QueryValue::Int(7)],
			},
			RecordedQuery {
				operation: "delete",
				sql: r#"DELETE FROM "renamed_primary_key_widgets" WHERE "widget_id" = $1"#.to_string(),
				params: vec![QueryValue::Int(1)],
			},
		]
	);
}

#[tokio::test]
async fn mysql_executor_reloads_through_the_physical_primary_key_column() {
	let (mut executor, _operations, queries) = recording_executor(DatabaseType::Mysql, 1);
	executor.model_id = 7;
	let mut widget = RenamedPrimaryKeyWidget {
		id: Some(7),
		name: "updated".to_string(),
	};

	widget
		.save_with_executor(&mut executor)
		.await
		.expect("MySQL update should reload through the physical primary key column");

	assert_eq!(
		queries
			.lock()
			.expect("queries mutex should not be poisoned")
			.as_slice(),
		[
			RecordedQuery {
				operation: "save",
				sql: "UPDATE `renamed_primary_key_widgets` SET `name` = ? WHERE `widget_id` = ?"
					.to_string(),
				params: vec![
					QueryValue::String("updated".to_string()),
					QueryValue::Int(7)
				],
			},
			RecordedQuery {
				operation: "reload",
				sql: "SELECT * FROM `renamed_primary_key_widgets` WHERE `widget_id` = ?"
					.to_string(),
				params: vec![QueryValue::Int(7)],
			},
		]
	);
}

#[tokio::test]
async fn insert_with_executor_inserts_models_with_assigned_primary_keys() {
	let (mut executor, _operations, queries) = recording_executor(DatabaseType::Postgres, 1);
	let mut widget = RenamedPrimaryKeyWidget {
		id: Some(7),
		name: "created".to_string(),
	};

	widget
		.insert_with_executor(&mut executor)
		.await
		.expect("assigned primary keys should not turn resource creation into an update");

	assert_eq!(
		queries
			.lock()
			.expect("queries mutex should not be poisoned")
			.as_slice(),
		[RecordedQuery {
			operation: "save",
			sql: r#"INSERT INTO "renamed_primary_key_widgets" ("id", "name") VALUES ($1, $2) RETURNING "id", "name""#.to_string(),
			params: vec![QueryValue::Int(7), QueryValue::String("created".to_string())],
		}]
	);
}

#[tokio::test(flavor = "current_thread")]
#[serial(transaction_executor_events)]
async fn executor_model_operations_dispatch_events_around_sql_in_order() {
	let (mut executor, operations, _queries) = recording_executor(DatabaseType::Postgres, 1);
	let registry = Arc::new(EventRegistry::new());
	registry.register_mapper_listener(
		Widget::table_name().to_string(),
		Arc::new(RecordingMapperEvents {
			operations: operations.clone(),
			veto: false,
		}),
	);
	let _guard = set_active_registry(registry);
	let mut widget = Widget {
		id: None,
		name: "created".to_string(),
	};

	widget
		.save_with_executor(&mut executor)
		.await
		.expect("insert should continue after before_insert");
	widget.name = "updated".to_string();
	widget
		.save_with_executor(&mut executor)
		.await
		.expect("update should continue after before_update");
	widget
		.delete_with_executor(&mut executor)
		.await
		.expect("delete should continue after before_delete");

	assert_eq!(
		operations
			.lock()
			.expect("operations mutex should not be poisoned")
			.as_slice(),
		[
			"before_insert",
			"save",
			"after_insert",
			"before_update",
			"save",
			"after_update",
			"before_delete",
			"delete",
			"after_delete",
		]
	);
}

#[tokio::test(flavor = "current_thread")]
#[serial(transaction_executor_events)]
async fn executor_model_event_veto_skips_sql_and_after_events() {
	let (mut executor, operations, queries) = recording_executor(DatabaseType::Postgres, 1);
	let registry = Arc::new(EventRegistry::new());
	registry.register_mapper_listener(
		Widget::table_name().to_string(),
		Arc::new(RecordingMapperEvents {
			operations: operations.clone(),
			veto: true,
		}),
	);
	let _guard = set_active_registry(registry);
	let mut new_widget = Widget {
		id: None,
		name: "new".to_string(),
	};
	let mut existing_widget = Widget {
		id: Some(7),
		name: "existing".to_string(),
	};

	let insert_error = new_widget
		.save_with_executor(&mut executor)
		.await
		.expect_err("before_insert veto should reject the insert");
	let update_error = existing_widget
		.save_with_executor(&mut executor)
		.await
		.expect_err("before_update veto should reject the update");
	let delete_error = existing_widget
		.delete_with_executor(&mut executor)
		.await
		.expect_err("before_delete veto should reject the delete");

	assert_eq!(
		insert_error,
		reinhardt_db::backends::error::DatabaseError::QueryError(
			"Insert operation vetoed by event listener".to_string()
		)
	);
	assert_eq!(
		update_error,
		reinhardt_db::backends::error::DatabaseError::QueryError(
			"Update operation vetoed by event listener".to_string()
		)
	);
	assert_eq!(
		delete_error,
		reinhardt_db::backends::error::DatabaseError::QueryError(
			"Delete operation vetoed by event listener".to_string()
		)
	);
	assert_eq!(
		operations
			.lock()
			.expect("operations mutex should not be poisoned")
			.as_slice(),
		["before_insert", "before_update", "before_delete"]
	);
	assert_eq!(
		queries
			.lock()
			.expect("queries mutex should not be poisoned")
			.as_slice(),
		[]
	);
}
