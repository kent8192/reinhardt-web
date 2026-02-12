//! IPC bridge runtime for Rust <-> JavaScript communication.

use std::collections::HashMap;
use std::sync::Arc;

use crate::{IpcRequest, IpcResponse, MobileResult};

/// IPC request handler function type.
pub(super) type IpcHandler =
	Arc<dyn Fn(IpcRequest) -> MobileResult<serde_json::Value> + Send + Sync>;

/// IPC bridge for Rust <-> JavaScript communication.
pub struct IpcBridge {
	handlers: HashMap<String, IpcHandler>,
}

impl IpcBridge {
	/// Creates a new IPC bridge.
	pub fn new() -> Self {
		Self {
			handlers: HashMap::new(),
		}
	}

	/// Registers a command handler.
	pub fn register<F>(&mut self, command: &str, handler: F)
	where
		F: Fn(IpcRequest) -> MobileResult<serde_json::Value> + Send + Sync + 'static,
	{
		self.handlers.insert(command.to_string(), Arc::new(handler));
	}

	/// Handles an incoming IPC request.
	pub fn handle(&self, request: IpcRequest) -> IpcResponse {
		let request_id = request.request_id.clone();

		match self.handlers.get(&request.command) {
			Some(handler) => match handler(request) {
				Ok(data) => IpcResponse {
					request_id,
					success: true,
					data: Some(data),
					error: None,
				},
				Err(e) => IpcResponse {
					request_id,
					success: false,
					data: None,
					error: Some(e.to_string()),
				},
			},
			None => IpcResponse {
				request_id,
				success: false,
				data: None,
				error: Some(format!("Unknown command: {}", request.command)),
			},
		}
	}

	/// Handles a raw JSON message from JavaScript.
	pub fn handle_message(&self, message: &str) -> String {
		match serde_json::from_str::<IpcRequest>(message) {
			Ok(request) => {
				let response = self.handle(request);
				serde_json::to_string(&response).unwrap_or_else(|_| {
					r#"{"success":false,"error":"Serialization error"}"#.to_string()
				})
			}
			Err(e) => {
				let response = IpcResponse {
					request_id: None,
					success: false,
					data: None,
					error: Some(format!("Invalid request: {}", e)),
				};
				serde_json::to_string(&response).unwrap_or_else(|_| {
					r#"{"success":false,"error":"Serialization error"}"#.to_string()
				})
			}
		}
	}

	/// Returns whether a command is registered.
	pub fn has_command(&self, command: &str) -> bool {
		self.handlers.contains_key(command)
	}

	/// Returns the list of registered commands.
	pub fn commands(&self) -> Vec<&str> {
		self.handlers.keys().map(|s| s.as_str()).collect()
	}
}

impl Default for IpcBridge {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_ipc_bridge() {
		let mut bridge = IpcBridge::new();

		bridge.register("greet", |req| {
			let name = req
				.payload
				.get("name")
				.and_then(|v| v.as_str())
				.unwrap_or("World");
			Ok(serde_json::json!({ "message": format!("Hello, {}!", name) }))
		});

		let request = IpcRequest {
			command: "greet".to_string(),
			payload: serde_json::json!({ "name": "Rust" }),
			request_id: Some("1".to_string()),
		};

		let response = bridge.handle(request);
		assert!(response.success);
		assert!(response.data.is_some());
	}

	#[test]
	fn test_unknown_command() {
		let bridge = IpcBridge::new();

		let request = IpcRequest {
			command: "unknown".to_string(),
			payload: serde_json::json!({}),
			request_id: None,
		};

		let response = bridge.handle(request);
		assert!(!response.success);
		assert!(response.error.is_some());
	}
}
