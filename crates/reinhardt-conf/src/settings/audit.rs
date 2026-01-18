//! Audit logging for configuration changes
//!
//! This module provides audit trail functionality for tracking configuration changes.
//! It supports multiple backends for storing audit events.
//!
//! ## Features
//!
//! - Track all configuration changes with detailed metadata
//! - Multiple storage backends (memory, file, database)
//! - Filtering and querying audit events
//! - User and timestamp tracking
//!
//! ## Example
//!
//! ```rust
//! use reinhardt_conf::settings::audit::{AuditLogger, AuditEvent, EventType, ChangeRecord};
//! use reinhardt_conf::settings::audit::backends::MemoryAuditBackend;
//! use std::sync::Arc;
//! use std::collections::HashMap;
//!
//! # async fn example() -> Result<(), String> {
//! // Create an audit logger with memory backend
//! let backend = Arc::new(MemoryAuditBackend::new());
//! let logger = AuditLogger::new(backend);
//!
//! // Log a configuration change
//! let mut changes = HashMap::new();
//! changes.insert(
//!     "debug".to_string(),
//!     ChangeRecord {
//!         old_value: Some(serde_json::json!(false)),
//!         new_value: Some(serde_json::json!(true)),
//!     },
//! );
//!
//! let event = AuditEvent::new(
//!     EventType::ConfigUpdate,
//!     Some("admin".to_string()),
//!     changes,
//! );
//!
//! logger.log_event(event).await?;
//!
//! // Query events
//! let events = logger.get_events(None).await?;
//! assert_eq!(events.len(), 1);
//! # Ok(())
//! # }
//! ```

pub mod backends;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Type of audit event
///
/// ## Example
///
/// ```rust
/// use reinhardt_conf::settings::audit::EventType;
///
/// let event_type = EventType::ConfigUpdate;
/// assert_eq!(event_type.as_str(), "config_update");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventType {
	/// Configuration value updated
	ConfigUpdate,
	/// Configuration value deleted
	ConfigDelete,
	/// Configuration value created
	ConfigCreate,
	/// Secret accessed
	SecretAccess,
	/// Secret rotated
	SecretRotation,
}

impl EventType {
	/// Get string representation of event type
	///
	/// ## Example
	///
	/// ```rust
	/// use reinhardt_conf::settings::audit::EventType;
	///
	/// assert_eq!(EventType::ConfigUpdate.as_str(), "config_update");
	/// assert_eq!(EventType::SecretAccess.as_str(), "secret_access");
	/// ```
	pub fn as_str(&self) -> &str {
		match self {
			EventType::ConfigUpdate => "config_update",
			EventType::ConfigDelete => "config_delete",
			EventType::ConfigCreate => "config_create",
			EventType::SecretAccess => "secret_access",
			EventType::SecretRotation => "secret_rotation",
		}
	}
}

/// Record of a configuration change
///
/// ## Example
///
/// ```rust
/// use reinhardt_conf::settings::audit::ChangeRecord;
/// use serde_json::json;
///
/// let record = ChangeRecord {
///     old_value: Some(json!(false)),
///     new_value: Some(json!(true)),
/// };
///
/// assert_eq!(record.old_value, Some(json!(false)));
/// assert_eq!(record.new_value, Some(json!(true)));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRecord {
	/// Previous value (None if newly created)
	pub old_value: Option<serde_json::Value>,
	/// New value (None if deleted)
	pub new_value: Option<serde_json::Value>,
}

/// Audit event representing a configuration change or access
///
/// ## Example
///
/// ```rust
/// use reinhardt_conf::settings::audit::{AuditEvent, EventType, ChangeRecord};
/// use std::collections::HashMap;
/// use serde_json::json;
///
/// let mut changes = HashMap::new();
/// changes.insert(
///     "port".to_string(),
///     ChangeRecord {
///         old_value: Some(json!(8000)),
///         new_value: Some(json!(8080)),
///     },
/// );
///
/// let event = AuditEvent::new(
///     EventType::ConfigUpdate,
///     Some("admin".to_string()),
///     changes,
/// );
///
/// assert_eq!(event.event_type, EventType::ConfigUpdate);
/// assert_eq!(event.user, Some("admin".to_string()));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
	/// Event timestamp
	pub timestamp: DateTime<Utc>,
	/// Type of event
	pub event_type: EventType,
	/// User who performed the action (if available)
	pub user: Option<String>,
	/// Map of configuration keys to their changes
	pub changes: HashMap<String, ChangeRecord>,
}

