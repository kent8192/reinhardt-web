//! Two-Phase Commit (2PC) module
//!
//! This module provides support for distributed transactions across multiple databases.

pub mod core;
pub mod transaction_log;

#[cfg(test)]
mod tests;

// Re-export core types
pub use core::{
	Participant, ParticipantStatus, TransactionState, TwoPhaseCommit, TwoPhaseCoordinator,
	TwoPhaseError, TwoPhaseParticipant,
};

#[cfg(test)]
pub use core::MockParticipant;

// Re-export adapters
#[cfg(feature = "postgres")]
pub use core::PostgresParticipantAdapter;

#[cfg(feature = "mysql")]
pub use core::MySqlParticipantAdapter;

// Re-export transaction log types
pub use transaction_log::{
	FileTransactionLog, InMemoryTransactionLog, TransactionLog, TransactionLogEntry,
};
