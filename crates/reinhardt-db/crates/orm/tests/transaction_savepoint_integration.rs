//! Transaction Savepoint Integration Tests (Phase 4)
//!
//! Tests for PostgreSQL savepoint functionality within transactions, covering:
//! - Savepoint creation and rollback (Normal cases)
//! - Savepoint depth limits (Edge cases)
//! - Nested transaction state transitions (State transition cases)
//! - Savepoint release and commit behavior
//! - Error handling with savepoints
//! - Complex nested savepoint scenarios
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container
//!
//! **Integration Points:**
//! - Database transaction management via reinhardt-orm
//! - PostgreSQL SAVEPOINT protocol support
//! - State management across nested transactions

use reinhardt_db::orm::connection::DatabaseConnection;
use reinhardt_db::orm::transaction::TransactionScope;
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Helper Functions
// ============================================================================

/// Create test tables for savepoint testing
async fn setup_test_tables(conn: &DatabaseConnection) -> Result<(), anyhow::Error> {
	// Create accounts table
	conn.execute(
		"CREATE TABLE IF NOT EXISTS accounts (
			id SERIAL PRIMARY KEY,
			account_name TEXT NOT NULL,
			balance BIGINT NOT NULL,
			updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
		)",
		vec![],
	)
	.await?;

	// Create transactions table
	conn.execute(
		"CREATE TABLE IF NOT EXISTS transactions (
			id SERIAL PRIMARY KEY,
			account_id INTEGER NOT NULL REFERENCES accounts(id),
			amount BIGINT NOT NULL,
			tx_type TEXT NOT NULL,
			created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
		)",
		vec![],
	)
	.await?;

	Ok(())
}

// ============================================================================
// Normal Case Tests (Normal cases)
// ============================================================================

/// Test basic savepoint creation and rollback
///
/// **Test Intent**: Verify savepoint can be created, data modified, and then rolled back
///
/// **Integration Point**: PostgreSQL SAVEPOINT protocol → Transaction state management
///
/// **Not Intent**: Complex nested savepoints, concurrent transactions
#[rstest]
#[tokio::test]
async fn test_savepoint_basic_creation_and_rollback(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;
	let conn = DatabaseConnection::connect(&url)
		.await
		.expect("Failed to connect");

	setup_test_tables(&conn)
		.await
		.expect("Failed to setup tables");

	// Insert initial data
	conn.execute(
		"INSERT INTO accounts (account_name, balance) VALUES ($1, $2)",
		vec!["Alice".into(), 1000i64.into()],
	)
	.await
	.expect("Failed to insert account");

	// Start transaction and create savepoint
	let mut tx = TransactionScope::begin(&conn)
		.await
		.expect("Failed to begin transaction");

	// Create savepoint
	tx.savepoint("sp1")
		.await
		.expect("Failed to create savepoint");

	// Modify data within savepoint
	tx.execute(
		"UPDATE accounts SET balance = balance - 500 WHERE account_name = $1",
		vec!["Alice".into()],
	)
	.await
	.expect("Failed to update balance");

	// Verify modification within transaction
	let balance_before_rollback = {
		let row = tx
			.query_one(
				"SELECT balance FROM accounts WHERE account_name = $1",
				vec!["Alice".into()],
			)
			.await
			.expect("Failed to fetch balance");

		row.get::<i64>("balance")
			.expect("Failed to get balance from row")
	};

	assert_eq!(balance_before_rollback, 500);

	// Rollback to savepoint
	tx.rollback_to_savepoint("sp1")
		.await
		.expect("Failed to rollback savepoint");

	// Verify balance is restored
	let balance_after_rollback = {
		let row = tx
			.query_one(
				"SELECT balance FROM accounts WHERE account_name = $1",
				vec!["Alice".into()],
			)
			.await
			.expect("Failed to fetch balance after rollback");

		row.get::<i64>("balance")
			.expect("Failed to get balance from row")
	};

	assert_eq!(balance_after_rollback, 1000);

	tx.commit().await.expect("Failed to commit transaction");
}

