//! Integration tests for MySQL two-phase commit (XA transactions)
//!
//! These tests use TestContainers to provide an isolated MySQL instance.
//! No manual setup required - the container is automatically created and destroyed.

#![cfg(feature = "mysql")]

use reinhardt_backends::drivers::mysql::two_phase::MySqlTwoPhaseParticipant;
use rstest::*;
use serial_test::serial;
use sqlx::{MySqlPool, Row};
use std::sync::Arc;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::{GenericImage, ImageExt, runners::AsyncRunner};

type MysqlContainer = testcontainers::ContainerAsync<GenericImage>;

#[fixture]
async fn mysql_pool() -> (MysqlContainer, Arc<MySqlPool>) {
	// Start MySQL container
	// Note: testcontainers 0.25.x uses default timeout settings
	// MySQL 8.0 requires extra time for initialization
	let mysql = GenericImage::new("mysql", "8.0")
		.with_exposed_port(3306.tcp())
		.with_wait_for(WaitFor::message_on_stderr(
			"port: 3306  MySQL Community Server",
		))
		.with_startup_timeout(std::time::Duration::from_secs(120))
		.with_env_var("MYSQL_ROOT_PASSWORD", "test")
		.with_env_var("MYSQL_DATABASE", "mysql")
		.start()
		.await
		.expect("Failed to start MySQL container");

	let port = {
		let mut retries = 0;
		let max_retries = 5;
		loop {
			match mysql.get_host_port_ipv4(3306).await {
				Ok(p) => break p,
				Err(e) => {
					if retries >= max_retries {
						panic!(
							"Failed to get MySQL port after {} retries: {:?}",
							max_retries, e
						);
					}
					retries += 1;
					let backoff = std::time::Duration::from_millis(100 * (1 << retries));
					tokio::time::sleep(backoff).await;
				}
			}
		}
	};

	let database_url = format!("mysql://root:test@localhost:{}/mysql", port);

	// MySQL 8.0 may need additional time after "ready for connections" message
	// Retry connection with exponential backoff
	let pool = {
		let mut retries = 0;
		let max_retries = 10;
		loop {
			match MySqlPool::connect(&database_url).await {
				Ok(pool) => break pool,
				Err(e) => {
					if retries >= max_retries {
						panic!(
							"Failed to connect to MySQL after {} retries: {}",
							max_retries, e
						);
					}
					retries += 1;
					let backoff = std::time::Duration::from_millis(100 * (1 << retries));
					tokio::time::sleep(backoff).await;
				}
			}
		}
	};

	(mysql, Arc::new(pool))
}

async fn cleanup_xa_transactions(pool: &MySqlPool) {
	// Cleanup any existing XA transactions from previous test runs
	let rows = sqlx::query("XA RECOVER")
		.fetch_all(pool)
		.await
		.unwrap_or_default();

	// Helper function to escape XID for SQL
	fn escape_xid(xid: &str) -> String {
		xid.replace('\'', "''")
	}

	for row in rows {
		if let Ok(data) = row.try_get::<Vec<u8>, _>("data")
			&& let Ok(xid) = String::from_utf8(data)
		{
			let escaped_xid = escape_xid(&xid);
			let _ = sqlx::query(&format!("XA ROLLBACK '{}'", escaped_xid))
				.execute(pool)
				.await;
		}
	}
}

async fn create_test_table(pool: &MySqlPool) {
	let _ = sqlx::query("DROP TABLE IF EXISTS test_2pc")
		.execute(pool)
		.await;

	sqlx::query("CREATE TABLE test_2pc (id INT AUTO_INCREMENT PRIMARY KEY, value VARCHAR(255))")
		.execute(pool)
		.await
		.expect("Failed to create test table");
}

async fn drop_test_table(pool: &MySqlPool) {
	let _ = sqlx::query("DROP TABLE IF EXISTS test_2pc")
		.execute(pool)
		.await;
}

#[rstest]
#[tokio::test]
#[serial(mysql_2pc)]
async fn test_basic_xa_transaction_flow(#[future] mysql_pool: (MysqlContainer, Arc<MySqlPool>)) {
	let (_container, pool) = mysql_pool.await;
	let pool = pool.as_ref();
	cleanup_xa_transactions(pool).await;
	create_test_table(pool).await;

	let participant = MySqlTwoPhaseParticipant::new(pool.clone());
	let xid = "test_xa_basic_001";

	// Start XA transaction and get session
	let mut session = participant
		.begin(xid.to_string())
		.await
		.expect("Failed to begin XA");

	// Insert data using session's connection
	sqlx::query("INSERT INTO test_2pc (value) VALUES ('xa_test')")
		.execute(&mut *session.connection)
		.await
		.expect("Failed to insert");

	// End XA transaction (consumes XaSessionStarted, returns XaSessionEnded)
	let ended_session = participant.end(session).await.expect("Failed to end XA");

	// Prepare XA transaction (consumes XaSessionEnded, returns XaSessionPrepared)
	let prepared_session = participant
		.prepare(ended_session)
		.await
		.expect("Failed to prepare XA");

	// Verify transaction is in prepared state
	let prepared = participant
		.find_prepared_transaction(xid)
		.await
		.expect("Failed to query XA transactions");
	assert!(prepared.is_some());

	// Commit prepared XA transaction (consumes XaSessionPrepared)
	participant
		.commit(prepared_session)
		.await
		.expect("Failed to commit XA");

	// Verify data was committed
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_2pc")
		.fetch_one(pool)
		.await
		.expect("Failed to count rows");
	assert_eq!(count, 1);

	drop_test_table(pool).await;
}

