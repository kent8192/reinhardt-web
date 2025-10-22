//! Two-Phase Commit (2PC) implementation for distributed transactions
//!
//! This module provides support for distributed transactions across multiple databases
//! using the two-phase commit protocol. The protocol consists of a prepare phase
//! where all participants vote on the transaction, followed by a commit or rollback phase.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Errors that can occur during two-phase commit operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TwoPhaseError {
    /// A participant failed during the prepare phase
    PrepareFailed(String, String),
    /// A participant failed during the commit phase
    CommitFailed(String, String),
    /// A participant failed during the rollback phase
    RollbackFailed(String, String),
    /// Invalid transaction state for the requested operation
    InvalidState(String),
    /// No participants registered for the transaction
    NoParticipants,
    /// Transaction ID already exists
    DuplicateTransactionId(String),
    /// Participant already exists
    DuplicateParticipant(String),
}

impl std::fmt::Display for TwoPhaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TwoPhaseError::PrepareFailed(participant, reason) => {
                write!(f, "Prepare failed for '{}': {}", participant, reason)
            }
            TwoPhaseError::CommitFailed(participant, reason) => {
                write!(f, "Commit failed for '{}': {}", participant, reason)
            }
            TwoPhaseError::RollbackFailed(participant, reason) => {
                write!(f, "Rollback failed for '{}': {}", participant, reason)
            }
            TwoPhaseError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            TwoPhaseError::NoParticipants => write!(f, "No participants registered"),
            TwoPhaseError::DuplicateTransactionId(id) => {
                write!(f, "Transaction ID '{}' already exists", id)
            }
            TwoPhaseError::DuplicateParticipant(participant) => {
                write!(f, "Participant '{}' already registered", participant)
            }
        }
    }
}

impl std::error::Error for TwoPhaseError {}

/// Transaction state in the two-phase commit protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionState {
    /// Transaction has not started
    NotStarted,
    /// Transaction is in progress
    Active,
    /// All participants have been prepared
    Prepared,
    /// Transaction has been committed
    Committed,
    /// Transaction has been aborted/rolled back
    Aborted,
}

/// Status of an individual participant
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParticipantStatus {
    /// Participant is active in transaction
    Active,
    /// Participant has successfully prepared
    Prepared,
    /// Participant has committed
    Committed,
    /// Participant has aborted
    Aborted,
}

/// A participant in the distributed transaction
#[derive(Debug, Clone)]
pub struct Participant {
    pub db_alias: String,
    pub status: ParticipantStatus,
}

impl Participant {
    /// Create a new participant
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::two_phase_commit::Participant;
    ///
    /// let participant = Participant::new("primary_db");
    /// assert_eq!(participant.db_alias, "primary_db");
    /// ```
    pub fn new(db_alias: impl Into<String>) -> Self {
        Self {
            db_alias: db_alias.into(),
            status: ParticipantStatus::Active,
        }
    }

    /// Check if participant is prepared
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::two_phase_commit::Participant;
    ///
    /// let mut participant = Participant::new("db1");
    /// assert!(!participant.is_prepared());
    ///
    /// participant.status = reinhardt_orm::two_phase_commit::ParticipantStatus::Prepared;
    /// assert!(participant.is_prepared());
    /// ```
    pub fn is_prepared(&self) -> bool {
        matches!(self.status, ParticipantStatus::Prepared)
    }
}

/// Two-Phase Commit coordinator for distributed transactions
///
/// Manages the two-phase commit protocol across multiple database participants.
/// The protocol ensures that either all participants commit or all abort.
#[derive(Debug)]
pub struct TwoPhaseCommit {
    transaction_id: String,
    state: Arc<Mutex<TransactionState>>,
    participants: Arc<Mutex<HashMap<String, Participant>>>,
}

