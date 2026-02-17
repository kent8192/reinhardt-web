//! Hot reload functionality for dynamic settings
//!
//! This module provides file system watching and automatic reload capabilities
//! for configuration files, enabling runtime configuration changes without
//! application restarts.
//!
//! ## Features
//!
//! - **File watching**: Monitor configuration files for changes using `notify`
//! - **Debouncing**: Coalesce multiple rapid changes into a single reload
//! - **Callbacks**: Execute custom logic when configuration changes
//! - **Error handling**: Graceful handling of file system errors
//!
//! ## Example
//!
//! ```rust,no_run
//! # #[cfg(feature = "hot-reload")]
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use reinhardt_conf::settings::hot_reload::HotReloadManager;
//! use std::path::Path;
//! use std::sync::Arc;
//!
//! // Create manager
//! let manager = HotReloadManager::new();
//!
//! // Register a callback
//! manager.on_reload(Arc::new(|path| {
//!     println!("Configuration file changed: {:?}", path);
//! }));
//!
//! // Start watching a file
//! manager.watch(Path::new("config.toml")).await?;
//!
//! // Later: stop watching
//! manager.stop().await?;
//! # Ok(())
//! # }
//! ```

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Type alias for reload callbacks
///
/// Callbacks are invoked when a watched file changes.
pub type ReloadCallback = Arc<dyn Fn(&Path) + Send + Sync>;

/// Hot reload manager for configuration files
///
/// This manager watches configuration files for changes and triggers reload
/// callbacks when changes are detected. It includes debouncing to avoid
/// excessive reloads during rapid file changes.
///
/// ## Example
///
/// ```rust,no_run
/// # #[cfg(feature = "hot-reload")]
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use reinhardt_conf::settings::hot_reload::HotReloadManager;
/// use std::path::Path;
/// use std::sync::Arc;
///
/// let manager = HotReloadManager::new();
///
/// // Add callback
/// manager.on_reload(Arc::new(|path| {
///     println!("Reloading: {:?}", path);
/// }));
///
/// // Watch file
/// manager.watch(Path::new("settings.toml")).await?;
/// # Ok(())
/// # }
/// ```
pub struct HotReloadManager {
	watcher: Arc<Mutex<Option<RecommendedWatcher>>>,
	callbacks: Arc<Mutex<Vec<ReloadCallback>>>,
	watched_paths: Arc<Mutex<HashMap<PathBuf, Instant>>>,
	debounce_duration: Duration,
	event_tx: Arc<Mutex<Option<mpsc::UnboundedSender<Event>>>>,
}

impl HotReloadManager {
	/// Create a new hot reload manager
	///
	/// The default debounce duration is 100ms, which means changes occurring
	/// within 100ms of each other will be coalesced into a single reload event.
	///
	/// ## Example
	///
	/// ```rust
	/// use reinhardt_conf::settings::hot_reload::HotReloadManager;
	///
	/// let manager = HotReloadManager::new();
	/// ```
	pub fn new() -> Self {
		Self {
			watcher: Arc::new(Mutex::new(None)),
			callbacks: Arc::new(Mutex::new(Vec::new())),
			watched_paths: Arc::new(Mutex::new(HashMap::new())),
			debounce_duration: Duration::from_millis(100),
			event_tx: Arc::new(Mutex::new(None)),
		}
	}

	/// Create a new hot reload manager with custom debounce duration
	///
	/// ## Arguments
	///
	/// * `debounce_duration` - Time window for coalescing file change events
	///
	/// ## Example
	///
	/// ```rust
	/// use reinhardt_conf::settings::hot_reload::HotReloadManager;
	/// use std::time::Duration;
	///
	/// let manager = HotReloadManager::with_debounce(Duration::from_millis(200));
	/// ```
	pub fn with_debounce(debounce_duration: Duration) -> Self {
		Self {
			watcher: Arc::new(Mutex::new(None)),
			callbacks: Arc::new(Mutex::new(Vec::new())),
			watched_paths: Arc::new(Mutex::new(HashMap::new())),
			debounce_duration,
			event_tx: Arc::new(Mutex::new(None)),
		}
	}

	/// Register a callback to be invoked when files change
	///
	/// Multiple callbacks can be registered and will all be invoked in the
	/// order they were registered.
	///
	/// ## Example
	///
	/// ```rust
	/// use reinhardt_conf::settings::hot_reload::HotReloadManager;
	/// use std::sync::Arc;
	///
	/// let manager = HotReloadManager::new();
	///
	/// manager.on_reload(Arc::new(|path| {
	///     println!("Configuration changed: {:?}", path);
	/// }));
	/// ```
	pub fn on_reload(&self, callback: ReloadCallback) {
		self.callbacks.lock().push(callback);
	}

