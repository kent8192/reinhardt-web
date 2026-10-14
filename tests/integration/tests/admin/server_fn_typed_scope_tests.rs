//! Native PostgreSQL persistence and authorization for scalar creation scopes.

use super::server_fn_helpers::{
	AdminDatabaseDepends, AdminSiteDepends, TEST_CSRF_TOKEN, make_auth_user, make_staff_request,
};
use reinhardt_admin::adapters::{ImportFormat, MutationRequest};
use reinhardt_admin::core::{
	AdminDatabase, AdminError, AdminRecord, AdminSite, AdminUser, ModelAdmin,
};
use reinhardt_admin::server::{create_record, import_data};
use reinhardt_db::backends::connection::DatabaseConnection as BackendsConnection;
use reinhardt_db::backends::dialect::PostgresBackend;
use reinhardt_db::orm::connection::{DatabaseBackend, DatabaseConnection};
use reinhardt_db::orm::{Filter, FilterCondition, FilterOperator, FilterValue};
use reinhardt_di::Depends;
use reinhardt_query::prelude::{
	Alias, ColumnDef, Expr, ExprTrait, Order, PostgresQueryBuilder, Query, QueryStatementBuilder,
};
use reinhardt_test::fixtures::shared_postgres::shared_db_pool;
use rstest::rstest;
use serde_json::{Value, json};
use serial_test::serial;
use sqlx::{Connection, Executor};
use std::{collections::HashMap, sync::Arc};

const TABLE: &str = "admin_typed_creation_scopes";

struct ScalarCase {
	value: FilterValue,
	matching: Value,
	mismatching: Value,
	persisted: &'static str,
	column: ColumnDef,
}

fn scalar_case(kind: &str) -> ScalarCase {
	let column = ColumnDef::new(Alias::new("scope")).not_null(true);
	let (value, matching, mismatching, persisted, column) = match kind {
		"float" => (
			FilterValue::Float(1.0),
			json!(1),
			json!(2),
			"1",
			column.double(),
		),
		"timestamp" => (
			FilterValue::Timestamp("2026-10-12T00:30:00Z".parse().unwrap()),
			json!("2026-10-12T09:30:00+09:00"),
			json!("2026-10-13T00:30:00Z"),
			"2026-10-12 00:30:00+00",
			column.timestamp_with_time_zone(),
		),
		"date" => (
			FilterValue::Date("2026-10-12".parse().unwrap()),
			json!("2026-10-12"),
			json!("2026-10-13"),
			"2026-10-12",
			column.date(),
		),
		"time" => (
			FilterValue::Time("09:30:00.125".parse().unwrap()),
			json!("09:30:00.125000"),
			json!("09:30:00.126"),
			"09:30:00.125",
			column.time(),
		),
		"naive_datetime" => (
			FilterValue::NaiveDateTime("2026-10-12T09:30:00.125".parse().unwrap()),
			json!("2026-10-12T09:30:00.125000"),
			json!("2026-10-13T09:30:00.125"),
			"2026-10-12 09:30:00.125",
			column.timestamp(),
		),
		"decimal" => (
			FilterValue::Decimal("9007199254740993.123456".parse().unwrap()),
			json!("9007199254740993.1234560"),
			json!("9007199254740993.123457"),
			"9007199254740993.123456",
			column.decimal(28, 6),
		),
		_ => panic!("Unknown scalar case: {kind}"),
	};
	ScalarCase {
		value,
		matching,
		mismatching,
		persisted,
		column,
	}
}

struct ScopedAdmin(FilterValue);

#[async_trait::async_trait]
impl ModelAdmin for ScopedAdmin {
	fn model_name(&self) -> &str {
		"TypedScope"
	}
	fn table_name(&self) -> &str {
		TABLE
	}
	fn fields(&self) -> Option<Vec<&str>> {
		Some(vec!["id", "name", "scope"])
	}
	async fn has_add_permission(&self, _user: &dyn AdminUser) -> bool {
		true
	}
	fn object_filters(&self, _user: &dyn AdminUser) -> Option<Vec<Filter>> {
		Some(vec![Filter::new(
			"scope",
			FilterOperator::Eq,
			self.0.clone(),
		)])
	}
}

/// Own the test table even when an assertion or asynchronous setup panics.
struct ScopedTableGuard(String);

