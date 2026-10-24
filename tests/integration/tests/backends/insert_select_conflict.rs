//! Fluent conflict clauses survive conversion from VALUES to SELECT sources.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use reinhardt_db::backends::query_builder::OnConflictClause;
use reinhardt_db::backends::types::TransactionExecutor;
use reinhardt_db::backends::{
	DatabaseBackend, DatabaseError, DatabaseType, InsertBuilder, QueryResult, QueryValue, Result,
	Row,
};
use reinhardt_query::prelude::{Expr, Query};
use rstest::rstest;

struct CountingBackend {
	db_type: DatabaseType,
	query_calls: AtomicUsize,
}

#[async_trait]
impl DatabaseBackend for CountingBackend {
	fn database_type(&self) -> DatabaseType {
		self.db_type
	}

	fn placeholder(&self, index: usize) -> String {
		match self.db_type {
			DatabaseType::Postgres => format!("${index}"),
			DatabaseType::Mysql | DatabaseType::Sqlite => "?".into(),
		}
	}

	fn supports_returning(&self) -> bool {
		self.db_type != DatabaseType::Mysql
	}

	fn supports_on_conflict(&self) -> bool {
		true
	}

	async fn execute(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<QueryResult> {
		self.query_calls.fetch_add(1, Ordering::Relaxed);
		Ok(QueryResult {
			rows_affected: 1,
			last_insert_id: None,
		})
	}

	async fn fetch_one(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<Row> {
		self.query_calls.fetch_add(1, Ordering::Relaxed);
		Ok(Row::new())
	}

	async fn fetch_all(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<Vec<Row>> {
		self.query_calls.fetch_add(1, Ordering::Relaxed);
		Ok(Vec::new())
	}

	async fn fetch_optional(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<Option<Row>> {
		self.query_calls.fetch_add(1, Ordering::Relaxed);
		Ok(None)
	}

	async fn begin(&self) -> Result<Box<dyn TransactionExecutor>> {
		self.query_calls.fetch_add(1, Ordering::Relaxed);
		Err(DatabaseError::new(
			reinhardt_core::exception::DatabaseErrorKind::Unsupported,
			"transactions are excluded from this fixture",
		)
		.into())
	}

	fn as_any(&self) -> &dyn std::any::Any {
		self
	}
}

#[rstest]
#[case::named_update(
	OnConflictClause::constraint("ignored_constraint").do_update(vec!["name"]),
	"MySQL does not support named conflict targets"
)]
#[case::named_ignore(
	OnConflictClause::constraint("ignored_constraint").do_nothing(),
	"MySQL does not support named conflict targets"
)]
#[case::named_conditional_update(
	OnConflictClause::constraint("ignored_constraint").do_update(vec!["name"]).where_clause("1 = 0"),
	"MySQL does not support named conflict targets"
)]
#[case::conditional_column_update(
	OnConflictClause::columns(vec!["id"]).do_update(vec!["name"]).where_clause("1 = 0"),
	"MySQL does not support conditional ON DUPLICATE KEY UPDATE"
)]
#[case::conditional_any_update(
	OnConflictClause::any().do_update(vec!["name"]).where_clause("1 = 0"),
	"MySQL does not support conditional ON DUPLICATE KEY UPDATE"
)]
#[tokio::test]
async fn unsupported_inherited_mysql_conflict_never_calls_backend(
	#[case] clause: OnConflictClause,
	#[case] message: &str,
) {
	// Arrange: a valid SELECT makes loss of the clause observable as a backend call.
	let backend = Arc::new(CountingBackend {
		db_type: DatabaseType::Mysql,
		query_calls: AtomicUsize::new(0),
	});
	let source = Query::select()
		.expr(Expr::val(1_i64))
		.expr(Expr::val("replacement"))
		.to_owned();
	let builder = InsertBuilder::new(backend.clone(), "options")
		.on_conflict_do_nothing(None)
		.on_conflict(clause)
		.from_select(vec!["id", "name"], source);

	// Act: execution uses a fallible path; the existing tuple-returning build fails loudly.
	let execute_error = builder.execute().await.unwrap_err();
	let fetch_error = builder.fetch_one().await.unwrap_err();
	let build_panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| builder.build()))
		.expect_err("infallible build must not discard an unsupported clause");

	// Assert
	let expected = DatabaseError::new(
		reinhardt_core::exception::DatabaseErrorKind::Unsupported,
		message,
	);
	assert_eq!(execute_error.database_kind(), Some(expected.kind()));
	assert_eq!(
		execute_error.database_error().unwrap().message(),
		expected.message()
	);
	assert_eq!(fetch_error.database_kind(), Some(expected.kind()));
	assert_eq!(
		fetch_error.database_error().unwrap().message(),
		expected.message()
	);
	let panic_message = build_panic.downcast_ref::<String>().unwrap();
	assert!(panic_message.contains("invalid INSERT SELECT conflict configuration"));
	assert!(panic_message.contains(message));
	assert_eq!(backend.query_calls.load(Ordering::Relaxed), 0);
}

