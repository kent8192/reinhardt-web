//! CTE (Common Table Expression) Integration Tests
//!
//! Tests comprehensive CTE functionality covering:
//! - Basic WITH clause (non-recursive) via QuerySet API
//! - Recursive CTEs via QuerySet API
//! - Multiple CTEs in single query
//! - CTE SQL generation
//! - Materialization hints
//! - CTE column specifications
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container
//!
//! **Test Data Schema:**
//! - employees(id SERIAL PRIMARY KEY, name TEXT NOT NULL, manager_id INT, salary BIGINT)

use reinhardt_orm::Model;
use reinhardt_orm::cte::{CTE, CTECollection};
use reinhardt_orm::manager::reinitialize_database;
use reinhardt_orm::query::{Filter, FilterOperator, FilterValue, QuerySet};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use serde::{Deserialize, Serialize};
use serial_test::serial;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Test Models
// ============================================================================

/// Employee model for CTE tests
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Employee {
	id: Option<i32>,
	name: String,
	manager_id: Option<i32>,
	salary: i64,
}

reinhardt_test::impl_test_model!(Employee, i32, "employees", "test");

// ============================================================================
// Fixtures
// ============================================================================

#[fixture]
async fn cte_test_db(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) -> (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String) {
	let (container, pool, port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();
	setup_test_data(pool.as_ref()).await;
	(container, pool, port, url)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create test table and insert test data
async fn setup_test_data(pool: &PgPool) {
	// Create employees table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS employees (
			id SERIAL PRIMARY KEY,
			name TEXT NOT NULL,
			manager_id INT,
			salary BIGINT NOT NULL
		)",
	)
	.execute(pool)
	.await
	.expect("Failed to create employees table");

	// Insert employees data
	// CEO (no manager)
	sqlx::query("INSERT INTO employees (name, manager_id, salary) VALUES ($1, $2, $3)")
		.bind("Alice (CEO)")
		.bind::<Option<i32>>(None)
		.bind(5000_i64)
		.execute(pool)
		.await
		.expect("Failed to insert CEO");

	// Level 1: Reports to CEO
	sqlx::query("INSERT INTO employees (name, manager_id, salary) VALUES ($1, $2, $3)")
		.bind("Bob")
		.bind(1)
		.bind(3000_i64)
		.execute(pool)
		.await
		.expect("Failed to insert Bob");

	sqlx::query("INSERT INTO employees (name, manager_id, salary) VALUES ($1, $2, $3)")
		.bind("Carol")
		.bind(1)
		.bind(3500_i64)
		.execute(pool)
		.await
		.expect("Failed to insert Carol");

	// Level 2: Reports to Bob
	sqlx::query("INSERT INTO employees (name, manager_id, salary) VALUES ($1, $2, $3)")
		.bind("Dave")
		.bind(2)
		.bind(2000_i64)
		.execute(pool)
		.await
		.expect("Failed to insert Dave");

	sqlx::query("INSERT INTO employees (name, manager_id, salary) VALUES ($1, $2, $3)")
		.bind("Eve")
		.bind(2)
		.bind(2500_i64)
		.execute(pool)
		.await
		.expect("Failed to insert Eve");

	// Level 2: Reports to Carol
	sqlx::query("INSERT INTO employees (name, manager_id, salary) VALUES ($1, $2, $3)")
		.bind("Frank")
		.bind(3)
		.bind(2200_i64)
		.execute(pool)
		.await
		.expect("Failed to insert Frank");
}

// ============================================================================
// Basic WITH Clause Tests - QuerySet API
// ============================================================================

/// Test CTE struct creation and SQL generation
///
/// **Test Intent**: Verify CTE struct generates correct SQL
///
/// **Integration Point**: CTE::new() → to_sql()
#[test]
fn test_cte_struct_creation() {
	let cte = CTE::new(
		"high_earners",
		"SELECT id, name, salary FROM employees WHERE salary > 2500",
	);

	assert_eq!(cte.name, "high_earners");
	assert!(!cte.recursive);
	assert!(cte.materialized.is_none());

	let sql = cte.to_sql();
	assert!(sql.contains("high_earners"));
	assert!(sql.contains("AS"));
	assert!(sql.contains("SELECT id, name, salary"));
}

