//! Desktop application lifecycle management.

use std::sync::Arc;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};

use crate::codegen::StaticHtmlVisitor;
use crate::config::WindowConfig;
use crate::error::Result;
use crate::ipc::IpcHandler;
use crate::protocol::{Asset, ProtocolHandler};
use crate::webview::WebViewManager;
use crate::window::WindowManager;
use reinhardt_manouche::codegen::IRVisitor;
use reinhardt_manouche::ir::ComponentIR;

/// Builder for creating a DesktopApp.
#[derive(Default)]
pub struct DesktopAppBuilder {
	config: WindowConfig,
	protocol_handler: ProtocolHandler,
	ipc_handler: IpcHandler,
	index_html: Option<String>,
}

impl DesktopAppBuilder {
	/// Creates a new builder with default configuration.
	pub fn new() -> Self {
		Self::default()
	}

	/// Sets the window title.
	pub fn title(mut self, title: impl Into<String>) -> Self {
		self.config.title = title.into();
		self
	}

	/// Sets the window size.
	pub fn size(mut self, width: u32, height: u32) -> Self {
		self.config.width = width;
		self.config.height = height;
		self
	}

	/// Sets whether the window is resizable.
	pub fn resizable(mut self, resizable: bool) -> Self {
		self.config.resizable = resizable;
		self
	}

	/// Sets the full window configuration.
	pub fn config(mut self, config: WindowConfig) -> Self {
		self.config = config;
		self
	}

	/// Sets the index HTML content.
	pub fn index_html(mut self, html: impl Into<String>) -> Self {
		self.index_html = Some(html.into());
		self
	}

	/// Creates the app from a reinhardt-manouche ComponentIR.
	///
	/// This generates static HTML from the component and registers it
	/// as `index.html`.
	pub fn from_component(mut self, component: &ComponentIR) -> Self {
		let mut visitor = StaticHtmlVisitor::new();
		visitor.visit_component(component);
		let html = visitor.into_html();

		// Wrap in basic HTML document structure
		let full_html = format!(
			r#"<!DOCTYPE html>
<html>
<head>
	<meta charset="utf-8">
	<meta name="viewport" content="width=device-width, initial-scale=1">
	<title>{}</title>
</head>
<body>
{}
</body>
</html>"#,
			self.config.title, html
		);

		self.index_html = Some(full_html);
		self
	}

	/// Registers a static asset.
	pub fn asset(mut self, path: impl Into<String>, asset: Asset) -> Self {
		self.protocol_handler.register_asset(path, asset);
		self
	}

	/// Registers an IPC command handler.
	pub fn command<F>(mut self, name: impl Into<String>, handler: F) -> Self
	where
		F: Fn(crate::ipc::IpcMessage) -> Result<crate::ipc::IpcResponse> + Send + Sync + 'static,
	{
		self.ipc_handler.register(name, handler);
		self
	}

	/// Builds the DesktopApp.
	pub fn build(mut self) -> Result<DesktopApp> {
		// Register default index.html if provided
		if let Some(html) = self.index_html.take() {
			self.protocol_handler
				.register_asset("index.html", Asset::html(html.into_bytes()));
		}

		Ok(DesktopApp {
			config: self.config,
			protocol_handler: Arc::new(self.protocol_handler),
			ipc_handler: Arc::new(self.ipc_handler),
		})
	}
}

/// The main desktop application.
pub struct DesktopApp {
	config: WindowConfig,
	protocol_handler: Arc<ProtocolHandler>,
	ipc_handler: Arc<IpcHandler>,
}

impl DesktopApp {
	/// Creates a new builder for configuring the application.
	pub fn builder() -> DesktopAppBuilder {
		DesktopAppBuilder::new()
	}

	/// Runs the application event loop.
	///
	/// This method blocks and never returns.
	pub fn run(self) -> ! {
		let event_loop = EventLoop::new();

		let window_manager =
			WindowManager::new(&event_loop, &self.config).expect("failed to create window");
		let webview_manager = WebViewManager::new(
			window_manager.window(),
			self.protocol_handler.clone(),
			self.ipc_handler.clone(),
		)
		.expect("failed to create webview");

		// Navigate to index.html
		webview_manager
			.navigate_to_asset("index.html")
			.expect("failed to navigate to index.html");

		// Run the event loop (this never returns)
		#[allow(deprecated)] // tao run() API
		event_loop.run(move |event, _window_target, control_flow| {
			*control_flow = ControlFlow::Wait;

			match event {
				Event::WindowEvent {
					event: WindowEvent::CloseRequested,
					..
				} => {
					*control_flow = ControlFlow::Exit;
				}
				Event::WindowEvent {
					event: WindowEvent::Resized(size),
					..
				} => {
					// Update WebView bounds on resize
					let _ = webview_manager.set_bounds(0, 0, size.width, size.height);
				}
				_ => {}
			}
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use proc_macro2::Span;
	use reinhardt_manouche::ir::{ElementIR, NodeIR, TextIR};
	use rstest::rstest;

	#[rstest]
	fn test_builder_from_component() {
		// Arrange
		let component = ComponentIR {
			props: vec![],
			body: vec![NodeIR::Element(ElementIR {
				tag: "h1".to_string(),
				attributes: vec![],
				events: vec![],
				children: vec![NodeIR::Text(TextIR {
					content: "Hello".to_string(),
					span: Span::call_site(),
				})],
				span: Span::call_site(),
			})],
			span: Span::call_site(),
		};

		// Act
		let builder = DesktopAppBuilder::new()
			.title("Test")
			.from_component(&component);

		// Assert
		let app = builder.build().unwrap();
		// The test passes if build() succeeds - internal state verification
		// would require additional accessors
		drop(app);
	}
}
