//! Throttling integration tests
//!
//! This module tests the integration of various throttling algorithms with MemoryBackend,
//! focusing on real-world usage patterns and time-based behavior simulation.

use reinhardt_throttling::{
	AnonRateThrottle, ScopedRateThrottle, Throttle, UserRateThrottle,
	adaptive::{AdaptiveConfig, AdaptiveThrottle, LoadMetrics},
	backend::{MemoryBackend, ThrottleBackend},
	leaky_bucket::{LeakyBucketConfig, LeakyBucketThrottle},
	time_provider::MockTimeProvider,
	token_bucket::{TokenBucket, TokenBucketConfig},
};
use rstest::*;
use std::sync::Arc;
use tokio::time::Instant;

/// Test TokenBucket algorithm integration with MemoryBackend
#[rstest]
#[tokio::test]
async fn test_token_bucket_algorithm_integration() {
	let mock_time = Arc::new(MockTimeProvider::new(Instant::now()));
	let backend = Arc::new(MemoryBackend::with_time_provider(mock_time.clone()));
	let config = TokenBucketConfig::new(10, 5, 2, 1);

	let throttle =
		TokenBucket::with_time_provider("api_key".to_string(), backend, config, mock_time.clone());

	// Verify initial token count equals capacity
	assert_eq!(throttle.tokens().await, 10);

	// Consume tokens
	for _ in 0..10 {
		assert!(throttle.allow_request("user").await.unwrap());
	}

	// Token exhaustion - next request should be rejected
	assert!(!throttle.allow_request("user").await.unwrap());
	assert_eq!(throttle.tokens().await, 0);

	// Advance time by refill interval (2 seconds)
	mock_time.advance(std::time::Duration::from_secs(2));

	// After refill, 5 new tokens should be available
	assert_eq!(throttle.tokens().await, 5);
	for _ in 0..5 {
		assert!(throttle.allow_request("user").await.unwrap());
	}

	// Token exhaustion again
	assert!(!throttle.allow_request("user").await.unwrap());
}

/// Test LeakyBucket algorithm integration with MemoryBackend
#[rstest]
#[tokio::test]
async fn test_leaky_bucket_algorithm_integration() {
	let mock_time = Arc::new(MockTimeProvider::new(Instant::now()));
	let backend = Arc::new(MemoryBackend::with_time_provider(mock_time.clone()));
	let config = LeakyBucketConfig::new(10, 2.0);

	let throttle = LeakyBucketThrottle::with_time_provider(
		"api_key".to_string(),
		backend,
		config,
		mock_time.clone(),
	);

	// Verify constant rate processing
	for _ in 0..10 {
		assert!(throttle.allow_request("user").await.unwrap());
	}
	assert_eq!(throttle.level().await, 10.0);

	// Bucket overflow - should be rejected
	assert!(!throttle.allow_request("user").await.unwrap());

	// Advance time by 1 second (2 requests leak at 2.0 req/sec)
	mock_time.advance(std::time::Duration::from_secs(1));

	// After leak, level should decrease by 2
	assert_eq!(throttle.level().await, 8.0);

	// Should allow 2 more requests
	assert!(throttle.allow_request("user").await.unwrap());
	assert!(throttle.allow_request("user").await.unwrap());
	assert_eq!(throttle.level().await, 10.0);

	// Bucket full again
	assert!(!throttle.allow_request("user").await.unwrap());
}

/// Test AdaptiveThrottle integration with load-based rate adjustment
#[rstest]
#[tokio::test]
async fn test_adaptive_throttle_integration() {
	let mock_time = Arc::new(MockTimeProvider::new(Instant::now()));
	let backend = Arc::new(MemoryBackend::with_time_provider(mock_time.clone()));
	let config = AdaptiveConfig::new((10, 60), (100, 60), (50, 60), 0.2, 0.7);

	let throttle = AdaptiveThrottle::with_time_provider(backend, config, mock_time.clone());

	// Initial rate should be 50 req/60sec
	assert_eq!(throttle.get_current_rate(), (50, 60));

	// Update with low stress metrics (system healthy)
	let low_stress = LoadMetrics::new(0.05, 100.0, 0.3);
	throttle.update_metrics(low_stress).await;

	// Advance time to trigger rate adjustment (5+ seconds required)
	mock_time.advance(std::time::Duration::from_secs(6));

	// Update metrics again to trigger adjustment
	throttle.update_metrics(low_stress).await;

	// Rate should increase under low stress
	let new_rate = throttle.get_current_rate();
	assert!(new_rate.0 > 50);
	assert!(new_rate.0 <= 100);

	// Update with high stress metrics
	let high_stress = LoadMetrics::new(0.9, 1500.0, 0.9);
	throttle.update_metrics(high_stress).await;

	// Advance time again
	mock_time.advance(std::time::Duration::from_secs(6));

	// Update to trigger adjustment
	throttle.update_metrics(high_stress).await;

	// Rate should decrease under high stress
	let reduced_rate = throttle.get_current_rate();
	assert!(reduced_rate.0 < new_rate.0);
}

/// Test AnonRateThrottle integration with IP-based rate limiting
#[rstest]
#[tokio::test]
async fn test_anon_rate_throttle_integration() {
	let mock_time = Arc::new(MockTimeProvider::new(Instant::now()));
	let backend = MemoryBackend::with_time_provider(mock_time.clone());
	let throttle = AnonRateThrottle::with_backend(5, 2, backend);

	// Verify anonymous user rate limit (IP-based)
	let ip1 = "192.168.1.100";
	for _ in 0..5 {
		assert!(throttle.allow_request(ip1).await.unwrap());
	}

	// 6th request from same IP should be rejected
	assert!(!throttle.allow_request(ip1).await.unwrap());

	// Different IP should have independent limit
	let ip2 = "192.168.1.101";
	assert!(throttle.allow_request(ip2).await.unwrap());

	// Advance time beyond window (2 seconds)
	mock_time.advance(std::time::Duration::from_secs(3));

	// After window expiration, ip1 should be allowed again
	assert!(throttle.allow_request(ip1).await.unwrap());
}

