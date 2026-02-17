//! Cache warming functionality

use super::cache_trait::Cache;
use async_trait::async_trait;
use reinhardt_core::exception::Result;

/// Cache warmer trait
///
/// Implement this trait to define how to pre-populate cache on application startup.
///
/// # Examples
///
/// ```
/// use reinhardt_utils::cache::{Cache, InMemoryCache, CacheWarmer};
/// use async_trait::async_trait;
/// use std::sync::Arc;
///
/// struct UserCacheWarmer {
///     user_ids: Vec<i64>,
/// }
///
/// #[async_trait]
/// impl CacheWarmer<InMemoryCache> for UserCacheWarmer {
///     async fn warm(&self, cache: Arc<InMemoryCache>) -> reinhardt_core::exception::Result<()> {
///         for user_id in &self.user_ids {
///             // Simulate fetching user data
///             let user_data = format!("user_{}", user_id);
///             let key = format!("user:{}", user_id);
///             cache.set(&key, &user_data, None).await?;
///         }
///         Ok(())
///     }
/// }
///
/// # async fn example() -> reinhardt_core::exception::Result<()> {
/// let cache = Arc::new(InMemoryCache::new());
/// let warmer = UserCacheWarmer {
///     user_ids: vec![1, 2, 3],
/// };
///
/// // Warm the cache
/// warmer.warm(cache.clone()).await?;
///
/// // Cache is now pre-populated
/// let user: Option<String> = cache.get("user:1").await?;
/// assert_eq!(user, Some("user_1".to_string()));
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait CacheWarmer<C: Cache>: Send + Sync {
	/// Warm the cache by pre-populating entries
	async fn warm(&self, cache: std::sync::Arc<C>) -> Result<()>;
}

/// Function-based cache warmer
///
/// Allows using a function as a cache warmer without implementing the trait.
///
/// # Examples
///
/// ```
/// use reinhardt_utils::cache::{Cache, InMemoryCache, FunctionWarmer, CacheWarmer};
/// use std::sync::Arc;
///
/// # async fn example() -> reinhardt_core::exception::Result<()> {
/// let cache = Arc::new(InMemoryCache::new());
///
/// let warmer = FunctionWarmer::new(|cache: Arc<InMemoryCache>| {
///     Box::pin(async move {
///         cache.set("config:version", &"1.0.0", None).await?;
///         cache.set("config:env", &"production", None).await?;
///         Ok(())
///     })
/// });
///
/// warmer.warm(cache.clone()).await?;
///
/// let version: Option<String> = cache.get("config:version").await?;
/// assert_eq!(version, Some("1.0.0".to_string()));
/// # Ok(())
/// # }
/// ```
pub struct FunctionWarmer<C, F>
where
	C: Cache,
	F: Fn(
			std::sync::Arc<C>,
		) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
		+ Send
		+ Sync,
{
	func: F,
	_phantom: std::marker::PhantomData<C>,
}

impl<C, F> FunctionWarmer<C, F>
where
	C: Cache,
	F: Fn(
			std::sync::Arc<C>,
		) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
		+ Send
		+ Sync,
{
	/// Create a new function-based cache warmer
	pub fn new(func: F) -> Self {
		Self {
			func,
			_phantom: std::marker::PhantomData,
		}
	}
}

#[async_trait]
impl<C, F> CacheWarmer<C> for FunctionWarmer<C, F>
where
	C: Cache,
	F: Fn(
			std::sync::Arc<C>,
		) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
		+ Send
		+ Sync,
{
	async fn warm(&self, cache: std::sync::Arc<C>) -> Result<()> {
		(self.func)(cache).await
	}
}

/// Batch cache warmer
///
/// Combines multiple cache warmers and runs them in sequence.
///
/// # Examples
///
/// ```
/// use reinhardt_utils::cache::{Cache, InMemoryCache, BatchWarmer, FunctionWarmer, CacheWarmer};
/// use std::sync::Arc;
///
/// # async fn example() -> reinhardt_core::exception::Result<()> {
/// let cache = Arc::new(InMemoryCache::new());
///
/// let config_warmer = FunctionWarmer::new(|cache: Arc<InMemoryCache>| {
///     Box::pin(async move {
///         cache.set("config:version", &"1.0.0", None).await?;
///         Ok(())
///     })
/// });
///
/// let user_warmer = FunctionWarmer::new(|cache: Arc<InMemoryCache>| {
///     Box::pin(async move {
///         cache.set("user:1", &"admin", None).await?;
///         Ok(())
///     })
/// });
///
/// let batch = BatchWarmer::new()
///     .with_warmer(Box::new(config_warmer))
///     .with_warmer(Box::new(user_warmer));
///
/// batch.warm(cache.clone()).await?;
///
/// let version: Option<String> = cache.get("config:version").await?;
/// assert_eq!(version, Some("1.0.0".to_string()));
///
/// let user: Option<String> = cache.get("user:1").await?;
/// assert_eq!(user, Some("admin".to_string()));
/// # Ok(())
/// # }
/// ```
pub struct BatchWarmer<C: Cache> {
	warmers: Vec<Box<dyn CacheWarmer<C>>>,
}

impl<C: Cache> BatchWarmer<C> {
	/// Create a new batch warmer
	pub fn new() -> Self {
		Self {
			warmers: Vec::new(),
		}
	}

	/// Add a warmer to the batch
	pub fn with_warmer(mut self, warmer: Box<dyn CacheWarmer<C>>) -> Self {
		self.warmers.push(warmer);
		self
	}
}

