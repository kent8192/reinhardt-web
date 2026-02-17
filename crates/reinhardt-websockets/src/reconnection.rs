//! WebSocket automatic reconnection support
//!
//! This module provides automatic reconnection functionality when WebSocket connections are disconnected.
//! It uses an exponential backoff algorithm to adjust retry intervals.
//!
//! ## Usage Example
//!
//! ```
//! use reinhardt_websockets::reconnection::{ReconnectionConfig, ReconnectionStrategy};
//! use std::time::Duration;
//!
//! let config = ReconnectionConfig::default()
//!     .with_max_attempts(5)
//!     .with_initial_delay(Duration::from_secs(1))
//!     .with_max_delay(Duration::from_secs(60));
//!
//! let mut strategy = ReconnectionStrategy::new(config);
//!
//! // On connection failure
//! if let Some(delay) = strategy.next_delay() {
//!     println!("Retrying in {:?}", delay);
//! }
//! ```

use std::time::Duration;

/// Reconnection configuration
#[derive(Debug, Clone)]
pub struct ReconnectionConfig {
	/// Maximum number of reconnection attempts (None for unlimited)
	pub max_attempts: Option<u32>,
	/// Initial reconnection delay
	pub initial_delay: Duration,
	/// Maximum delay time
	pub max_delay: Duration,
	/// Backoff multiplier (default: 2.0)
	pub backoff_multiplier: f64,
	/// Jitter factor (default: 0.1 = 10%)
	pub jitter_factor: f64,
}

impl Default for ReconnectionConfig {
	/// Creates a default reconnection configuration.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::reconnection::ReconnectionConfig;
	/// use std::time::Duration;
	///
	/// let config = ReconnectionConfig::default();
	/// assert_eq!(config.max_attempts, Some(10));
	/// assert_eq!(config.initial_delay, Duration::from_secs(1));
	/// assert_eq!(config.max_delay, Duration::from_secs(300));
	/// assert_eq!(config.backoff_multiplier, 2.0);
	/// assert_eq!(config.jitter_factor, 0.1);
	/// ```
	fn default() -> Self {
		Self {
			max_attempts: Some(10),
			initial_delay: Duration::from_secs(1),
			max_delay: Duration::from_secs(300), // 5 minutes
			backoff_multiplier: 2.0,
			jitter_factor: 0.1,
		}
	}
}

impl ReconnectionConfig {
	/// Creates a new reconnection configuration.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::reconnection::ReconnectionConfig;
	/// use std::time::Duration;
	///
	/// let config = ReconnectionConfig::new(
	///     Some(5),
	///     Duration::from_secs(2),
	///     Duration::from_secs(60),
	/// );
	/// assert_eq!(config.max_attempts, Some(5));
	/// assert_eq!(config.initial_delay, Duration::from_secs(2));
	/// assert_eq!(config.max_delay, Duration::from_secs(60));
	/// ```
	pub fn new(max_attempts: Option<u32>, initial_delay: Duration, max_delay: Duration) -> Self {
		Self {
			max_attempts,
			initial_delay,
			max_delay,
			backoff_multiplier: 2.0,
			jitter_factor: 0.1,
		}
	}

	/// Sets the maximum number of reconnection attempts.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::reconnection::ReconnectionConfig;
	///
	/// let config = ReconnectionConfig::default()
	///     .with_max_attempts(5);
	/// assert_eq!(config.max_attempts, Some(5));
	/// ```
	pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
		self.max_attempts = Some(max_attempts);
		self
	}

	/// Sets unlimited reconnection attempts.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::reconnection::ReconnectionConfig;
	///
	/// let config = ReconnectionConfig::default()
	///     .with_unlimited_attempts();
	/// assert_eq!(config.max_attempts, None);
	/// ```
	pub fn with_unlimited_attempts(mut self) -> Self {
		self.max_attempts = None;
		self
	}

	/// Sets the initial reconnection delay.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::reconnection::ReconnectionConfig;
	/// use std::time::Duration;
	///
	/// let config = ReconnectionConfig::default()
	///     .with_initial_delay(Duration::from_secs(2));
	/// assert_eq!(config.initial_delay, Duration::from_secs(2));
	/// ```
	pub fn with_initial_delay(mut self, delay: Duration) -> Self {
		self.initial_delay = delay;
		self
	}

	/// Sets the maximum delay time.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::reconnection::ReconnectionConfig;
	/// use std::time::Duration;
	///
	/// let config = ReconnectionConfig::default()
	///     .with_max_delay(Duration::from_secs(120));
	/// assert_eq!(config.max_delay, Duration::from_secs(120));
	/// ```
	pub fn with_max_delay(mut self, delay: Duration) -> Self {
		self.max_delay = delay;
		self
	}

	/// Sets the backoff multiplier.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::reconnection::ReconnectionConfig;
	///
	/// let config = ReconnectionConfig::default()
	///     .with_backoff_multiplier(1.5);
	/// assert_eq!(config.backoff_multiplier, 1.5);
	/// ```
	pub fn with_backoff_multiplier(mut self, multiplier: f64) -> Self {
		self.backoff_multiplier = multiplier;
		self
	}

	/// Sets the jitter factor.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::reconnection::ReconnectionConfig;
	///
	/// let config = ReconnectionConfig::default()
	///     .with_jitter_factor(0.2);
	/// assert_eq!(config.jitter_factor, 0.2);
	/// ```
	pub fn with_jitter_factor(mut self, factor: f64) -> Self {
		self.jitter_factor = factor;
		self
	}
}

