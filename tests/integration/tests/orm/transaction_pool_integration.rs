//! ORM Transaction + Pool Integration Tests
//!
//! Tests transaction management integration with connection pooling, covering:
//! - Transaction lifecycle with pool connections
//! - Isolation levels
//! - Savepoints and nested transactions
//! - Connection reuse after transactions
//! - Transaction rollback and error handling
//! - Pool behavior during concurrent transactions
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container

use reinhardt_db::pool::{ConnectionPool, PoolConfig};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Transaction Lifecycle Tests
// ============================================================================

/// Test basic transaction commit with pool connection
///
/// **Test Intent**: Verify transaction successfully commits changes when using
/// pooled connection
///
/// **Integration Point**: Transaction → Pool connection lifecycle
///
/// **Not Intent**: Rollback behavior, savepoints
#[rstest]
#[tokio::test]
async fn test_transaction_commit_with_pool(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Create test table
	sqlx::query("CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name TEXT NOT NULL)")
		.execute(pool.inner())
		.await
		.expect("Failed to create table");

	// Begin transaction
	let mut tx = pool
		.inner()
		.begin()
		.await
		.expect("Failed to begin transaction");

	// Insert data within transaction
	sqlx::query("INSERT INTO users (name) VALUES ($1)")
		.bind("Alice")
		.execute(&mut *tx)
		.await
		.expect("Failed to insert");

	// Commit transaction
	tx.commit().await.expect("Failed to commit transaction");

	// Verify data is persisted
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to count");

	assert_eq!(count, 1, "Transaction should commit successfully");
}

/// Test transaction rollback with pool connection
///
/// **Test Intent**: Verify transaction rollback discards changes when using
/// pooled connection
///
/// **Integration Point**: Transaction rollback → Pool connection cleanup
///
/// **Not Intent**: Commit behavior, savepoints
#[rstest]
#[tokio::test]
async fn test_transaction_rollback_with_pool(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Create test table
	sqlx::query("CREATE TABLE IF NOT EXISTS products (id SERIAL PRIMARY KEY, name TEXT NOT NULL)")
		.execute(pool.inner())
		.await
		.expect("Failed to create table");

	// Begin transaction
	let mut tx = pool
		.inner()
		.begin()
		.await
		.expect("Failed to begin transaction");

	// Insert data within transaction
	sqlx::query("INSERT INTO products (name) VALUES ($1)")
		.bind("Product A")
		.execute(&mut *tx)
		.await
		.expect("Failed to insert");

	// Rollback transaction
	tx.rollback().await.expect("Failed to rollback transaction");

	// Verify data is NOT persisted
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM products")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to count");

	assert_eq!(count, 0, "Transaction rollback should discard changes");
}

/// Test transaction auto-rollback on drop
///
/// **Test Intent**: Verify transaction automatically rolls back when dropped
/// without explicit commit
///
/// **Integration Point**: Transaction Drop trait → Pool connection cleanup
///
/// **Not Intent**: Explicit rollback, commit behavior
#[rstest]
#[tokio::test]
async fn test_transaction_auto_rollback_on_drop(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Create test table
	sqlx::query("CREATE TABLE IF NOT EXISTS orders (id SERIAL PRIMARY KEY, total BIGINT NOT NULL)")
		.execute(pool.inner())
		.await
		.expect("Failed to create table");

	{
		// Begin transaction in inner scope
		let mut tx = pool
			.inner()
			.begin()
			.await
			.expect("Failed to begin transaction");

		// Insert data within transaction
		sqlx::query("INSERT INTO orders (total) VALUES ($1)")
			.bind(100_i64)
			.execute(&mut *tx)
			.await
			.expect("Failed to insert");

		// Transaction dropped here without commit
	}

	// Verify data is NOT persisted (auto-rollback)
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to count");

	assert_eq!(count, 0, "Transaction should auto-rollback on drop");
}

// ============================================================================
// Isolation Level Tests
// ============================================================================