impl Drop for ScopedTableGuard {
	fn drop(&mut self) {
		let url = self.0.clone();
		// Use a fresh connection/runtime: the test runtime may already be unwinding.
		let cleanup = std::thread::spawn(move || {
			tokio::runtime::Builder::new_current_thread()
				.enable_all()
				.build()
				.expect("cleanup runtime")
				.block_on(async {
					let mut connection = sqlx::PgConnection::connect(&url).await?;
					let sql = Query::drop_table()
						.table(Alias::new(TABLE))
						.if_exists()
						.to_string(PostgresQueryBuilder);
					connection.execute(sql.as_str()).await?;
					connection.close().await
				})
		})
		.join();
		if std::thread::panicking() {
			if !matches!(cleanup, Ok(Ok(()))) {
				eprintln!("Scoped table cleanup failed: {cleanup:?}");
			}
		} else {
			cleanup
				.expect("cleanup thread")
				.expect("scoped table cleanup");
		}
	}
}

async fn context(
	pool: &sqlx::PgPool,
	case: &ScalarCase,
) -> (AdminSiteDepends, AdminDatabaseDepends) {
	let sql = Query::create_table()
		.table(Alias::new(TABLE))
		.col(
			ColumnDef::new(Alias::new("id"))
				.integer()
				.not_null(true)
				.auto_increment(true)
				.primary_key(true),
		)
		.col(
			ColumnDef::new(Alias::new("name"))
				.string_len(255)
				.not_null(true),
		)
		.col(case.column.clone())
		.to_string(PostgresQueryBuilder);
	pool.execute(sql.as_str())
		.await
		.expect("create native scalar table");
	let backend = Arc::new(PostgresBackend::new(pool.clone()));
	let db = AdminDatabase::new(DatabaseConnection::new(
		DatabaseBackend::Postgres,
		BackendsConnection::new(backend),
	));
	let site = AdminSite::new("typed_scopes");
	site.register("TypedScope", ScopedAdmin(case.value.clone()))
		.unwrap();
	(Depends::from_value(site), Depends::from_value(db))
}

async fn assert_persisted(pool: &sqlx::PgPool, db: &AdminDatabase, case: &ScalarCase) {
	let sql = Query::select()
		.from(Alias::new(TABLE))
		.expr(Expr::col(Alias::new("scope")).cast_as(Alias::new("text")))
		.order_by(Alias::new("id"), Order::Asc)
		.to_string(PostgresQueryBuilder);
	let stored: Vec<String> = sqlx::query_scalar(&sql).fetch_all(pool).await.unwrap();
	assert_eq!(
		stored,
		vec![case.persisted, case.persisted],
		"both authorized records retain their native value"
	);
	for id in ["1", "2"] {
		let found = db
			.get_with_filters::<AdminRecord>(
				TABLE,
				"id",
				id,
				vec![Filter::new("scope", FilterOperator::Eq, case.value.clone())],
			)
			.await
			.unwrap();
		assert!(
			found.is_some(),
			"created record must be visible in its typed scope"
		);
		let excluded = db
			.get_with_filters::<AdminRecord>(
				TABLE,
				"id",
				id,
				vec![Filter::new("scope", FilterOperator::Ne, case.value.clone())],
			)
			.await
			.unwrap();
		assert!(
			excluded.is_none(),
			"an incompatible scope must hide the record"
		);
		let excluded = db
			.get_with_filters::<AdminRecord>(
				TABLE,
				"id",
				id,
				vec![Filter::new(
					"scope",
					FilterOperator::Eq,
					mismatching_value(case),
				)],
			)
			.await
			.unwrap();
		assert!(
			excluded.is_none(),
			"distinct scalar values must not match through lossy conversion"
		);
	}
}

#[rstest]
#[tokio::test]
#[serial(admin_typed_scopes)]
async fn create_record_preserves_native_scalar_scope(
	#[values("float", "timestamp", "date", "time", "naive_datetime", "decimal")] kind: &str,
	#[future] shared_db_pool: (sqlx::PgPool, String),
) {
	// Arrange
	let (pool, url) = shared_db_pool.await;
	let _table = ScopedTableGuard(url);
	let case = scalar_case(kind);
	let (site, db) = context(&pool, &case).await;
	for (scope, authorized) in [
		(None, true),
		(Some(case.matching.clone()), true),
		(Some(case.mismatching.clone()), false),
	] {
		let mut data = HashMap::from([("name".to_owned(), json!("Scoped record"))]);
		if let Some(scope) = scope {
			data.insert("scope".to_owned(), scope);
		}
		// Act
		let result = create_record(
			"TypedScope".to_owned(),
			MutationRequest {
				csrf_token: TEST_CSRF_TOKEN.to_owned(),
				data,
			},
			site.clone(),
			db.clone(),
			make_staff_request(),
			make_auth_user(),
		)
		.await;
		// Assert
		if authorized {
			assert!(result.unwrap().success);
		} else {
			assert_eq!(
				result.unwrap_err().to_string(),
				"Server error (403): Object permission denied"
			);
		}
	}
	assert_persisted(&pool, &db, &case).await;
}

