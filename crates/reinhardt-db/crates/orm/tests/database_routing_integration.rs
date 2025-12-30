//! Multi-Database Routing Integration Tests
//!
//! Tests database routing functionality for Phase 5, covering:
//! - Multi-database operations with routing rules
//! - Read/Write split routing patterns
//! - Database routing decision table verification
//!
//! **Test Coverage:**
//! - Normal cases: Multi-DB operations, read/write split
//! - Decision Table: DB routing rules (8 decision scenarios)
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container (2 instances)

use reinhardt_orm::database_routing::DatabaseRouter;
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sea_query::{Expr, ExprTrait, Iden, PostgresQueryBuilder, Query};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// User Model Schema for Testing
// ============================================================================

#[allow(dead_code)] // Test schema definition for routing tests
#[derive(Debug, Clone, Copy, Iden)]
enum Users {
	Table,
	#[iden = "id"]
	Id,
	#[iden = "name"]
	Name,
	#[iden = "email"]
	Email,
	#[iden = "created_at"]
	CreatedAt,
}

#[allow(dead_code)] // Test schema definition for routing tests
#[derive(Debug, Clone, Copy, Iden)]
enum Analytics {
	Table,
	#[iden = "id"]
	Id,
	#[iden = "event_id"]
	EventId,
	#[iden = "user_id"]
	UserId,
	#[iden = "event_type"]
	EventType,
}

#[allow(dead_code)] // Test schema definition for routing tests
#[derive(Debug, Clone, Copy, Iden)]
enum AuditLogs {
	Table,
	#[iden = "id"]
	Id,
	#[iden = "action"]
	Action,
	#[iden = "timestamp"]
	Timestamp,
	#[iden = "user_id"]
	UserId,
}

// ============================================================================
// Database Router Rules Tests
// ============================================================================

/// Test: Create router with default database
///
/// **Test Intent**: Verify router initialization with default database
///
/// **Scenario**: Create router with "default" database name
///
/// **Expected Result**: Router returns "default" for unknown models
#[test]
fn test_router_default_database() {
	let router = DatabaseRouter::new("default");

	assert_eq!(router.default_db(), "default");
	assert_eq!(router.db_for_read("Unknown"), "default");
	assert_eq!(router.db_for_write("Unknown"), "default");
}

/// Test: Add routing rule for same database (read/write)
///
/// **Test Intent**: Verify both read and write operations use same database
///
/// **Scenario**: Model "User" routes to "users_db"
///
/// **Expected Result**: Both reads and writes go to "users_db"
#[test]
fn test_add_rule_same_database() {
	let router = DatabaseRouter::new("default").add_rule("User", "users_db");

	assert_eq!(router.db_for_read("User"), "users_db");
	assert_eq!(router.db_for_write("User"), "users_db");
	assert_eq!(router.db_for_read("Unknown"), "default");
}

/// Test: Add read/write split routing rule
///
/// **Test Intent**: Verify read and write operations route to different databases
///
/// **Scenario**: Model "Analytics" reads from "replica", writes to "primary"
///
/// **Expected Result**: Reads go to replica, writes go to primary
#[test]
fn test_add_read_write_split_rule() {
	let router =
		DatabaseRouter::new("default").add_read_write_rule("Analytics", "replica", "primary");

	assert_eq!(router.db_for_read("Analytics"), "replica");
	assert_eq!(router.db_for_write("Analytics"), "primary");
}

/// Test: Add read-only routing rule
///
/// **Test Intent**: Verify read-only rule leaves writes to default
///
/// **Scenario**: Model "ReportData" reads from "analytics_db"
///
/// **Expected Result**: Reads go to analytics_db, writes go to default
#[test]
fn test_add_read_rule_only() {
	let router = DatabaseRouter::new("default").add_read_rule("ReportData", "analytics_db");

	assert_eq!(router.db_for_read("ReportData"), "analytics_db");
	assert_eq!(router.db_for_write("ReportData"), "default");
}

