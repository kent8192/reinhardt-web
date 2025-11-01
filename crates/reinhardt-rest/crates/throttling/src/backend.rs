use crate::throttle::ThrottleError;
use crate::time_provider::{SystemTimeProvider, TimeProvider};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};

#[async_trait]
pub trait ThrottleBackend: Send + Sync {
	async fn increment(&self, key: &str, window: u64) -> Result<usize, String>;
	async fn get_count(&self, key: &str) -> Result<usize, String>;

	/// Increment with Duration instead of u64
	async fn increment_duration(
		&self,
		key: &str,
		window: Duration,
	) -> Result<usize, ThrottleError> {
		self.increment(key, window.as_secs())
			.await
			.map_err(ThrottleError::ThrottleError)
	}

	/// Get wait time for rate limit
	async fn get_wait_time(&self, _key: &str) -> Result<Option<Duration>, ThrottleError> {
		// Default implementation returns None (not implemented)
		Ok(None)
	}
}

#[derive(Clone)]
pub struct MemoryBackend<T: TimeProvider = SystemTimeProvider> {
	storage: Arc<RwLock<HashMap<String, (usize, Instant)>>>,
	time_provider: Arc<T>,
}

impl MemoryBackend<SystemTimeProvider> {
	/// Creates a new `MemoryBackend` with the default system time provider.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_throttling::MemoryBackend;
	///
	/// let backend = MemoryBackend::new();
	// Backend is ready to track rate limits in memory
	/// ```
	pub fn new() -> Self {
		Self {
			storage: Arc::new(RwLock::new(HashMap::new())),
			time_provider: Arc::new(SystemTimeProvider::new()),
		}
	}
}

impl<T: TimeProvider> MemoryBackend<T> {
	/// Create a new MemoryBackend with a custom time provider
	pub fn with_time_provider(time_provider: Arc<T>) -> Self {
		Self {
			storage: Arc::new(RwLock::new(HashMap::new())),
			time_provider,
		}
	}
}

impl Default for MemoryBackend<SystemTimeProvider> {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl<T: TimeProvider> ThrottleBackend for MemoryBackend<T> {
	async fn increment(&self, key: &str, window_secs: u64) -> Result<usize, String> {
		let mut storage = self.storage.write().await;
		let now = self.time_provider.now();
		let entry = storage.entry(key.to_string()).or_insert((0, now));
		if now.duration_since(entry.1) > Duration::from_secs(window_secs) {
			*entry = (1, now);
			Ok(1)
		} else {
			entry.0 += 1;
			Ok(entry.0)
		}
	}
	async fn get_count(&self, key: &str) -> Result<usize, String> {
		let storage = self.storage.read().await;
		Ok(storage.get(key).map(|(count, _)| *count).unwrap_or(0))
	}
}

#[cfg(feature = "redis-backend")]
pub struct RedisBackend {
	client: redis::Client,
}

#[cfg(feature = "redis-backend")]
impl RedisBackend {
	/// Creates a new `RedisBackend` connected to the specified Redis URL.
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_throttling::RedisBackend;
	///
	/// let backend = RedisBackend::new("redis://127.0.0.1:6379").unwrap();
	// Backend is now connected to Redis for distributed rate limiting
	/// ```
	pub fn new(url: &str) -> Result<Self, String> {
		let client = redis::Client::open(url).map_err(|e| e.to_string())?;
		Ok(Self { client })
	}
}

#[cfg(feature = "redis-backend")]
#[async_trait]
impl ThrottleBackend for RedisBackend {
	async fn increment(&self, key: &str, window: u64) -> Result<usize, String> {
		use redis::AsyncCommands;
		let mut conn = self
			.client
			.get_multiplexed_async_connection()
			.await
			.map_err(|e| e.to_string())?;
		let count: usize = conn.incr(key, 1).await.map_err(|e| e.to_string())?;
		if count == 1 {
			let _: () = conn
				.expire(key, window as i64)
				.await
				.map_err(|e| e.to_string())?;
		}
		Ok(count)
	}
	async fn get_count(&self, key: &str) -> Result<usize, String> {
		use redis::AsyncCommands;
		let mut conn = self
			.client
			.get_multiplexed_async_connection()
			.await
			.map_err(|e| e.to_string())?;
		conn.get(key).await.map_err(|e| e.to_string())
	}
}

#[cfg(feature = "memcached-backend")]
pub struct MemcachedBackend {
	pool: bb8::Pool<bb8_memcached::MemcacheConnectionManager>,
}

#[cfg(feature = "memcached-backend")]
impl MemcachedBackend {
	/// Creates a new `MemcachedBackend` connected to the specified Memcached servers.
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_throttling::MemcachedBackend;
	///
	/// # tokio_test::block_on(async {
	/// let backend = MemcachedBackend::new(vec!["memcache://127.0.0.1:11211"]).await.unwrap();
	/// // Backend is now connected to Memcached for distributed rate limiting
	/// # });
	/// ```
	pub async fn new(urls: Vec<&str>) -> Result<Self, String> {
		if urls.is_empty() {
			return Err("At least one URL must be provided".to_string());
		}
		// bb8-memcached accepts a single URL or comma-separated URLs
		let url = urls.join(",");
		let manager = bb8_memcached::MemcacheConnectionManager::new(url)
			.map_err(|e| format!("Failed to create connection manager: {}", e))?;
		let pool = bb8::Pool::builder()
			.build(manager)
			.await
			.map_err(|e| format!("Failed to create connection pool: {}", e))?;
		Ok(Self { pool })
	}