/// Test CTECollection for multiple CTEs
///
/// **Test Intent**: Verify CTECollection generates correct WITH clause
///
/// **Integration Point**: CTECollection::add() → to_sql()
#[test]
fn test_cte_collection() {
	let mut collection = CTECollection::new();
	assert!(collection.is_empty());

	let cte1 = CTE::new(
		"high_earners",
		"SELECT * FROM employees WHERE salary > 3000",
	);
	let cte2 = CTE::new(
		"low_earners",
		"SELECT * FROM employees WHERE salary <= 2500",
	);

	collection.add(cte1);
	collection.add(cte2);

	assert_eq!(collection.len(), 2);
	assert!(!collection.is_empty());

	let sql = collection.to_sql().unwrap();
	assert!(sql.starts_with("WITH "));
	assert!(sql.contains("high_earners"));
	assert!(sql.contains("low_earners"));
}

/// Test QuerySet with_cte method - basic CTE
///
/// **Test Intent**: Verify QuerySet.with_cte() generates correct SQL with CTE prepended
///
/// **Integration Point**: QuerySet::with_cte() → to_sql()
#[test]
fn test_queryset_with_cte_basic() {
	let high_earners = CTE::new(
		"high_earners",
		"SELECT id, name, salary FROM employees WHERE salary > 2500",
	);

	let sql = QuerySet::<Employee>::new().with_cte(high_earners).to_sql();

	// Verify CTE is prepended
	assert!(sql.starts_with("WITH high_earners AS"));
	// Verify main SELECT follows
	assert!(sql.contains("SELECT * FROM"));
	assert!(sql.contains("\"employees\""));
}

/// Test QuerySet with multiple CTEs
///
/// **Test Intent**: Verify QuerySet supports multiple CTEs
///
/// **Integration Point**: QuerySet::with_cte() chaining → to_sql()
#[test]
fn test_queryset_with_multiple_ctes() {
	let high_earners = CTE::new(
		"high_earners",
		"SELECT * FROM employees WHERE salary > 3000",
	);
	let low_earners = CTE::new(
		"low_earners",
		"SELECT * FROM employees WHERE salary <= 2500",
	);

	let sql = QuerySet::<Employee>::new()
		.with_cte(high_earners)
		.with_cte(low_earners)
		.to_sql();

	// Verify both CTEs are included
	assert!(sql.contains("high_earners"));
	assert!(sql.contains("low_earners"));
	// Should be comma-separated in WITH clause
	assert!(sql.contains(","));
}

// ============================================================================
// Recursive CTE Tests
// ============================================================================

/// Test recursive CTE creation
///
/// **Test Intent**: Verify recursive CTE generates WITH RECURSIVE
///
/// **Integration Point**: CTE::recursive() → CTECollection::to_sql()
#[test]
fn test_recursive_cte() {
	let hierarchy = CTE::new(
		"org_hierarchy",
		"SELECT id, name, manager_id, 1 as level FROM employees WHERE manager_id IS NULL \
		 UNION ALL \
		 SELECT e.id, e.name, e.manager_id, h.level + 1 \
		 FROM employees e JOIN org_hierarchy h ON e.manager_id = h.id",
	)
	.recursive();

	assert!(hierarchy.recursive);

	let mut collection = CTECollection::new();
	collection.add(hierarchy);

	let sql = collection.to_sql().unwrap();
	assert!(
		sql.starts_with("WITH RECURSIVE"),
		"Expected WITH RECURSIVE, got: {}",
		sql
	);
}

/// Test QuerySet with recursive CTE
///
/// **Test Intent**: Verify QuerySet with recursive CTE generates correct SQL
///
/// **Integration Point**: QuerySet::with_cte() + recursive CTE → to_sql()
#[test]
fn test_queryset_with_recursive_cte() {
	let hierarchy = CTE::new(
		"reporting_chain",
		"SELECT id, name, manager_id, 1 as level FROM employees WHERE manager_id IS NULL \
		 UNION ALL \
		 SELECT e.id, e.name, e.manager_id, rc.level + 1 \
		 FROM employees e INNER JOIN reporting_chain rc ON e.manager_id = rc.id",
	)
	.recursive();

	let sql = QuerySet::<Employee>::new().with_cte(hierarchy).to_sql();

	assert!(
		sql.starts_with("WITH RECURSIVE"),
		"Expected WITH RECURSIVE prefix"
	);
	assert!(sql.contains("reporting_chain"));
	assert!(sql.contains("UNION ALL"));
}

// ============================================================================
// CTE with Column Specification Tests
// ============================================================================

/// Test CTE with explicit column names
///
/// **Test Intent**: Verify CTE with_columns generates column list
///
/// **Integration Point**: CTE::with_columns() → to_sql()
#[test]
fn test_cte_with_column_specification() {
	let cte = CTE::new(
		"salary_stats",
		"SELECT id, name, salary FROM employees WHERE salary > 2000",
	)
	.with_columns(vec![
		"emp_id".to_string(),
		"emp_name".to_string(),
		"gross_salary".to_string(),
	]);

	let sql = cte.to_sql();

	assert!(
		sql.contains("salary_stats (emp_id, emp_name, gross_salary)"),
		"Expected column list in CTE definition, got: {}",
		sql
	);
}