impl AuditEvent {
	/// Create a new audit event
	///
	/// ## Example
	///
	/// ```rust
	/// use reinhardt_conf::settings::audit::{AuditEvent, EventType, ChangeRecord};
	/// use std::collections::HashMap;
	/// use serde_json::json;
	///
	/// let mut changes = HashMap::new();
	/// changes.insert(
	///     "setting".to_string(),
	///     ChangeRecord {
	///         old_value: None,
	///         new_value: Some(json!("value")),
	///     },
	/// );
	///
	/// let event = AuditEvent::new(
	///     EventType::ConfigCreate,
	///     Some("system".to_string()),
	///     changes,
	/// );
	///
	/// assert!(event.timestamp <= chrono::Utc::now());
	/// ```
	pub fn new(
		event_type: EventType,
		user: Option<String>,
		changes: HashMap<String, ChangeRecord>,
	) -> Self {
		Self {
			timestamp: Utc::now(),
			event_type,
			user,
			changes,
		}
	}
}

/// Filter for querying audit events
///
/// ## Example
///
/// ```rust
/// use reinhardt_conf::settings::audit::{EventFilter, EventType};
/// use chrono::Utc;
///
/// let filter = EventFilter {
///     event_type: Some(EventType::ConfigUpdate),
///     user: Some("admin".to_string()),
///     start_time: None,
///     end_time: Some(Utc::now()),
/// };
///
/// assert_eq!(filter.event_type, Some(EventType::ConfigUpdate));
/// ```
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
	/// Filter by event type
	pub event_type: Option<EventType>,
	/// Filter by user
	pub user: Option<String>,
	/// Filter events after this time
	pub start_time: Option<DateTime<Utc>>,
	/// Filter events before this time
	pub end_time: Option<DateTime<Utc>>,
}

/// Trait for audit backends
///
/// Implement this trait to create custom audit storage backends.
///
/// ## Example
///
/// ```rust,no_run
/// use reinhardt_conf::settings::audit::{AuditBackend, AuditEvent, EventFilter};
/// use std::sync::Mutex;
///
/// struct CustomBackend {
///     events: Mutex<Vec<AuditEvent>>,
/// }
///
/// #[async_trait::async_trait]
/// impl AuditBackend for CustomBackend {
///     async fn log_event(&self, event: AuditEvent) -> Result<(), String> {
///         self.events.lock().unwrap().push(event);
///         Ok(())
///     }
///
///     async fn get_events(&self, _filter: Option<EventFilter>) -> Result<Vec<AuditEvent>, String> {
///         Ok(self.events.lock().unwrap().clone())
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait AuditBackend: Send + Sync {
	/// Log an audit event
	async fn log_event(&self, event: AuditEvent) -> Result<(), String>;

	/// Retrieve audit events with optional filtering
	async fn get_events(&self, filter: Option<EventFilter>) -> Result<Vec<AuditEvent>, String>;
}

/// Audit logger for configuration changes
///
/// ## Example
///
/// ```rust
/// use reinhardt_conf::settings::audit::{AuditLogger, AuditEvent, EventType, ChangeRecord};
/// use reinhardt_conf::settings::audit::backends::MemoryAuditBackend;
/// use std::sync::Arc;
/// use std::collections::HashMap;
/// use serde_json::json;
///
/// # async fn example() -> Result<(), String> {
/// let backend = Arc::new(MemoryAuditBackend::new());
/// let logger = AuditLogger::new(backend);
///
/// let mut changes = HashMap::new();
/// changes.insert(
///     "key".to_string(),
///     ChangeRecord {
///         old_value: Some(json!("old")),
///         new_value: Some(json!("new")),
///     },
/// );
///
/// let event = AuditEvent::new(EventType::ConfigUpdate, None, changes);
/// logger.log_event(event).await?;
/// # Ok(())
/// # }
/// ```
pub struct AuditLogger {
	backend: Arc<dyn AuditBackend>,
}

impl AuditLogger {
	/// Create a new audit logger with the specified backend
	///
	/// ## Example
	///
	/// ```rust
	/// use reinhardt_conf::settings::audit::AuditLogger;
	/// use reinhardt_conf::settings::audit::backends::MemoryAuditBackend;
	/// use std::sync::Arc;
	///
	/// let backend = Arc::new(MemoryAuditBackend::new());
	/// let logger = AuditLogger::new(backend);
	/// ```
	pub fn new(backend: Arc<dyn AuditBackend>) -> Self {
		Self { backend }
	}