impl TwoPhaseCommit {
    /// Create a new two-phase commit transaction
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::two_phase_commit::TwoPhaseCommit;
    ///
    /// let tpc = TwoPhaseCommit::new("txn_001");
    /// assert_eq!(tpc.transaction_id(), "txn_001");
    /// assert_eq!(tpc.state().unwrap(), reinhardt_orm::two_phase_commit::TransactionState::NotStarted);
    /// ```
    pub fn new(transaction_id: impl Into<String>) -> Self {
        Self {
            transaction_id: transaction_id.into(),
            state: Arc::new(Mutex::new(TransactionState::NotStarted)),
            participants: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get the transaction ID
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::two_phase_commit::TwoPhaseCommit;
    ///
    /// let tpc = TwoPhaseCommit::new("my_transaction");
    /// assert_eq!(tpc.transaction_id(), "my_transaction");
    /// ```
    pub fn transaction_id(&self) -> &str {
        &self.transaction_id
    }

    /// Get the current transaction state
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::two_phase_commit::{TwoPhaseCommit, TransactionState};
    ///
    /// let mut tpc = TwoPhaseCommit::new("txn_002");
    /// assert_eq!(tpc.state().unwrap(), TransactionState::NotStarted);
    ///
    /// tpc.begin().unwrap();
    /// assert_eq!(tpc.state().unwrap(), TransactionState::Active);
    /// ```
    pub fn state(&self) -> Result<TransactionState, TwoPhaseError> {
        self.state
            .lock()
            .map(|s| *s)
            .map_err(|_| TwoPhaseError::InvalidState("Failed to acquire state lock".to_string()))
    }

    /// Begin the distributed transaction
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::two_phase_commit::{TwoPhaseCommit, TransactionState};
    ///
    /// let mut tpc = TwoPhaseCommit::new("txn_003");
    /// tpc.begin().unwrap();
    /// assert_eq!(tpc.state().unwrap(), TransactionState::Active);
    /// ```
    pub fn begin(&mut self) -> Result<(), TwoPhaseError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| TwoPhaseError::InvalidState("Failed to acquire state lock".to_string()))?;

        if *state != TransactionState::NotStarted {
            return Err(TwoPhaseError::InvalidState(
                "Transaction already started".to_string(),
            ));
        }

        *state = TransactionState::Active;
        Ok(())
    }

    /// Add a participant to the distributed transaction
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::two_phase_commit::TwoPhaseCommit;
    ///
    /// let mut tpc = TwoPhaseCommit::new("txn_004");
    /// tpc.begin().unwrap();
    /// tpc.add_participant("database_1").unwrap();
    /// tpc.add_participant("database_2").unwrap();
    /// assert_eq!(tpc.participant_count(), 2);
    /// ```
    pub fn add_participant(&mut self, db_alias: impl Into<String>) -> Result<(), TwoPhaseError> {
        let state = self.state().unwrap();
        if state != TransactionState::Active {
            return Err(TwoPhaseError::InvalidState(
                "Can only add participants to active transaction".to_string(),
            ));
        }

        let db_alias = db_alias.into();
        let mut participants = self.participants.lock().map_err(|_| {
            TwoPhaseError::InvalidState("Failed to acquire participants lock".to_string())
        })?;

        if participants.contains_key(&db_alias) {
            return Err(TwoPhaseError::DuplicateParticipant(db_alias));
        }

        participants.insert(db_alias.clone(), Participant::new(db_alias));
        Ok(())
    }

    /// Get the number of participants
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::two_phase_commit::TwoPhaseCommit;
    ///
    /// let mut tpc = TwoPhaseCommit::new("txn_005");
    /// tpc.begin().unwrap();
    /// assert_eq!(tpc.participant_count(), 0);
    ///
    /// tpc.add_participant("db1").unwrap();
    /// assert_eq!(tpc.participant_count(), 1);
    /// ```
    pub fn participant_count(&self) -> usize {
        self.participants.lock().map(|p| p.len()).unwrap_or(0)
    }

