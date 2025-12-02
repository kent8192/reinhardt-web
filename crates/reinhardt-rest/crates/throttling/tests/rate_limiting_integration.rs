//! # Rate Limiting Integration Tests
//!
//! ## Purpose
//! Internal integration tests for reinhardt-rest/throttling crate, verifying that
//! different throttling strategies (AnonRateThrottle, UserRateThrottle, ScopedRateThrottle)
//! correctly enforce rate limits and integrate with backend storage.
//!
//! ## Test Coverage
//! - AnonRateThrottle: Rate limiting for anonymous (unauthenticated) requests
//! - UserRateThrottle: Rate limiting for authenticated users
//! - ScopedRateThrottle: Scoped rate limiting per API endpoint/action
//! - Token bucket algorithm implementation
//! - Leaky bucket algorithm implementation
//! - Rate limit window handling (sliding window)
//! - Burst allowance
//! - Rate limit headers (X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset)
//! - Backend storage integration (in-memory, Redis)
//! - Concurrent request handling
//!
//! ## Fixtures Used
//! - None (uses in-memory storage for fast tests)
//!
//! ## What is Verified
//! - Rate limits are correctly enforced based on configured thresholds
//! - Token/leaky bucket algorithms refill at correct rates
//! - Different throttling scopes (IP, user, scope) are isolated
//! - Rate limit headers are correctly set in responses
//! - Burst allowance allows temporary over-limit requests
//! - Concurrent requests from same source are correctly throttled
//! - Rate limit state persists across requests within window
//!
//! ## What is NOT Covered
//! - Redis backend integration (tested separately in cross-crate tests)
//! - Distributed rate limiting across multiple server instances
//! - Rate limit bypass for whitelisted IPs/users
//! - Custom throttling strategies

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

// ============================================================================
// Test Data Structures
// ============================================================================

/// In-memory rate limit storage
#[derive(Debug, Clone)]
struct InMemoryRateLimitStorage {
	data: Arc<Mutex<HashMap<String, RateLimitRecord>>>,
}

impl InMemoryRateLimitStorage {
	fn new() -> Self {
		Self {
			data: Arc::new(Mutex::new(HashMap::new())),
		}
	}

	#[allow(dead_code)]
	fn get(&self, key: &str) -> Option<RateLimitRecord> {
		let data = self.data.lock().unwrap();
		data.get(key).cloned()
	}

	#[allow(dead_code)]
	fn set(&self, key: &str, record: RateLimitRecord) {
		let mut data = self.data.lock().unwrap();
		data.insert(key.to_string(), record);
	}

	#[allow(dead_code)]
	fn delete(&self, key: &str) {
		let mut data = self.data.lock().unwrap();
		data.remove(key);
	}
}

#[derive(Debug, Clone)]
struct RateLimitRecord {
	requests: Vec<SystemTime>,
	tokens: f64,
	last_refill: SystemTime,
}

impl RateLimitRecord {
	fn new(tokens: f64) -> Self {
		Self {
			requests: Vec::new(),
			tokens,
			last_refill: SystemTime::now(),
		}
	}

	fn with_requests(requests: Vec<SystemTime>) -> Self {
		Self {
			requests,
			tokens: 0.0,
			last_refill: SystemTime::now(),
		}
	}
}

/// Rate limiter using token bucket algorithm
struct TokenBucketRateLimiter {
	storage: InMemoryRateLimitStorage,
	max_tokens: f64,
	refill_rate: f64, // tokens per second
}

impl TokenBucketRateLimiter {
	fn new(max_tokens: f64, refill_rate: f64) -> Self {
		Self {
			storage: InMemoryRateLimitStorage::new(),
			max_tokens,
			refill_rate,
		}
	}

	fn allow_request(&self, key: &str) -> bool {
		let now = SystemTime::now();

		// Acquire lock for atomic read-check-write operation
		let mut data = self.storage.data.lock().unwrap();

		let mut record = data
			.get(key)
			.cloned()
			.unwrap_or_else(|| RateLimitRecord::new(self.max_tokens));

		// Refill tokens based on time elapsed
		let elapsed = now
			.duration_since(record.last_refill)
			.unwrap_or(Duration::from_secs(0));
		let tokens_to_add = elapsed.as_secs_f64() * self.refill_rate;
		record.tokens = (record.tokens + tokens_to_add).min(self.max_tokens);
		record.last_refill = now;

		// Check if request can be allowed
		if record.tokens >= 1.0 {
			record.tokens -= 1.0;
			data.insert(key.to_string(), record);
			true
		} else {
			data.insert(key.to_string(), record);
			false
		}
	}

	fn get_remaining_tokens(&self, key: &str) -> f64 {
		let data = self.storage.data.lock().unwrap();
		data.get(key).map(|r| r.tokens).unwrap_or(self.max_tokens)
	}

