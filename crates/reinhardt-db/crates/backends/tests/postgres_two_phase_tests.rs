//! Integration tests for PostgreSQL two-phase commit
//!
//! These tests use TestContainers to provide an isolated PostgreSQL instance.
//! No manual setup required - the container is automatically created and destroyed.

use reinhardt_db::backends::postgresql::two_phase::PostgresTwoPhaseParticipant;
use rstest::*;
use serial_test::serial;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ImageExt, runners::AsyncRunner};
use testcontainers_modules::postgres::Postgres;

type PostgresContainer = testcontainers::ContainerAsync<Postgres>;

#[fixture]
async fn postgres_pool() -> (PostgresContainer, Arc<PgPool>) {
	// Start PostgreSQL container with max_prepared_transactions enabled
	let postgres = Postgres::default()
		.with_cmd(vec!["-c", "max_prepared_transactions=100"])
		.start()
		.await
		.expect("Failed to start PostgreSQL container");

	let port = postgres
		.get_host_port_ipv4(5432)
		.await
		.expect("Failed to get PostgreSQL port");

	let database_url = format!("postgresql://postgres:postgres@localhost:{}/postgres", port);

	let pool = PgPool::connect(&database_url)
		.await
		.expect("Failed to connect to PostgreSQL");

	(postgres, Arc::new(pool))
}

async fn cleanup_prepared_transactions(pool: &PgPool) {
	// Cleanup any existing prepared transactions from previous test runs
	let rows = sqlx::query("SELECT gid FROM pg_prepared_xacts")
		.fetch_all(pool)
		.await
		.unwrap_or_default();

	for row in rows {
		if let Ok(gid) = row.try_get::<String, _>("gid") {
			let _ = sqlx::query(&format!("ROLLBACK PREPARED '{}'", gid))
				.execute(pool)
				.await;
		}
	}
}

async fn create_test_table(pool: &PgPool) {
	let _ = sqlx::query("DROP TABLE IF EXISTS test_2pc")
		.execute(pool)
		.await;

	sqlx::query("CREATE TABLE test_2pc (id SERIAL PRIMARY KEY, value TEXT)")
		.execute(pool)
		.await
		.expect("Failed to create test table");
}

async fn drop_test_table(pool: &PgPool) {
	let _ = sqlx::query("DROP TABLE IF EXISTS test_2pc")
		.execute(pool)
		.await;
}

