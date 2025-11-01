//! Database lifecycle event helpers
//!
//! These helpers provide convenient database lifecycle events similar to SQLAlchemy,
//! integrated with the Django-style signal system.

use crate::core::SignalName;
use crate::registry::get_signal;
use crate::signal::Signal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Generic database event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbEvent {
	pub event_type: String,
	pub table: String,
	pub id: Option<String>,
	pub data: HashMap<String, String>,
}

impl DbEvent {
	pub fn new(event_type: impl Into<String>, table: impl Into<String>) -> Self {
		Self {
			event_type: event_type.into(),
			table: table.into(),
			id: None,
			data: HashMap::new(),
		}
	}

	pub fn with_id(mut self, id: impl Into<String>) -> Self {
		self.id = Some(id.into());
		self
	}

	pub fn with_data(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
		self.data.insert(key.into(), value.into());
		self
	}
}

/// Before insert signal
pub fn before_insert() -> Signal<DbEvent> {
	get_signal::<DbEvent>(SignalName::DB_BEFORE_INSERT)
}

/// After insert signal
pub fn after_insert() -> Signal<DbEvent> {
	get_signal::<DbEvent>(SignalName::DB_AFTER_INSERT)
}

/// Before update signal
pub fn before_update() -> Signal<DbEvent> {
	get_signal::<DbEvent>(SignalName::DB_BEFORE_UPDATE)
}

/// After update signal
pub fn after_update() -> Signal<DbEvent> {
	get_signal::<DbEvent>(SignalName::DB_AFTER_UPDATE)
}

/// Before delete signal
pub fn before_delete() -> Signal<DbEvent> {
	get_signal::<DbEvent>(SignalName::DB_BEFORE_DELETE)
}

/// After delete signal
pub fn after_delete() -> Signal<DbEvent> {
	get_signal::<DbEvent>(SignalName::DB_AFTER_DELETE)
}
