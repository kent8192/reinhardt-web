//! Hot reload support for desktop applications.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::bundling::AssetType;

/// Kind of file change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
	/// File was created.
	Created,
	/// File was modified.
	Modified,
	/// File was deleted.
	Deleted,
}

/// A file change event.
#[derive(Debug, Clone)]
pub struct FileChangeEvent {
	/// Path to the changed file.
	pub path: PathBuf,
	/// Kind of change.
	pub kind: ChangeKind,
}

impl FileChangeEvent {
	/// Creates a new file change event.
	pub fn new(path: impl AsRef<Path>, kind: ChangeKind) -> Self {
		Self {
			path: path.as_ref().to_path_buf(),
			kind,
		}
	}

	/// Determines the asset type based on file extension.
	pub fn asset_type(&self) -> Option<AssetType> {
		let ext = self.path.extension()?.to_str()?;
		match ext.to_lowercase().as_str() {
			"css" => Some(AssetType::Css),
			"js" | "mjs" => Some(AssetType::Js),
			_ => None,
		}
	}
}

/// Strategy for reloading content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadStrategy {
	/// Inject CSS without full page reload.
	CssInjection,
	/// Full page reload required.
	FullReload,
	/// No action needed.
	None,
}

impl ReloadStrategy {
	/// Determines reload strategy for a file change event.
	pub fn for_event(event: &FileChangeEvent) -> Self {
		// Deleted files always require full reload
		if event.kind == ChangeKind::Deleted {
			return Self::FullReload;
		}

		match event.asset_type() {
			Some(AssetType::Css) => Self::CssInjection,
			Some(AssetType::Js) => Self::FullReload,
			None => Self::None,
		}
	}
}

/// IPC message for hot reload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum HotReloadMessage {
	/// CSS content update (inject without reload).
	#[serde(rename = "css_update")]
	CssUpdate {
		/// The CSS content to inject.
		css: String,
	},
	/// Full page reload required.
	#[serde(rename = "full_reload")]
	FullReload,
}

impl HotReloadMessage {
	/// Creates a CSS update message.
	pub fn css_update(css: impl Into<String>) -> Self {
		Self::CssUpdate { css: css.into() }
	}

	/// Creates a full reload message.
	pub fn full_reload() -> Self {
		Self::FullReload
	}
}

/// Manages hot reload for desktop applications.
#[derive(Debug, Default)]
pub struct DesktopHotReloadManager {
	watch_paths: Vec<PathBuf>,
	is_watching: bool,
}

impl DesktopHotReloadManager {
	/// Creates a new hot reload manager.
	pub fn new() -> Self {
		Self::default()
	}

	/// Adds a path to watch for changes.
	pub fn add_watch_path(&mut self, path: impl AsRef<Path>) {
		self.watch_paths.push(path.as_ref().to_path_buf());
	}

	/// Returns current watch paths.
	pub fn watch_paths(&self) -> &[PathBuf] {
		&self.watch_paths
	}

	/// Returns whether watching is active.
	pub fn is_watching(&self) -> bool {
		self.is_watching
	}

	/// Starts watching for file changes.
	///
	/// Note: Full implementation requires the `notify` crate.
	pub fn start(&mut self) -> crate::error::Result<()> {
		self.is_watching = true;
		Ok(())
	}

