//! Secret access audit logging
//!
//! This module provides audit trail functionality for tracking secret accesses.
//!
//! ## Features
//!
//! - Track all secret access attempts
//! - Record success and failure
//! - Multiple storage backends (memory, file, database)
//! - Query and filter access logs
//!
//! ## Example
//!
//! ```rust
//! use reinhardt_conf::settings::secrets::audit::{SecretAuditLogger, SecretAccessEvent};
//! use reinhardt_conf::settings::secrets::audit::backends::MemorySecretAuditBackend;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), String> {
//! let backend = Arc::new(MemorySecretAuditBackend::new());
//! let logger = SecretAuditLogger::new(backend);
//!
//! // Log a secret access
//! let event = SecretAccessEvent::new(
//!     "database_password".to_string(),
//!     "app_server".to_string(),
//!     true,
//!     None,
//! );
//!
//! logger.log_access(event).await?;
//! # Ok(())
//! # }
//! ```

pub mod backends;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Secret access event
///
/// ## Example
///
/// ```rust
/// use reinhardt_conf::settings::secrets::audit::SecretAccessEvent;
///
/// let event = SecretAccessEvent::new(
///     "api_key".to_string(),
///     "web_service".to_string(),
///     true,
///     Some("Regular access".to_string()),
/// );
///
/// assert_eq!(event.secret_name, "api_key");
/// assert_eq!(event.accessor, "web_service");
/// assert!(event.success);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretAccessEvent {
	/// Event timestamp
	pub timestamp: DateTime<Utc>,
	/// Name of the secret accessed
	pub secret_name: String,
	/// Identifier of who accessed the secret
	pub accessor: String,
	/// Whether access was successful
	pub success: bool,
	/// Optional context information
	pub context: Option<String>,
}

impl SecretAccessEvent {
	/// Create a new secret access event
	///
	/// ## Example
	///
	/// ```rust
	/// use reinhardt_conf::settings::secrets::audit::SecretAccessEvent;
	///
	/// let event = SecretAccessEvent::new(
	///     "secret_key".to_string(),
	///     "app".to_string(),
	///     true,
	///     None,
	/// );
	///
	/// assert!(event.timestamp <= chrono::Utc::now());
	/// ```
	pub fn new(
		secret_name: String,
		accessor: String,
		success: bool,
		context: Option<String>,
	) -> Self {
		Self {
			timestamp: Utc::now(),
			secret_name,
			accessor,
			success,
			context,
		}
	}
}

/// Filter for querying secret access events
///
/// ## Example
///
/// ```rust
/// use reinhardt_conf::settings::secrets::audit::SecretAccessFilter;
///
/// let filter = SecretAccessFilter {
///     secret_name: Some("api_key".to_string()),
///     accessor: Some("service".to_string()),
///     success_only: Some(true),
///     start_time: None,
///     end_time: None,
/// };
///
/// assert_eq!(filter.secret_name, Some("api_key".to_string()));
/// ```
#[derive(Debug, Clone, Default)]
pub struct SecretAccessFilter {
	/// Filter by secret name
	pub secret_name: Option<String>,
	/// Filter by accessor
	pub accessor: Option<String>,
	/// Filter to only successful accesses
	pub success_only: Option<bool>,
	/// Filter events after this time
	pub start_time: Option<DateTime<Utc>>,
	/// Filter events before this time
	pub end_time: Option<DateTime<Utc>>,
}

/// Trait for secret audit backends
///
/// ## Example
///
/// ```rust,no_run
/// use reinhardt_conf::settings::secrets::audit::{SecretAuditBackend, SecretAccessEvent, SecretAccessFilter};
/// use std::sync::Mutex;
///
/// struct CustomBackend {
///     events: Mutex<Vec<SecretAccessEvent>>,
/// }
///
/// #[async_trait::async_trait]
/// impl SecretAuditBackend for CustomBackend {
///     async fn log_access(&self, event: SecretAccessEvent) -> Result<(), String> {
///         self.events.lock().unwrap().push(event);
///         Ok(())
///     }
///
///     async fn get_accesses(&self, _filter: Option<SecretAccessFilter>) -> Result<Vec<SecretAccessEvent>, String> {
///         Ok(self.events.lock().unwrap().clone())
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait SecretAuditBackend: Send + Sync {
	/// Log a secret access event
	async fn log_access(&self, event: SecretAccessEvent) -> Result<(), String>;

	/// Retrieve secret access events with optional filtering
	async fn get_accesses(
		&self,
		filter: Option<SecretAccessFilter>,
	) -> Result<Vec<SecretAccessEvent>, String>;
}