/// Test read committed isolation level
///
/// **Test Intent**: Verify READ COMMITTED isolation prevents dirty reads
/// but allows non-repeatable reads
///
/// **Integration Point**: Isolation level configuration → Transaction behavior
///
/// **Not Intent**: Other isolation levels, phantom reads
#[rstest]
#[tokio::test]
async fn test_read_committed_isolation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default();
	let pool = Arc::new(
		ConnectionPool::new_postgres(&url, config)
			.await
			.expect("Failed to create pool"),
	);

	// Create test table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS accounts (id SERIAL PRIMARY KEY, balance BIGINT NOT NULL)",
	)
	.execute(pool.inner())
	.await
	.expect("Failed to create table");

	// Insert initial data
	sqlx::query("INSERT INTO accounts (id, balance) VALUES ($1, $2)")
		.bind(1)
		.bind(1000_i64)
		.execute(pool.inner())
		.await
		.expect("Failed to insert");

	// Transaction 1: Read initial balance
	let mut tx1 = pool.inner().begin().await.expect("Failed to begin tx1");

	sqlx::query("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
		.execute(&mut *tx1)
		.await
		.expect("Failed to set isolation level");

	let balance1: i64 = sqlx::query_scalar("SELECT balance FROM accounts WHERE id = $1")
		.bind(1)
		.fetch_one(&mut *tx1)
		.await
		.expect("Failed to read balance");

	assert_eq!(balance1, 1000);

	// Transaction 2: Update balance and commit
	let mut tx2 = pool.inner().begin().await.expect("Failed to begin tx2");

	sqlx::query("UPDATE accounts SET balance = $1 WHERE id = $2")
		.bind(1500_i64)
		.bind(1)
		.execute(&mut *tx2)
		.await
		.expect("Failed to update");

	tx2.commit().await.expect("Failed to commit tx2");

	// Transaction 1: Read again (should see committed changes)
	let balance2: i64 = sqlx::query_scalar("SELECT balance FROM accounts WHERE id = $1")
		.bind(1)
		.fetch_one(&mut *tx1)
		.await
		.expect("Failed to read balance again");

	assert_eq!(
		balance2, 1500,
		"READ COMMITTED should see committed changes"
	);

	tx1.rollback().await.expect("Failed to rollback tx1");
}

/// Test repeatable read isolation level
///
/// **Test Intent**: Verify REPEATABLE READ isolation prevents non-repeatable reads
/// by maintaining consistent snapshot
///
/// **Integration Point**: Isolation level → Snapshot isolation behavior
///
/// **Not Intent**: Read committed, serializable
#[rstest]
#[tokio::test]
async fn test_repeatable_read_isolation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default();
	let pool = Arc::new(
		ConnectionPool::new_postgres(&url, config)
			.await
			.expect("Failed to create pool"),
	);

	// Create test table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS inventory (id SERIAL PRIMARY KEY, quantity BIGINT NOT NULL)",
	)
	.execute(pool.inner())
	.await
	.expect("Failed to create table");

	// Insert initial data
	sqlx::query("INSERT INTO inventory (id, quantity) VALUES ($1, $2)")
		.bind(1)
		.bind(100_i64)
		.execute(pool.inner())
		.await
		.expect("Failed to insert");

	// Transaction 1: Read initial quantity with REPEATABLE READ
	let mut tx1 = pool.inner().begin().await.expect("Failed to begin tx1");

	sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
		.execute(&mut *tx1)
		.await
		.expect("Failed to set isolation level");

	let qty1: i64 = sqlx::query_scalar("SELECT quantity FROM inventory WHERE id = $1")
		.bind(1)
		.fetch_one(&mut *tx1)
		.await
		.expect("Failed to read quantity");

	assert_eq!(qty1, 100);

	// Transaction 2: Update quantity and commit
	let mut tx2 = pool.inner().begin().await.expect("Failed to begin tx2");

	sqlx::query("UPDATE inventory SET quantity = $1 WHERE id = $2")
		.bind(150_i64)
		.bind(1)
		.execute(&mut *tx2)
		.await
		.expect("Failed to update");

	tx2.commit().await.expect("Failed to commit tx2");

	// Transaction 1: Read again (should still see original value)
	let qty2: i64 = sqlx::query_scalar("SELECT quantity FROM inventory WHERE id = $1")
		.bind(1)
		.fetch_one(&mut *tx1)
		.await
		.expect("Failed to read quantity again");

	assert_eq!(qty2, 100, "REPEATABLE READ should maintain snapshot");

	tx1.rollback().await.expect("Failed to rollback tx1");
}

// ============================================================================
// Savepoint Tests
// ============================================================================

