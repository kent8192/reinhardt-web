//! gRPC server configuration
//!
//! This module provides server-level configuration for gRPC services,
//! including message size limits to prevent denial of service attacks
//! from oversized protobuf messages.
//!
//! # Default Message Size Limits
//!
//! By default, [`GrpcServerConfig`] enforces a 4MB limit on both
//! decoding (incoming) and encoding (outgoing) messages. This matches
//! the default behavior of tonic when explicit limits are configured.
//!
//! # Example
//!
//! ```rust
//! use reinhardt_grpc::server::GrpcServerConfig;
//!
//! // Use defaults (4MB limits)
//! let config = GrpcServerConfig::default();
//! assert_eq!(config.max_decoding_message_size(), 4 * 1024 * 1024);
//! assert_eq!(config.max_encoding_message_size(), 4 * 1024 * 1024);
//!
//! // Custom limits
//! let config = GrpcServerConfig::builder()
//!     .max_decoding_message_size(8 * 1024 * 1024) // 8MB
//!     .max_encoding_message_size(16 * 1024 * 1024) // 16MB
//!     .build();
//! assert_eq!(config.max_decoding_message_size(), 8 * 1024 * 1024);
//! ```

/// Default maximum decoding (incoming) message size: 4MB
const DEFAULT_MAX_DECODING_MESSAGE_SIZE: usize = 4 * 1024 * 1024;

/// Default maximum encoding (outgoing) message size: 4MB
const DEFAULT_MAX_ENCODING_MESSAGE_SIZE: usize = 4 * 1024 * 1024;

/// Configuration for gRPC server message size limits.
///
/// This struct holds the configuration for maximum message sizes
/// that the gRPC server will accept and send. Setting appropriate
/// limits prevents denial of service attacks from oversized messages.
///
/// # Example
///
/// ```rust
/// use reinhardt_grpc::server::GrpcServerConfig;
///
/// let config = GrpcServerConfig::builder()
///     .max_decoding_message_size(2 * 1024 * 1024) // 2MB for incoming
///     .max_encoding_message_size(8 * 1024 * 1024) // 8MB for outgoing
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct GrpcServerConfig {
	max_decoding_message_size: usize,
	max_encoding_message_size: usize,
}

impl GrpcServerConfig {
	/// Create a new builder for `GrpcServerConfig`.
	pub fn builder() -> GrpcServerConfigBuilder {
		GrpcServerConfigBuilder::default()
	}

	/// Returns the maximum decoding (incoming) message size in bytes.
	pub fn max_decoding_message_size(&self) -> usize {
		self.max_decoding_message_size
	}

	/// Returns the maximum encoding (outgoing) message size in bytes.
	pub fn max_encoding_message_size(&self) -> usize {
		self.max_encoding_message_size
	}
}

impl Default for GrpcServerConfig {
	fn default() -> Self {
		Self {
			max_decoding_message_size: DEFAULT_MAX_DECODING_MESSAGE_SIZE,
			max_encoding_message_size: DEFAULT_MAX_ENCODING_MESSAGE_SIZE,
		}
	}
}

/// Builder for [`GrpcServerConfig`].
///
/// Uses the builder pattern to construct a `GrpcServerConfig` with
/// custom message size limits. If not explicitly set, limits default
/// to 4MB each.
#[derive(Debug, Clone)]
pub struct GrpcServerConfigBuilder {
	max_decoding_message_size: usize,
	max_encoding_message_size: usize,
}

impl GrpcServerConfigBuilder {
	/// Set the maximum decoding (incoming) message size in bytes.
	///
	/// This limits the maximum size of a protobuf message that the
	/// server will accept from clients. Messages exceeding this limit
	/// will be rejected with a `ResourceExhausted` status.
	pub fn max_decoding_message_size(mut self, size: usize) -> Self {
		self.max_decoding_message_size = size;
		self
	}

	/// Set the maximum encoding (outgoing) message size in bytes.
	///
	/// This limits the maximum size of a protobuf message that the
	/// server will send to clients.
	pub fn max_encoding_message_size(mut self, size: usize) -> Self {
		self.max_encoding_message_size = size;
		self
	}

	/// Build the `GrpcServerConfig`.
	pub fn build(self) -> GrpcServerConfig {
		GrpcServerConfig {
			max_decoding_message_size: self.max_decoding_message_size,
			max_encoding_message_size: self.max_encoding_message_size,
		}
	}
}

impl Default for GrpcServerConfigBuilder {
	fn default() -> Self {
		Self {
			max_decoding_message_size: DEFAULT_MAX_DECODING_MESSAGE_SIZE,
			max_encoding_message_size: DEFAULT_MAX_ENCODING_MESSAGE_SIZE,
		}
	}
}