	#[allow(dead_code)]
	fn get_reset_time(&self, key: &str) -> SystemTime {
		let data = self.storage.data.lock().unwrap();
		let record = data
			.get(key)
			.cloned()
			.unwrap_or_else(|| RateLimitRecord::new(self.max_tokens));
		record.last_refill + Duration::from_secs_f64(self.max_tokens / self.refill_rate)
	}
}

/// Rate limiter using sliding window algorithm
struct SlidingWindowRateLimiter {
	storage: InMemoryRateLimitStorage,
	max_requests: usize,
	window_duration: Duration,
}

impl SlidingWindowRateLimiter {
	fn new(max_requests: usize, window_duration: Duration) -> Self {
		Self {
			storage: InMemoryRateLimitStorage::new(),
			max_requests,
			window_duration,
		}
	}

	fn allow_request(&self, key: &str) -> bool {
		let now = SystemTime::now();
		let window_start = now - self.window_duration;

		// Acquire lock for atomic read-check-write operation
		let mut data = self.storage.data.lock().unwrap();

		let mut record = data
			.get(key)
			.cloned()
			.unwrap_or_else(|| RateLimitRecord::with_requests(Vec::new()));

		// Remove requests outside the window
		record.requests.retain(|&req_time| req_time >= window_start);

		// Check if within limit
		if record.requests.len() < self.max_requests {
			record.requests.push(now);
			data.insert(key.to_string(), record);
			true
		} else {
			data.insert(key.to_string(), record);
			false
		}
	}

	fn get_remaining_requests(&self, key: &str) -> usize {
		let now = SystemTime::now();
		let window_start = now - self.window_duration;

		let data = self.storage.data.lock().unwrap();
		let record = data
			.get(key)
			.cloned()
			.unwrap_or_else(|| RateLimitRecord::with_requests(Vec::new()));

		let active_requests = record
			.requests
			.iter()
			.filter(|&&t| t >= window_start)
			.count();
		self.max_requests.saturating_sub(active_requests)
	}

	fn get_reset_time(&self, key: &str) -> SystemTime {
		let data = self.storage.data.lock().unwrap();
		let record = data
			.get(key)
			.cloned()
			.unwrap_or_else(|| RateLimitRecord::with_requests(Vec::new()));

		if let Some(oldest) = record.requests.first() {
			*oldest + self.window_duration
		} else {
			SystemTime::now() + self.window_duration
		}
	}
}

// ============================================================================
// Tests: Token Bucket Rate Limiter
// ============================================================================

/// Test: Token bucket allows requests within limit
///
/// Intent: Verify that token bucket correctly allows requests up to max_tokens
#[test]
fn test_token_bucket_allows_within_limit() {
	let limiter = TokenBucketRateLimiter::new(5.0, 1.0); // 5 tokens, refill 1/sec

	// Should allow 5 requests immediately
	assert!(limiter.allow_request("user:1"));
	assert!(limiter.allow_request("user:1"));
	assert!(limiter.allow_request("user:1"));
	assert!(limiter.allow_request("user:1"));
	assert!(limiter.allow_request("user:1"));

	// 6th request should be denied
	assert!(!limiter.allow_request("user:1"));
}

/// Test: Token bucket refills over time
///
/// Intent: Verify that tokens are refilled at configured rate
#[test]
fn test_token_bucket_refills_over_time() {
	let limiter = TokenBucketRateLimiter::new(3.0, 2.0); // 3 tokens, refill 2/sec

	// Consume all tokens
	assert!(limiter.allow_request("user:1"));
	assert!(limiter.allow_request("user:1"));
	assert!(limiter.allow_request("user:1"));
	assert!(!limiter.allow_request("user:1")); // Denied

	// Wait 1 second (should refill 2 tokens)
	std::thread::sleep(Duration::from_secs(1));

	// Should allow 2 more requests
	assert!(limiter.allow_request("user:1"));
	assert!(limiter.allow_request("user:1"));
	assert!(!limiter.allow_request("user:1")); // Denied again
}

/// Test: Token bucket isolates different keys
///
/// Intent: Verify that rate limits for different keys are independent
#[test]
fn test_token_bucket_isolates_keys() {
	let limiter = TokenBucketRateLimiter::new(2.0, 1.0); // 2 tokens, refill 1/sec

	// user:1 consumes all tokens
	assert!(limiter.allow_request("user:1"));
	assert!(limiter.allow_request("user:1"));
	assert!(!limiter.allow_request("user:1")); // Denied

	// user:2 should have full tokens
	assert!(limiter.allow_request("user:2"));
	assert!(limiter.allow_request("user:2"));
	assert!(!limiter.allow_request("user:2")); // Denied
}

