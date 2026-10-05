//! Tests for backend query builder types and supporting structures
//!
//! Covers OnConflictClause, ConflictTarget, DatabaseType, IsolationLevel,
//! QueryValue, DatabaseError, QueryCacheConfig, and builder construction.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use rstest::rstest;

use reinhardt_db::backends::query_builder::{
	ConflictTarget, DeleteBuilder, OnConflictClause, OnConflictClauseAction,
};
use reinhardt_db::backends::types::{Savepoint, TransactionExecutor};
use reinhardt_db::backends::{
	DatabaseBackend, DatabaseError, DatabaseType, InsertBuilder, IsolationLevel, QueryCache,
	QueryCacheConfig, QueryResult, QueryValue, Row, SelectBuilder, UpdateBuilder,
};

// ==================== Mock backend for testing ====================

/// Mock backend that does not require a real database connection.
/// Used to test query builder SQL generation.
struct MockBackend {
	db_type: DatabaseType,
}

impl MockBackend {
	fn new(db_type: DatabaseType) -> Arc<Self> {
		Arc::new(Self { db_type })
	}
}

#[async_trait]
impl DatabaseBackend for MockBackend {
	fn database_type(&self) -> DatabaseType {
		self.db_type
	}

	fn placeholder(&self, index: usize) -> String {
		match self.db_type {
			DatabaseType::Postgres => format!("${}", index),
			DatabaseType::Mysql | DatabaseType::Sqlite => "?".to_string(),
		}
	}

	fn supports_returning(&self) -> bool {
		matches!(self.db_type, DatabaseType::Postgres | DatabaseType::Sqlite)
	}

	fn supports_on_conflict(&self) -> bool {
		true
	}

	async fn execute(
		&self,
		_sql: &str,
		_params: Vec<QueryValue>,
	) -> reinhardt_db::backends::Result<QueryResult> {
		Ok(QueryResult { rows_affected: 0 })
	}

	async fn fetch_one(
		&self,
		_sql: &str,
		_params: Vec<QueryValue>,
	) -> reinhardt_db::backends::Result<Row> {
		Ok(Row::new())
	}

	async fn fetch_all(
		&self,
		_sql: &str,
		_params: Vec<QueryValue>,
	) -> reinhardt_db::backends::Result<Vec<Row>> {
		Ok(Vec::new())
	}

	async fn fetch_optional(
		&self,
		_sql: &str,
		_params: Vec<QueryValue>,
	) -> reinhardt_db::backends::Result<Option<Row>> {
		Ok(None)
	}

	async fn begin(&self) -> reinhardt_db::backends::Result<Box<dyn TransactionExecutor>> {
		Err(DatabaseError::NotSupported(
			"Mock backend does not support transactions".to_string(),
		))
	}

	fn as_any(&self) -> &dyn std::any::Any {
		self
	}
}

// ==================== OnConflictClause tests ====================

#[rstest]
fn test_on_conflict_clause_columns_creates_column_target() {
	// Arrange

	// Act
	let clause = OnConflictClause::columns(vec!["email", "tenant_id"]);

	// Assert
	let debug = format!("{:?}", clause);
	assert!(debug.contains("Columns"));
	assert!(debug.contains("email"));
	assert!(debug.contains("tenant_id"));
}

#[rstest]
fn test_on_conflict_clause_constraint_creates_constraint_target() {
	// Arrange

	// Act
	let clause = OnConflictClause::constraint("users_email_key");

	// Assert
	let debug = format!("{:?}", clause);
	assert!(debug.contains("Constraint"));
	assert!(debug.contains("users_email_key"));
}

#[rstest]
fn test_on_conflict_clause_any_creates_no_target() {
	// Arrange

	// Act
	let clause = OnConflictClause::any();

	// Assert
	let debug = format!("{:?}", clause);
	assert!(debug.contains("None"));
}

#[rstest]
fn test_on_conflict_clause_do_nothing_sets_action() {
	// Arrange
	let clause = OnConflictClause::columns(vec!["email"]);

	// Act
	let clause = clause.do_nothing();

	// Assert
	let debug = format!("{:?}", clause);
	assert!(debug.contains("DoNothing"));
}

#[rstest]
fn test_on_conflict_clause_do_update_sets_action() {
	// Arrange
	let clause = OnConflictClause::columns(vec!["email"]);

	// Act
	let clause = clause.do_update(vec!["name", "updated_at"]);

	// Assert
	let debug = format!("{:?}", clause);
	assert!(debug.contains("DoUpdate"));
	assert!(debug.contains("name"));
	assert!(debug.contains("updated_at"));
}

#[rstest]
fn test_on_conflict_clause_where_clause_sets_condition() {
	// Arrange

	// Act
	let clause = OnConflictClause::columns(vec!["email"])
		.do_update(vec!["name", "updated_at"])
		.where_clause("users.updated_at < EXCLUDED.updated_at");

	// Assert
	let debug = format!("{:?}", clause);
	assert!(debug.contains("users.updated_at < EXCLUDED.updated_at"));
}

#[rstest]
fn test_on_conflict_clause_chaining() {
	// Arrange

	// Act
	let clause = OnConflictClause::columns(vec!["id"])
		.do_update(vec!["data", "version"])
		.where_clause("users.version < EXCLUDED.version");

	// Assert
	let debug = format!("{:?}", clause);
	assert!(debug.contains("Columns"));
	assert!(debug.contains("DoUpdate"));
	assert!(debug.contains("data"));
	assert!(debug.contains("version"));
}

// ==================== ConflictTarget tests ====================

#[rstest]
fn test_conflict_target_columns_variant() {
	// Arrange

	// Act
	let target = ConflictTarget::Columns(vec!["email".to_string(), "tenant_id".to_string()]);

	// Assert
	match target {
		ConflictTarget::Columns(cols) => {
			assert_eq!(cols.len(), 2);
			assert_eq!(cols[0], "email");
			assert_eq!(cols[1], "tenant_id");
		}
		_ => panic!("Expected Columns variant"),
	}
}

#[rstest]
fn test_conflict_target_constraint_variant() {
	// Arrange

	// Act
	let target = ConflictTarget::Constraint("users_email_key".to_string());

	// Assert
	match target {
		ConflictTarget::Constraint(name) => {
			assert_eq!(name, "users_email_key");
		}
		_ => panic!("Expected Constraint variant"),
	}
}

#[rstest]
fn test_conflict_target_clone() {
	// Arrange
	let target = ConflictTarget::Columns(vec!["id".to_string()]);

	// Act
	let cloned = target.clone();

	// Assert
	let debug_original = format!("{:?}", target);
	let debug_cloned = format!("{:?}", cloned);
	assert_eq!(debug_original, debug_cloned);
}

// ==================== DatabaseType tests ====================

#[rstest]
fn test_database_type_postgres_supports_transactional_ddl() {
	// Arrange

	// Act

	// Assert
	assert!(DatabaseType::Postgres.supports_transactional_ddl());
}

#[rstest]
fn test_database_type_sqlite_supports_transactional_ddl() {
	// Arrange

	// Act

	// Assert
	assert!(DatabaseType::Sqlite.supports_transactional_ddl());
}

#[rstest]
fn test_database_type_mysql_does_not_support_transactional_ddl() {
	// Arrange

	// Act

	// Assert
	assert!(!DatabaseType::Mysql.supports_transactional_ddl());
}

#[rstest]
fn test_database_type_equality() {
	// Arrange

	// Act

	// Assert
	assert_eq!(DatabaseType::Postgres, DatabaseType::Postgres);
	assert_eq!(DatabaseType::Mysql, DatabaseType::Mysql);
	assert_eq!(DatabaseType::Sqlite, DatabaseType::Sqlite);
	assert_ne!(DatabaseType::Postgres, DatabaseType::Mysql);
	assert_ne!(DatabaseType::Postgres, DatabaseType::Sqlite);
	assert_ne!(DatabaseType::Mysql, DatabaseType::Sqlite);
}

#[rstest]
fn test_database_type_clone() {
	// Arrange
	let db_type = DatabaseType::Postgres;

	// Act
	let cloned = db_type;

	// Assert
	assert_eq!(db_type, cloned);
}

// ==================== IsolationLevel tests ====================

#[rstest]
fn test_isolation_level_default_is_read_committed() {
	// Arrange

	// Act
	let level = IsolationLevel::default();

	// Assert
	assert_eq!(level, IsolationLevel::ReadCommitted);
}

#[rstest]
#[case(IsolationLevel::ReadUncommitted, "READ UNCOMMITTED")]
#[case(IsolationLevel::ReadCommitted, "READ COMMITTED")]
#[case(IsolationLevel::RepeatableRead, "REPEATABLE READ")]
#[case(IsolationLevel::Serializable, "SERIALIZABLE")]
fn test_isolation_level_to_sql(#[case] level: IsolationLevel, #[case] expected: &str) {
	// Arrange

	// Act
	let sql = level.to_sql(DatabaseType::Postgres);

	// Assert
	assert_eq!(sql, expected);
}

#[rstest]
fn test_isolation_level_begin_transaction_sql_postgres() {
	// Arrange

	// Act
	let sql = IsolationLevel::Serializable.begin_transaction_sql(DatabaseType::Postgres);

	// Assert
	assert_eq!(sql, "BEGIN ISOLATION LEVEL SERIALIZABLE");
}

#[rstest]
fn test_isolation_level_begin_transaction_sql_mysql() {
	// Arrange

	// Act
	let sql = IsolationLevel::RepeatableRead.begin_transaction_sql(DatabaseType::Mysql);

	// Assert
	assert_eq!(
		sql,
		"SET TRANSACTION ISOLATION LEVEL REPEATABLE READ; START TRANSACTION"
	);
}

#[rstest]
fn test_isolation_level_begin_transaction_sql_sqlite_serializable() {
	// Arrange

	// Act
	let sql = IsolationLevel::Serializable.begin_transaction_sql(DatabaseType::Sqlite);

	// Assert
	assert_eq!(sql, "BEGIN EXCLUSIVE");
}

#[rstest]
fn test_isolation_level_begin_transaction_sql_sqlite_default() {
	// Arrange

	// Act
	let sql = IsolationLevel::ReadCommitted.begin_transaction_sql(DatabaseType::Sqlite);

	// Assert
	assert_eq!(sql, "BEGIN");
}

#[rstest]
fn test_isolation_level_all_variants_exist() {
	// Arrange

	// Act

	// Assert
	let levels = [
		IsolationLevel::ReadUncommitted,
		IsolationLevel::ReadCommitted,
		IsolationLevel::RepeatableRead,
		IsolationLevel::Serializable,
	];
	assert_eq!(levels.len(), 4);
}

