//! Settings fragment for the gRPC server configuration.
//!
//! This module provides [`GrpcServerSettings`], a `#[settings]` fragment that
//! maps to the `[grpc_server]` configuration section. It is the settings-first
//! replacement for the deprecated [`GrpcServerConfig`] DoS-protection config.
//!
//! Durations are stored as integer seconds in the fragment so they can be
//! expressed naturally in TOML, and are converted back into [`std::time::Duration`]
//! by the [`From`] bridge.

#![allow(deprecated)] // Settings conversion targets the legacy config during the compatibility window.

use crate::server::GrpcServerConfig;
use reinhardt_core::macros::settings;
use serde::{Deserialize, Serialize};

/// Default maximum decoding (incoming) message size: 4MB.
const DEFAULT_MAX_DECODING_MESSAGE_SIZE: usize = 4 * 1024 * 1024;

/// Default maximum encoding (outgoing) message size: 4MB.
const DEFAULT_MAX_ENCODING_MESSAGE_SIZE: usize = 4 * 1024 * 1024;

/// Default request timeout in seconds: 30 seconds.
const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;

/// Default maximum concurrent connections: 1000.
const DEFAULT_MAX_CONCURRENT_CONNECTIONS: usize = 1000;

fn default_max_decoding_message_size() -> usize {
	DEFAULT_MAX_DECODING_MESSAGE_SIZE
}

fn default_max_encoding_message_size() -> usize {
	DEFAULT_MAX_ENCODING_MESSAGE_SIZE
}

fn default_request_timeout_secs() -> u64 {
	DEFAULT_REQUEST_TIMEOUT_SECS
}

fn default_max_concurrent_connections() -> usize {
	DEFAULT_MAX_CONCURRENT_CONNECTIONS
}

/// gRPC server configuration fragment.
///
/// This fragment maps to the `[grpc_server]` section and can be composed with
/// the `#[settings]` macro from downstream applications. It configures
/// DoS-protection limits for gRPC services: message size limits, request
/// timeouts, and connection limits.
///
/// # Example
///
/// ```rust
/// use reinhardt_grpc::GrpcServerSettings;
///
/// let settings: GrpcServerSettings = toml::from_str(r#"
/// max_decoding_message_size = 8388608
/// request_timeout_secs = 60
/// max_concurrent_connections = 500
/// "#).unwrap();
///
/// assert_eq!(settings.max_decoding_message_size, 8 * 1024 * 1024);
/// assert_eq!(settings.request_timeout_secs, 60);
/// assert_eq!(settings.max_concurrent_connections, 500);
/// ```
#[settings(fragment = true, section = "grpc_server")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GrpcServerSettings {
	/// Maximum decoding (incoming) message size in bytes.
	#[serde(default = "default_max_decoding_message_size")]
	pub max_decoding_message_size: usize,
	/// Maximum encoding (outgoing) message size in bytes.
	#[serde(default = "default_max_encoding_message_size")]
	pub max_encoding_message_size: usize,
	/// Request timeout in seconds.
	#[serde(default = "default_request_timeout_secs")]
	pub request_timeout_secs: u64,
	/// Maximum number of concurrent connections allowed.
	#[serde(default = "default_max_concurrent_connections")]
	pub max_concurrent_connections: usize,
}

impl Default for GrpcServerSettings {
	fn default() -> Self {
		Self {
			max_decoding_message_size: DEFAULT_MAX_DECODING_MESSAGE_SIZE,
			max_encoding_message_size: DEFAULT_MAX_ENCODING_MESSAGE_SIZE,
			request_timeout_secs: DEFAULT_REQUEST_TIMEOUT_SECS,
			max_concurrent_connections: DEFAULT_MAX_CONCURRENT_CONNECTIONS,
		}
	}
}

impl From<&GrpcServerSettings> for GrpcServerConfig {
	fn from(settings: &GrpcServerSettings) -> Self {
		// `GrpcServerConfig` has private fields, so rebuild it through its builder.
		GrpcServerConfig::builder()
			.max_decoding_message_size(settings.max_decoding_message_size)
			.max_encoding_message_size(settings.max_encoding_message_size)
			.request_timeout(std::time::Duration::from_secs(
				settings.request_timeout_secs,
			))
			.max_concurrent_connections(settings.max_concurrent_connections)
			.build()
	}
}

/// Create a [`GrpcServerConfig`] from a [`GrpcServerSettings`] fragment.
///
/// This is the settings-first entry point that replaces direct construction of
/// the deprecated [`GrpcServerConfig`].
pub fn create_grpc_server_config_from_settings(settings: &GrpcServerSettings) -> GrpcServerConfig {
	GrpcServerConfig::from(settings)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::Duration;

	#[test]
	fn default_settings_match_config_defaults() {
		// Arrange & Act
		let settings = GrpcServerSettings::default();
		let config = GrpcServerConfig::from(&settings);
		let default_config = GrpcServerConfig::default();

		// Assert
		assert_eq!(
			config.max_decoding_message_size(),
			default_config.max_decoding_message_size()
		);
		assert_eq!(
			config.max_encoding_message_size(),
			default_config.max_encoding_message_size()
		);
		assert_eq!(config.request_timeout(), default_config.request_timeout());
		assert_eq!(
			config.max_concurrent_connections(),
			default_config.max_concurrent_connections()
		);
	}

	#[test]
	fn custom_settings_convert_to_config() {
		// Arrange
		let settings = GrpcServerSettings {
			max_decoding_message_size: 8 * 1024 * 1024,
			max_encoding_message_size: 16 * 1024 * 1024,
			request_timeout_secs: 60,
			max_concurrent_connections: 500,
		};

		// Act
		let config = create_grpc_server_config_from_settings(&settings);

		// Assert
		assert_eq!(config.max_decoding_message_size(), 8 * 1024 * 1024);
		assert_eq!(config.max_encoding_message_size(), 16 * 1024 * 1024);
		assert_eq!(config.request_timeout(), Duration::from_secs(60));
		assert_eq!(config.max_concurrent_connections(), 500);
	}

	#[test]
	fn settings_deserialize_from_toml_with_partial_fields() {
		// Arrange & Act
		let settings: GrpcServerSettings = toml::from_str(
			r#"
			max_decoding_message_size = 2097152
			"#,
		)
		.expect("settings should deserialize");

		// Assert: provided field is honored, missing fields fall back to defaults.
		assert_eq!(settings.max_decoding_message_size, 2 * 1024 * 1024);
		assert_eq!(
			settings.max_encoding_message_size,
			DEFAULT_MAX_ENCODING_MESSAGE_SIZE
		);
		assert_eq!(settings.request_timeout_secs, DEFAULT_REQUEST_TIMEOUT_SECS);
		assert_eq!(
			settings.max_concurrent_connections,
			DEFAULT_MAX_CONCURRENT_CONNECTIONS
		);
	}
}
