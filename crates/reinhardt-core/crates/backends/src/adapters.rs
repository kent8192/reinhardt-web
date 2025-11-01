//! Adapters for integrating reinhardt-backends with other components

use crate::Backend;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::time::Duration;

/// Adapter trait for Throttle backends
#[async_trait]
pub trait ThrottleBackend: Send + Sync {
	async fn increment(&self, key: &str, window: u64) -> Result<usize, String>;
	async fn get_count(&self, key: &str) -> Result<usize, String>;

	async fn increment_duration(&self, key: &str, window: Duration) -> Result<usize, String> {
		self.increment(key, window.as_secs()).await
	}

	async fn get_wait_time(&self, _key: &str) -> Result<Option<Duration>, String> {
		Ok(None)
	}
}

/// Adapter that makes any Backend compatible with ThrottleBackend
pub struct ThrottleBackendAdapter<B: Backend> {
	backend: Arc<B>,
}

impl<B: Backend> ThrottleBackendAdapter<B> {
	pub fn new(backend: Arc<B>) -> Self {
		Self { backend }
	}
}

#[async_trait]
impl<B: Backend> ThrottleBackend for ThrottleBackendAdapter<B> {
	async fn increment(&self, key: &str, window: u64) -> Result<usize, String> {
		let count = self
			.backend
			.increment(key, Some(Duration::from_secs(window)))
			.await
			.map_err(|e| e.to_string())?;

		Ok(count as usize)
	}

	async fn get_count(&self, key: &str) -> Result<usize, String> {
		let value: Option<i64> = self.backend.get(key).await.map_err(|e| e.to_string())?;

		Ok(value.unwrap_or(0) as usize)
	}

	async fn get_wait_time(&self, _key: &str) -> Result<Option<Duration>, String> {
		// Check TTL if backend supports it
		// For now, return None (not implemented)
		Ok(None)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::MemoryBackend;

	#[tokio::test]
	async fn test_throttle_adapter_increment() {
		let backend = Arc::new(MemoryBackend::new());
		let adapter = ThrottleBackendAdapter::new(backend);

		let count1 = adapter.increment("test_key", 60).await.unwrap();
		assert_eq!(count1, 1);

		let count2 = adapter.increment("test_key", 60).await.unwrap();
		assert_eq!(count2, 2);
	}

	#[tokio::test]
	async fn test_throttle_adapter_get_count() {
		let backend = Arc::new(MemoryBackend::new());
		let adapter = ThrottleBackendAdapter::new(backend);

		let initial = adapter.get_count("test_key").await.unwrap();
		assert_eq!(initial, 0);

		adapter.increment("test_key", 60).await.unwrap();

		let count = adapter.get_count("test_key").await.unwrap();
		assert_eq!(count, 1);
	}
}
