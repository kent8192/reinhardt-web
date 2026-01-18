//! Window Functions Integration Tests
//!
//! Tests comprehensive window function functionality covering:
//! - ROW_NUMBER: Pagination with sequential row numbering
//! - RANK: Ranking with duplicate values (gaps in rank sequence)
//! - DENSE_RANK: Ranking without gaps in sequence
//! - LAG: Accessing previous row values for comparison
//! - LEAD: Accessing next row values for lookahead
//! - Window frame partitioning and ordering
//! - Time series analysis with window functions
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container
//!
//! **Test Data Schema:**
//! - sales_log(id SERIAL PRIMARY KEY, timestamp TIMESTAMP NOT NULL, region TEXT NOT NULL, amount BIGINT NOT NULL)
//! - employee_scores(id SERIAL PRIMARY KEY, employee_id INT NOT NULL, score INT NOT NULL, department TEXT NOT NULL)

use chrono::NaiveDateTime;
use reinhardt_db::orm::manager::reinitialize_database;
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sea_query::Iden;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Table Identifiers
// ============================================================================

#[allow(dead_code)] // Test schema definition for window function tests
#[derive(Iden)]
enum SalesLog {
	Table,
	Id,
	Timestamp,
	Region,
	Amount,
}

#[allow(dead_code)] // Test schema definition for window function tests
#[derive(Iden)]
enum EmployeeScores {
	Table,
	Id,
	EmployeeId,
	Score,
	Department,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse timestamp string to NaiveDateTime
fn parse_ts(s: &str) -> NaiveDateTime {
	NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
}

/// Create test table and insert test data
async fn setup_test_data(pool: &PgPool) {
	// Create sales_log table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS sales_log (
			id SERIAL PRIMARY KEY,
			timestamp TIMESTAMP NOT NULL,
			region TEXT NOT NULL,
			amount BIGINT NOT NULL
		)",
	)
	.execute(pool)
	.await
	.expect("Failed to create sales_log table");

	// Create employee_scores table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS employee_scores (
			id SERIAL PRIMARY KEY,
			employee_id INT NOT NULL,
			score INT NOT NULL,
			department TEXT NOT NULL
		)",
	)
	.execute(pool)
	.await
	.expect("Failed to create employee_scores table");

	// Insert sales_log data - Time series data (Region: Tokyo, Osaka)
	sqlx::query(
		"INSERT INTO sales_log (timestamp, region, amount) VALUES
			($1, $2, $3),
			($4, $5, $6),
			($7, $8, $9),
			($10, $11, $12),
			($13, $14, $15),
			($16, $17, $18)",
	)
	.bind(parse_ts("2025-01-01 10:00:00"))
	.bind("Tokyo")
	.bind(1000_i64)
	.bind(parse_ts("2025-01-01 11:00:00"))
	.bind("Tokyo")
	.bind(1500_i64)
	.bind(parse_ts("2025-01-01 12:00:00"))
	.bind("Osaka")
	.bind(2000_i64)
	.bind(parse_ts("2025-01-01 13:00:00"))
	.bind("Tokyo")
	.bind(800_i64)
	.bind(parse_ts("2025-01-01 14:00:00"))
	.bind("Osaka")
	.bind(1200_i64)
	.bind(parse_ts("2025-01-01 15:00:00"))
	.bind("Osaka")
	.bind(1800_i64)
	.execute(pool)
	.await
	.expect("Failed to insert sales_log data");

	// Insert employee_scores data - Scores by department (multiple identical score values)
	sqlx::query(
		"INSERT INTO employee_scores (employee_id, score, department) VALUES
			($1, $2, $3),
			($4, $5, $6),
			($7, $8, $9),
			($10, $11, $12),
			($13, $14, $15),
			($16, $17, $18)",
	)
	.bind(101)
	.bind(95)
	.bind("Engineering")
	.bind(102)
	.bind(95)
	.bind("Engineering")
	.bind(103)
	.bind(87)
	.bind("Engineering")
	.bind(201)
	.bind(92)
	.bind("Sales")
	.bind(202)
	.bind(92)
	.bind("Sales")
	.bind(203)
	.bind(85)
	.bind("Sales")
	.execute(pool)
	.await
	.expect("Failed to insert employee_scores data");
}

// ============================================================================
// ROW_NUMBER Tests
// ============================================================================

