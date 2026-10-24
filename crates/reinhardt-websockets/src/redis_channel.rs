//! Redis-backed channel layer for distributed WebSocket systems
//!
//! This module provides a channel layer for distributed WebSocket systems using Redis.
//! It allows sharing WebSocket connections across multiple application instances.
//!
//! ## Usage Examples
//!
//! ```no_run
//! use reinhardt_websockets::redis_channel::{RedisChannelLayer, RedisConfig};
//! use reinhardt_websockets::channels::{ChannelLayer, ChannelMessage};
//! use reinhardt_websockets::Message;
//!
//! # tokio_test::block_on(async {
//! let config = RedisConfig::default();
//! let layer = RedisChannelLayer::new(config).await.unwrap();
//!
//! let msg = ChannelMessage::new(
//!     "user_1".to_string(),
//!     Message::text("Hello".to_string()),
//! );
//!
//! layer.send("channel_1", msg).await.unwrap();
//! # });
//! ```

#![allow(deprecated)] // `RedisConfig` is deprecated but still used internally during the compatibility window.

#[cfg(feature = "redis-channel")]
use crate::channels::{ChannelError, ChannelLayer, ChannelMessage, ChannelResult};
#[cfg(feature = "redis-channel")]
use async_trait::async_trait;
#[cfg(feature = "redis-channel")]
use redis::aio::ConnectionManager;
#[cfg(feature = "redis-channel")]
use redis::{AsyncCommands, Client, ConnectionAddr, ConnectionInfo, IntoConnectionInfo};
#[cfg(feature = "redis-channel")]
use tracing::warn;

/// Redis channel layer configuration
///
/// The `Debug` output redacts `password`.
///
/// # Effective connection
///
/// [`RedisChannelLayer::new`] connects with a single descriptor built from
/// `url` plus the dedicated `username`, `password` and `tls` fields:
///
/// - `username` and `password` are sent to Redis. Credentials embedded in
///   `url` are also honoured, but a URL credential that differs from the
///   matching field is rejected as contradictory.
/// - A username without a password is rejected, because Redis only
///   authenticates when a password is present.
/// - `tls = true` upgrades a `redis://` URL to TLS with certificate and
///   hostname verification, and is rejected for addresses that cannot use TLS
///   (Unix sockets). A `rediss://` URL always uses TLS.
/// - `require_auth = true` fails unless `url` or `password` supplies a
///   password.
#[cfg(feature = "redis-channel")]
#[deprecated(
	since = "0.2.0",
	note = "Use `RedisChannelSettings` with the `#[settings]` macro instead."
)]
#[derive(Clone)]
pub struct RedisConfig {
	/// Redis connection URL
	pub url: String,
	/// Channel prefix
	pub channel_prefix: String,
	/// Group prefix
	pub group_prefix: String,
	/// Message expiry time (seconds)
	pub message_expiry: u64,
	/// Redis password for authentication.
	///
	/// Must match the password embedded in `url`, if any.
	pub password: Option<String>,
	/// Redis username for authentication (Redis 6+ ACL).
	///
	/// Requires a password and must match the username embedded in `url`, if any.
	pub username: Option<String>,
	/// Enable TLS for secure connection.
	///
	/// Upgrades a `redis://` URL to TLS; connecting fails rather than falling
	/// back to plaintext.
	pub tls: bool,
	/// Require authentication: connecting fails unless `url` or `password`
	/// supplies a password.
	pub require_auth: bool,
}

#[cfg(feature = "redis-channel")]
impl std::fmt::Debug for RedisConfig {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		// Exhaustive destructuring makes adding a field a compile error here,
		// so new fields cannot silently bypass the redaction review.
		let Self {
			url,
			channel_prefix,
			group_prefix,
			message_expiry,
			password,
			username,
			tls,
			require_auth,
		} = self;
		f.debug_struct("RedisConfig")
			.field("url", url)
			.field("channel_prefix", channel_prefix)
			.field("group_prefix", group_prefix)
			.field("message_expiry", message_expiry)
			.field("password", &password.as_ref().map(|_| "[REDACTED]"))
			.field("username", username)
			.field("tls", tls)
			.field("require_auth", require_auth)
			.finish()
	}
}

#[cfg(feature = "redis-channel")]
impl Default for RedisConfig {
	/// Creates default Redis configuration.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::redis_channel::RedisConfig;
	///
	/// let config = RedisConfig::default();
	/// assert_eq!(config.url, "redis://127.0.0.1:6379");
	/// assert_eq!(config.channel_prefix, "ws:channel:");
	/// assert_eq!(config.group_prefix, "ws:group:");
	/// assert_eq!(config.message_expiry, 60);
	/// assert!(config.password.is_none());
	/// assert!(config.username.is_none());
	/// assert!(!config.tls);
	/// assert!(config.require_auth);
	/// ```
	fn default() -> Self {
		Self {
			url: "redis://127.0.0.1:6379".to_string(),
			channel_prefix: "ws:channel:".to_string(),
			group_prefix: "ws:group:".to_string(),
			message_expiry: 60, // 60 seconds
			password: None,
			username: None,
			tls: false,
			require_auth: true, // Security: require auth by default
		}
	}
}

