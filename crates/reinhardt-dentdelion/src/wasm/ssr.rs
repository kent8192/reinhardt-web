//! Server-Side Rendering (SSR) Proxy for WASM Plugins
//!
//! This module provides SSR functionality for WASM plugins through the host's
//! JavaScript/TypeScript runtime. It allows plugins to render React/Preact components
//! without requiring a JavaScript engine within the WASM sandbox.
//!
//! # SSR Backend
//!
//! - **TypeScript** (`ts` feature): Full TypeScript runtime using deno_core/rustyscript
//!
//! # Example
//!
//! ```ignore
//! use reinhardt_dentdelion::wasm::ssr::{SsrProxy, RenderOptions};
//! use reinhardt_dentdelion::wasm::TsRuntime;
//! use std::sync::Arc;
//!
//! // Create SSR proxy with TypeScript backend
//! let ts_runtime = Arc::new(TsRuntime::new()?);
//! let proxy = SsrProxy::with_ts_runtime(ts_runtime);
//!
//! // Check if SSR is available
//! if proxy.is_available() {
//!     let component_code = r#"
//!         function Component(props) {
//!             return h('div', null, 'Hello, ' + props.name);
//!         }
//!     "#;
//!     let result = proxy.render_component(
//!         component_code,
//!         r#"{"name": "World"}"#,
//!         RenderOptions::default(),
//!     )?;
//!     println!("Rendered HTML: {}", result.html);
//! }
//! ```

use serde::{Deserialize, Serialize};

#[cfg(feature = "ts")]
use super::ts_runtime::{SharedTsRuntime, TsError};

/// Render options for SSR.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RenderOptions {
	/// Whether to include hydration script for client-side takeover
	pub include_hydration: bool,
	/// Whether to extract CSS from components
	pub extract_css: bool,
	/// Whether to extract meta tags (title, description, etc.)
	pub extract_meta: bool,
}

/// Render result containing the HTML and optional extracted assets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderResult {
	/// Rendered HTML string
	pub html: String,
	/// Extracted CSS (if extract_css was true)
	pub css: Option<String>,
	/// Extracted meta tags as key-value pairs (if extract_meta was true)
	pub meta: Option<Vec<(String, String)>>,
	/// Hydration script to include in the page (if include_hydration was true)
	pub hydration_script: Option<String>,
}

/// SSR Proxy error types.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SsrError {
	/// SSR is not available (no JavaScript runtime enabled)
	#[error("SSR is not available: no JavaScript runtime is enabled")]
	NotAvailable,

	/// Component not found at the specified path
	#[error("Component not found: {0}")]
	ComponentNotFound(String),

	/// Props serialization failed
	#[error("Failed to serialize props: {0}")]
	PropsSerialization(String),

	/// Render execution failed
	#[error("Render failed: {0}")]
	RenderFailed(String),

	/// JavaScript evaluation failed
	#[error("JavaScript evaluation failed: {0}")]
	EvalFailed(String),
}

#[cfg(feature = "ts")]
impl From<TsError> for SsrError {
	fn from(err: TsError) -> Self {
		match err {
			TsError::InitFailed(_msg) => SsrError::NotAvailable,
			TsError::EvalFailed(msg) => SsrError::EvalFailed(msg),
			TsError::PropsSerialization(msg) => SsrError::PropsSerialization(msg),
			TsError::RenderFailed(msg) => SsrError::RenderFailed(msg),
		}
	}
}

/// SSR Proxy for delegating rendering to the host's JavaScript/TypeScript runtime.
///
/// This proxy provides a bridge between WASM plugins and the host's SSR
/// capabilities using the TypeScript runtime:
///
/// - **TypeScript** (`ts` feature): Full runtime with deno_core/rustyscript
///
/// When no JavaScript runtime is available, all render operations return
/// [`SsrError::NotAvailable`].
pub struct SsrProxy {
	/// Whether SSR is available
	available: bool,

	/// TypeScript runtime (when `ts` feature is enabled)
	#[cfg(feature = "ts")]
	ts_runtime: Option<SharedTsRuntime>,
}