#[rstest]
#[case::mysql_update(
	DatabaseType::Mysql,
	OnConflictClause::columns(vec!["id"]).do_update(vec!["name"]),
	"INSERT INTO `options` (`id`, `name`) SELECT 1, 'replacement' ON DUPLICATE KEY UPDATE `name` = VALUES(`name`)"
)]
#[case::mysql_ignore(
	DatabaseType::Mysql,
	OnConflictClause::any().do_nothing(),
	"INSERT IGNORE INTO `options` (`id`, `name`) SELECT 1, 'replacement'"
)]
#[case::postgres_named_condition(
	DatabaseType::Postgres,
	OnConflictClause::constraint("options_pkey").do_update(vec!["name"]).where_clause("1 = 0"),
	"INSERT INTO \"options\" (\"id\", \"name\") SELECT 1, 'replacement' ON CONFLICT ON CONSTRAINT \"options_pkey\" DO UPDATE SET \"name\" = EXCLUDED.\"name\" WHERE 1 = 0 RETURNING \"id\""
)]
#[case::postgres_ignore(
	DatabaseType::Postgres,
	OnConflictClause::columns(vec!["id"]).do_nothing(),
	"INSERT INTO \"options\" (\"id\", \"name\") SELECT 1, 'replacement' ON CONFLICT (\"id\") DO NOTHING RETURNING \"id\""
)]
#[case::sqlite_condition(
	DatabaseType::Sqlite,
	OnConflictClause::columns(vec!["id"]).do_update(vec!["name"]).where_clause("1 = 0"),
	"INSERT INTO \"options\" (\"id\", \"name\") SELECT 1, 'replacement' ON CONFLICT (\"id\") DO UPDATE SET \"name\" = EXCLUDED.\"name\" WHERE 1 = 0 RETURNING \"id\""
)]
#[case::sqlite_ignore(
	DatabaseType::Sqlite,
	OnConflictClause::any().do_nothing(),
	"INSERT INTO \"options\" (\"id\", \"name\") SELECT 1, 'replacement' ON CONFLICT DO NOTHING RETURNING \"id\""
)]
#[tokio::test]
async fn supported_inherited_conflicts_preserve_sql_and_precedence(
	#[case] db_type: DatabaseType,
	#[case] clause: OnConflictClause,
	#[case] expected_sql: &str,
	#[values(false, true)] legacy_update: bool,
) {
	// Arrange: an inherited fluent clause retains precedence over later legacy settings.
	let backend = Arc::new(CountingBackend {
		db_type,
		query_calls: AtomicUsize::new(0),
	});
	let source = Query::select()
		.expr(Expr::val(1_i64))
		.expr(Expr::val("replacement"))
		.to_owned();
	let builder = InsertBuilder::new(backend.clone(), "options")
		.returning(vec!["id"])
		.on_conflict(clause)
		.from_select(vec!["id", "name"], source);
	let builder = if legacy_update {
		builder.on_conflict_do_update(Some(vec!["name".into()]), vec!["id".into()])
	} else {
		builder.on_conflict_do_nothing(None)
	};

	// Act
	let (sql, params) = builder.build();
	let result = builder.execute().await.unwrap();
	builder.fetch_one().await.unwrap();

	// Assert
	assert_eq!(sql, expected_sql);
	assert!(params.is_empty());
	assert_eq!(result.rows_affected, 1);
	assert_eq!(backend.query_calls.load(Ordering::Relaxed), 2);
}
