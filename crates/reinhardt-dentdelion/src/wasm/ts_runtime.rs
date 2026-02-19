//! JavaScript Runtime for SSR
//!
//! This module provides a pure Rust JavaScript runtime using boa_engine,
//! enabling React/Preact SSR without external runtime dependencies.
//!
//! # Architecture
//!
//! The JavaScript runtime runs on the **host side** (not inside WASM plugins).
//! WASM plugins request SSR through host functions, and the host executes
//! JavaScript using this runtime.
//!
//! # Thread Safety
//!
//! `boa_engine::Context` is intentionally `!Send + !Sync` because it may rely
//! on thread-local state. To provide safe multi-threaded access, `TsRuntime`
//! uses a dedicated thread that owns the `Context` and communicates via
//! `std::sync::mpsc` channels. This makes `TsRuntime` naturally `Send + Sync`
//! without requiring `unsafe impl`.
//!
//! # Features
//!
//! - Pure Rust ECMAScript engine (no V8, no native dependencies)
//! - Preact bundled locally for SSR
//! - Thread-safe via dedicated runtime thread and channel communication
//!
//! # Example
//!
//! ```ignore
//! use reinhardt_dentdelion::wasm::TsRuntime;
//!
//! let runtime = TsRuntime::new()?;
//!
//! // Evaluate simple JavaScript
//! let result: String = runtime.eval("(1 + 2).toString()")?;
//! assert_eq!(result, "3");
//!
//! // Render a Preact component
//! let component_code = r#"
//!     function Component(props) {
//!         return h('div', null, 'Hello, ' + props.name);
//!     }
//! "#;
//! let html = runtime.render_component(component_code, r#"{"name": "World"}"#)?;
//! assert!(html.contains("Hello, World"));
//! ```

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread;

use boa_engine::{Context, JsError, JsValue, Source};

/// Preact core library (minified)
const PREACT_CORE: &str = include_str!("js/preact.min.js");

/// Preact render-to-string library (minified)
const PREACT_RENDER_TO_STRING: &str = include_str!("js/preact-render-to-string.min.js");

/// Command sent to the dedicated JavaScript runtime thread.
///
/// Each variant carries a response channel for returning the result
/// back to the calling thread.
enum RuntimeCommand {
	/// Evaluate JavaScript code and return the result as a string.
	Eval {
		code: String,
		response_tx: mpsc::Sender<Result<String, TsError>>,
	},
	/// Evaluate JavaScript code without expecting a return value.
	EvalVoid {
		code: String,
		response_tx: mpsc::Sender<Result<(), TsError>>,
	},
	/// Render a Preact component to HTML string.
	RenderComponent {
		component_code: String,
		props_json: String,
		response_tx: mpsc::Sender<Result<String, TsError>>,
	},
}

/// JavaScript runtime for SSR.
///
/// This runtime provides a sandboxed JavaScript execution environment
/// using boa_engine (pure Rust ECMAScript implementation).
///
/// # Thread Safety
///
/// `boa_engine::Context` is `!Send + !Sync` by design. Instead of using
/// `unsafe impl Send/Sync`, this struct uses a dedicated thread that owns
/// the `Context` and communicates via `std::sync::mpsc::SyncSender`.
/// `SyncSender<T>` is `Send + Sync` when `T: Send`, making `TsRuntime`
/// safely shareable across threads without `unsafe`.
pub struct TsRuntime {
	/// Channel to send commands to the dedicated runtime thread.
	///
	/// `SyncSender` is both `Send` and `Sync` (when `T: Send`),
	/// which makes `TsRuntime` naturally `Send + Sync`.
	command_tx: SyncSender<RuntimeCommand>,
	/// Whether Preact has been initialized (set once during construction)
	preact_initialized: bool,
}