#[rstest]
#[tokio::test]
#[serial(postgres_2pc)]
async fn test_basic_two_phase_commit_flow(
	#[future] postgres_pool: (PostgresContainer, Arc<PgPool>),
) {
	let (_container, pool) = postgres_pool.await;
	let pool = pool.as_ref();
	cleanup_prepared_transactions(pool).await;
	create_test_table(pool).await;

	let participant = PostgresTwoPhaseParticipant::new(pool.clone());
	let xid = "test_basic_2pc_001";

	// Begin transaction and get session
	let mut session = participant.begin(xid).await.expect("Failed to begin");

	// Insert data using the session's dedicated connection
	sqlx::query("INSERT INTO test_2pc (value) VALUES ('test')")
		.execute(&mut *session.connection)
		.await
		.expect("Failed to insert");

	// Prepare transaction
	participant
		.prepare(&mut session)
		.await
		.expect("Failed to prepare");

	// Verify transaction is in prepared state
	let prepared = participant
		.find_prepared_transaction(xid)
		.await
		.expect("Failed to query prepared transactions");
	assert!(prepared.is_some());
	assert_eq!(prepared.unwrap().gid, xid);

	// Commit prepared transaction (consumes session)
	participant
		.commit_by_xid(xid)
		.await
		.expect("Failed to commit");

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
#[serial(postgres_2pc)]
async fn test_prepare_and_rollback(#[future] postgres_pool: (PostgresContainer, Arc<PgPool>)) {
	let (_container, pool) = postgres_pool.await;
	let pool = pool.as_ref();
	cleanup_prepared_transactions(pool).await;
	create_test_table(pool).await;

	let participant = PostgresTwoPhaseParticipant::new(pool.clone());
	let xid = "test_rollback_2pc_002";

	// Begin transaction and get session with dedicated connection
	let mut session = participant.begin(xid).await.expect("Failed to begin");

	// Insert data using session's dedicated connection
	sqlx::query("INSERT INTO test_2pc (value) VALUES ('rollback_test')")
		.execute(&mut *session.connection)
		.await
		.expect("Failed to insert");

	// Prepare transaction
	participant
		.prepare(&mut session)
		.await
		.expect("Failed to prepare");

	// Verify transaction is prepared
	let prepared = participant.find_prepared_transaction(xid).await.unwrap();
	assert!(prepared.is_some());

	// Rollback prepared transaction (using recovery API since we no longer have session after prepare)
	participant
		.rollback_by_xid(xid)
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
#[serial(postgres_2pc)]
async fn test_list_prepared_transactions(
	#[future] postgres_pool: (PostgresContainer, Arc<PgPool>),
) {
	let (_container, pool) = postgres_pool.await;
	let pool = pool.as_ref();
	cleanup_prepared_transactions(pool).await;
	create_test_table(pool).await;

	let participant = PostgresTwoPhaseParticipant::new(pool.clone());
	let xid1 = "test_list_2pc_003_a";
	let xid2 = "test_list_2pc_003_b";

	// Prepare multiple transactions
	let mut session1 = participant.begin(xid1).await.expect("Failed to begin 1");
	sqlx::query("INSERT INTO test_2pc (value) VALUES ('tx1')")
		.execute(&mut *session1.connection)
		.await
		.expect("Failed to insert 1");
	participant
		.prepare(&mut session1)
		.await
		.expect("Failed to prepare 1");

	let mut session2 = participant.begin(xid2).await.expect("Failed to begin 2");
	sqlx::query("INSERT INTO test_2pc (value) VALUES ('tx2')")
		.execute(&mut *session2.connection)
		.await
		.expect("Failed to insert 2");
	participant
		.prepare(&mut session2)
		.await
		.expect("Failed to prepare 2");

	// List all prepared transactions
	let prepared_list = participant
		.list_prepared_transactions()
		.await
		.expect("Failed to list prepared transactions");

	assert!(prepared_list.len() >= 2);
	let gids: Vec<String> = prepared_list.iter().map(|p| p.gid.clone()).collect();
	assert!(gids.contains(&xid1.to_string()));
	assert!(gids.contains(&xid2.to_string()));

	// Cleanup
	participant
		.commit_by_xid(xid1)
		.await
		.expect("Failed to commit 1");
	participant
		.commit_by_xid(xid2)
		.await
		.expect("Failed to commit 2");

	drop_test_table(pool).await;
}

#[rstest]
#[tokio::test]
#[serial(postgres_2pc)]
async fn test_recovery_from_prepared_state(
	#[future] postgres_pool: (PostgresContainer, Arc<PgPool>),
) {
	let (_container, pool) = postgres_pool.await;
	let pool = pool.as_ref();
	cleanup_prepared_transactions(pool).await;
	create_test_table(pool).await;

	let xid = "test_recovery_2pc_004";

	// Simulate a crash scenario: prepare but don't commit
	{
		let participant = PostgresTwoPhaseParticipant::new(pool.clone());
		let mut session = participant.begin(xid).await.expect("Failed to begin");
		sqlx::query("INSERT INTO test_2pc (value) VALUES ('recovery_test')")
			.execute(&mut *session.connection)
			.await
			.expect("Failed to insert");
		participant
			.prepare(&mut session)
			.await
			.expect("Failed to prepare");
		// Session and participant go out of scope (simulating crash)
	}

	// Recovery: New participant instance finds and commits the prepared transaction
	{
		let participant = PostgresTwoPhaseParticipant::new(pool.clone());
		let prepared = participant
			.find_prepared_transaction(xid)
			.await
			.expect("Failed to find prepared transaction");
		assert!(prepared.is_some());

		// Decide to commit (in real scenario, coordinator would decide)
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
#[serial(postgres_2pc)]
async fn test_cleanup_stale_transactions(
	#[future] postgres_pool: (PostgresContainer, Arc<PgPool>),
) {
	let (_container, pool) = postgres_pool.await;
	let pool = pool.as_ref();
	cleanup_prepared_transactions(pool).await;
	create_test_table(pool).await;

	let participant = PostgresTwoPhaseParticipant::new(pool.clone());
	let xid = "test_cleanup_2pc_005";

	// Prepare a transaction
	let mut session = participant.begin(xid).await.expect("Failed to begin");
	sqlx::query("INSERT INTO test_2pc (value) VALUES ('stale_test')")
		.execute(&mut *session.connection)
		.await
		.expect("Failed to insert");
	participant
		.prepare(&mut session)
		.await
		.expect("Failed to prepare");

	// Wait for transaction to become stale (threshold is 1 second)
	tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

	// Cleanup transactions older than 1 second
	let cleaned = participant
		.cleanup_stale_transactions(std::time::Duration::from_secs(1))
		.await
		.expect("Failed to cleanup");

	assert!(cleaned >= 1, "Should clean at least one stale transaction");

	// Verify transaction no longer exists
	let prepared = participant.find_prepared_transaction(xid).await.unwrap();
	assert!(prepared.is_none());

	drop_test_table(pool).await;
}

#[rstest]
#[tokio::test]
#[serial(postgres_2pc)]
async fn test_concurrent_transactions(#[future] postgres_pool: (PostgresContainer, Arc<PgPool>)) {
	let (_container, pool) = postgres_pool.await;
	let pool = pool.as_ref();
	cleanup_prepared_transactions(pool).await;
	create_test_table(pool).await;

	let participant1 = PostgresTwoPhaseParticipant::new(pool.clone());
	let participant2 = PostgresTwoPhaseParticipant::new(pool.clone());

	let xid1 = "test_concurrent_2pc_006_a";
	let xid2 = "test_concurrent_2pc_006_b";

	// Run two transactions concurrently
	let handle1 = tokio::spawn(async move {
		let mut session = participant1.begin(xid1).await.unwrap();
		sqlx::query("INSERT INTO test_2pc (value) VALUES ('concurrent1')")
			.execute(&mut *session.connection)
			.await
			.unwrap();
		participant1.prepare(&mut session).await.unwrap();
		participant1.commit_by_xid(xid1).await.unwrap();
	});

	let handle2 = tokio::spawn(async move {
		let mut session = participant2.begin(xid2).await.unwrap();
		sqlx::query("INSERT INTO test_2pc (value) VALUES ('concurrent2')")
			.execute(&mut *session.connection)
			.await
			.unwrap();
		participant2.prepare(&mut session).await.unwrap();
		participant2.commit_by_xid(xid2).await.unwrap();
	});

	handle1.await.expect("Task 1 failed");
	handle2.await.expect("Task 2 failed");

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
#[serial(postgres_2pc)]
async fn test_participant_clone(#[future] postgres_pool: (PostgresContainer, Arc<PgPool>)) {
	let (_container, pool) = postgres_pool.await;
	let pool = pool.as_ref();
	cleanup_prepared_transactions(pool).await;

	let participant1 = PostgresTwoPhaseParticipant::new(pool.clone());
	let participant2 = participant1.clone();

	// Both participants should work independently
	let xid1 = "test_clone_2pc_007_a";
	let xid2 = "test_clone_2pc_007_b";

	participant1.begin(xid1).await.expect("Failed to begin 1");
	participant2.begin(xid2).await.expect("Failed to begin 2");

	// Both should be able to query prepared transactions
	let _ = participant1.list_prepared_transactions().await;
	let _ = participant2.list_prepared_transactions().await;
}

#[rstest]
#[tokio::test]
#[serial(postgres_2pc)]
async fn test_error_handling_duplicate_prepare(
	#[future] postgres_pool: (PostgresContainer, Arc<PgPool>),
) {
	let (_container, pool) = postgres_pool.await;
	let pool = pool.as_ref();
	cleanup_prepared_transactions(pool).await;
	create_test_table(pool).await;

	let participant = PostgresTwoPhaseParticipant::new(pool.clone());
	let xid = "test_error_2pc_008";

	let mut session = participant.begin(xid).await.expect("Failed to begin");
	sqlx::query("INSERT INTO test_2pc (value) VALUES ('error_test')")
		.execute(&mut *session.connection)
		.await
		.expect("Failed to insert");
	participant
		.prepare(&mut session)
		.await
		.expect("Failed to prepare");

	// Try to prepare again with the same xid (should fail)
	let mut session2 = participant.begin(xid).await.expect("Failed to begin again");
	// Insert another row to make this a non-empty transaction
	// (empty transactions don't trigger duplicate XID error in PostgreSQL)
	sqlx::query("INSERT INTO test_2pc (value) VALUES ('second_attempt')")
		.execute(&mut *session2.connection)
		.await
		.expect("Failed to insert second row");
	let result = participant.prepare(&mut session2).await;
	assert!(result.is_err(), "Second PREPARE with same XID should fail");

	// Cleanup the first prepared transaction
	participant
		.rollback_by_xid(xid)
		.await
		.expect("Failed to rollback");

	drop_test_table(pool).await;
}