impl Default for SsrProxy {
	fn default() -> Self {
		Self::new()
	}
}

impl std::fmt::Debug for SsrProxy {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut debug = f.debug_struct("SsrProxy");
		debug.field("available", &self.available);

		#[cfg(feature = "ts")]
		debug.field("ts_enabled", &self.ts_runtime.is_some());

		debug.finish()
	}
}

impl SsrProxy {
	/// Create a new SSR proxy.
	///
	/// By default, SSR is not available. Use [`with_ts_runtime`](Self::with_ts_runtime)
	/// to create a proxy with TypeScript backend support.
	pub fn new() -> Self {
		Self {
			available: false,
			#[cfg(feature = "ts")]
			ts_runtime: None,
		}
	}

	/// Create an SSR proxy with the specified availability.
	///
	/// This is used by the host to indicate whether a JavaScript runtime
	/// is available for SSR.
	pub fn with_availability(available: bool) -> Self {
		Self {
			available,
			#[cfg(feature = "ts")]
			ts_runtime: None,
		}
	}

	/// Create an SSR proxy with TypeScript backend.
	///
	/// This enables JavaScript/TypeScript SSR using rustyscript (deno_core)
	/// with Preact for React-compatible component rendering.
	///
	/// # Example
	///
	/// ```ignore
	/// use reinhardt_dentdelion::wasm::{TsRuntime, SsrProxy};
	/// use std::sync::Arc;
	///
	/// let ts_runtime = Arc::new(TsRuntime::new()?);
	/// let proxy = SsrProxy::with_ts_runtime(ts_runtime);
	/// assert!(proxy.is_available());
	/// ```
	#[cfg(feature = "ts")]
	pub fn with_ts_runtime(runtime: SharedTsRuntime) -> Self {
		Self {
			available: true,
			ts_runtime: Some(runtime),
		}
	}

	/// Check if SSR is available on the host.
	pub fn is_available(&self) -> bool {
		self.available
	}

	/// Render a component to HTML using inline component code.
	///
	/// This method accepts JavaScript code that defines a `Component` function
	/// and renders it with the provided props.
	///
	/// # Arguments
	///
	/// * `component_code` - JavaScript code defining the component
	/// * `props_json` - JSON string representing component props
	/// * `options` - Rendering options
	///
	/// # Component Code Format
	///
	/// The component code should define a `Component` function using Preact's `h()`:
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
	/// let result = proxy.render_component(
	///     component,
	///     r#"{"name": "World"}"#,
	///     RenderOptions::default(),
	/// )?;
	/// assert!(result.html.contains("Hello, World"));
	/// ```
	pub fn render_component(
		&self,
		component_code: &str,
		props_json: &str,
		options: RenderOptions,
	) -> Result<RenderResult, SsrError> {
		if !self.available {
			return Err(SsrError::NotAvailable);
		}

		#[cfg(feature = "ts")]
		if let Some(ref runtime) = self.ts_runtime {
			let html = runtime.render_component(component_code, props_json)?;

			return Ok(RenderResult {
				html,
				css: None,  // CSS extraction not yet supported
				meta: None, // Meta extraction not yet supported
				hydration_script: if options.include_hydration {
					Some(self.generate_hydration_script(component_code, props_json))
				} else {
					None
				},
			});
		}

		Err(SsrError::NotAvailable)
	}

	/// Render a React component to HTML.
	///
	/// # Arguments
	///
	/// * `component_path` - Path to the component file (relative to plugin assets)
	/// * `props` - MessagePack-serialized component props
	/// * `options` - Rendering options
	///
	/// # Returns
	///
	/// Rendered HTML and optional extracted assets, or an error if rendering fails.
	///
	/// # Note
	///
	/// This method requires loading component code from a file path.
	/// For inline component code, use [`render_component`](Self::render_component).
	pub async fn render_react(
		&self,
		_component_path: &str,
		_props: &[u8],
		_options: RenderOptions,
	) -> Result<RenderResult, SsrError> {
		if !self.available {
			return Err(SsrError::NotAvailable);
		}

		// TODO: Implement file-based component loading
		// 1. Load the component code from component_path
		// 2. Deserialize props from MessagePack
		// 3. Call render_component with the loaded code

		Err(SsrError::ComponentNotFound(
			"File-based component loading not yet implemented".to_string(),
		))
	}

