//! IPC code generation for mobile applications.
//!
//! Generates IPC bridge code for Rust <-> JavaScript communication.

use proc_macro2::TokenStream;
use quote::quote;

/// IPC command definition for code generation.
#[derive(Debug, Clone)]
pub struct IpcCommandDef {
	/// Command name
	pub name: String,
	/// Parameter names
	pub params: Vec<String>,
	/// Return type (as string for code generation)
	pub return_type: Option<String>,
}

/// Generates IPC handler registration code.
pub struct IpcCodeGenerator {
	commands: Vec<IpcCommandDef>,
}

impl IpcCodeGenerator {
	/// Creates a new IPC code generator.
	pub fn new() -> Self {
		Self {
			commands: Vec::new(),
		}
	}

	/// Adds a command definition.
	pub fn add_command(&mut self, command: IpcCommandDef) {
		self.commands.push(command);
	}

	/// Generates the IPC handler registration code.
	pub fn generate_handlers(&self) -> TokenStream {
		let handlers = self.commands.iter().map(|cmd| {
			let name = &cmd.name;
			quote! {
				bridge.register(#name, |request| {
					// TODO: Implement handler dispatch
					Ok(serde_json::json!({}))
				});
			}
		});

		quote! {
			fn register_ipc_handlers(bridge: &mut IpcBridge) {
				#(#handlers)*
			}
		}
	}

	/// Generates JavaScript bindings for commands.
	pub fn generate_js_bindings(&self) -> String {
		let mut js = String::from("// Generated IPC bindings\n");
		js.push_str("window.__REINHARDT_COMMANDS__ = {\n");

		for cmd in &self.commands {
			let params = cmd.params.join(", ");
			js.push_str(&format!("	{}: function({}) {{\n", cmd.name, params));
			js.push_str(&format!(
				"		return window.__REINHARDT_IPC__.invoke('{}', {{ {} }});\n",
				cmd.name,
				cmd.params
					.iter()
					.map(|p| format!("{}: {}", p, p))
					.collect::<Vec<_>>()
					.join(", ")
			));
			js.push_str("	},\n");
		}

		js.push_str("};\n");
		js
	}
}

impl Default for IpcCodeGenerator {
	fn default() -> Self {
		Self::new()
	}
}

/// Generates the base IPC bridge structure.
pub fn generate_ipc_bridge() -> TokenStream {
	quote! {
		use std::collections::HashMap;
		use std::sync::Arc;

		/// IPC request handler function type.
		pub type IpcHandler = Arc<dyn Fn(IpcRequest) -> crate::MobileResult<serde_json::Value> + Send + Sync>;

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
				F: Fn(IpcRequest) -> crate::MobileResult<serde_json::Value> + Send + Sync + 'static,
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
						error: Some(format!("Unknown command")),
					},
				}
			}
		}

		impl Default for IpcBridge {
			fn default() -> Self {
				Self::new()
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_js_bindings_generation() {
		let mut generator = IpcCodeGenerator::new();
		generator.add_command(IpcCommandDef {
			name: "greet".to_string(),
			params: vec!["name".to_string()],
			return_type: Some("String".to_string()),
		});

		let js = generator.generate_js_bindings();
		assert!(js.contains("greet"));
		assert!(js.contains("name"));
	}
}