    /// Prepare phase: ask all participants to prepare for commit
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::two_phase_commit::{TwoPhaseCommit, TransactionState};
    ///
    /// let mut tpc = TwoPhaseCommit::new("txn_006");
    /// tpc.begin().unwrap();
    /// tpc.add_participant("db1").unwrap();
    /// tpc.add_participant("db2").unwrap();
    ///
    /// let result = tpc.prepare();
    /// assert!(result.is_ok());
    /// assert_eq!(tpc.state().unwrap(), TransactionState::Prepared);
    /// ```
    pub fn prepare(&mut self) -> Result<Vec<String>, TwoPhaseError> {
        let state = self.state().unwrap();
        if state != TransactionState::Active {
            return Err(TwoPhaseError::InvalidState(
                "Can only prepare active transaction".to_string(),
            ));
        }

        let mut participants = self.participants.lock().map_err(|_| {
            TwoPhaseError::InvalidState("Failed to acquire participants lock".to_string())
        })?;

        if participants.is_empty() {
            return Err(TwoPhaseError::NoParticipants);
        }

        let mut prepared_sqls = Vec::new();

        for (db_alias, participant) in participants.iter_mut() {
            let sql = format!("PREPARE TRANSACTION '{}'", self.transaction_id);
            prepared_sqls.push(format!("{}: {}", db_alias, sql));
            participant.status = ParticipantStatus::Prepared;
        }

        let mut state = self
            .state
            .lock()
            .map_err(|_| TwoPhaseError::InvalidState("Failed to acquire state lock".to_string()))?;
        *state = TransactionState::Prepared;

        Ok(prepared_sqls)
    }

    /// Commit phase: commit the transaction on all prepared participants
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::two_phase_commit::{TwoPhaseCommit, TransactionState};
    ///
    /// let mut tpc = TwoPhaseCommit::new("txn_007");
    /// tpc.begin().unwrap();
    /// tpc.add_participant("db1").unwrap();
    /// tpc.prepare().unwrap();
    ///
    /// let result = tpc.commit();
    /// assert!(result.is_ok());
    /// assert_eq!(tpc.state().unwrap(), TransactionState::Committed);
    /// ```
    pub fn commit(&mut self) -> Result<Vec<String>, TwoPhaseError> {
        let state = self.state().unwrap();
        if state != TransactionState::Prepared {
            return Err(TwoPhaseError::InvalidState(
                "Can only commit prepared transaction".to_string(),
            ));
        }

        let mut participants = self.participants.lock().map_err(|_| {
            TwoPhaseError::InvalidState("Failed to acquire participants lock".to_string())
        })?;

        let mut commit_sqls = Vec::new();

        for (db_alias, participant) in participants.iter_mut() {
            if !participant.is_prepared() {
                return Err(TwoPhaseError::CommitFailed(
                    db_alias.clone(),
                    "Participant not prepared".to_string(),
                ));
            }

            let sql = format!("COMMIT PREPARED '{}'", self.transaction_id);
            commit_sqls.push(format!("{}: {}", db_alias, sql));
            participant.status = ParticipantStatus::Committed;
        }

        let mut state = self
            .state
            .lock()
            .map_err(|_| TwoPhaseError::InvalidState("Failed to acquire state lock".to_string()))?;
        *state = TransactionState::Committed;

        Ok(commit_sqls)
    }

    /// Rollback/abort the transaction on all participants
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::two_phase_commit::{TwoPhaseCommit, TransactionState};
    ///
    /// let mut tpc = TwoPhaseCommit::new("txn_008");
    /// tpc.begin().unwrap();
    /// tpc.add_participant("db1").unwrap();
    ///
    /// let result = tpc.rollback();
    /// assert!(result.is_ok());
    /// assert_eq!(tpc.state().unwrap(), TransactionState::Aborted);
    /// ```
    pub fn rollback(&mut self) -> Result<Vec<String>, TwoPhaseError> {
        let state = self.state().unwrap();
        if state != TransactionState::Active && state != TransactionState::Prepared {
            return Err(TwoPhaseError::InvalidState(
                "Can only rollback active or prepared transaction".to_string(),
            ));
        }

        let mut participants = self.participants.lock().map_err(|_| {
            TwoPhaseError::InvalidState("Failed to acquire participants lock".to_string())
        })?;

        let mut rollback_sqls = Vec::new();

        for (db_alias, participant) in participants.iter_mut() {
            let sql = if participant.is_prepared() {
                format!("ROLLBACK PREPARED '{}'", self.transaction_id)
            } else {
                "ROLLBACK".to_string()
            };
            rollback_sqls.push(format!("{}: {}", db_alias, sql));
            participant.status = ParticipantStatus::Aborted;
        }

        let mut state = self
            .state
            .lock()
            .map_err(|_| TwoPhaseError::InvalidState("Failed to acquire state lock".to_string()))?;
        *state = TransactionState::Aborted;

        Ok(rollback_sqls)
    }

