//! Rate limiting types and strategies.
//!
//! This module provides core types for rate limiting functionality
//! shared across the Reinhardt framework.

/// Rate limiting strategy
///
/// Defines how rate limits are applied to requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitStrategy {
	/// Rate limiting per route
	PerRoute,
	/// Rate limiting per user
	PerUser,
	/// Rate limiting per IP address
	PerIp,
	/// Rate limiting per IP address and user (combination)
	PerIpAndUser,
}
