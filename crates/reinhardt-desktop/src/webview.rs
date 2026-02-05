//! WebView management using wry.

use std::sync::Arc;
use tao::window::Window;
use wry::http::Response;
use wry::{WebView, WebViewBuilder};

use crate::error::{DesktopError, Result};
use crate::ipc::{IPC_INIT_SCRIPT, IpcHandler};
use crate::protocol::{PROTOCOL_SCHEME, ProtocolHandler};

/// Manages the WebView instance.
pub struct WebViewManager {
	webview: WebView,
}

impl WebViewManager {
	/// Creates a new WebView attached to the given window.
	pub fn new(
		window: &Window,
		protocol_handler: Arc<ProtocolHandler>,
		ipc_handler: Arc<IpcHandler>,
	) -> Result<Self> {
		let ipc_handler_clone = ipc_handler.clone();

		let webview = WebViewBuilder::new()
			.with_initialization_script(IPC_INIT_SCRIPT)
			.with_ipc_handler(move |request| {
				let body = request.body();
				let response = ipc_handler_clone.handle_raw(body);

				// Evaluate response callback in WebView
				// Note: This is a simplified implementation. In production,
				// you'd want to use a proper callback mechanism.
				tracing::debug!("IPC response: {}", response);
			})
			.with_asynchronous_custom_protocol(
				PROTOCOL_SCHEME.to_string(),
				move |_webview_id, request, responder| {
					let path = request.uri().path();
					let protocol = protocol_handler.clone();

					match protocol.resolve(path) {
						Ok(asset) => {
							let response = Response::builder()
								.status(200)
								.header("Content-Type", &asset.mime_type)
								.header("Access-Control-Allow-Origin", "*")
								.body(asset.content.to_vec())
								.unwrap();
							responder.respond(response);
						}
						Err(_) => {
							let response = Response::builder()
								.status(404)
								.header("Content-Type", "text/plain")
								.body(b"Not Found".to_vec())
								.unwrap();
							responder.respond(response);
						}
					}
				},
			)
			.with_devtools(cfg!(debug_assertions))
			.build_as_child(window)
			.map_err(|e| DesktopError::WebViewCreation(e.to_string()))?;

		Ok(Self { webview })
	}

	/// Returns a reference to the underlying WebView.
	pub fn webview(&self) -> &WebView {
		&self.webview
	}

	/// Navigates to a URL.
	pub fn navigate(&self, url: &str) -> Result<()> {
		self.webview
			.load_url(url)
			.map_err(|e| DesktopError::WebViewCreation(e.to_string()))
	}

	/// Navigates to an asset using the custom protocol.
	pub fn navigate_to_asset(&self, path: &str) -> Result<()> {
		let url = format!(
			"{}://localhost/{}",
			PROTOCOL_SCHEME,
			path.trim_start_matches('/')
		);
		self.navigate(&url)
	}

	/// Evaluates JavaScript in the WebView.
	pub fn evaluate_script(&self, script: &str) -> Result<()> {
		self.webview
			.evaluate_script(script)
			.map_err(|e| DesktopError::WebViewCreation(e.to_string()))
	}

	/// Sets the WebView bounds to fill the window.
	pub fn set_bounds(&self, x: i32, y: i32, width: u32, height: u32) -> Result<()> {
		self.webview
			.set_bounds(wry::Rect {
				position: wry::dpi::Position::Logical(wry::dpi::LogicalPosition::new(
					x as f64, y as f64,
				)),
				size: wry::dpi::Size::Logical(wry::dpi::LogicalSize::new(
					width as f64,
					height as f64,
				)),
			})
			.map_err(|e| DesktopError::WebViewCreation(e.to_string()))
	}
}