impl TsRuntime {
	/// Create a new JavaScript runtime with Preact pre-loaded.
	///
	/// This spawns a dedicated thread that owns the `boa_engine::Context`
	/// and initializes Preact libraries. The `Context` never leaves this
	/// thread, ensuring safe single-threaded access.
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - Runtime thread creation fails
	/// - Preact initialization fails
	pub fn new() -> Result<Self, TsError> {
		// Bounded channel for command passing (backpressure at 16 pending commands)
		let (command_tx, command_rx) = mpsc::sync_channel::<RuntimeCommand>(16);

		// Oneshot channel for initialization result
		let (init_tx, init_rx) = mpsc::channel::<Result<(), TsError>>();

		// Spawn dedicated thread for boa_engine::Context.
		// Context is !Send + !Sync, so it must stay on this single thread.
		thread::spawn(move || {
			runtime_thread_main(command_rx, init_tx);
		});

		// Wait for initialization result from the dedicated thread
		init_rx.recv().map_err(|_| {
			TsError::InitFailed("Runtime thread terminated during initialization".to_string())
		})??;

		Ok(Self {
			command_tx,
			preact_initialized: true,
		})
	}

	/// Evaluate JavaScript code and return the result as a string.
	///
	/// # Arguments
	///
	/// * `code` - JavaScript code to evaluate
	///
	/// # Example
	///
	/// ```ignore
	/// let result = runtime.eval("(1 + 2).toString()")?;
	/// assert_eq!(result, "3");
	/// ```
	pub fn eval(&self, code: &str) -> Result<String, TsError> {
		let (response_tx, response_rx) = mpsc::channel();
		self.command_tx
			.send(RuntimeCommand::Eval {
				code: code.to_string(),
				response_tx,
			})
			.map_err(|_| TsError::EvalFailed("Runtime thread is not available".to_string()))?;

		response_rx.recv().map_err(|_| {
			TsError::EvalFailed("Runtime thread terminated during evaluation".to_string())
		})?
	}

	/// Evaluate JavaScript code without expecting a return value.
	///
	/// Useful for executing side-effect code like defining functions.
	pub fn eval_void(&self, code: &str) -> Result<(), TsError> {
		let (response_tx, response_rx) = mpsc::channel();
		self.command_tx
			.send(RuntimeCommand::EvalVoid {
				code: code.to_string(),
				response_tx,
			})
			.map_err(|_| TsError::EvalFailed("Runtime thread is not available".to_string()))?;

		response_rx.recv().map_err(|_| {
			TsError::EvalFailed("Runtime thread terminated during evaluation".to_string())
		})?
	}

	/// Render a Preact/React component to HTML string.
	///
	/// # Arguments
	///
	/// * `component_code` - JavaScript code defining the component.
	///   Must define a function named `Component`.
	/// * `props_json` - JSON string representing component props
	///
	/// # Component Code Format
	///
	/// The component code should define a `Component` function:
	///
	/// ```javascript
	/// function Component(props) {
	///     return h('div', { class: 'container' },
	///         h('h1', null, props.title),
	///         h('p', null, props.content)
	///     );
	/// }
	/// ```
	///
	/// # Example
	///
	/// ```ignore
	/// let component = r#"
	///     function Component(props) {
	///         return h('div', null, 'Hello, ' + props.name);
	///     }
	/// "#;
	/// let html = runtime.render_component(component, r#"{"name": "World"}"#)?;
	/// assert!(html.contains("Hello, World"));
	/// ```
	pub fn render_component(
		&self,
		component_code: &str,
		props_json: &str,
	) -> Result<String, TsError> {
		if !self.preact_initialized {
			return Err(TsError::InitFailed("Preact not initialized".to_string()));
		}

		let (response_tx, response_rx) = mpsc::channel();
		self.command_tx
			.send(RuntimeCommand::RenderComponent {
				component_code: component_code.to_string(),
				props_json: props_json.to_string(),
				response_tx,
			})
			.map_err(|_| TsError::RenderFailed("Runtime thread is not available".to_string()))?;

		response_rx.recv().map_err(|_| {
			TsError::RenderFailed("Runtime thread terminated during rendering".to_string())
		})?
	}

	/// Check if Preact is initialized and ready for SSR.
	pub fn is_preact_ready(&self) -> bool {
		self.preact_initialized
	}
}

