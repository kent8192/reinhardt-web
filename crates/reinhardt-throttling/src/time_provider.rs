use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::time::Instant;

/// Trait for providing time information to throttle backends.
/// This allows for time mocking in tests.
#[async_trait]
pub trait TimeProvider: Send + Sync {
	fn now(&self) -> Instant;
}

/// System time provider that uses the actual system clock.
#[derive(Clone, Default)]
pub struct SystemTimeProvider;

impl SystemTimeProvider {
	pub fn new() -> Self {
		Self
	}
}

#[async_trait]
impl TimeProvider for SystemTimeProvider {
	fn now(&self) -> Instant {
		Instant::now()
	}
}

/// Mock time provider for testing that allows manual time control.
#[derive(Clone)]
pub struct MockTimeProvider {
	current_time: Arc<RwLock<Instant>>,
}

impl MockTimeProvider {
	pub fn new(start_time: Instant) -> Self {
		Self {
			current_time: Arc::new(RwLock::new(start_time)),
		}
	}

	pub fn advance(&self, duration: std::time::Duration) {
		let mut time = self.current_time.write();
		*time += duration;
	}

	pub fn set_time(&self, time: Instant) {
		let mut current = self.current_time.write();
		*current = time;
	}
}

impl Default for MockTimeProvider {
	fn default() -> Self {
		Self::new(Instant::now())
	}
}

#[async_trait]
impl TimeProvider for MockTimeProvider {
	fn now(&self) -> Instant {
		*self.current_time.read()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use std::time::Duration;

	#[rstest]
	fn test_system_time_provider_returns_current_time() {
		let provider = SystemTimeProvider::new();
		let time1 = provider.now();
		std::thread::sleep(Duration::from_millis(10));
		let time2 = provider.now();
		assert!(time2 > time1);
	}

	#[rstest]
	fn test_mock_time_provider_allows_time_control() {
		let start = Instant::now();
		let provider = MockTimeProvider::new(start);

		let time1 = provider.now();
		assert_eq!(time1, start);

		provider.advance(Duration::from_secs(60));
		let time2 = provider.now();
		assert_eq!(time2, start + Duration::from_secs(60));
	}

	#[rstest]
	fn test_mock_time_provider_set_time() {
		let provider = MockTimeProvider::new(Instant::now());
		let new_time = Instant::now() + Duration::from_secs(100);
		provider.set_time(new_time);
		assert_eq!(provider.now(), new_time);
	}
}