#[cfg(feature = "redis-channel")]
impl RedisConfig {
	/// Creates a new Redis configuration.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::redis_channel::RedisConfig;
	///
	/// let config = RedisConfig::new("redis://localhost:6379".to_string());
	/// assert_eq!(config.url, "redis://localhost:6379");
	/// ```
	pub fn new(url: String) -> Self {
		Self {
			url,
			..Default::default()
		}
	}

	/// Sets the channel prefix.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::redis_channel::RedisConfig;
	///
	/// let config = RedisConfig::default()
	///     .with_channel_prefix("custom:channel:".to_string());
	/// assert_eq!(config.channel_prefix, "custom:channel:");
	/// ```
	pub fn with_channel_prefix(mut self, prefix: String) -> Self {
		self.channel_prefix = prefix;
		self
	}

	/// Sets the group prefix.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::redis_channel::RedisConfig;
	///
	/// let config = RedisConfig::default()
	///     .with_group_prefix("custom:group:".to_string());
	/// assert_eq!(config.group_prefix, "custom:group:");
	/// ```
	pub fn with_group_prefix(mut self, prefix: String) -> Self {
		self.group_prefix = prefix;
		self
	}

	/// Sets the message expiry time.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::redis_channel::RedisConfig;
	///
	/// let config = RedisConfig::default()
	///     .with_message_expiry(120);
	/// assert_eq!(config.message_expiry, 120);
	/// ```
	pub fn with_message_expiry(mut self, expiry: u64) -> Self {
		self.message_expiry = expiry;
		self
	}

	/// Sets the Redis password for authentication.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::redis_channel::RedisConfig;
	///
	/// let config = RedisConfig::default()
	///     .with_password("secret".to_string());
	/// assert_eq!(config.password, Some("secret".to_string()));
	/// ```
	pub fn with_password(mut self, password: String) -> Self {
		self.password = Some(password);
		self
	}

	/// Sets the Redis username for authentication (Redis 6+ ACL).
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::redis_channel::RedisConfig;
	///
	/// let config = RedisConfig::default()
	///     .with_username("app_user".to_string());
	/// assert_eq!(config.username, Some("app_user".to_string()));
	/// ```
	pub fn with_username(mut self, username: String) -> Self {
		self.username = Some(username);
		self
	}

	/// Enables TLS for secure connection.
	///
	/// A `redis://` URL is upgraded to TLS when connecting.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::redis_channel::RedisConfig;
	///
	/// let config = RedisConfig::default().with_tls();
	/// assert!(config.tls);
	/// ```
	pub fn with_tls(mut self) -> Self {
		self.tls = true;
		self
	}

	/// Sets whether authentication is required.
	///
	/// By default, authentication is required for security.
	/// Set to `false` only for local development or trusted networks.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::redis_channel::RedisConfig;
	///
	/// let config = RedisConfig::default().with_require_auth(false);
	/// assert!(!config.require_auth);
	/// ```
	pub fn with_require_auth(mut self, require: bool) -> Self {
		self.require_auth = require;
		self
	}

	/// Validates authentication configuration and logs warnings.
	///
	/// A password embedded in `url` counts as configured. Returns an error if
	/// authentication is required but no password is configured.
	pub fn validate_auth(&self) -> Result<(), crate::channels::ChannelError> {
		let url_has_password = self
			.url
			.as_str()
			.into_connection_info()
			.is_ok_and(|info| info.redis_settings().password().is_some());
		self.check_auth(url_has_password || non_empty(self.password.as_deref()).is_some())
	}

	/// Enforces `require_auth` against whether a password is effectively configured.
	fn check_auth(&self, has_password: bool) -> ChannelResult<()> {
		if self.require_auth && !has_password {
			warn!(
				"Redis authentication is required but no password is configured. \
				 This is a security risk. Set a password using .with_password() or \
				 disable auth requirement using .with_require_auth(false) for local development only."
			);
			return Err(ChannelError::AuthenticationRequired);
		}

		if !has_password {
			warn!(
				"Redis connection is configured without authentication. \
				 This is not recommended for production environments."
			);
		}

		Ok(())
	}