/// Main function for the dedicated JavaScript runtime thread.
///
/// This thread owns the `boa_engine::Context` (which is `!Send + !Sync`)
/// and processes commands received via channel. The context never leaves
/// this thread, ensuring safe single-threaded access without `unsafe`.
fn runtime_thread_main(
	command_rx: Receiver<RuntimeCommand>,
	init_tx: mpsc::Sender<Result<(), TsError>>,
) {
	let mut context = Context::default();

	// Initialize Preact libraries
	let init_result = init_preact(&mut context);
	let init_ok = init_result.is_ok();

	// Send initialization result back to the caller
	if init_tx.send(init_result).is_err() {
		// Caller dropped the receiver; exit the thread
		return;
	}

	if !init_ok {
		// Initialization failed; exit the thread
		return;
	}

	// Process commands until the channel is closed (all senders dropped)
	while let Ok(command) = command_rx.recv() {
		match command {
			RuntimeCommand::Eval { code, response_tx } => {
				let result = eval_code(&code, &mut context);
				let _ = response_tx.send(result);
			}
			RuntimeCommand::EvalVoid { code, response_tx } => {
				let result = eval_void_code(&code, &mut context);
				let _ = response_tx.send(result);
			}
			RuntimeCommand::RenderComponent {
				component_code,
				props_json,
				response_tx,
			} => {
				let result = render_component_code(&component_code, &props_json, &mut context);
				let _ = response_tx.send(result);
			}
		}
	}
}

/// Initialize Preact libraries in the JavaScript context.
fn init_preact(context: &mut Context) -> Result<(), TsError> {
	// Load Preact core
	context.eval(Source::from_bytes(PREACT_CORE)).map_err(|e| {
		TsError::InitFailed(format!(
			"Failed to load Preact core: {}",
			js_error_to_string(&e, context)
		))
	})?;

	// Load Preact render-to-string
	context
		.eval(Source::from_bytes(PREACT_RENDER_TO_STRING))
		.map_err(|e| {
			TsError::InitFailed(format!(
				"Failed to load Preact render-to-string: {}",
				js_error_to_string(&e, context)
			))
		})?;

	// Create global bindings for convenience
	// This makes `h`, `Fragment`, and `renderToString` available directly in the global scope
	const GLOBAL_BINDINGS: &str = r#"
		var h = preact.h;
		var Fragment = preact.Fragment;
		var createElement = preact.createElement;
		var renderToString = preactRenderToString.renderToString;
		var renderToStaticMarkup = preactRenderToString.renderToStaticMarkup;
	"#;
	context
		.eval(Source::from_bytes(GLOBAL_BINDINGS))
		.map_err(|e| {
			TsError::InitFailed(format!(
				"Failed to create global bindings: {}",
				js_error_to_string(&e, context)
			))
		})?;

	Ok(())
}

/// Evaluate JavaScript code and return the result as a string.
fn eval_code(code: &str, context: &mut Context) -> Result<String, TsError> {
	let result = context
		.eval(Source::from_bytes(code))
		.map_err(|e| TsError::EvalFailed(js_error_to_string(&e, context)))?;

	js_value_to_string(&result, context)
}

/// Evaluate JavaScript code without expecting a return value.
fn eval_void_code(code: &str, context: &mut Context) -> Result<(), TsError> {
	context
		.eval(Source::from_bytes(code))
		.map_err(|e| TsError::EvalFailed(js_error_to_string(&e, context)))?;
	Ok(())
}

/// Render a Preact component to HTML using the JavaScript context.
fn render_component_code(
	component_code: &str,
	props_json: &str,
	context: &mut Context,
) -> Result<String, TsError> {
	let render_script = format!(
		r#"
		(function() {{
			{component_code}
			var props = {props_json};
			return renderToString(h(Component, props));
		}})()
		"#
	);

	let result = context
		.eval(Source::from_bytes(&render_script))
		.map_err(|e| TsError::RenderFailed(js_error_to_string(&e, context)))?;

	js_value_to_string(&result, context)
}

/// Convert `JsValue` to String
fn js_value_to_string(value: &JsValue, context: &mut Context) -> Result<String, TsError> {
	value
		.to_string(context)
		.map(|s| s.to_std_string_escaped())
		.map_err(|e| TsError::EvalFailed(js_error_to_string(&e, context)))
}

/// Convert `JsError` to String
fn js_error_to_string(error: &JsError, context: &mut Context) -> String {
	error
		.to_opaque(context)
		.to_string(context)
		.map(|s| s.to_std_string_escaped())
		.unwrap_or_else(|_| "Unknown JavaScript error".to_string())
}