// ==================== QueryValue tests ====================

#[rstest]
fn test_query_value_null() {
	// Arrange

	// Act
	let val = QueryValue::Null;

	// Assert
	let debug = format!("{:?}", val);
	assert!(debug.contains("Null"));
}

#[rstest]
fn test_query_value_bool() {
	// Arrange

	// Act
	let val_true = QueryValue::Bool(true);
	let val_false = QueryValue::Bool(false);

	// Assert
	assert_eq!(val_true, QueryValue::Bool(true));
	assert_eq!(val_false, QueryValue::Bool(false));
	assert_ne!(val_true, val_false);
}

#[rstest]
fn test_query_value_int() {
	// Arrange

	// Act
	let val = QueryValue::Int(42);

	// Assert
	assert_eq!(val, QueryValue::Int(42));
	assert_ne!(val, QueryValue::Int(0));
}

#[rstest]
fn test_query_value_float() {
	// Arrange

	// Act
	let val = QueryValue::Float(3.5);

	// Assert
	assert_eq!(val, QueryValue::Float(3.5));
}

#[rstest]
fn test_query_value_string() {
	// Arrange

	// Act
	let val = QueryValue::String("hello".to_string());

	// Assert
	assert_eq!(val, QueryValue::String("hello".to_string()));
}

#[rstest]
fn test_query_value_bytes() {
	// Arrange

	// Act
	let val = QueryValue::Bytes(vec![1, 2, 3]);

	// Assert
	assert_eq!(val, QueryValue::Bytes(vec![1, 2, 3]));
}

#[rstest]
fn test_query_value_timestamp() {
	// Arrange
	let now = chrono::Utc::now();

	// Act
	let val = QueryValue::Timestamp(now);

	// Assert
	assert_eq!(val, QueryValue::Timestamp(now));
}

#[rstest]
fn test_query_value_uuid() {
	// Arrange
	let id = uuid::Uuid::now_v7();

	// Act
	let val = QueryValue::Uuid(id);

	// Assert
	assert_eq!(val, QueryValue::Uuid(id));
}

#[rstest]
fn test_query_value_now() {
	// Arrange

	// Act
	let val = QueryValue::Now;

	// Assert
	let debug = format!("{:?}", val);
	assert!(debug.contains("Now"));
}

#[rstest]
fn test_query_value_from_str() {
	// Arrange

	// Act
	let val: QueryValue = "test".into();

	// Assert
	assert_eq!(val, QueryValue::String("test".to_string()));
}

#[rstest]
fn test_query_value_from_string() {
	// Arrange

	// Act
	let val: QueryValue = String::from("test").into();

	// Assert
	assert_eq!(val, QueryValue::String("test".to_string()));
}

#[rstest]
fn test_query_value_from_i64() {
	// Arrange

	// Act
	let val: QueryValue = 100i64.into();

	// Assert
	assert_eq!(val, QueryValue::Int(100));
}

#[rstest]
fn test_query_value_from_i32() {
	// Arrange

	// Act
	let val: QueryValue = 50i32.into();

	// Assert
	assert_eq!(val, QueryValue::Int(50));
}

#[rstest]
fn test_query_value_from_f64() {
	// Arrange

	// Act
	let val: QueryValue = 2.75f64.into();

	// Assert
	assert_eq!(val, QueryValue::Float(2.75));
}

#[rstest]
fn test_query_value_from_bool() {
	// Arrange

	// Act
	let val: QueryValue = true.into();

	// Assert
	assert_eq!(val, QueryValue::Bool(true));
}

#[rstest]
fn test_query_value_from_chrono_datetime() {
	// Arrange
	let dt = chrono::Utc::now();

	// Act
	let val: QueryValue = dt.into();

	// Assert
	assert_eq!(val, QueryValue::Timestamp(dt));
}

#[rstest]
fn test_query_value_from_uuid() {
	// Arrange
	let id = uuid::Uuid::now_v7();

	// Act
	let val: QueryValue = id.into();

	// Assert
	assert_eq!(val, QueryValue::Uuid(id));
}

// ==================== DatabaseError tests ====================

#[rstest]
fn test_database_error_unsupported_feature() {
	// Arrange

	// Act
	let err = DatabaseError::UnsupportedFeature {
		database: "MySQL".to_string(),
		feature: "transactional DDL".to_string(),
	};

	// Assert
	let msg = err.to_string();
	assert!(msg.contains("transactional DDL"));
	assert!(msg.contains("MySQL"));
}

#[rstest]
fn test_database_error_not_supported() {
	// Arrange

	// Act
	let err = DatabaseError::NotSupported("savepoints".to_string());

	// Assert
	assert!(err.to_string().contains("savepoints"));
}

#[rstest]
fn test_database_error_syntax_error() {
	// Arrange

	// Act
	let err = DatabaseError::SyntaxError("unexpected token".to_string());

	// Assert
	assert!(err.to_string().contains("unexpected token"));
}

#[rstest]
fn test_database_error_type_error() {
	// Arrange

	// Act
	let err = DatabaseError::TypeError("cannot convert".to_string());

	// Assert
	assert!(err.to_string().contains("cannot convert"));
}

#[rstest]
fn test_database_error_connection_error() {
	// Arrange

	// Act
	let err = DatabaseError::ConnectionError("timeout".to_string());

	// Assert
	assert!(err.to_string().contains("timeout"));
}

#[rstest]
fn test_database_error_query_error() {
	// Arrange

	// Act
	let err = DatabaseError::QueryError("invalid column".to_string());

	// Assert
	assert!(err.to_string().contains("invalid column"));
}

#[rstest]
fn test_database_error_serialization_error() {
	// Arrange

	// Act
	let err = DatabaseError::SerializationError("invalid json".to_string());

	// Assert
	assert!(err.to_string().contains("invalid json"));
}

#[rstest]
fn test_database_error_config_error() {
	// Arrange

	// Act
	let err = DatabaseError::ConfigError("missing url".to_string());

	// Assert
	assert!(err.to_string().contains("missing url"));
}

#[rstest]
fn test_database_error_column_not_found() {
	// Arrange

	// Act
	let err = DatabaseError::ColumnNotFound("user_id".to_string());

	// Assert
	assert!(err.to_string().contains("user_id"));
}

#[rstest]
fn test_database_error_transaction_error() {
	// Arrange

	// Act
	let err = DatabaseError::TransactionError("deadlock detected".to_string());

	// Assert
	assert!(err.to_string().contains("deadlock detected"));
}

#[rstest]
fn test_database_error_other() {
	// Arrange

	// Act
	let err = DatabaseError::Other("unknown error".to_string());

	// Assert
	assert!(err.to_string().contains("unknown error"));
}

#[rstest]
fn test_database_error_equality() {
	// Arrange
	let err1 = DatabaseError::QueryError("test".to_string());
	let err2 = DatabaseError::QueryError("test".to_string());
	let err3 = DatabaseError::QueryError("other".to_string());

	// Act

	// Assert
	assert_eq!(err1, err2);
	assert_ne!(err1, err3);
}

// ==================== QueryCacheConfig tests ====================

#[rstest]
fn test_query_cache_config_default() {
	// Arrange

	// Act
	let config = QueryCacheConfig::default();

	// Assert
	assert_eq!(config.max_size, 1000);
	assert_eq!(config.ttl, Duration::from_secs(300));
	assert!(config.cache_plans);
}

#[rstest]
fn test_query_cache_config_custom() {
	// Arrange

	// Act
	let mut config = QueryCacheConfig::default();
	config.max_size = 500;
	config.ttl = Duration::from_secs(60);
	config.cache_plans = false;

	// Assert
	assert_eq!(config.max_size, 500);
	assert_eq!(config.ttl, Duration::from_secs(60));
	assert!(!config.cache_plans);
}

#[rstest]
fn test_query_cache_set_and_get() {
	// Arrange
	let cache = QueryCache::new(QueryCacheConfig::default());
	let sql = "SELECT * FROM users WHERE id = $1".to_string();
	let params_hash = 12345u64;

	// Act
	cache.set(sql.clone(), params_hash, Some(vec![1, 2, 3]));
	let result = cache.get(&sql, params_hash);

	// Assert
	assert!(result.is_some());
	let cached = result.unwrap();
	assert_eq!(cached.sql, sql);
	assert_eq!(cached.params_hash, params_hash);
	assert_eq!(cached.result, Some(vec![1, 2, 3]));
}

#[rstest]
fn test_query_cache_miss_on_different_params() {
	// Arrange
	let cache = QueryCache::new(QueryCacheConfig::default());
	let sql = "SELECT * FROM users".to_string();

	// Act
	cache.set(sql.clone(), 100, None);

	// Assert
	assert!(cache.get(&sql, 200).is_none());
}

#[rstest]
fn test_query_cache_clear() {
	// Arrange
	let cache = QueryCache::new(QueryCacheConfig::default());
	cache.set("query1".to_string(), 1, None);
	cache.set("query2".to_string(), 2, None);

	// Act
	cache.clear();

	// Assert
	assert!(cache.get("query1", 1).is_none());
	assert!(cache.get("query2", 2).is_none());
}

#[rstest]
fn test_query_cache_stats() {
	// Arrange
	let cache = QueryCache::new(QueryCacheConfig::default());
	cache.set("q1".to_string(), 1, None);
	cache.set("q2".to_string(), 2, None);
	cache.record_hit("q1");
	cache.record_hit("q1");
	cache.record_hit("q2");

	// Act
	let stats = cache.stats();

	// Assert
	assert_eq!(stats.total_entries, 2);
	assert_eq!(stats.total_hits, 3);
}

#[rstest]
fn test_query_cache_eviction_on_max_size() {
	// Arrange
	let mut config = QueryCacheConfig::default();
	config.max_size = 2;
	config.ttl = Duration::from_secs(300);
	config.cache_plans = true;
	let cache = QueryCache::new(config);

	// Act
	cache.set("q1".to_string(), 1, None);
	cache.set("q2".to_string(), 2, None);
	cache.set("q3".to_string(), 3, None);

	// Assert: cache should have at most 2 entries
	let stats = cache.stats();
	assert_eq!(stats.total_entries, 2);
}

// ==================== InsertBuilder SQL generation tests ====================

#[rstest]
fn test_insert_builder_postgres_basic() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Postgres);

	// Act
	let (sql, params) = InsertBuilder::new(backend, "users")
		.value("name", QueryValue::String("Alice".to_string()))
		.value("age", QueryValue::Int(30))
		.build()
		.unwrap();

	// Assert
	assert!(sql.contains("INSERT INTO"));
	assert!(sql.contains("users"));
	assert_eq!(params.len(), 2);
}