// ============================================================================
// CTE Materialization Hints Tests
// ============================================================================

/// Test CTE with MATERIALIZED hint
///
/// **Test Intent**: Verify CTE materialized() generates MATERIALIZED keyword
///
/// **Integration Point**: CTE::materialized() → to_sql()
#[test]
fn test_cte_materialized_hint() {
	let cte = CTE::new(
		"expensive_cte",
		"SELECT id, name, salary FROM employees WHERE salary > 2000",
	)
	.materialized(true);

	let sql = cte.to_sql();

	assert!(
		sql.contains("MATERIALIZED"),
		"Expected MATERIALIZED hint, got: {}",
		sql
	);
	assert!(
		!sql.contains("NOT MATERIALIZED"),
		"Should not contain NOT MATERIALIZED"
	);
}

/// Test CTE with NOT MATERIALIZED hint
///
/// **Test Intent**: Verify CTE materialized(false) generates NOT MATERIALIZED
///
/// **Integration Point**: CTE::materialized(false) → to_sql()
#[test]
fn test_cte_not_materialized_hint() {
	let cte = CTE::new(
		"volatile_cte",
		"SELECT id, name, salary FROM employees WHERE salary > 2000",
	)
	.materialized(false);

	let sql = cte.to_sql();

	assert!(
		sql.contains("NOT MATERIALIZED"),
		"Expected NOT MATERIALIZED hint, got: {}",
		sql
	);
}

// ============================================================================
// QuerySet Integration with Filters and CTEs
// ============================================================================

/// Test QuerySet with CTE and filters
///
/// **Test Intent**: Verify QuerySet combines CTE with WHERE clause correctly
///
/// **Integration Point**: QuerySet::with_cte() + filter() → to_sql()
#[test]
fn test_queryset_cte_with_filters() {
	let high_earners = CTE::new(
		"high_earners",
		"SELECT * FROM employees WHERE salary > 2500",
	);

	let sql = QuerySet::<Employee>::new()
		.with_cte(high_earners)
		.filter(Filter::new(
			"salary".to_string(),
			FilterOperator::Gt,
			FilterValue::Int(3000),
		))
		.to_sql();

	// Verify CTE is present
	assert!(sql.contains("WITH high_earners AS"));
	// Verify WHERE clause is present
	assert!(sql.contains("WHERE"));
	assert!(sql.contains("3000"));
}

/// Test QuerySet with CTE and ordering
///
/// **Test Intent**: Verify QuerySet combines CTE with ORDER BY correctly
///
/// **Integration Point**: QuerySet::with_cte() + order_by() → to_sql()
#[test]
fn test_queryset_cte_with_ordering() {
	let high_earners = CTE::new(
		"high_earners",
		"SELECT * FROM employees WHERE salary > 2500",
	);

	let sql = QuerySet::<Employee>::new()
		.with_cte(high_earners)
		.order_by(&["-salary"])
		.to_sql();

	// Verify CTE is present
	assert!(sql.contains("WITH high_earners AS"));
	// Verify ORDER BY clause is present
	assert!(sql.contains("ORDER BY"));
	assert!(sql.contains("DESC"));
}

// ============================================================================
// Database Execution Tests
// ============================================================================