	/// Render a Vue component to HTML.
	///
	/// # Arguments
	///
	/// * `component_path` - Path to the component file (relative to plugin assets)
	/// * `props` - MessagePack-serialized component props
	/// * `options` - Rendering options
	///
	/// # Returns
	///
	/// Rendered HTML and optional extracted assets, or an error if rendering fails.
	///
	/// # Note
	///
	/// Vue SSR requires @vue/server-renderer which is not yet integrated.
	pub async fn render_vue(
		&self,
		_component_path: &str,
		_props: &[u8],
		_options: RenderOptions,
	) -> Result<RenderResult, SsrError> {
		if !self.available {
			return Err(SsrError::NotAvailable);
		}

		// TODO: Implement Vue SSR
		// Vue requires @vue/server-renderer integration

		Err(SsrError::RenderFailed(
			"Vue SSR not yet implemented".to_string(),
		))
	}

	/// Execute arbitrary JavaScript/TypeScript code and return the result.
	///
	/// # Arguments
	///
	/// * `code` - JavaScript/TypeScript code to execute
	///
	/// # Returns
	///
	/// The result as a string, or an error if execution fails.
	///
	/// # Security
	///
	/// This function should only be available to plugins with elevated trust levels.
	pub fn eval_js(&self, code: &str) -> Result<String, SsrError> {
		if !self.available {
			return Err(SsrError::NotAvailable);
		}

		#[cfg(feature = "ts")]
		if let Some(ref runtime) = self.ts_runtime {
			return runtime.eval(code).map_err(SsrError::from);
		}

		Err(SsrError::NotAvailable)
	}

	/// Execute arbitrary JavaScript code and return the result as MessagePack bytes.
	///
	/// # Arguments
	///
	/// * `code` - JavaScript code to execute
	///
	/// # Returns
	///
	/// MessagePack-serialized result, or an error if execution fails.
	pub async fn eval_js_bytes(&self, _code: &str) -> Result<Vec<u8>, SsrError> {
		if !self.available {
			return Err(SsrError::NotAvailable);
		}

		// TODO: Implement MessagePack serialization of JS result
		Err(SsrError::EvalFailed(
			"MessagePack serialization not yet implemented".to_string(),
		))
	}

	/// Generate a hydration script for client-side takeover.
	///
	/// This creates a `<script>` tag that will re-render the component
	/// on the client side for interactivity.
	fn generate_hydration_script(&self, component_code: &str, props_json: &str) -> String {
		format!(
			r#"<script type="module">
import {{ h, render }} from 'https://esm.sh/preact@10';

{component_code}

const props = {props_json};
const root = document.getElementById('root');
if (root) {{
    render(h(Component, props), root);
}}
</script>"#
		)
	}
}