	/// Start watching a file or directory for changes
	///
	/// If watching a directory, all files within it will be monitored.
	///
	/// ## Arguments
	///
	/// * `path` - Path to watch (file or directory)
	///
	/// ## Example
	///
	/// ```rust,no_run
	/// # #[cfg(feature = "hot-reload")]
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// use reinhardt_conf::settings::hot_reload::HotReloadManager;
	/// use std::path::Path;
	///
	/// let manager = HotReloadManager::new();
	/// manager.watch(Path::new("config.toml")).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn watch(&self, path: &Path) -> Result<(), String> {
		// Canonicalize path
		let canonical_path = path
			.canonicalize()
			.map_err(|e| format!("Failed to canonicalize path {:?}: {}", path, e))?;

		// Initialize watcher if not already done
		self.ensure_watcher().await?;

		// Add path to watcher
		let mut watcher_guard = self.watcher.lock();
		if let Some(watcher) = watcher_guard.as_mut() {
			watcher
				.watch(&canonical_path, RecursiveMode::NonRecursive)
				.map_err(|e| format!("Failed to watch path {:?}: {}", canonical_path, e))?;

			// Track watched path with timestamp in the past to allow immediate first event
			drop(watcher_guard);
			let initial_timestamp = Instant::now()
				.checked_sub(self.debounce_duration)
				.unwrap_or_else(Instant::now);
			self.watched_paths
				.lock()
				.insert(canonical_path.clone(), initial_timestamp);

			Ok(())
		} else {
			Err("Watcher not initialized".to_string())
		}
	}

	/// Stop watching a previously watched path
	///
	/// ## Arguments
	///
	/// * `path` - Path to stop watching
	///
	/// ## Example
	///
	/// ```rust,no_run
	/// # #[cfg(feature = "hot-reload")]
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// use reinhardt_conf::settings::hot_reload::HotReloadManager;
	/// use std::path::Path;
	///
	/// let manager = HotReloadManager::new();
	/// manager.watch(Path::new("config.toml")).await?;
	/// manager.unwatch(Path::new("config.toml")).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn unwatch(&self, path: &Path) -> Result<(), String> {
		let canonical_path = path
			.canonicalize()
			.map_err(|e| format!("Failed to canonicalize path {:?}: {}", path, e))?;

		let mut watcher_guard = self.watcher.lock();
		if let Some(watcher) = watcher_guard.as_mut() {
			watcher
				.unwatch(&canonical_path)
				.map_err(|e| format!("Failed to unwatch path {:?}: {}", canonical_path, e))?;

			// Remove from tracked paths
			drop(watcher_guard);
			self.watched_paths.lock().remove(&canonical_path);

			Ok(())
		} else {
			Err("Watcher not initialized".to_string())
		}
	}

	/// Stop all file watching
	///
	/// This stops the watcher and clears all watched paths.
	///
	/// ## Example
	///
	/// ```rust,no_run
	/// # #[cfg(feature = "hot-reload")]
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// use reinhardt_conf::settings::hot_reload::HotReloadManager;
	/// use std::path::Path;
	///
	/// let manager = HotReloadManager::new();
	/// manager.watch(Path::new("config.toml")).await?;
	/// manager.stop().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn stop(&self) -> Result<(), String> {
		// Stop watcher
		*self.watcher.lock() = None;

		// Stop event processing
		*self.event_tx.lock() = None;

		// Clear watched paths
		self.watched_paths.lock().clear();

		Ok(())
	}

	/// Get list of currently watched paths
	///
	/// ## Example
	///
	/// ```rust,no_run
	/// # #[cfg(feature = "hot-reload")]
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// use reinhardt_conf::settings::hot_reload::HotReloadManager;
	/// use std::path::Path;
	///
	/// let manager = HotReloadManager::new();
	/// manager.watch(Path::new("config.toml")).await?;
	///
	/// let paths = manager.watched_paths();
	/// assert_eq!(paths.len(), 1);
	/// # Ok(())
	/// # }
	/// ```
	pub fn watched_paths(&self) -> Vec<PathBuf> {
		self.watched_paths.lock().keys().cloned().collect()
	}

	/// Initialize the file watcher if not already initialized
	async fn ensure_watcher(&self) -> Result<(), String> {
		let mut watcher_guard = self.watcher.lock();
		if watcher_guard.is_some() {
			return Ok(());
		}

		// Create event channel
		let (tx, mut rx) = mpsc::unbounded_channel();
		*self.event_tx.lock() = Some(tx.clone());

		// Create watcher
		let watcher = RecommendedWatcher::new(
			move |result: notify::Result<Event>| {
				if let Ok(event) = result {
					let _ = tx.send(event);
				}
			},
			Config::default(),
		)
		.map_err(|e| format!("Failed to create watcher: {}", e))?;

		*watcher_guard = Some(watcher);
		drop(watcher_guard);

		// Spawn event processing task
		let callbacks = self.callbacks.clone();
		let watched_paths = self.watched_paths.clone();
		let debounce_duration = self.debounce_duration;

		tokio::spawn(async move {
			while let Some(event) = rx.recv().await {
				Self::process_event(event, &callbacks, &watched_paths, debounce_duration).await;
			}
		});

		Ok(())
	}

	/// Process a file system event
	async fn process_event(
		event: Event,
		callbacks: &Arc<Mutex<Vec<ReloadCallback>>>,
		watched_paths: &Arc<Mutex<HashMap<PathBuf, Instant>>>,
		debounce_duration: Duration,
	) {
		// Only process modify events
		if !matches!(
			event.kind,
			EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
		) {
			return;
		}

		for path in event.paths {
			// Canonicalize path to match against watched paths
			let canonical_path = match path.canonicalize() {
				Ok(p) => p,
				Err(_) => continue, // Skip if path cannot be canonicalized (e.g., deleted file)
			};

			// Check if this path is being watched
			let mut paths_guard = watched_paths.lock();
			if let Some(last_event) = paths_guard.get_mut(&canonical_path) {
				let now = Instant::now();

				// Debounce: skip if event is too soon after last one
				if now.duration_since(*last_event) < debounce_duration {
					continue;
				}

				// Update last event time
				*last_event = now;
				drop(paths_guard);

				// Invoke all callbacks
				let callbacks_guard = callbacks.lock();
				for callback in callbacks_guard.iter() {
					callback(&canonical_path);
				}
			}
		}
	}
}