#[rstest]
fn test_insert_builder_mysql_basic() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Mysql);

	// Act
	let (sql, params) = InsertBuilder::new(backend, "users")
		.value("name", QueryValue::String("Bob".to_string()))
		.build()
		.unwrap();

	// Assert
	assert!(sql.contains("INSERT INTO"));
	assert_eq!(params.len(), 1);
}

#[rstest]
fn test_insert_builder_sqlite_basic() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Sqlite);

	// Act
	let (sql, params) = InsertBuilder::new(backend, "users")
		.value("email", QueryValue::String("test@example.com".to_string()))
		.build()
		.unwrap();

	// Assert
	assert!(sql.contains("INSERT INTO"));
	assert_eq!(params.len(), 1);
}

#[rstest]
#[case::postgres(
	DatabaseType::Postgres,
	r#"INSERT INTO "insert_probe" ("created", "touched") SELECT CURRENT_TIMESTAMP, CURRENT_TIMESTAMP"#
)]
#[case::mysql(
	DatabaseType::Mysql,
	"INSERT INTO `insert_probe` (`created`, `touched`) SELECT CURRENT_TIMESTAMP, CURRENT_TIMESTAMP"
)]
#[case::sqlite(
	DatabaseType::Sqlite,
	r#"INSERT INTO "insert_probe" ("created", "touched") SELECT CURRENT_TIMESTAMP, CURRENT_TIMESTAMP"#
)]
fn test_insert_builder_now_without_bindings(
	#[case] db_type: DatabaseType,
	#[case] expected_sql: &str,
) {
	// Arrange
	let backend = MockBackend::new(db_type);
	let builder = InsertBuilder::new(backend, "insert_probe")
		.value("created", QueryValue::Now)
		.value("touched", QueryValue::Now);

	// Act
	let (sql, params) = builder
		.build()
		.expect("current-time expressions must build");

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(params, Vec::<QueryValue>::new());
}

#[rstest]
#[case::postgres(
	DatabaseType::Postgres,
	r#"INSERT INTO "insert_probe" ("created", "nullable", "id", "touched", "payload", "updated") SELECT CURRENT_TIMESTAMP, NULL, $1, CURRENT_TIMESTAMP, $2, CURRENT_TIMESTAMP"#
)]
#[case::mysql(
	DatabaseType::Mysql,
	"INSERT INTO `insert_probe` (`created`, `nullable`, `id`, `touched`, `payload`, `updated`) SELECT CURRENT_TIMESTAMP, NULL, ?, CURRENT_TIMESTAMP, ?, CURRENT_TIMESTAMP"
)]
#[case::sqlite(
	DatabaseType::Sqlite,
	r#"INSERT INTO "insert_probe" ("created", "nullable", "id", "touched", "payload", "updated") SELECT CURRENT_TIMESTAMP, NULL, ?, CURRENT_TIMESTAMP, ?, CURRENT_TIMESTAMP"#
)]
fn test_insert_builder_now_preserves_binding_order(
	#[case] db_type: DatabaseType,
	#[case] expected_sql: &str,
) {
	// Arrange
	let backend = MockBackend::new(db_type);
	let payload = "literal CURRENT_TIMESTAMP '__REINHARDT_NOW__' $9 ?";
	let builder = InsertBuilder::new(backend, "insert_probe")
		.value("created", QueryValue::Now)
		.value("nullable", QueryValue::Null)
		.value("id", 2_i64)
		.value("touched", QueryValue::Now)
		.value("payload", payload)
		.value("updated", QueryValue::Now);

	// Act
	let (sql, params) = builder
		.build()
		.expect("mixed current-time values must build");

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(
		params,
		vec![QueryValue::Int(2), QueryValue::String(payload.to_owned())]
	);
}

#[rstest]
#[case::postgres(
	DatabaseType::Postgres,
	r#"INSERT INTO "insert_probe" ("created", "id", "touched", "payload") SELECT CURRENT_TIMESTAMP, $1, CURRENT_TIMESTAMP, $2 ON CONFLICT ("id") DO UPDATE SET "touched" = EXCLUDED."touched", "payload" = EXCLUDED."payload" RETURNING "id", "touched", "payload""#
)]
#[case::sqlite(
	DatabaseType::Sqlite,
	r#"INSERT INTO "insert_probe" ("created", "id", "touched", "payload") SELECT CURRENT_TIMESTAMP, ?, CURRENT_TIMESTAMP, ? ON CONFLICT ("id") DO UPDATE SET "touched" = excluded."touched", "payload" = excluded."payload" RETURNING "id", "touched", "payload""#
)]
fn test_insert_builder_now_conflict_returning_preserves_bindings(
	#[case] db_type: DatabaseType,
	#[case] expected_sql: &str,
	#[values(false, true)] fluent: bool,
) {
	// Arrange
	let payload = "literal CURRENT_TIMESTAMP '__REINHARDT_NOW__' $9 ?";
	let builder = InsertBuilder::new(MockBackend::new(db_type), "insert_probe")
		.value("created", QueryValue::Now)
		.value("id", 2_i64)
		.value("touched", QueryValue::Now)
		.value("payload", payload)
		.returning(vec!["id", "touched", "payload"]);
	let builder = if fluent {
		builder.on_conflict(
			OnConflictClause::columns(vec!["id"]).do_update(vec!["touched", "payload"]),
		)
	} else {
		builder.on_conflict_do_update(
			Some(vec!["id".into()]),
			vec!["touched".into(), "payload".into()],
		)
	};

	// Act
	let (sql, params) = builder.build().expect("current-time upsert must build");

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(
		params,
		vec![QueryValue::Int(2), QueryValue::String(payload.to_owned())]
	);
}

#[rstest]
fn test_insert_builder_with_on_conflict_do_nothing_postgres() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Postgres);

	// Act
	let (sql, _) = InsertBuilder::new(backend, "users")
		.value("email", QueryValue::String("test@example.com".to_string()))
		.on_conflict(OnConflictClause::columns(vec!["email"]).do_nothing())
		.build()
		.unwrap();

	// Assert
	assert!(sql.contains("ON CONFLICT"));
	assert!(sql.contains("DO NOTHING"));
}

#[rstest]
fn test_insert_builder_with_on_conflict_do_update_postgres() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Postgres);

	// Act
	let (sql, _) = InsertBuilder::new(backend, "users")
		.value("email", QueryValue::String("test@example.com".to_string()))
		.value("name", QueryValue::String("Alice".to_string()))
		.on_conflict(OnConflictClause::columns(vec!["email"]).do_update(vec!["name"]))
		.build()
		.unwrap();

	// Assert
	assert!(sql.contains("ON CONFLICT"));
	assert!(sql.contains("DO UPDATE SET"));
	assert!(sql.contains("EXCLUDED"));
}

#[rstest]
fn test_insert_builder_with_on_conflict_where_clause_postgres() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Postgres);

	// Act
	let (sql, _) = InsertBuilder::new(backend, "users")
		.value("email", QueryValue::String("test@example.com".to_string()))
		.value("name", QueryValue::String("Alice".to_string()))
		.on_conflict(
			OnConflictClause::columns(vec!["email"])
				.do_update(vec!["name"])
				.where_clause("users.updated_at < EXCLUDED.updated_at"),
		)
		.build()
		.unwrap();

	// Assert
	assert!(sql.contains("WHERE"));
	assert!(sql.contains("users.updated_at < EXCLUDED.updated_at"));
}

#[rstest]
fn test_insert_builder_with_on_conflict_constraint_postgres() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Postgres);

	// Act
	let (sql, _) = InsertBuilder::new(backend, "users")
		.value("email", QueryValue::String("test@example.com".to_string()))
		.on_conflict(OnConflictClause::constraint("users_email_key").do_update(vec!["name"]))
		.build()
		.unwrap();

	// Assert
	assert!(sql.contains(r#"ON CONSTRAINT "users_email_key""#));
}

#[rstest]
fn test_insert_builder_mysql_on_conflict_do_nothing() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Mysql);

	// Act
	let (sql, _) = InsertBuilder::new(backend, "users")
		.value("email", QueryValue::String("test@example.com".to_string()))
		.on_conflict(OnConflictClause::columns(vec!["email"]).do_nothing())
		.build()
		.unwrap();

	// Assert
	assert!(sql.contains("INSERT IGNORE"));
}

#[rstest]
fn test_insert_builder_mysql_on_conflict_do_update() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Mysql);

	// Act
	let (sql, _) = InsertBuilder::new(backend, "users")
		.value("email", QueryValue::String("test@example.com".to_string()))
		.value("name", QueryValue::String("Alice".to_string()))
		.on_conflict(OnConflictClause::columns(vec!["email"]).do_update(vec!["name"]))
		.build()
		.unwrap();

	// Assert
	assert!(sql.contains("ON DUPLICATE KEY UPDATE"));
	assert!(sql.contains("VALUES"));
}

#[rstest]
fn test_insert_builder_sqlite_on_conflict_do_nothing() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Sqlite);

	// Act
	let (sql, _) = InsertBuilder::new(backend, "users")
		.value("email", QueryValue::String("test@example.com".to_string()))
		.on_conflict(OnConflictClause::columns(vec!["email"]).do_nothing())
		.build()
		.unwrap();

	// Assert
	assert!(sql.contains("INSERT OR IGNORE"));
}

#[rstest]
fn test_insert_builder_sqlite_on_conflict_do_update() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Sqlite);

	// Act
	let (sql, _) = InsertBuilder::new(backend, "users")
		.value("email", QueryValue::String("test@example.com".to_string()))
		.value("name", QueryValue::String("Alice".to_string()))
		.on_conflict(OnConflictClause::columns(vec!["email"]).do_update(vec!["name"]))
		.build()
		.unwrap();

	// Assert
	assert!(sql.contains("ON CONFLICT"));
	assert!(sql.contains("DO UPDATE SET"));
	assert!(sql.contains("excluded")); // lowercase for SQLite
}

#[rstest]
fn test_insert_builder_returning_clause_postgres() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Postgres);

	// Act
	let (sql, _) = InsertBuilder::new(backend, "users")
		.value("name", QueryValue::String("Alice".to_string()))
		.returning(vec!["id"])
		.build()
		.unwrap();

	// Assert
	assert!(sql.contains("RETURNING"));
}

