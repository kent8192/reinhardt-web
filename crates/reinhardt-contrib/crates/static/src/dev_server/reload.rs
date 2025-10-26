//! Auto-reload functionality for development
//!
//! Provides a mechanism to notify clients (typically browsers) when files change,
//! enabling automatic page reloads during development.

use super::WatchEvent;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

/// Auto-reload notification system
///
/// Manages reload notifications to connected clients when files change.
pub struct AutoReload {
    /// Broadcast channel for reload events
    tx: broadcast::Sender<ReloadEvent>,
    /// Number of currently connected clients
    clients: Arc<RwLock<usize>>,
}

/// Events sent to reload clients
#[derive(Debug, Clone)]
pub enum ReloadEvent {
    /// Reload the page
    Reload,
    /// Reload a specific file (e.g., CSS without full page reload)
    ReloadFile(String),
    /// Clear the cache
    ClearCache,
}

impl AutoReload {
    /// Create a new auto-reload system
    ///
    /// # Examples
    ///
    /// ```rust
    /// use reinhardt_static::AutoReload;
    ///
    /// let reload = AutoReload::new();
    /// ```
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            tx,
            clients: Arc::new(RwLock::new(0)),
        }
    }

    /// Trigger a full page reload
    ///
    /// # Examples
    ///
    /// ```rust
    /// use reinhardt_static::AutoReload;
    ///
    /// let reload = AutoReload::new();
    /// reload.trigger_reload();
    /// ```
    pub fn trigger_reload(&self) {
        let _ = self.tx.send(ReloadEvent::Reload);
    }

    /// Trigger a reload for a specific file
    ///
    /// This is useful for CSS files where a full page reload is not necessary.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use reinhardt_static::AutoReload;
    ///
    /// let reload = AutoReload::new();
    /// reload.trigger_file_reload("/static/css/main.css".to_string());
    /// ```
    pub fn trigger_file_reload(&self, file_path: String) {
        let _ = self.tx.send(ReloadEvent::ReloadFile(file_path));
    }

    /// Trigger a cache clear
    pub fn trigger_cache_clear(&self) {
        let _ = self.tx.send(ReloadEvent::ClearCache);
    }

    /// Subscribe to reload events
    ///
    /// Returns a receiver that will receive reload events.
    pub fn subscribe(&self) -> broadcast::Receiver<ReloadEvent> {
        self.tx.subscribe()
    }

    /// Get the number of currently connected clients
    pub async fn client_count(&self) -> usize {
        *self.clients.read().await
    }

    /// Increment the client count
    pub async fn add_client(&self) {
        let mut count = self.clients.write().await;
        *count += 1;
    }

    /// Decrement the client count
    pub async fn remove_client(&self) {
        let mut count = self.clients.write().await;
        if *count > 0 {
            *count -= 1;
        }
    }

    /// Handle a file watcher event
    ///
    /// Determines the appropriate reload action based on the file change.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use reinhardt_static::{AutoReload, WatchEvent};
    /// use std::path::PathBuf;
    ///
    /// let reload = AutoReload::new();
    /// let event = WatchEvent::Modified(PathBuf::from("./static/css/main.css"));
    /// reload.handle_watch_event(event);
    /// ```
    pub fn handle_watch_event(&self, event: WatchEvent) {
        match event {
            WatchEvent::Modified(path) | WatchEvent::Created(path) => {
                let path_str = path.to_string_lossy().to_string();

                // CSS files can be reloaded without full page reload
                if path_str.ends_with(".css") {
                    self.trigger_file_reload(path_str);
                } else {
                    // For other files, trigger full reload
                    self.trigger_reload();
                }
            }
            WatchEvent::Deleted(_) => {
                // Clear cache and reload
                self.trigger_cache_clear();
                self.trigger_reload();
            }
            WatchEvent::Error(err) => {
                eprintln!("File watch error: {}", err);
            }
        }
    }
}

impl Default for AutoReload {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating an auto-reload system with a file watcher
pub struct AutoReloadBuilder {
    reload: AutoReload,
}

impl AutoReloadBuilder {
    /// Create a new builder
    ///
    /// # Examples
    ///
    /// ```rust
    /// use reinhardt_static::AutoReloadBuilder;
    ///
    /// let builder = AutoReloadBuilder::new();
    /// let reload = builder.build();
    /// ```
    pub fn new() -> Self {
        Self {
            reload: AutoReload::new(),
        }
    }

