//! IPC (Inter-Process Communication) bridge between Rust and JavaScript.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::Result;

/// A message received from JavaScript via IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcMessage {
	/// The command/action name.
	pub command: String,
	/// The message payload.
	pub payload: serde_json::Value,
	/// Optional request ID for correlating responses.
	#[serde(default)]
	pub request_id: Option<String>,
}

/// A response to send back to JavaScript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
	/// Whether the operation succeeded.
	pub success: bool,
	/// The response data (if successful).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub data: Option<serde_json::Value>,
	/// Error message (if failed).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<String>,
	/// The request ID this response is for.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub request_id: Option<String>,
}

impl IpcResponse {
	/// Creates a successful response with data.
	pub fn success(data: impl Serialize) -> Self {
		Self {
			success: true,
			data: Some(serde_json::to_value(data).unwrap_or(serde_json::Value::Null)),
			error: None,
			request_id: None,
		}
	}

	/// Creates a successful response without data.
	pub fn ok() -> Self {
		Self {
			success: true,
			data: None,
			error: None,
			request_id: None,
		}
	}

	/// Creates an error response.
	pub fn error(message: impl Into<String>) -> Self {
		Self {
			success: false,
			data: None,
			error: Some(message.into()),
			request_id: None,
		}
	}

	/// Sets the request ID for this response.
	pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
		self.request_id = Some(id.into());
		self
	}
}

/// Handler function type for IPC commands.
pub(crate) type CommandHandler = Arc<dyn Fn(IpcMessage) -> Result<IpcResponse> + Send + Sync>;

/// Manages IPC handlers for different commands.
#[derive(Default)]
pub struct IpcHandler {
	handlers: HashMap<String, CommandHandler>,
}

impl IpcHandler {
	/// Creates a new IPC handler.
	pub fn new() -> Self {
		Self::default()
	}

	/// Registers a handler for a command.
	pub fn register<F>(&mut self, command: impl Into<String>, handler: F)
	where
		F: Fn(IpcMessage) -> Result<IpcResponse> + Send + Sync + 'static,
	{
		self.handlers.insert(command.into(), Arc::new(handler));
	}

	/// Handles an incoming IPC message.
	pub fn handle(&self, message: IpcMessage) -> IpcResponse {
		let request_id = message.request_id.clone();

		let response = if let Some(handler) = self.handlers.get(&message.command) {
			match handler(message) {
				Ok(resp) => resp,
				Err(e) => IpcResponse::error(e.to_string()),
			}
		} else {
			IpcResponse::error(format!("unknown command: {}", message.command))
		};

		if let Some(id) = request_id {
			response.with_request_id(id)
		} else {
			response
		}
	}

	/// Parses a raw JSON string into an IpcMessage and handles it.
	pub fn handle_raw(&self, raw: &str) -> String {
		match serde_json::from_str::<IpcMessage>(raw) {
			Ok(message) => {
				let response = self.handle(message);
				serde_json::to_string(&response).unwrap_or_else(|_| {
					r#"{"success":false,"error":"failed to serialize response"}"#.to_string()
				})
			}
			Err(e) => {
				let response = IpcResponse::error(format!("invalid message format: {}", e));
				serde_json::to_string(&response).unwrap_or_else(|_| {
					r#"{"success":false,"error":"failed to serialize response"}"#.to_string()
				})
			}
		}
	}
}

/// JavaScript code to inject for IPC support.
pub(crate) const IPC_INIT_SCRIPT: &str = r#"
(function() {
    window.__reinhardt_ipc = {
        _requestId: 0,
        _pending: new Map(),

        invoke: function(command, payload) {
            return new Promise((resolve, reject) => {
                const requestId = String(++this._requestId);
                this._pending.set(requestId, { resolve, reject });

                const message = JSON.stringify({
                    command: command,
                    payload: payload || {},
                    request_id: requestId
                });

                window.ipc.postMessage(message);
            });
        },

        _handleResponse: function(response) {
            const data = typeof response === 'string' ? JSON.parse(response) : response;
            const pending = this._pending.get(data.request_id);
            if (pending) {
                this._pending.delete(data.request_id);
                if (data.success) {
                    pending.resolve(data.data);
                } else {
                    pending.reject(new Error(data.error || 'Unknown error'));
                }
            }
        }
    };

    // Expose a simplified API
    window.reinhardt = {
        invoke: (cmd, payload) => window.__reinhardt_ipc.invoke(cmd, payload)
    };
})();
"#;