#[rstest]
#[case::postgres(
	DatabaseType::Postgres,
	"INSERT INTO \"users\" (\"id\") VALUES ($1) ON CONFLICT (\"id\") DO UPDATE SET \"id\" = EXCLUDED.\"id\" RETURNING \"id\""
)]
#[case::sqlite(
	DatabaseType::Sqlite,
	"INSERT INTO \"users\" (\"id\") VALUES (?) ON CONFLICT (\"id\") DO UPDATE SET \"id\" = excluded.\"id\" RETURNING \"id\""
)]
#[case::mysql(
	DatabaseType::Mysql,
	"INSERT INTO `users` (`id`) VALUES (?) ON DUPLICATE KEY UPDATE `id` = VALUES(`id`)"
)]
fn test_insert_conflict_update_before_returning(
	#[case] db_type: DatabaseType,
	#[case] expected_sql: &str,
	#[values(false, true)] fluent: bool,
) {
	// Arrange
	let builder = InsertBuilder::new(MockBackend::new(db_type), "users")
		.value("id", 3_i64)
		.returning(vec!["id"]);
	let builder = if fluent {
		builder.on_conflict(OnConflictClause::columns(vec!["id"]).do_update(vec!["id"]))
	} else {
		builder.on_conflict_do_update(Some(vec!["id".into()]), vec!["id".into()])
	};

	// Act
	let (sql, params) = builder.build().expect("upsert must build");

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(params, vec![QueryValue::Int(3)]);
}

#[rstest]
#[case::postgres(
	DatabaseType::Postgres,
	"INSERT INTO \"users\" (\"id\") VALUES ($1) ON CONFLICT DO NOTHING RETURNING \"id\""
)]
#[case::sqlite(
	DatabaseType::Sqlite,
	"INSERT OR IGNORE INTO \"users\" (\"id\") VALUES (?) RETURNING \"id\""
)]
#[case::mysql(DatabaseType::Mysql, "INSERT IGNORE INTO `users` (`id`) VALUES (?)")]
fn test_insert_conflict_do_nothing_with_returning(
	#[case] db_type: DatabaseType,
	#[case] expected_sql: &str,
	#[values(false, true)] fluent: bool,
) {
	// Arrange
	let builder = InsertBuilder::new(MockBackend::new(db_type), "users")
		.value("id", 3_i64)
		.returning(vec!["id"]);
	let builder = if fluent {
		builder.on_conflict(OnConflictClause::any().do_nothing())
	} else {
		builder.on_conflict_do_nothing(None)
	};

	// Act
	let (sql, params) = builder.build().expect("conflict ignore must build");

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(params, vec![QueryValue::Int(3)]);
}

#[rstest]
#[case::postgres(
	DatabaseType::Postgres,
	"INSERT INTO \"users\" (\"id\") VALUES ($1) ON CONFLICT (\"id\") DO UPDATE SET \"id\" = EXCLUDED.\"id\" WHERE users.id = 3 RETURNING \"id\", \"return\"\"ing\""
)]
#[case::sqlite(
	DatabaseType::Sqlite,
	"INSERT INTO \"users\" (\"id\") VALUES (?) ON CONFLICT (\"id\") DO UPDATE SET \"id\" = excluded.\"id\" WHERE users.id = 3 RETURNING \"id\", \"return\"\"ing\""
)]
fn test_insert_conflict_condition_before_returning(
	#[case] db_type: DatabaseType,
	#[case] expected_sql: &str,
) {
	// Arrange
	let builder = InsertBuilder::new(MockBackend::new(db_type), "users")
		.value("id", 3_i64)
		.returning(vec!["id", "return\"ing"])
		.on_conflict_do_nothing(None)
		.on_conflict(
			OnConflictClause::columns(vec!["id"])
				.do_update(vec!["id"])
				.where_clause("users.id = 3"),
		);

	// Act
	let (sql, params) = builder.build().expect("conditional upsert must build");

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(params, vec![QueryValue::Int(3)]);
}

#[rstest]
#[case::postgres(
	DatabaseType::Postgres,
	"INSERT INTO \"users\" (\"id\") SELECT 3 ON CONFLICT (\"id\") DO UPDATE SET \"id\" = EXCLUDED.\"id\" RETURNING \"id\""
)]
#[case::sqlite(
	DatabaseType::Sqlite,
	"INSERT INTO \"users\" (\"id\") SELECT * FROM (SELECT 3) AS \"__reinhardt_insert_source\" WHERE TRUE ON CONFLICT (\"id\") DO UPDATE SET \"id\" = excluded.\"id\" RETURNING \"id\""
)]
#[case::mysql(
	DatabaseType::Mysql,
	"INSERT INTO `users` (`id`) SELECT 3 ON DUPLICATE KEY UPDATE `id` = VALUES(`id`)"
)]
fn test_insert_select_conflict_update_before_returning(
	#[case] db_type: DatabaseType,
	#[case] expected_sql: &str,
	#[values(false, true)] configure_before_conversion: bool,
) {
	use reinhardt_query::prelude::{Expr, Query};

	// Arrange
	let select = Query::select().expr(Expr::val(3_i64)).to_owned();
	let builder = InsertBuilder::new(MockBackend::new(db_type), "users");
	let builder = if configure_before_conversion {
		builder
			.returning(vec!["id"])
			.on_conflict_do_update(Some(vec!["id".into()]), vec!["id".into()])
			.from_select(vec!["id"], select)
	} else {
		builder
			.from_select(vec!["id"], select)
			.on_conflict_do_update(Some(vec!["id".into()]), vec!["id".into()])
			.returning(vec!["id"])
	};

	// Act
	let (sql, params) = builder.build();

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(params, Vec::<QueryValue>::new());
}

#[rstest]
#[case::postgres(
	DatabaseType::Postgres,
	"INSERT INTO \"users\" (\"id\") SELECT 3 ON CONFLICT DO NOTHING RETURNING \"id\""
)]
#[case::sqlite(
	DatabaseType::Sqlite,
	"INSERT OR IGNORE INTO \"users\" (\"id\") SELECT 3 RETURNING \"id\""
)]
#[case::mysql(DatabaseType::Mysql, "INSERT IGNORE INTO `users` (`id`) SELECT 3")]
fn test_insert_select_conflict_do_nothing_with_returning(
	#[case] db_type: DatabaseType,
	#[case] expected_sql: &str,
) {
	use reinhardt_query::prelude::{Expr, Query};

	// Arrange
	let select = Query::select().expr(Expr::val(3_i64)).to_owned();
	let builder = InsertBuilder::new(MockBackend::new(db_type), "users")
		.from_select(vec!["id"], select)
		.returning(vec!["id"])
		.on_conflict_do_nothing(None);

	// Act
	let (sql, params) = builder.build();

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(params, Vec::<QueryValue>::new());
}

// ==================== SelectBuilder SQL generation tests ====================

#[rstest]
fn test_select_builder_basic_postgres() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Postgres);

	// Act
	let (sql, _) = SelectBuilder::new(backend).from("users").build();

	// Assert
	assert!(sql.contains("SELECT"));
	assert!(sql.contains("FROM"));
	assert!(sql.contains("users"));
}

#[rstest]
fn test_select_builder_with_columns() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Postgres);

	// Act
	let (sql, _) = SelectBuilder::new(backend)
		.columns(vec!["id", "name", "email"])
		.from("users")
		.build();

	// Assert
	assert!(sql.contains("id"));
	assert!(sql.contains("name"));
	assert!(sql.contains("email"));
}

#[rstest]
fn test_select_builder_with_where_clause() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Postgres);

	// Act
	let (sql, params) = SelectBuilder::new(backend)
		.from("users")
		.where_eq("id", QueryValue::Int(1))
		.build();

	// Assert
	assert!(sql.contains("WHERE"));
	assert_eq!(params.len(), 1);
	assert_eq!(params[0], QueryValue::Int(1));
}

#[rstest]
fn test_select_builder_with_limit() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Postgres);

	// Act
	let (sql, _) = SelectBuilder::new(backend).from("users").limit(10).build();

	// Assert
	// LIMIT is parameterized as $1 by build_select() (commit 0c337c302)
	assert!(sql.contains("LIMIT"));
	assert!(sql.contains("$1"));
}

#[rstest]
fn test_select_builder_mysql() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Mysql);

	// Act
	let (sql, _) = SelectBuilder::new(backend)
		.columns(vec!["id", "name"])
		.from("users")
		.where_eq("active", QueryValue::Bool(true))
		.build();

	// Assert
	assert!(sql.contains("SELECT"));
	assert!(sql.contains("FROM"));
}

// ==================== UpdateBuilder SQL generation tests ====================

#[rstest]
fn test_update_builder_basic_postgres() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Postgres);

	// Act
	let (sql, params) = UpdateBuilder::new(backend, "users")
		.set("name", QueryValue::String("Bob".to_string()))
		.where_eq("id", QueryValue::Int(1))
		.build();

	// Assert
	assert!(sql.contains("UPDATE"));
	assert!(sql.contains("users"));
	assert!(sql.contains("SET"));
	assert!(sql.contains("WHERE"));
	assert_eq!(params.len(), 2);
}

#[rstest]
#[case::postgres(
	DatabaseType::Postgres,
	"UPDATE \"users\" SET \"name\" = $1, \"updated_at\" = CURRENT_TIMESTAMP WHERE \"id\" = $2"
)]
#[case::mysql(
	DatabaseType::Mysql,
	"UPDATE `users` SET `name` = ?, `updated_at` = CURRENT_TIMESTAMP WHERE `id` = ?"
)]
#[case::sqlite(
	DatabaseType::Sqlite,
	"UPDATE \"users\" SET \"name\" = ?, \"updated_at\" = CURRENT_TIMESTAMP WHERE \"id\" = ?"
)]
fn test_update_builder_set_now(#[case] db_type: DatabaseType, #[case] expected_sql: &str) {
	// Arrange
	let backend = MockBackend::new(db_type);

	// Act
	let (sql, params) = UpdateBuilder::new(backend, "users")
		.set("name", "Alice")
		.set_now("updated_at")
		.where_eq("id", 1_i64)
		.build();

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(params, vec![QueryValue::from("Alice"), QueryValue::Int(1)]);
}

#[rstest]
#[case::postgres(
	DatabaseType::Postgres,
	"UPDATE \"users\" SET \"updated_at\" = CURRENT_TIMESTAMP WHERE \"id\" = $1"
)]
#[case::mysql(
	DatabaseType::Mysql,
	"UPDATE `users` SET `updated_at` = CURRENT_TIMESTAMP WHERE `id` = ?"
)]
#[case::sqlite(
	DatabaseType::Sqlite,
	"UPDATE \"users\" SET \"updated_at\" = CURRENT_TIMESTAMP WHERE \"id\" = ?"
)]
fn test_update_builder_set_now_only(#[case] db_type: DatabaseType, #[case] expected_sql: &str) {
	// Arrange
	let backend = MockBackend::new(db_type);

	// Act
	let (sql, params) = UpdateBuilder::new(backend, "users")
		.set_now("updated_at")
		.where_eq("id", 1_i64)
		.build();

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(params, vec![QueryValue::Int(1)]);
}