    /// Build the auto-reload system
    pub fn build(self) -> AutoReload {
        self.reload
    }
}

impl Default for AutoReloadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_auto_reload_new() {
        let reload = AutoReload::new();
        assert_eq!(tokio_test::block_on(reload.client_count()), 0);
    }

    #[test]
    fn test_trigger_reload() {
        let reload = AutoReload::new();
        let mut rx = reload.subscribe();

        reload.trigger_reload();

        let event = rx.try_recv().unwrap();
        assert!(matches!(event, ReloadEvent::Reload));
    }

    #[test]
    fn test_trigger_file_reload() {
        let reload = AutoReload::new();
        let mut rx = reload.subscribe();

        reload.trigger_file_reload("/static/css/main.css".to_string());

        let event = rx.try_recv().unwrap();
        match event {
            ReloadEvent::ReloadFile(path) => {
                assert_eq!(path, "/static/css/main.css");
            }
            _ => panic!("Expected ReloadFile event"),
        }
    }

    #[test]
    fn test_trigger_cache_clear() {
        let reload = AutoReload::new();
        let mut rx = reload.subscribe();

        reload.trigger_cache_clear();

        let event = rx.try_recv().unwrap();
        assert!(matches!(event, ReloadEvent::ClearCache));
    }

    #[tokio::test]
    async fn test_client_count() {
        let reload = AutoReload::new();

        assert_eq!(reload.client_count().await, 0);

        reload.add_client().await;
        assert_eq!(reload.client_count().await, 1);

        reload.add_client().await;
        assert_eq!(reload.client_count().await, 2);

        reload.remove_client().await;
        assert_eq!(reload.client_count().await, 1);

        reload.remove_client().await;
        assert_eq!(reload.client_count().await, 0);
    }

    #[tokio::test]
    async fn test_remove_client_at_zero() {
        let reload = AutoReload::new();

        reload.remove_client().await;
        assert_eq!(reload.client_count().await, 0);
    }

    #[test]
    fn test_handle_css_modification() {
        let reload = AutoReload::new();
        let mut rx = reload.subscribe();

        let event = WatchEvent::Modified(PathBuf::from("./static/css/main.css"));
        reload.handle_watch_event(event);

        let reload_event = rx.try_recv().unwrap();
        assert!(matches!(reload_event, ReloadEvent::ReloadFile(_)));
    }

    #[test]
    fn test_handle_js_modification() {
        let reload = AutoReload::new();
        let mut rx = reload.subscribe();

        let event = WatchEvent::Modified(PathBuf::from("./static/js/app.js"));
        reload.handle_watch_event(event);

        let reload_event = rx.try_recv().unwrap();
        assert!(matches!(reload_event, ReloadEvent::Reload));
    }

    #[test]
    fn test_handle_file_deletion() {
        let reload = AutoReload::new();
        let mut rx = reload.subscribe();

        let event = WatchEvent::Deleted(PathBuf::from("./static/test.txt"));
        reload.handle_watch_event(event);

        // Should receive both ClearCache and Reload events
        let event1 = rx.try_recv().unwrap();
        let event2 = rx.try_recv().unwrap();

        assert!(matches!(event1, ReloadEvent::ClearCache) || matches!(event1, ReloadEvent::Reload));
        assert!(matches!(event2, ReloadEvent::ClearCache) || matches!(event2, ReloadEvent::Reload));
    }

    #[test]
    fn test_multiple_subscribers() {
        let reload = AutoReload::new();
        let mut rx1 = reload.subscribe();
        let mut rx2 = reload.subscribe();

        reload.trigger_reload();

        let event1 = rx1.try_recv().unwrap();
        let event2 = rx2.try_recv().unwrap();

        assert!(matches!(event1, ReloadEvent::Reload));
        assert!(matches!(event2, ReloadEvent::Reload));
    }

    #[test]
    fn test_builder() {
        let builder = AutoReloadBuilder::new();
        let reload = builder.build();
        assert_eq!(tokio_test::block_on(reload.client_count()), 0);
    }

    #[test]
    fn test_default() {
        let reload = AutoReload::default();
        assert_eq!(tokio_test::block_on(reload.client_count()), 0);
    }
}