/// Test basic CTE execution against database
///
/// **Test Intent**: Execute CTE query and verify results
///
/// **Integration Point**: QuerySet → PostgreSQL execution
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_cte_database_execution_basic(
	#[future] cte_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = cte_test_db.await;

	// Use CTE to find high earners
	let sql = r#"
		WITH high_earners AS (
			SELECT id, name, salary FROM employees WHERE salary > 2500
		)
		SELECT * FROM high_earners ORDER BY salary DESC
	"#;

	let rows = sqlx::query(sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute CTE query");

	assert_eq!(rows.len(), 3, "Expected 3 high earners (Alice, Carol, Bob)");

	let names: Vec<String> = rows.iter().map(|row| row.get("name")).collect();
	assert!(names.contains(&"Alice (CEO)".to_string()));
	assert!(names.contains(&"Carol".to_string()));
	assert!(names.contains(&"Bob".to_string()));
}

/// Test recursive CTE execution - hierarchy traversal
///
/// **Test Intent**: Execute recursive CTE and verify hierarchy levels
///
/// **Integration Point**: WITH RECURSIVE → PostgreSQL execution
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_recursive_cte_hierarchy_execution(
	#[future] cte_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = cte_test_db.await;

	let sql = r#"
		WITH RECURSIVE reporting_chain AS (
			SELECT id, name, manager_id, 1 as level
			FROM employees
			WHERE manager_id IS NULL
			UNION ALL
			SELECT e.id, e.name, e.manager_id, rc.level + 1
			FROM employees e
			INNER JOIN reporting_chain rc ON e.manager_id = rc.id
		)
		SELECT * FROM reporting_chain
		ORDER BY level, id
	"#;

	let rows = sqlx::query(sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute recursive CTE");

	assert_eq!(rows.len(), 6, "Expected 6 employees in hierarchy");

	// Verify levels are correct
	let level_1: i32 = rows[0].get("level");
	assert_eq!(level_1, 1, "CEO should be at level 1");

	let level_2: i32 = rows[1].get("level");
	assert_eq!(level_2, 2, "First level managers should be at level 2");
}

/// Test multiple CTEs execution
///
/// **Test Intent**: Execute query with multiple CTEs
///
/// **Integration Point**: Multiple CTEs → PostgreSQL execution
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_multiple_ctes_execution(
	#[future] cte_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = cte_test_db.await;

	let sql = r#"
		WITH high_earners AS (
			SELECT id, name, salary FROM employees WHERE salary > 3000
		),
		low_earners AS (
			SELECT id, name, salary FROM employees WHERE salary <= 2500
		)
		SELECT name, salary, 'high' as category FROM high_earners
		UNION ALL
		SELECT name, salary, 'low' as category FROM low_earners
		ORDER BY salary DESC
	"#;

	let rows = sqlx::query(sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute multiple CTEs");

	assert!(rows.len() >= 4, "Expected at least 4 employees");

	let categories: Vec<String> = rows.iter().map(|row| row.get("category")).collect();
	assert!(categories.contains(&"high".to_string()));
	assert!(categories.contains(&"low".to_string()));
}

/// Test recursive CTE with aggregation
///
/// **Test Intent**: Execute recursive CTE with GROUP BY aggregation
///
/// **Integration Point**: WITH RECURSIVE + aggregation → PostgreSQL execution
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_recursive_cte_with_aggregation_execution(
	#[future] cte_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = cte_test_db.await;

	let sql = r#"
		WITH RECURSIVE reporting_chain AS (
			SELECT id, name, manager_id, salary, 1 as level
			FROM employees
			WHERE manager_id IS NULL
			UNION ALL
			SELECT e.id, e.name, e.manager_id, e.salary, rc.level + 1
			FROM employees e
			INNER JOIN reporting_chain rc ON e.manager_id = rc.id
		)
		SELECT level, COUNT(*) as emp_count, SUM(salary) as total_salary
		FROM reporting_chain
		GROUP BY level
		ORDER BY level
	"#;

	let rows = sqlx::query(sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute recursive CTE with aggregation");

	assert!(!rows.is_empty(), "Expected aggregation results");

	// Level 1 should have 1 employee (CEO)
	let first_row = &rows[0];
	let level: i32 = first_row.get("level");
	let count: i64 = first_row.get("emp_count");

	assert_eq!(level, 1, "First level should be 1");
	assert_eq!(count, 1, "Should have 1 CEO");
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

/// Test empty CTE collection
///
/// **Test Intent**: Verify empty CTECollection returns None for to_sql()
///
/// **Integration Point**: CTECollection::to_sql() edge case
#[test]
fn test_empty_cte_collection() {
	let collection = CTECollection::new();
	assert!(collection.is_empty());
	assert!(collection.to_sql().is_none());
}

/// Test CTE retrieval by name
///
/// **Test Intent**: Verify CTECollection::get() returns correct CTE
///
/// **Integration Point**: CTECollection::get()
#[test]
fn test_cte_collection_get_by_name() {
	let mut collection = CTECollection::new();
	collection.add(CTE::new("cte_one", "SELECT 1"));
	collection.add(CTE::new("cte_two", "SELECT 2"));

	let found = collection.get("cte_one");
	assert!(found.is_some());
	assert_eq!(found.unwrap().name, "cte_one");

	let not_found = collection.get("nonexistent");
	assert!(not_found.is_none());
}

/// Test QuerySet without CTEs produces standard SQL
///
/// **Test Intent**: Verify QuerySet without CTEs doesn't add WITH clause
///
/// **Integration Point**: QuerySet::to_sql() without CTEs
#[test]
fn test_queryset_without_cte() {
	let sql = QuerySet::<Employee>::new().to_sql();

	// Should not have WITH clause
	assert!(
		!sql.contains("WITH "),
		"Expected no WITH clause, got: {}",
		sql
	);
	// Should have standard SELECT
	assert!(sql.starts_with("SELECT"));
}