#[rstest]
#[tokio::test]
#[serial(mysql_2pc)]
async fn test_xa_prepare_and_rollback(#[future] mysql_pool: (MysqlContainer, Arc<MySqlPool>)) {
	let (_container, pool) = mysql_pool.await;
	let pool = pool.as_ref();
	cleanup_xa_transactions(pool).await;
	create_test_table(pool).await;

	let participant = MySqlTwoPhaseParticipant::new(pool.clone());
	let xid = "test_xa_rollback_002";

	// Start, insert, end, and prepare
	let mut session = participant
		.begin(xid.to_string())
		.await
		.expect("Failed to begin");

	// Insert data using session's connection
	sqlx::query("INSERT INTO test_2pc (value) VALUES ('rollback_test')")
		.execute(&mut *session.connection)
		.await
		.expect("Failed to insert");

	let ended_session = participant.end(session).await.expect("Failed to end");
	let prepared_session = participant
		.prepare(ended_session)
		.await
		.expect("Failed to prepare");

	// Verify transaction is prepared
	let prepared = participant.find_prepared_transaction(xid).await.unwrap();
	assert!(prepared.is_some());

	// Rollback prepared transaction
	participant
		.rollback_prepared(prepared_session)
		.await
		.expect("Failed to rollback");

	// Verify data was not committed
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_2pc")
		.fetch_one(pool)
		.await
		.expect("Failed to count rows");
	assert_eq!(count, 0);

	drop_test_table(pool).await;
}

#[rstest]
#[tokio::test]
#[serial(mysql_2pc)]
async fn test_xa_one_phase_commit(#[future] mysql_pool: (MysqlContainer, Arc<MySqlPool>)) {
	let (_container, pool) = mysql_pool.await;
	let pool = pool.as_ref();
	cleanup_xa_transactions(pool).await;
	create_test_table(pool).await;

	let participant = MySqlTwoPhaseParticipant::new(pool.clone());
	let xid = "test_xa_one_phase_003";

	// Start XA transaction
	let mut session = participant
		.begin(xid.to_string())
		.await
		.expect("Failed to begin");

	// Insert data using session's connection
	sqlx::query("INSERT INTO test_2pc (value) VALUES ('one_phase_test')")
		.execute(&mut *session.connection)
		.await
		.expect("Failed to insert");

	// End XA transaction
	let ended_session = participant.end(session).await.expect("Failed to end");

	// Commit with one-phase optimization (skip prepare)
	participant
		.commit_one_phase(ended_session)
		.await
		.expect("Failed to commit one phase");

	// Verify data was committed
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_2pc")
		.fetch_one(pool)
		.await
		.expect("Failed to count rows");
	assert_eq!(count, 1);

	drop_test_table(pool).await;
}

#[rstest]
#[tokio::test]
#[serial(mysql_2pc)]
async fn test_list_xa_transactions(#[future] mysql_pool: (MysqlContainer, Arc<MySqlPool>)) {
	let (_container, pool) = mysql_pool.await;
	let pool = pool.as_ref();
	cleanup_xa_transactions(pool).await;
	create_test_table(pool).await;

	let participant1 = MySqlTwoPhaseParticipant::new(pool.clone());
	let participant2 = MySqlTwoPhaseParticipant::new(pool.clone());
	let xid1 = "test_xa_list_004_a";
	let xid2 = "test_xa_list_004_b";

	// Prepare first XA transaction
	let mut session1 = participant1
		.begin(xid1.to_string())
		.await
		.expect("Failed to begin 1");
	sqlx::query("INSERT INTO test_2pc (value) VALUES ('tx1')")
		.execute(&mut *session1.connection)
		.await
		.expect("Failed to insert 1");
	let ended1 = participant1.end(session1).await.expect("Failed to end 1");
	participant1
		.prepare(ended1)
		.await
		.expect("Failed to prepare 1");

	// Prepare second XA transaction
	let mut session2 = participant2
		.begin(xid2.to_string())
		.await
		.expect("Failed to begin 2");
	sqlx::query("INSERT INTO test_2pc (value) VALUES ('tx2')")
		.execute(&mut *session2.connection)
		.await
		.expect("Failed to insert 2");
	let ended2 = participant2.end(session2).await.expect("Failed to end 2");
	participant2
		.prepare(ended2)
		.await
		.expect("Failed to prepare 2");

	// List all prepared XA transactions
	let prepared_list = participant1
		.list_prepared_transactions()
		.await
		.expect("Failed to list XA transactions");

	assert!(prepared_list.len() >= 2);
	let xids: Vec<String> = prepared_list.iter().map(|p| p.xid.clone()).collect();
	assert!(xids.contains(&xid1.to_string()));
	assert!(xids.contains(&xid2.to_string()));

	// Cleanup (use commit by XID for recovery scenarios)
	participant1
		.commit_by_xid(xid1)
		.await
		.expect("Failed to commit 1");
	participant2
		.commit_by_xid(xid2)
		.await
		.expect("Failed to commit 2");

	drop_test_table(pool).await;
}

