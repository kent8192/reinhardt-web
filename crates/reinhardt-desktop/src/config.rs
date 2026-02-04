//! Window configuration types.

/// Configuration for the desktop window.
#[derive(Debug, Clone)]
pub struct WindowConfig {
	/// Window title.
	pub title: String,

	/// Window width in pixels.
	pub width: u32,

	/// Window height in pixels.
	pub height: u32,

	/// Whether the window is resizable.
	pub resizable: bool,

	/// Whether the window starts maximized.
	pub maximized: bool,

	/// Whether the window starts fullscreen.
	pub fullscreen: bool,

	/// Whether to show window decorations (title bar, borders).
	pub decorations: bool,

	/// Whether the window is always on top.
	pub always_on_top: bool,

	/// Minimum window size (width, height).
	pub min_size: Option<(u32, u32)>,

	/// Maximum window size (width, height).
	pub max_size: Option<(u32, u32)>,
}

impl Default for WindowConfig {
	fn default() -> Self {
		Self {
			title: "Reinhardt Desktop App".to_string(),
			width: 800,
			height: 600,
			resizable: true,
			maximized: false,
			fullscreen: false,
			decorations: true,
			always_on_top: false,
			min_size: None,
			max_size: None,
		}
	}
}

impl WindowConfig {
	/// Creates a new WindowConfig with default values.
	pub fn new() -> Self {
		Self::default()
	}

	/// Sets the window title.
	pub fn title(mut self, title: impl Into<String>) -> Self {
		self.title = title.into();
		self
	}

	/// Sets the window size.
	pub fn size(mut self, width: u32, height: u32) -> Self {
		self.width = width;
		self.height = height;
		self
	}

	/// Sets whether the window is resizable.
	pub fn resizable(mut self, resizable: bool) -> Self {
		self.resizable = resizable;
		self
	}

	/// Sets whether the window starts maximized.
	pub fn maximized(mut self, maximized: bool) -> Self {
		self.maximized = maximized;
		self
	}

	/// Sets whether the window starts fullscreen.
	pub fn fullscreen(mut self, fullscreen: bool) -> Self {
		self.fullscreen = fullscreen;
		self
	}

	/// Sets whether to show window decorations.
	pub fn decorations(mut self, decorations: bool) -> Self {
		self.decorations = decorations;
		self
	}

	/// Sets whether the window is always on top.
	pub fn always_on_top(mut self, always_on_top: bool) -> Self {
		self.always_on_top = always_on_top;
		self
	}

	/// Sets the minimum window size.
	pub fn min_size(mut self, width: u32, height: u32) -> Self {
		self.min_size = Some((width, height));
		self
	}

	/// Sets the maximum window size.
	pub fn max_size(mut self, width: u32, height: u32) -> Self {
		self.max_size = Some((width, height));
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_window_config_default_values() {
		// Arrange & Act
		let config = WindowConfig::default();

		// Assert
		assert_eq!(config.title, "Reinhardt Desktop App");
		assert_eq!(config.width, 800);
		assert_eq!(config.height, 600);
		assert!(config.resizable);
		assert!(!config.maximized);
		assert!(!config.fullscreen);
		assert!(config.decorations);
		assert!(!config.always_on_top);
		assert!(config.min_size.is_none());
		assert!(config.max_size.is_none());
	}

	#[rstest]
	fn test_window_config_builder_chain() {
		// Arrange & Act
		let config = WindowConfig::new()
			.title("Test App")
			.size(1024, 768)
			.resizable(false)
			.maximized(true)
			.fullscreen(false)
			.decorations(false)
			.always_on_top(true)
			.min_size(400, 300)
			.max_size(1920, 1080);

		// Assert
		assert_eq!(config.title, "Test App");
		assert_eq!(config.width, 1024);
		assert_eq!(config.height, 768);
		assert!(!config.resizable);
		assert!(config.maximized);
		assert!(!config.fullscreen);
		assert!(!config.decorations);
		assert!(config.always_on_top);
		assert_eq!(config.min_size, Some((400, 300)));
		assert_eq!(config.max_size, Some((1920, 1080)));
	}
}