	/// Creates a backend from a single URL
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_throttling::MemcachedBackend;
	///
	/// # tokio_test::block_on(async {
	/// let backend = MemcachedBackend::from_url("memcache://127.0.0.1:11211").await.unwrap();
	/// # });
	/// ```
	pub async fn from_url(url: &str) -> Result<Self, String> {
		Self::new(vec![url]).await
	}
}

#[cfg(feature = "memcached-backend")]
#[async_trait]
impl ThrottleBackend for MemcachedBackend {
	async fn increment(&self, key: &str, window: u64) -> Result<usize, String> {
		let mut conn = self
			.pool
			.get()
			.await
			.map_err(|e| format!("Failed to get connection: {}", e))?;

		// Try to increment the key
		match conn.increment(&key.to_string(), 1).await {
			Ok(value) => Ok(value as usize),
			Err(_) => {
				// Key doesn't exist, create it with initial value 1 and TTL
				let value_bytes = b"1";
				conn.set(&key.to_string(), value_bytes, window as u32)
					.await
					.map_err(|e| format!("Failed to set key: {}", e))?;
				Ok(1)
			}
		}
	}

	async fn get_count(&self, key: &str) -> Result<usize, String> {
		let mut conn = self
			.pool
			.get()
			.await
			.map_err(|e| format!("Failed to get connection: {}", e))?;

		match conn.get(&key.to_string()).await {
			Ok(bytes) => {
				// Parse bytes as string, then as number
				let value_str = String::from_utf8(bytes)
					.map_err(|e| format!("Failed to parse value as UTF-8: {}", e))?;
				let value = value_str
					.parse::<usize>()
					.map_err(|e| format!("Failed to parse value as number: {}", e))?;
				Ok(value)
			}
			Err(e) => {
				// Key doesn't exist
				if e.kind() == std::io::ErrorKind::NotFound {
					Ok(0)
				} else {
					Err(format!("Failed to get key: {}", e))
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_memory_backend_increment() {
		let backend = MemoryBackend::new();
		let key = "test_key";

		let count1 = backend.increment(key, 60).await.unwrap();
		assert_eq!(count1, 1);

		let count2 = backend.increment(key, 60).await.unwrap();
		assert_eq!(count2, 2);

		let count3 = backend.increment(key, 60).await.unwrap();
		assert_eq!(count3, 3);
	}

	#[tokio::test]
	async fn test_memory_backend_get_count() {
		let backend = MemoryBackend::new();
		let key = "test_key";

		let initial_count = backend.get_count(key).await.unwrap();
		assert_eq!(initial_count, 0);

		backend.increment(key, 60).await.unwrap();
		backend.increment(key, 60).await.unwrap();

		let count = backend.get_count(key).await.unwrap();
		assert_eq!(count, 2);
	}

	#[tokio::test]
	async fn test_memory_backend_increment_duration() {
		let backend = MemoryBackend::new();
		let key = "test_key";

		let count = backend
			.increment_duration(key, Duration::from_secs(60))
			.await
			.unwrap();
		assert_eq!(count, 1);
	}

	#[tokio::test]
	async fn test_memory_backend_separate_keys() {
		let backend = MemoryBackend::new();

		backend.increment("key1", 60).await.unwrap();
		backend.increment("key1", 60).await.unwrap();
		backend.increment("key2", 60).await.unwrap();

		let count1 = backend.get_count("key1").await.unwrap();
		let count2 = backend.get_count("key2").await.unwrap();

		assert_eq!(count1, 2);
		assert_eq!(count2, 1);
	}

	#[tokio::test]
	async fn test_memory_backend_default() {
		let backend = MemoryBackend::default();
		let count = backend.increment("test", 60).await.unwrap();
		assert_eq!(count, 1);
	}
}
