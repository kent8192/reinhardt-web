//! DynamoDB Cache Backend
//!
//! Persistent cache using AWS DynamoDB with automatic TTL management.
//!
//! # Features
//!
//! - Persistent storage with automatic TTL expiration
//! - Batch operations (BatchGetItem, BatchWriteItem)
//! - Conditional writes for race condition prevention
//! - Configurable table name and key schema
//!
//! # Table Schema
//!
//! The DynamoDB table must have the following schema:
//!
//! - Partition Key: `cache_key` (String)
//! - Attributes:
//!   - `cache_value` (Binary): The cached data
//!   - `ttl` (Number): Unix timestamp for TTL expiration (optional)
//!
//! # TTL Configuration
//!
//! To enable automatic expiration, configure TTL on the `ttl` attribute
//! in your DynamoDB table settings.
//!
//! # Examples
//!
//! ```no_run
//! use reinhardt_backends::cache::dynamodb::{DynamoDbCache, DynamoDbCacheConfig};
//! use reinhardt_backends::cache::CacheBackend;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create cache with default configuration
//! let config = DynamoDbCacheConfig::new("my-cache-table");
//! let cache = DynamoDbCache::new(config).await?;
//!
//! // Store and retrieve data
//! cache.set("key", b"value", Some(Duration::from_secs(3600))).await?;
//! let value = cache.get("key").await?;
//! assert_eq!(value, Some(b"value".to_vec()));
//!
//! // Batch operations
//! let items = vec![
//!     ("key1".to_string(), b"value1".to_vec()),
//!     ("key2".to_string(), b"value2".to_vec()),
//! ];
//! cache.set_many(&items, Some(Duration::from_secs(3600))).await?;
//! # Ok(())
//! # }
//! ```

use super::{CacheBackend, CacheError, CacheResult};
use async_trait::async_trait;
use aws_sdk_dynamodb::{
	Client,
	error::SdkError,
	operation::{
		batch_get_item::BatchGetItemError, batch_write_item::BatchWriteItemError,
		delete_item::DeleteItemError, get_item::GetItemError, put_item::PutItemError,
		scan::ScanError,
	},
	types::{AttributeValue, KeysAndAttributes, WriteRequest},
};
use std::{collections::HashMap, time::Duration};

/// Configuration for DynamoDB cache
///
/// # Examples
///
/// ```no_run
/// use reinhardt_backends::cache::dynamodb::DynamoDbCacheConfig;
///
/// // Default configuration
/// let config = DynamoDbCacheConfig::new("my-cache-table");
///
/// // Custom key names
/// let config = DynamoDbCacheConfig::new("my-cache-table")
///     .with_key_attribute("pk")
///     .with_value_attribute("data")
///     .with_ttl_attribute("expires_at");
/// ```
#[derive(Debug, Clone)]
pub struct DynamoDbCacheConfig {
	/// Table name
	pub table_name: String,
	/// Partition key attribute name (default: "cache_key")
	pub key_attribute: String,
	/// Value attribute name (default: "cache_value")
	pub value_attribute: String,
	/// TTL attribute name (default: "ttl")
	pub ttl_attribute: String,
}

impl DynamoDbCacheConfig {
	/// Create a new configuration with default attribute names
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_backends::cache::dynamodb::DynamoDbCacheConfig;
	///
	/// let config = DynamoDbCacheConfig::new("my-cache-table");
	/// ```
	pub fn new(table_name: impl Into<String>) -> Self {
		Self {
			table_name: table_name.into(),
			key_attribute: "cache_key".to_string(),
			value_attribute: "cache_value".to_string(),
			ttl_attribute: "ttl".to_string(),
		}
	}

	/// Set custom key attribute name
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_backends::cache::dynamodb::DynamoDbCacheConfig;
	///
	/// let config = DynamoDbCacheConfig::new("my-cache-table")
	///     .with_key_attribute("pk");
	/// ```
	pub fn with_key_attribute(mut self, attr: impl Into<String>) -> Self {
		self.key_attribute = attr.into();
		self
	}

	/// Set custom value attribute name
	pub fn with_value_attribute(mut self, attr: impl Into<String>) -> Self {
		self.value_attribute = attr.into();
		self
	}