/// Reconnection strategy
pub struct ReconnectionStrategy {
	config: ReconnectionConfig,
	current_attempt: u32,
	current_delay: Duration,
}

impl ReconnectionStrategy {
	/// Creates a new reconnection strategy.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::reconnection::{ReconnectionConfig, ReconnectionStrategy};
	///
	/// let config = ReconnectionConfig::default();
	/// let strategy = ReconnectionStrategy::new(config);
	/// assert_eq!(strategy.attempt_count(), 0);
	/// ```
	pub fn new(config: ReconnectionConfig) -> Self {
		let current_delay = config.initial_delay;
		Self {
			config,
			current_attempt: 0,
			current_delay,
		}
	}

	/// Returns the current attempt count.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::reconnection::{ReconnectionConfig, ReconnectionStrategy};
	///
	/// let mut strategy = ReconnectionStrategy::new(ReconnectionConfig::default());
	/// assert_eq!(strategy.attempt_count(), 0);
	///
	/// strategy.next_delay();
	/// assert_eq!(strategy.attempt_count(), 1);
	/// ```
	pub fn attempt_count(&self) -> u32 {
		self.current_attempt
	}

	/// Returns the next reconnection delay.
	///
	/// Returns None if the maximum number of attempts has been reached.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::reconnection::{ReconnectionConfig, ReconnectionStrategy};
	/// use std::time::Duration;
	///
	/// let config = ReconnectionConfig::default()
	///     .with_max_attempts(2)
	///     .with_initial_delay(Duration::from_secs(1));
	///
	/// let mut strategy = ReconnectionStrategy::new(config);
	///
	/// // First attempt
	/// let delay1 = strategy.next_delay();
	/// assert!(delay1.is_some());
	///
	/// // Second attempt
	/// let delay2 = strategy.next_delay();
	/// assert!(delay2.is_some());
	///
	/// // Third attempt (exceeds max attempts)
	/// let delay3 = strategy.next_delay();
	/// assert!(delay3.is_none());
	/// ```
	pub fn next_delay(&mut self) -> Option<Duration> {
		// Check maximum attempt count
		if let Some(max) = self.config.max_attempts
			&& self.current_attempt >= max
		{
			return None;
		}

		let delay = if self.current_attempt == 0 {
			self.config.initial_delay
		} else {
			self.current_delay
		};

		// Apply jitter (±jitter_factor)
		let jitter = self.apply_jitter(delay);

		self.current_attempt += 1;

		// Calculate next delay (exponential backoff)
		let next_delay_secs = delay.as_secs_f64() * self.config.backoff_multiplier;
		let next_delay =
			Duration::from_secs_f64(next_delay_secs.min(self.config.max_delay.as_secs_f64()));
		self.current_delay = next_delay;

		Some(jitter)
	}

	/// Returns the delay with applied jitter.
	fn apply_jitter(&self, delay: Duration) -> Duration {
		use std::collections::hash_map::RandomState;
		use std::hash::BuildHasher;

		// Simple pseudo-random value generation (for testability)
		let hash = RandomState::new().hash_one(self.current_attempt);
		let random = (hash % 1000) as f64 / 1000.0; // 0.0 ~ 1.0

		let jitter_range = delay.as_secs_f64() * self.config.jitter_factor;
		let jitter = (random - 0.5) * 2.0 * jitter_range; // -jitter_range ~ +jitter_range

		let final_delay = (delay.as_secs_f64() + jitter).max(0.0);
		Duration::from_secs_f64(final_delay)
	}

	/// Resets the strategy.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::reconnection::{ReconnectionConfig, ReconnectionStrategy};
	///
	/// let mut strategy = ReconnectionStrategy::new(ReconnectionConfig::default());
	///
	/// strategy.next_delay();
	/// strategy.next_delay();
	/// assert_eq!(strategy.attempt_count(), 2);
	///
	/// strategy.reset();
	/// assert_eq!(strategy.attempt_count(), 0);
	/// ```
	pub fn reset(&mut self) {
		self.current_attempt = 0;
		self.current_delay = self.config.initial_delay;
	}

