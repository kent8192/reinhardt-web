//! File system watcher for detecting changes
//!
//! Monitors specified directories for file changes and notifies listeners
//! when modifications occur.

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Events emitted by the file watcher
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// A file was created
    Created(PathBuf),
    /// A file was modified
    Modified(PathBuf),
    /// A file was deleted
    Deleted(PathBuf),
    /// An error occurred while watching
    Error(String),
}

/// File system watcher
///
/// Monitors directories for file changes and sends events through a channel.
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    receiver: mpsc::UnboundedReceiver<WatchEvent>,
}

impl FileWatcher {
    /// Create a new file watcher
    ///
    /// # Arguments
    ///
    /// * `paths` - Directories to watch for changes
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use reinhardt_static::FileWatcher;
    /// use std::path::PathBuf;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let paths = vec![PathBuf::from("./static")];
    ///     let mut watcher = FileWatcher::new(&paths).unwrap();
    ///
    ///     while let Some(event) = watcher.next_event().await {
    ///         println!("File changed: {:?}", event);
    ///     }
    /// }
    /// ```
    pub fn new(paths: &[PathBuf]) -> Result<Self, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let tx = Arc::new(tx);

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            let event = match res {
                Ok(event) => match event.kind {
                    EventKind::Create(_) => {
                        if let Some(path) = event.paths.first() {
                            WatchEvent::Created(path.clone())
                        } else {
                            return;
                        }
                    }
                    EventKind::Modify(_) => {
                        if let Some(path) = event.paths.first() {
                            WatchEvent::Modified(path.clone())
                        } else {
                            return;
                        }
                    }
                    EventKind::Remove(_) => {
                        if let Some(path) = event.paths.first() {
                            WatchEvent::Deleted(path.clone())
                        } else {
                            return;
                        }
                    }
                    _ => return,
                },
                Err(e) => WatchEvent::Error(e.to_string()),
            };

            let _ = tx.send(event);
        })?;

        // Watch all specified paths
        for path in paths {
            watcher.watch(path, RecursiveMode::Recursive)?;
        }

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
        })
    }

    /// Wait for the next file system event
    ///
    /// Returns `None` if the watcher has been closed.
    pub async fn next_event(&mut self) -> Option<WatchEvent> {
        self.receiver.recv().await
    }

    /// Try to receive an event without blocking
    ///
    /// Returns `None` if no event is available.
    pub fn try_next_event(&mut self) -> Option<WatchEvent> {
        self.receiver.try_recv().ok()
    }
}

/// Builder for creating a file watcher with specific configuration
pub struct FileWatcherBuilder {
    paths: Vec<PathBuf>,
}

impl FileWatcherBuilder {
    /// Create a new builder
    ///
    /// # Examples
    ///
    /// ```rust
    /// use reinhardt_static::FileWatcherBuilder;
    /// use std::path::PathBuf;
    ///
    /// let builder = FileWatcherBuilder::new()
    ///     .watch_path(PathBuf::from("./static"))
    ///     .watch_path(PathBuf::from("./templates"));
    /// ```
    pub fn new() -> Self {
        Self { paths: Vec::new() }
    }

    /// Add a path to watch
    pub fn watch_path(mut self, path: PathBuf) -> Self {
        self.paths.push(path);
        self
    }

    /// Build the file watcher
    pub fn build(self) -> Result<FileWatcher, Box<dyn std::error::Error>> {
        FileWatcher::new(&self.paths)
    }
}

impl Default for FileWatcherBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_watch_event_variants() {
        let event = WatchEvent::Created(PathBuf::from("test.txt"));
        assert!(matches!(event, WatchEvent::Created(_)));

        let event = WatchEvent::Modified(PathBuf::from("test.txt"));
        assert!(matches!(event, WatchEvent::Modified(_)));

        let event = WatchEvent::Deleted(PathBuf::from("test.txt"));
        assert!(matches!(event, WatchEvent::Deleted(_)));

        let event = WatchEvent::Error("test error".to_string());
        assert!(matches!(event, WatchEvent::Error(_)));
    }

    #[tokio::test]
    async fn test_file_watcher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let paths = vec![temp_dir.path().to_path_buf()];

        let watcher = FileWatcher::new(&paths);
        assert!(watcher.is_ok());
    }

    #[tokio::test]
    async fn test_file_watcher_detects_changes() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let paths = vec![temp_dir.path().to_path_buf()];

        let mut watcher = FileWatcher::new(&paths).unwrap();

        // Create a file
        fs::write(&test_file, "test content").unwrap();

        // Wait for event (with timeout)
        let event =
            tokio::time::timeout(std::time::Duration::from_secs(2), watcher.next_event()).await;

        assert!(event.is_ok());
        let event = event.unwrap();
        assert!(event.is_some());
    }

    #[tokio::test]
    async fn test_file_watcher_try_next() {
        let temp_dir = TempDir::new().unwrap();
        let paths = vec![temp_dir.path().to_path_buf()];

        let mut watcher = FileWatcher::new(&paths).unwrap();

        // Try to receive without blocking (should return None immediately)
        let event = watcher.try_next_event();
        assert!(event.is_none());
    }

    #[test]
    fn test_builder_new() {
        let builder = FileWatcherBuilder::new();
        assert_eq!(builder.paths.len(), 0);
    }

    #[test]
    fn test_builder_watch_path() {
        let builder = FileWatcherBuilder::new()
            .watch_path(PathBuf::from("./static"))
            .watch_path(PathBuf::from("./templates"));

        assert_eq!(builder.paths.len(), 2);
    }

    #[test]
    fn test_builder_default() {
        let builder = FileWatcherBuilder::default();
        assert_eq!(builder.paths.len(), 0);
    }
}
