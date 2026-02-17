//! Transaction Log
//!
//! Persists the state of Two-Phase Commit transactions and enables failure recovery.

use super::{TransactionState, TwoPhaseError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Transaction log entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransactionLogEntry {
	/// Transaction ID
	pub transaction_id: String,
	/// Current state
	pub state: TransactionState,
	/// List of participants
	pub participants: Vec<String>,
	/// Timestamp of log entry
	pub timestamp: chrono::DateTime<chrono::Utc>,
	/// Additional metadata
	pub metadata: HashMap<String, String>,
}

impl TransactionLogEntry {
	/// Create a new log entry
	pub fn new(
		transaction_id: impl Into<String>,
		state: TransactionState,
		participants: Vec<String>,
	) -> Self {
		Self {
			transaction_id: transaction_id.into(),
			state,
			participants,
			timestamp: chrono::Utc::now(),
			metadata: HashMap::new(),
		}
	}

	/// Add metadata
	pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
		self.metadata.insert(key.into(), value.into());
		self
	}
}

/// Transaction log interface
pub trait TransactionLog: Send + Sync {
	/// Write a log entry
	fn write(&self, entry: &TransactionLogEntry) -> Result<(), TwoPhaseError>;

	/// Read a log entry by transaction ID
	fn read(&self, transaction_id: &str) -> Result<Option<TransactionLogEntry>, TwoPhaseError>;

	/// Read all log entries
	fn read_all(&self) -> Result<Vec<TransactionLogEntry>, TwoPhaseError>;

	/// Delete a log entry
	fn delete(&self, transaction_id: &str) -> Result<(), TwoPhaseError>;

	/// Find transactions by specific state
	fn find_by_state(
		&self,
		state: TransactionState,
	) -> Result<Vec<TransactionLogEntry>, TwoPhaseError>;
}

/// In-memory transaction log
///
/// Simple memory-based implementation for testing. Persisted logs should be used in production environments.
#[derive(Debug, Clone)]
pub struct InMemoryTransactionLog {
	entries: Arc<Mutex<HashMap<String, TransactionLogEntry>>>,
}

impl InMemoryTransactionLog {
	/// Create a new in-memory log
	pub fn new() -> Self {
		Self {
			entries: Arc::new(Mutex::new(HashMap::new())),
		}
	}
}

impl Default for InMemoryTransactionLog {
	fn default() -> Self {
		Self::new()
	}
}

impl TransactionLog for InMemoryTransactionLog {
	fn write(&self, entry: &TransactionLogEntry) -> Result<(), TwoPhaseError> {
		let mut entries = self
			.entries
			.lock()
			.map_err(|_| TwoPhaseError::LogError("Failed to acquire lock".to_string()))?;
		entries.insert(entry.transaction_id.clone(), entry.clone());
		Ok(())
	}

	fn read(&self, transaction_id: &str) -> Result<Option<TransactionLogEntry>, TwoPhaseError> {
		let entries = self
			.entries
			.lock()
			.map_err(|_| TwoPhaseError::LogError("Failed to acquire lock".to_string()))?;
		Ok(entries.get(transaction_id).cloned())
	}

	fn read_all(&self) -> Result<Vec<TransactionLogEntry>, TwoPhaseError> {
		let entries = self
			.entries
			.lock()
			.map_err(|_| TwoPhaseError::LogError("Failed to acquire lock".to_string()))?;
		Ok(entries.values().cloned().collect())
	}

	fn delete(&self, transaction_id: &str) -> Result<(), TwoPhaseError> {
		let mut entries = self
			.entries
			.lock()
			.map_err(|_| TwoPhaseError::LogError("Failed to acquire lock".to_string()))?;
		entries.remove(transaction_id);
		Ok(())
	}

	fn find_by_state(
		&self,
		state: TransactionState,
	) -> Result<Vec<TransactionLogEntry>, TwoPhaseError> {
		let entries = self
			.entries
			.lock()
			.map_err(|_| TwoPhaseError::LogError("Failed to acquire lock".to_string()))?;
		Ok(entries
			.values()
			.filter(|e| e.state == state)
			.cloned()
			.collect())
	}
}

/// File-based transaction log
///
/// Persists transaction state to the file system in JSON format.
#[derive(Debug)]
pub struct FileTransactionLog {
	log_dir: PathBuf,
}