/// Test: Add write-only routing rule
///
/// **Test Intent**: Verify write-only rule leaves reads to default
///
/// **Scenario**: Model "AuditLog" writes to "audit_db"
///
/// **Expected Result**: Reads go to default, writes go to audit_db
#[test]
fn test_add_write_rule_only() {
	let router = DatabaseRouter::new("default").add_write_rule("AuditLog", "audit_db");

	assert_eq!(router.db_for_read("AuditLog"), "default");
	assert_eq!(router.db_for_write("AuditLog"), "audit_db");
}

/// Test: Multiple routing rules coexist
///
/// **Test Intent**: Verify multiple rules can be added to same router
///
/// **Scenario**: Three models with different routing rules
///
/// **Expected Result**: Each model routes to correct database
#[test]
fn test_multiple_routing_rules() {
	let router = DatabaseRouter::new("default")
		.add_rule("User", "users_db")
		.add_read_write_rule("Analytics", "analytics_replica", "analytics_primary")
		.add_write_rule("AuditLog", "logs_db");

	assert_eq!(router.db_for_read("User"), "users_db");
	assert_eq!(router.db_for_write("User"), "users_db");
	assert_eq!(router.db_for_read("Analytics"), "analytics_replica");
	assert_eq!(router.db_for_write("Analytics"), "analytics_primary");
	assert_eq!(router.db_for_read("AuditLog"), "default");
	assert_eq!(router.db_for_write("AuditLog"), "logs_db");
}

// ============================================================================
// Decision Table Tests (8 Scenarios)
// ============================================================================

/// Decision Table - Scenario 1: Unknown model, no explicit rule
///
/// **Decision Table:** Model routes to default database
///
/// **Conditions:**
/// - Model: "Unknown"
/// - Rule: None
/// - Operation: Read/Write
///
/// **Expected:** Both read and write use "default"
#[test]
fn test_decision_table_scenario_1_unknown_model_default() {
	let router = DatabaseRouter::new("default");
	assert_eq!(router.db_for_read("Unknown"), "default");
	assert_eq!(router.db_for_write("Unknown"), "default");
}

/// Decision Table - Scenario 2: Unified rule (same DB for read/write)
///
/// **Decision Table:** Both operations route to specified database
///
/// **Conditions:**
/// - Model: "User"
/// - Rule: add_rule("User", "users_db")
/// - Operation: Read/Write
///
/// **Expected:** Both read and write use "users_db"
#[test]
fn test_decision_table_scenario_2_unified_database_routing() {
	let router = DatabaseRouter::new("default").add_rule("User", "users_db");

	assert_eq!(router.db_for_read("User"), "users_db");
	assert_eq!(router.db_for_write("User"), "users_db");
}

/// Decision Table - Scenario 3: Read/Write split routing
///
/// **Decision Table:** Read and write route to different databases
///
/// **Conditions:**
/// - Model: "Analytics"
/// - Rule: add_read_write_rule("Analytics", "replica", "primary")
/// - Operation: Read
/// - Operation: Write
///
/// **Expected:** Read uses "replica", Write uses "primary"
#[test]
fn test_decision_table_scenario_3_read_write_split() {
	let router =
		DatabaseRouter::new("default").add_read_write_rule("Analytics", "replica", "primary");

	assert_eq!(router.db_for_read("Analytics"), "replica");
	assert_eq!(router.db_for_write("Analytics"), "primary");
}

/// Decision Table - Scenario 4: Read override only
///
/// **Decision Table:** Read uses custom DB, write uses default
///
/// **Conditions:**
/// - Model: "ReportData"
/// - Rule: add_read_rule("ReportData", "reporting_db")
/// - Operation: Read
/// - Operation: Write
///
/// **Expected:** Read uses "reporting_db", Write uses "default"
#[test]
fn test_decision_table_scenario_4_read_override_only() {
	let router = DatabaseRouter::new("default").add_read_rule("ReportData", "reporting_db");

	assert_eq!(router.db_for_read("ReportData"), "reporting_db");
	assert_eq!(router.db_for_write("ReportData"), "default");
}