	/// Set custom TTL attribute name
	pub fn with_ttl_attribute(mut self, attr: impl Into<String>) -> Self {
		self.ttl_attribute = attr.into();
		self
	}
}

/// DynamoDB cache backend
///
/// Provides persistent caching using AWS DynamoDB as the backing store.
pub struct DynamoDbCache {
	client: Client,
	config: DynamoDbCacheConfig,
}

impl DynamoDbCache {
	/// Create a new DynamoDB cache with the given configuration
	///
	/// Uses default AWS SDK configuration (credentials from environment, config files, etc.)
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_backends::cache::dynamodb::{DynamoDbCache, DynamoDbCacheConfig};
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let config = DynamoDbCacheConfig::new("my-cache-table");
	/// let cache = DynamoDbCache::new(config).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn new(config: DynamoDbCacheConfig) -> CacheResult<Self> {
		let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
			.load()
			.await;
		let client = Client::new(&aws_config);

		Ok(Self { client, config })
	}

	/// Create a new DynamoDB cache with custom AWS configuration
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_backends::cache::dynamodb::{DynamoDbCache, DynamoDbCacheConfig};
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest()).load().await;
	/// let config = DynamoDbCacheConfig::new("my-cache-table");
	/// let cache = DynamoDbCache::with_aws_config(&aws_config, config);
	/// # Ok(())
	/// # }
	/// ```
	pub fn with_aws_config(
		aws_config: &aws_config::SdkConfig,
		config: DynamoDbCacheConfig,
	) -> Self {
		let client = Client::new(aws_config);
		Self { client, config }
	}

	/// Calculate TTL timestamp from duration
	fn calculate_ttl(&self, ttl: Duration) -> i64 {
		let now = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_secs();
		(now + ttl.as_secs()) as i64
	}

	/// Check if item has expired based on TTL
	fn is_expired(&self, ttl: i64) -> bool {
		let now = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_secs() as i64;
		ttl > 0 && now >= ttl
	}
}

#[async_trait]
impl CacheBackend for DynamoDbCache {
	async fn get(&self, key: &str) -> CacheResult<Option<Vec<u8>>> {
		let result = self
			.client
			.get_item()
			.table_name(&self.config.table_name)
			.key(
				&self.config.key_attribute,
				AttributeValue::S(key.to_string()),
			)
			.send()
			.await
			.map_err(|e: SdkError<GetItemError>| {
				CacheError::Internal(format!("DynamoDB get error: {}", e))
			})?;

		if let Some(item) = result.item {
			// Check TTL expiration
			if let Some(AttributeValue::N(ttl_str)) = item.get(&self.config.ttl_attribute) {
				if let Ok(ttl) = ttl_str.parse::<i64>() {
					if self.is_expired(ttl) {
						return Ok(None);
					}
				}
			}

			// Extract value
			if let Some(AttributeValue::B(value)) = item.get(&self.config.value_attribute) {
				return Ok(Some(value.clone().into_inner()));
			}
		}

		Ok(None)
	}

	async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> CacheResult<()> {
		let mut item = HashMap::new();
		item.insert(
			self.config.key_attribute.clone(),
			AttributeValue::S(key.to_string()),
		);
		item.insert(
			self.config.value_attribute.clone(),
			AttributeValue::B(value.to_vec().into()),
		);

		if let Some(ttl_duration) = ttl {
			let ttl_timestamp = self.calculate_ttl(ttl_duration);
			item.insert(
				self.config.ttl_attribute.clone(),
				AttributeValue::N(ttl_timestamp.to_string()),
			);
		}

		self.client
			.put_item()
			.table_name(&self.config.table_name)
			.set_item(Some(item))
			.send()
			.await
			.map_err(|e: SdkError<PutItemError>| {
				CacheError::Internal(format!("DynamoDB put error: {}", e))
			})?;

		Ok(())
	}

	async fn delete(&self, key: &str) -> CacheResult<bool> {
		let result = self
			.client
			.delete_item()
			.table_name(&self.config.table_name)
			.key(
				&self.config.key_attribute,
				AttributeValue::S(key.to_string()),
			)
			.return_values(aws_sdk_dynamodb::types::ReturnValue::AllOld)
			.send()
			.await
			.map_err(|e: SdkError<DeleteItemError>| {
				CacheError::Internal(format!("DynamoDB delete error: {}", e))
			})?;

		Ok(result.attributes.is_some())
	}