/// Trait for applying message size limits to tonic-generated gRPC service servers.
///
/// Tonic generates service server structs (e.g., `GreeterServer<T>`) that have
/// `max_decoding_message_size` and `max_encoding_message_size` methods. This
/// trait provides a unified way to apply [`GrpcServerConfig`] limits to any
/// such service.
///
/// # Example
///
/// ```rust,ignore
/// use reinhardt_grpc::server::{GrpcServerConfig, MessageSizeLimiter};
///
/// let config = GrpcServerConfig::default();
/// let service = MyServiceServer::new(my_impl).apply_message_size_limits(&config);
/// ```
pub trait MessageSizeLimiter: Sized {
	/// Apply message size limits from the given configuration.
	fn apply_message_size_limits(self, config: &GrpcServerConfig) -> Self;
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn default_config_has_4mb_limits() {
		// Arrange
		let expected_size = 4 * 1024 * 1024;

		// Act
		let config = GrpcServerConfig::default();

		// Assert
		assert_eq!(config.max_decoding_message_size(), expected_size);
		assert_eq!(config.max_encoding_message_size(), expected_size);
	}

	#[rstest]
	fn builder_default_matches_default_config() {
		// Arrange
		let default_config = GrpcServerConfig::default();

		// Act
		let builder_config = GrpcServerConfig::builder().build();

		// Assert
		assert_eq!(
			builder_config.max_decoding_message_size(),
			default_config.max_decoding_message_size()
		);
		assert_eq!(
			builder_config.max_encoding_message_size(),
			default_config.max_encoding_message_size()
		);
	}

	#[rstest]
	fn builder_sets_custom_decoding_limit() {
		// Arrange
		let custom_size = 8 * 1024 * 1024; // 8MB

		// Act
		let config = GrpcServerConfig::builder()
			.max_decoding_message_size(custom_size)
			.build();

		// Assert
		assert_eq!(config.max_decoding_message_size(), custom_size);
		// Encoding should remain default
		assert_eq!(
			config.max_encoding_message_size(),
			DEFAULT_MAX_ENCODING_MESSAGE_SIZE
		);
	}

	#[rstest]
	fn builder_sets_custom_encoding_limit() {
		// Arrange
		let custom_size = 16 * 1024 * 1024; // 16MB

		// Act
		let config = GrpcServerConfig::builder()
			.max_encoding_message_size(custom_size)
			.build();

		// Assert
		assert_eq!(
			config.max_decoding_message_size(),
			DEFAULT_MAX_DECODING_MESSAGE_SIZE
		);
		assert_eq!(config.max_encoding_message_size(), custom_size);
	}

	#[rstest]
	fn builder_sets_both_limits() {
		// Arrange
		let decoding_size = 2 * 1024 * 1024; // 2MB
		let encoding_size = 8 * 1024 * 1024; // 8MB

		// Act
		let config = GrpcServerConfig::builder()
			.max_decoding_message_size(decoding_size)
			.max_encoding_message_size(encoding_size)
			.build();

		// Assert
		assert_eq!(config.max_decoding_message_size(), decoding_size);
		assert_eq!(config.max_encoding_message_size(), encoding_size);
	}

	#[rstest]
	fn config_clone_preserves_values() {
		// Arrange
		let config = GrpcServerConfig::builder()
			.max_decoding_message_size(1024)
			.max_encoding_message_size(2048)
			.build();

		// Act
		let cloned = config.clone();

		// Assert
		assert_eq!(
			cloned.max_decoding_message_size(),
			config.max_decoding_message_size()
		);
		assert_eq!(
			cloned.max_encoding_message_size(),
			config.max_encoding_message_size()
		);
	}

	#[rstest]
	fn builder_allows_zero_size() {
		// Arrange & Act
		let config = GrpcServerConfig::builder()
			.max_decoding_message_size(0)
			.max_encoding_message_size(0)
			.build();

		// Assert
		assert_eq!(config.max_decoding_message_size(), 0);
		assert_eq!(config.max_encoding_message_size(), 0);
	}

	// Test that MessageSizeLimiter trait can be implemented for a mock service
	struct MockService {
		max_decoding: Option<usize>,
		max_encoding: Option<usize>,
	}

	impl MockService {
		fn new() -> Self {
			Self {
				max_decoding: None,
				max_encoding: None,
			}
		}
	}

	impl MessageSizeLimiter for MockService {
		fn apply_message_size_limits(mut self, config: &GrpcServerConfig) -> Self {
			self.max_decoding = Some(config.max_decoding_message_size());
			self.max_encoding = Some(config.max_encoding_message_size());
			self
		}
	}

	#[rstest]
	fn message_size_limiter_applies_config() {
		// Arrange
		let config = GrpcServerConfig::builder()
			.max_decoding_message_size(1024 * 1024)
			.max_encoding_message_size(2 * 1024 * 1024)
			.build();
		let service = MockService::new();

		// Act
		let service = service.apply_message_size_limits(&config);

		// Assert
		assert_eq!(service.max_decoding, Some(1024 * 1024));
		assert_eq!(service.max_encoding, Some(2 * 1024 * 1024));
	}

	#[rstest]
	fn message_size_limiter_applies_defaults() {
		// Arrange
		let config = GrpcServerConfig::default();
		let service = MockService::new();

		// Act
		let service = service.apply_message_size_limits(&config);

		// Assert
		assert_eq!(
			service.max_decoding,
			Some(DEFAULT_MAX_DECODING_MESSAGE_SIZE)
		);
		assert_eq!(
			service.max_encoding,
			Some(DEFAULT_MAX_ENCODING_MESSAGE_SIZE)
		);
	}
}