	/// Log an audit event
	///
	/// ## Example
	///
	/// ```rust
	/// use reinhardt_conf::settings::audit::{AuditLogger, AuditEvent, EventType};
	/// use reinhardt_conf::settings::audit::backends::MemoryAuditBackend;
	/// use std::sync::Arc;
	/// use std::collections::HashMap;
	///
	/// # async fn example() -> Result<(), String> {
	/// let backend = Arc::new(MemoryAuditBackend::new());
	/// let logger = AuditLogger::new(backend);
	///
	/// let event = AuditEvent::new(EventType::ConfigCreate, None, HashMap::new());
	/// logger.log_event(event).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn log_event(&self, event: AuditEvent) -> Result<(), String> {
		self.backend.log_event(event).await
	}

	/// Get audit events with optional filtering
	///
	/// ## Example
	///
	/// ```rust
	/// use reinhardt_conf::settings::audit::{AuditLogger, EventFilter, EventType};
	/// use reinhardt_conf::settings::audit::backends::MemoryAuditBackend;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), String> {
	/// let backend = Arc::new(MemoryAuditBackend::new());
	/// let logger = AuditLogger::new(backend);
	///
	/// let filter = EventFilter {
	///     event_type: Some(EventType::ConfigUpdate),
	///     ..Default::default()
	/// };
	///
	/// let events = logger.get_events(Some(filter)).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn get_events(&self, filter: Option<EventFilter>) -> Result<Vec<AuditEvent>, String> {
		self.backend.get_events(filter).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use backends::MemoryAuditBackend;
	use serde_json::json;

	#[test]
	fn test_event_type_as_str() {
		assert_eq!(EventType::ConfigUpdate.as_str(), "config_update");
		assert_eq!(EventType::ConfigDelete.as_str(), "config_delete");
		assert_eq!(EventType::ConfigCreate.as_str(), "config_create");
		assert_eq!(EventType::SecretAccess.as_str(), "secret_access");
		assert_eq!(EventType::SecretRotation.as_str(), "secret_rotation");
	}

	#[test]
	fn test_change_record_creation() {
		let record = ChangeRecord {
			old_value: Some(json!(false)),
			new_value: Some(json!(true)),
		};

		assert_eq!(record.old_value, Some(json!(false)));
		assert_eq!(record.new_value, Some(json!(true)));
	}

	#[test]
	fn test_audit_event_creation() {
		let mut changes = HashMap::new();
		changes.insert(
			"test_key".to_string(),
			ChangeRecord {
				old_value: Some(json!("old")),
				new_value: Some(json!("new")),
			},
		);

		let event = AuditEvent::new(
			EventType::ConfigUpdate,
			Some("test_user".to_string()),
			changes.clone(),
		);

		assert_eq!(event.event_type, EventType::ConfigUpdate);
		assert_eq!(event.user, Some("test_user".to_string()));
		assert_eq!(event.changes.len(), 1);
		assert!(event.timestamp <= Utc::now());
	}

	#[tokio::test]
	async fn test_audit_logger_log_event() {
		let backend = Arc::new(MemoryAuditBackend::new());
		let logger = AuditLogger::new(backend.clone());

		let mut changes = HashMap::new();
		changes.insert(
			"setting".to_string(),
			ChangeRecord {
				old_value: None,
				new_value: Some(json!("value")),
			},
		);

		let event = AuditEvent::new(EventType::ConfigCreate, Some("user".to_string()), changes);

		let result = logger.log_event(event).await;
		assert!(result.is_ok());

		let events = logger.get_events(None).await.unwrap();
		assert_eq!(events.len(), 1);
	}

	#[tokio::test]
	async fn test_audit_logger_with_filter() {
		let backend = Arc::new(MemoryAuditBackend::new());
		let logger = AuditLogger::new(backend);

		// Log multiple events
		for i in 0..3 {
			let mut changes = HashMap::new();
			changes.insert(
				format!("key_{}", i),
				ChangeRecord {
					old_value: None,
					new_value: Some(json!(i)),
				},
			);

			let event = AuditEvent::new(EventType::ConfigCreate, Some("user".to_string()), changes);
			logger.log_event(event).await.unwrap();
		}

		let filter = EventFilter {
			event_type: Some(EventType::ConfigCreate),
			user: Some("user".to_string()),
			..Default::default()
		};

		let events = logger.get_events(Some(filter)).await.unwrap();
		assert_eq!(events.len(), 3);
	}
}