	/// Builds the connection descriptor that [`RedisChannelLayer::new`] connects with.
	///
	/// Applies `username`, `password` and `tls` on top of `url`, rejects
	/// contradictory combinations, and enforces `require_auth` against the
	/// effective credentials. Error messages never include credential values.
	fn connection_info(&self) -> ChannelResult<ConnectionInfo> {
		let info = self
			.url
			.as_str()
			.into_connection_info()
			.map_err(|e| config_error(format_args!("invalid Redis URL: {e}")))?;

		let url_settings = info.redis_settings();
		let username = merge_credential(
			"username",
			url_settings.username(),
			self.username.as_deref(),
		)?;
		let password = merge_credential(
			"password",
			url_settings.password(),
			self.password.as_deref(),
		)?;
		if username.is_some() && password.is_none() {
			return Err(config_error(
				"a Redis username requires a password; Redis does not authenticate with a username alone",
			));
		}
		self.check_auth(password.is_some())?;

		let mut redis_settings = url_settings.clone();
		if let Some(username) = username {
			redis_settings = redis_settings.set_username(username);
		}
		if let Some(password) = password {
			redis_settings = redis_settings.set_password(password);
		}

		let addr = match (self.tls, info.addr()) {
			(false, addr) | (true, addr @ ConnectionAddr::TcpTls { .. }) => addr.clone(),
			(true, ConnectionAddr::Tcp(host, port)) => ConnectionAddr::TcpTls {
				host: host.clone(),
				port: *port,
				insecure: false,
				tls_params: None,
			},
			(true, addr) => {
				return Err(config_error(format_args!(
					"TLS was requested but the Redis address {addr} cannot use TLS"
				)));
			}
		};

		Ok(info.set_addr(addr).set_redis_settings(redis_settings))
	}
}

/// Treats an empty credential as absent.
#[cfg(feature = "redis-channel")]
fn non_empty(value: Option<&str>) -> Option<&str> {
	value.filter(|value| !value.is_empty())
}

/// Combines a credential embedded in the Redis URL with its dedicated field.
///
/// Returns an error when both are set to different values, so neither source
/// silently overrides the other.
#[cfg(feature = "redis-channel")]
fn merge_credential<'a>(
	name: &str,
	from_url: Option<&'a str>,
	from_field: Option<&'a str>,
) -> ChannelResult<Option<&'a str>> {
	match (non_empty(from_url), non_empty(from_field)) {
		(Some(url_value), Some(field_value)) if url_value != field_value => Err(config_error(
			format_args!("the Redis {name} embedded in the URL contradicts the configured {name}"),
		)),
		(url_value, field_value) => Ok(field_value.or(url_value)),
	}
}

/// Builds the error returned for an invalid Redis connection configuration.
#[cfg(feature = "redis-channel")]
fn config_error(message: impl std::fmt::Display) -> ChannelError {
	ChannelError::SendError(format!("Redis configuration error: {message}"))
}

/// Redis-backed channel layer
///
/// # Examples
///
/// ```no_run
/// use reinhardt_websockets::redis_channel::{RedisChannelLayer, RedisConfig};
///
/// # tokio_test::block_on(async {
/// let config = RedisConfig::default();
/// let layer = RedisChannelLayer::new(config).await.unwrap();
/// # });
/// ```
#[cfg(feature = "redis-channel")]
pub struct RedisChannelLayer {
	config: RedisConfig,
	connection: ConnectionManager,
}

#[cfg(feature = "redis-channel")]
impl RedisChannelLayer {
	/// Creates a new Redis channel layer.
	///
	/// The connection uses `url` together with the `username`, `password` and
	/// `tls` fields; see [`RedisConfig`] for how they combine.
	///
	/// # Errors
	///
	/// Returns [`ChannelError::AuthenticationRequired`] when `require_auth` is
	/// set without a password, and [`ChannelError::SendError`] when the
	/// configuration is contradictory or the connection fails.
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_websockets::redis_channel::{RedisChannelLayer, RedisConfig};
	///
	/// # tokio_test::block_on(async {
	/// let config = RedisConfig::default()
	///     .with_password("secret".to_string());
	/// let layer = RedisChannelLayer::new(config).await.unwrap();
	/// # });
	/// ```
	pub async fn new(config: RedisConfig) -> ChannelResult<Self> {
		// Build and validate the effective descriptor before connecting, so the
		// credential and TLS fields actually govern the connection.
		let connection_info = config.connection_info()?;

		let client = Client::open(connection_info)
			.map_err(|e| ChannelError::SendError(format!("Redis client error: {}", e)))?;

		let connection = ConnectionManager::new(client)
			.await
			.map_err(|e| ChannelError::SendError(format!("Redis connection error: {}", e)))?;

		Ok(Self { config, connection })
	}

	/// Generates a channel key.
	fn channel_key(&self, channel: &str) -> String {
		format!("{}{}", self.config.channel_prefix, channel)
	}