#[rstest]
#[case::postgres(
	DatabaseType::Postgres,
	"UPDATE \"users\" SET \"updated_at\" = CURRENT_TIMESTAMP, \"name\" = $1, \"touched_at\" = CURRENT_TIMESTAMP, \"age\" = $2 WHERE \"id\" = $3 AND \"active\" = $4"
)]
#[case::mysql(
	DatabaseType::Mysql,
	"UPDATE `users` SET `updated_at` = CURRENT_TIMESTAMP, `name` = ?, `touched_at` = CURRENT_TIMESTAMP, `age` = ? WHERE `id` = ? AND `active` = ?"
)]
#[case::sqlite(
	DatabaseType::Sqlite,
	"UPDATE \"users\" SET \"updated_at\" = CURRENT_TIMESTAMP, \"name\" = ?, \"touched_at\" = CURRENT_TIMESTAMP, \"age\" = ? WHERE \"id\" = ? AND \"active\" = ?"
)]
fn test_update_builder_set_now_preserves_parameter_order(
	#[case] db_type: DatabaseType,
	#[case] expected_sql: &str,
) {
	// Arrange
	let backend = MockBackend::new(db_type);
	let name = "'__REINHARDT_NOW__' ? $42";

	// Act
	let (sql, params) = UpdateBuilder::new(backend, "users")
		.set_now("updated_at")
		.set("name", name)
		.set_now("touched_at")
		.set("age", 25_i64)
		.where_eq("id", 1_i64)
		.where_eq("active", true)
		.build();

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(
		params,
		vec![
			QueryValue::from(name),
			QueryValue::Int(25),
			QueryValue::Int(1),
			QueryValue::Bool(true),
		]
	);
}

#[rstest]
#[case::bool(QueryValue::Bool(true))]
#[case::int(QueryValue::Int(i64::MAX))]
#[case::float(QueryValue::Float(5.25))]
#[case::string(QueryValue::from("'__REINHARDT_NOW__' ? $42"))]
#[case::bytes(QueryValue::Bytes(vec![0, 255]))]
#[case::timestamp(QueryValue::Timestamp(chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap()))]
#[case::uuid(QueryValue::Uuid(uuid::Uuid::from_u128(1)))]
#[case::null(QueryValue::Null)]
fn test_update_builder_set_now_preserves_value_types(
	#[values(DatabaseType::Postgres, DatabaseType::Mysql, DatabaseType::Sqlite)]
	db_type: DatabaseType,
	#[case] value: QueryValue,
) {
	// Arrange
	let backend = MockBackend::new(db_type);
	let is_null = value == QueryValue::Null;
	let (assignment, predicate) = match (db_type, is_null) {
		(DatabaseType::Postgres, false) => ("$1", "$2"),
		(DatabaseType::Postgres, true) => ("NULL", "$1"),
		(_, false) => ("?", "?"),
		(_, true) => ("NULL", "?"),
	};
	let expected_sql = if db_type == DatabaseType::Mysql {
		format!(
			"UPDATE `users` SET `updated_at` = CURRENT_TIMESTAMP, `value` = {assignment} WHERE `id` = {predicate}"
		)
	} else {
		format!(
			"UPDATE \"users\" SET \"updated_at\" = CURRENT_TIMESTAMP, \"value\" = {assignment} WHERE \"id\" = {predicate}"
		)
	};
	let expected_params = if is_null {
		vec![QueryValue::Int(1)]
	} else {
		vec![value.clone(), QueryValue::Int(1)]
	};

	// Act
	let (sql, params) = UpdateBuilder::new(backend, "users")
		.set_now("updated_at")
		.set("value", value)
		.where_eq("id", 1_i64)
		.build();

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(params, expected_params);
}

#[cfg(feature = "sqlite")]
#[rstest::fixture]
async fn timestamp_connection() -> reinhardt_db::backends::DatabaseConnection {
	use reinhardt_db::backends::DatabaseConnection;
	use reinhardt_query::prelude::{
		ColumnDef, Iden, IntoIden, Query, QueryBuilder, SqliteQueryBuilder,
	};

	#[derive(Debug, Iden)]
	enum BuilderProbe {
		Table,
		Id,
		Touched,
	}

	let connection = DatabaseConnection::connect_sqlite("sqlite::memory:")
		.await
		.unwrap();
	let create = Query::create_table()
		.table(BuilderProbe::Table.into_iden())
		.col(
			ColumnDef::new(BuilderProbe::Id)
				.big_integer()
				.primary_key(true),
		)
		.col(ColumnDef::new(BuilderProbe::Touched).timestamp())
		.to_owned();
	let (sql, _) = SqliteQueryBuilder.build_create_table(&create);
	connection.execute(&sql, vec![]).await.unwrap();
	for id in [1_i64, 2_i64] {
		InsertBuilder::new(connection.backend(), "builder_probe")
			.value("id", id)
			.execute()
			.await
			.unwrap();
	}
	connection
}

#[cfg(feature = "sqlite")]
#[rstest]
#[tokio::test]
async fn test_update_builder_set_now_updates_sqlite_row(
	#[future] timestamp_connection: reinhardt_db::backends::DatabaseConnection,
) {
	use reinhardt_query::prelude::{
		Alias, Expr, ExprTrait, Query, QueryBuilder, SqliteQueryBuilder,
	};

	// Arrange
	let connection = timestamp_connection.await;
	let builder = UpdateBuilder::new(connection.backend(), "builder_probe")
		.set_now("touched")
		.where_eq("id", 1_i64);

	// Act
	let result = builder.execute().await.unwrap();

	// Assert
	assert_eq!(result.rows_affected, 1);
	let mut select = Query::select();
	select
		.expr_as(Expr::col("touched").is_not_null(), Alias::new("is_touched"))
		.from("builder_probe")
		.and_where(Expr::col("id").eq(1_i64));
	let (sql, _) = SqliteQueryBuilder.build_select(&select);
	let updated = connection
		.fetch_one(&sql, vec![QueryValue::Int(1)])
		.await
		.unwrap();
	assert_eq!(updated.get::<i64>("is_touched").unwrap(), 1);
	let untouched = connection
		.fetch_one(&sql, vec![QueryValue::Int(2)])
		.await
		.unwrap();
	assert_eq!(untouched.get::<i64>("is_touched").unwrap(), 0);
}

#[rstest]
fn test_update_builder_multiple_sets() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Postgres);

	// Act
	let (sql, params) = UpdateBuilder::new(backend, "users")
		.set("name", QueryValue::String("Alice".to_string()))
		.set("age", QueryValue::Int(25))
		.where_eq("id", QueryValue::Int(1))
		.build();

	// Assert
	assert!(sql.contains("SET"));
	assert_eq!(params.len(), 3);
}

#[rstest]
fn test_update_builder_mysql() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Mysql);

	// Act
	let (sql, params) = UpdateBuilder::new(backend, "users")
		.set("email", QueryValue::String("new@example.com".to_string()))
		.where_eq("id", QueryValue::Int(42))
		.build();

	// Assert
	assert!(sql.contains("UPDATE"));
	assert_eq!(params.len(), 2);
}

// ==================== QueryResult tests ====================

#[rstest]
fn test_query_result_rows_affected() {
	// Arrange

	// Act
	let result = QueryResult { rows_affected: 5 };

	// Assert
	assert_eq!(result.rows_affected, 5);
}

#[rstest]
fn test_query_result_equality() {
	// Arrange
	let r1 = QueryResult { rows_affected: 3 };
	let r2 = QueryResult { rows_affected: 3 };
	let r3 = QueryResult { rows_affected: 7 };

	// Act

	// Assert
	assert_eq!(r1, r2);
	assert_ne!(r1, r3);
}

// ==================== Row tests ====================

#[rstest]
fn test_row_new_is_empty() {
	// Arrange

	// Act
	let row = Row::new();

	// Assert
	assert!(row.data.is_empty());
}

#[rstest]
fn test_row_insert_and_get() {
	// Arrange
	let mut row = Row::new();
	row.insert("id".to_string(), QueryValue::Int(1));
	row.insert("name".to_string(), QueryValue::String("Alice".to_string()));

	// Act
	let id: i64 = row.get("id").unwrap();
	let name: String = row.get("name").unwrap();

	// Assert
	assert_eq!(id, 1);
	assert_eq!(name, "Alice");
}

#[rstest]
fn test_row_get_column_not_found() {
	// Arrange
	let row = Row::new();

	// Act
	let result: std::result::Result<i64, _> = row.get("nonexistent");

	// Assert
	assert!(result.is_err());
}

#[rstest]
fn test_row_default() {
	// Arrange

	// Act
	let row = Row::default();

	// Assert
	assert!(row.data.is_empty());
}

// ==================== Savepoint tests ====================

#[rstest]
fn test_savepoint_to_sql() {
	// Arrange
	let sp = Savepoint::new("test_sp");

	// Act

	// Assert
	assert_eq!(sp.to_sql(), "SAVEPOINT \"test_sp\"");
	assert_eq!(sp.release_sql(), "RELEASE SAVEPOINT \"test_sp\"");
	assert_eq!(sp.rollback_sql(), "ROLLBACK TO SAVEPOINT \"test_sp\"");
}

#[rstest]
fn test_savepoint_name() {
	// Arrange

	// Act
	let sp = Savepoint::new("my_sp");

	// Assert
	assert_eq!(sp.name(), "my_sp");
}

// ==================== Legacy OnConflictAction via InsertBuilder ====================