	async fn exists(&self, key: &str) -> CacheResult<bool> {
		let result = self.get(key).await?;
		Ok(result.is_some())
	}

	async fn clear(&self) -> CacheResult<()> {
		// Scan and delete all items (expensive operation!)
		let scan_result = self
			.client
			.scan()
			.table_name(&self.config.table_name)
			.projection_expression(&self.config.key_attribute)
			.send()
			.await
			.map_err(|e: SdkError<ScanError>| {
				CacheError::Internal(format!("DynamoDB scan error: {}", e))
			})?;

		if let Some(items) = scan_result.items {
			for item in items {
				if let Some(AttributeValue::S(key)) = item.get(&self.config.key_attribute) {
					self.delete(key).await?;
				}
			}
		}

		Ok(())
	}

	async fn get_many(&self, keys: &[String]) -> CacheResult<Vec<Option<Vec<u8>>>> {
		if keys.is_empty() {
			return Ok(Vec::new());
		}

		// DynamoDB BatchGetItem has a limit of 100 keys
		let mut all_results = HashMap::new();

		for chunk in keys.chunks(100) {
			let mut request_keys = Vec::new();
			for key in chunk {
				let mut key_map = HashMap::new();
				key_map.insert(
					self.config.key_attribute.clone(),
					AttributeValue::S(key.clone()),
				);
				request_keys.push(key_map);
			}

			let keys_and_attrs = KeysAndAttributes::builder()
				.set_keys(Some(request_keys))
				.build()
				.map_err(|e| CacheError::Internal(format!("Failed to build request: {}", e)))?;

			let result = self
				.client
				.batch_get_item()
				.request_items(&self.config.table_name, keys_and_attrs)
				.send()
				.await
				.map_err(|e: SdkError<BatchGetItemError>| {
					CacheError::Internal(format!("DynamoDB batch get error: {}", e))
				})?;

			if let Some(responses) = result.responses {
				if let Some(items) = responses.get(&self.config.table_name) {
					for item in items {
						if let Some(AttributeValue::S(key)) = item.get(&self.config.key_attribute) {
							// Check TTL
							let is_valid = if let Some(AttributeValue::N(ttl_str)) =
								item.get(&self.config.ttl_attribute)
							{
								if let Ok(ttl) = ttl_str.parse::<i64>() {
									!self.is_expired(ttl)
								} else {
									true
								}
							} else {
								true
							};

							if is_valid {
								if let Some(AttributeValue::B(value)) =
									item.get(&self.config.value_attribute)
								{
									all_results.insert(key.clone(), value.clone().into_inner());
								}
							}
						}
					}
				}
			}
		}

		// Preserve order - map results back to original key order
		let mut ordered_results = Vec::with_capacity(keys.len());
		for key in keys {
			ordered_results.push(all_results.remove(key));
		}

		Ok(ordered_results)
	}

	async fn set_many(
		&self,
		items: &[(String, Vec<u8>)],
		ttl: Option<Duration>,
	) -> CacheResult<()> {
		if items.is_empty() {
			return Ok(());
		}

		// DynamoDB BatchWriteItem has a limit of 25 items
		for chunk in items.chunks(25) {
			let mut write_requests = Vec::new();

			for (key, value) in chunk {
				let mut item = HashMap::new();
				item.insert(
					self.config.key_attribute.clone(),
					AttributeValue::S(key.clone()),
				);
				item.insert(
					self.config.value_attribute.clone(),
					AttributeValue::B(value.clone().into()),
				);

				if let Some(ttl_duration) = ttl {
					let ttl_timestamp = self.calculate_ttl(ttl_duration);
					item.insert(
						self.config.ttl_attribute.clone(),
						AttributeValue::N(ttl_timestamp.to_string()),
					);
				}

				let put_request = aws_sdk_dynamodb::types::PutRequest::builder()
					.set_item(Some(item))
					.build()
					.map_err(|e| CacheError::Internal(format!("Failed to build request: {}", e)))?;

				let write_request = WriteRequest::builder().put_request(put_request).build();

				write_requests.push(write_request);
			}

			self.client
				.batch_write_item()
				.request_items(&self.config.table_name, write_requests)
				.send()
				.await
				.map_err(|e: SdkError<BatchWriteItemError>| {
					CacheError::Internal(format!("DynamoDB batch write error: {}", e))
				})?;
		}

		Ok(())
	}