impl FileTransactionLog {
	/// Create a new file-based log
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::two_phase_commit::transaction_log::FileTransactionLog;
	/// use std::path::PathBuf;
	///
	/// let log = FileTransactionLog::new(PathBuf::from("/var/log/2pc"));
	/// ```
	pub fn new(log_dir: PathBuf) -> Result<Self, TwoPhaseError> {
		// Create log directory if it doesn't exist
		if !log_dir.exists() {
			std::fs::create_dir_all(&log_dir).map_err(|e| {
				TwoPhaseError::LogError(format!("Failed to create log directory: {}", e))
			})?;
		}

		Ok(Self { log_dir })
	}

	/// Generate file path from transaction ID
	fn entry_path(&self, transaction_id: &str) -> PathBuf {
		self.log_dir.join(format!("{}.json", transaction_id))
	}
}

impl TransactionLog for FileTransactionLog {
	fn write(&self, entry: &TransactionLogEntry) -> Result<(), TwoPhaseError> {
		let path = self.entry_path(&entry.transaction_id);
		let json = serde_json::to_string_pretty(entry)
			.map_err(|e| TwoPhaseError::LogError(format!("Serialization error: {}", e)))?;

		std::fs::write(&path, json)
			.map_err(|e| TwoPhaseError::LogError(format!("Failed to write log file: {}", e)))?;

		Ok(())
	}

	fn read(&self, transaction_id: &str) -> Result<Option<TransactionLogEntry>, TwoPhaseError> {
		let path = self.entry_path(transaction_id);

		if !path.exists() {
			return Ok(None);
		}

		let json = std::fs::read_to_string(&path)
			.map_err(|e| TwoPhaseError::LogError(format!("Failed to read log file: {}", e)))?;

		let entry: TransactionLogEntry = serde_json::from_str(&json)
			.map_err(|e| TwoPhaseError::LogError(format!("Deserialization error: {}", e)))?;

		Ok(Some(entry))
	}

	fn read_all(&self) -> Result<Vec<TransactionLogEntry>, TwoPhaseError> {
		let mut entries = Vec::new();

		let dir_entries = std::fs::read_dir(&self.log_dir)
			.map_err(|e| TwoPhaseError::LogError(format!("Failed to read log directory: {}", e)))?;

		for entry_result in dir_entries {
			let entry = entry_result.map_err(|e| {
				TwoPhaseError::LogError(format!("Failed to read directory entry: {}", e))
			})?;

			let path = entry.path();
			if path.extension().and_then(|s| s.to_str()) == Some("json") {
				let json = std::fs::read_to_string(&path).map_err(|e| {
					TwoPhaseError::LogError(format!("Failed to read log file: {}", e))
				})?;

				let log_entry: TransactionLogEntry = serde_json::from_str(&json).map_err(|e| {
					TwoPhaseError::LogError(format!("Deserialization error: {}", e))
				})?;

				entries.push(log_entry);
			}
		}

		Ok(entries)
	}

	fn delete(&self, transaction_id: &str) -> Result<(), TwoPhaseError> {
		let path = self.entry_path(transaction_id);

		if path.exists() {
			std::fs::remove_file(&path).map_err(|e| {
				TwoPhaseError::LogError(format!("Failed to delete log file: {}", e))
			})?;
		}

		Ok(())
	}

