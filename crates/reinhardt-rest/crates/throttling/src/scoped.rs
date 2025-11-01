use crate::backend::{MemoryBackend, ThrottleBackend};
use crate::throttle::{Throttle, ThrottleResult};
use async_trait::async_trait;
use std::collections::HashMap;

pub struct ScopedRateThrottle<B: ThrottleBackend = MemoryBackend> {
	pub scopes: HashMap<String, (usize, u64)>,
	backend: B,
}

impl ScopedRateThrottle<MemoryBackend> {
	/// Creates a new `ScopedRateThrottle` with a default memory backend.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_throttling::ScopedRateThrottle;
	///
	/// let throttle = ScopedRateThrottle::new();
	/// assert_eq!(throttle.scopes.len(), 0);
	/// ```
	pub fn new() -> Self {
		Self {
			scopes: HashMap::new(),
			backend: MemoryBackend::new(),
		}
	}
}

impl Default for ScopedRateThrottle<MemoryBackend> {
	fn default() -> Self {
		Self::new()
	}
}

impl<B: ThrottleBackend> ScopedRateThrottle<B> {
	/// Creates a new `ScopedRateThrottle` with a custom backend.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_throttling::{ScopedRateThrottle, MemoryBackend};
	///
	/// let backend = MemoryBackend::new();
	/// let throttle = ScopedRateThrottle::with_backend(backend);
	/// assert_eq!(throttle.scopes.len(), 0);
	/// ```
	pub fn with_backend(backend: B) -> Self {
		Self {
			scopes: HashMap::new(),
			backend,
		}
	}

	/// Add a scope with rate limit configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_throttling::ScopedRateThrottle;
	///
	/// let throttle = ScopedRateThrottle::new()
	///     .add_scope("api", 100, 60)
	///     .add_scope("upload", 10, 60);
	/// assert_eq!(throttle.scopes.len(), 2);
	/// assert_eq!(throttle.scopes.get("api"), Some(&(100, 60)));
	/// assert_eq!(throttle.scopes.get("upload"), Some(&(10, 60)));
	/// ```
	pub fn add_scope(mut self, scope: impl Into<String>, rate: usize, window: u64) -> Self {
		self.scopes.insert(scope.into(), (rate, window));
		self
	}
}

#[async_trait]
impl<B: ThrottleBackend> Throttle for ScopedRateThrottle<B> {
	async fn allow_request(&self, scope_key: &str) -> ThrottleResult<bool> {
		let parts: Vec<&str> = scope_key.splitn(2, ':').collect();
		if parts.len() != 2 {
			return Ok(true);
		}
		let (scope, identifier) = (parts[0], parts[1]);
		if let Some(&(rate, window)) = self.scopes.get(scope) {
			let key = format!("throttle:scope:{}:{}", scope, identifier);
			let count = self
				.backend
				.increment(&key, window)
				.await
				.map_err(crate::throttle::ThrottleError::ThrottleError)?;
			Ok(count <= rate)
		} else {
			Ok(true)
		}
	}
	async fn wait_time(&self, scope_key: &str) -> ThrottleResult<Option<u64>> {
		let parts: Vec<&str> = scope_key.splitn(2, ':').collect();
		if parts.len() != 2 {
			return Ok(None);
		}
		let (scope, identifier) = (parts[0], parts[1]);
		if let Some(&(rate, window)) = self.scopes.get(scope) {
			let key = format!("throttle:scope:{}:{}", scope, identifier);
			let count = self
				.backend
				.get_count(&key)
				.await
				.map_err(crate::throttle::ThrottleError::ThrottleError)?;
			if count > rate {
				Ok(Some(window))
			} else {
				Ok(None)
			}
		} else {
			Ok(None)
		}
	}
	fn get_rate(&self) -> (usize, u64) {
		(0, 0)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_scoped_throttle() {
		let throttle = ScopedRateThrottle::new()
			.add_scope("api", 100, 60)
			.add_scope("upload", 10, 60);
		for _ in 0..10 {
			assert!(throttle.allow_request("upload:user1").await.unwrap());
		}
		assert!(!throttle.allow_request("upload:user1").await.unwrap());
		assert!(throttle.allow_request("api:user1").await.unwrap());
	}

	#[tokio::test]
	async fn test_scoped_rate_throttle() {
		let throttle = ScopedRateThrottle::new()
			.add_scope("x", 3, 60)
			.add_scope("y", 1, 60);

		// Should be able to hit x scope 3 times
		assert!(throttle.allow_request("x:req1").await.unwrap());
		assert!(throttle.allow_request("x:req1").await.unwrap());
		assert!(throttle.allow_request("x:req1").await.unwrap());

		// Fourth request should be throttled
		assert!(!throttle.allow_request("x:req1").await.unwrap());

		// Should be able to hit y scope 1 time
		assert!(throttle.allow_request("y:req1").await.unwrap());
		assert!(!throttle.allow_request("y:req1").await.unwrap());

		// Different identifier should have separate limit
		assert!(throttle.allow_request("x:req2").await.unwrap());
		assert!(throttle.allow_request("y:req2").await.unwrap());
	}

	#[tokio::test]
	async fn test_scoped_rate_throttle_with_time() {
		use crate::backend::MemoryBackend;
		use crate::time_provider::MockTimeProvider;
		use std::sync::Arc;
		use tokio::time::Instant;

		let mock_time = Arc::new(MockTimeProvider::new(Instant::now()));
		let backend = MemoryBackend::with_time_provider(mock_time.clone());

		let mut throttle = ScopedRateThrottle::with_backend(backend);
		throttle = throttle.add_scope("api", 2, 5);

		// Fill up the limit
		assert!(throttle.allow_request("api:user1").await.unwrap());
		assert!(throttle.allow_request("api:user1").await.unwrap());

		// Should be throttled
		assert!(!throttle.allow_request("api:user1").await.unwrap());

		// Advance time past the window
		mock_time.advance(std::time::Duration::from_secs(6));

		// Should be allowed again after window expires
		assert!(throttle.allow_request("api:user1").await.unwrap());
		assert!(throttle.allow_request("api:user1").await.unwrap());

		// Third request should be throttled
		assert!(!throttle.allow_request("api:user1").await.unwrap());
	}

	#[tokio::test]
	async fn test_unscoped_view_not_throttled() {
		let throttle = ScopedRateThrottle::new().add_scope("x", 3, 60);

		// Requests without scope should not be throttled
		for _ in 0..10 {
			assert!(throttle.allow_request("unknown:req1").await.unwrap());
		}

		// Requests with invalid format should not be throttled
		for _ in 0..10 {
			assert!(throttle.allow_request("invalid_format").await.unwrap());
		}
	}

	#[tokio::test]
	async fn test_get_cache_key_returns_correct_key() {
		let throttle = ScopedRateThrottle::new().add_scope("user", 10, 60);

		// Test that different users get different keys
		assert!(throttle.allow_request("user:123").await.unwrap());
		assert!(throttle.allow_request("user:456").await.unwrap());

		// Verify they are tracked separately
		for _ in 0..9 {
			assert!(throttle.allow_request("user:123").await.unwrap());
		}
		// user:123 should now be at limit
		assert!(!throttle.allow_request("user:123").await.unwrap());

		// user:456 should still have capacity
		assert!(throttle.allow_request("user:456").await.unwrap());
	}
}