	/// Generates a group key.
	fn group_key(&self, group: &str) -> String {
		format!("{}{}", self.config.group_prefix, group)
	}

	/// Serializes a message.
	fn serialize_message(&self, message: &ChannelMessage) -> ChannelResult<String> {
		serde_json::to_string(message)
			.map_err(|e| ChannelError::SerializationError(format!("Serialize error: {}", e)))
	}

	/// Deserializes a message.
	fn deserialize_message(&self, data: &str) -> ChannelResult<ChannelMessage> {
		serde_json::from_str(data)
			.map_err(|e| ChannelError::SerializationError(format!("Deserialize error: {}", e)))
	}
}

#[cfg(feature = "redis-channel")]
#[async_trait]
impl ChannelLayer for RedisChannelLayer {
	async fn send(&self, channel: &str, message: ChannelMessage) -> ChannelResult<()> {
		let key = self.channel_key(channel);
		let data = self.serialize_message(&message)?;

		let mut conn = self.connection.clone();

		// Push message to list
		conn.rpush::<_, _, ()>(&key, &data)
			.await
			.map_err(|e| ChannelError::SendError(format!("Redis rpush error: {}", e)))?;

		// Set expiry time
		conn.expire::<_, ()>(&key, self.config.message_expiry as i64)
			.await
			.map_err(|e| ChannelError::SendError(format!("Redis expire error: {}", e)))?;

		Ok(())
	}

	async fn receive(&self, channel: &str) -> ChannelResult<Option<ChannelMessage>> {
		let key = self.channel_key(channel);
		let mut conn = self.connection.clone();

		// Get first message from list (non-blocking)
		let result: Option<String> = conn
			.lpop(&key, None)
			.await
			.map_err(|e| ChannelError::ReceiveError(format!("Redis lpop error: {}", e)))?;

		match result {
			Some(data) => {
				let message = self.deserialize_message(&data)?;
				Ok(Some(message))
			}
			None => Ok(None),
		}
	}

	async fn group_add(&self, group: &str, channel: &str) -> ChannelResult<()> {
		let key = self.group_key(group);
		let mut conn = self.connection.clone();

		// Add channel to group (set)
		conn.sadd::<_, _, ()>(&key, channel)
			.await
			.map_err(|e| ChannelError::SendError(format!("Redis sadd error: {}", e)))?;

		Ok(())
	}

	async fn group_discard(&self, group: &str, channel: &str) -> ChannelResult<()> {
		let key = self.group_key(group);
		let mut conn = self.connection.clone();

		// Remove channel from group
		conn.srem::<_, _, ()>(&key, channel)
			.await
			.map_err(|e| ChannelError::SendError(format!("Redis srem error: {}", e)))?;

		Ok(())
	}

	async fn group_send(&self, group: &str, message: ChannelMessage) -> ChannelResult<()> {
		let key = self.group_key(group);
		let mut conn = self.connection.clone();

		// Get all channels in the group
		let channels: Vec<String> = conn
			.smembers(&key)
			.await
			.map_err(|e| ChannelError::SendError(format!("Redis smembers error: {}", e)))?;

		if channels.is_empty() {
			return Err(ChannelError::GroupNotFound(group.to_string()));
		}

		// Send message to each channel
		for channel in channels {
			self.send(&channel, message.clone()).await?;
		}

		Ok(())
	}
}

/// Feature-gated placeholder: Empty type when redis-channel is disabled.
///
/// This type exists to allow code to compile when the `redis-channel` feature
/// is not enabled, but provides no functionality. To use Redis-backed channel
/// layers for distributed WebSocket systems, enable the feature:
///
/// ```toml
/// reinhardt-websockets = { version = "...", features = ["redis-channel"] }
/// ```
#[cfg(not(feature = "redis-channel"))]
pub struct RedisConfig;

#[cfg(not(feature = "redis-channel"))]
impl RedisConfig {
	/// Creates a placeholder configuration (no-op when feature is disabled).
	pub fn default() -> Self {
		Self
	}
}

/// Feature-gated placeholder: Empty type when redis-channel is disabled.
///
/// This type exists to allow code to compile when the `redis-channel` feature
/// is not enabled, but provides no functionality. To use Redis-backed channel
/// layers, enable the feature in Cargo.toml.
#[cfg(not(feature = "redis-channel"))]
pub struct RedisChannelLayer;

#[cfg(all(test, feature = "redis-channel"))]
mod tests {
	use super::*;
	use crate::connection::Message;
	use std::collections::HashMap;
	use std::sync::{Arc, Mutex};

	/// Mock Redis storage for testing
	///
	/// This mock provides an in-memory implementation of Redis operations
	/// needed for testing the RedisChannelLayer without requiring a real Redis instance.
	#[derive(Clone, Default)]
	struct MockRedisStorage {
		/// In-memory storage for lists (channel messages)
		lists: Arc<Mutex<HashMap<String, Vec<String>>>>,
		/// In-memory storage for sets (group members)
		sets: Arc<Mutex<HashMap<String, std::collections::HashSet<String>>>>,
	}