	/// Stops watching for file changes.
	pub fn stop(&mut self) {
		self.is_watching = false;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_file_change_event_css() {
		// Arrange & Act
		let event = FileChangeEvent::new("/path/to/style.css", ChangeKind::Modified);

		// Assert
		assert_eq!(event.path.to_str().unwrap(), "/path/to/style.css");
		assert_eq!(event.kind, ChangeKind::Modified);
		assert_eq!(event.asset_type(), Some(AssetType::Css));
	}

	#[rstest]
	fn test_file_change_event_js() {
		// Arrange & Act
		let event = FileChangeEvent::new("/path/to/app.js", ChangeKind::Created);

		// Assert
		assert_eq!(event.asset_type(), Some(AssetType::Js));
	}

	#[rstest]
	fn test_file_change_event_mjs() {
		// Arrange & Act
		let event = FileChangeEvent::new("/path/to/module.mjs", ChangeKind::Modified);

		// Assert
		assert_eq!(event.asset_type(), Some(AssetType::Js));
	}

	#[rstest]
	fn test_file_change_event_unknown() {
		// Arrange & Act
		let event = FileChangeEvent::new("/path/to/file.txt", ChangeKind::Modified);

		// Assert
		assert_eq!(event.asset_type(), None);
	}

	#[rstest]
	fn test_reload_strategy_for_css() {
		// Arrange
		let event = FileChangeEvent::new("/style.css", ChangeKind::Modified);

		// Act
		let strategy = ReloadStrategy::for_event(&event);

		// Assert
		assert_eq!(strategy, ReloadStrategy::CssInjection);
	}

	#[rstest]
	fn test_reload_strategy_for_js() {
		// Arrange
		let event = FileChangeEvent::new("/app.js", ChangeKind::Modified);

		// Act
		let strategy = ReloadStrategy::for_event(&event);

		// Assert
		assert_eq!(strategy, ReloadStrategy::FullReload);
	}

	#[rstest]
	fn test_reload_strategy_for_deleted() {
		// Arrange
		let event = FileChangeEvent::new("/style.css", ChangeKind::Deleted);

		// Act
		let strategy = ReloadStrategy::for_event(&event);

		// Assert
		assert_eq!(strategy, ReloadStrategy::FullReload);
	}

	#[rstest]
	fn test_reload_strategy_for_unknown() {
		// Arrange
		let event = FileChangeEvent::new("/file.txt", ChangeKind::Modified);

		// Act
		let strategy = ReloadStrategy::for_event(&event);

		// Assert
		assert_eq!(strategy, ReloadStrategy::None);
	}

	#[rstest]
	fn test_hot_reload_message_css_update() {
		// Arrange
		let css = ".updated { color: green; }";

		// Act
		let msg = HotReloadMessage::css_update(css);
		let json = serde_json::to_string(&msg).unwrap();

		// Assert
		assert!(json.contains("css_update"));
		assert!(json.contains(".updated { color: green; }"));
	}

	#[rstest]
	fn test_hot_reload_message_full_reload() {
		// Act
		let msg = HotReloadMessage::full_reload();
		let json = serde_json::to_string(&msg).unwrap();

		// Assert
		assert!(json.contains("full_reload"));
	}

	#[rstest]
	fn test_hot_reload_message_deserialize() {
		// Arrange
		let json = r#"{"type":"css_update","payload":{"css":"body{}"}}"#;

		// Act
		let msg: HotReloadMessage = serde_json::from_str(json).unwrap();

		// Assert
		match msg {
			HotReloadMessage::CssUpdate { css } => assert_eq!(css, "body{}"),
			_ => panic!("Expected CssUpdate"),
		}
	}

	#[rstest]
	fn test_hot_reload_manager_creation() {
		// Arrange & Act
		let manager = DesktopHotReloadManager::new();

		// Assert
		assert!(!manager.is_watching());
		assert!(manager.watch_paths().is_empty());
	}

	#[rstest]
	fn test_hot_reload_manager_watch_paths() {
		// Arrange
		let mut manager = DesktopHotReloadManager::new();

		// Act
		manager.add_watch_path("/tmp/test_assets");
		manager.add_watch_path("/tmp/other_assets");

		// Assert
		assert_eq!(manager.watch_paths().len(), 2);
	}

	#[rstest]
	fn test_hot_reload_manager_start_stop() {
		// Arrange
		let mut manager = DesktopHotReloadManager::new();

		// Act & Assert
		assert!(!manager.is_watching());
		manager.start().unwrap();
		assert!(manager.is_watching());
		manager.stop();
		assert!(!manager.is_watching());
	}
}