#[rstest]
#[tokio::test]
#[serial(mysql_2pc)]
async fn test_recovery_from_xa_prepared_state(
	#[future] mysql_pool: (MysqlContainer, Arc<MySqlPool>),
) {
	let (_container, pool) = mysql_pool.await;
	let pool = pool.as_ref();
	cleanup_xa_transactions(pool).await;
	create_test_table(pool).await;

	let xid = "test_xa_recovery_005";

	// Simulate a crash scenario: prepare but don't commit
	{
		let participant = MySqlTwoPhaseParticipant::new(pool.clone());
		let mut session = participant
			.begin(xid.to_string())
			.await
			.expect("Failed to begin");

		// Insert data using session's connection
		sqlx::query("INSERT INTO test_2pc (value) VALUES ('recovery_test')")
			.execute(&mut *session.connection)
			.await
			.expect("Failed to insert");

		let ended = participant.end(session).await.expect("Failed to end");
		participant.prepare(ended).await.expect("Failed to prepare");
		// Session and participant go out of scope (simulating crash)
	}

	// Recovery: New participant instance finds and commits the prepared transaction
	{
		let participant = MySqlTwoPhaseParticipant::new(pool.clone());
		let prepared = participant
			.find_prepared_transaction(xid)
			.await
			.expect("Failed to find prepared transaction");
		assert!(prepared.is_some());

		// Decide to commit (in real scenario, coordinator would decide)
		// Use commit_by_xid for recovery scenarios where we don't have the original session
		participant
			.commit_by_xid(xid)
			.await
			.expect("Failed to commit");
	}

	// Verify data was committed
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_2pc")
		.fetch_one(pool)
		.await
		.expect("Failed to count rows");
	assert_eq!(count, 1);

	drop_test_table(pool).await;
}

#[rstest]
#[tokio::test]
#[serial(mysql_2pc)]
async fn test_cleanup_stale_xa_transactions(
	#[future] mysql_pool: (MysqlContainer, Arc<MySqlPool>),
) {
	let (_container, pool) = mysql_pool.await;
	let pool = pool.as_ref();
	cleanup_xa_transactions(pool).await;
	create_test_table(pool).await;

	let participant = MySqlTwoPhaseParticipant::new(pool.clone());
	let xid = "stale_test_xa_006";

	// Prepare an XA transaction
	let mut session = participant
		.begin(xid.to_string())
		.await
		.expect("Failed to begin");

	// Insert data using session connection
	sqlx::query("INSERT INTO test_2pc (value) VALUES ('stale_test')")
		.execute(&mut *session.connection)
		.await
		.expect("Failed to insert");

	let ended = participant.end(session).await.expect("Failed to end");
	participant.prepare(ended).await.expect("Failed to prepare");

	// Cleanup transactions with "stale_" prefix
	let cleaned = participant
		.cleanup_stale_transactions("stale_")
		.await
		.expect("Failed to cleanup");

	assert!(cleaned >= 1);

	// Verify transaction no longer exists
	let prepared = participant.find_prepared_transaction(xid).await.unwrap();
	assert!(prepared.is_none());

	drop_test_table(pool).await;
}

