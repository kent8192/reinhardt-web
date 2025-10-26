//! Integration tests for PostgreSQL two-phase commit
//!
//! These tests require:
//! - A running PostgreSQL server
//! - max_prepared_transactions > 0 in postgresql.conf
//!
//! Set the DATABASE_URL environment variable to run these tests:
//! ```bash
//! export DATABASE_URL="postgresql://localhost/testdb"
//! cargo test --test postgres_two_phase_tests -- --test-threads=1
//! ```

use reinhardt_db_backends::backends::postgresql::two_phase::PostgresTwoPhaseParticipant;
use serial_test::serial;
use sqlx::{PgPool, Row};

async fn setup_pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/postgres".to_string());
    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to PostgreSQL")
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

#[tokio::test]
#[serial(postgres_2pc)]
async fn test_basic_two_phase_commit_flow() {
    let pool = setup_pool().await;
    cleanup_prepared_transactions(&pool).await;
    create_test_table(&pool).await;

    let participant = PostgresTwoPhaseParticipant::new(pool.clone());
    let xid = "test_basic_2pc_001";

    // Begin transaction
    participant.begin(xid).await.expect("Failed to begin");

    // Insert data
    sqlx::query("INSERT INTO test_2pc (value) VALUES ('test')")
        .execute(&pool)
        .await
        .expect("Failed to insert");

    // Prepare transaction
    participant.prepare(xid).await.expect("Failed to prepare");

    // Verify transaction is in prepared state
    let prepared = participant
        .find_prepared_transaction(xid)
        .await
        .expect("Failed to query prepared transactions");
    assert!(prepared.is_some());
    assert_eq!(prepared.unwrap().gid, xid);

    // Commit prepared transaction
    participant.commit(xid).await.expect("Failed to commit");

    // Verify data was committed
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_2pc")
        .fetch_one(&pool)
        .await
        .expect("Failed to count rows");
    assert_eq!(count, 1);

    drop_test_table(&pool).await;
}