impl<C: Cache> Default for BatchWarmer<C> {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl<C: Cache> CacheWarmer<C> for BatchWarmer<C> {
	async fn warm(&self, cache: std::sync::Arc<C>) -> Result<()> {
		for warmer in &self.warmers {
			warmer.warm(cache.clone()).await?;
		}
		Ok(())
	}
}

/// Parallel cache warmer
///
/// Runs multiple cache warmers in parallel for better performance.
///
/// # Examples
///
/// ```
/// use reinhardt_utils::cache::{Cache, InMemoryCache, ParallelWarmer, FunctionWarmer, CacheWarmer};
/// use std::sync::Arc;
///
/// # async fn example() -> reinhardt_core::exception::Result<()> {
/// let cache = Arc::new(InMemoryCache::new());
///
/// let warmer1 = FunctionWarmer::new(|cache: Arc<InMemoryCache>| {
///     Box::pin(async move {
///         cache.set("key1", &"value1", None).await?;
///         Ok(())
///     })
/// });
///
/// let warmer2 = FunctionWarmer::new(|cache: Arc<InMemoryCache>| {
///     Box::pin(async move {
///         cache.set("key2", &"value2", None).await?;
///         Ok(())
///     })
/// });
///
/// let parallel = ParallelWarmer::new()
///     .with_warmer(Box::new(warmer1))
///     .with_warmer(Box::new(warmer2));
///
/// parallel.warm(cache.clone()).await?;
///
/// let value1: Option<String> = cache.get("key1").await?;
/// let value2: Option<String> = cache.get("key2").await?;
/// assert_eq!(value1, Some("value1".to_string()));
/// assert_eq!(value2, Some("value2".to_string()));
/// # Ok(())
/// # }
/// ```
pub struct ParallelWarmer<C: Cache> {
	warmers: Vec<Box<dyn CacheWarmer<C>>>,
}

impl<C: Cache> ParallelWarmer<C> {
	/// Create a new parallel warmer
	pub fn new() -> Self {
		Self {
			warmers: Vec::new(),
		}
	}

	/// Add a warmer to run in parallel
	pub fn with_warmer(mut self, warmer: Box<dyn CacheWarmer<C>>) -> Self {
		self.warmers.push(warmer);
		self
	}
}

impl<C: Cache> Default for ParallelWarmer<C> {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl<C: Cache> CacheWarmer<C> for ParallelWarmer<C> {
	async fn warm(&self, cache: std::sync::Arc<C>) -> Result<()> {
		let tasks: Vec<_> = self
			.warmers
			.iter()
			.map(|warmer| {
				let cache = cache.clone();
				async move { warmer.warm(cache).await }
			})
			.collect();

		for result in futures::future::join_all(tasks).await {
			result?;
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::cache::InMemoryCache;
	use rstest::rstest;
	use std::sync::Arc;

	struct TestWarmer {
		key: String,
		value: String,
	}

	#[async_trait]
	impl CacheWarmer<InMemoryCache> for TestWarmer {
		async fn warm(&self, cache: Arc<InMemoryCache>) -> Result<()> {
			cache.set(&self.key, &self.value, None).await
		}
	}

	#[rstest]
	#[tokio::test]
	async fn test_basic_warmer() {
		let cache = Arc::new(InMemoryCache::new());
		let warmer = TestWarmer {
			key: "test_key".to_string(),
			value: "test_value".to_string(),
		};

		warmer.warm(cache.clone()).await.unwrap();

		let value: Option<String> = cache.get("test_key").await.unwrap();
		assert_eq!(value, Some("test_value".to_string()));
	}

	#[rstest]
	#[tokio::test]
	async fn test_function_warmer() {
		let cache = Arc::new(InMemoryCache::new());

		let warmer = FunctionWarmer::new(|cache: Arc<InMemoryCache>| {
			Box::pin(async move {
				cache.set("func_key", &"func_value", None).await?;
				Ok(())
			})
		});

		warmer.warm(cache.clone()).await.unwrap();

		let value: Option<String> = cache.get("func_key").await.unwrap();
		assert_eq!(value, Some("func_value".to_string()));
	}

	#[rstest]
	#[tokio::test]
	async fn test_batch_warmer() {
		let cache = Arc::new(InMemoryCache::new());

		let warmer1 = TestWarmer {
			key: "key1".to_string(),
			value: "value1".to_string(),
		};
		let warmer2 = TestWarmer {
			key: "key2".to_string(),
			value: "value2".to_string(),
		};

		let batch = BatchWarmer::new()
			.with_warmer(Box::new(warmer1))
			.with_warmer(Box::new(warmer2));

		batch.warm(cache.clone()).await.unwrap();

		let value1: Option<String> = cache.get("key1").await.unwrap();
		let value2: Option<String> = cache.get("key2").await.unwrap();
		assert_eq!(value1, Some("value1".to_string()));
		assert_eq!(value2, Some("value2".to_string()));
	}

	#[rstest]
	#[tokio::test]
	async fn test_parallel_warmer() {
		let cache = Arc::new(InMemoryCache::new());

		let warmer1 = TestWarmer {
			key: "parallel1".to_string(),
			value: "value1".to_string(),
		};
		let warmer2 = TestWarmer {
			key: "parallel2".to_string(),
			value: "value2".to_string(),
		};

		let parallel = ParallelWarmer::new()
			.with_warmer(Box::new(warmer1))
			.with_warmer(Box::new(warmer2));

		parallel.warm(cache.clone()).await.unwrap();

		let value1: Option<String> = cache.get("parallel1").await.unwrap();
		let value2: Option<String> = cache.get("parallel2").await.unwrap();
		assert_eq!(value1, Some("value1".to_string()));
		assert_eq!(value2, Some("value2".to_string()));
	}
}