	fn find_by_state(
		&self,
		state: TransactionState,
	) -> Result<Vec<TransactionLogEntry>, TwoPhaseError> {
		let all_entries = self.read_all()?;
		Ok(all_entries
			.into_iter()
			.filter(|e| e.state == state)
			.collect())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_log_entry_creation() {
		let entry = TransactionLogEntry::new(
			"txn_001",
			TransactionState::Active,
			vec!["db1".to_string(), "db2".to_string()],
		);

		assert_eq!(entry.transaction_id, "txn_001");
		assert_eq!(entry.state, TransactionState::Active);
		assert_eq!(entry.participants.len(), 2);
		assert!(entry.metadata.is_empty());
	}

	#[rstest]
	fn test_log_entry_with_metadata() {
		let entry = TransactionLogEntry::new("txn_002", TransactionState::Prepared, vec![])
			.with_metadata("user", "alice")
			.with_metadata("operation", "transfer");

		assert_eq!(entry.metadata.get("user"), Some(&"alice".to_string()));
		assert_eq!(
			entry.metadata.get("operation"),
			Some(&"transfer".to_string())
		);
	}

	#[rstest]
	fn test_in_memory_log_write_read() {
		let log = InMemoryTransactionLog::new();
		let entry =
			TransactionLogEntry::new("txn_003", TransactionState::Active, vec!["db1".to_string()]);

		log.write(&entry).unwrap();
		let read_entry = log.read("txn_003").unwrap();

		assert_eq!(read_entry, Some(entry));
	}

	#[rstest]
	fn test_in_memory_log_delete() {
		let log = InMemoryTransactionLog::new();
		let entry = TransactionLogEntry::new("txn_004", TransactionState::Active, vec![]);

		log.write(&entry).unwrap();
		assert!(log.read("txn_004").unwrap().is_some());

		log.delete("txn_004").unwrap();
		assert!(log.read("txn_004").unwrap().is_none());
	}

	#[rstest]
	fn test_in_memory_log_find_by_state() {
		let log = InMemoryTransactionLog::new();

		log.write(&TransactionLogEntry::new(
			"txn_005",
			TransactionState::Active,
			vec![],
		))
		.unwrap();
		log.write(&TransactionLogEntry::new(
			"txn_006",
			TransactionState::Prepared,
			vec![],
		))
		.unwrap();
		log.write(&TransactionLogEntry::new(
			"txn_007",
			TransactionState::Prepared,
			vec![],
		))
		.unwrap();

		let prepared = log.find_by_state(TransactionState::Prepared).unwrap();
		assert_eq!(prepared.len(), 2);
	}

	#[rstest]
	fn test_in_memory_log_read_all() {
		let log = InMemoryTransactionLog::new();

		log.write(&TransactionLogEntry::new(
			"txn_008",
			TransactionState::Active,
			vec![],
		))
		.unwrap();
		log.write(&TransactionLogEntry::new(
			"txn_009",
			TransactionState::Prepared,
			vec![],
		))
		.unwrap();

		let all = log.read_all().unwrap();
		assert_eq!(all.len(), 2);
	}

	#[rstest]
	fn test_file_log_write_read() {
		let temp_dir = std::env::temp_dir().join("reinhardt_2pc_test");
		let _ = std::fs::remove_dir_all(&temp_dir); // Clean up if exists

		let log = FileTransactionLog::new(temp_dir.clone()).unwrap();
		let entry = TransactionLogEntry::new(
			"txn_file_001",
			TransactionState::Active,
			vec!["db1".to_string()],
		);

		log.write(&entry).unwrap();
		let read_entry = log.read("txn_file_001").unwrap();

		assert_eq!(
			read_entry.as_ref().map(|e| &e.transaction_id),
			Some(&"txn_file_001".to_string())
		);

		// Cleanup
		std::fs::remove_dir_all(&temp_dir).unwrap();
	}

	#[rstest]
	fn test_file_log_delete() {
		let temp_dir = std::env::temp_dir().join("reinhardt_2pc_test_delete");
		let _ = std::fs::remove_dir_all(&temp_dir);

		let log = FileTransactionLog::new(temp_dir.clone()).unwrap();
		let entry = TransactionLogEntry::new("txn_file_002", TransactionState::Active, vec![]);

		log.write(&entry).unwrap();
		assert!(log.read("txn_file_002").unwrap().is_some());

		log.delete("txn_file_002").unwrap();
		assert!(log.read("txn_file_002").unwrap().is_none());

		// Cleanup
		std::fs::remove_dir_all(&temp_dir).unwrap();
	}

	#[rstest]
	fn test_file_log_find_by_state() {
		let temp_dir = std::env::temp_dir().join("reinhardt_2pc_test_state");
		let _ = std::fs::remove_dir_all(&temp_dir);

		let log = FileTransactionLog::new(temp_dir.clone()).unwrap();

		log.write(&TransactionLogEntry::new(
			"txn_file_003",
			TransactionState::Active,
			vec![],
		))
		.unwrap();
		log.write(&TransactionLogEntry::new(
			"txn_file_004",
			TransactionState::Prepared,
			vec![],
		))
		.unwrap();
		log.write(&TransactionLogEntry::new(
			"txn_file_005",
			TransactionState::Prepared,
			vec![],
		))
		.unwrap();

		let prepared = log.find_by_state(TransactionState::Prepared).unwrap();
		assert_eq!(prepared.len(), 2);

		// Cleanup
		std::fs::remove_dir_all(&temp_dir).unwrap();
	}
}