	impl MockRedisStorage {
		fn new() -> Self {
			Self::default()
		}

		fn rpush(&self, key: &str, value: &str) {
			let mut lists = self.lists.lock().unwrap();
			lists
				.entry(key.to_string())
				.or_default()
				.push(value.to_string());
		}

		fn lpop(&self, key: &str) -> Option<String> {
			let mut lists = self.lists.lock().unwrap();
			if let Some(list) = lists.get_mut(key)
				&& !list.is_empty()
			{
				return Some(list.remove(0));
			}
			None
		}

		fn sadd(&self, key: &str, member: &str) {
			let mut sets = self.sets.lock().unwrap();
			sets.entry(key.to_string())
				.or_default()
				.insert(member.to_string());
		}

		fn srem(&self, key: &str, member: &str) {
			let mut sets = self.sets.lock().unwrap();
			if let Some(set) = sets.get_mut(key) {
				set.remove(member);
			}
		}

		fn smembers(&self, key: &str) -> Vec<String> {
			let sets = self.sets.lock().unwrap();
			sets.get(key)
				.map(|set| set.iter().cloned().collect())
				.unwrap_or_default()
		}
	}

	/// Mock RedisChannelLayer for testing
	///
	/// This structure mimics the behavior of RedisChannelLayer but uses in-memory storage
	/// instead of a real Redis connection.
	struct MockRedisChannelLayer {
		config: RedisConfig,
		storage: MockRedisStorage,
	}

	impl MockRedisChannelLayer {
		fn new(config: RedisConfig) -> Self {
			Self {
				config,
				storage: MockRedisStorage::new(),
			}
		}

		fn channel_key(&self, channel: &str) -> String {
			format!("{}{}", self.config.channel_prefix, channel)
		}

		fn group_key(&self, group: &str) -> String {
			format!("{}{}", self.config.group_prefix, group)
		}

		fn serialize_message(&self, message: &ChannelMessage) -> ChannelResult<String> {
			serde_json::to_string(message)
				.map_err(|e| ChannelError::SerializationError(format!("Serialize error: {}", e)))
		}

		fn deserialize_message(&self, data: &str) -> ChannelResult<ChannelMessage> {
			serde_json::from_str(data)
				.map_err(|e| ChannelError::SerializationError(format!("Deserialize error: {}", e)))
		}

		async fn send(&self, channel: &str, message: ChannelMessage) -> ChannelResult<()> {
			let key = self.channel_key(channel);
			let data = self.serialize_message(&message)?;
			self.storage.rpush(&key, &data);
			Ok(())
		}

		async fn receive(&self, channel: &str) -> ChannelResult<Option<ChannelMessage>> {
			let key = self.channel_key(channel);
			match self.storage.lpop(&key) {
				Some(data) => {
					let message = self.deserialize_message(&data)?;
					Ok(Some(message))
				}
				None => Ok(None),
			}
		}

		async fn group_add(&self, group: &str, channel: &str) -> ChannelResult<()> {
			let key = self.group_key(group);
			self.storage.sadd(&key, channel);
			Ok(())
		}

		async fn group_discard(&self, group: &str, channel: &str) -> ChannelResult<()> {
			let key = self.group_key(group);
			self.storage.srem(&key, channel);
			Ok(())
		}

		async fn group_send(&self, group: &str, message: ChannelMessage) -> ChannelResult<()> {
			let key = self.group_key(group);
			let channels = self.storage.smembers(&key);

			if channels.is_empty() {
				return Err(ChannelError::GroupNotFound(group.to_string()));
			}

			for channel in channels {
				self.send(&channel, message.clone()).await?;
			}

			Ok(())
		}
	}

	#[test]
	fn test_redis_config_default() {
		let config = RedisConfig::default();
		assert_eq!(config.url, "redis://127.0.0.1:6379");
		assert_eq!(config.channel_prefix, "ws:channel:");
		assert_eq!(config.group_prefix, "ws:group:");
		assert_eq!(config.message_expiry, 60);
		assert!(config.password.is_none());
		assert!(config.username.is_none());
		assert!(!config.tls);
		assert!(config.require_auth); // Security: auth required by default
	}