impl Default for HotReloadManager {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use std::fs;
	use tempfile::TempDir;

	#[rstest]
	#[tokio::test]
	async fn test_new_manager() {
		let manager = HotReloadManager::new();
		assert_eq!(manager.watched_paths().len(), 0);
	}

	#[rstest]
	#[tokio::test]
	async fn test_with_custom_debounce() {
		let manager = HotReloadManager::with_debounce(Duration::from_millis(500));
		assert_eq!(manager.debounce_duration, Duration::from_millis(500));
	}

	#[rstest]
	#[tokio::test]
	async fn test_watch_file() {
		let temp_dir = TempDir::new().unwrap();
		let file_path = temp_dir.path().join("test.txt");
		fs::write(&file_path, "initial content").unwrap();

		let manager = HotReloadManager::new();
		manager.watch(&file_path).await.unwrap();

		let watched = manager.watched_paths();
		assert_eq!(watched.len(), 1);
	}

	#[rstest]
	#[tokio::test]
	async fn test_unwatch_file() {
		let temp_dir = TempDir::new().unwrap();
		let file_path = temp_dir.path().join("test.txt");
		fs::write(&file_path, "initial content").unwrap();

		let manager = HotReloadManager::new();
		manager.watch(&file_path).await.unwrap();
		assert_eq!(manager.watched_paths().len(), 1);

		manager.unwatch(&file_path).await.unwrap();
		assert_eq!(manager.watched_paths().len(), 0);
	}

	#[rstest]
	#[tokio::test]
	async fn test_stop_watching() {
		let temp_dir = TempDir::new().unwrap();
		let file_path = temp_dir.path().join("test.txt");
		fs::write(&file_path, "initial content").unwrap();

		let manager = HotReloadManager::new();
		manager.watch(&file_path).await.unwrap();

		manager.stop().await.unwrap();
		assert_eq!(manager.watched_paths().len(), 0);
	}

	// Note: File system event tests are moved to integration tests
	// because they depend on OS-level file watching which can be flaky in unit tests

	#[rstest]
	#[tokio::test]
	async fn test_watch_nonexistent_file() {
		let manager = HotReloadManager::new();
		let result = manager.watch(Path::new("/nonexistent/file.txt")).await;
		assert!(result.is_err());
	}

	#[rstest]
	#[tokio::test]
	async fn test_default_implementation() {
		let manager = HotReloadManager::default();
		assert_eq!(manager.watched_paths().len(), 0);
		assert_eq!(manager.debounce_duration, Duration::from_millis(100));
	}
}