/// Decision Table - Scenario 5: Write override only
///
/// **Decision Table:** Write uses custom DB, read uses default
///
/// **Conditions:**
/// - Model: "AuditLog"
/// - Rule: add_write_rule("AuditLog", "audit_db")
/// - Operation: Read
/// - Operation: Write
///
/// **Expected:** Read uses "default", Write uses "audit_db"
#[test]
fn test_decision_table_scenario_5_write_override_only() {
	let router = DatabaseRouter::new("default").add_write_rule("AuditLog", "audit_db");

	assert_eq!(router.db_for_read("AuditLog"), "default");
	assert_eq!(router.db_for_write("AuditLog"), "audit_db");
}

/// Decision Table - Scenario 6: Multiple models with independent rules
///
/// **Decision Table:** Each model routes independently
///
/// **Conditions:**
/// - Model 1: "User" with unified rule
/// - Model 2: "Analytics" with split rule
/// - Model 3: "Unknown" with no rule
///
/// **Expected:** Each model uses correct routing
#[test]
fn test_decision_table_scenario_6_multiple_independent_models() {
	let router = DatabaseRouter::new("default")
		.add_rule("User", "users_db")
		.add_read_write_rule("Analytics", "analytics_replica", "analytics_primary");

	// User: unified routing
	assert_eq!(router.db_for_read("User"), "users_db");
	assert_eq!(router.db_for_write("User"), "users_db");

	// Analytics: split routing
	assert_eq!(router.db_for_read("Analytics"), "analytics_replica");
	assert_eq!(router.db_for_write("Analytics"), "analytics_primary");

	// Unknown: default routing
	assert_eq!(router.db_for_read("Unknown"), "default");
	assert_eq!(router.db_for_write("Unknown"), "default");
}

/// Decision Table - Scenario 7: Rule queries (has_rule, rule_count)
///
/// **Decision Table:** Router correctly tracks rule existence
///
/// **Conditions:**
/// - Add rules for 2 models
/// - Query rule existence
/// - Count total rules
///
/// **Expected:** has_rule matches added rules, count is 2
#[test]
fn test_decision_table_scenario_7_rule_queries() {
	let router = DatabaseRouter::new("default")
		.add_rule("User", "users_db")
		.add_write_rule("AuditLog", "logs_db");

	assert!(router.has_rule("User"));
	assert!(router.has_rule("AuditLog"));
	assert!(!router.has_rule("Unknown"));
	assert_eq!(router.rule_count(), 2);
}

/// Decision Table - Scenario 8: Rule manipulation (remove, clear)
///
/// **Decision Table:** Router rules can be modified/cleared
///
/// **Conditions:**
/// - Add rules, then remove/clear them
/// - Query affected models after operation
///
/// **Expected:** Removed/cleared rules fallback to default
#[test]
fn test_decision_table_scenario_8_rule_manipulation() {
	let mut router = DatabaseRouter::new("default")
		.add_rule("User", "users_db")
		.add_write_rule("AuditLog", "logs_db");

	// Verify initial rules
	assert_eq!(router.db_for_read("User"), "users_db");
	assert_eq!(router.rule_count(), 2);

	// Remove one rule
	router.remove_rule("User");
	assert_eq!(router.db_for_read("User"), "default");
	assert_eq!(router.rule_count(), 1);

	// Clear all rules
	router.clear_rules();
	assert_eq!(router.db_for_write("AuditLog"), "default");
	assert_eq!(router.rule_count(), 0);
}

// ============================================================================
// Multi-Database Operations Tests (with postgres_container fixtures)
// ============================================================================