/// Test UserRateThrottle integration with user ID-based rate limiting
#[rstest]
#[tokio::test]
async fn test_user_rate_throttle_integration() {
	let mock_time = Arc::new(MockTimeProvider::new(Instant::now()));
	let backend = MemoryBackend::with_time_provider(mock_time.clone());
	let throttle = UserRateThrottle::with_backend(10, 5, backend);

	// Verify authenticated user rate limit (user ID-based)
	let user1 = "user_alice";
	for _ in 0..10 {
		assert!(throttle.allow_request(user1).await.unwrap());
	}

	// 11th request should be rejected
	assert!(!throttle.allow_request(user1).await.unwrap());

	// Different user should have independent limit
	let user2 = "user_bob";
	assert!(throttle.allow_request(user2).await.unwrap());

	// Advance time beyond window (5 seconds)
	mock_time.advance(std::time::Duration::from_secs(6));

	// After window expiration, user1 should be allowed again
	for _ in 0..10 {
		assert!(throttle.allow_request(user1).await.unwrap());
	}

	// 11th request should be rejected again
	assert!(!throttle.allow_request(user1).await.unwrap());
}

/// Test ScopedRateThrottle integration with independent scope-based limits
#[rstest]
#[tokio::test]
async fn test_scoped_rate_throttle_integration() {
	let mock_time = Arc::new(MockTimeProvider::new(Instant::now()));
	let backend = MemoryBackend::with_time_provider(mock_time.clone());

	let throttle = ScopedRateThrottle::with_backend(backend)
		.add_scope("api", 100, 60)
		.add_scope("upload", 5, 60)
		.add_scope("download", 20, 60);

	// Verify independent rate limits per scope
	let user1 = "user123";

	// Upload scope: 5 requests allowed
	for _ in 0..5 {
		assert!(
			throttle
				.allow_request(&format!("upload:{}", user1))
				.await
				.unwrap()
		);
	}
	assert!(
		!throttle
			.allow_request(&format!("upload:{}", user1))
			.await
			.unwrap()
	);

	// API scope: should be independent and allow 100 requests
	for _ in 0..100 {
		assert!(
			throttle
				.allow_request(&format!("api:{}", user1))
				.await
				.unwrap()
		);
	}
	assert!(
		!throttle
			.allow_request(&format!("api:{}", user1))
			.await
			.unwrap()
	);

	// Download scope: should be independent and allow 20 requests
	for _ in 0..20 {
		assert!(
			throttle
				.allow_request(&format!("download:{}", user1))
				.await
				.unwrap()
		);
	}
	assert!(
		!throttle
			.allow_request(&format!("download:{}", user1))
			.await
			.unwrap()
	);

	// Verify no interference between scopes for different users
	let user2 = "user456";
	assert!(
		throttle
			.allow_request(&format!("upload:{}", user2))
			.await
			.unwrap()
	);
	assert!(
		throttle
			.allow_request(&format!("api:{}", user2))
			.await
			.unwrap()
	);
	assert!(
		throttle
			.allow_request(&format!("download:{}", user2))
			.await
			.unwrap()
	);
}

/// Test MemoryBackend integration with multiple throttles
#[rstest]
#[tokio::test]
async fn test_memory_backend_integration() {
	let mock_time = Arc::new(MockTimeProvider::new(Instant::now()));
	let backend = Arc::new(MemoryBackend::with_time_provider(mock_time.clone()));

	// Create multiple throttles sharing the same backend
	let token_config = TokenBucketConfig::new(10, 10, 5, 1);
	let token_throttle = TokenBucket::with_time_provider(
		"token_key".to_string(),
		backend.clone(),
		token_config,
		mock_time.clone(),
	);

	let leaky_config = LeakyBucketConfig::new(15, 3.0);
	let leaky_throttle = LeakyBucketThrottle::with_time_provider(
		"leaky_key".to_string(),
		backend.clone(),
		leaky_config,
		mock_time.clone(),
	);

	// Verify memory backend manages counters correctly
	let key1 = "throttle:test:key1";
	let count1 = backend.increment(key1, 60).await.unwrap();
	assert_eq!(count1, 1);

	let count2 = backend.increment(key1, 60).await.unwrap();
	assert_eq!(count2, 2);

	// Verify different keys are tracked separately
	let key2 = "throttle:test:key2";
	let count3 = backend.increment(key2, 60).await.unwrap();
	assert_eq!(count3, 1);

	// Verify parallel throttle operations
	for _ in 0..10 {
		assert!(token_throttle.allow_request("user_a").await.unwrap());
	}
	assert!(!token_throttle.allow_request("user_a").await.unwrap());

	for _ in 0..15 {
		assert!(leaky_throttle.allow_request("user_b").await.unwrap());
	}
	assert!(!leaky_throttle.allow_request("user_b").await.unwrap());

	// Advance time
	mock_time.advance(std::time::Duration::from_secs(5));

	// TokenBucket should refill after 5 seconds (refill_interval=5)
	assert_eq!(token_throttle.tokens().await, 10);

	// LeakyBucket should leak requests (3.0 req/sec * 5 sec = 15 requests leaked)
	assert_eq!(leaky_throttle.level().await, 0.0);
}