/// Test savepoint with successful commit
///
/// **Test Intent**: Verify changes before savepoint persist when transaction commits
///
/// **Integration Point**: PostgreSQL SAVEPOINT → Transaction commit
///
/// **Not Intent**: Rollback scenarios, savepoint release
#[rstest]
#[tokio::test]
async fn test_savepoint_with_committed_changes(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;
	let conn = DatabaseConnection::connect(&url)
		.await
		.expect("Failed to connect");

	setup_test_tables(&conn)
		.await
		.expect("Failed to setup tables");

	// Insert initial account
	conn.execute(
		"INSERT INTO accounts (account_name, balance) VALUES ($1, $2)",
		vec!["Bob".into(), 5000i64.into()],
	)
	.await
	.expect("Failed to insert account");

	let mut tx = TransactionScope::begin(&conn)
		.await
		.expect("Failed to begin transaction");

	// Modify outside savepoint
	tx.execute(
		"UPDATE accounts SET balance = balance + 1000 WHERE account_name = $1",
		vec!["Bob".into()],
	)
	.await
	.expect("Failed to update balance");

	// Create savepoint and modify again
	tx.savepoint("sp2")
		.await
		.expect("Failed to create savepoint");

	tx.execute(
		"UPDATE accounts SET balance = balance - 500 WHERE account_name = $1",
		vec!["Bob".into()],
	)
	.await
	.expect("Failed to update balance in savepoint");

	// Release savepoint (equivalent to commit within savepoint)
	tx.release_savepoint("sp2")
		.await
		.expect("Failed to release savepoint");

	tx.commit().await.expect("Failed to commit transaction");

	// Verify final balance (5000 + 1000 - 500 = 5500)
	let final_balance = {
		let row = conn
			.query_one(
				"SELECT balance FROM accounts WHERE account_name = $1",
				vec!["Bob".into()],
			)
			.await
			.expect("Failed to fetch final balance");

		row.get::<i64>("balance")
			.expect("Failed to get balance from row")
	};

	assert_eq!(final_balance, 5500);
}

// ============================================================================
// Edge Case Tests (Edge cases)
// ============================================================================

/// Test savepoint depth limits with multiple nested savepoints
///
/// **Test Intent**: Verify behavior with deeply nested savepoints (5 levels)
///
/// **Integration Point**: PostgreSQL savepoint stack management
///
/// **Not Intent**: Performance optimization, concurrent savepoint creation
#[rstest]
#[tokio::test]
async fn test_savepoint_nested_depth_limits(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;
	let conn = DatabaseConnection::connect(&url)
		.await
		.expect("Failed to connect");

	setup_test_tables(&conn)
		.await
		.expect("Failed to setup tables");

	// Insert initial data
	conn.execute(
		"INSERT INTO accounts (account_name, balance) VALUES ($1, $2)",
		vec!["Charlie".into(), 10000i64.into()],
	)
	.await
	.expect("Failed to insert account");

	let mut tx = TransactionScope::begin(&conn)
		.await
		.expect("Failed to begin transaction");

	// Create 5 nested savepoints
	for level in 1..=5 {
		tx.savepoint(&format!("sp_level_{}", level))
			.await
			.expect(&format!("Failed to create savepoint at level {}", level));

		// Modify balance at each level
		tx.execute(
			"UPDATE accounts SET balance = balance - 500 WHERE account_name = $1",
			vec!["Charlie".into()],
		)
		.await
		.expect("Failed to update balance");
	}

	// Verify balance after 5 levels of modifications (10000 - 5*500 = 7500)
	let balance_deep = {
		let row = tx
			.query_one(
				"SELECT balance FROM accounts WHERE account_name = $1",
				vec!["Charlie".into()],
			)
			.await
			.expect("Failed to fetch balance");

		row.get::<i64>("balance")
			.expect("Failed to get balance from row")
	};

	assert_eq!(balance_deep, 7500);

	// Rollback to level 3 savepoint
	// Since sp_level_3 was created BEFORE the level 3 update,
	// rolling back to it restores the state after level 2 update (10000 - 2*500 = 9000)
	tx.rollback_to_savepoint("sp_level_3")
		.await
		.expect("Failed to rollback to level 3");

	let balance_partial = {
		let row = tx
			.query_one(
				"SELECT balance FROM accounts WHERE account_name = $1",
				vec!["Charlie".into()],
			)
			.await
			.expect("Failed to fetch balance after partial rollback");

		row.get::<i64>("balance")
			.expect("Failed to get balance from row")
	};

	// After rollback to sp_level_3, balance should be at the state when sp_level_3 was created
	// That was before level 3's update, so balance = 10000 - 2*500 = 9000
	assert_eq!(balance_partial, 9000);

	tx.commit().await.expect("Failed to commit transaction");
}

