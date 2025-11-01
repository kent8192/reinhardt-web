//! WebSocket rate limiting and throttling
//!
//! This module provides rate limiting capabilities for WebSocket connections,
//! preventing abuse and ensuring fair resource usage.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Throttling errors
#[derive(Debug, thiserror::Error)]
pub enum ThrottleError {
	#[error("Rate limit exceeded: {0}")]
	RateLimitExceeded(String),
	#[error("Too many connections from {0}")]
	TooManyConnections(String),
}

/// Result type for throttling operations
pub type ThrottleResult<T> = Result<T, ThrottleError>;

/// Rate limit configuration
///
/// # Examples
///
/// ```
/// use reinhardt_websockets::throttling::RateLimitConfig;
/// use std::time::Duration;
///
/// let config = RateLimitConfig::new(100, Duration::from_secs(60));
/// assert_eq!(config.max_requests(), 100);
/// assert_eq!(config.window(), Duration::from_secs(60));
/// ```
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
	max_requests: usize,
	window: Duration,
}

impl RateLimitConfig {
	/// Create a new rate limit configuration
	///
	/// # Arguments
	///
	/// * `max_requests` - Maximum number of requests allowed
	/// * `window` - Time window for the rate limit
	pub fn new(max_requests: usize, window: Duration) -> Self {
		Self {
			max_requests,
			window,
		}
	}

	/// Get maximum requests allowed
	pub fn max_requests(&self) -> usize {
		self.max_requests
	}

	/// Get time window
	pub fn window(&self) -> Duration {
		self.window
	}

	/// Create a permissive rate limit (high limit)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::throttling::RateLimitConfig;
	///
	/// let config = RateLimitConfig::permissive();
	/// assert_eq!(config.max_requests(), 10000);
	/// ```
	pub fn permissive() -> Self {
		Self::new(10000, Duration::from_secs(60))
	}

	/// Create a strict rate limit (low limit)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::throttling::RateLimitConfig;
	///
	/// let config = RateLimitConfig::strict();
	/// assert_eq!(config.max_requests(), 10);
	/// ```
	pub fn strict() -> Self {
		Self::new(10, Duration::from_secs(60))
	}
}

/// Request counter for tracking rate limits
#[derive(Debug)]
struct RequestCounter {
	count: usize,
	window_start: Instant,
}

impl RequestCounter {
	fn new() -> Self {
		Self {
			count: 0,
			window_start: Instant::now(),
		}
	}

	fn increment(&mut self, config: &RateLimitConfig) -> ThrottleResult<()> {
		let elapsed = self.window_start.elapsed();

		if elapsed >= config.window {
			// Reset window
			self.count = 1;
			self.window_start = Instant::now();
			Ok(())
		} else if self.count < config.max_requests {
			self.count += 1;
			Ok(())
		} else {
			Err(ThrottleError::RateLimitExceeded(format!(
				"Exceeded {} requests per {:?}",
				config.max_requests, config.window
			)))
		}
	}

	fn reset(&mut self) {
		self.count = 0;
		self.window_start = Instant::now();
	}
}

/// Rate limiter for WebSocket connections
///
/// # Examples
///
/// ```
/// use reinhardt_websockets::throttling::{RateLimiter, RateLimitConfig};
/// use std::time::Duration;
///
/// # tokio_test::block_on(async {
/// let config = RateLimitConfig::new(5, Duration::from_secs(10));
/// let limiter = RateLimiter::new(config);
///
/// // First 5 requests should succeed
/// for _ in 0..5 {
///     assert!(limiter.check_rate_limit("user_1").await.is_ok());
/// }
///
/// // 6th request should fail
/// assert!(limiter.check_rate_limit("user_1").await.is_err());
/// # });
/// ```
pub struct RateLimiter {
	config: RateLimitConfig,
	counters: Arc<RwLock<HashMap<String, RequestCounter>>>,
}

