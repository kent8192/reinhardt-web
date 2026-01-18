//! Burst rate throttling

use crate::backend::ThrottleBackend;
use crate::throttle::{Throttle, ThrottleError, ThrottleResult};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Burst rate throttle with sustained rate
pub struct BurstRateThrottle<B: ThrottleBackend> {
	backend: Arc<Mutex<B>>,
	burst_rate: usize,
	sustained_rate: usize,
	burst_duration: std::time::Duration,
	sustained_duration: std::time::Duration,
}

impl<B: ThrottleBackend> BurstRateThrottle<B> {
	/// Creates a new burst rate throttle with separate burst and sustained rates.
	///
	/// # Arguments
	///
	/// * `backend` - The throttle backend to use for storing request counts
	/// * `burst_rate` - Maximum requests allowed in burst period
	/// * `sustained_rate` - Maximum requests allowed in sustained period
	/// * `burst_duration` - Duration for burst rate window
	/// * `sustained_duration` - Duration for sustained rate window
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::throttling::burst::BurstRateThrottle;
	/// use reinhardt_rest::throttling::backend::MemoryBackend;
	/// use std::sync::Arc;
	/// use tokio::sync::Mutex;
	/// use std::time::Duration;
	///
	/// let backend = Arc::new(Mutex::new(MemoryBackend::new()));
	/// let throttle = BurstRateThrottle::new(
	///     backend,
	///     10,  // 10 requests burst
	///     100, // 100 requests sustained
	///     Duration::from_secs(1),   // 1 second burst window
	///     Duration::from_secs(60),  // 60 second sustained window
	/// );
	/// ```
	pub fn new(
		backend: Arc<Mutex<B>>,
		burst_rate: usize,
		sustained_rate: usize,
		burst_duration: std::time::Duration,
		sustained_duration: std::time::Duration,
	) -> Self {
		Self {
			backend,
			burst_rate,
			sustained_rate,
			burst_duration,
			sustained_duration,
		}
	}
}

#[async_trait::async_trait]
impl<B: ThrottleBackend> Throttle for BurstRateThrottle<B> {
	async fn allow_request(&self, key: &str) -> ThrottleResult<bool> {
		let backend = self.backend.lock().await;

		let burst_key = format!("burst:{}", key);
		let sustained_key = format!("sustained:{}", key);

		// Check burst rate
		let burst_count = backend
			.get_count(&burst_key)
			.await
			.map_err(ThrottleError::ThrottleError)?;
		if burst_count >= self.burst_rate {
			return Ok(false);
		}

		// Check sustained rate
		let sustained_count = backend
			.get_count(&sustained_key)
			.await
			.map_err(ThrottleError::ThrottleError)?;
		if sustained_count >= self.sustained_rate {
			return Ok(false);
		}

		// Increment both counters
		backend
			.increment_duration(&burst_key, self.burst_duration)
			.await?;
		backend
			.increment_duration(&sustained_key, self.sustained_duration)
			.await?;

		Ok(true)
	}

	async fn wait_time(&self, key: &str) -> ThrottleResult<Option<u64>> {
		let backend = self.backend.lock().await;
		backend
			.get_wait_time(key)
			.await
			.map(|opt| opt.map(|d| d.as_secs()))
	}

	fn get_rate(&self) -> (usize, u64) {
		(self.sustained_rate, self.sustained_duration.as_secs())
	}
}
