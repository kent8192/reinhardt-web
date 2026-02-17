//! Retry mechanism tests
//!
//! Tests retry strategies (exponential backoff, fixed delay), retry state management,
//! maximum retry limits, and delay calculations.

use reinhardt_tasks::{RetryState, RetryStrategy};
use rstest::rstest;
use std::time::Duration;

/// Test: Exponential backoff retry strategy
#[rstest]
fn test_exponential_backoff_strategy() {
	let strategy = RetryStrategy::exponential_backoff();

	assert_eq!(strategy.max_retries(), 3);
	assert_eq!(strategy.initial_delay(), Duration::from_secs(1));
	assert_eq!(strategy.max_delay(), Duration::from_secs(60));
	assert_eq!(strategy.multiplier(), 2.0);
	assert!(strategy.has_jitter());
}

/// Test: Fixed delay retry strategy
#[rstest]
fn test_fixed_delay_strategy() {
	let delay = Duration::from_secs(5);
	let strategy = RetryStrategy::fixed_delay(delay);

	assert_eq!(strategy.max_retries(), 3);
	assert_eq!(strategy.initial_delay(), delay);
	assert_eq!(strategy.max_delay(), delay);
	assert_eq!(strategy.multiplier(), 1.0);
	assert!(!strategy.has_jitter());
}

/// Test: No retry strategy
#[rstest]
fn test_no_retry_strategy() {
	let strategy = RetryStrategy::no_retry();

	assert_eq!(strategy.max_retries(), 0);
	assert_eq!(strategy.initial_delay(), Duration::from_secs(0));
	assert_eq!(strategy.max_delay(), Duration::from_secs(0));
	assert!(!strategy.should_retry(0));
	assert!(!strategy.should_retry(1));
}

/// Test: Custom retry strategy configuration
#[rstest]
fn test_custom_retry_strategy() {
	let strategy = RetryStrategy::exponential_backoff()
		.with_max_retries(5)
		.with_initial_delay(Duration::from_millis(500))
		.with_max_delay(Duration::from_secs(300))
		.with_multiplier(3.0)
		.with_jitter(false);

	assert_eq!(strategy.max_retries(), 5);
	assert_eq!(strategy.initial_delay(), Duration::from_millis(500));
	assert_eq!(strategy.max_delay(), Duration::from_secs(300));
	assert_eq!(strategy.multiplier(), 3.0);
	assert!(!strategy.has_jitter());
}

/// Test: Should retry logic
#[rstest]
fn test_should_retry() {
	let strategy = RetryStrategy::exponential_backoff().with_max_retries(3);

	// Attempts 0, 1, 2 should retry (attempts < max_retries)
	assert!(strategy.should_retry(0));
	assert!(strategy.should_retry(1));
	assert!(strategy.should_retry(2));

	// Attempts 3 and beyond should not retry
	assert!(!strategy.should_retry(3));
	assert!(!strategy.should_retry(4));
	assert!(!strategy.should_retry(10));
}

/// Test: Exponential backoff delay calculation (without jitter)
#[rstest]
fn test_exponential_backoff_delays() {
	let strategy = RetryStrategy::exponential_backoff()
		.with_initial_delay(Duration::from_secs(1))
		.with_multiplier(2.0)
		.with_jitter(false); // Disable jitter for predictable results

	// Delay calculation: initial_delay * multiplier^(attempt - 1)
	// Attempt 1: 1 * 2^0 = 1 second
	// Attempt 2: 1 * 2^1 = 2 seconds
	// Attempt 3: 1 * 2^2 = 4 seconds

	let delay1 = strategy.calculate_delay(1);
	let delay2 = strategy.calculate_delay(2);
	let delay3 = strategy.calculate_delay(3);

	assert_eq!(delay1, Duration::from_secs(1));
	assert_eq!(delay2, Duration::from_secs(2));
	assert_eq!(delay3, Duration::from_secs(4));

	// Verify exponential growth
	assert!(delay2 > delay1);
	assert!(delay3 > delay2);
}

/// Test: Fixed delay calculation
#[rstest]
fn test_fixed_delay_calculation() {
	let delay = Duration::from_secs(5);
	let strategy = RetryStrategy::fixed_delay(delay);

	// All attempts should have the same delay
	assert_eq!(strategy.calculate_delay(1), delay);
	assert_eq!(strategy.calculate_delay(2), delay);
	assert_eq!(strategy.calculate_delay(3), delay);
	assert_eq!(strategy.calculate_delay(10), delay);
}

/// Test: Maximum delay cap
#[rstest]
fn test_max_delay_cap() {
	let strategy = RetryStrategy::exponential_backoff()
		.with_initial_delay(Duration::from_secs(1))
		.with_max_delay(Duration::from_secs(10))
		.with_multiplier(2.0)
		.with_jitter(false);

	// Attempt 1: 1 second
	// Attempt 2: 2 seconds
	// Attempt 3: 4 seconds
	// Attempt 4: 8 seconds
	// Attempt 5: 16 seconds -> capped at 10 seconds
	// Attempt 6: 32 seconds -> capped at 10 seconds

	let delay5 = strategy.calculate_delay(5);
	let delay6 = strategy.calculate_delay(6);

	assert_eq!(delay5, Duration::from_secs(10));
	assert_eq!(delay6, Duration::from_secs(10));
}

/// Test: RetryState initialization and tracking
#[rstest]
fn test_retry_state() {
	let strategy = RetryStrategy::exponential_backoff().with_max_retries(3);
	let mut state = RetryState::new(strategy);

	// Initial state
	assert_eq!(state.attempts(), 0);
	assert!(state.can_retry());

	// Increment attempts
	state.record_attempt();
	assert_eq!(state.attempts(), 1);
	assert!(state.can_retry());

	state.record_attempt();
	assert_eq!(state.attempts(), 2);
	assert!(state.can_retry());

	state.record_attempt();
	assert_eq!(state.attempts(), 3);
	assert!(!state.can_retry()); // Reached max_retries
}

/// Test: RetryState reset
#[rstest]
fn test_retry_state_reset() {
	let strategy = RetryStrategy::exponential_backoff();
	let mut state = RetryState::new(strategy);

	// Record some attempts
	state.record_attempt();
	state.record_attempt();
	assert_eq!(state.attempts(), 2);

	// Reset state
	state.reset();
	assert_eq!(state.attempts(), 0);
	assert!(state.can_retry());
}

/// Test: Jitter introduces randomness
#[rstest]
fn test_jitter_randomness() {
	let strategy = RetryStrategy::exponential_backoff()
		.with_jitter(true)
		.with_initial_delay(Duration::from_secs(10))
		.with_multiplier(1.0); // No exponential growth, just jitter

	// With jitter enabled, delays should vary
	// Generate multiple delays and check they're not all identical
	let delays: Vec<Duration> = (0..10).map(|_| strategy.calculate_delay(1)).collect();

	// Check that not all delays are identical (jitter introduces variation)
	let first_delay = delays[0];
	let all_same = delays.iter().all(|&d| d == first_delay);

	assert!(
		!all_same,
		"Jitter should introduce variation in delays (though rare, all identical is unlikely)"
	);

	// All delays should be <= base delay (jitter reduces delay)
	let base_delay = Duration::from_secs(10);
	for delay in delays {
		assert!(delay <= base_delay);
	}
}
