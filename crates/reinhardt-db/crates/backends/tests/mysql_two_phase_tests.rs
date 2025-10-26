//! Integration tests for MySQL two-phase commit (XA transactions)
//!
//! These tests require a running MySQL server.
//!
//! Set the DATABASE_URL environment variable to run these tests:
//! ```bash
//! export DATABASE_URL="mysql://root@localhost/testdb"
//! cargo test --test mysql_two_phase_tests -- --test-threads=1
//! ```

use reinhardt_db_backends::backends::mysql::two_phase::MySqlTwoPhaseParticipant;
use serial_test::serial;
use sqlx::{MySqlPool, Row};

async fn setup_pool() -> MySqlPool {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "mysql://root@localhost/mysql".to_string());
    MySqlPool::connect(&database_url)
        .await
        .expect("Failed to connect to MySQL")
}

async fn cleanup_xa_transactions(pool: &MySqlPool) {
    // Cleanup any existing XA transactions from previous test runs
    let rows = sqlx::query("XA RECOVER")
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    for row in rows {
        if let Ok(data) = row.try_get::<Vec<u8>, _>("data") {
            if let Ok(xid) = String::from_utf8(data) {
                let _ = sqlx::query(&format!("XA ROLLBACK '{}'", xid))
                    .execute(pool)
                    .await;
            }
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

#[tokio::test]
#[serial(mysql_2pc)]
async fn test_basic_xa_transaction_flow() {
    let pool = setup_pool().await;
    cleanup_xa_transactions(&pool).await;
    create_test_table(&pool).await;

    let participant = MySqlTwoPhaseParticipant::new(pool.clone());
    let xid = "test_xa_basic_001";

    // Start XA transaction
    participant.begin(xid).await.expect("Failed to begin XA");

    // Insert data
    sqlx::query("INSERT INTO test_2pc (value) VALUES ('xa_test')")
        .execute(&pool)
        .await
        .expect("Failed to insert");

    // End XA transaction
    participant.end(xid).await.expect("Failed to end XA");

    // Prepare XA transaction
    participant
        .prepare(xid)
        .await
        .expect("Failed to prepare XA");

    // Verify transaction is in prepared state
    let prepared = participant
        .find_prepared_transaction(xid)
        .await
        .expect("Failed to query XA transactions");
    assert!(prepared.is_some());

    // Commit prepared XA transaction
    participant.commit(xid).await.expect("Failed to commit XA");

    // Verify data was committed
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_2pc")
        .fetch_one(&pool)
        .await
        .expect("Failed to count rows");
    assert_eq!(count, 1);

    drop_test_table(&pool).await;
}

#[tokio::test]
#[serial(mysql_2pc)]
async fn test_xa_prepare_and_rollback() {
    let pool = setup_pool().await;
    cleanup_xa_transactions(&pool).await;
    create_test_table(&pool).await;

    let participant = MySqlTwoPhaseParticipant::new(pool.clone());
    let xid = "test_xa_rollback_002";

    // Start, insert, end, and prepare
    participant.begin(xid).await.expect("Failed to begin");
    sqlx::query("INSERT INTO test_2pc (value) VALUES ('rollback_test')")
        .execute(&pool)
        .await
        .expect("Failed to insert");
    participant.end(xid).await.expect("Failed to end");
    participant.prepare(xid).await.expect("Failed to prepare");

    // Verify transaction is prepared
    let prepared = participant.find_prepared_transaction(xid).await.unwrap();
    assert!(prepared.is_some());

    // Rollback prepared transaction
    participant
        .rollback(xid)
        .await
        .expect("Failed to rollback");

    // Verify data was not committed
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_2pc")
        .fetch_one(&pool)
        .await
        .expect("Failed to count rows");
    assert_eq!(count, 0);

    drop_test_table(&pool).await;
}

#[tokio::test]
#[serial(mysql_2pc)]
async fn test_xa_one_phase_commit() {
    let pool = setup_pool().await;
    cleanup_xa_transactions(&pool).await;
    create_test_table(&pool).await;

    let participant = MySqlTwoPhaseParticipant::new(pool.clone());
    let xid = "test_xa_one_phase_003";

    // Start XA transaction
    participant.begin(xid).await.expect("Failed to begin");

    // Insert data
    sqlx::query("INSERT INTO test_2pc (value) VALUES ('one_phase_test')")
        .execute(&pool)
        .await
        .expect("Failed to insert");

    // End XA transaction
    participant.end(xid).await.expect("Failed to end");

    // Commit with one-phase optimization (skip prepare)
    participant
        .commit_one_phase(xid)
        .await
        .expect("Failed to commit one phase");

    // Verify data was committed
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_2pc")
        .fetch_one(&pool)
        .await
        .expect("Failed to count rows");
    assert_eq!(count, 1);

    drop_test_table(&pool).await;
}

#[tokio::test]
#[serial(mysql_2pc)]
async fn test_list_xa_transactions() {
    let pool = setup_pool().await;
    cleanup_xa_transactions(&pool).await;
    create_test_table(&pool).await;

    let participant = MySqlTwoPhaseParticipant::new(pool.clone());
    let xid1 = "test_xa_list_004_a";
    let xid2 = "test_xa_list_004_b";

    // Prepare multiple XA transactions
    participant.begin(xid1).await.expect("Failed to begin 1");
    sqlx::query("INSERT INTO test_2pc (value) VALUES ('tx1')")
        .execute(&pool)
        .await
        .expect("Failed to insert 1");
    participant.end(xid1).await.expect("Failed to end 1");
    participant.prepare(xid1).await.expect("Failed to prepare 1");

    participant.begin(xid2).await.expect("Failed to begin 2");
    sqlx::query("INSERT INTO test_2pc (value) VALUES ('tx2')")
        .execute(&pool)
        .await
        .expect("Failed to insert 2");
    participant.end(xid2).await.expect("Failed to end 2");
    participant.prepare(xid2).await.expect("Failed to prepare 2");

    // List all prepared XA transactions
    let prepared_list = participant
        .list_prepared_transactions()
        .await
        .expect("Failed to list XA transactions");

    assert!(prepared_list.len() >= 2);
    let xids: Vec<String> = prepared_list.iter().map(|p| p.xid.clone()).collect();
    assert!(xids.contains(&xid1.to_string()));
    assert!(xids.contains(&xid2.to_string()));

    // Cleanup
    participant.commit(xid1).await.expect("Failed to commit 1");
    participant.commit(xid2).await.expect("Failed to commit 2");

    drop_test_table(&pool).await;
}

#[tokio::test]
#[serial(mysql_2pc)]
async fn test_recovery_from_xa_prepared_state() {
    let pool = setup_pool().await;
    cleanup_xa_transactions(&pool).await;
    create_test_table(&pool).await;

    let xid = "test_xa_recovery_005";

    // Simulate a crash scenario: prepare but don't commit
    {
        let participant = MySqlTwoPhaseParticipant::new(pool.clone());
        participant.begin(xid).await.expect("Failed to begin");
        sqlx::query("INSERT INTO test_2pc (value) VALUES ('recovery_test')")
            .execute(&pool)
            .await
            .expect("Failed to insert");
        participant.end(xid).await.expect("Failed to end");
        participant.prepare(xid).await.expect("Failed to prepare");
        // Participant goes out of scope (simulating crash)
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
        participant.commit(xid).await.expect("Failed to commit");
    }

    // Verify data was committed
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_2pc")
        .fetch_one(&pool)
        .await
        .expect("Failed to count rows");
    assert_eq!(count, 1);

    drop_test_table(&pool).await;
}

#[tokio::test]
#[serial(mysql_2pc)]
async fn test_cleanup_stale_xa_transactions() {
    let pool = setup_pool().await;
    cleanup_xa_transactions(&pool).await;
    create_test_table(&pool).await;

    let participant = MySqlTwoPhaseParticipant::new(pool.clone());
    let xid = "stale_test_xa_006";

    // Prepare an XA transaction
    participant.begin(xid).await.expect("Failed to begin");
    sqlx::query("INSERT INTO test_2pc (value) VALUES ('stale_test')")
        .execute(&pool)
        .await
        .expect("Failed to insert");
    participant.end(xid).await.expect("Failed to end");
    participant.prepare(xid).await.expect("Failed to prepare");

    // Cleanup transactions with "stale_" prefix
    let cleaned = participant
        .cleanup_stale_transactions("stale_")
        .await
        .expect("Failed to cleanup");

    assert!(cleaned >= 1);

    // Verify transaction no longer exists
    let prepared = participant.find_prepared_transaction(xid).await.unwrap();
    assert!(prepared.is_none());

    drop_test_table(&pool).await;
}

#[tokio::test]
#[serial(mysql_2pc)]
async fn test_concurrent_xa_transactions() {
    let pool = setup_pool().await;
    cleanup_xa_transactions(&pool).await;
    create_test_table(&pool).await;

    let participant1 = MySqlTwoPhaseParticipant::new(pool.clone());
    let participant2 = MySqlTwoPhaseParticipant::new(pool.clone());

    let xid1 = "test_xa_concurrent_007_a";
    let xid2 = "test_xa_concurrent_007_b";

    // Run two XA transactions concurrently
    let handle1 = tokio::spawn(async move {
        participant1.begin(xid1).await.unwrap();
        sqlx::query("INSERT INTO test_2pc (value) VALUES ('concurrent1')")
            .execute(participant1.pool.as_ref())
            .await
            .unwrap();
        participant1.end(xid1).await.unwrap();
        participant1.prepare(xid1).await.unwrap();
        participant1.commit(xid1).await.unwrap();
    });

    let handle2 = tokio::spawn(async move {
        participant2.begin(xid2).await.unwrap();
        sqlx::query("INSERT INTO test_2pc (value) VALUES ('concurrent2')")
            .execute(participant2.pool.as_ref())
            .await
            .unwrap();
        participant2.end(xid2).await.unwrap();
        participant2.prepare(xid2).await.unwrap();
        participant2.commit(xid2).await.unwrap();
    });

    handle1.await.expect("Task 1 failed");
    handle2.await.expect("Task 2 failed");

    // Verify both transactions committed
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_2pc")
        .fetch_one(&pool)
        .await
        .expect("Failed to count rows");
    assert_eq!(count, 2);

    drop_test_table(&pool).await;
}

#[tokio::test]
#[serial(mysql_2pc)]
async fn test_participant_clone() {
    let pool = setup_pool().await;
    cleanup_xa_transactions(&pool).await;

    let participant1 = MySqlTwoPhaseParticipant::new(pool.clone());
    let participant2 = participant1.clone();

    // Both participants should work independently
    let xid1 = "test_xa_clone_008_a";
    let xid2 = "test_xa_clone_008_b";

    participant1.begin(xid1).await.expect("Failed to begin 1");
    participant2.begin(xid2).await.expect("Failed to begin 2");

    // Both should be able to query XA transactions
    let _ = participant1.list_prepared_transactions().await;
    let _ = participant2.list_prepared_transactions().await;

    // Cleanup
    participant1.end(xid1).await.expect("Failed to end 1");
    participant2.end(xid2).await.expect("Failed to end 2");
    let _ = participant1.rollback(xid1).await;
    let _ = participant2.rollback(xid2).await;
}

#[tokio::test]
#[serial(mysql_2pc)]
async fn test_xa_transaction_info_structure() {
    let pool = setup_pool().await;
    cleanup_xa_transactions(&pool).await;
    create_test_table(&pool).await;

    let participant = MySqlTwoPhaseParticipant::new(pool.clone());
    let xid = "test_xa_info_009";

    // Prepare an XA transaction
    participant.begin(xid).await.expect("Failed to begin");
    sqlx::query("INSERT INTO test_2pc (value) VALUES ('info_test')")
        .execute(&pool)
        .await
        .expect("Failed to insert");
    participant.end(xid).await.expect("Failed to end");
    participant.prepare(xid).await.expect("Failed to prepare");

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

    // Cleanup
    participant.commit(xid).await.expect("Failed to commit");

    drop_test_table(&pool).await;
}

#[tokio::test]
#[serial(mysql_2pc)]
async fn test_error_handling_missing_end() {
    let pool = setup_pool().await;
    cleanup_xa_transactions(&pool).await;
    create_test_table(&pool).await;

    let participant = MySqlTwoPhaseParticipant::new(pool.clone());
    let xid = "test_xa_error_010";

    // Start XA transaction but don't end it before prepare
    participant.begin(xid).await.expect("Failed to begin");
    sqlx::query("INSERT INTO test_2pc (value) VALUES ('error_test')")
        .execute(&pool)
        .await
        .expect("Failed to insert");

    // Try to prepare without ending (should fail)
    let result = participant.prepare(xid).await;
    assert!(result.is_err());

    // Cleanup
    let _ = participant.end(xid).await;
    let _ = participant.rollback(xid).await;

    drop_test_table(&pool).await;
}
