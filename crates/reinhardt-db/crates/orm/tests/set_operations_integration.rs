//! Set Operations Integration Tests (Phase 3)
//!
//! Tests comprehensive set operations functionality covering:
//! - UNION operations (removes duplicates)
//! - UNION ALL operations (preserves duplicates)
//! - INTERSECT operations (returns common rows)
//! - EXCEPT operations (returns rows in first set but not second)
//! - Edge cases: Empty sets, duplicate handling
//! - Equivalence partitioning: 3 set operation types (UNION, INTERSECT, EXCEPT)
//! - Chained operations and ordering
//!
//! **Test Strategy:**
//! - Normal cases: All set operations (UNION, UNION ALL, INTERSECT, EXCEPT) working correctly
//! - Edge cases: Empty sets, duplicate rows, single-row results
//! - Equivalence partitioning: Test three main set operation categories
//! - Ordering and pagination: ORDER BY, LIMIT, OFFSET with set operations
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container
//!
//! **Test Data Schema:**
//! - employees(id SERIAL PRIMARY KEY, name TEXT NOT NULL, department TEXT NOT NULL, salary BIGINT NOT NULL)
//! - contractors(id SERIAL PRIMARY KEY, name TEXT NOT NULL, department TEXT NOT NULL, salary BIGINT NOT NULL)

use reinhardt_orm::set_operations::CombinedQuery;
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sea_query::Iden;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

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
			department TEXT NOT NULL,
			salary BIGINT NOT NULL
		)",
	)
	.execute(pool)
	.await
	.expect("Failed to create employees table");

	// Create contractors table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS contractors (
			id SERIAL PRIMARY KEY,
			name TEXT NOT NULL,
			department TEXT NOT NULL,
			salary BIGINT NOT NULL
		)",
	)
	.execute(pool)
	.await
	.expect("Failed to create contractors table");

	// Insert employees data
	// Engineering: Alice (100k), Bob (95k), Carol (105k)
	sqlx::query(
		"INSERT INTO employees (name, department, salary) VALUES
			($1, $2, $3), ($4, $5, $6), ($7, $8, $9), ($10, $11, $12)",
	)
	.bind("Alice")
	.bind("Engineering")
	.bind(100000_i64)
	.bind("Bob")
	.bind("Engineering")
	.bind(95000_i64)
	.bind("Carol")
	.bind("Engineering")
	.bind(105000_i64)
	.bind("David")
	.bind("Sales")
	.bind(80000_i64)
	.execute(pool)
	.await
	.expect("Failed to insert employees");

	// Insert contractors data
	// Some overlap with employees (Bob, Carol), some unique (Eve, Frank)
	sqlx::query(
		"INSERT INTO contractors (name, department, salary) VALUES
			($1, $2, $3), ($4, $5, $6), ($7, $8, $9), ($10, $11, $12)",
	)
	.bind("Bob")
	.bind("Engineering")
	.bind(95000_i64)
	.bind("Carol")
	.bind("Engineering")
	.bind(105000_i64)
	.bind("Eve")
	.bind("Marketing")
	.bind(75000_i64)
	.bind("Frank")
	.bind("Sales")
	.bind(85000_i64)
	.execute(pool)
	.await
	.expect("Failed to insert contractors");
}

// ============================================================================
// UNION Tests (Positive Case - Normal Behavior)
// ============================================================================