	#[test]
	fn test_redis_config_builder() {
		let config = RedisConfig::new("redis://localhost:6379".to_string())
			.with_channel_prefix("custom:channel:".to_string())
			.with_group_prefix("custom:group:".to_string())
			.with_message_expiry(120)
			// codeql[rust/hard-coded-cryptographic-value] -- Test fixture credential, not a deployed secret.
			.with_password("secret".to_string())
			.with_username("app_user".to_string())
			.with_tls()
			.with_require_auth(true);

		assert_eq!(config.url, "redis://localhost:6379");
		assert_eq!(config.channel_prefix, "custom:channel:");
		assert_eq!(config.group_prefix, "custom:group:");
		assert_eq!(config.message_expiry, 120);
		assert_eq!(config.password, Some("secret".to_string()));
		assert_eq!(config.username, Some("app_user".to_string()));
		assert!(config.tls);
		assert!(config.require_auth);
	}

	#[test]
	fn test_channel_key_generation() {
		let config = RedisConfig::default();
		let layer = MockRedisChannelLayer::new(config);
		assert_eq!(layer.channel_key("test"), "ws:channel:test");
	}

	#[test]
	fn test_group_key_generation() {
		let config = RedisConfig::default();
		let layer = MockRedisChannelLayer::new(config);
		assert_eq!(layer.group_key("test_group"), "ws:group:test_group");
	}

	#[test]
	fn test_message_serialization() {
		let config = RedisConfig::default();
		let layer = MockRedisChannelLayer::new(config);

		let msg = ChannelMessage::new("user_1".to_string(), Message::text("Hello".to_string()));

		let serialized = layer.serialize_message(&msg).unwrap();
		let deserialized = layer.deserialize_message(&serialized).unwrap();

		assert_eq!(msg.sender(), deserialized.sender());
	}

	#[tokio::test]
	async fn test_send_message_to_channel() {
		let config = RedisConfig::default();
		let layer = MockRedisChannelLayer::new(config);

		let msg = ChannelMessage::new(
			"user_1".to_string(),
			Message::text("Test message".to_string()),
		);

		// Send message
		let result = layer.send("test_channel", msg.clone()).await;
		assert!(result.is_ok());

		// Verify message was stored
		let key = layer.channel_key("test_channel");
		let stored = layer.storage.lpop(&key);
		assert!(stored.is_some());

		// Verify deserialized message
		let deserialized = layer.deserialize_message(&stored.unwrap()).unwrap();
		assert_eq!(deserialized.sender(), msg.sender());
	}

	#[tokio::test]
	async fn test_receive_message_from_channel() {
		let config = RedisConfig::default();
		let layer = MockRedisChannelLayer::new(config);

		let msg = ChannelMessage::new(
			"user_1".to_string(),
			Message::text("Test message".to_string()),
		);

		// Manually add message to mock storage
		let key = layer.channel_key("test_channel");
		let serialized = layer.serialize_message(&msg).unwrap();
		layer.storage.rpush(&key, &serialized);

		// Receive message
		let result = layer.receive("test_channel").await;
		let received = result.unwrap();
		assert!(received.is_some());
		assert_eq!(received.unwrap().sender(), msg.sender());

		// Verify queue is empty
		let result2 = layer.receive("test_channel").await;
		assert!(result2.is_ok());
		assert!(result2.unwrap().is_none());
	}

	#[tokio::test]
	async fn test_group_add_and_discard() {
		let config = RedisConfig::default();
		let layer = MockRedisChannelLayer::new(config);

		// Add channel to group
		let result = layer.group_add("test_group", "channel_1").await;
		assert!(result.is_ok());

		// Verify channel was added
		let key = layer.group_key("test_group");
		let members = layer.storage.smembers(&key);
		assert_eq!(members.len(), 1);
		assert!(members.contains(&"channel_1".to_string()));

		// Add another channel
		let result = layer.group_add("test_group", "channel_2").await;
		assert!(result.is_ok());

		let members = layer.storage.smembers(&key);
		assert_eq!(members.len(), 2);

		// Discard channel
		let result = layer.group_discard("test_group", "channel_1").await;
		assert!(result.is_ok());

		let members = layer.storage.smembers(&key);
		assert_eq!(members.len(), 1);
		assert!(members.contains(&"channel_2".to_string()));
	}

	#[tokio::test]
	async fn test_group_send() {
		let config = RedisConfig::default();
		let layer = MockRedisChannelLayer::new(config);

		// Add channels to group
		layer.group_add("test_group", "channel_1").await.unwrap();
		layer.group_add("test_group", "channel_2").await.unwrap();

		// Send message to group
		let msg = ChannelMessage::new(
			"user_1".to_string(),
			Message::text("Group message".to_string()),
		);

		let result = layer.group_send("test_group", msg.clone()).await;
		assert!(result.is_ok());

		// Verify both channels received the message
		let key1 = layer.channel_key("channel_1");
		let key2 = layer.channel_key("channel_2");

		let msg1 = layer.storage.lpop(&key1);
		let msg2 = layer.storage.lpop(&key2);

		assert!(msg1.is_some());
		assert!(msg2.is_some());

		let deserialized1 = layer.deserialize_message(&msg1.unwrap()).unwrap();
		let deserialized2 = layer.deserialize_message(&msg2.unwrap()).unwrap();

		assert_eq!(deserialized1.sender(), msg.sender());
		assert_eq!(deserialized2.sender(), msg.sender());
	}