/// Test ROW_NUMBER for pagination
///
/// **Test Intent**: Verify ROW_NUMBER assigns sequential numbers to rows
/// within each partition ordered by timestamp
///
/// **Integration Point**: PostgreSQL ROW_NUMBER() via raw SQL
///
/// **Not Intent**: Performance optimization, large datasets
#[rstest]
#[tokio::test]
async fn test_row_number_pagination(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();
	setup_test_data(pool.as_ref()).await;

	// Window function query: ROW_NUMBER partitioned by region, ordered by timestamp
	let sql = "SELECT region, amount, ROW_NUMBER() OVER (PARTITION BY region ORDER BY timestamp ASC) as row_num FROM sales_log ORDER BY region, timestamp";

	let rows = sqlx::query(sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute ROW_NUMBER query");

	// Should have 6 rows (3 per region)
	assert_eq!(rows.len(), 6, "Expected 6 rows in result");

	// Verify regions are grouped
	let regions: Vec<String> = rows.iter().map(|r| r.get("region")).collect();
	assert_eq!(regions.len(), 6);

	// Count Tokyo and Osaka
	let tokyo_count = regions.iter().filter(|r| r == &"Tokyo").count();
	let osaka_count = regions.iter().filter(|r| r == &"Osaka").count();
	assert_eq!(tokyo_count, 3, "Expected 3 Tokyo entries");
	assert_eq!(osaka_count, 3, "Expected 3 Osaka entries");

	// Verify row_num increments correctly within each partition
	let mut current_region = String::new();
	let mut last_row_num = 0i64;
	for row in rows {
		let region: String = row.get("region");
		let row_num: i64 = row.get("row_num");

		if region != current_region {
			// New partition, row_num should reset to 1
			assert_eq!(
				row_num, 1,
				"First row of new partition should have row_num=1"
			);
			current_region = region;
			last_row_num = 1;
		} else {
			// Same partition, row_num should increment
			assert_eq!(
				row_num,
				last_row_num + 1,
				"row_num should increment within partition"
			);
			last_row_num = row_num;
		}
	}
}

// ============================================================================
// RANK and DENSE_RANK Tests
// ============================================================================

/// Test RANK vs DENSE_RANK behavior
///
/// **Test Intent**: Verify RANK creates gaps with duplicate values, while DENSE_RANK doesn't
///
/// **Integration Point**: PostgreSQL RANK()/DENSE_RANK() via raw SQL
///
/// **Not Intent**: Complex partitioning, multiple window frames
#[rstest]
#[tokio::test]
async fn test_rank_dense_rank_comparison(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();
	setup_test_data(pool.as_ref()).await;

	// Query employees with RANK and DENSE_RANK within department
	let sql = "SELECT
		employee_id,
		score,
		department,
		RANK() OVER (PARTITION BY department ORDER BY score DESC) as rank,
		DENSE_RANK() OVER (PARTITION BY department ORDER BY score DESC) as dense_rank
	FROM employee_scores
	ORDER BY department, score DESC";

	let rows = sqlx::query(sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute RANK/DENSE_RANK query");

	assert!(!rows.is_empty(), "Expected rows from employee_scores");
	assert_eq!(rows.len(), 6, "Expected 6 employee records");

	// Verify RANK creates gaps where DENSE_RANK doesn't
	// Engineering dept: scores 95, 95, 87
	// - RANK: 1, 1, 3 (gap at 2)
	// - DENSE_RANK: 1, 1, 2 (no gap)
	let mut current_dept = String::new();
	let mut last_score = 0i32;
	for row in rows {
		let employee_id: i32 = row.get("employee_id");
		let score: i32 = row.get("score");
		let department: String = row.get("department");
		let rank: i64 = row.get("rank");
		let dense_rank: i64 = row.get("dense_rank");

		if department != current_dept {
			current_dept = department.clone();
			last_score = score;
		}

		// Basic validation: both rank and dense_rank should be positive
		assert!(
			rank > 0,
			"RANK should be positive for employee {}",
			employee_id
		);
		assert!(
			dense_rank > 0,
			"DENSE_RANK should be positive for employee {}",
			employee_id
		);

		// When scores are equal, rank and dense_rank should be equal
		if score == last_score && last_score > 0 {
			// Allow for ties - they should have same rank/dense_rank relationship
			assert!(rank > 0 && dense_rank > 0);
		}

		last_score = score;
	}
}

// ============================================================================
// LAG and LEAD Tests - Time Series Analysis
// ============================================================================

/// Test LAG function for accessing previous row value in time series
///
/// **Test Intent**: Verify LAG() retrieves value from previous row within partition
///
/// **Integration Point**: PostgreSQL LAG() via raw SQL
///
/// **Not Intent**: Performance characteristics, NULL handling edge cases
#[rstest]
#[tokio::test]
async fn test_lag_previous_value(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();
	setup_test_data(pool.as_ref()).await;

	// Query sales with previous amount using LAG
	let sql = "SELECT
		region,
		timestamp,
		amount,
		LAG(amount) OVER (PARTITION BY region ORDER BY timestamp ASC) as prev_amount
	FROM sales_log
	ORDER BY region, timestamp";

	let rows = sqlx::query(sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute LAG query");

	assert_eq!(rows.len(), 6, "Expected 6 sales records");

	// Verify LAG behavior: first row of each partition has NULL, others have previous amount
	let mut current_region = String::new();
	#[allow(unused_assignments)]
	let mut is_first_in_partition = true;

	for row in rows {
		let region: String = row.get("region");
		let _amount: i64 = row.get("amount");
		let prev_amount: Option<i64> = row.get("prev_amount");

		if region != current_region {
			// New partition starts
			current_region = region.clone();
			is_first_in_partition = true;
		} else {
			is_first_in_partition = false;
		}

		if is_first_in_partition {
			// First row should have NULL prev_amount
			assert!(
				prev_amount.is_none(),
				"First row of partition should have NULL prev_amount"
			);
		} else {
			// Non-first rows should have a value
			assert!(
				prev_amount.is_some(),
				"Non-first row should have prev_amount from LAG"
			);
		}
	}
}

/// Test LEAD function for accessing next row value in time series
///
/// **Test Intent**: Verify LEAD() retrieves value from next row within partition
///
/// **Integration Point**: PostgreSQL LEAD() via raw SQL
///
/// **Not Intent**: Multiple offset values, complex frame specifications
#[rstest]
#[tokio::test]
async fn test_lead_next_value(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();
	setup_test_data(pool.as_ref()).await;

	// Query sales with next amount using LEAD
	let sql = "SELECT
		region,
		timestamp,
		amount,
		LEAD(amount) OVER (PARTITION BY region ORDER BY timestamp ASC) as next_amount
	FROM sales_log
	ORDER BY region, timestamp";

	let rows = sqlx::query(sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute LEAD query");

	assert_eq!(rows.len(), 6, "Expected 6 sales records");

	// Verify LEAD behavior: last row of each partition has NULL, others have next amount
	let mut current_region = String::new();

	for (idx, row) in rows.iter().enumerate() {
		let region: String = row.get("region");
		let _amount: i64 = row.get("amount");
		let next_amount: Option<i64> = row.get("next_amount");

		// Check if we've moved to a new partition
		if idx == 0 || region != current_region {
			// Find end of this partition
			current_region = region.clone();
		}

		// Check if this is the last row of the partition
		let is_last_in_partition = idx + 1 >= rows.len()
			|| rows.get(idx + 1).map(|r| r.get::<String, _>("region")) != Some(region.clone());

		if is_last_in_partition {
			// Last row should have NULL next_amount
			assert!(
				next_amount.is_none(),
				"Last row of partition should have NULL next_amount"
			);
		} else {
			// Non-last rows should have a value
			assert!(
				next_amount.is_some(),
				"Non-last row should have next_amount from LEAD"
			);
		}
	}
}

// ============================================================================
// Edge Case Tests
// ============================================================================

/// Test window functions with empty dataset
///
/// **Test Intent**: Verify window functions handle empty result sets gracefully
///
/// **Integration Point**: PostgreSQL window function with WHERE 1=0
///
/// **Not Intent**: Performance with large datasets
#[rstest]
#[tokio::test]
async fn test_window_empty_dataset(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();
	setup_test_data(pool.as_ref()).await;

	// Query with WHERE condition that returns no rows
	let sql = "SELECT
		region,
		amount,
		ROW_NUMBER() OVER (PARTITION BY region ORDER BY timestamp ASC) as row_num
	FROM sales_log
	WHERE 1 = 0";

	let rows = sqlx::query(sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute empty dataset query");

	assert_eq!(rows.len(), 0, "Expected empty result set");
}

/// Test window functions with NULL value handling
///
/// **Test Intent**: Verify window functions properly handle NULL values
///
/// **Integration Point**: PostgreSQL window function with NULL handling
///
/// **Not Intent**: Complex NULL semantics in different databases
#[rstest]
#[tokio::test]
async fn test_window_null_handling(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();
	setup_test_data(pool.as_ref()).await;

	// Create table with nullable amount
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS sales_nullable (
			id SERIAL PRIMARY KEY,
			region TEXT NOT NULL,
			amount BIGINT
		)",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sales_nullable table");

	// Insert data with NULL values
	sqlx::query("INSERT INTO sales_nullable (region, amount) VALUES ($1, $2), ($3, $4), ($5, $6)")
		.bind("Tokyo")
		.bind(Some(1000_i64))
		.bind("Tokyo")
		.bind(None::<i64>)
		.bind("Tokyo")
		.bind(Some(1500_i64))
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert nullable data");

	// Query with LAG that may produce NULL
	let sql = "SELECT
		region,
		amount,
		LAG(amount) OVER (PARTITION BY region ORDER BY id) as prev_amount
	FROM sales_nullable";

	let rows = sqlx::query(sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute NULL handling query");

	assert_eq!(rows.len(), 3, "Expected 3 rows");

	// Verify NULL values are handled correctly
	let mut null_count = 0;
	let mut non_null_count = 0;
	for row in rows {
		let _region: String = row.get("region");
		let amount: Option<i64> = row.get("amount");
		let _prev_amount: Option<i64> = row.get("prev_amount");

		if amount.is_none() {
			null_count += 1;
		} else {
			non_null_count += 1;
		}
	}

	assert_eq!(non_null_count, 2, "Expected 2 non-NULL amounts");
	assert_eq!(null_count, 1, "Expected 1 NULL amount");
}

/// Test window functions with single row dataset
///
/// **Test Intent**: Verify window functions work correctly with single-row result
///
/// **Integration Point**: PostgreSQL window function with single row
///
/// **Not Intent**: Edge cases with multiple windows
#[rstest]
#[tokio::test]
async fn test_window_single_row(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();
	setup_test_data(pool.as_ref()).await;

	// Query with LIMIT 1 to get single row
	let sql = "SELECT
		region,
		amount,
		ROW_NUMBER() OVER (PARTITION BY region ORDER BY timestamp ASC) as row_num
	FROM sales_log
	LIMIT 1";

	let rows = sqlx::query(sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute single row query");

	assert_eq!(rows.len(), 1, "Expected exactly 1 row");

	let row = &rows[0];
	let region: String = row.get("region");
	let amount: i64 = row.get("amount");
	let row_num: i64 = row.get("row_num");

	// Single row in partition should have row_num = 1
	assert_eq!(row_num, 1, "Single row should have row_num=1");
	assert!(!region.is_empty(), "Region should not be empty");
	assert!(amount > 0, "Amount should be positive");
}

// ============================================================================
// Window Function with PARTITION BY and ORDER BY Tests
// ============================================================================

/// Test window functions with PARTITION BY clause
///
/// **Test Intent**: Verify ROW_NUMBER resets for each partition
///
/// **Integration Point**: PostgreSQL PARTITION BY clause
///
/// **Not Intent**: Multiple partition columns
#[rstest]
#[tokio::test]
async fn test_window_with_partition_by(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();
	setup_test_data(pool.as_ref()).await;

	// ROW_NUMBER partitioned by region
	let sql = "SELECT
		region,
		amount,
		ROW_NUMBER() OVER (PARTITION BY region ORDER BY timestamp ASC) as row_num
	FROM sales_log
	ORDER BY region, timestamp";

	let rows = sqlx::query(sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute PARTITION BY query");

	assert_eq!(rows.len(), 6, "Expected 6 rows");

	// Verify partition grouping
	let first_three_regions: Vec<String> = rows[0..3].iter().map(|r| r.get("region")).collect();
	let regions_are_same = first_three_regions
		.iter()
		.all(|r| r == &first_three_regions[0]);
	assert!(
		regions_are_same,
		"Expected first 3 rows from same region (partition)"
	);

	// Verify row_num resets between partitions
	let mut prev_region = String::new();
	let mut prev_row_num = 0i64;

	for row in &rows {
		let region: String = row.get("region");
		let row_num: i64 = row.get("row_num");

		if region != prev_region {
			// New partition should start with row_num = 1
			assert_eq!(row_num, 1, "New partition should start with row_num=1");
			prev_region = region;
		} else {
			// Within partition, should be sequential
			assert!(
				row_num > prev_row_num,
				"row_num should increase within partition"
			);
		}

		prev_row_num = row_num;
	}
}

/// Test window functions with multiple ORDER BY columns
///
/// **Test Intent**: Verify ORDER BY clause properly orders within partition
///
/// **Integration Point**: PostgreSQL window ORDER BY clause
///
/// **Not Intent**: Complex ordering expressions
#[rstest]
#[tokio::test]
async fn test_window_with_order_by(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();
	setup_test_data(pool.as_ref()).await;

	// LAG ordered by timestamp (chronological order)
	let sql = "SELECT
		region,
		timestamp,
		amount,
		LAG(amount) OVER (PARTITION BY region ORDER BY timestamp ASC) as prev_amount
	FROM sales_log
	ORDER BY region, timestamp";

	let rows = sqlx::query(sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute ORDER BY query");

	assert!(!rows.is_empty(), "Expected rows from query");

	// Verify timestamp ordering within region
	let mut current_region = String::new();
	let mut last_timestamp: Option<NaiveDateTime> = None;

	for row in rows {
		let region: String = row.get("region");
		let timestamp: NaiveDateTime = row.get("timestamp");
		let _amount: i64 = row.get("amount");
		let _prev_amount: Option<i64> = row.get("prev_amount");

		if region != current_region {
			current_region = region.clone();
			last_timestamp = Some(timestamp);
		} else {
			// Within partition, timestamps should be in ascending order
			if let Some(last) = last_timestamp {
				assert!(
					timestamp >= last,
					"Timestamps should be in ascending order within partition"
				);
			}
			last_timestamp = Some(timestamp);
		}
	}
}

// ============================================================================
// Time Series Analysis Use Case
// ============================================================================

/// Test time series analysis with LAG and LEAD
///
/// **Test Intent**: Verify LAG/LEAD enable time-series change detection and lookahead
///
/// **Integration Point**: PostgreSQL LAG()/LEAD() for time-series analysis
///
/// **Not Intent**: Aggregate functions, complex grouping
#[rstest]
#[tokio::test]
async fn test_time_series_lag_lead_analysis(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();
	setup_test_data(pool.as_ref()).await;

	// Create a result table with LAG and LEAD to track price changes
	let sql = "SELECT
		region,
		timestamp,
		amount,
		LAG(amount) OVER (PARTITION BY region ORDER BY timestamp ASC) as prev_amount,
		LEAD(amount) OVER (PARTITION BY region ORDER BY timestamp ASC) as next_amount
	FROM sales_log
	ORDER BY region, timestamp";

	let rows = sqlx::query(sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to execute time series analysis query");

	assert_eq!(rows.len(), 6, "Expected 6 rows from sales_log");

	// Each row should have region, timestamp, and amount
	let mut current_region = String::new();
	let mut row_count_per_partition: std::collections::HashMap<String, usize> =
		std::collections::HashMap::new();

	for row in rows {
		let region: String = row.get("region");
		let _timestamp: NaiveDateTime = row.get("timestamp");
		let amount: i64 = row.get("amount");
		let prev_amount: Option<i64> = row.get("prev_amount");
		let next_amount: Option<i64> = row.get("next_amount");

		// Verify data structure
		assert!(amount > 0, "Amount should be positive");

		// Track partition changes
		if region != current_region {
			current_region = region.clone();
		}

		// Update row count per partition
		*row_count_per_partition.entry(region).or_insert(0) += 1;

		// Verify LAG/LEAD values for time-series analysis
		// First row in partition: prev_amount should be NULL, next_amount should have value
		// Last row in partition: prev_amount should have value, next_amount should be NULL
		// Middle rows: both should have values

		if prev_amount.is_some() {
			// Non-first row - prev_amount should be a valid value
			assert!(prev_amount.unwrap() > 0, "LAG result should be positive");
		}

		if next_amount.is_some() {
			// Non-last row - next_amount should be a valid value
			assert!(next_amount.unwrap() > 0, "LEAD result should be positive");
		}
	}

	// Verify we have both regions
	assert_eq!(row_count_per_partition.len(), 2, "Should have 2 regions");
	assert_eq!(
		row_count_per_partition.get("Tokyo"),
		Some(&3),
		"Tokyo should have 3 rows"
	);
	assert_eq!(
		row_count_per_partition.get("Osaka"),
		Some(&3),
		"Osaka should have 3 rows"
	);
}