/// Shared JavaScript runtime instance type.
pub type SharedTsRuntime = Arc<TsRuntime>;

/// JavaScript runtime errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum TsError {
	/// JavaScript runtime initialization failed
	#[error("JavaScript runtime initialization failed: {0}")]
	InitFailed(String),

	/// JavaScript evaluation failed
	#[error("JavaScript evaluation failed: {0}")]
	EvalFailed(String),

	/// Props serialization failed
	#[error("Props serialization failed: {0}")]
	PropsSerialization(String),

	/// Component rendering failed
	#[error("Component rendering failed: {0}")]
	RenderFailed(String),
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_ts_runtime_creation() {
		let runtime = TsRuntime::new();
		assert!(runtime.is_ok());
		assert!(runtime.unwrap().is_preact_ready());
	}

	#[test]
	fn test_eval_simple_expression() {
		let runtime = TsRuntime::new().unwrap();
		let result = runtime.eval("(1 + 2).toString()").unwrap();
		assert_eq!(result, "3");
	}

	#[test]
	fn test_eval_string_expression() {
		let runtime = TsRuntime::new().unwrap();
		let result = runtime.eval("'Hello, ' + 'World'").unwrap();
		assert_eq!(result, "Hello, World");
	}

	#[test]
	fn test_eval_void() {
		let runtime = TsRuntime::new().unwrap();
		runtime.eval_void("var x = 42;").unwrap();
	}

	#[test]
	fn test_render_simple_component() {
		let runtime = TsRuntime::new().unwrap();
		let component_code = r#"
			function Component(props) {
				return h('div', null, 'Hello, ' + props.name);
			}
		"#;
		let props = r#"{"name": "World"}"#;

		let result = runtime.render_component(component_code, props);
		assert!(result.is_ok());
		let html = result.unwrap();
		assert!(html.contains("Hello, World"));
		assert!(html.contains("<div>"));
	}

	#[test]
	fn test_render_nested_component() {
		let runtime = TsRuntime::new().unwrap();
		let component_code = r#"
			function Child(props) {
				return h('span', null, props.text);
			}
			function Component(props) {
				return h('div', { class: 'container' },
					h(Child, { text: props.message })
				);
			}
		"#;
		let props = r#"{"message": "Nested!"}"#;

		let result = runtime.render_component(component_code, props);
		assert!(result.is_ok());
		let html = result.unwrap();
		assert!(html.contains("container"));
		assert!(html.contains("Nested!"));
	}

	#[test]
	fn test_render_with_attributes() {
		let runtime = TsRuntime::new().unwrap();
		let component_code = r#"
			function Component(props) {
				return h('a', { href: props.url, class: 'link' }, props.label);
			}
		"#;
		let props = r#"{"url": "https://example.com", "label": "Click me"}"#;

		let result = runtime.render_component(component_code, props);
		assert!(result.is_ok());
		let html = result.unwrap();
		assert!(html.contains("href="));
		assert!(html.contains("Click me"));
	}

	/// Verify `TsRuntime` is `Send + Sync` at compile time.
	///
	/// This is the core safety property: because the `boa_engine::Context`
	/// is confined to a dedicated thread and `TsRuntime` only holds a
	/// `SyncSender` (which is `Send + Sync`), the struct is naturally
	/// thread-safe without `unsafe impl`.
	#[rstest::rstest]
	fn test_ts_runtime_is_send_and_sync() {
		fn assert_send<T: Send>() {}
		fn assert_sync<T: Sync>() {}
		assert_send::<TsRuntime>();
		assert_sync::<TsRuntime>();
	}

	/// Verify `SharedTsRuntime` (`Arc<TsRuntime>`) can be shared across threads.
	#[rstest::rstest]
	fn test_shared_ts_runtime_across_threads() {
		// Arrange
		let runtime = Arc::new(TsRuntime::new().unwrap());
		let runtime_clone = Arc::clone(&runtime);

		// Act: use from another thread
		let handle = std::thread::spawn(move || runtime_clone.eval("(2 + 3).toString()").unwrap());

		// Assert
		let result = handle.join().unwrap();
		assert_eq!(result, "5");
	}
}