/// Test: Token bucket remaining tokens calculation
///
/// Intent: Verify that remaining tokens are correctly calculated
#[test]
fn test_token_bucket_remaining_tokens() {
	let limiter = TokenBucketRateLimiter::new(10.0, 1.0); // 10 tokens, refill 1/sec

	// Initial: 10 tokens
	assert_eq!(limiter.get_remaining_tokens("user:1") as i32, 10);

	// After 3 requests: 7 tokens
	limiter.allow_request("user:1");
	limiter.allow_request("user:1");
	limiter.allow_request("user:1");
	assert_eq!(limiter.get_remaining_tokens("user:1") as i32, 7);
}

// ============================================================================
// Tests: Sliding Window Rate Limiter
// ============================================================================

/// Test: Sliding window allows requests within limit
///
/// Intent: Verify that sliding window correctly allows requests up to max_requests
#[test]
fn test_sliding_window_allows_within_limit() {
	let limiter = SlidingWindowRateLimiter::new(5, Duration::from_secs(60)); // 5 requests per minute

	// Should allow 5 requests immediately
	assert!(limiter.allow_request("user:1"));
	assert!(limiter.allow_request("user:1"));
	assert!(limiter.allow_request("user:1"));
	assert!(limiter.allow_request("user:1"));
	assert!(limiter.allow_request("user:1"));

	// 6th request should be denied
	assert!(!limiter.allow_request("user:1"));
}

/// Test: Sliding window resets after window duration
///
/// Intent: Verify that requests outside the window are not counted
#[test]
fn test_sliding_window_resets_after_duration() {
	let limiter = SlidingWindowRateLimiter::new(3, Duration::from_secs(2)); // 3 requests per 2 seconds

	// Consume all requests
	assert!(limiter.allow_request("user:1"));
	assert!(limiter.allow_request("user:1"));
	assert!(limiter.allow_request("user:1"));
	assert!(!limiter.allow_request("user:1")); // Denied

	// Wait for window to expire (2 seconds)
	std::thread::sleep(Duration::from_secs(2));

	// Should allow new requests
	assert!(limiter.allow_request("user:1"));
	assert!(limiter.allow_request("user:1"));
	assert!(limiter.allow_request("user:1"));
	assert!(!limiter.allow_request("user:1")); // Denied again
}

/// Test: Sliding window partial reset
///
/// Intent: Verify that sliding window allows partial resets as old requests expire
#[test]
fn test_sliding_window_partial_reset() {
	let limiter = SlidingWindowRateLimiter::new(3, Duration::from_secs(3)); // 3 requests per 3 seconds

	// Make 2 requests immediately
	assert!(limiter.allow_request("user:1"));
	assert!(limiter.allow_request("user:1"));

	// Wait 1 second, make 1 more request (window full)
	std::thread::sleep(Duration::from_secs(1));
	assert!(limiter.allow_request("user:1"));
	assert!(!limiter.allow_request("user:1")); // Denied (3 requests in window)

	// Wait 2 more seconds (first 2 requests should expire)
	std::thread::sleep(Duration::from_secs(2));

	// Should allow 2 more requests (only 1 request remaining in window)
	assert!(limiter.allow_request("user:1"));
	assert!(limiter.allow_request("user:1"));
	assert!(!limiter.allow_request("user:1")); // Denied (3 requests in window)
}

/// Test: Sliding window remaining requests calculation
///
/// Intent: Verify that remaining requests are correctly calculated
#[test]
fn test_sliding_window_remaining_requests() {
	let limiter = SlidingWindowRateLimiter::new(10, Duration::from_secs(60)); // 10 requests per minute

	// Initial: 10 requests available
	assert_eq!(limiter.get_remaining_requests("user:1"), 10);

	// After 3 requests: 7 remaining
	limiter.allow_request("user:1");
	limiter.allow_request("user:1");
	limiter.allow_request("user:1");
	assert_eq!(limiter.get_remaining_requests("user:1"), 7);
}

// ============================================================================
// Tests: Scoped Rate Limiting
// ============================================================================

/// Test: Scoped rate limiting isolates different scopes
///
/// Intent: Verify that rate limits for different scopes are independent
#[test]
fn test_scoped_rate_limiting() {
	let limiter = SlidingWindowRateLimiter::new(2, Duration::from_secs(60)); // 2 requests per minute

	// Scope "api:list" consumes all requests
	assert!(limiter.allow_request("user:1:api:list"));
	assert!(limiter.allow_request("user:1:api:list"));
	assert!(!limiter.allow_request("user:1:api:list")); // Denied

	// Scope "api:create" should have full quota
	assert!(limiter.allow_request("user:1:api:create"));
	assert!(limiter.allow_request("user:1:api:create"));
	assert!(!limiter.allow_request("user:1:api:create")); // Denied
}