/// Test savepoint creation and rollback
///
/// **Test Intent**: Verify savepoint allows partial rollback within transaction
///
/// **Integration Point**: Savepoint → Transaction state management
///
/// **Not Intent**: Full transaction rollback, nested transactions
#[rstest]
#[tokio::test]
async fn test_savepoint_rollback(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Create test table
	sqlx::query("CREATE TABLE IF NOT EXISTS logs (id SERIAL PRIMARY KEY, message TEXT NOT NULL)")
		.execute(pool.inner())
		.await
		.expect("Failed to create table");

	// Begin transaction
	let mut tx = pool
		.inner()
		.begin()
		.await
		.expect("Failed to begin transaction");

	// Insert first record
	sqlx::query("INSERT INTO logs (message) VALUES ($1)")
		.bind("First log")
		.execute(&mut *tx)
		.await
		.expect("Failed to insert first");

	// Create savepoint
	sqlx::query("SAVEPOINT sp1")
		.execute(&mut *tx)
		.await
		.expect("Failed to create savepoint");

	// Insert second record
	sqlx::query("INSERT INTO logs (message) VALUES ($1)")
		.bind("Second log")
		.execute(&mut *tx)
		.await
		.expect("Failed to insert second");

	// Rollback to savepoint
	sqlx::query("ROLLBACK TO SAVEPOINT sp1")
		.execute(&mut *tx)
		.await
		.expect("Failed to rollback to savepoint");

	// Commit transaction
	tx.commit().await.expect("Failed to commit");

	// Verify only first record persisted
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM logs")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to count");

	assert_eq!(count, 1, "Savepoint rollback should discard second insert");

	let message: String = sqlx::query_scalar("SELECT message FROM logs WHERE id = 1")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to get message");

	assert_eq!(message, "First log");
}

/// Test nested savepoints
///
/// **Test Intent**: Verify multiple savepoints can be created and rolled back
/// independently within single transaction
///
/// **Integration Point**: Nested savepoints → Transaction state stack
///
/// **Not Intent**: Single savepoint, full rollback
#[rstest]
#[tokio::test]
async fn test_nested_savepoints(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Create test table
	sqlx::query("CREATE TABLE IF NOT EXISTS events (id SERIAL PRIMARY KEY, name TEXT NOT NULL)")
		.execute(pool.inner())
		.await
		.expect("Failed to create table");

	// Begin transaction
	let mut tx = pool
		.inner()
		.begin()
		.await
		.expect("Failed to begin transaction");

	// Insert event 1
	sqlx::query("INSERT INTO events (name) VALUES ($1)")
		.bind("Event 1")
		.execute(&mut *tx)
		.await
		.expect("Failed to insert event 1");

	// Create savepoint 1
	sqlx::query("SAVEPOINT sp1")
		.execute(&mut *tx)
		.await
		.expect("Failed to create sp1");

	// Insert event 2
	sqlx::query("INSERT INTO events (name) VALUES ($1)")
		.bind("Event 2")
		.execute(&mut *tx)
		.await
		.expect("Failed to insert event 2");

	// Create savepoint 2
	sqlx::query("SAVEPOINT sp2")
		.execute(&mut *tx)
		.await
		.expect("Failed to create sp2");

	// Insert event 3
	sqlx::query("INSERT INTO events (name) VALUES ($1)")
		.bind("Event 3")
		.execute(&mut *tx)
		.await
		.expect("Failed to insert event 3");

	// Rollback to sp2 (discards event 3)
	sqlx::query("ROLLBACK TO SAVEPOINT sp2")
		.execute(&mut *tx)
		.await
		.expect("Failed to rollback to sp2");

	// Commit transaction
	tx.commit().await.expect("Failed to commit");

	// Verify event 1 and 2 persisted, event 3 discarded
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to count");

	assert_eq!(
		count, 2,
		"Should have 2 events after nested savepoint rollback"
	);

	let names: Vec<String> = sqlx::query_scalar("SELECT name FROM events ORDER BY id")
		.fetch_all(pool.inner())
		.await
		.expect("Failed to get names");

	assert_eq!(names, vec!["Event 1", "Event 2"]);
}

// ============================================================================
// Connection Reuse Tests
// ============================================================================

/// Test connection reuse after transaction commit
///
/// **Test Intent**: Verify pooled connection is returned and reusable after
/// transaction commits
///
/// **Integration Point**: Transaction commit → Pool connection reuse
///
/// **Not Intent**: Connection leak, rollback behavior
#[rstest]
#[tokio::test]
async fn test_connection_reuse_after_commit(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default().with_max_connections(2);
	let pool = Arc::new(
		ConnectionPool::new_postgres(&url, config)
			.await
			.expect("Failed to create pool"),
	);

	// Create test table
	sqlx::query("CREATE TABLE IF NOT EXISTS reuse_test (id SERIAL PRIMARY KEY, value INT)")
		.execute(pool.inner())
		.await
		.expect("Failed to create table");

	// First transaction
	{
		let mut tx = pool.inner().begin().await.expect("Failed to begin tx1");

		sqlx::query("INSERT INTO reuse_test (value) VALUES ($1)")
			.bind(1)
			.execute(&mut *tx)
			.await
			.expect("Failed to insert");

		tx.commit().await.expect("Failed to commit tx1");
	}

	// Second transaction (should reuse connection)
	{
		let mut tx = pool.inner().begin().await.expect("Failed to begin tx2");

		sqlx::query("INSERT INTO reuse_test (value) VALUES ($1)")
			.bind(2)
			.execute(&mut *tx)
			.await
			.expect("Failed to insert");

		tx.commit().await.expect("Failed to commit tx2");
	}

	// Verify both inserts succeeded
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reuse_test")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to count");

	assert_eq!(count, 2, "Connection should be reused after commit");
}