#[rstest]
#[tokio::test]
#[serial(admin_typed_scopes)]
async fn scoped_get_propagates_database_errors(#[future] shared_db_pool: (sqlx::PgPool, String)) {
	// Arrange
	let (pool, url) = shared_db_pool.await;
	let _table = ScopedTableGuard(url);
	let case = scalar_case("float");
	let (_site, db) = context(&pool, &case).await;
	let invalid_scope = vec![Filter::new(
		"missing_scope_column",
		FilterOperator::Eq,
		case.value.clone(),
	)];

	// Act
	let result = db
		.get_with_filters::<AdminRecord>(TABLE, "id", "1", invalid_scope)
		.await;

	// Assert
	assert!(
		matches!(&result, Err(AdminError::DatabaseError(message)) if message.contains("missing_scope_column")),
		"a failing scoped query must not be reported as a missing record: {result:?}"
	);
}

fn import_bytes(format: ImportFormat, scope: Option<Value>) -> Vec<u8> {
	if matches!(format, ImportFormat::JSON) {
		let mut record = json!({"name": "Imported record"});
		if let Some(scope) = scope {
			record["scope"] = scope;
		}
		return serde_json::to_vec(&vec![record]).unwrap();
	}
	let delimiter = if matches!(format, ImportFormat::CSV) {
		','
	} else {
		'\t'
	};
	if let Some(scope) = scope {
		let cell = match scope {
			Value::String(s) => s,
			other => other.to_string(),
		};
		format!("name{delimiter}scope\nImported record{delimiter}{cell}\n").into_bytes()
	} else {
		b"name\nImported record\n".to_vec()
	}
}

#[rstest]
#[tokio::test]
#[serial(admin_typed_scopes)]
async fn import_preserves_native_scalar_scope(
	#[values("float", "timestamp", "date", "time", "naive_datetime", "decimal")] kind: &str,
	#[values(ImportFormat::JSON, ImportFormat::CSV, ImportFormat::TSV)] format: ImportFormat,
	#[future] shared_db_pool: (sqlx::PgPool, String),
) {
	// Arrange
	let (pool, url) = shared_db_pool.await;
	let _table = ScopedTableGuard(url);
	let case = scalar_case(kind);
	let (site, db) = context(&pool, &case).await;
	for (scope, authorized) in [
		(None, true),
		(Some(case.matching.clone()), true),
		(Some(case.mismatching.clone()), false),
	] {
		let bytes = import_bytes(format.clone(), scope);
		// Act
		let result = import_data(
			"TypedScope".to_owned(),
			format.clone(),
			bytes,
			site.clone(),
			db.clone(),
			make_staff_request(),
			make_auth_user(),
		)
		.await
		.unwrap();
		// Assert
		assert_eq!(result.success, authorized);
		assert_eq!(result.imported, u64::from(authorized));
		assert_eq!(result.failed, u64::from(!authorized));
	}
	assert_persisted(&pool, &db, &case).await;
}

fn mismatching_value(case: &ScalarCase) -> FilterValue {
	let text = case.mismatching.as_str().unwrap_or_default();
	match &case.value {
		FilterValue::Float(_) => FilterValue::Float(case.mismatching.as_f64().unwrap()),
		FilterValue::Timestamp(_) => FilterValue::Timestamp(text.parse().unwrap()),
		FilterValue::Date(_) => FilterValue::Date(text.parse().unwrap()),
		FilterValue::Time(_) => FilterValue::Time(text.parse().unwrap()),
		FilterValue::NaiveDateTime(_) => FilterValue::NaiveDateTime(text.parse().unwrap()),
		FilterValue::Decimal(_) => FilterValue::Decimal(text.parse().unwrap()),
		_ => panic!("unsupported test scalar"),
	}
}