#[rstest]
fn test_insert_builder_legacy_on_conflict_do_nothing() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Postgres);

	// Act
	let (sql, _) = InsertBuilder::new(backend, "users")
		.value("email", QueryValue::String("test@example.com".to_string()))
		.on_conflict_do_nothing(Some(vec!["email".to_string()]))
		.build()
		.unwrap();

	// Assert
	assert!(sql.contains(r#"ON CONFLICT ("email") DO NOTHING"#));
}

#[rstest]
fn test_insert_builder_legacy_on_conflict_do_update() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Postgres);

	// Act
	let (sql, _) = InsertBuilder::new(backend, "users")
		.value("email", QueryValue::String("test@example.com".to_string()))
		.value("name", QueryValue::String("Alice".to_string()))
		.on_conflict_do_update(Some(vec!["email".to_string()]), vec!["name".to_string()])
		.build()
		.unwrap();

	// Assert
	assert!(sql.contains(r#"ON CONFLICT ("email") DO UPDATE SET"#));
	assert!(sql.contains(r#""name" = EXCLUDED."name""#));
}

// ==================== OnConflictClauseAction tests ====================

#[rstest]
fn test_on_conflict_clause_action_do_nothing_debug() {
	// Arrange

	// Act
	let action = OnConflictClauseAction::DoNothing;

	// Assert
	let debug = format!("{:?}", action);
	assert_eq!(debug, "DoNothing");
}

#[rstest]
fn test_on_conflict_clause_action_do_update_debug() {
	// Arrange

	// Act
	let action = OnConflictClauseAction::DoUpdate {
		update_columns: vec!["name".to_string(), "email".to_string()],
	};

	// Assert
	let debug = format!("{:?}", action);
	assert!(debug.contains("DoUpdate"));
	assert!(debug.contains("name"));
	assert!(debug.contains("email"));
}

// ==================== OnConflictAction variant tests ====================

#[rstest]
fn test_on_conflict_action_do_nothing_without_columns() {
	// Arrange

	// Act
	let action = reinhardt_db::backends::query_builder::OnConflictAction::DoNothing {
		conflict_columns: None,
	};

	// Assert
	let debug = format!("{:?}", action);
	assert!(debug.contains("DoNothing"));
	assert!(debug.contains("None"));
}

#[rstest]
fn test_on_conflict_action_do_nothing_with_columns() {
	// Arrange

	// Act
	let action = reinhardt_db::backends::query_builder::OnConflictAction::DoNothing {
		conflict_columns: Some(vec!["email".to_string(), "tenant_id".to_string()]),
	};

	// Assert
	let debug = format!("{:?}", action);
	assert!(debug.contains("DoNothing"));
	assert!(debug.contains("email"));
	assert!(debug.contains("tenant_id"));
}

#[rstest]
fn test_on_conflict_action_do_update_with_columns() {
	// Arrange

	// Act
	let action = reinhardt_db::backends::query_builder::OnConflictAction::DoUpdate {
		conflict_columns: Some(vec!["id".to_string()]),
		update_columns: vec!["name".to_string(), "email".to_string()],
	};

	// Assert
	let debug = format!("{:?}", action);
	assert!(debug.contains("DoUpdate"));
	assert!(debug.contains("id"));
	assert!(debug.contains("name"));
	assert!(debug.contains("email"));
}

#[rstest]
fn test_on_conflict_action_do_update_without_conflict_columns() {
	// Arrange

	// Act
	let action = reinhardt_db::backends::query_builder::OnConflictAction::DoUpdate {
		conflict_columns: None,
		update_columns: vec!["status".to_string()],
	};

	// Assert
	let debug = format!("{:?}", action);
	assert!(debug.contains("DoUpdate"));
	assert!(debug.contains("None"));
	assert!(debug.contains("status"));
}

// ==================== OnConflictClause advanced tests ====================

#[rstest]
fn test_on_conflict_clause_clone() {
	// Arrange
	let clause = OnConflictClause::columns(vec!["email"])
		.do_update(vec!["name"])
		.where_clause("users.version > 1");

	// Act
	let cloned = clause.clone();

	// Assert
	let debug_original = format!("{:?}", clause);
	let debug_cloned = format!("{:?}", cloned);
	assert_eq!(debug_original, debug_cloned);
}

#[rstest]
fn test_on_conflict_clause_single_column() {
	// Arrange

	// Act
	let clause = OnConflictClause::columns(vec!["id"]);

	// Assert
	let debug = format!("{:?}", clause);
	assert!(debug.contains("Columns"));
	assert!(debug.contains("id"));
}

#[rstest]
fn test_on_conflict_clause_many_columns() {
	// Arrange

	// Act
	let clause = OnConflictClause::columns(vec!["email", "tenant_id", "region"]);

	// Assert
	let debug = format!("{:?}", clause);
	assert!(debug.contains("email"));
	assert!(debug.contains("tenant_id"));
	assert!(debug.contains("region"));
}

#[rstest]
fn test_on_conflict_clause_override_action_do_update_then_do_nothing() {
	// Arrange
	let clause = OnConflictClause::columns(vec!["email"]).do_update(vec!["name"]);

	// Act: override with do_nothing
	let clause = clause.do_nothing();

	// Assert
	let debug = format!("{:?}", clause);
	assert!(debug.contains("DoNothing"));
	// Should no longer contain DoUpdate
	assert!(!debug.contains("update_columns"));
}

#[rstest]
fn test_on_conflict_clause_where_clause_without_do_update() {
	// Arrange

	// Act - where_clause can be called even with DoNothing action
	let clause = OnConflictClause::columns(vec!["email"])
		.do_nothing()
		.where_clause("1 = 1");

	// Assert
	let debug = format!("{:?}", clause);
	assert!(debug.contains("DoNothing"));
	assert!(debug.contains("1 = 1"));
}

#[rstest]
fn test_on_conflict_clause_any_do_nothing_sql_postgres() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Postgres);

	// Act
	let (sql, _) = InsertBuilder::new(backend, "users")
		.value("email", QueryValue::String("test@example.com".to_string()))
		.on_conflict(OnConflictClause::any().do_nothing())
		.build()
		.unwrap();

	// Assert
	assert!(sql.contains("ON CONFLICT DO NOTHING"));
}

// ==================== QueryValue TryFrom error tests ====================

#[rstest]
fn test_query_value_try_from_i64_type_error() {
	// Arrange
	let val = QueryValue::String("not a number".to_string());

	// Act
	let result: std::result::Result<i64, _> = val.try_into();

	// Assert
	assert!(result.is_err());
}

#[rstest]
fn test_query_value_try_from_i32_type_error() {
	// Arrange
	let val = QueryValue::Bool(true);

	// Act
	let result: std::result::Result<i32, _> = val.try_into();

	// Assert
	assert!(result.is_err());
}

#[rstest]
fn test_query_value_try_from_i32_overflow() {
	// Arrange: i64 value too large for i32
	let val = QueryValue::Int(i64::MAX);

	// Act
	let result: std::result::Result<i32, _> = val.try_into();

	// Assert
	assert!(result.is_err());
}

#[rstest]
fn test_query_value_try_from_u64_negative() {
	// Arrange: negative i64 cannot convert to u64
	let val = QueryValue::Int(-1);

	// Act
	let result: std::result::Result<u64, _> = val.try_into();

	// Assert
	assert!(result.is_err());
}

#[rstest]
fn test_query_value_try_from_u32_overflow() {
	// Arrange: i64 value too large for u32
	let val = QueryValue::Int(i64::from(u32::MAX) + 1);

	// Act
	let result: std::result::Result<u32, _> = val.try_into();

	// Assert
	assert!(result.is_err());
}

#[rstest]
fn test_query_value_try_from_string_type_error() {
	// Arrange
	let val = QueryValue::Int(42);

	// Act
	let result: std::result::Result<String, _> = val.try_into();

	// Assert
	assert!(result.is_err());
}

#[rstest]
fn test_query_value_try_from_bool_type_error() {
	// Arrange
	let val = QueryValue::Int(1);

	// Act
	let result: std::result::Result<bool, _> = val.try_into();

	// Assert
	assert!(result.is_err());
}

#[rstest]
fn test_query_value_try_from_f64_type_error() {
	// Arrange
	let val = QueryValue::String("3.14".to_string());

	// Act
	let result: std::result::Result<f64, _> = val.try_into();

	// Assert
	assert!(result.is_err());
}

#[rstest]
fn test_query_value_try_from_uuid_from_valid_string() {
	// Arrange
	let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
	let val = QueryValue::String(uuid_str.to_string());

	// Act
	let result: std::result::Result<uuid::Uuid, _> = val.try_into();

	// Assert
	assert!(result.is_ok());
	assert_eq!(result.unwrap().to_string(), uuid_str);
}

#[rstest]
fn test_query_value_try_from_uuid_from_invalid_string() {
	// Arrange
	let val = QueryValue::String("not-a-uuid".to_string());

	// Act
	let result: std::result::Result<uuid::Uuid, _> = val.try_into();

	// Assert
	assert!(result.is_err());
}

#[rstest]
fn test_query_value_try_from_uuid_type_error() {
	// Arrange
	let val = QueryValue::Int(42);

	// Act
	let result: std::result::Result<uuid::Uuid, _> = val.try_into();

	// Assert
	assert!(result.is_err());
}

#[rstest]
fn test_query_value_try_from_chrono_type_error() {
	// Arrange
	let val = QueryValue::String("2024-01-01".to_string());

	// Act
	let result: std::result::Result<chrono::DateTime<chrono::Utc>, _> = val.try_into();

	// Assert
	assert!(result.is_err());
}

// ==================== Row advanced tests ====================

#[rstest]
fn test_row_get_wrong_type_conversion() {
	// Arrange
	let mut row = Row::new();
	row.insert(
		"count".to_string(),
		QueryValue::String("not_a_number".to_string()),
	);

	// Act: try to get a String value as i64
	let result: std::result::Result<i64, _> = row.get("count");

	// Assert
	assert!(result.is_err());
}

#[rstest]
fn test_row_equality() {
	// Arrange
	let mut row1 = Row::new();
	row1.insert("id".to_string(), QueryValue::Int(1));

	let mut row2 = Row::new();
	row2.insert("id".to_string(), QueryValue::Int(1));

	// Act

	// Assert
	assert_eq!(row1, row2);
}

// ==================== InsertBuilder edge cases ====================

#[rstest]
fn test_insert_builder_returning_ignored_on_mysql() {
	// Arrange: MySQL does not support RETURNING
	let backend = MockBackend::new(DatabaseType::Mysql);

	// Act
	let (sql, _) = InsertBuilder::new(backend, "users")
		.value("name", QueryValue::String("Alice".to_string()))
		.returning(vec!["id"])
		.build()
		.unwrap();

	// Assert: RETURNING should not appear in MySQL SQL
	assert!(!sql.contains("RETURNING"));
}

#[rstest]
fn test_insert_builder_returning_clause_sqlite() {
	// Arrange: SQLite supports RETURNING
	let backend = MockBackend::new(DatabaseType::Sqlite);

	// Act
	let (sql, _) = InsertBuilder::new(backend, "users")
		.value("name", QueryValue::String("Alice".to_string()))
		.returning(vec!["id", "name"])
		.build()
		.unwrap();

	// Assert
	assert!(sql.contains("RETURNING"));
}

#[rstest]
fn test_insert_builder_legacy_on_conflict_do_nothing_without_columns() {
	// Arrange
	let backend = MockBackend::new(DatabaseType::Postgres);

	// Act
	let (sql, _) = InsertBuilder::new(backend, "users")
		.value("email", QueryValue::String("test@example.com".to_string()))
		.on_conflict_do_nothing(None)
		.build()
		.unwrap();

	// Assert: should produce ON CONFLICT DO NOTHING without column list
	assert!(sql.contains("ON CONFLICT DO NOTHING"));
	assert!(!sql.contains("ON CONFLICT ("));
}

// ==================== Savepoint additional tests ====================

#[rstest]
fn test_savepoint_equality() {
	// Arrange
	let sp1 = Savepoint::new("sp1");
	let sp2 = Savepoint::new("sp1");
	let sp3 = Savepoint::new("sp2");

	// Act

	// Assert
	assert_eq!(sp1, sp2);
	assert_ne!(sp1, sp3);
}

#[rstest]
fn test_savepoint_clone() {
	// Arrange
	let sp = Savepoint::new("my_sp");

	// Act
	let cloned = sp.clone();

	// Assert
	assert_eq!(sp, cloned);
	assert_eq!(sp.name(), cloned.name());
}

// ==================== OnConflictClauseAction clone tests ====================

#[rstest]
fn test_on_conflict_clause_action_do_nothing_clone() {
	// Arrange
	let action = OnConflictClauseAction::DoNothing;

	// Act
	let cloned = action.clone();

	// Assert
	let debug_original = format!("{:?}", action);
	let debug_cloned = format!("{:?}", cloned);
	assert_eq!(debug_original, debug_cloned);
}

#[rstest]
fn test_on_conflict_clause_action_do_update_clone() {
	// Arrange
	let action = OnConflictClauseAction::DoUpdate {
		update_columns: vec!["name".to_string(), "email".to_string()],
	};

	// Act
	let cloned = action.clone();

	// Assert
	let debug_original = format!("{:?}", action);
	let debug_cloned = format!("{:?}", cloned);
	assert_eq!(debug_original, debug_cloned);
}

// ==================== QueryValue clone and equality edge cases ====================

#[rstest]
fn test_query_value_clone() {
	// Arrange
	let val = QueryValue::String("test_clone".to_string());

	// Act
	let cloned = val.clone();

	// Assert
	assert_eq!(val, cloned);
}

#[rstest]
fn test_query_value_ne_different_variants() {
	// Arrange

	// Act

	// Assert - different variants are never equal
	assert_ne!(QueryValue::Null, QueryValue::Bool(false));
	assert_ne!(QueryValue::Int(0), QueryValue::Float(0.0));
	assert_ne!(QueryValue::String("0".to_string()), QueryValue::Int(0));
	assert_ne!(QueryValue::Now, QueryValue::Null);
}

// ==================== DeleteBuilder IN predicate contracts ====================

#[rstest]
#[case::postgres(
	DatabaseType::Postgres,
	"DELETE FROM \"users\" WHERE \"id\" IN ($1, $2, $3)"
)]
#[case::mysql(DatabaseType::Mysql, "DELETE FROM `users` WHERE `id` IN (?, ?, ?)")]
#[case::sqlite(
	DatabaseType::Sqlite,
	"DELETE FROM \"users\" WHERE \"id\" IN (?, ?, ?)"
)]
fn test_delete_builder_where_in_groups_values(
	#[case] db_type: DatabaseType,
	#[case] expected_sql: &str,
) {
	// Arrange: include duplicate values and preserve their original order.
	let values = vec![QueryValue::Int(2), QueryValue::Int(1), QueryValue::Int(2)];
	let builder =
		DeleteBuilder::new(MockBackend::new(db_type), "users").where_in("id", values.clone());

	// Act
	let (sql, params) = builder.build();

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(params, values);
}

