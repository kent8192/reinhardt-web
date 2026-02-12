//! Reactive system integration for mobile applications.
//!
//! Provides reactive primitives integration with the mobile WebView runtime.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Reactive runtime for managing state synchronization.
pub struct ReactiveRuntime {
	/// State values
	state: Arc<RwLock<HashMap<String, serde_json::Value>>>,
	/// Pending updates to sync to WebView
	pending_updates: Arc<RwLock<Vec<StateUpdate>>>,
}

/// A state update to be synced to WebView.
#[derive(Debug, Clone)]
pub struct StateUpdate {
	/// State key
	pub key: String,
	/// New value
	pub value: serde_json::Value,
}

impl ReactiveRuntime {
	/// Creates a new reactive runtime.
	pub fn new() -> Self {
		Self {
			state: Arc::new(RwLock::new(HashMap::new())),
			pending_updates: Arc::new(RwLock::new(Vec::new())),
		}
	}

	/// Gets a state value.
	pub fn get(&self, key: &str) -> Option<serde_json::Value> {
		self.state.read().ok()?.get(key).cloned()
	}

	/// Sets a state value and queues an update.
	pub fn set(&self, key: impl Into<String>, value: serde_json::Value) {
		let key = key.into();
		if let Ok(mut state) = self.state.write() {
			state.insert(key.clone(), value.clone());
		}
		if let Ok(mut updates) = self.pending_updates.write() {
			updates.push(StateUpdate { key, value });
		}
	}

	/// Updates a state value using a function.
	pub fn update<F>(&self, key: &str, f: F)
	where
		F: FnOnce(Option<&serde_json::Value>) -> serde_json::Value,
	{
		if let Ok(mut state) = self.state.write() {
			let new_value = f(state.get(key));
			state.insert(key.to_string(), new_value.clone());
			if let Ok(mut updates) = self.pending_updates.write() {
				updates.push(StateUpdate {
					key: key.to_string(),
					value: new_value,
				});
			}
		}
	}

	/// Takes all pending updates.
	pub fn take_pending_updates(&self) -> Vec<StateUpdate> {
		self.pending_updates
			.write()
			.map(|mut updates| std::mem::take(&mut *updates))
			.unwrap_or_default()
	}

	/// Generates JavaScript code to apply pending updates.
	pub fn generate_sync_script(&self) -> String {
		let updates = self.take_pending_updates();
		if updates.is_empty() {
			return String::new();
		}

		let mut script = String::from("(function() {\n");
		for update in updates {
			let json_value = serde_json::to_string(&update.value).unwrap_or_default();
			script.push_str(&format!(
				"  window.__REINHARDT_STATE__['{}'] = {};\n",
				update.key, json_value
			));
		}
		script.push_str("  window.__REINHARDT_STATE__.dispatchUpdates();\n");
		script.push_str("})();");
		script
	}

	/// Clears all state.
	pub fn clear(&self) {
		if let Ok(mut state) = self.state.write() {
			state.clear();
		}
		if let Ok(mut updates) = self.pending_updates.write() {
			updates.clear();
		}
	}
}

impl Default for ReactiveRuntime {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_state_management() {
		let runtime = ReactiveRuntime::new();

		runtime.set("counter", serde_json::json!(0));
		assert_eq!(runtime.get("counter"), Some(serde_json::json!(0)));

		runtime.update("counter", |v| {
			let current = v.and_then(|v| v.as_i64()).unwrap_or(0);
			serde_json::json!(current + 1)
		});
		assert_eq!(runtime.get("counter"), Some(serde_json::json!(1)));
	}

	#[test]
	fn test_pending_updates() {
		let runtime = ReactiveRuntime::new();

		runtime.set("a", serde_json::json!(1));
		runtime.set("b", serde_json::json!(2));

		let updates = runtime.take_pending_updates();
		assert_eq!(updates.len(), 2);

		// Updates should be cleared
		let updates = runtime.take_pending_updates();
		assert!(updates.is_empty());
	}
}