/// Secret audit logger
///
/// ## Example
///
/// ```rust
/// use reinhardt_conf::settings::secrets::audit::{SecretAuditLogger, SecretAccessEvent};
/// use reinhardt_conf::settings::secrets::audit::backends::MemorySecretAuditBackend;
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), String> {
/// let backend = Arc::new(MemorySecretAuditBackend::new());
/// let logger = SecretAuditLogger::new(backend);
///
/// let event = SecretAccessEvent::new(
///     "secret".to_string(),
///     "app".to_string(),
///     true,
///     None,
/// );
///
/// logger.log_access(event).await?;
/// # Ok(())
/// # }
/// ```
pub struct SecretAuditLogger {
	backend: Arc<dyn SecretAuditBackend>,
}

impl SecretAuditLogger {
	/// Create a new secret audit logger
	///
	/// ## Example
	///
	/// ```rust
	/// use reinhardt_conf::settings::secrets::audit::SecretAuditLogger;
	/// use reinhardt_conf::settings::secrets::audit::backends::MemorySecretAuditBackend;
	/// use std::sync::Arc;
	///
	/// let backend = Arc::new(MemorySecretAuditBackend::new());
	/// let logger = SecretAuditLogger::new(backend);
	/// ```
	pub fn new(backend: Arc<dyn SecretAuditBackend>) -> Self {
		Self { backend }
	}

	/// Log a secret access event
	///
	/// ## Example
	///
	/// ```rust
	/// use reinhardt_conf::settings::secrets::audit::{SecretAuditLogger, SecretAccessEvent};
	/// use reinhardt_conf::settings::secrets::audit::backends::MemorySecretAuditBackend;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), String> {
	/// let backend = Arc::new(MemorySecretAuditBackend::new());
	/// let logger = SecretAuditLogger::new(backend);
	///
	/// let event = SecretAccessEvent::new("key".to_string(), "app".to_string(), true, None);
	/// logger.log_access(event).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn log_access(&self, event: SecretAccessEvent) -> Result<(), String> {
		self.backend.log_access(event).await
	}

	/// Get secret access events with optional filtering
	///
	/// ## Example
	///
	/// ```rust
	/// use reinhardt_conf::settings::secrets::audit::{SecretAuditLogger, SecretAccessFilter};
	/// use reinhardt_conf::settings::secrets::audit::backends::MemorySecretAuditBackend;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), String> {
	/// let backend = Arc::new(MemorySecretAuditBackend::new());
	/// let logger = SecretAuditLogger::new(backend);
	///
	/// let filter = SecretAccessFilter {
	///     success_only: Some(true),
	///     ..Default::default()
	/// };
	///
	/// let events = logger.get_accesses(Some(filter)).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn get_accesses(
		&self,
		filter: Option<SecretAccessFilter>,
	) -> Result<Vec<SecretAccessEvent>, String> {
		self.backend.get_accesses(filter).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use backends::MemorySecretAuditBackend;
	use rstest::rstest;

	#[rstest]
	fn test_secret_access_event_creation() {
		let event = SecretAccessEvent::new(
			"test_secret".to_string(),
			"test_accessor".to_string(),
			true,
			Some("context".to_string()),
		);

		assert_eq!(event.secret_name, "test_secret");
		assert_eq!(event.accessor, "test_accessor");
		assert!(event.success);
		assert_eq!(event.context, Some("context".to_string()));
		assert!(event.timestamp <= Utc::now());
	}

	#[rstest]
	#[tokio::test]
	async fn test_secret_audit_logger() {
		let backend = Arc::new(MemorySecretAuditBackend::new());
		let logger = SecretAuditLogger::new(backend);

		let event = SecretAccessEvent::new("secret".to_string(), "app".to_string(), true, None);

		logger.log_access(event).await.unwrap();

		let events = logger.get_accesses(None).await.unwrap();
		assert_eq!(events.len(), 1);
	}

	#[rstest]
	#[tokio::test]
	async fn test_secret_audit_logger_with_filter() {
		let backend = Arc::new(MemorySecretAuditBackend::new());
		let logger = SecretAuditLogger::new(backend);

		// Log successful and failed accesses
		let event1 = SecretAccessEvent::new("s1".to_string(), "app".to_string(), true, None);
		let event2 = SecretAccessEvent::new("s2".to_string(), "app".to_string(), false, None);
		let event3 = SecretAccessEvent::new("s1".to_string(), "app".to_string(), true, None);

		logger.log_access(event1).await.unwrap();
		logger.log_access(event2).await.unwrap();
		logger.log_access(event3).await.unwrap();

		let filter = SecretAccessFilter {
			success_only: Some(true),
			..Default::default()
		};

		let events = logger.get_accesses(Some(filter)).await.unwrap();
		assert_eq!(events.len(), 2);
	}

	#[rstest]
	#[tokio::test]
	async fn test_secret_audit_filter_by_name() {
		let backend = Arc::new(MemorySecretAuditBackend::new());
		let logger = SecretAuditLogger::new(backend);

		let event1 = SecretAccessEvent::new("secret1".to_string(), "app".to_string(), true, None);
		let event2 = SecretAccessEvent::new("secret2".to_string(), "app".to_string(), true, None);
		let event3 = SecretAccessEvent::new("secret1".to_string(), "app".to_string(), true, None);

		logger.log_access(event1).await.unwrap();
		logger.log_access(event2).await.unwrap();
		logger.log_access(event3).await.unwrap();

		let filter = SecretAccessFilter {
			secret_name: Some("secret1".to_string()),
			..Default::default()
		};

		let events = logger.get_accesses(Some(filter)).await.unwrap();
		assert_eq!(events.len(), 2);
	}
}