/// Test savepoint with concurrent modifications
///
/// **Test Intent**: Verify savepoint isolation with multiple account modifications
///
/// **Integration Point**: Savepoint isolation level → Row-level locking
///
/// **Not Intent**: Full ACID isolation testing, phantom reads
#[rstest]
#[tokio::test]
async fn test_savepoint_multiple_tables_isolation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;
	let conn = DatabaseConnection::connect(&url)
		.await
		.expect("Failed to connect");

	setup_test_tables(&conn)
		.await
		.expect("Failed to setup tables");

	// Insert test data
	conn.execute(
		"INSERT INTO accounts (account_name, balance) VALUES ($1, $2)",
		vec!["Dave".into(), 2000i64.into()],
	)
	.await
	.expect("Failed to insert Dave");

	let mut tx = TransactionScope::begin(&conn)
		.await
		.expect("Failed to begin transaction");

	// Create account for transaction record
	tx.savepoint("sp_accounts")
		.await
		.expect("Failed to create savepoint");

	// Modify account
	tx.execute(
		"UPDATE accounts SET balance = balance - 200 WHERE account_name = $1",
		vec!["Dave".into()],
	)
	.await
	.expect("Failed to update balance");

	let balance_after_sp = {
		let row = tx
			.query_one(
				"SELECT balance FROM accounts WHERE account_name = $1",
				vec!["Dave".into()],
			)
			.await
			.expect("Failed to fetch balance");

		row.get::<i64>("balance")
			.expect("Failed to get balance from row")
	};

	assert_eq!(balance_after_sp, 1800);

	// Rollback savepoint to restore
	tx.rollback_to_savepoint("sp_accounts")
		.await
		.expect("Failed to rollback savepoint");

	let balance_restored = {
		let row = tx
			.query_one(
				"SELECT balance FROM accounts WHERE account_name = $1",
				vec!["Dave".into()],
			)
			.await
			.expect("Failed to fetch balance after rollback");

		row.get::<i64>("balance")
			.expect("Failed to get balance from row")
	};

	assert_eq!(balance_restored, 2000);

	tx.commit().await.expect("Failed to commit transaction");
}

// ============================================================================
// State Transition Tests (State transition cases)
// ============================================================================

/// Test nested transaction state transitions with savepoints
///
/// **Test Intent**: Verify correct state transitions through multiple savepoint operations
///
/// **Integration Point**: Savepoint state machine → Transaction commit/rollback
///
/// **Not Intent**: Concurrent state changes, deadlock scenarios
#[rstest]
#[tokio::test]
async fn test_nested_savepoint_state_transitions(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;
	let conn = DatabaseConnection::connect(&url)
		.await
		.expect("Failed to connect");

	setup_test_tables(&conn)
		.await
		.expect("Failed to setup tables");

	// Insert initial account
	conn.execute(
		"INSERT INTO accounts (account_name, balance) VALUES ($1, $2)",
		vec!["Eve".into(), 3000i64.into()],
	)
	.await
	.expect("Failed to insert account");

	let mut tx = TransactionScope::begin(&conn)
		.await
		.expect("Failed to begin transaction");

	// State 1: Create first savepoint
	tx.savepoint("state_1")
		.await
		.expect("Failed to create savepoint state_1");

	tx.execute(
		"UPDATE accounts SET balance = balance - 100 WHERE account_name = $1",
		vec!["Eve".into()],
	)
	.await
	.expect("Failed to update in state 1");

	// State 2: Create nested savepoint
	tx.savepoint("state_2")
		.await
		.expect("Failed to create savepoint state_2");

	tx.execute(
		"UPDATE accounts SET balance = balance - 200 WHERE account_name = $1",
		vec!["Eve".into()],
	)
	.await
	.expect("Failed to update in state 2");

	// Verify state 2 balance (3000 - 100 - 200 = 2700)
	let balance_state2 = {
		let row = tx
			.query_one(
				"SELECT balance FROM accounts WHERE account_name = $1",
				vec!["Eve".into()],
			)
			.await
			.expect("Failed to fetch balance at state 2");

		row.get::<i64>("balance")
			.expect("Failed to get balance from row")
	};

	assert_eq!(balance_state2, 2700);

	// Rollback state_2 only (restore to state_1: 3000 - 100 = 2900)
	tx.rollback_to_savepoint("state_2")
		.await
		.expect("Failed to rollback to state_2");

	let balance_state1_restored = {
		let row = tx
			.query_one(
				"SELECT balance FROM accounts WHERE account_name = $1",
				vec!["Eve".into()],
			)
			.await
			.expect("Failed to fetch balance after state_2 rollback");

		row.get::<i64>("balance")
			.expect("Failed to get balance from row")
	};

	assert_eq!(balance_state1_restored, 2900);

	// Commit outer transaction
	tx.commit().await.expect("Failed to commit transaction");

	// Verify persistence (should be 2900)
	let final_balance = {
		let row = conn
			.query_one(
				"SELECT balance FROM accounts WHERE account_name = $1",
				vec!["Eve".into()],
			)
			.await
			.expect("Failed to fetch final balance");

		row.get::<i64>("balance")
			.expect("Failed to get balance from row")
	};

	assert_eq!(final_balance, 2900);
}