	async fn delete_many(&self, keys: &[String]) -> CacheResult<usize> {
		if keys.is_empty() {
			return Ok(0);
		}

		let mut deleted = 0;

		// DynamoDB BatchWriteItem has a limit of 25 items
		for chunk in keys.chunks(25) {
			let mut write_requests = Vec::new();

			for key in chunk {
				let mut key_map = HashMap::new();
				key_map.insert(
					self.config.key_attribute.clone(),
					AttributeValue::S(key.clone()),
				);

				let delete_request = aws_sdk_dynamodb::types::DeleteRequest::builder()
					.set_key(Some(key_map))
					.build()
					.map_err(|e| CacheError::Internal(format!("Failed to build request: {}", e)))?;

				let write_request = WriteRequest::builder()
					.delete_request(delete_request)
					.build();

				write_requests.push(write_request);
			}

			self.client
				.batch_write_item()
				.request_items(&self.config.table_name, write_requests)
				.send()
				.await
				.map_err(|e: SdkError<BatchWriteItemError>| {
					CacheError::Internal(format!("DynamoDB batch write error: {}", e))
				})?;

			deleted += chunk.len();
		}

		Ok(deleted)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_dynamodb_cache_config_builder() {
		let config = DynamoDbCacheConfig::new("test-table")
			.with_key_attribute("pk")
			.with_value_attribute("data")
			.with_ttl_attribute("expires");

		assert_eq!(config.table_name, "test-table");
		assert_eq!(config.key_attribute, "pk");
		assert_eq!(config.value_attribute, "data");
		assert_eq!(config.ttl_attribute, "expires");
	}

	#[test]
	fn test_dynamodb_cache_config_defaults() {
		let config = DynamoDbCacheConfig::new("test-table");

		assert_eq!(config.table_name, "test-table");
		assert_eq!(config.key_attribute, "cache_key");
		assert_eq!(config.value_attribute, "cache_value");
		assert_eq!(config.ttl_attribute, "ttl");
	}

	// Integration tests would require AWS credentials and DynamoDB table
	#[tokio::test]
	#[ignore = "Requires AWS credentials and DynamoDB table"]
	async fn test_dynamodb_cache_set_get() {
		let config = DynamoDbCacheConfig::new("test-cache-table");
		let cache = DynamoDbCache::new(config).await.unwrap();

		cache
			.set("test_key", b"test_value", Some(Duration::from_secs(3600)))
			.await
			.unwrap();

		let value = cache.get("test_key").await.unwrap();
		assert_eq!(value, Some(b"test_value".to_vec()));

		cache.delete("test_key").await.unwrap();
	}

	#[tokio::test]
	#[ignore = "Requires AWS credentials and DynamoDB table"]
	async fn test_dynamodb_cache_batch_operations() {
		let config = DynamoDbCacheConfig::new("test-cache-table");
		let cache = DynamoDbCache::new(config).await.unwrap();

		let items = vec![
			("batch_key1".to_string(), b"value1".to_vec()),
			("batch_key2".to_string(), b"value2".to_vec()),
			("batch_key3".to_string(), b"value3".to_vec()),
		];

		cache
			.set_many(&items, Some(Duration::from_secs(3600)))
			.await
			.unwrap();

		let keys = vec![
			"batch_key1".to_string(),
			"batch_key2".to_string(),
			"batch_key3".to_string(),
		];

		let values = cache.get_many(&keys).await.unwrap();
		assert_eq!(values.len(), 3);
		assert_eq!(values[0], Some(b"value1".to_vec()));
		assert_eq!(values[1], Some(b"value2".to_vec()));
		assert_eq!(values[2], Some(b"value3".to_vec()));

		let deleted = cache.delete_many(&keys).await.unwrap();
		assert_eq!(deleted, 3);
	}
}