/// Test connection reuse after transaction rollback
///
/// **Test Intent**: Verify pooled connection is returned and reusable after
/// transaction rolls back
///
/// **Integration Point**: Transaction rollback → Pool connection reuse
///
/// **Not Intent**: Commit behavior, connection leak
#[rstest]
#[tokio::test]
async fn test_connection_reuse_after_rollback(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default().with_max_connections(2);
	let pool = Arc::new(
		ConnectionPool::new_postgres(&url, config)
			.await
			.expect("Failed to create pool"),
	);

	// Create test table
	sqlx::query("CREATE TABLE IF NOT EXISTS rollback_test (id SERIAL PRIMARY KEY, value INT)")
		.execute(pool.inner())
		.await
		.expect("Failed to create table");

	// First transaction (rollback)
	{
		let mut tx = pool.inner().begin().await.expect("Failed to begin tx1");

		sqlx::query("INSERT INTO rollback_test (value) VALUES ($1)")
			.bind(10)
			.execute(&mut *tx)
			.await
			.expect("Failed to insert");

		tx.rollback().await.expect("Failed to rollback tx1");
	}

	// Second transaction (should reuse connection)
	{
		let mut tx = pool.inner().begin().await.expect("Failed to begin tx2");

		sqlx::query("INSERT INTO rollback_test (value) VALUES ($1)")
			.bind(20)
			.execute(&mut *tx)
			.await
			.expect("Failed to insert");

		tx.commit().await.expect("Failed to commit tx2");
	}

	// Verify only second insert persisted
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rollback_test")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to count");

	assert_eq!(count, 1, "Connection should be reused after rollback");

	let value: i32 = sqlx::query_scalar("SELECT value FROM rollback_test WHERE id = 1")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to get value");

	assert_eq!(value, 20);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

/// Test transaction error handling
///
/// **Test Intent**: Verify transaction correctly handles SQL errors and
/// allows recovery
///
/// **Integration Point**: SQL error → Transaction state + Pool recovery
///
/// **Not Intent**: Successful operations, savepoints
#[rstest]
#[tokio::test]
async fn test_transaction_error_handling(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Create test table with UNIQUE constraint
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS unique_test (id SERIAL PRIMARY KEY, email TEXT UNIQUE NOT NULL)",
	)
	.execute(pool.inner())
	.await
	.expect("Failed to create table");

	// Begin transaction
	let mut tx = pool
		.inner()
		.begin()
		.await
		.expect("Failed to begin transaction");

	// Insert first record
	sqlx::query("INSERT INTO unique_test (email) VALUES ($1)")
		.bind("test@example.com")
		.execute(&mut *tx)
		.await
		.expect("Failed to insert first");

	// Attempt to insert duplicate (should fail)
	let result = sqlx::query("INSERT INTO unique_test (email) VALUES ($1)")
		.bind("test@example.com")
		.execute(&mut *tx)
		.await;

	assert!(result.is_err(), "Duplicate insert should fail");

	// Rollback due to error
	tx.rollback().await.expect("Failed to rollback after error");

	// Pool should still work
	let mut new_tx = pool
		.inner()
		.begin()
		.await
		.expect("Failed to begin new transaction after error");

	sqlx::query("INSERT INTO unique_test (email) VALUES ($1)")
		.bind("valid@example.com")
		.execute(&mut *new_tx)
		.await
		.expect("Failed to insert in new transaction");

	new_tx
		.commit()
		.await
		.expect("Failed to commit new transaction");

	// Verify only second insert persisted
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM unique_test")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to count");

	assert_eq!(count, 1, "Pool should recover from transaction error");
}

// ============================================================================
// Concurrent Transaction Tests
// ============================================================================