#[rstest]
#[case::postgres(DatabaseType::Postgres, "DELETE FROM \"users\" WHERE \"id\" IN ($1)")]
#[case::mysql(DatabaseType::Mysql, "DELETE FROM `users` WHERE `id` IN (?)")]
#[case::sqlite(DatabaseType::Sqlite, "DELETE FROM \"users\" WHERE \"id\" IN (?)")]
fn test_delete_builder_where_in_single_value(
	#[case] db_type: DatabaseType,
	#[case] expected_sql: &str,
) {
	// Arrange
	let builder = DeleteBuilder::new(MockBackend::new(db_type), "users")
		.where_in("id", vec![QueryValue::Int(2)]);

	// Act
	let (sql, params) = builder.build();

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(params, vec![QueryValue::Int(2)]);
}

#[rstest]
#[case::postgres(
	DatabaseType::Postgres,
	"DELETE FROM \"users\" WHERE \"status\" = $1 AND \"id\" IN ($2, $3) AND \"tenant_id\" = $4 AND \"id\" IN ($5, $6)"
)]
#[case::mysql(
	DatabaseType::Mysql,
	"DELETE FROM `users` WHERE `status` = ? AND `id` IN (?, ?) AND `tenant_id` = ? AND `id` IN (?, ?)"
)]
#[case::sqlite(
	DatabaseType::Sqlite,
	"DELETE FROM \"users\" WHERE \"status\" = ? AND \"id\" IN (?, ?) AND \"tenant_id\" = ? AND \"id\" IN (?, ?)"
)]
fn test_delete_builder_where_in_repeated_with_equality(
	#[case] db_type: DatabaseType,
	#[case] expected_sql: &str,
) {
	// Arrange
	let builder = DeleteBuilder::new(MockBackend::new(db_type), "users")
		.where_eq("status", "inactive")
		.where_in("id", vec![QueryValue::Int(2), QueryValue::Int(1)])
		.where_eq("tenant_id", 9_i64)
		.where_in("id", vec![QueryValue::Int(2), QueryValue::Int(3)]);

	// Act
	let (sql, params) = builder.build();

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(
		params,
		vec![
			QueryValue::String("inactive".to_owned()),
			QueryValue::Int(2),
			QueryValue::Int(1),
			QueryValue::Int(9),
			QueryValue::Int(2),
			QueryValue::Int(3),
		]
	);
}

#[rstest]
#[case::postgres(DatabaseType::Postgres, "DELETE FROM \"users\" WHERE FALSE")]
#[case::mysql(DatabaseType::Mysql, "DELETE FROM `users` WHERE FALSE")]
#[case::sqlite(DatabaseType::Sqlite, "DELETE FROM \"users\" WHERE FALSE")]
fn test_delete_builder_where_in_empty_matches_nothing(
	#[case] db_type: DatabaseType,
	#[case] expected_sql: &str,
) {
	// Arrange
	let builder = DeleteBuilder::new(MockBackend::new(db_type), "users").where_in("id", Vec::new());

	// Act
	let (sql, params) = builder.build();

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(params, Vec::<QueryValue>::new());
}

#[rstest]
#[case::postgres(
	DatabaseType::Postgres,
	"DELETE FROM \"users\" WHERE \"tenant_id\" = $1 AND FALSE AND \"id\" IN ($2, $3)"
)]
#[case::mysql(
	DatabaseType::Mysql,
	"DELETE FROM `users` WHERE `tenant_id` = ? AND FALSE AND `id` IN (?, ?)"
)]
#[case::sqlite(
	DatabaseType::Sqlite,
	"DELETE FROM \"users\" WHERE \"tenant_id\" = ? AND FALSE AND \"id\" IN (?, ?)"
)]
fn test_delete_builder_where_in_empty_preserves_other_parameters(
	#[case] db_type: DatabaseType,
	#[case] expected_sql: &str,
) {
	// Arrange
	let builder = DeleteBuilder::new(MockBackend::new(db_type), "users")
		.where_eq("tenant_id", 9_i64)
		.where_in("id", Vec::new())
		.where_in("id", vec![QueryValue::Int(2), QueryValue::Int(1)]);

	// Act
	let (sql, params) = builder.build();

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(
		params,
		vec![QueryValue::Int(9), QueryValue::Int(2), QueryValue::Int(1)]
	);
}

#[cfg(feature = "postgres")]
mod insert_builder_postgres_tests {
	use super::*;
	use reinhardt_db::backends::dialect::PostgresBackend;
	use reinhardt_query::prelude::{
		ColumnDef, Expr, Iden, IntoIden, PostgresQueryBuilder, Query, QueryStatementBuilder,
	};
	use rstest::fixture;
	use testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner};
	use testcontainers_modules::postgres::Postgres;

	#[derive(Debug, Iden)]
	enum InsertProbe {
		Table,
		Id,
		Name,
	}

	#[fixture]
	async fn insert_probe() -> (ContainerAsync<Postgres>, Arc<PostgresBackend>) {
		let container = Postgres::default()
			.with_tag("17-alpine")
			.start()
			.await
			.expect("PostgreSQL container must start");
		let host = container.get_host().await.expect("host must be available");
		let port = container
			.get_host_port_ipv4(5432)
			.await
			.expect("port must be available");
		let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
		let pool = sqlx::PgPool::connect(&url)
			.await
			.expect("PostgreSQL must connect");
		let backend = Arc::new(PostgresBackend::new(pool));
		let create = Query::create_table()
			.table(InsertProbe::Table.into_iden())
			.col(
				ColumnDef::new(InsertProbe::Id)
					.big_integer()
					.primary_key(true),
			)
			.col(ColumnDef::new(InsertProbe::Name).string())
			.to_string(PostgresQueryBuilder);
		backend
			.execute(&create, Vec::new())
			.await
			.expect("table must be created");
		InsertBuilder::new(backend.clone(), "insert_probe")
			.value("id", 3_i64)
			.value("name", "original")
			.execute()
			.await
			.expect("original row must be inserted");
		(container, backend)
	}

	#[rstest]
	#[tokio::test]
	async fn test_insert_conflict_returning_postgres_execution(
		#[future] insert_probe: (ContainerAsync<Postgres>, Arc<PostgresBackend>),
		#[values(false, true)] select_source: bool,
	) {
		// Arrange
		let (_container, backend) = insert_probe.await;
		let builder = InsertBuilder::new(backend, "insert_probe")
			.returning(vec!["id", "name"])
			.on_conflict_do_update(Some(vec!["id".into()]), vec!["name".into()]);

		// Act
		let row = if select_source {
			let select = Query::select()
				.expr(Expr::val(3_i64))
				.expr(Expr::val("updated"))
				.to_owned();
			builder
				.from_select(vec!["id", "name"], select)
				.fetch_one()
				.await
		} else {
			builder
				.value("id", 3_i64)
				.value("name", "updated")
				.fetch_one()
				.await
		}
		.expect("upsert must return the updated row");

		// Assert
		assert_eq!(row.get::<i64>("id").expect("id must be returned"), 3);
		assert_eq!(
			row.get::<String>("name").expect("name must be returned"),
			"updated"
		);
	}
}