    /// Get all participants in the transaction
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::two_phase_commit::TwoPhaseCommit;
    ///
    /// let mut tpc = TwoPhaseCommit::new("txn_009");
    /// tpc.begin().unwrap();
    /// tpc.add_participant("db1").unwrap();
    /// tpc.add_participant("db2").unwrap();
    ///
    /// let participants = tpc.participants();
    /// assert_eq!(participants.len(), 2);
    /// assert!(participants.contains_key("db1"));
    /// assert!(participants.contains_key("db2"));
    /// ```
    pub fn participants(&self) -> HashMap<String, Participant> {
        self.participants
            .lock()
            .map(|p| p.clone())
            .unwrap_or_default()
    }

    /// Check if all participants are prepared
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::two_phase_commit::TwoPhaseCommit;
    ///
    /// let mut tpc = TwoPhaseCommit::new("txn_010");
    /// tpc.begin().unwrap();
    /// tpc.add_participant("db1").unwrap();
    /// assert!(!tpc.all_prepared());
    ///
    /// tpc.prepare().unwrap();
    /// assert!(tpc.all_prepared());
    /// ```
    pub fn all_prepared(&self) -> bool {
        self.participants
            .lock()
            .map(|p| p.values().all(|participant| participant.is_prepared()))
            .unwrap_or(false)
    }
}

impl Default for TwoPhaseCommit {
    fn default() -> Self {
        Self::new(uuid::Uuid::new_v4().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_transaction() {
        let tpc = TwoPhaseCommit::new("txn_test_001");
        assert_eq!(tpc.transaction_id(), "txn_test_001");
        assert_eq!(tpc.state().unwrap(), TransactionState::NotStarted);
        assert_eq!(tpc.participant_count(), 0);
    }

    #[test]
    fn test_begin_transaction() {
        let mut tpc = TwoPhaseCommit::new("txn_test_002");
        let result = tpc.begin();
        assert!(result.is_ok());
        assert_eq!(tpc.state().unwrap(), TransactionState::Active);
    }

    #[test]
    fn test_cannot_begin_twice() {
        let mut tpc = TwoPhaseCommit::new("txn_test_003");
        tpc.begin().unwrap();
        let result = tpc.begin();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TwoPhaseError::InvalidState(_)
        ));
    }

    #[test]
    fn test_add_participant() {
        let mut tpc = TwoPhaseCommit::new("txn_test_004");
        tpc.begin().unwrap();
        let result = tpc.add_participant("db1");
        assert!(result.is_ok());
        assert_eq!(tpc.participant_count(), 1);
    }