impl RateLimiter {
	/// Create a new rate limiter
	pub fn new(config: RateLimitConfig) -> Self {
		Self {
			config,
			counters: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	/// Check if a client is within rate limits
	///
	/// # Arguments
	///
	/// * `client_id` - Unique identifier for the client
	pub async fn check_rate_limit(&self, client_id: &str) -> ThrottleResult<()> {
		let mut counters = self.counters.write().await;

		let counter = counters
			.entry(client_id.to_string())
			.or_insert_with(RequestCounter::new);

		counter.increment(&self.config)
	}

	/// Reset rate limit for a specific client
	pub async fn reset_client(&self, client_id: &str) {
		let mut counters = self.counters.write().await;
		if let Some(counter) = counters.get_mut(client_id) {
			counter.reset();
		}
	}

	/// Clear all rate limit counters
	pub async fn clear_all(&self) {
		let mut counters = self.counters.write().await;
		counters.clear();
	}

	/// Get current request count for a client
	pub async fn get_count(&self, client_id: &str) -> usize {
		let counters = self.counters.read().await;
		counters
			.get(client_id)
			.map(|counter| counter.count)
			.unwrap_or(0)
	}
}

/// Connection throttler for limiting concurrent connections
///
/// # Examples
///
/// ```
/// use reinhardt_websockets::throttling::ConnectionThrottler;
///
/// # tokio_test::block_on(async {
/// let throttler = ConnectionThrottler::new(3);
///
/// // First 3 connections should succeed
/// assert!(throttler.acquire_connection("192.168.1.1").await.is_ok());
/// assert!(throttler.acquire_connection("192.168.1.1").await.is_ok());
/// assert!(throttler.acquire_connection("192.168.1.1").await.is_ok());
///
/// // 4th connection should fail
/// assert!(throttler.acquire_connection("192.168.1.1").await.is_err());
///
/// // After releasing one, should succeed again
/// throttler.release_connection("192.168.1.1").await;
/// assert!(throttler.acquire_connection("192.168.1.1").await.is_ok());
/// # });
/// ```
pub struct ConnectionThrottler {
	max_connections_per_ip: usize,
	connections: Arc<RwLock<HashMap<String, usize>>>,
}

impl ConnectionThrottler {
	/// Create a new connection throttler
	///
	/// # Arguments
	///
	/// * `max_connections_per_ip` - Maximum concurrent connections per IP address
	pub fn new(max_connections_per_ip: usize) -> Self {
		Self {
			max_connections_per_ip,
			connections: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	/// Acquire a connection slot for an IP address
	pub async fn acquire_connection(&self, ip: &str) -> ThrottleResult<()> {
		let mut connections = self.connections.write().await;

		let count = connections.entry(ip.to_string()).or_insert(0);

		if *count >= self.max_connections_per_ip {
			Err(ThrottleError::TooManyConnections(ip.to_string()))
		} else {
			*count += 1;
			Ok(())
		}
	}

	/// Release a connection slot for an IP address
	pub async fn release_connection(&self, ip: &str) {
		let mut connections = self.connections.write().await;

		if let Some(count) = connections.get_mut(ip) {
			if *count > 0 {
				*count -= 1;
			}
			if *count == 0 {
				connections.remove(ip);
			}
		}
	}

	/// Get current connection count for an IP address
	pub async fn get_connection_count(&self, ip: &str) -> usize {
		let connections = self.connections.read().await;
		connections.get(ip).copied().unwrap_or(0)
	}

	/// Clear all connection counts
	pub async fn clear_all(&self) {
		let mut connections = self.connections.write().await;
		connections.clear();
	}
}

/// Combined throttler with both rate limiting and connection throttling
///
/// # Examples
///
/// ```
/// use reinhardt_websockets::throttling::{CombinedThrottler, RateLimitConfig};
/// use std::time::Duration;
///
/// # tokio_test::block_on(async {
/// let throttler = CombinedThrottler::new(
///     RateLimitConfig::new(100, Duration::from_secs(60)),
///     10,
/// );
///
/// // Check connection limit
/// assert!(throttler.check_connection("192.168.1.1").await.is_ok());
///
/// // Check message rate limit
/// assert!(throttler.check_message_rate("user_1").await.is_ok());
/// # });
/// ```
pub struct CombinedThrottler {
	rate_limiter: RateLimiter,
	connection_throttler: ConnectionThrottler,
}

impl CombinedThrottler {
	/// Create a new combined throttler
	pub fn new(rate_config: RateLimitConfig, max_connections_per_ip: usize) -> Self {
		Self {
			rate_limiter: RateLimiter::new(rate_config),
			connection_throttler: ConnectionThrottler::new(max_connections_per_ip),
		}
	}

	/// Check if a connection is allowed
	pub async fn check_connection(&self, ip: &str) -> ThrottleResult<()> {
		self.connection_throttler.acquire_connection(ip).await
	}

	/// Release a connection
	pub async fn release_connection(&self, ip: &str) {
		self.connection_throttler.release_connection(ip).await
	}

	/// Check message rate limit
	pub async fn check_message_rate(&self, client_id: &str) -> ThrottleResult<()> {
		self.rate_limiter.check_rate_limit(client_id).await
	}

	/// Reset client rate limit
	pub async fn reset_client_rate(&self, client_id: &str) {
		self.rate_limiter.reset_client(client_id).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_rate_limit_config() {
		let config = RateLimitConfig::new(100, Duration::from_secs(60));
		assert_eq!(config.max_requests(), 100);
		assert_eq!(config.window(), Duration::from_secs(60));
	}

	#[test]
	fn test_rate_limit_config_presets() {
		let permissive = RateLimitConfig::permissive();
		assert_eq!(permissive.max_requests(), 10000);

		let strict = RateLimitConfig::strict();
		assert_eq!(strict.max_requests(), 10);
	}

	#[tokio::test]
	async fn test_rate_limiter_within_limit() {
		let config = RateLimitConfig::new(5, Duration::from_secs(10));
		let limiter = RateLimiter::new(config);

		for _ in 0..5 {
			assert!(limiter.check_rate_limit("user_1").await.is_ok());
		}
	}

	#[tokio::test]
	async fn test_rate_limiter_exceeds_limit() {
		let config = RateLimitConfig::new(3, Duration::from_secs(10));
		let limiter = RateLimiter::new(config);

		for _ in 0..3 {
			limiter.check_rate_limit("user_1").await.unwrap();
		}

		let result = limiter.check_rate_limit("user_1").await;
		assert!(result.is_err());
		assert!(matches!(
			result.unwrap_err(),
			ThrottleError::RateLimitExceeded(_)
		));
	}

	#[tokio::test]
	async fn test_rate_limiter_reset() {
		let config = RateLimitConfig::new(2, Duration::from_secs(10));
		let limiter = RateLimiter::new(config);

		limiter.check_rate_limit("user_1").await.unwrap();
		limiter.check_rate_limit("user_1").await.unwrap();

		// Should fail without reset
		assert!(limiter.check_rate_limit("user_1").await.is_err());

		// After reset, should succeed
		limiter.reset_client("user_1").await;
		assert!(limiter.check_rate_limit("user_1").await.is_ok());
	}

	#[tokio::test]
	async fn test_rate_limiter_get_count() {
		let config = RateLimitConfig::new(10, Duration::from_secs(10));
		let limiter = RateLimiter::new(config);

		assert_eq!(limiter.get_count("user_1").await, 0);

		limiter.check_rate_limit("user_1").await.unwrap();
		limiter.check_rate_limit("user_1").await.unwrap();

		assert_eq!(limiter.get_count("user_1").await, 2);
	}

	#[tokio::test]
	async fn test_connection_throttler_within_limit() {
		let throttler = ConnectionThrottler::new(3);

		assert!(throttler.acquire_connection("192.168.1.1").await.is_ok());
		assert!(throttler.acquire_connection("192.168.1.1").await.is_ok());
		assert!(throttler.acquire_connection("192.168.1.1").await.is_ok());
	}

	#[tokio::test]
	async fn test_connection_throttler_exceeds_limit() {
		let throttler = ConnectionThrottler::new(2);

		throttler.acquire_connection("192.168.1.1").await.unwrap();
		throttler.acquire_connection("192.168.1.1").await.unwrap();

		let result = throttler.acquire_connection("192.168.1.1").await;
		assert!(result.is_err());
		assert!(matches!(
			result.unwrap_err(),
			ThrottleError::TooManyConnections(_)
		));
	}

	#[tokio::test]
	async fn test_connection_throttler_release() {
		let throttler = ConnectionThrottler::new(2);

		throttler.acquire_connection("192.168.1.1").await.unwrap();
		throttler.acquire_connection("192.168.1.1").await.unwrap();

		// Should fail
		assert!(throttler.acquire_connection("192.168.1.1").await.is_err());

		// After release, should succeed
		throttler.release_connection("192.168.1.1").await;
		assert!(throttler.acquire_connection("192.168.1.1").await.is_ok());
	}

	#[tokio::test]
	async fn test_connection_throttler_get_count() {
		let throttler = ConnectionThrottler::new(10);

		assert_eq!(throttler.get_connection_count("192.168.1.1").await, 0);

		throttler.acquire_connection("192.168.1.1").await.unwrap();
		throttler.acquire_connection("192.168.1.1").await.unwrap();

		assert_eq!(throttler.get_connection_count("192.168.1.1").await, 2);
	}

	#[tokio::test]
	async fn test_combined_throttler() {
		let config = RateLimitConfig::new(10, Duration::from_secs(10));
		let throttler = CombinedThrottler::new(config, 5);

		// Check connection
		assert!(throttler.check_connection("192.168.1.1").await.is_ok());

		// Check message rate
		assert!(throttler.check_message_rate("user_1").await.is_ok());

		// Release connection
		throttler.release_connection("192.168.1.1").await;

		// Reset rate
		throttler.reset_client_rate("user_1").await;
	}
}
