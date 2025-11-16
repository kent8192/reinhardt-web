//! Integration tests for DatabaseAuditLogger
//!
//! These tests verify the DatabaseAuditLogger implementation using
//! TestContainers and PostgreSQL, testing log insertion, querying,
//! and counting operations.

use chrono::Utc;
use reinhardt_orm::DatabaseConnection;
use reinhardt_panel::audit::{
	AuditAction, AuditLog, AuditLogQuery, AuditLogger, DatabaseAuditLogger,
};
use reinhardt_panel::AdminDatabase;
use rstest::*;
use serde_json::json;
use serial_test::serial;
use std::net::IpAddr;
use std::sync::Arc;
use testcontainers::{core::WaitFor, runners::AsyncRunner, GenericImage, ImageExt};

/// Fixture providing test database with audit_logs table
#[fixture]
async fn setup_test_db() -> (
	testcontainers::ContainerAsync<GenericImage>,
	DatabaseAuditLogger,
) {
	// Start PostgreSQL container
	let postgres = GenericImage::new("postgres", "16-alpine")
		.with_wait_for(WaitFor::message_on_stderr(
			"database system is ready to accept connections",
		))
		.with_env_var("POSTGRES_PASSWORD", "test")
		.with_env_var("POSTGRES_DB", "test_db")
		.start()
		.await
		.expect("Failed to start PostgreSQL container");

	let port = postgres
		.get_host_port_ipv4(5432)
		.await
		.expect("Failed to get PostgreSQL port");

	let database_url = format!("postgres://postgres:test@localhost:{}/test_db", port);

	// Create connection using DatabaseConnection
	let conn = DatabaseConnection::connect(&database_url)
		.await
		.expect("Failed to connect to database");

	// Create audit_logs table
	conn.execute(
		"CREATE TABLE IF NOT EXISTS audit_logs (
            id SERIAL PRIMARY KEY,
            user_id TEXT NOT NULL,
            model_name TEXT NOT NULL,
            object_id TEXT NOT NULL,
            action TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            changes TEXT,
            ip_address TEXT,
            user_agent TEXT
        )",
		vec![],
	)
	.await
	.expect("Failed to create audit_logs table");

	// Create AdminDatabase and DatabaseAuditLogger
	let admin_db = Arc::new(AdminDatabase::new(conn));
	let logger = DatabaseAuditLogger::new(admin_db, "audit_logs".to_string());

	(postgres, logger)
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_database_audit_logger_log(
	#[future] setup_test_db: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = setup_test_db.await;

	// Create AuditLog
	let log = AuditLog::builder()
		.user_id("admin_user".to_string())
		.model_name("User".to_string())
		.object_id("123".to_string())
		.action(AuditAction::Create)
		.changes(json!({"name": "Alice", "email": "alice@example.com"}))
		.ip_address("192.168.1.1".parse::<IpAddr>().unwrap())
		.user_agent("Mozilla/5.0".to_string())
		.build();

	// Log entry
	let result = logger.log(log).await;
	assert!(result.is_ok(), "Log insertion should succeed");

	let inserted_log = result.unwrap();

	// Verify ID is set after insertion
	assert!(
		inserted_log.id().is_some(),
		"ID should be set after insertion"
	);
	assert_eq!(inserted_log.user_id(), "admin_user");
	assert_eq!(inserted_log.model_name(), "User");
	assert_eq!(inserted_log.object_id(), "123");
	assert_eq!(inserted_log.action(), AuditAction::Create);
	assert!(inserted_log.changes().is_some());
	assert!(inserted_log.ip_address().is_some());
	assert_eq!(inserted_log.user_agent(), Some("Mozilla/5.0"));
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_database_audit_logger_log_minimal(
	#[future] setup_test_db: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = setup_test_db.await;

	// Create AuditLog with minimal fields (no changes, ip_address, user_agent)
	let log = AuditLog::builder()
		.user_id("test_user".to_string())
		.model_name("Article".to_string())
		.object_id("456".to_string())
		.action(AuditAction::View)
		.build();

	// Log entry
	let result = logger.log(log).await;
	assert!(
		result.is_ok(),
		"Log insertion with minimal fields should succeed"
	);

	let inserted_log = result.unwrap();
	assert!(inserted_log.id().is_some());
	assert_eq!(inserted_log.user_id(), "test_user");
	assert!(inserted_log.changes().is_none());
	assert!(inserted_log.ip_address().is_none());
	assert_eq!(inserted_log.user_agent(), None);
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_database_audit_logger_query_by_user_id(
	#[future] setup_test_db: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = setup_test_db.await;

	// Insert multiple logs
	let log1 = AuditLog::builder()
		.user_id("user1".to_string())
		.model_name("Post".to_string())
		.object_id("1".to_string())
		.action(AuditAction::Create)
		.build();

	let log2 = AuditLog::builder()
		.user_id("user1".to_string())
		.model_name("Post".to_string())
		.object_id("2".to_string())
		.action(AuditAction::Update)
		.build();

	let log3 = AuditLog::builder()
		.user_id("user2".to_string())
		.model_name("Post".to_string())
		.object_id("3".to_string())
		.action(AuditAction::Delete)
		.build();

	logger.log(log1).await.unwrap();
	logger.log(log2).await.unwrap();
	logger.log(log3).await.unwrap();

	// Query by user_id
	let query = AuditLogQuery::builder()
		.user_id("user1".to_string())
		.build();

	let results = logger.query(&query).await.expect("Query should succeed");
	assert_eq!(results.len(), 2, "Should return 2 logs for user1");

	for log in &results {
		assert_eq!(log.user_id(), "user1");
	}
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_database_audit_logger_query_by_model_name(
	#[future] setup_test_db: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = setup_test_db.await;

	// Insert logs for different models
	let log1 = AuditLog::builder()
		.user_id("admin".to_string())
		.model_name("User".to_string())
		.object_id("1".to_string())
		.action(AuditAction::Create)
		.build();

	let log2 = AuditLog::builder()
		.user_id("admin".to_string())
		.model_name("Article".to_string())
		.object_id("1".to_string())
		.action(AuditAction::Create)
		.build();

	logger.log(log1).await.unwrap();
	logger.log(log2).await.unwrap();

	// Query by model_name
	let query = AuditLogQuery::builder()
		.model_name("User".to_string())
		.build();

	let results = logger.query(&query).await.expect("Query should succeed");
	assert_eq!(results.len(), 1, "Should return 1 log for User model");
	assert_eq!(results[0].model_name(), "User");
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_database_audit_logger_query_by_action(
	#[future] setup_test_db: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = setup_test_db.await;

	// Insert logs with different actions
	let log1 = AuditLog::builder()
		.user_id("admin".to_string())
		.model_name("Post".to_string())
		.object_id("1".to_string())
		.action(AuditAction::Create)
		.build();

	let log2 = AuditLog::builder()
		.user_id("admin".to_string())
		.model_name("Post".to_string())
		.object_id("2".to_string())
		.action(AuditAction::Update)
		.build();

	let log3 = AuditLog::builder()
		.user_id("admin".to_string())
		.model_name("Post".to_string())
		.object_id("1".to_string())
		.action(AuditAction::Delete)
		.build();

	logger.log(log1).await.unwrap();
	logger.log(log2).await.unwrap();
	logger.log(log3).await.unwrap();

	// Query by action
	let query = AuditLogQuery::builder().action(AuditAction::Update).build();

	let results = logger.query(&query).await.expect("Query should succeed");
	assert_eq!(results.len(), 1, "Should return 1 update action");
	assert_eq!(results[0].action(), AuditAction::Update);
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_database_audit_logger_query_by_timestamp_range(
	#[future] setup_test_db: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = setup_test_db.await;

	// Insert logs at different times
	let now = Utc::now();
	let past = now - chrono::Duration::hours(2);
	let future = now + chrono::Duration::hours(2);

	let log = AuditLog::builder()
		.user_id("admin".to_string())
		.model_name("Post".to_string())
		.object_id("1".to_string())
		.action(AuditAction::Create)
		.build();

	logger.log(log).await.unwrap();

	// Query by timestamp range
	let query = AuditLogQuery::builder()
		.start_date(past)
		.end_date(future)
		.build();

	let results = logger.query(&query).await.expect("Query should succeed");
	assert_eq!(results.len(), 1, "Should return 1 log within range");

	// Query with narrow range (should return 0)
	let query_empty = AuditLogQuery::builder()
		.start_date(past - chrono::Duration::hours(10))
		.end_date(past - chrono::Duration::hours(5))
		.build();

	let results_empty = logger
		.query(&query_empty)
		.await
		.expect("Query should succeed");
	assert_eq!(results_empty.len(), 0, "Should return 0 logs outside range");
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_database_audit_logger_query_pagination(
	#[future] setup_test_db: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = setup_test_db.await;

	// Insert multiple logs
	for i in 1..=10 {
		let log = AuditLog::builder()
			.user_id("admin".to_string())
			.model_name("Post".to_string())
			.object_id(i.to_string())
			.action(AuditAction::Create)
			.build();
		logger.log(log).await.unwrap();
	}

	// Query first page (limit 5)
	let query_page1 = AuditLogQuery::builder().limit(5).offset(0).build();

	let results_page1 = logger
		.query(&query_page1)
		.await
		.expect("Query should succeed");
	assert_eq!(results_page1.len(), 5, "Should return 5 logs");

	// Query second page (offset 5, limit 5)
	let query_page2 = AuditLogQuery::builder().limit(5).offset(5).build();

	let results_page2 = logger
		.query(&query_page2)
		.await
		.expect("Query should succeed");
	assert_eq!(results_page2.len(), 5, "Should return 5 logs");

	// Verify no overlap
	let ids_page1: Vec<_> = results_page1.iter().filter_map(|l| l.id()).collect();
	let ids_page2: Vec<_> = results_page2.iter().filter_map(|l| l.id()).collect();

	for id in &ids_page1 {
		assert!(!ids_page2.contains(id), "Pages should not overlap");
	}
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_database_audit_logger_query_ordering(
	#[future] setup_test_db: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = setup_test_db.await;

	// Insert logs with delays to ensure different timestamps
	for i in 1..=3 {
		let log = AuditLog::builder()
			.user_id("admin".to_string())
			.model_name("Post".to_string())
			.object_id(i.to_string())
			.action(AuditAction::Create)
			.build();
		logger.log(log).await.unwrap();
		tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
	}

	// Query all logs
	let query = AuditLogQuery::builder().build();

	let results = logger.query(&query).await.expect("Query should succeed");
	assert_eq!(results.len(), 3);

	// Verify descending order (most recent first)
	for i in 0..results.len() - 1 {
		assert!(
			results[i].timestamp() >= results[i + 1].timestamp(),
			"Results should be ordered by timestamp DESC"
		);
	}
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_database_audit_logger_count(
	#[future] setup_test_db: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = setup_test_db.await;

	// Insert multiple logs
	for i in 1..=5 {
		let log = AuditLog::builder()
			.user_id("admin".to_string())
			.model_name("Post".to_string())
			.object_id(i.to_string())
			.action(AuditAction::Create)
			.build();
		logger.log(log).await.unwrap();
	}

	// Count all logs
	let query = AuditLogQuery::builder().build();
	let count = logger.count(&query).await.expect("Count should succeed");
	assert_eq!(count, 5, "Should count 5 logs");
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_database_audit_logger_count_with_filters(
	#[future] setup_test_db: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = setup_test_db.await;

	// Insert logs for different users
	for i in 1..=3 {
		let log = AuditLog::builder()
			.user_id("user1".to_string())
			.model_name("Post".to_string())
			.object_id(i.to_string())
			.action(AuditAction::Create)
			.build();
		logger.log(log).await.unwrap();
	}

	for i in 4..=7 {
		let log = AuditLog::builder()
			.user_id("user2".to_string())
			.model_name("Post".to_string())
			.object_id(i.to_string())
			.action(AuditAction::Create)
			.build();
		logger.log(log).await.unwrap();
	}

	// Count for user1
	let query_user1 = AuditLogQuery::builder()
		.user_id("user1".to_string())
		.build();

	let count_user1 = logger
		.count(&query_user1)
		.await
		.expect("Count should succeed");
	assert_eq!(count_user1, 3, "Should count 3 logs for user1");

	// Count for user2
	let query_user2 = AuditLogQuery::builder()
		.user_id("user2".to_string())
		.build();

	let count_user2 = logger
		.count(&query_user2)
		.await
		.expect("Count should succeed");
	assert_eq!(count_user2, 4, "Should count 4 logs for user2");

	// Count all
	let query_all = AuditLogQuery::builder().build();
	let count_all = logger
		.count(&query_all)
		.await
		.expect("Count should succeed");
	assert_eq!(count_all, 7, "Should count 7 total logs");
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_database_audit_logger_count_empty(
	#[future] setup_test_db: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = setup_test_db.await;

	// Count when no logs exist
	let query = AuditLogQuery::builder().build();
	let count = logger.count(&query).await.expect("Count should succeed");
	assert_eq!(count, 0, "Should count 0 logs when empty");
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_database_audit_logger_query_combined_filters(
	#[future] setup_test_db: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = setup_test_db.await;

	// Insert diverse logs
	let log1 = AuditLog::builder()
		.user_id("user1".to_string())
		.model_name("User".to_string())
		.object_id("1".to_string())
		.action(AuditAction::Create)
		.build();

	let log2 = AuditLog::builder()
		.user_id("user1".to_string())
		.model_name("Article".to_string())
		.object_id("2".to_string())
		.action(AuditAction::Update)
		.build();

	let log3 = AuditLog::builder()
		.user_id("user2".to_string())
		.model_name("Article".to_string())
		.object_id("3".to_string())
		.action(AuditAction::Update)
		.build();

	logger.log(log1).await.unwrap();
	logger.log(log2).await.unwrap();
	logger.log(log3).await.unwrap();

	// Query with combined filters: user_id + model_name + action
	let query = AuditLogQuery::builder()
		.user_id("user1".to_string())
		.model_name("Article".to_string())
		.action(AuditAction::Update)
		.build();

	let results = logger.query(&query).await.expect("Query should succeed");
	assert_eq!(results.len(), 1, "Should return 1 log matching all filters");
	assert_eq!(results[0].user_id(), "user1");
	assert_eq!(results[0].model_name(), "Article");
	assert_eq!(results[0].action(), AuditAction::Update);
}