	#[tokio::test]
	async fn test_group_send_to_empty_group() {
		let config = RedisConfig::default();
		let layer = MockRedisChannelLayer::new(config);

		let msg = ChannelMessage::new(
			"user_1".to_string(),
			Message::text("Test message".to_string()),
		);

		// Send to non-existent group
		let result = layer.group_send("empty_group", msg).await;
		assert!(result.is_err());

		match result {
			Err(ChannelError::GroupNotFound(group)) => {
				assert_eq!(group, "empty_group");
			}
			_ => panic!("Expected GroupNotFound error"),
		}
	}

	#[test]
	fn test_auth_validation_fails_without_password() {
		// Default config requires auth but has no password
		let config = RedisConfig::default();
		let result = config.validate_auth();
		assert!(result.is_err());
		match result {
			Err(ChannelError::AuthenticationRequired) => {}
			_ => panic!("Expected AuthenticationRequired error"),
		}
	}

	#[test]
	fn test_auth_validation_passes_with_password() {
		let config = RedisConfig::default()
			// codeql[rust/hard-coded-cryptographic-value] -- Test fixture credential, not a deployed secret.
			.with_password("secret".to_string());
		let result = config.validate_auth();
		assert!(result.is_ok());
	}

	#[test]
	fn test_auth_validation_passes_when_disabled() {
		// Auth disabled should not require password
		let config = RedisConfig::default().with_require_auth(false);
		let result = config.validate_auth();
		assert!(result.is_ok());
	}

	#[test]
	fn test_auth_config_with_all_fields() {
		let config = RedisConfig::default()
			// codeql[rust/hard-coded-cryptographic-value] -- Test fixture credential, not a deployed secret.
			.with_password("secret".to_string())
			.with_username("admin".to_string())
			.with_tls()
			.with_require_auth(true);

		assert_eq!(config.password, Some("secret".to_string()));
		assert_eq!(config.username, Some("admin".to_string()));
		assert!(config.tls);
		assert!(config.require_auth);

		// Should pass validation
		let result = config.validate_auth();
		assert!(result.is_ok());
	}

	#[rstest::rstest]
	fn test_redis_config_debug_redacts_password() {
		// Arrange
		let config = RedisConfig::new("redis://127.0.0.1:6379".to_string())
			.with_password("redis-config-password".to_string());

		// Act
		let output = format!("{config:?}");

		// Assert
		assert!(!output.contains("redis-config-password"), "{output}");
		assert!(
			output.contains("password: Some(\"[REDACTED]\")"),
			"{output}"
		);
	}

	/// Unwraps a configuration error raised before any connection attempt.
	fn expect_config_error(result: ChannelResult<ConnectionInfo>) -> String {
		match result {
			Err(ChannelError::SendError(message)) => message,
			Err(other) => panic!("expected a configuration error, got {other:?}"),
			Ok(info) => panic!("expected a configuration error, got {info:?}"),
		}
	}

	#[rstest::rstest]
	fn test_connection_info_applies_credential_fields() {
		// Arrange
		let config = RedisConfig::new("redis://127.0.0.1:6379/2".to_string())
			.with_username("app_user".to_string())
			.with_password("field-password".to_string());

		// Act
		let info = config.connection_info().unwrap();

		// Assert
		let redis = info.redis_settings();
		assert_eq!(redis.username(), Some("app_user"));
		assert_eq!(redis.password(), Some("field-password"));
		assert_eq!(redis.db(), 2);
		assert_eq!(
			info.addr(),
			&ConnectionAddr::Tcp("127.0.0.1".to_string(), 6379)
		);
	}

	#[rstest::rstest]
	fn test_connection_info_upgrades_plain_url_when_tls_requested() {
		// Arrange
		let config = RedisConfig::new("redis://cache.example.com:6380".to_string())
			.with_password("field-password".to_string())
			.with_tls();

		// Act
		let info = config.connection_info().unwrap();

		// Assert
		assert_eq!(
			info.addr(),
			&ConnectionAddr::TcpTls {
				host: "cache.example.com".to_string(),
				port: 6380,
				insecure: false,
				tls_params: None,
			}
		);
	}

