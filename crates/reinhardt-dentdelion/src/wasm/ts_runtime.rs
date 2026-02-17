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
//! # Features
//!
//! - Pure Rust ECMAScript engine (no V8, no native dependencies)
//! - Preact bundled locally for SSR
//! - Thread-safe with internal locking
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

use boa_engine::{Context, JsError, JsValue, Source};
use parking_lot::Mutex;

/// Preact core library (minified)
const PREACT_CORE: &str = include_str!("js/preact.min.js");

/// Preact render-to-string library (minified)
const PREACT_RENDER_TO_STRING: &str = include_str!("js/preact-render-to-string.min.js");

/// JavaScript runtime for SSR.
///
/// This runtime provides a sandboxed JavaScript execution environment
/// using boa_engine (pure Rust ECMAScript implementation).
///
/// # Thread Safety
///
/// The runtime is wrapped in a `Mutex` to ensure thread-safe access.
pub struct TsRuntime {
	/// boa_engine context
	context: Mutex<Context>,
	/// Whether Preact has been initialized
	preact_initialized: bool,
}

impl TsRuntime {
	/// Create a new JavaScript runtime with Preact pre-loaded.
	///
	/// This initializes the boa_engine and loads:
	/// - Preact core library (`h`, `Fragment`)
	/// - Preact render-to-string (`renderToString`)
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - Runtime creation fails
	/// - Preact initialization fails
	pub fn new() -> Result<Self, TsError> {
		let context = Context::default();

		let mut instance = Self {
			context: Mutex::new(context),
			preact_initialized: false,
		};

		// Initialize Preact
		instance.init_preact()?;

		Ok(instance)
	}

	/// Initialize Preact libraries in the context.
	fn init_preact(&mut self) -> Result<(), TsError> {
		let mut context = self.context.lock();

		// Load Preact core
		context.eval(Source::from_bytes(PREACT_CORE)).map_err(|e| {
			TsError::InitFailed(format!(
				"Failed to load Preact core: {}",
				js_error_to_string(&e, &mut context)
			))
		})?;

		// Load Preact render-to-string
		context
			.eval(Source::from_bytes(PREACT_RENDER_TO_STRING))
			.map_err(|e| {
				TsError::InitFailed(format!(
					"Failed to load Preact render-to-string: {}",
					js_error_to_string(&e, &mut context)
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
					js_error_to_string(&e, &mut context)
				))
			})?;

		drop(context);
		self.preact_initialized = true;
		Ok(())
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
		let mut context = self.context.lock();
		let result = context
			.eval(Source::from_bytes(code))
			.map_err(|e| TsError::EvalFailed(js_error_to_string(&e, &mut context)))?;

		js_value_to_string(&result, &mut context)
	}

	/// Evaluate JavaScript code without expecting a return value.
	///
	/// Useful for executing side-effect code like defining functions.
	pub fn eval_void(&self, code: &str) -> Result<(), TsError> {
		let mut context = self.context.lock();
		context
			.eval(Source::from_bytes(code))
			.map_err(|e| TsError::EvalFailed(js_error_to_string(&e, &mut context)))?;
		Ok(())
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

		// Create render script with component and props
		let render_script = format!(
			r#"
			(function() {{
				{component_code}
				var props = {props_json};
				return renderToString(h(Component, props));
			}})()
			"#
		);

		let mut context = self.context.lock();
		let result = context
			.eval(Source::from_bytes(&render_script))
			.map_err(|e| TsError::RenderFailed(js_error_to_string(&e, &mut context)))?;

		js_value_to_string(&result, &mut context)
	}

	/// Check if Preact is initialized and ready for SSR.
	pub fn is_preact_ready(&self) -> bool {
		self.preact_initialized
	}
}

/// Convert JsValue to String
fn js_value_to_string(value: &JsValue, context: &mut Context) -> Result<String, TsError> {
	value
		.to_string(context)
		.map(|s| s.to_std_string_escaped())
		.map_err(|e| TsError::EvalFailed(js_error_to_string(&e, context)))
}

/// Convert JsError to String
fn js_error_to_string(error: &JsError, context: &mut Context) -> String {
	error
		.to_opaque(context)
		.to_string(context)
		.map(|s| s.to_std_string_escaped())
		.unwrap_or_else(|_| "Unknown JavaScript error".to_string())
}

// Safety: TsRuntime uses internal Mutex for thread-safe access
unsafe impl Send for TsRuntime {}
unsafe impl Sync for TsRuntime {}

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
	use rstest::rstest;

	#[rstest]
	fn test_ts_runtime_creation() {
		let runtime = TsRuntime::new();
		assert!(runtime.is_ok());
		assert!(runtime.unwrap().is_preact_ready());
	}

	#[rstest]
	fn test_eval_simple_expression() {
		let runtime = TsRuntime::new().unwrap();
		let result = runtime.eval("(1 + 2).toString()").unwrap();
		assert_eq!(result, "3");
	}

	#[rstest]
	fn test_eval_string_expression() {
		let runtime = TsRuntime::new().unwrap();
		let result = runtime.eval("'Hello, ' + 'World'").unwrap();
		assert_eq!(result, "Hello, World");
	}

	#[rstest]
	fn test_eval_void() {
		let runtime = TsRuntime::new().unwrap();
		runtime.eval_void("var x = 42;").unwrap();
	}

	#[rstest]
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

	#[rstest]
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

	#[rstest]
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
}