#[cfg(feature = "sqlite")]
mod builder_sqlite_tests {
	use super::*;
	use reinhardt_db::backends::dialect::SqliteBackend;
	use reinhardt_query::prelude::{
		ColumnDef, Expr, Iden, IntoIden, Order, Query, QueryStatementBuilder, SqliteQueryBuilder,
	};
	use rstest::fixture;
	use sqlx::sqlite::SqlitePoolOptions;

	#[derive(Debug, Iden)]
	enum BuilderProbe {
		Table,
		Id,
	}

	// Each fixture owns an isolated memory database; dropping its pool releases it.
	#[fixture]
	async fn builder_probe() -> Arc<SqliteBackend> {
		let pool = SqlitePoolOptions::new()
			.max_connections(1)
			.connect("sqlite::memory:")
			.await
			.expect("in-memory SQLite must connect");
		let backend = Arc::new(SqliteBackend::new(pool));
		let create = Query::create_table()
			.table(BuilderProbe::Table.into_iden())
			.col(
				ColumnDef::new(BuilderProbe::Id)
					.integer()
					.not_null(true)
					.primary_key(true),
			)
			.to_string(SqliteQueryBuilder);
		backend
			.execute(&create, Vec::new())
			.await
			.expect("probe table must be created");
		let insert = Query::insert()
			.into_table(BuilderProbe::Table.into_iden())
			.columns([BuilderProbe::Id])
			.values_panic([1_i64])
			.values_panic([2_i64])
			.values_panic([3_i64])
			.to_string(SqliteQueryBuilder);
		backend
			.execute(&insert, Vec::new())
			.await
			.expect("probe rows must be inserted");
		backend
	}

	#[rstest]
	#[tokio::test]
	async fn test_insert_conflict_returning_sqlite_execution(
		#[future] builder_probe: Arc<SqliteBackend>,
		#[values(3_i64, 4_i64)] id: i64,
		#[values(false, true)] fluent: bool,
	) {
		// Arrange
		let backend = builder_probe.await;
		let builder = InsertBuilder::new(backend, "builder_probe")
			.value("id", id)
			.returning(vec!["id"]);
		let builder = if fluent {
			builder.on_conflict(OnConflictClause::columns(vec!["id"]).do_update(vec!["id"]))
		} else {
			builder.on_conflict_do_update(Some(vec!["id".into()]), vec!["id".into()])
		};

		// Act
		let row = builder.fetch_one().await.expect("upsert must return a row");

		// Assert
		assert_eq!(row.get::<i64>("id").expect("id must be returned"), id);
	}

	#[rstest]
	#[tokio::test]
	async fn test_insert_select_conflict_returning_sqlite_execution(
		#[future] builder_probe: Arc<SqliteBackend>,
		#[values(3_i64, 4_i64)] id: i64,
	) {
		// Arrange
		let backend = builder_probe.await;
		let select = Query::select().expr(Expr::val(id)).to_owned();
		let builder = InsertBuilder::new(backend, "builder_probe")
			.returning(vec!["id"])
			.on_conflict_do_update(Some(vec!["id".into()]), vec!["id".into()])
			.from_select(vec!["id"], select);

		// Act
		let row = builder
			.fetch_one()
			.await
			.expect("select upsert must return a row");

		// Assert
		assert_eq!(row.get::<i64>("id").expect("id must be returned"), id);
	}

	#[rstest]
	#[case::multiple(vec![vec![2, 1]], None, vec![3])]
	#[case::single(vec![vec![2]], None, vec![1, 3])]
	#[case::duplicates(vec![vec![1, 1, 2]], None, vec![3])]
	#[case::repeated_overlap(vec![vec![1, 2], vec![2, 3]], None, vec![1, 3])]
	#[case::repeated_disjoint(vec![vec![1], vec![2]], None, vec![1, 2, 3])]
	#[case::empty(vec![vec![]], None, vec![1, 2, 3])]
	#[case::empty_first(vec![vec![], vec![1]], None, vec![1, 2, 3])]
	#[case::empty_last(vec![vec![1], vec![]], None, vec![1, 2, 3])]
	#[case::equality(vec![vec![1, 2, 3]], Some(2), vec![1, 3])]
	#[tokio::test]
	async fn test_delete_builder_where_in_sqlite_execution(
		#[future] builder_probe: Arc<SqliteBackend>,
		#[case] sets: Vec<Vec<i64>>,
		#[case] equal_id: Option<i64>,
		#[case] expected_remaining: Vec<i64>,
	) {
		// Arrange
		let backend = builder_probe.await;
		let mut builder = DeleteBuilder::new(backend.clone(), "builder_probe");
		for values in sets {
			builder = builder.where_in("id", values.into_iter().map(QueryValue::Int).collect());
		}
		if let Some(id) = equal_id {
			builder = builder.where_eq("id", id);
		}

		// Act
		let result = builder.execute().await.expect("DELETE must execute");

		// Assert
		assert_eq!(result.rows_affected, 3 - expected_remaining.len() as u64);
		let select = Query::select()
			.column(BuilderProbe::Id)
			.from(BuilderProbe::Table.into_iden())
			.order_by(BuilderProbe::Id, Order::Asc)
			.to_string(SqliteQueryBuilder);
		let rows = backend
			.fetch_all(&select, Vec::new())
			.await
			.expect("remaining rows must be fetched");
		let remaining: Vec<i64> = rows
			.iter()
			.map(|row| row.get("id").expect("id must be an integer"))
			.collect();
		assert_eq!(remaining, expected_remaining);
	}
}

#[cfg(feature = "sqlite")]
mod insert_builder_sqlite_tests {
	use super::*;
	use reinhardt_db::backends::dialect::SqliteBackend;
	use reinhardt_query::prelude::{
		ColumnDef, Iden, IntoIden, Query, QueryStatementBuilder, SqliteQueryBuilder,
	};
	use rstest::fixture;
	use sqlx::sqlite::SqlitePoolOptions;

	#[derive(Debug, Iden)]
	enum InsertProbe {
		Table,
		Id,
		Created,
		Touched,
		Payload,
	}

	// Each fixture owns an isolated memory database; dropping its pool releases it.
	#[fixture]
	async fn insert_probe() -> Arc<SqliteBackend> {
		let pool = SqlitePoolOptions::new()
			.max_connections(1)
			.connect("sqlite::memory:")
			.await
			.expect("in-memory SQLite must connect");
		let backend = Arc::new(SqliteBackend::new(pool));
		let create = Query::create_table()
			.table(InsertProbe::Table.into_iden())
			.col(ColumnDef::new(InsertProbe::Id).integer().primary_key(true))
			.col(ColumnDef::new(InsertProbe::Created).text().not_null(true))
			.col(ColumnDef::new(InsertProbe::Touched).text().not_null(true))
			.col(ColumnDef::new(InsertProbe::Payload).text())
			.to_string(SqliteQueryBuilder);
		backend
			.execute(&create, Vec::new())
			.await
			.expect("probe table must be created");
		backend
	}

	#[rstest]
	#[case::mixed_execute(false, false, false)]
	#[case::mixed_returning(false, true, false)]
	#[case::mixed_upsert_returning(false, true, true)]
	#[case::only_now_execute(true, false, false)]
	#[case::only_now_returning(true, true, false)]
	#[tokio::test]
	async fn test_insert_builder_now_sqlite_execution(
		#[future] insert_probe: Arc<SqliteBackend>,
		#[case] only_now: bool,
		#[case] returning: bool,
		#[case] upsert: bool,
	) {
		// Arrange
		let backend = insert_probe.await;
		let payload = "literal CURRENT_TIMESTAMP '__REINHARDT_NOW__' $9 ?";
		if upsert {
			InsertBuilder::new(backend.clone(), "insert_probe")
				.value("id", 2_i64)
				.value("created", "2000-01-01 00:00:00")
				.value("touched", "2000-01-01 00:00:00")
				.value("payload", "old")
				.execute()
				.await
				.expect("existing row must be inserted");
		}
		let mut builder =
			InsertBuilder::new(backend.clone(), "insert_probe").value("created", QueryValue::Now);
		if !only_now {
			builder = builder.value("id", 2_i64).value("payload", payload);
		}
		builder = builder.value("touched", QueryValue::Now);
		if upsert {
			builder = builder.on_conflict(
				OnConflictClause::columns(vec!["id"])
					.do_update(vec!["created", "touched", "payload"]),
			);
		}
		let before = chrono::Utc::now().timestamp();

		// Act
		let (row, rows_affected) = if returning {
			let row = builder
				.returning(vec!["id", "created", "touched", "payload"])
				.fetch_one()
				.await
				.expect("INSERT RETURNING must execute");
			(row, None)
		} else {
			let result = builder.execute().await.expect("INSERT must execute");
			let select = Query::select()
				.columns([
					InsertProbe::Id,
					InsertProbe::Created,
					InsertProbe::Touched,
					InsertProbe::Payload,
				])
				.from(InsertProbe::Table.into_iden())
				.to_string(SqliteQueryBuilder);
			let row = backend
				.fetch_one(&select, Vec::new())
				.await
				.expect("inserted row must be fetched");
			(row, Some(result.rows_affected))
		};
		let after = chrono::Utc::now().timestamp();

		// Assert
		if let Some(rows_affected) = rows_affected {
			assert_eq!(rows_affected, 1);
		}
		assert_eq!(row.get::<i64>("id").unwrap(), if only_now { 1 } else { 2 });
		if only_now {
			assert_eq!(row.data.get("payload"), Some(&QueryValue::Null));
		} else {
			assert_eq!(row.get::<String>("payload").unwrap(), payload);
		}
		let created: String = row.get("created").unwrap();
		let touched: String = row.get("touched").unwrap();
		assert_eq!(created, touched);
		let timestamp = chrono::NaiveDateTime::parse_from_str(&touched, "%Y-%m-%d %H:%M:%S")
			.expect("SQLite must store the database CURRENT_TIMESTAMP format")
			.and_utc()
			.timestamp();
		assert!(timestamp >= before, "stored timestamp predates INSERT");
		assert!(timestamp <= after, "stored timestamp follows INSERT");
	}
}