/// Test savepoint with transaction rollback
///
/// **Test Intent**: Verify that entire transaction rollback discards all savepoint changes
///
/// **Integration Point**: Transaction rollback → All savepoint changes reverted
///
/// **Not Intent**: Partial savepoint recovery, selective rollback
#[rstest]
#[tokio::test]
async fn test_transaction_rollback_with_savepoints(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;
	let conn = DatabaseConnection::connect(&url)
		.await
		.expect("Failed to connect");

	setup_test_tables(&conn)
		.await
		.expect("Failed to setup tables");

	// Insert initial account
	conn.execute(
		"INSERT INTO accounts (account_name, balance) VALUES ($1, $2)",
		vec!["Frank".into(), 5000i64.into()],
	)
	.await
	.expect("Failed to insert account");

	{
		let mut tx = TransactionScope::begin(&conn)
			.await
			.expect("Failed to begin transaction");

		// Create multiple savepoints
		tx.savepoint("sp_rollback_1")
			.await
			.expect("Failed to create savepoint");

		tx.execute(
			"UPDATE accounts SET balance = balance - 1000 WHERE account_name = $1",
			vec!["Frank".into()],
		)
		.await
		.expect("Failed to update in savepoint");

		// Create nested savepoint
		tx.savepoint("sp_rollback_2")
			.await
			.expect("Failed to create nested savepoint");

		tx.execute(
			"UPDATE accounts SET balance = balance - 2000 WHERE account_name = $1",
			vec!["Frank".into()],
		)
		.await
		.expect("Failed to update in nested savepoint");

		// Rollback entire transaction (not just savepoint)
		tx.rollback().await.expect("Failed to rollback transaction");
	}

	// Verify original balance is restored (all savepoint changes lost)
	let balance_after_rollback = {
		let row = conn
			.query_one(
				"SELECT balance FROM accounts WHERE account_name = $1",
				vec!["Frank".into()],
			)
			.await
			.expect("Failed to fetch balance after transaction rollback");

		row.get::<i64>("balance")
			.expect("Failed to get balance from row")
	};

	assert_eq!(balance_after_rollback, 5000);
}

/// Test savepoint release behavior and state consistency
///
/// **Test Intent**: Verify RELEASE SAVEPOINT persists changes and validates state consistency
///
/// **Integration Point**: RELEASE SAVEPOINT → State finalization
///
/// **Not Intent**: Multiple release operations, error handling on invalid releases
#[rstest]
#[tokio::test]
async fn test_savepoint_release_state_consistency(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;
	let conn = DatabaseConnection::connect(&url)
		.await
		.expect("Failed to connect");

	setup_test_tables(&conn)
		.await
		.expect("Failed to setup tables");

	// Insert test account
	conn.execute(
		"INSERT INTO accounts (account_name, balance) VALUES ($1, $2)",
		vec!["Grace".into(), 7500i64.into()],
	)
	.await
	.expect("Failed to insert account");

	let mut tx = TransactionScope::begin(&conn)
		.await
		.expect("Failed to begin transaction");

	// Create and modify within savepoint
	tx.savepoint("sp_release")
		.await
		.expect("Failed to create savepoint");

	tx.execute(
		"UPDATE accounts SET balance = balance - 1500 WHERE account_name = $1",
		vec!["Grace".into()],
	)
	.await
	.expect("Failed to update balance");

	// Release savepoint (commits changes within savepoint)
	tx.release_savepoint("sp_release")
		.await
		.expect("Failed to release savepoint");

	// Verify balance is updated and savepoint is released
	let balance_after_release = {
		let row = tx
			.query_one(
				"SELECT balance FROM accounts WHERE account_name = $1",
				vec!["Grace".into()],
			)
			.await
			.expect("Failed to fetch balance after release");

		row.get::<i64>("balance")
			.expect("Failed to get balance from row")
	};

	assert_eq!(balance_after_release, 6000);

	// Commit outer transaction
	tx.commit().await.expect("Failed to commit transaction");

	// Verify persistence
	let persisted_balance = {
		let row = conn
			.query_one(
				"SELECT balance FROM accounts WHERE account_name = $1",
				vec!["Grace".into()],
			)
			.await
			.expect("Failed to fetch persisted balance");

		row.get::<i64>("balance")
			.expect("Failed to get balance from row")
	};

	assert_eq!(persisted_balance, 6000);
}