	/// Returns whether reconnection is possible.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::reconnection::{ReconnectionConfig, ReconnectionStrategy};
	///
	/// let config = ReconnectionConfig::default().with_max_attempts(1);
	/// let mut strategy = ReconnectionStrategy::new(config);
	///
	/// assert!(strategy.can_reconnect());
	/// strategy.next_delay();
	/// assert!(!strategy.can_reconnect());
	/// ```
	pub fn can_reconnect(&self) -> bool {
		if let Some(max) = self.config.max_attempts {
			self.current_attempt < max
		} else {
			true
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_default_config() {
		let config = ReconnectionConfig::default();
		assert_eq!(config.max_attempts, Some(10));
		assert_eq!(config.initial_delay, Duration::from_secs(1));
		assert_eq!(config.max_delay, Duration::from_secs(300));
		assert_eq!(config.backoff_multiplier, 2.0);
		assert_eq!(config.jitter_factor, 0.1);
	}

	#[rstest]
	fn test_config_builder() {
		let config = ReconnectionConfig::default()
			.with_max_attempts(5)
			.with_initial_delay(Duration::from_secs(2))
			.with_max_delay(Duration::from_secs(60))
			.with_backoff_multiplier(1.5)
			.with_jitter_factor(0.2);

		assert_eq!(config.max_attempts, Some(5));
		assert_eq!(config.initial_delay, Duration::from_secs(2));
		assert_eq!(config.max_delay, Duration::from_secs(60));
		assert_eq!(config.backoff_multiplier, 1.5);
		assert_eq!(config.jitter_factor, 0.2);
	}

	#[rstest]
	fn test_unlimited_attempts() {
		let config = ReconnectionConfig::default().with_unlimited_attempts();
		assert_eq!(config.max_attempts, None);
	}

	#[rstest]
	fn test_reconnection_strategy() {
		let config = ReconnectionConfig::default()
			.with_max_attempts(3)
			.with_initial_delay(Duration::from_secs(1))
			.with_jitter_factor(0.0); // No jitter

		let mut strategy = ReconnectionStrategy::new(config);

		assert_eq!(strategy.attempt_count(), 0);
		assert!(strategy.can_reconnect());

		// First: returns initial_delay (1 second)
		// Updates current_delay to 1 * 2.0 = 2 seconds for next time
		let delay1 = strategy.next_delay().unwrap();
		assert_eq!(delay1, Duration::from_secs(1));
		assert_eq!(strategy.attempt_count(), 1);

		// Second: returns current_delay (2 seconds)
		// Updates current_delay to 2 * 2.0 = 4 seconds for next time
		let delay2 = strategy.next_delay().unwrap();
		assert_eq!(delay2, Duration::from_secs(2));
		assert_eq!(strategy.attempt_count(), 2);

		// Third: returns current_delay (4 seconds)
		let delay3 = strategy.next_delay().unwrap();
		assert_eq!(delay3, Duration::from_secs(4));
		assert_eq!(strategy.attempt_count(), 3);

		// Fourth (exceeds max attempts)
		let delay4 = strategy.next_delay();
		assert!(delay4.is_none());
		assert!(!strategy.can_reconnect());
	}

	#[rstest]
	fn test_exponential_backoff() {
		let config = ReconnectionConfig::default()
			.with_unlimited_attempts()
			.with_initial_delay(Duration::from_secs(1))
			.with_backoff_multiplier(2.0)
			.with_max_delay(Duration::from_secs(100))
			.with_jitter_factor(0.0);

		let mut strategy = ReconnectionStrategy::new(config);

		let delay1 = strategy.next_delay().unwrap();
		assert_eq!(delay1, Duration::from_secs(1));

		let delay2 = strategy.next_delay().unwrap();
		// After 1 second, doubled
		assert!(delay2.as_secs() >= 1);

		let delay3 = strategy.next_delay().unwrap();
		// Doubled again
		assert!(delay3.as_secs() >= 2);
	}

	#[rstest]
	fn test_max_delay_cap() {
		let config = ReconnectionConfig::default()
			.with_unlimited_attempts()
			.with_initial_delay(Duration::from_secs(1))
			.with_backoff_multiplier(10.0)
			.with_max_delay(Duration::from_secs(5))
			.with_jitter_factor(0.0);

		let mut strategy = ReconnectionStrategy::new(config);

		// Verify that the delay is capped at the maximum delay time after several executions
		for _ in 0..10 {
			if let Some(delay) = strategy.next_delay() {
				assert!(delay.as_secs() <= 5);
			}
		}
	}

	#[rstest]
	fn test_reset() {
		let config = ReconnectionConfig::default().with_max_attempts(5);
		let mut strategy = ReconnectionStrategy::new(config);

		strategy.next_delay();
		strategy.next_delay();
		assert_eq!(strategy.attempt_count(), 2);

		strategy.reset();
		assert_eq!(strategy.attempt_count(), 0);
		assert!(strategy.can_reconnect());
	}

	#[rstest]
	fn test_jitter_applied() {
		let config = ReconnectionConfig::default()
			.with_initial_delay(Duration::from_secs(1))
			.with_jitter_factor(0.1);

		let mut strategy = ReconnectionStrategy::new(config);

		let delay = strategy.next_delay().unwrap();
		// Jitter is applied, so it's not exactly 1 second
		// However, it's within 1 second ±10%
		let delay_secs = delay.as_secs_f64();
		assert!((0.9..=1.1).contains(&delay_secs));
	}
}
