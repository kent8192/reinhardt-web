//! Integration tests
//!
//! Integration tests for Two-Phase Commit

#[cfg(test)]
mod integration_tests {
	use crate::orm::two_phase_commit::core::{
		MockParticipant, TransactionState, TwoPhaseCoordinator,
	};
	use crate::orm::two_phase_commit::transaction_log::{InMemoryTransactionLog, TransactionLog};
	use rstest::rstest;
	use std::sync::Arc;

	#[rstest]
	#[tokio::test]
	async fn test_coordinator_with_logging() {
		let log = Arc::new(InMemoryTransactionLog::new());
		let mut coordinator = TwoPhaseCoordinator::with_log("txn_log_001", log.clone());

		coordinator
			.add_participant(Box::new(MockParticipant::new("db1")))
			.await
			.unwrap();
		coordinator
			.add_participant(Box::new(MockParticipant::new("db2")))
			.await
			.unwrap();

		coordinator.begin().await.unwrap();

		// Verify that Active state is recorded in the log
		let entry = log.read("txn_log_001").unwrap().unwrap();
		assert_eq!(entry.state, TransactionState::Active);
		assert_eq!(entry.participants.len(), 2);

		coordinator.prepare().await.unwrap();

		// Verify that Prepared state is recorded in the log
		let entry = log.read("txn_log_001").unwrap().unwrap();
		assert_eq!(entry.state, TransactionState::Prepared);

		coordinator.commit().await.unwrap();

		// Verify that the log is deleted after commit
		assert!(log.read("txn_log_001").unwrap().is_none());
	}

	#[rstest]
	#[tokio::test]
	async fn test_coordinator_rollback_with_logging() {
		let log = Arc::new(InMemoryTransactionLog::new());
		let mut coordinator = TwoPhaseCoordinator::with_log("txn_log_002", log.clone());

		coordinator
			.add_participant(Box::new(MockParticipant::new("db1")))
			.await
			.unwrap();

		coordinator.begin().await.unwrap();
		coordinator.prepare().await.unwrap();
		coordinator.rollback().await.unwrap();

		// Verify that the log is deleted after rollback
		assert!(log.read("txn_log_002").unwrap().is_none());
	}

	#[rstest]
	#[tokio::test]
	async fn test_recovery_from_log() {
		let log = Arc::new(InMemoryTransactionLog::new());

		// Transaction 1: Successfully committed
		let mut coordinator1 = TwoPhaseCoordinator::with_log("txn_rec_001", log.clone());
		coordinator1
			.add_participant(Box::new(MockParticipant::new("db1")))
			.await
			.unwrap();
		coordinator1.begin().await.unwrap();
		coordinator1.prepare().await.unwrap();
		coordinator1.commit().await.unwrap();

		// Transaction 2: Stopped in Prepared state (simulate a failure)
		let mut coordinator2 = TwoPhaseCoordinator::with_log("txn_rec_002", log.clone());
		coordinator2
			.add_participant(Box::new(MockParticipant::new("db2")))
			.await
			.unwrap();
		coordinator2.begin().await.unwrap();
		coordinator2.prepare().await.unwrap();
		// Assume a failure occurred here (do not commit)

		// Recovery: Search for transactions in Prepared state
		let prepared_txns = log.find_by_state(TransactionState::Prepared).unwrap();
		assert_eq!(prepared_txns.len(), 1);
		assert_eq!(prepared_txns[0].transaction_id, "txn_rec_002");

		// Rollback as recovery procedure
		coordinator2.rollback().await.unwrap();

		// Verify that the log is deleted after rollback
		let prepared_txns = log.find_by_state(TransactionState::Prepared).unwrap();
		assert_eq!(prepared_txns.len(), 0);
	}

	#[rstest]
	#[tokio::test]
	async fn test_coordinator_prepare_failure() {
		let mut coordinator = TwoPhaseCoordinator::new("txn_fail_001");

		coordinator
			.add_participant(Box::new(MockParticipant::new("db1")))
			.await
			.unwrap();
		coordinator
			.add_participant(Box::new(MockParticipant::new("db2").with_prepare_failure()))
			.await
			.unwrap();

		coordinator.begin().await.unwrap();

		let result = coordinator.prepare().await;
		assert!(result.is_err());

		// On prepare failure, automatically rolled back
		assert_eq!(
			coordinator.state().await.unwrap(),
			TransactionState::Aborted
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_multiple_coordinators_isolation() {
		let log = Arc::new(InMemoryTransactionLog::new());

		let mut coord1 = TwoPhaseCoordinator::with_log("txn_iso_001", log.clone());
		let mut coord2 = TwoPhaseCoordinator::with_log("txn_iso_002", log.clone());

		coord1
			.add_participant(Box::new(MockParticipant::new("db1")))
			.await
			.unwrap();
		coord2
			.add_participant(Box::new(MockParticipant::new("db2")))
			.await
			.unwrap();

		coord1.begin().await.unwrap();
		coord2.begin().await.unwrap();

		coord1.prepare().await.unwrap();
		coord2.prepare().await.unwrap();

		// Both transactions are recorded in the log
		let all_entries = log.read_all().unwrap();
		assert_eq!(all_entries.len(), 2);

		coord1.commit().await.unwrap();
		coord2.commit().await.unwrap();

		// Both are deleted from the log after commit
		let all_entries = log.read_all().unwrap();
		assert_eq!(all_entries.len(), 0);
	}

	#[rstest]
	#[tokio::test]
	async fn test_log_entry_metadata() {
		let log = Arc::new(InMemoryTransactionLog::new());
		let mut coordinator = TwoPhaseCoordinator::with_log("txn_meta_001", log.clone());

		coordinator
			.add_participant(Box::new(MockParticipant::new("db1")))
			.await
			.unwrap();
		coordinator.begin().await.unwrap();

		// Retrieve log entry and verify metadata
		let entry = log.read("txn_meta_001").unwrap().unwrap();
		assert_eq!(entry.transaction_id, "txn_meta_001");
		assert_eq!(entry.state, TransactionState::Active);
		assert_eq!(entry.participants, vec!["db1"]);
		assert!(!entry.timestamp.timestamp().is_negative());
	}
}