#[rstest]
#[tokio::test]
#[serial(admin_typed_scopes)]
async fn scoped_consumers_preserve_native_scalar_predicates(
	#[values("float", "timestamp", "date", "time", "naive_datetime", "decimal")] kind: &str,
	#[values(
		"list",
		"condition_list",
		"list_and_count",
		"count",
		"condition_count",
		"update",
		"delete",
		"bulk_delete"
	)]
	operation: &str,
	#[future] shared_db_pool: (sqlx::PgPool, String),
) {
	// Arrange: create two native values through the real authorized server function.
	let (pool, url) = shared_db_pool.await;
	let _table = ScopedTableGuard(url);
	let case = scalar_case(kind);
	let (site, db) = context(&pool, &case).await;
	for name in ["Original 1", "Original 2"] {
		let request = MutationRequest {
			csrf_token: TEST_CSRF_TOKEN.to_owned(),
			data: HashMap::from([("name".to_owned(), json!(name))]),
		};
		assert!(
			create_record(
				"TypedScope".to_owned(),
				request,
				site.clone(),
				db.clone(),
				make_staff_request(),
				make_auth_user()
			)
			.await
			.unwrap()
			.success
		);
	}
	let matching = vec![Filter::new("scope", FilterOperator::Eq, case.value.clone())];
	let denied = vec![Filter::new(
		"scope",
		FilterOperator::Eq,
		mismatching_value(&case),
	)];
	let matching_condition = FilterCondition::Single(matching[0].clone());
	let denied_condition = FilterCondition::Single(denied[0].clone());
	let mut expected_names = vec!["Original 1", "Original 2"];

	// Act / Assert: equal native values remain visible; distinct values cannot access them.
	match operation {
		"list" => {
			assert_eq!(
				db.list::<AdminRecord>(TABLE, matching, 0, 10)
					.await
					.unwrap()
					.len(),
				2
			);
			assert!(
				db.list::<AdminRecord>(TABLE, denied, 0, 10)
					.await
					.unwrap()
					.is_empty()
			);
		}
		"condition_list" => {
			assert_eq!(
				db.list_with_condition::<AdminRecord>(
					TABLE,
					Some(&matching_condition),
					vec![],
					Some("id"),
					0,
					10
				)
				.await
				.unwrap()
				.len(),
				2
			);
			assert!(
				db.list_with_condition::<AdminRecord>(
					TABLE,
					Some(&denied_condition),
					vec![],
					Some("id"),
					0,
					10
				)
				.await
				.unwrap()
				.is_empty()
			);
		}
		"list_and_count" => {
			let (rows, count) = db
				.list_with_condition_and_count::<AdminRecord>(
					TABLE,
					None,
					matching.clone(),
					Some("id"),
					0,
					1,
				)
				.await
				.unwrap();
			assert_eq!((rows.len(), count), (1, 2));
			let (rows, count) = db
				.list_with_condition_and_count::<AdminRecord>(
					TABLE,
					None,
					matching,
					Some("id"),
					100,
					1,
				)
				.await
				.unwrap();
			assert_eq!(
				(rows.len(), count),
				(0, 2),
				"empty-page count fallback retains the scope"
			);
			let (rows, count) = db
				.list_with_condition_and_count::<AdminRecord>(TABLE, None, denied, Some("id"), 0, 1)
				.await
				.unwrap();
			assert_eq!((rows.len(), count), (0, 0));
		}
		"count" => {
			assert_eq!(db.count::<AdminRecord>(TABLE, matching).await.unwrap(), 2);
			assert_eq!(db.count::<AdminRecord>(TABLE, denied).await.unwrap(), 0);
		}
		"condition_count" => {
			assert_eq!(
				db.count_with_condition::<AdminRecord>(TABLE, Some(&matching_condition), vec![])
					.await
					.unwrap(),
				2
			);
			assert_eq!(
				db.count_with_condition::<AdminRecord>(TABLE, Some(&denied_condition), vec![])
					.await
					.unwrap(),
				0
			);
		}
		"update" => {
			let data = HashMap::from([("name".to_owned(), json!("Renamed"))]);
			assert_eq!(
				db.update_with_filters::<AdminRecord>(TABLE, "id", "1", data.clone(), denied)
					.await
					.unwrap(),
				0
			);
			assert_eq!(
				db.update_with_filters::<AdminRecord>(TABLE, "id", "1", data, matching)
					.await
					.unwrap(),
				1
			);
			expected_names[0] = "Renamed";
		}
		"delete" => {
			assert_eq!(
				db.delete_with_filters::<AdminRecord>(TABLE, "id", "1", denied)
					.await
					.unwrap(),
				0
			);
			assert_eq!(
				db.delete_with_filters::<AdminRecord>(TABLE, "id", "1", matching)
					.await
					.unwrap(),
				1
			);
			expected_names.remove(0);
		}
		"bulk_delete" => {
			let ids = vec!["1".to_owned(), "2".to_owned()];
			assert_eq!(
				db.bulk_delete_with_filters::<AdminRecord>(TABLE, "id", ids.clone(), denied)
					.await
					.unwrap(),
				0
			);
			assert_eq!(
				db.bulk_delete_with_filters::<AdminRecord>(TABLE, "id", ids, matching)
					.await
					.unwrap(),
				2
			);
			expected_names.clear();
		}
		_ => panic!("unknown scoped operation"),
	}
	// Inspect persistence independently of the admin predicates under test.
	let sql = Query::select()
		.from(Alias::new(TABLE))
		.column(Alias::new("name"))
		.order_by(Alias::new("id"), Order::Asc)
		.to_string(PostgresQueryBuilder);
	let names: Vec<String> = sqlx::query_scalar(&sql).fetch_all(&pool).await.unwrap();
	assert_eq!(names, expected_names);
}