#[rstest]
#[tokio::test]
#[serial(mysql_2pc)]
async fn test_concurrent_xa_transactions(#[future] mysql_pool: (MysqlContainer, Arc<MySqlPool>)) {
	let (_container, pool) = mysql_pool.await;
	let pool = pool.as_ref();
	cleanup_xa_transactions(pool).await;
	create_test_table(pool).await;

	let participant1 = MySqlTwoPhaseParticipant::new(pool.clone());
	let participant2 = MySqlTwoPhaseParticipant::new(pool.clone());

	let xid1 = "test_xa_concurrent_007_a";
	let xid2 = "test_xa_concurrent_007_b";

	// Start both XA transactions
	let mut session1 = participant1.begin(xid1.to_string()).await.unwrap();
	let mut session2 = participant2.begin(xid2.to_string()).await.unwrap();

	// Insert data in both transactions using session connections
	sqlx::query("INSERT INTO test_2pc (value) VALUES ('concurrent1')")
		.execute(&mut *session1.connection)
		.await
		.unwrap();

	sqlx::query("INSERT INTO test_2pc (value) VALUES ('concurrent2')")
		.execute(&mut *session2.connection)
		.await
		.unwrap();

	// End and prepare both
	let ended1 = participant1.end(session1).await.unwrap();
	let ended2 = participant2.end(session2).await.unwrap();
	let prepared1 = participant1.prepare(ended1).await.unwrap();
	let prepared2 = participant2.prepare(ended2).await.unwrap();

	// Commit both
	participant1.commit(prepared1).await.unwrap();
	participant2.commit(prepared2).await.unwrap();

	// Verify both transactions committed
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_2pc")
		.fetch_one(pool)
		.await
		.expect("Failed to count rows");
	assert_eq!(count, 2);

	drop_test_table(pool).await;
}

#[rstest]
#[tokio::test]
#[serial(mysql_2pc)]
async fn test_participant_clone(#[future] mysql_pool: (MysqlContainer, Arc<MySqlPool>)) {
	let (_container, pool) = mysql_pool.await;
	let pool = pool.as_ref();
	cleanup_xa_transactions(pool).await;

	let participant1 = MySqlTwoPhaseParticipant::new(pool.clone());
	let participant2 = participant1.clone();

	// Cloned participants can be used independently
	// (each creates its own session with independent connections)
	let xid1 = "test_xa_clone_008_a";

	let session1 = participant1
		.begin(xid1.to_string())
		.await
		.expect("Failed to begin 1");
	// Rollback without ending (valid XA operation)
	let _ = participant1.rollback_started(session1).await;

	// participant2 can start its own XA transaction
	let xid2 = "test_xa_clone_008_b";
	let session2 = participant2
		.begin(xid2.to_string())
		.await
		.expect("Failed to begin 2");
	// Rollback without ending (valid XA operation)
	let _ = participant2.rollback_started(session2).await;
}

#[rstest]
#[tokio::test]
#[serial(mysql_2pc)]
async fn test_xa_transaction_info_structure(
	#[future] mysql_pool: (MysqlContainer, Arc<MySqlPool>),
) {
	let (_container, pool) = mysql_pool.await;
	let pool = pool.as_ref();
	cleanup_xa_transactions(pool).await;
	create_test_table(pool).await;

	let participant = MySqlTwoPhaseParticipant::new(pool.clone());
	let xid = "test_xa_info_009";

	// Prepare an XA transaction
	let mut session = participant
		.begin(xid.to_string())
		.await
		.expect("Failed to begin");

	// Insert data using session connection
	sqlx::query("INSERT INTO test_2pc (value) VALUES ('info_test')")
		.execute(&mut *session.connection)
		.await
		.expect("Failed to insert");

	let ended = participant.end(session).await.expect("Failed to end");
	participant.prepare(ended).await.expect("Failed to prepare");

	// Get transaction info
	let info = participant
		.find_prepared_transaction(xid)
		.await
		.expect("Failed to find")
		.expect("Transaction not found");

	// Verify structure
	assert!(!info.data.is_empty());
	assert_eq!(info.xid, xid);
	assert!(info.gtrid_length > 0);

	// Cleanup - use commit_by_xid since session is consumed
	participant
		.commit_by_xid(xid)
		.await
		.expect("Failed to commit");

	drop_test_table(pool).await;
}

// Note: This test is no longer relevant with the type-safe API.
// The new API enforces correct state transitions at compile time:
// - prepare() requires XaSessionEnded (from end())
// - Attempting to call prepare() without end() results in a compile error
//
// The type system now prevents this error case, which is a significant improvement
// over runtime validation. This test has been removed because the error condition
// it tested for is now impossible to express in code.
//
// #[rstest]
// #[tokio::test]
// #[serial(mysql_2pc)]
// async fn test_error_handling_missing_end(#[future] mysql_pool: (MysqlContainer, Arc<MySqlPool>)) {
// 	let (_container, pool) = mysql_pool.await;
// 	let pool = pool.as_ref();
// 	cleanup_xa_transactions(pool).await;
// 	create_test_table(pool).await;
//
// 	let participant = MySqlTwoPhaseParticipant::new(pool.clone());
// 	let xid = "test_xa_error_010";
//
// 	// With the new API, the following code would not compile:
// 	// let mut session = participant.begin(xid.to_string()).await.expect("Failed to begin");
// 	// participant.prepare(session).await; // Compile error: prepare() requires XaSessionEnded
//
// 	drop_test_table(pool).await;
// }
