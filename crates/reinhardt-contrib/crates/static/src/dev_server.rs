//! Development server features
//!
//! Provides development-only features like file watching, auto-reload,
//! and enhanced error pages for a better development experience.

use std::path::PathBuf;

pub mod error_pages;
pub mod reload;
pub mod watcher;

pub use error_pages::DevelopmentErrorHandler;
pub use reload::{AutoReload, AutoReloadBuilder, ReloadEvent};
pub use watcher::{FileWatcher, FileWatcherBuilder, WatchEvent};

/// Configuration for development server features
#[derive(Debug, Clone)]
pub struct DevServerConfig {
	/// Enable file watching
	pub watch_files: bool,
	/// Enable auto-reload on file changes
	pub auto_reload: bool,
	/// Enable development error pages
	pub debug_errors: bool,
	/// Directories to watch for changes
	pub watch_paths: Vec<PathBuf>,
	/// Port for auto-reload WebSocket server
	pub reload_port: u16,
}

impl DevServerConfig {
	/// Create a new development server configuration
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_static::DevServerConfig;
	/// use std::path::PathBuf;
	///
	/// let config = DevServerConfig::new()
	///     .with_watch_path(PathBuf::from("./static"))
	///     .with_auto_reload(true);
	/// ```
	pub fn new() -> Self {
		Self {
			watch_files: true,
			auto_reload: true,
			debug_errors: true,
			watch_paths: Vec::new(),
			reload_port: 35729, // LiveReload default port
		}
	}

	/// Enable or disable file watching
	pub fn with_watch_files(mut self, enable: bool) -> Self {
		self.watch_files = enable;
		self
	}

	/// Enable or disable auto-reload
	pub fn with_auto_reload(mut self, enable: bool) -> Self {
		self.auto_reload = enable;
		self
	}

	/// Enable or disable debug error pages
	pub fn with_debug_errors(mut self, enable: bool) -> Self {
		self.debug_errors = enable;
		self
	}

	/// Add a path to watch for changes
	pub fn with_watch_path(mut self, path: PathBuf) -> Self {
		self.watch_paths.push(path);
		self
	}

	/// Set the port for the auto-reload WebSocket server
	pub fn with_reload_port(mut self, port: u16) -> Self {
		self.reload_port = port;
		self
	}
}

impl Default for DevServerConfig {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_config_defaults() {
		let config = DevServerConfig::new();
		assert!(config.watch_files);
		assert!(config.auto_reload);
		assert!(config.debug_errors);
		assert_eq!(config.reload_port, 35729);
		assert!(config.watch_paths.is_empty());
	}

	#[test]
	fn test_config_builder() {
		let config = DevServerConfig::new()
			.with_watch_files(false)
			.with_auto_reload(false)
			.with_debug_errors(false)
			.with_watch_path(PathBuf::from("./static"))
			.with_reload_port(8080);

		assert!(!config.watch_files);
		assert!(!config.auto_reload);
		assert!(!config.debug_errors);
		assert_eq!(config.reload_port, 8080);
		assert_eq!(config.watch_paths.len(), 1);
	}

	#[test]
	fn test_multiple_watch_paths() {
		let config = DevServerConfig::new()
			.with_watch_path(PathBuf::from("./static"))
			.with_watch_path(PathBuf::from("./templates"))
			.with_watch_path(PathBuf::from("./assets"));

		assert_eq!(config.watch_paths.len(), 3);
	}
}