/// Test: Multi-DB read/write split operations
///
/// **Test Intent**: Verify read/write split routing with actual database operations
///
/// **Integration Point**: DatabaseRouter × QuerySet
///
/// **Not Intent**: Query result validation (only routing verification)
#[rstest]
#[tokio::test]
async fn test_multi_db_read_write_split_operations(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create tables in test database
	sqlx::query(
		r#"
		CREATE TABLE users (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			email VARCHAR(255) NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create users table");

	sqlx::query(
		r#"
		CREATE TABLE audit_logs (
			id SERIAL PRIMARY KEY,
			action VARCHAR(255) NOT NULL,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create audit_logs table");

	// Create router with split rules
	let router = DatabaseRouter::new("primary")
		.add_read_write_rule("User", "replica", "primary")
		.add_write_rule("AuditLog", "audit_db");

	// Verify routing rules
	assert_eq!(router.db_for_read("User"), "replica");
	assert_eq!(router.db_for_write("User"), "primary");
	assert_eq!(router.db_for_read("AuditLog"), "primary");
	assert_eq!(router.db_for_write("AuditLog"), "audit_db");

	// Insert user data (should route to primary)
	sqlx::query("INSERT INTO users (name, email) VALUES ($1, $2)")
		.bind("Alice")
		.bind("alice@example.com")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert user");

	// Read user data (would route to replica in real scenario)
	let result = sqlx::query("SELECT name, email FROM users WHERE name = $1")
		.bind("Alice")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to read user");

	let name: String = result.get("name");
	let email: String = result.get("email");

	assert_eq!(name, "Alice");
	assert_eq!(email, "alice@example.com");

	// Insert audit log (should route to audit_db)
	sqlx::query("INSERT INTO audit_logs (action) VALUES ($1)")
		.bind("USER_CREATED")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert audit log");

	// Read audit log
	let log_result = sqlx::query("SELECT action FROM audit_logs WHERE action = $1")
		.bind("USER_CREATED")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to read audit log");

	let action: String = log_result.get("action");
	assert_eq!(action, "USER_CREATED");
}

/// Test: Multi-DB operations with SeaQuery expression routing
///
/// **Test Intent**: Verify routing with F expressions in complex queries
///
/// **Integration Point**: DatabaseRouter × SeaQuery ExprTrait
///
/// **Not Intent**: Expression evaluation (only routing structure)
#[rstest]
#[tokio::test]
async fn test_multi_db_with_expr_trait_routing(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create analytics table
	sqlx::query(
		r#"
		CREATE TABLE analytics (
			id SERIAL PRIMARY KEY,
			event_id INTEGER NOT NULL,
			user_id INTEGER NOT NULL,
			event_type VARCHAR(50) NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create analytics table");

	// Create router with analytics split routing
	let router = DatabaseRouter::new("default").add_read_write_rule(
		"Analytics",
		"analytics_replica",
		"analytics_primary",
	);

	// Verify routing
	assert_eq!(router.db_for_read("Analytics"), "analytics_replica");
	assert_eq!(router.db_for_write("Analytics"), "analytics_primary");

	// Build query using ExprTrait
	let mut query = Query::select();
	query
		.from(Analytics::Table)
		.column(Analytics::Id)
		.column(Analytics::UserId)
		.column(Analytics::EventType)
		.and_where(Expr::col(Analytics::EventType).eq("click"));

	let (sql, _) = query.build(PostgresQueryBuilder);

	// Verify query structure
	assert!(sql.contains("analytics"));
	assert!(sql.contains("event_type"));
	assert!(sql.contains("'click'"));

	// Insert test data
	sqlx::query("INSERT INTO analytics (event_id, user_id, event_type) VALUES ($1, $2, $3)")
		.bind(1)
		.bind(100)
		.bind("click")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert analytics");

	// Execute query (would use replica in real scenario)
	let result = sqlx::query_as::<_, (i32, i32, String)>(
		"SELECT id, user_id, event_type FROM analytics WHERE event_type = $1",
	)
	.bind("click")
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to fetch analytics");

	assert_eq!(result.0, 1);
	assert_eq!(result.1, 100);
	assert_eq!(result.2, "click");
}