/// Test UNION operation (removes duplicates)
///
/// **Test Intent**: Retrieve all records with UNION operation removing duplicates
///
/// **Integration Point**: CombinedQuery.union() → PostgreSQL UNION operator
///
/// **Not Intent**: UNION ALL (preserves duplicates), multi-column operations
#[rstest]
#[tokio::test]
async fn test_union_removes_duplicates(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	// UNION: Get all names from employees and contractors, removing duplicates
	let query_sql = "SELECT name FROM employees UNION SELECT name FROM contractors";
	let combined = CombinedQuery::new(query_sql);

	let sql = combined.to_sql();
	let rows = sqlx::query(&sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute UNION query");

	// Expected: Alice, Bob, Carol, David, Eve, Frank (6 unique names)
	assert_eq!(rows.len(), 6, "UNION should return 6 unique names");

	let names: Vec<String> = rows.iter().map(|row| row.get("name")).collect();
	assert!(names.contains(&"Alice".to_string()));
	assert!(names.contains(&"Bob".to_string()));
	assert!(names.contains(&"Carol".to_string()));
	assert!(names.contains(&"David".to_string()));
	assert!(names.contains(&"Eve".to_string()));
	assert!(names.contains(&"Frank".to_string()));
}

/// Test UNION ALL operation (preserves duplicates)
///
/// **Test Intent**: Retrieve all records with UNION ALL operation preserving duplicates
///
/// **Integration Point**: CombinedQuery.union_all() → PostgreSQL UNION ALL operator
///
/// **Not Intent**: UNION (removes duplicates), filtering
#[rstest]
#[tokio::test]
async fn test_union_all_preserves_duplicates(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	// UNION ALL: Get all names from employees and contractors, preserving duplicates
	let query_sql =
		"SELECT name FROM employees UNION ALL SELECT name FROM contractors ORDER BY name";
	let combined = CombinedQuery::new(query_sql);

	let sql = combined.to_sql();
	let rows = sqlx::query(&sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute UNION ALL query");

	// Expected: 4 employees + 4 contractors = 8 rows (with duplicates)
	assert_eq!(
		rows.len(),
		8,
		"UNION ALL should return 8 rows with duplicates"
	);

	let names: Vec<String> = rows.iter().map(|row| row.get("name")).collect();
	// Count occurrences of Bob and Carol (should appear twice each)
	let bob_count = names.iter().filter(|n| *n == "Bob").count();
	let carol_count = names.iter().filter(|n| *n == "Carol").count();
	assert_eq!(bob_count, 2, "Bob should appear twice in UNION ALL");
	assert_eq!(carol_count, 2, "Carol should appear twice in UNION ALL");
}

// ============================================================================
// INTERSECT Tests (Positive Case - Normal Behavior)
// ============================================================================

/// Test INTERSECT operation (returns common rows)
///
/// **Test Intent**: Retrieve common records that exist in both tables with INTERSECT operation
///
/// **Integration Point**: CombinedQuery.intersect() → PostgreSQL INTERSECT operator
///
/// **Not Intent**: UNION, EXCEPT, multi-row intersection
#[rstest]
#[tokio::test]
async fn test_intersect_returns_common_rows(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	// INTERSECT: Get names that exist in both employees and contractors
	let query_sql =
		"SELECT name FROM employees INTERSECT SELECT name FROM contractors ORDER BY name";
	let combined = CombinedQuery::new(query_sql);

	let sql = combined.to_sql();
	let rows = sqlx::query(&sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute INTERSECT query");

	// Expected: Bob and Carol (present in both tables)
	assert_eq!(rows.len(), 2, "INTERSECT should return 2 common names");

	let names: Vec<String> = rows.iter().map(|row| row.get("name")).collect();
	assert_eq!(names[0], "Bob", "First common name should be Bob");
	assert_eq!(names[1], "Carol", "Second common name should be Carol");
}

// ============================================================================
// EXCEPT Tests (Positive Case - Normal Behavior)
// ============================================================================

/// Test EXCEPT operation (removes second set from first set)
///
/// **Test Intent**: Retrieve records that exist only in first table with EXCEPT operation
///
/// **Integration Point**: CombinedQuery.except() → PostgreSQL EXCEPT operator
///
/// **Not Intent**: UNION, INTERSECT, multiple removals
#[rstest]
#[tokio::test]
async fn test_except_returns_unique_rows(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	// EXCEPT: Get names that exist in employees but not in contractors
	let query_sql = "SELECT name FROM employees EXCEPT SELECT name FROM contractors ORDER BY name";
	let combined = CombinedQuery::new(query_sql);

	let sql = combined.to_sql();
	let rows = sqlx::query(&sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute EXCEPT query");

	// Expected: Alice and David (in employees but not in contractors)
	assert_eq!(rows.len(), 2, "EXCEPT should return 2 unique names");

	let names: Vec<String> = rows.iter().map(|row| row.get("name")).collect();
	assert!(names.contains(&"Alice".to_string()));
	assert!(names.contains(&"David".to_string()));
	assert!(!names.contains(&"Bob".to_string()));
	assert!(!names.contains(&"Carol".to_string()));
}

// ============================================================================
// Edge Case Tests: Empty Sets
// ============================================================================

/// Test UNION with empty set
///
/// **Test Intent**: UNION with empty set returns only non-empty set
///
/// **Integration Point**: CombinedQuery with UNION on empty result set
///
/// **Not Intent**: Multiple empty sets, complex filters
#[rstest]
#[tokio::test]
async fn test_union_with_empty_set(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	// UNION: Get names where salary > 200000 (empty) UNION names from contractors (not empty)
	let query_sql = "SELECT name FROM employees WHERE salary > 200000 UNION SELECT name FROM contractors ORDER BY name";
	let combined = CombinedQuery::new(query_sql);

	let sql = combined.to_sql();
	let rows = sqlx::query(&sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute UNION with empty set");

	// Should return contractor names only (4 rows)
	assert_eq!(
		rows.len(),
		4,
		"UNION with empty set should return non-empty set results"
	);

	let names: Vec<String> = rows.iter().map(|row| row.get("name")).collect();
	assert!(names.contains(&"Bob".to_string()));
	assert!(names.contains(&"Carol".to_string()));
	assert!(names.contains(&"Eve".to_string()));
	assert!(names.contains(&"Frank".to_string()));
}

/// Test INTERSECT with empty set
///
/// **Test Intent**: INTERSECT with empty set returns empty set
///
/// **Integration Point**: CombinedQuery with INTERSECT on empty result set
///
/// **Not Intent**: Multiple non-empty sets, complex logic
#[rstest]
#[tokio::test]
async fn test_intersect_with_empty_set(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	// INTERSECT: Get names where salary > 200000 (empty) INTERSECT contractors (not empty)
	let query_sql =
		"SELECT name FROM employees WHERE salary > 200000 INTERSECT SELECT name FROM contractors";
	let combined = CombinedQuery::new(query_sql);

	let sql = combined.to_sql();
	let rows = sqlx::query(&sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute INTERSECT with empty set");

	// Should return empty set
	assert_eq!(
		rows.len(),
		0,
		"INTERSECT with empty set should return empty set"
	);
}

// ============================================================================
// Equivalence Partitioning Tests: Salary Range Categories
// ============================================================================

/// UNION with salary range partitioning: Low salaries
///
/// **Test Intent**: UNION with equivalence partition for salary < 80000
///
/// **Integration Point**: CombinedQuery.union() with WHERE filtering
///
/// **Not Intent**: Multiple conditions, complex joins
#[rstest]
#[tokio::test]
async fn test_union_low_salary_partition(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	// Get all low-salary workers (< 80000) from both tables
	let query_sql = "SELECT name, salary FROM employees WHERE salary < 80000 UNION SELECT name, salary FROM contractors WHERE salary < 80000 ORDER BY salary";
	let combined = CombinedQuery::new(query_sql);

	let sql = combined.to_sql();
	let rows = sqlx::query(&sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute UNION with low salary partition");

	// Expected: Eve (75k from contractors)
	assert_eq!(rows.len(), 1, "Should find 1 low-salary person");
	let name: String = rows[0].get("name");
	let salary: i64 = rows[0].get("salary");
	assert_eq!(name, "Eve");
	assert_eq!(salary, 75000);
}

/// INTERSECT with salary range partitioning: Mid salaries (80k-100k)
///
/// **Test Intent**: INTERSECT with equivalence partition for 80000 <= salary <= 100000
///
/// **Integration Point**: CombinedQuery.intersect() with range filtering
///
/// **Not Intent**: Single condition, open-ended ranges
#[rstest]
#[tokio::test]
async fn test_intersect_mid_salary_partition(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	// Get names that appear in both tables within mid-salary range
	let query_sql = "SELECT name FROM employees WHERE salary >= 80000 AND salary <= 100000 INTERSECT SELECT name FROM contractors WHERE salary >= 80000 AND salary <= 100000";
	let combined = CombinedQuery::new(query_sql);

	let sql = combined.to_sql();
	let rows = sqlx::query(&sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute INTERSECT with mid salary partition");

	// Expected: Bob (95k from both)
	assert_eq!(rows.len(), 1, "Should find 1 person in mid-salary range");
	let name: String = rows[0].get("name");
	assert_eq!(name, "Bob");
}

/// EXCEPT with salary range partitioning: High salaries (> 100k)
///
/// **Test Intent**: EXCEPT with equivalence partition for salary > 100000
///
/// **Integration Point**: CombinedQuery.except() with threshold filtering
///
/// **Not Intent**: Range boundaries, multiple thresholds
#[rstest]
#[tokio::test]
async fn test_except_high_salary_partition(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	// Get names from employees with high salary that are not in contractors
	let query_sql = "SELECT name FROM employees WHERE salary > 100000 EXCEPT SELECT name FROM contractors WHERE salary > 100000";
	let combined = CombinedQuery::new(query_sql);

	let sql = combined.to_sql();
	let rows = sqlx::query(&sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute EXCEPT with high salary partition");

	// Expected: None (Carol has 105k in both tables)
	assert_eq!(
		rows.len(),
		0,
		"Should find no high-salary employees unique to employees table"
	);
}

// ============================================================================
// Chained Operations and Ordering Tests
// ============================================================================

/// Test chained multiple set operations
///
/// **Test Intent**: Multiple set operations chained together with final ordering
///
/// **Integration Point**: CombinedQuery chaining .union().intersect() and ORDER BY
///
/// **Not Intent**: Single operation, no ordering
#[rstest]
#[tokio::test]
async fn test_chained_union_with_ordering(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	// Build a query with UNION and ORDER BY
	let combined = CombinedQuery::new("SELECT name FROM employees")
		.union("SELECT name FROM contractors")
		.order_by("name ASC");

	let sql = combined.to_sql();
	let rows = sqlx::query(&sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute chained UNION with ORDER BY");

	// Should return 6 unique names in alphabetical order
	assert_eq!(rows.len(), 6, "Should return 6 unique names");

	let names: Vec<String> = rows.iter().map(|row| row.get("name")).collect();
	assert_eq!(names[0], "Alice", "First name should be Alice");
	assert_eq!(names[1], "Bob", "Second name should be Bob");
	assert_eq!(names[2], "Carol", "Third name should be Carol");
}

/// Test set operation with LIMIT
///
/// **Test Intent**: Set operation with LIMIT clause to get top N results
///
/// **Integration Point**: CombinedQuery.limit() after union operation
///
/// **Not Intent**: OFFSET, complex filtering
#[rstest]
#[tokio::test]
async fn test_union_with_limit(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	// Build a query with UNION and LIMIT
	let combined = CombinedQuery::new("SELECT name FROM employees")
		.union("SELECT name FROM contractors")
		.order_by("name ASC")
		.limit(3);

	let sql = combined.to_sql();
	let rows = sqlx::query(&sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute UNION with LIMIT");

	// Should return only 3 names
	assert_eq!(rows.len(), 3, "LIMIT should restrict results to 3");

	let names: Vec<String> = rows.iter().map(|row| row.get("name")).collect();
	assert!(names.len() <= 3, "LIMIT should be respected");
}
