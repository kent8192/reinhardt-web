//! Admin Audit Integration Tests
//!
//! Tests for production scenarios using DatabaseAuditLogger.
//! Uses PostgreSQL container to test actual database logging,
//! querying, and aggregation.

use chrono::{Duration, Utc};
use reinhardt_orm::DatabaseConnection;
use reinhardt_panel::audit::{
	AuditAction, AuditLog, AuditLogQuery, AuditLogger, DatabaseAuditLogger,
};
use reinhardt_panel::AdminDatabase;
use reinhardt_test::fixtures::testcontainers::postgres_container;
use rstest::{fixture, rstest};
use serde_json::json;
use serial_test::serial;
use std::net::IpAddr;
use std::sync::Arc;
use testcontainers::GenericImage;

/// Fixture providing PostgreSQL container with audit_logs table and DatabaseAuditLogger
///
/// Test intent: Provide a ready-to-use DatabaseAuditLogger with PostgreSQL backend
/// for testing audit functionality in a production-like environment.
///
/// This fixture chains from the standard postgres_container fixture and:
/// 1. Creates audit_logs table with proper schema
/// 2. Initializes AdminDatabase connection
/// 3. Returns DatabaseAuditLogger instance
///
/// The audit_logs table schema includes:
/// - id (SERIAL PRIMARY KEY)
/// - user_id, model_name, object_id, action, timestamp (required fields)
/// - changes, ip_address, user_agent (optional fields)
#[fixture]
async fn database_audit_logger(
	#[future] postgres_container: (
		testcontainers::ContainerAsync<GenericImage>,
		Arc<sqlx::PgPool>,
		u16,
		String,
	),
) -> (
	testcontainers::ContainerAsync<GenericImage>,
	DatabaseAuditLogger,
) {
	let (container, _pool, _port, database_url) = postgres_container.await;

	// Create DatabaseConnection with smaller pool size for tests
	let conn = DatabaseConnection::connect_with_pool_size(&database_url, Some(5))
		.await
		.expect("Failed to create DatabaseConnection");

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

	(container, logger)
}