/// Test concurrent transactions with pool
///
/// **Test Intent**: Verify pool correctly handles multiple concurrent transactions
/// without interference
///
/// **Integration Point**: Concurrent transactions → Pool connection isolation
///
/// **Not Intent**: Sequential transactions, single connection
#[rstest]
#[tokio::test]
async fn test_concurrent_transactions(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default().with_max_connections(5);
	let pool = Arc::new(
		ConnectionPool::new_postgres(&url, config)
			.await
			.expect("Failed to create pool"),
	);

	// Create test table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS concurrent_test (id SERIAL PRIMARY KEY, thread_id INT)",
	)
	.execute(pool.inner())
	.await
	.expect("Failed to create table");

	// Spawn multiple concurrent transactions
	let mut handles = Vec::new();
	for i in 0..3 {
		let pool_clone = Arc::clone(&pool);
		let handle = tokio::spawn(async move {
			let mut tx = pool_clone
				.inner()
				.begin()
				.await
				.expect("Failed to begin transaction");

			sqlx::query("INSERT INTO concurrent_test (thread_id) VALUES ($1)")
				.bind(i)
				.execute(&mut *tx)
				.await
				.expect("Failed to insert");

			// Simulate some work
			tokio::time::sleep(Duration::from_millis(10)).await;

			tx.commit().await.expect("Failed to commit");
		});
		handles.push(handle);
	}

	// Wait for all transactions to complete
	for handle in handles {
		handle.await.expect("Task panicked");
	}

	// Verify all inserts succeeded
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM concurrent_test")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to count");

	assert_eq!(count, 3, "All concurrent transactions should succeed");
}

/// Test concurrent transactions with lock contention
///
/// **Test Intent**: Verify pool handles lock contention between concurrent transactions
/// correctly with proper waiting/timeout
///
/// **Integration Point**: Row locks → Transaction blocking + Pool timeout
///
/// **Not Intent**: No contention, single transaction
#[rstest]
#[tokio::test]
async fn test_concurrent_transactions_with_locks(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default().with_max_connections(5);
	let pool = Arc::new(
		ConnectionPool::new_postgres(&url, config)
			.await
			.expect("Failed to create pool"),
	);

	// Create test table
	sqlx::query("CREATE TABLE IF NOT EXISTS locked_test (id INT PRIMARY KEY, value INT)")
		.execute(pool.inner())
		.await
		.expect("Failed to create table");

	// Insert initial row
	sqlx::query("INSERT INTO locked_test (id, value) VALUES ($1, $2)")
		.bind(1)
		.bind(0)
		.execute(pool.inner())
		.await
		.expect("Failed to insert initial row");

	// Transaction 1: Lock row with FOR UPDATE
	let pool1 = Arc::clone(&pool);
	let handle1 = tokio::spawn(async move {
		let mut tx = pool1.inner().begin().await.expect("Failed to begin tx1");

		// Lock row
		sqlx::query("SELECT value FROM locked_test WHERE id = $1 FOR UPDATE")
			.bind(1)
			.fetch_one(&mut *tx)
			.await
			.expect("Failed to lock row");

		// Hold lock for a bit
		tokio::time::sleep(Duration::from_millis(500)).await;

		// Update value
		sqlx::query("UPDATE locked_test SET value = $1 WHERE id = $2")
			.bind(100)
			.bind(1)
			.execute(&mut *tx)
			.await
			.expect("Failed to update");

		tx.commit().await.expect("Failed to commit tx1");
	});

	// Wait a bit to ensure tx1 acquires lock first
	tokio::time::sleep(Duration::from_millis(100)).await;

	// Transaction 2: Try to update same row (will wait for lock)
	let pool2 = Arc::clone(&pool);
	let handle2 = tokio::spawn(async move {
		let mut tx = pool2.inner().begin().await.expect("Failed to begin tx2");

		// This will wait for tx1's lock to release
		sqlx::query("UPDATE locked_test SET value = value + $1 WHERE id = $2")
			.bind(50)
			.bind(1)
			.execute(&mut *tx)
			.await
			.expect("Failed to update");

		tx.commit().await.expect("Failed to commit tx2");
	});

	// Wait for both transactions
	handle1.await.expect("tx1 panicked");
	handle2.await.expect("tx2 panicked");

	// Verify final value (should be 100 + 50 = 150)
	let final_value: i32 = sqlx::query_scalar("SELECT value FROM locked_test WHERE id = $1")
		.bind(1)
		.fetch_one(pool.inner())
		.await
		.expect("Failed to get final value");

	assert_eq!(
		final_value, 150,
		"Lock contention should be handled correctly"
	);
}