    #[test]
    fn test_cannot_add_duplicate_participant() {
        let mut tpc = TwoPhaseCommit::new("txn_test_005");
        tpc.begin().unwrap();
        tpc.add_participant("db1").unwrap();
        let result = tpc.add_participant("db1");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TwoPhaseError::DuplicateParticipant(_)
        ));
    }

    #[test]
    fn test_prepare_phase() {
        let mut tpc = TwoPhaseCommit::new("txn_test_006");
        tpc.begin().unwrap();
        tpc.add_participant("db1").unwrap();
        tpc.add_participant("db2").unwrap();

        let result = tpc.prepare();
        assert!(result.is_ok());
        assert_eq!(tpc.state().unwrap(), TransactionState::Prepared);
        assert!(tpc.all_prepared());

        let sqls = result.unwrap();
        assert_eq!(sqls.len(), 2);
    }

    #[test]
    fn test_prepare_without_participants() {
        let mut tpc = TwoPhaseCommit::new("txn_test_007");
        tpc.begin().unwrap();
        let result = tpc.prepare();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TwoPhaseError::NoParticipants));
    }

    #[test]
    fn test_commit_phase() {
        let mut tpc = TwoPhaseCommit::new("txn_test_008");
        tpc.begin().unwrap();
        tpc.add_participant("db1").unwrap();
        tpc.prepare().unwrap();

        let result = tpc.commit();
        assert!(result.is_ok());
        assert_eq!(tpc.state().unwrap(), TransactionState::Committed);

        let sqls = result.unwrap();
        assert_eq!(sqls.len(), 1);
        assert!(sqls[0].contains("COMMIT PREPARED"));
    }

    #[test]
    fn test_cannot_commit_without_prepare() {
        let mut tpc = TwoPhaseCommit::new("txn_test_009");
        tpc.begin().unwrap();
        tpc.add_participant("db1").unwrap();

        let result = tpc.commit();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TwoPhaseError::InvalidState(_)
        ));
    }

    #[test]
    fn test_rollback_active_transaction() {
        let mut tpc = TwoPhaseCommit::new("txn_test_010");
        tpc.begin().unwrap();
        tpc.add_participant("db1").unwrap();

        let result = tpc.rollback();
        assert!(result.is_ok());
        assert_eq!(tpc.state().unwrap(), TransactionState::Aborted);

        let sqls = result.unwrap();
        assert_eq!(sqls.len(), 1);
        assert!(sqls[0].contains("ROLLBACK"));
    }

    #[test]
    fn test_rollback_prepared_transaction() {
        let mut tpc = TwoPhaseCommit::new("txn_test_011");
        tpc.begin().unwrap();
        tpc.add_participant("db1").unwrap();
        tpc.prepare().unwrap();

        let result = tpc.rollback();
        assert!(result.is_ok());
        assert_eq!(tpc.state().unwrap(), TransactionState::Aborted);

        let sqls = result.unwrap();
        assert_eq!(sqls.len(), 1);
        assert!(sqls[0].contains("ROLLBACK PREPARED"));
    }

    #[test]
    fn test_multiple_participants_prepare_commit() {
        let mut tpc = TwoPhaseCommit::new("txn_test_012");
        tpc.begin().unwrap();
        tpc.add_participant("primary_db").unwrap();
        tpc.add_participant("secondary_db").unwrap();
        tpc.add_participant("cache_db").unwrap();

        assert_eq!(tpc.participant_count(), 3);

        tpc.prepare().unwrap();
        assert!(tpc.all_prepared());

        tpc.commit().unwrap();
        assert_eq!(tpc.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    fn test_participant_new() {
        let participant = Participant::new("test_db");
        assert_eq!(participant.db_alias, "test_db");
        assert_eq!(participant.status, ParticipantStatus::Active);
        assert!(!participant.is_prepared());
    }

    #[test]
    fn test_participant_is_prepared() {
        let mut participant = Participant::new("test_db");
        assert!(!participant.is_prepared());

        participant.status = ParticipantStatus::Prepared;
        assert!(participant.is_prepared());
    }

    #[test]
    fn test_get_participants() {
        let mut tpc = TwoPhaseCommit::new("txn_test_013");
        tpc.begin().unwrap();
        tpc.add_participant("db1").unwrap();
        tpc.add_participant("db2").unwrap();

        let participants = tpc.participants();
        assert_eq!(participants.len(), 2);
        assert!(participants.contains_key("db1"));
        assert!(participants.contains_key("db2"));
    }

    #[test]
    fn test_default_transaction_id() {
        let tpc = TwoPhaseCommit::default();
        assert!(!tpc.transaction_id().is_empty());
        assert_eq!(tpc.state().unwrap(), TransactionState::NotStarted);
    }

    #[test]
    fn test_transaction_state_transitions() {
        let mut tpc = TwoPhaseCommit::new("txn_test_014");
        assert_eq!(tpc.state().unwrap(), TransactionState::NotStarted);

        tpc.begin().unwrap();
        assert_eq!(tpc.state().unwrap(), TransactionState::Active);

        tpc.add_participant("db1").unwrap();
        tpc.prepare().unwrap();
        assert_eq!(tpc.state().unwrap(), TransactionState::Prepared);

        tpc.commit().unwrap();
        assert_eq!(tpc.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    fn test_error_display() {
        let err = TwoPhaseError::PrepareFailed("db1".to_string(), "Connection lost".to_string());
        assert_eq!(err.to_string(), "Prepare failed for 'db1': Connection lost");

        let err = TwoPhaseError::NoParticipants;
        assert_eq!(err.to_string(), "No participants registered");

        let err = TwoPhaseError::DuplicateParticipant("db1".to_string());
        assert_eq!(err.to_string(), "Participant 'db1' already registered");
    }

    #[test]
    fn test_cannot_add_participant_before_begin() {
        let mut tpc = TwoPhaseCommit::new("txn_test_015");
        let result = tpc.add_participant("db1");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TwoPhaseError::InvalidState(_)
        ));
    }
}