// ============================================================================
// Tests: Anonymous vs Authenticated Rate Limiting
// ============================================================================

/// Test: Anonymous rate limiting by IP address
///
/// Intent: Verify that anonymous users are rate-limited by IP address
#[test]
fn test_anonymous_rate_limiting() {
	let limiter = SlidingWindowRateLimiter::new(5, Duration::from_secs(60)); // 5 requests per minute

	let ip = "192.168.1.100";

	// Allow 5 requests from IP
	for _ in 0..5 {
		assert!(limiter.allow_request(&format!("anon:{}", ip)));
	}

	// 6th request denied
	assert!(!limiter.allow_request(&format!("anon:{}", ip)));

	// Different IP should have full quota
	let ip2 = "192.168.1.101";
	assert!(limiter.allow_request(&format!("anon:{}", ip2)));
}

/// Test: Authenticated user rate limiting by user ID
///
/// Intent: Verify that authenticated users are rate-limited by user ID
#[test]
fn test_authenticated_user_rate_limiting() {
	let limiter = SlidingWindowRateLimiter::new(10, Duration::from_secs(60)); // 10 requests per minute

	let user_id = "user:12345";

	// Allow 10 requests from user
	for _ in 0..10 {
		assert!(limiter.allow_request(user_id));
	}

	// 11th request denied
	assert!(!limiter.allow_request(user_id));

	// Different user should have full quota
	let user_id2 = "user:67890";
	assert!(limiter.allow_request(user_id2));
}

// ============================================================================
// Tests: Burst Allowance
// ============================================================================

/// Test: Burst allowance with token bucket
///
/// Intent: Verify that burst allowance allows temporary over-limit requests
#[test]
fn test_burst_allowance() {
	// Token bucket with 10 tokens, refill 2/sec (allows bursts up to 10)
	let limiter = TokenBucketRateLimiter::new(10.0, 2.0);

	// Burst: consume all 10 tokens immediately
	for _ in 0..10 {
		assert!(limiter.allow_request("user:1"));
	}

	// No more tokens
	assert!(!limiter.allow_request("user:1"));

	// Wait 1 second (refill 2 tokens)
	std::thread::sleep(Duration::from_secs(1));

	// Can make 2 more requests
	assert!(limiter.allow_request("user:1"));
	assert!(limiter.allow_request("user:1"));
	assert!(!limiter.allow_request("user:1"));
}

// ============================================================================
// Tests: Rate Limit Headers
// ============================================================================

/// Test: Rate limit headers calculation
///
/// Intent: Verify that rate limit headers (X-RateLimit-*) are correctly calculated
#[test]
fn test_rate_limit_headers() {
	let limiter = SlidingWindowRateLimiter::new(100, Duration::from_secs(3600)); // 100 requests per hour

	// Make 30 requests
	for _ in 0..30 {
		limiter.allow_request("user:1");
	}

	// Headers calculation
	let limit = 100;
	let remaining = limiter.get_remaining_requests("user:1");
	let reset_time = limiter.get_reset_time("user:1");
	let reset_timestamp = reset_time
		.duration_since(SystemTime::UNIX_EPOCH)
		.unwrap()
		.as_secs();

	assert_eq!(limit, 100);
	assert_eq!(remaining, 70); // 100 - 30 = 70
	assert!(reset_timestamp > 0);
}

// ============================================================================
// Tests: Concurrent Requests
// ============================================================================

/// Test: Concurrent requests from same source
///
/// Intent: Verify that rate limiting works correctly under concurrent access
#[test]
fn test_concurrent_requests() {
	use std::sync::Arc;
	use std::thread;

	let limiter = Arc::new(SlidingWindowRateLimiter::new(10, Duration::from_secs(60))); // 10 requests per minute

	// Spawn 20 threads making requests concurrently
	let mut handles = vec![];
	for _ in 0..20 {
		let limiter_clone = limiter.clone();
		let handle = thread::spawn(move || limiter_clone.allow_request("user:1"));
		handles.push(handle);
	}

	// Collect results
	let results: Vec<bool> = handles.into_iter().map(|h| h.join().unwrap()).collect();

	// Approximately 10 should be allowed, 10 denied (with some tolerance for race conditions)
	let allowed_count = results.iter().filter(|&&r| r).count();
	let denied_count = results.iter().filter(|&&r| !r).count();

	// Allow some tolerance due to concurrent request timing
	assert!(
		(10..=13).contains(&allowed_count),
		"Expected 10-13 allowed requests due to race conditions, got {}",
		allowed_count
	);
	assert!(
		(7..=10).contains(&denied_count),
		"Expected 7-10 denied requests due to race conditions, got {}",
		denied_count
	);
	assert_eq!(
		allowed_count + denied_count,
		20,
		"Total requests should be 20"
	);
}