/// Shared SSR proxy instance type.
pub type SharedSsrProxy = std::sync::Arc<SsrProxy>;

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_ssr_proxy_unavailable_by_default() {
		let proxy = SsrProxy::new();
		assert!(!proxy.is_available());
	}

	#[test]
	fn test_ssr_proxy_with_availability() {
		let proxy = SsrProxy::with_availability(true);
		assert!(proxy.is_available());

		let proxy = SsrProxy::with_availability(false);
		assert!(!proxy.is_available());
	}

	#[tokio::test]
	async fn test_render_react_returns_not_available() {
		let proxy = SsrProxy::new();
		let result = proxy
			.render_react("test.jsx", &[], RenderOptions::default())
			.await;
		assert!(matches!(result, Err(SsrError::NotAvailable)));
	}

	#[tokio::test]
	async fn test_render_vue_returns_not_available() {
		let proxy = SsrProxy::new();
		let result = proxy
			.render_vue("test.vue", &[], RenderOptions::default())
			.await;
		assert!(matches!(result, Err(SsrError::NotAvailable)));
	}

	#[test]
	fn test_eval_js_returns_not_available() {
		let proxy = SsrProxy::new();
		let result = proxy.eval_js("console.log('test')");
		assert!(matches!(result, Err(SsrError::NotAvailable)));
	}

	#[test]
	fn test_render_options_default() {
		let options = RenderOptions::default();
		assert!(!options.include_hydration);
		assert!(!options.extract_css);
		assert!(!options.extract_meta);
	}

	#[test]
	fn test_render_result_structure() {
		let result = RenderResult {
			html: "<div>Hello</div>".to_string(),
			css: Some(".container { color: red; }".to_string()),
			meta: Some(vec![("title".to_string(), "My Page".to_string())]),
			hydration_script: Some("<script>hydrate()</script>".to_string()),
		};

		assert_eq!(result.html, "<div>Hello</div>");
		assert!(result.css.is_some());
		assert!(result.meta.is_some());
		assert!(result.hydration_script.is_some());
	}

	#[test]
	fn test_ssr_error_display() {
		let err = SsrError::NotAvailable;
		assert!(err.to_string().contains("not available"));

		let err = SsrError::ComponentNotFound("Header.jsx".to_string());
		assert!(err.to_string().contains("Header.jsx"));

		let err = SsrError::RenderFailed("syntax error".to_string());
		assert!(err.to_string().contains("syntax error"));
	}

	#[test]
	fn test_render_component_without_runtime() {
		let proxy = SsrProxy::new();
		let result = proxy.render_component(
			"function Component(props) { return h('div', null, 'test'); }",
			"{}",
			RenderOptions::default(),
		);
		assert!(matches!(result, Err(SsrError::NotAvailable)));
	}

	#[cfg(feature = "ts")]
	mod ts_tests {
		use super::*;
		use crate::wasm::ts_runtime::TsRuntime;
		use std::sync::Arc;

		#[test]
		fn test_ssr_proxy_with_ts_runtime() {
			let runtime = Arc::new(TsRuntime::new().unwrap());
			let proxy = SsrProxy::with_ts_runtime(runtime);
			assert!(proxy.is_available());
		}

		#[test]
		fn test_render_simple_component_with_ts_runtime() {
			let runtime = Arc::new(TsRuntime::new().unwrap());
			let proxy = SsrProxy::with_ts_runtime(runtime);

			let component = r#"
                function Component(props) {
                    return h('div', null, 'Hello, ' + props.name);
                }
            "#;
			let result =
				proxy.render_component(component, r#"{"name": "World"}"#, RenderOptions::default());

			assert!(result.is_ok());
			let render_result = result.unwrap();
			assert!(render_result.html.contains("Hello, World"));
			assert!(render_result.html.contains("<div>"));
		}

		#[test]
		fn test_eval_js_with_ts_runtime() {
			let runtime = Arc::new(TsRuntime::new().unwrap());
			let proxy = SsrProxy::with_ts_runtime(runtime);

			let result = proxy.eval_js("'Hello, ' + 'World'");
			assert!(result.is_ok());
			assert_eq!(result.unwrap(), "Hello, World");
		}

		#[test]
		fn test_hydration_script_generation() {
			let runtime = Arc::new(TsRuntime::new().unwrap());
			let proxy = SsrProxy::with_ts_runtime(runtime);

			let component = r#"
                function Component(props) {
                    return h('div', null, props.message);
                }
            "#;
			let options = RenderOptions {
				include_hydration: true,
				..Default::default()
			};
			let result = proxy.render_component(component, r#"{"message": "Hi"}"#, options);

			assert!(result.is_ok());
			let render_result = result.unwrap();
			assert!(render_result.hydration_script.is_some());
			let script = render_result.hydration_script.unwrap();
			assert!(script.contains("preact"));
			assert!(script.contains("Component"));
		}
	}
}