	#[rstest::rstest]
	#[case::plain_url_without_tls("redis://cache.example.com:6380", false, false)]
	#[case::tls_url_without_tls_field("rediss://cache.example.com:6380", false, true)]
	#[case::tls_url_with_tls_field("rediss://cache.example.com:6380", true, true)]
	fn test_connection_info_transport_follows_url_and_tls_field(
		#[case] url: &str,
		#[case] tls: bool,
		#[case] expect_tls: bool,
	) {
		// Arrange
		let mut config =
			RedisConfig::new(url.to_string()).with_password("field-password".to_string());
		config.tls = tls;

		// Act
		let info = config.connection_info().unwrap();

		// Assert
		let expected = if expect_tls {
			ConnectionAddr::TcpTls {
				host: "cache.example.com".to_string(),
				port: 6380,
				insecure: false,
				tls_params: None,
			}
		} else {
			ConnectionAddr::Tcp("cache.example.com".to_string(), 6380)
		};
		assert_eq!(info.addr(), &expected);
	}

	#[cfg(unix)]
	#[rstest::rstest]
	fn test_connection_info_rejects_tls_for_unix_socket() {
		// Arrange
		let config = RedisConfig::new("redis+unix:///tmp/redis.sock".to_string())
			.with_password("field-password".to_string())
			.with_tls();

		// Act
		let message = expect_config_error(config.connection_info());

		// Assert
		assert_eq!(
			message,
			"Redis configuration error: TLS was requested but the Redis address /tmp/redis.sock cannot use TLS"
		);
	}

	#[rstest::rstest]
	#[case::password(
		"redis://:url-password@127.0.0.1:6379",
		None,
		Some("field-password"),
		"Redis configuration error: the Redis password embedded in the URL contradicts the configured password"
	)]
	#[case::username(
		"redis://url_user:shared-password@127.0.0.1:6379",
		Some("field_user"),
		None,
		"Redis configuration error: the Redis username embedded in the URL contradicts the configured username"
	)]
	#[case::username_without_password(
		"redis://127.0.0.1:6379",
		Some("app_user"),
		None,
		"Redis configuration error: a Redis username requires a password; Redis does not authenticate with a username alone"
	)]
	fn test_connection_info_rejects_contradictory_credentials(
		#[case] url: &str,
		#[case] username: Option<&str>,
		#[case] password: Option<&str>,
		#[case] expected: &str,
	) {
		// Arrange
		let mut config = RedisConfig::new(url.to_string()).with_require_auth(false);
		config.username = username.map(str::to_owned);
		config.password = password.map(str::to_owned);

		// Act
		let message = expect_config_error(config.connection_info());

		// Assert
		assert_eq!(message, expected);
		assert!(!message.contains("url-password"), "{message}");
		assert!(!message.contains("field-password"), "{message}");
		assert!(!message.contains("shared-password"), "{message}");
	}

	#[rstest::rstest]
	fn test_connection_info_accepts_matching_url_and_field_credentials() {
		// Arrange
		let config =
			RedisConfig::new("redis://app_user:shared-password@127.0.0.1:6379".to_string())
				.with_username("app_user".to_string())
				.with_password("shared-password".to_string());

		// Act
		let info = config.connection_info().unwrap();

		// Assert
		assert_eq!(info.redis_settings().username(), Some("app_user"));
		assert_eq!(info.redis_settings().password(), Some("shared-password"));
	}

	#[rstest::rstest]
	fn test_require_auth_accepts_password_embedded_in_url() {
		// Arrange
		let config = RedisConfig::new("redis://:url-password@127.0.0.1:6379".to_string());

		// Act
		let validated = config.validate_auth();
		let info = config.connection_info().unwrap();

		// Assert
		assert!(validated.is_ok(), "{validated:?}");
		assert_eq!(info.redis_settings().password(), Some("url-password"));
	}

	#[rstest::rstest]
	#[case::no_password(None)]
	#[case::empty_password(Some(""))]
	fn test_require_auth_rejects_missing_effective_password(#[case] password: Option<&str>) {
		// Arrange
		let mut config = RedisConfig::new("redis://127.0.0.1:6379".to_string());
		config.password = password.map(str::to_owned);

		// Act
		let result = config.connection_info();

		// Assert
		assert!(
			matches!(result, Err(ChannelError::AuthenticationRequired)),
			"{result:?}"
		);
	}

	#[rstest::rstest]
	#[tokio::test]
	async fn test_new_rejects_contradictory_configuration_before_connecting() {
		// Arrange
		let config = RedisConfig::new("redis://:url-password@127.0.0.1:1".to_string())
			.with_password("field-password".to_string());

		// Act
		let result = RedisChannelLayer::new(config).await;

		// Assert
		match result {
			Err(ChannelError::SendError(message)) => assert_eq!(
				message,
				"Redis configuration error: the Redis password embedded in the URL contradicts the configured password"
			),
			Err(other) => panic!("expected a configuration error, got {other:?}"),
			Ok(_) => panic!("expected a configuration error, got a connected layer"),
		}
	}
}
