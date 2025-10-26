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

#[cfg(feature = "redis-channel")]
use crate::channels::{ChannelError, ChannelLayer, ChannelMessage, ChannelResult};
#[cfg(feature = "redis-channel")]
use async_trait::async_trait;
#[cfg(feature = "redis-channel")]
use redis::aio::ConnectionManager;
#[cfg(feature = "redis-channel")]
use redis::{AsyncCommands, Client};
#[cfg(feature = "redis-channel")]
use std::time::Duration;

/// Redis channel layer configuration
#[cfg(feature = "redis-channel")]
#[derive(Debug, Clone)]
pub struct RedisConfig {
    /// Redis connection URL
    pub url: String,
    /// Channel prefix
    pub channel_prefix: String,
    /// Group prefix
    pub group_prefix: String,
    /// Message expiry time (seconds)
    pub message_expiry: u64,
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
    /// ```
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:6379".to_string(),
            channel_prefix: "ws:channel:".to_string(),
            group_prefix: "ws:group:".to_string(),
            message_expiry: 60, // 60 seconds
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
    pub async fn new(config: RedisConfig) -> ChannelResult<Self> {
        let client = Client::open(config.url.as_str())
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

// Stub implementation when redis-channel feature is disabled
#[cfg(not(feature = "redis-channel"))]
pub struct RedisConfig;

#[cfg(not(feature = "redis-channel"))]
impl RedisConfig {
    pub fn default() -> Self {
        Self
    }
}

#[cfg(not(feature = "redis-channel"))]
pub struct RedisChannelLayer;

#[cfg(all(test, feature = "redis-channel"))]
mod tests {
    use super::*;
    use crate::connection::Message;

    #[test]
    fn test_redis_config_default() {
        let config = RedisConfig::default();
        assert_eq!(config.url, "redis://127.0.0.1:6379");
        assert_eq!(config.channel_prefix, "ws:channel:");
        assert_eq!(config.group_prefix, "ws:group:");
        assert_eq!(config.message_expiry, 60);
    }

    #[test]
    fn test_redis_config_builder() {
        let config = RedisConfig::new("redis://localhost:6379".to_string())
            .with_channel_prefix("custom:channel:".to_string())
            .with_group_prefix("custom:group:".to_string())
            .with_message_expiry(120);

        assert_eq!(config.url, "redis://localhost:6379");
        assert_eq!(config.channel_prefix, "custom:channel:");
        assert_eq!(config.group_prefix, "custom:group:");
        assert_eq!(config.message_expiry, 120);
    }

    #[test]
    fn test_channel_key_generation() {
        let config = RedisConfig::default();
        let layer = RedisChannelLayer {
            config: config.clone(),
            connection: todo!("Mock connection for testing"),
        };

        assert_eq!(layer.channel_key("test"), "ws:channel:test");
    }

    #[test]
    fn test_group_key_generation() {
        let config = RedisConfig::default();
        let layer = RedisChannelLayer {
            config: config.clone(),
            connection: todo!("Mock connection for testing"),
        };

        assert_eq!(layer.group_key("test_group"), "ws:group:test_group");
    }

    #[test]
    fn test_message_serialization() {
        let config = RedisConfig::default();
        let layer = RedisChannelLayer {
            config: config.clone(),
            connection: todo!("Mock connection for testing"),
        };

        let msg = ChannelMessage::new("user_1".to_string(), Message::text("Hello".to_string()));

        let serialized = layer.serialize_message(&msg).unwrap();
        let deserialized = layer.deserialize_message(&serialized).unwrap();

        assert_eq!(msg.sender(), deserialized.sender());
    }
}