/// Test intent: Verify DatabaseAuditLogger correctly logs Create actions with all optional fields
///
/// This test ensures that:
/// 1. DatabaseAuditLogger can successfully log a Create action to PostgreSQL
/// 2. All fields (required and optional) are correctly persisted
/// 3. The logged entry is returned with an assigned database ID
/// 4. All field values match the input data (user_id, model_name, object_id, action, changes, ip_address, user_agent)
///
/// Background: Audit logging is critical for compliance and security monitoring in admin panels.
/// Create actions represent the initial creation of entities and must capture all relevant metadata.
#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_admin_audit_create_action(
	#[future] database_audit_logger: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = database_audit_logger.await;

	// Create log for Create action
	let changes = json!({
		"name": "John Doe",
		"email": "john@example.com",
		"role": "admin"
	});

	let log = AuditLog::builder()
		.user_id("admin_user".to_string())
		.model_name("User".to_string())
		.object_id("123".to_string())
		.action(AuditAction::Create)
		.changes(changes.clone())
		.ip_address("192.168.1.100".parse::<IpAddr>().unwrap())
		.user_agent("Mozilla/5.0 (Admin Client)".to_string())
		.build();

	// Log the record
	let result = logger.log(log).await;
	assert!(result.is_ok(), "Create action logging should succeed");

	let inserted_log = result.unwrap();

	// Verify audit log contents
	assert!(inserted_log.id().is_some(), "ID should be assigned");
	assert_eq!(inserted_log.user_id(), "admin_user");
	assert_eq!(inserted_log.model_name(), "User");
	assert_eq!(inserted_log.object_id(), "123");
	assert_eq!(inserted_log.action(), AuditAction::Create);
	assert_eq!(inserted_log.changes(), Some(&changes));
	assert_eq!(
		inserted_log.ip_address(),
		Some("192.168.1.100".parse::<IpAddr>().unwrap())
	);
	assert_eq!(
		inserted_log.user_agent(),
		Some("Mozilla/5.0 (Admin Client)")
	);
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_admin_audit_update_action(
	#[future] database_audit_logger: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = database_audit_logger.await;

	// Create log for Update action (record before/after values)
	let changes = json!({
		"email": {
			"old": "old@example.com",
			"new": "new@example.com"
		},
		"role": {
			"old": "user",
			"new": "admin"
		}
	});

	let log = AuditLog::builder()
		.user_id("admin_user".to_string())
		.model_name("User".to_string())
		.object_id("456".to_string())
		.action(AuditAction::Update)
		.changes(changes.clone())
		.build();

	// Log the record
	let result = logger.log(log).await;
	assert!(result.is_ok(), "Update action logging should succeed");

	let inserted_log = result.unwrap();

	// Verify audit log
	assert_eq!(inserted_log.action(), AuditAction::Update);
	assert_eq!(inserted_log.object_id(), "456");
	assert_eq!(inserted_log.changes(), Some(&changes));
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_admin_audit_delete_action(
	#[future] database_audit_logger: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = database_audit_logger.await;

	// Create log for Delete action (record deleted data information)
	let changes = json!({
		"deleted_record": {
			"name": "Jane Smith",
			"email": "jane@example.com"
		}
	});

	let log = AuditLog::builder()
		.user_id("admin_user".to_string())
		.model_name("User".to_string())
		.object_id("789".to_string())
		.action(AuditAction::Delete)
		.changes(changes.clone())
		.build();

	// Log the record
	let result = logger.log(log).await;
	assert!(result.is_ok(), "Delete action logging should succeed");

	let inserted_log = result.unwrap();

	// Verify audit log
	assert_eq!(inserted_log.action(), AuditAction::Delete);
	assert_eq!(inserted_log.object_id(), "789");
	assert_eq!(inserted_log.changes(), Some(&changes));
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_admin_audit_query_by_user(
	#[future] database_audit_logger: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = database_audit_logger.await;

	// Record logs for multiple users
	for i in 1..=5 {
		let user_id = if i % 2 == 0 { "user_a" } else { "user_b" };
		let log = AuditLog::builder()
			.user_id(user_id.to_string())
			.model_name("Article".to_string())
			.object_id(i.to_string())
			.action(AuditAction::Create)
			.build();
		logger.log(log).await.unwrap();
	}

	// Query by user_a
	let query = AuditLogQuery::builder()
		.user_id("user_a".to_string())
		.build();

	let results = logger.query(&query).await.expect("Query should succeed");

	// Verify only user_a's logs are retrieved
	assert_eq!(results.len(), 2, "Should return 2 logs for user_a");
	for log in &results {
		assert_eq!(log.user_id(), "user_a");
	}
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_admin_audit_query_by_model(
	#[future] database_audit_logger: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = database_audit_logger.await;

	// Record logs for different models
	let models = ["User", "Article", "Comment"];
	for (i, model) in models.iter().enumerate() {
		let log = AuditLog::builder()
			.user_id("admin".to_string())
			.model_name(model.to_string())
			.object_id((i + 1).to_string())
			.action(AuditAction::Create)
			.build();
		logger.log(log).await.unwrap();
	}

	// Query by Article model
	let query = AuditLogQuery::builder()
		.model_name("Article".to_string())
		.build();

	let results = logger.query(&query).await.expect("Query should succeed");

	// Verify only Article logs are retrieved
	assert_eq!(results.len(), 1, "Should return 1 log for Article model");
	assert_eq!(results[0].model_name(), "Article");
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_admin_audit_query_by_date_range(
	#[future] database_audit_logger: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = database_audit_logger.await;

	// Record logs with past, present, and future timestamps
	let now = Utc::now();
	let past = now - Duration::hours(2);
	let future = now + Duration::hours(2);

	let log_now = AuditLog::builder()
		.user_id("admin".to_string())
		.model_name("Post".to_string())
		.object_id("1".to_string())
		.action(AuditAction::Create)
		.build();

	logger.log(log_now).await.unwrap();

	// Query by date range (from past to future - included)
	let query_include = AuditLogQuery::builder()
		.start_date(past)
		.end_date(future)
		.build();

	let results_include = logger
		.query(&query_include)
		.await
		.expect("Query should succeed");

	assert_eq!(
		results_include.len(),
		1,
		"Should return 1 log within date range"
	);

	// Query by date range (outside range - not included)
	let query_exclude = AuditLogQuery::builder()
		.start_date(past - Duration::hours(10))
		.end_date(past - Duration::hours(5))
		.build();

	let results_exclude = logger
		.query(&query_exclude)
		.await
		.expect("Query should succeed");

	assert_eq!(
		results_exclude.len(),
		0,
		"Should return 0 logs outside date range"
	);
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_admin_audit_count(
	#[future] database_audit_logger: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = database_audit_logger.await;

	// Record multiple logs
	for i in 1..=10 {
		let log = AuditLog::builder()
			.user_id("admin".to_string())
			.model_name("Product".to_string())
			.object_id(i.to_string())
			.action(AuditAction::Create)
			.build();
		logger.log(log).await.unwrap();
	}

	// Count all logs
	let query_all = AuditLogQuery::builder().build();
	let count_all = logger
		.count(&query_all)
		.await
		.expect("Count should succeed");

	assert_eq!(count_all, 10, "Should count 10 total logs");
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_admin_audit_complex_query(
	#[future] database_audit_logger: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = database_audit_logger.await;

	// Record diverse logs
	let scenarios = vec![
		("user1", "User", "1", AuditAction::Create),
		("user1", "User", "1", AuditAction::Update),
		("user1", "Article", "1", AuditAction::Create),
		("user2", "User", "2", AuditAction::Create),
		("user2", "Article", "2", AuditAction::Delete),
	];

	for (user, model, obj_id, action) in scenarios {
		let log = AuditLog::builder()
			.user_id(user.to_string())
			.model_name(model.to_string())
			.object_id(obj_id.to_string())
			.action(action)
			.build();
		logger.log(log).await.unwrap();
	}

	// Composite query: user1 + User model + Update action
	let query = AuditLogQuery::builder()
		.user_id("user1".to_string())
		.model_name("User".to_string())
		.action(AuditAction::Update)
		.build();

	let results = logger.query(&query).await.expect("Query should succeed");

	assert_eq!(results.len(), 1, "Should return 1 log matching all filters");
	assert_eq!(results[0].user_id(), "user1");
	assert_eq!(results[0].model_name(), "User");
	assert_eq!(results[0].action(), AuditAction::Update);
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_admin_audit_pagination(
	#[future] database_audit_logger: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = database_audit_logger.await;

	// Record 20 logs
	for i in 1..=20 {
		let log = AuditLog::builder()
			.user_id("admin".to_string())
			.model_name("Item".to_string())
			.object_id(i.to_string())
			.action(AuditAction::Create)
			.build();
		logger.log(log).await.unwrap();
		// Ensure timestamp differences
		tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
	}

	// Page 1 (first 10 records)
	let query_page1 = AuditLogQuery::builder().limit(10).offset(0).build();

	let results_page1 = logger
		.query(&query_page1)
		.await
		.expect("Query should succeed");

	assert_eq!(results_page1.len(), 10, "Should return 10 logs on page 1");

	// Page 2 (next 10 records)
	let query_page2 = AuditLogQuery::builder().limit(10).offset(10).build();

	let results_page2 = logger
		.query(&query_page2)
		.await
		.expect("Query should succeed");

	assert_eq!(results_page2.len(), 10, "Should return 10 logs on page 2");

	// Verify logs don't overlap between pages
	let ids_page1: Vec<_> = results_page1.iter().filter_map(|l| l.id()).collect();
	let ids_page2: Vec<_> = results_page2.iter().filter_map(|l| l.id()).collect();

	for id in &ids_page1 {
		assert!(!ids_page2.contains(id), "Pages should not overlap");
	}
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_admin_audit_action_variety(
	#[future] database_audit_logger: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = database_audit_logger.await;

	// Record logs for various action types
	let actions = vec![
		AuditAction::Create,
		AuditAction::Update,
		AuditAction::Delete,
		AuditAction::View,
		AuditAction::BulkDelete,
		AuditAction::Export,
		AuditAction::Import,
	];

	for (i, action) in actions.iter().enumerate() {
		let log = AuditLog::builder()
			.user_id("admin".to_string())
			.model_name("Resource".to_string())
			.object_id((i + 1).to_string())
			.action(*action)
			.build();
		logger.log(log).await.unwrap();
	}

	// Query and verify for each action type
	for action in actions {
		let query = AuditLogQuery::builder().action(action).build();

		let results = logger.query(&query).await.expect("Query should succeed");

		assert_eq!(results.len(), 1, "Should return 1 log for {:?}", action);
		assert_eq!(results[0].action(), action);
	}
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_admin_audit_bulk_operations(
	#[future] database_audit_logger: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = database_audit_logger.await;

	// Log for BulkDelete action (bulk deletion of multiple records)
	let changes = json!({
		"deleted_count": 5,
		"deleted_ids": ["1", "2", "3", "4", "5"]
	});

	let log = AuditLog::builder()
		.user_id("admin".to_string())
		.model_name("User".to_string())
		.object_id("bulk_operation_1".to_string())
		.action(AuditAction::BulkDelete)
		.changes(changes.clone())
		.build();

	let result = logger.log(log).await;
	assert!(result.is_ok(), "Bulk delete logging should succeed");

	let inserted_log = result.unwrap();

	// Verify BulkDelete log
	assert_eq!(inserted_log.action(), AuditAction::BulkDelete);
	assert_eq!(inserted_log.changes(), Some(&changes));

	// Query by BulkDelete action
	let query = AuditLogQuery::builder()
		.action(AuditAction::BulkDelete)
		.build();

	let results = logger.query(&query).await.expect("Query should succeed");

	assert_eq!(results.len(), 1, "Should return 1 bulk delete log");
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_admin_audit_export_import_actions(
	#[future] database_audit_logger: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = database_audit_logger.await;

	// Log for Export action
	let export_changes = json!({
		"format": "csv",
		"record_count": 100,
		"exported_at": Utc::now().to_rfc3339()
	});

	let export_log = AuditLog::builder()
		.user_id("admin".to_string())
		.model_name("User".to_string())
		.object_id("export_1".to_string())
		.action(AuditAction::Export)
		.changes(export_changes)
		.build();

	logger.log(export_log).await.unwrap();

	// Log for Import action
	let import_changes = json!({
		"format": "json",
		"record_count": 50,
		"imported_at": Utc::now().to_rfc3339()
	});

	let import_log = AuditLog::builder()
		.user_id("admin".to_string())
		.model_name("User".to_string())
		.object_id("import_1".to_string())
		.action(AuditAction::Import)
		.changes(import_changes)
		.build();

	logger.log(import_log).await.unwrap();

	// Query separately for Export and Import actions
	let export_query = AuditLogQuery::builder().action(AuditAction::Export).build();

	let export_results = logger
		.query(&export_query)
		.await
		.expect("Export query should succeed");

	assert_eq!(export_results.len(), 1, "Should return 1 export log");

	let import_query = AuditLogQuery::builder().action(AuditAction::Import).build();

	let import_results = logger
		.query(&import_query)
		.await
		.expect("Import query should succeed");

	assert_eq!(import_results.len(), 1, "Should return 1 import log");
}

#[rstest]
#[tokio::test]
#[serial(admin_audit)]
async fn test_admin_audit_ordering(
	#[future] database_audit_logger: (
		testcontainers::ContainerAsync<GenericImage>,
		DatabaseAuditLogger,
	),
) {
	let (_container, logger) = database_audit_logger.await;

	// Record logs with different timestamps
	for i in 1..=5 {
		let log = AuditLog::builder()
			.user_id("admin".to_string())
			.model_name("Task".to_string())
			.object_id(i.to_string())
			.action(AuditAction::Create)
			.build();
		logger.log(log).await.unwrap();
		// Ensure timestamp differences
		tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
	}

	// Query all logs
	let query = AuditLogQuery::builder().build();

	let results = logger.query(&query).await.expect("Query should succeed");

	// Verify descending order (newest first)
	for i in 0..results.len() - 1 {
		assert!(
			results[i].timestamp() >= results[i + 1].timestamp(),
			"Results should be ordered by timestamp DESC"
		);
	}
}