#[tokio::test]
#[serial(postgres_2pc)]
async fn test_prepare_and_rollback() {
    let pool = setup_pool().await;
    cleanup_prepared_transactions(&pool).await;
    create_test_table(&pool).await;

    let participant = PostgresTwoPhaseParticipant::new(pool.clone());
    let xid = "test_rollback_2pc_002";

    // Begin and prepare transaction
    participant.begin(xid).await.expect("Failed to begin");
    sqlx::query("INSERT INTO test_2pc (value) VALUES ('rollback_test')")
        .execute(&pool)
        .await
        .expect("Failed to insert");
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
#[serial(postgres_2pc)]
async fn test_list_prepared_transactions() {
    let pool = setup_pool().await;
    cleanup_prepared_transactions(&pool).await;
    create_test_table(&pool).await;

    let participant = PostgresTwoPhaseParticipant::new(pool.clone());
    let xid1 = "test_list_2pc_003_a";
    let xid2 = "test_list_2pc_003_b";

    // Prepare multiple transactions
    participant.begin(xid1).await.expect("Failed to begin 1");
    sqlx::query("INSERT INTO test_2pc (value) VALUES ('tx1')")
        .execute(&pool)
        .await
        .expect("Failed to insert 1");
    participant.prepare(xid1).await.expect("Failed to prepare 1");

    participant.begin(xid2).await.expect("Failed to begin 2");
    sqlx::query("INSERT INTO test_2pc (value) VALUES ('tx2')")
        .execute(&pool)
        .await
        .expect("Failed to insert 2");
    participant.prepare(xid2).await.expect("Failed to prepare 2");

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
    participant.commit(xid1).await.expect("Failed to commit 1");
    participant.commit(xid2).await.expect("Failed to commit 2");

    drop_test_table(&pool).await;
}

#[tokio::test]
#[serial(postgres_2pc)]
async fn test_recovery_from_prepared_state() {
    let pool = setup_pool().await;
    cleanup_prepared_transactions(&pool).await;
    create_test_table(&pool).await;

    let xid = "test_recovery_2pc_004";

    // Simulate a crash scenario: prepare but don't commit
    {
        let participant = PostgresTwoPhaseParticipant::new(pool.clone());
        participant.begin(xid).await.expect("Failed to begin");
        sqlx::query("INSERT INTO test_2pc (value) VALUES ('recovery_test')")
            .execute(&pool)
            .await
            .expect("Failed to insert");
        participant.prepare(xid).await.expect("Failed to prepare");
        // Participant goes out of scope (simulating crash)
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
#[serial(postgres_2pc)]
async fn test_cleanup_stale_transactions() {
    let pool = setup_pool().await;
    cleanup_prepared_transactions(&pool).await;
    create_test_table(&pool).await;

    let participant = PostgresTwoPhaseParticipant::new(pool.clone());
    let xid = "test_cleanup_2pc_005";

    // Prepare a transaction
    participant.begin(xid).await.expect("Failed to begin");
    sqlx::query("INSERT INTO test_2pc (value) VALUES ('stale_test')")
        .execute(&pool)
        .await
        .expect("Failed to insert");
    participant.prepare(xid).await.expect("Failed to prepare");

    // Wait a moment
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Cleanup transactions older than 1 second
    let cleaned = participant
        .cleanup_stale_transactions(std::time::Duration::from_secs(1))
        .await
        .expect("Failed to cleanup");

    assert!(cleaned >= 1);

    // Verify transaction no longer exists
    let prepared = participant.find_prepared_transaction(xid).await.unwrap();
    assert!(prepared.is_none());

    drop_test_table(&pool).await;
}

#[tokio::test]
#[serial(postgres_2pc)]
async fn test_concurrent_transactions() {
    let pool = setup_pool().await;
    cleanup_prepared_transactions(&pool).await;
    create_test_table(&pool).await;

    let participant1 = PostgresTwoPhaseParticipant::new(pool.clone());
    let participant2 = PostgresTwoPhaseParticipant::new(pool.clone());

    let xid1 = "test_concurrent_2pc_006_a";
    let xid2 = "test_concurrent_2pc_006_b";

    // Run two transactions concurrently
    let handle1 = tokio::spawn(async move {
        participant1.begin(xid1).await.unwrap();
        sqlx::query("INSERT INTO test_2pc (value) VALUES ('concurrent1')")
            .execute(participant1.pool.as_ref())
            .await
            .unwrap();
        participant1.prepare(xid1).await.unwrap();
        participant1.commit(xid1).await.unwrap();
    });

    let handle2 = tokio::spawn(async move {
        participant2.begin(xid2).await.unwrap();
        sqlx::query("INSERT INTO test_2pc (value) VALUES ('concurrent2')")
            .execute(participant2.pool.as_ref())
            .await
            .unwrap();
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
#[serial(postgres_2pc)]
async fn test_participant_clone() {
    let pool = setup_pool().await;
    cleanup_prepared_transactions(&pool).await;

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

#[tokio::test]
#[serial(postgres_2pc)]
async fn test_error_handling_duplicate_prepare() {
    let pool = setup_pool().await;
    cleanup_prepared_transactions(&pool).await;
    create_test_table(&pool).await;

    let participant = PostgresTwoPhaseParticipant::new(pool.clone());
    let xid = "test_error_2pc_008";

    participant.begin(xid).await.expect("Failed to begin");
    sqlx::query("INSERT INTO test_2pc (value) VALUES ('error_test')")
        .execute(&pool)
        .await
        .expect("Failed to insert");
    participant.prepare(xid).await.expect("Failed to prepare");

    // Try to prepare again with the same xid (should fail)
    participant.begin(xid).await.expect("Failed to begin again");
    let result = participant.prepare(xid).await;
    assert!(result.is_err());

    // Cleanup
    participant
        .rollback(xid)
        .await
        .expect("Failed to rollback");

    drop_test_table(&pool).await;
}
