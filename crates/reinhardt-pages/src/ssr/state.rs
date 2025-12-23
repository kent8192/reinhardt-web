//! SSR State serialization for hydration.
//!
//! This module handles serialization of reactive state during SSR
//! so it can be restored during client-side hydration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The global JavaScript variable name for SSR state.
pub(super) const SSR_STATE_VAR: &str = "__REINHARDT_SSR_STATE__";

/// Represents the serialized SSR state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SsrState {
	/// Signal values indexed by their hydration ID.
	signals: HashMap<String, serde_json::Value>,
	/// Component props indexed by their hydration ID.
	props: HashMap<String, serde_json::Value>,
	/// Additional metadata.
	metadata: HashMap<String, serde_json::Value>,
}

impl SsrState {
	/// Creates a new empty SSR state.
	pub fn new() -> Self {
		Self::default()
	}

	/// Adds a signal value to the state.
	pub fn add_signal(&mut self, id: impl Into<String>, value: impl Serialize) {
		if let Ok(json) = serde_json::to_value(value) {
			self.signals.insert(id.into(), json);
		}
	}

	/// Adds component props to the state.
	pub fn add_props(&mut self, id: impl Into<String>, props: impl Serialize) {
		if let Ok(json) = serde_json::to_value(props) {
			self.props.insert(id.into(), json);
		}
	}

	/// Adds metadata to the state.
	pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Serialize) {
		if let Ok(json) = serde_json::to_value(value) {
			self.metadata.insert(key.into(), json);
		}
	}

	/// Gets a signal value by ID.
	pub fn get_signal(&self, id: &str) -> Option<&serde_json::Value> {
		self.signals.get(id)
	}

	/// Gets component props by ID.
	pub fn get_props(&self, id: &str) -> Option<&serde_json::Value> {
		self.props.get(id)
	}

	/// Gets metadata by key.
	pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
		self.metadata.get(key)
	}

	/// Returns the number of signals.
	pub fn signal_count(&self) -> usize {
		self.signals.len()
	}

	/// Returns the number of props entries.
	pub fn props_count(&self) -> usize {
		self.props.len()
	}

	/// Checks if the state is empty.
	pub fn is_empty(&self) -> bool {
		self.signals.is_empty() && self.props.is_empty() && self.metadata.is_empty()
	}

	/// Serializes the state to JSON.
	pub fn to_json(&self) -> Result<String, serde_json::Error> {
		serde_json::to_string(self)
	}

	/// Serializes the state to pretty-printed JSON.
	pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
		serde_json::to_string_pretty(self)
	}

	/// Generates a `<script>` tag containing the serialized state.
	pub fn to_script_tag(&self) -> String {
		let json = self.to_json().unwrap_or_else(|_| "{}".to_string());
		format!(
			r#"<script id="ssr-state" type="application/json">window.{} = {};</script>"#,
			SSR_STATE_VAR, json
		)
	}

	/// Deserializes state from JSON.
	pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
		serde_json::from_str(json)
	}

	/// Merges another state into this one.
	pub fn merge(&mut self, other: SsrState) {
		self.signals.extend(other.signals);
		self.props.extend(other.props);
		self.metadata.extend(other.metadata);
	}
}

/// A single entry in the SSR state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateEntry {
	/// The hydration ID this entry belongs to.
	pub id: String,
	/// The type of entry.
	pub entry_type: StateEntryType,
	/// The serialized value.
	pub value: serde_json::Value,
}

/// The type of a state entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateEntryType {
	/// A reactive signal value.
	Signal,
	/// Component props.
	Props,
	/// Generic metadata.
	Metadata,
}

impl StateEntry {
	/// Creates a new signal entry.
	pub fn signal(id: impl Into<String>, value: impl Serialize) -> Option<Self> {
		serde_json::to_value(value).ok().map(|v| Self {
			id: id.into(),
			entry_type: StateEntryType::Signal,
			value: v,
		})
	}

	/// Creates a new props entry.
	pub fn props(id: impl Into<String>, value: impl Serialize) -> Option<Self> {
		serde_json::to_value(value).ok().map(|v| Self {
			id: id.into(),
			entry_type: StateEntryType::Props,
			value: v,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_ssr_state_new() {
		let state = SsrState::new();
		assert!(state.is_empty());
	}

	#[test]
	fn test_ssr_state_add_signal() {
		let mut state = SsrState::new();
		state.add_signal("counter", 42);
		assert_eq!(state.signal_count(), 1);
		assert_eq!(state.get_signal("counter"), Some(&serde_json::json!(42)));
	}

	#[test]
	fn test_ssr_state_add_props() {
		let mut state = SsrState::new();
		state.add_props("rh-0", serde_json::json!({"name": "test"}));
		assert_eq!(state.props_count(), 1);
	}

	#[test]
	fn test_ssr_state_add_metadata() {
		let mut state = SsrState::new();
		state.add_metadata("version", "1.0.0");
		assert_eq!(
			state.get_metadata("version"),
			Some(&serde_json::json!("1.0.0"))
		);
	}

	#[test]
	fn test_ssr_state_to_json() {
		let mut state = SsrState::new();
		state.add_signal("count", 10);
		let json = state.to_json().unwrap();
		assert!(json.contains("\"signals\""));
		assert!(json.contains("\"count\""));
	}

	#[test]
	fn test_ssr_state_from_json() {
		let json = r#"{"signals":{"x":5},"props":{},"metadata":{}}"#;
		let state = SsrState::from_json(json).unwrap();
		assert_eq!(state.get_signal("x"), Some(&serde_json::json!(5)));
	}

	#[test]
	fn test_ssr_state_to_script_tag() {
		let mut state = SsrState::new();
		state.add_signal("test", true);
		let script = state.to_script_tag();
		assert!(script.starts_with("<script"));
		assert!(script.contains("__REINHARDT_SSR_STATE__"));
		assert!(script.contains("</script>"));
	}

	#[test]
	fn test_ssr_state_merge() {
		let mut state1 = SsrState::new();
		state1.add_signal("a", 1);

		let mut state2 = SsrState::new();
		state2.add_signal("b", 2);

		state1.merge(state2);
		assert_eq!(state1.signal_count(), 2);
	}

	#[test]
	fn test_state_entry_signal() {
		let entry = StateEntry::signal("rh-1", 42).unwrap();
		assert_eq!(entry.id, "rh-1");
		assert!(matches!(entry.entry_type, StateEntryType::Signal));
	}

	#[test]
	fn test_state_entry_props() {
		let entry = StateEntry::props("rh-2", serde_json::json!({"x": 1})).unwrap();
		assert_eq!(entry.id, "rh-2");
		assert!(matches!(entry.entry_type, StateEntryType::Props));
	}
}
