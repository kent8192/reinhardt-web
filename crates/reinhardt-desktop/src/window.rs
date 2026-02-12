//! Window management using tao.

use tao::dpi::{LogicalSize, PhysicalSize};
use tao::event_loop::EventLoop;
use tao::window::{Window, WindowBuilder};

use crate::config::WindowConfig;
use crate::error::{DesktopError, Result};

/// Manages the application window.
pub struct WindowManager {
	window: Window,
}

impl WindowManager {
	/// Creates a new window with the given configuration.
	pub fn new(event_loop: &EventLoop<()>, config: &WindowConfig) -> Result<Self> {
		let mut builder = WindowBuilder::new()
			.with_title(&config.title)
			.with_inner_size(LogicalSize::new(config.width, config.height))
			.with_resizable(config.resizable)
			.with_maximized(config.maximized)
			.with_decorations(config.decorations)
			.with_always_on_top(config.always_on_top);

		if let Some((min_w, min_h)) = config.min_size {
			builder = builder.with_min_inner_size(PhysicalSize::new(min_w, min_h));
		}

		if let Some((max_w, max_h)) = config.max_size {
			builder = builder.with_max_inner_size(PhysicalSize::new(max_w, max_h));
		}

		if config.fullscreen {
			builder = builder.with_fullscreen(Some(tao::window::Fullscreen::Borderless(None)));
		}

		let window = builder
			.build(event_loop)
			.map_err(|e| DesktopError::WindowCreation(e.to_string()))?;

		Ok(Self { window })
	}

	/// Returns a reference to the underlying tao Window.
	pub fn window(&self) -> &Window {
		&self.window
	}

	/// Returns the window's current size.
	pub fn inner_size(&self) -> (u32, u32) {
		let size = self.window.inner_size();
		(size.width, size.height)
	}

	/// Sets the window title.
	pub fn set_title(&self, title: &str) {
		self.window.set_title(title);
	}

	/// Sets whether the window is visible.
	pub fn set_visible(&self, visible: bool) {
		self.window.set_visible(visible);
	}

	/// Requests the window to be redrawn.
	pub fn request_redraw(&self) {
		self.window.request_redraw();
	}
}
