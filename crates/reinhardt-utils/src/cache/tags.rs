//! Cache tags functionality

use super::cache_trait::Cache;
use async_trait::async_trait;
use reinhardt_core::exception::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Tagged cache trait
///
/// Extends the base cache interface with tag-based invalidation.
///
/// # Examples
///
/// ```
/// use reinhardt_utils::cache::{Cache, InMemoryCache, TaggedCache, TaggedCacheWrapper};
/// use std::sync::Arc;
///
/// # async fn example() -> reinhardt_core::exception::Result<()> {
/// let cache = Arc::new(InMemoryCache::new());
/// let tagged_cache = TaggedCacheWrapper::new(cache);
///
/// // Set a value with tags
/// tagged_cache.set_with_tags("user:1", &"John", None, &["users", "active"]).await?;
/// tagged_cache.set_with_tags("user:2", &"Jane", None, &["users", "active"]).await?;
/// tagged_cache.set_with_tags("post:1", &"Hello", None, &["posts"]).await?;
///
/// // Invalidate all entries with "users" tag
/// tagged_cache.invalidate_tag("users").await?;
///
/// // user:1 and user:2 are now invalidated
/// let user1: Option<String> = tagged_cache.get("user:1").await?;
/// assert_eq!(user1, None);
///
/// // post:1 is still valid
/// let post1: Option<String> = tagged_cache.get("post:1").await?;
/// assert_eq!(post1, Some("Hello".to_string()));
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait TaggedCache: Send + Sync {
	/// Set a value with associated tags
	async fn set_with_tags<T>(
		&self,
		key: &str,
		value: &T,
		ttl: Option<Duration>,
		tags: &[&str],
	) -> Result<()>
	where
		T: Serialize + Send + Sync;

	/// Get tags associated with a key
	async fn get_tags(&self, key: &str) -> Result<Vec<String>>;

	/// Invalidate all entries with a specific tag
	async fn invalidate_tag(&self, tag: &str) -> Result<()>;

	/// Invalidate all entries with any of the specified tags
	async fn invalidate_tags(&self, tags: &[&str]) -> Result<()>;

	/// Get all keys associated with a tag
	async fn get_keys_for_tag(&self, tag: &str) -> Result<Vec<String>>;

	/// Get a value from the cache (delegates to underlying cache)
	async fn get<T>(&self, key: &str) -> Result<Option<T>>
	where
		T: for<'de> Deserialize<'de> + Serialize + Send + Sync;

	/// Delete a value from the cache (delegates to underlying cache)
	async fn delete(&self, key: &str) -> Result<()>;
}

/// Tagged cache wrapper
///
/// Wraps a standard cache implementation and adds tag-based invalidation.
///
/// # Examples
///
/// ```
/// use reinhardt_utils::cache::{Cache, InMemoryCache, TaggedCacheWrapper, TaggedCache};
/// use std::sync::Arc;
///
/// # async fn example() -> reinhardt_core::exception::Result<()> {
/// let cache = Arc::new(InMemoryCache::new());
/// let tagged_cache = TaggedCacheWrapper::new(cache);
///
/// // Set values with tags
/// tagged_cache.set_with_tags("product:1", &"Laptop", None, &["products", "electronics"]).await?;
/// tagged_cache.set_with_tags("product:2", &"Mouse", None, &["products", "electronics"]).await?;
/// tagged_cache.set_with_tags("product:3", &"Book", None, &["products", "books"]).await?;
///
/// // Get all products
/// let keys = tagged_cache.get_keys_for_tag("products").await?;
/// assert_eq!(keys.len(), 3);
///
/// // Invalidate all electronics
/// tagged_cache.invalidate_tag("electronics").await?;
///
/// let product1: Option<String> = tagged_cache.get("product:1").await?;
/// let product3: Option<String> = tagged_cache.get("product:3").await?;
/// assert_eq!(product1, None); // Laptop was invalidated
/// assert_eq!(product3, Some("Book".to_string())); // Book is still valid
/// # Ok(())
/// # }
/// ```
pub struct TaggedCacheWrapper<C: Cache> {
	cache: Arc<C>,
	tag_index: Arc<RwLock<TagIndex>>,
}

#[derive(Default)]
struct TagIndex {
	// tag -> set of keys
	tag_to_keys: HashMap<String, HashSet<String>>,
	// key -> set of tags
	key_to_tags: HashMap<String, HashSet<String>>,
}

impl<C: Cache> TaggedCacheWrapper<C> {
	/// Create a new tagged cache wrapper
	pub fn new(cache: Arc<C>) -> Self {
		Self {
			cache,
			tag_index: Arc::new(RwLock::new(TagIndex::default())),
		}
	}

	/// Add tags to a key
	async fn add_tags(&self, key: &str, tags: &[&str]) {
		let mut index = self.tag_index.write().await;

		for tag in tags {
			index
				.tag_to_keys
				.entry(tag.to_string())
				.or_insert_with(HashSet::new)
				.insert(key.to_string());

			index
				.key_to_tags
				.entry(key.to_string())
				.or_insert_with(HashSet::new)
				.insert(tag.to_string());
		}
	}

	/// Remove a key from all tag indexes
	async fn remove_key_from_tags(&self, key: &str) {
		let mut index = self.tag_index.write().await;

		if let Some(tags) = index.key_to_tags.remove(key) {
			for tag in tags {
				if let Some(keys) = index.tag_to_keys.get_mut(&tag) {
					keys.remove(key);
					if keys.is_empty() {
						index.tag_to_keys.remove(&tag);
					}
				}
			}
		}
	}
}

#[async_trait]
impl<C: Cache> TaggedCache for TaggedCacheWrapper<C> {
	async fn set_with_tags<T>(
		&self,
		key: &str,
		value: &T,
		ttl: Option<Duration>,
		tags: &[&str],
	) -> Result<()>
	where
		T: Serialize + Send + Sync,
	{
		self.cache.set(key, value, ttl).await?;
		self.add_tags(key, tags).await;
		Ok(())
	}

	async fn get_tags(&self, key: &str) -> Result<Vec<String>> {
		let index = self.tag_index.read().await;
		Ok(index
			.key_to_tags
			.get(key)
			.map(|tags| tags.iter().cloned().collect())
			.unwrap_or_default())
	}

	async fn invalidate_tag(&self, tag: &str) -> Result<()> {
		let index = self.tag_index.read().await;

		if let Some(keys) = index.tag_to_keys.get(tag) {
			for key in keys {
				self.cache.delete(key).await?;
			}
		}

		drop(index);

		// Clean up tag index
		let mut index = self.tag_index.write().await;
		if let Some(keys) = index.tag_to_keys.remove(tag) {
			for key in &keys {
				if let Some(tags) = index.key_to_tags.remove(key) {
					// Remove this key from all other tags
					for other_tag in tags {
						if other_tag != tag
							&& let Some(tag_keys) = index.tag_to_keys.get_mut(&other_tag)
						{
							tag_keys.remove(key);
							if tag_keys.is_empty() {
								index.tag_to_keys.remove(&other_tag);
							}
						}
					}
				}
			}
		}

		Ok(())
	}

	async fn invalidate_tags(&self, tags: &[&str]) -> Result<()> {
		for tag in tags {
			self.invalidate_tag(tag).await?;
		}
		Ok(())
	}

	async fn get_keys_for_tag(&self, tag: &str) -> Result<Vec<String>> {
		let index = self.tag_index.read().await;
		Ok(index
			.tag_to_keys
			.get(tag)
			.map(|keys| keys.iter().cloned().collect())
			.unwrap_or_default())
	}

	async fn get<T>(&self, key: &str) -> Result<Option<T>>
	where
		T: for<'de> Deserialize<'de> + Serialize + Send + Sync,
	{
		self.cache.get(key).await
	}

	async fn delete(&self, key: &str) -> Result<()> {
		self.cache.delete(key).await?;
		self.remove_key_from_tags(key).await;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::cache::InMemoryCache;
	use rstest::rstest;

	#[rstest]
	#[tokio::test]
	async fn test_tagged_cache_basic() {
		let cache = Arc::new(InMemoryCache::new());
		let tagged = TaggedCacheWrapper::new(cache);

		// Set values with tags
		tagged
			.set_with_tags("key1", &"value1", None, &["tag1", "tag2"])
			.await
			.unwrap();
		tagged
			.set_with_tags("key2", &"value2", None, &["tag1"])
			.await
			.unwrap();

		// Get tags for key
		let tags = tagged.get_tags("key1").await.unwrap();
		assert_eq!(tags.len(), 2);
		assert!(tags.contains(&"tag1".to_string()));
		assert!(tags.contains(&"tag2".to_string()));

		// Get values
		let value1: Option<String> = tagged.get("key1").await.unwrap();
		let value2: Option<String> = tagged.get("key2").await.unwrap();
		assert_eq!(value1, Some("value1".to_string()));
		assert_eq!(value2, Some("value2".to_string()));
	}

	#[rstest]
	#[tokio::test]
	async fn test_tagged_cache_invalidate_tag() {
		let cache = Arc::new(InMemoryCache::new());
		let tagged = TaggedCacheWrapper::new(cache);

		// Set values with tags
		tagged
			.set_with_tags("user:1", &"Alice", None, &["users", "active"])
			.await
			.unwrap();
		tagged
			.set_with_tags("user:2", &"Bob", None, &["users", "active"])
			.await
			.unwrap();
		tagged
			.set_with_tags("post:1", &"Hello", None, &["posts"])
			.await
			.unwrap();

		// Invalidate "users" tag
		tagged.invalidate_tag("users").await.unwrap();

		// Users should be invalidated
		let user1: Option<String> = tagged.get("user:1").await.unwrap();
		let user2: Option<String> = tagged.get("user:2").await.unwrap();
		assert_eq!(user1, None);
		assert_eq!(user2, None);

		// Post should still exist
		let post1: Option<String> = tagged.get("post:1").await.unwrap();
		assert_eq!(post1, Some("Hello".to_string()));
	}

	#[rstest]
	#[tokio::test]
	async fn test_tagged_cache_get_keys_for_tag() {
		let cache = Arc::new(InMemoryCache::new());
		let tagged = TaggedCacheWrapper::new(cache);

		// Set values with tags
		tagged
			.set_with_tags("product:1", &"Laptop", None, &["products", "electronics"])
			.await
			.unwrap();
		tagged
			.set_with_tags("product:2", &"Mouse", None, &["products", "electronics"])
			.await
			.unwrap();
		tagged
			.set_with_tags("product:3", &"Book", None, &["products", "books"])
			.await
			.unwrap();

		// Get all products
		let mut keys = tagged.get_keys_for_tag("products").await.unwrap();
		keys.sort();
		assert_eq!(keys.len(), 3);
		assert!(keys.contains(&"product:1".to_string()));
		assert!(keys.contains(&"product:2".to_string()));
		assert!(keys.contains(&"product:3".to_string()));

		// Get electronics
		let mut electronics = tagged.get_keys_for_tag("electronics").await.unwrap();
		electronics.sort();
		assert_eq!(electronics.len(), 2);
		assert!(electronics.contains(&"product:1".to_string()));
		assert!(electronics.contains(&"product:2".to_string()));
	}

	#[rstest]
	#[tokio::test]
	async fn test_tagged_cache_invalidate_multiple_tags() {
		let cache = Arc::new(InMemoryCache::new());
		let tagged = TaggedCacheWrapper::new(cache);

		// Set values with different tags
		tagged
			.set_with_tags("item:1", &"A", None, &["tag1"])
			.await
			.unwrap();
		tagged
			.set_with_tags("item:2", &"B", None, &["tag2"])
			.await
			.unwrap();
		tagged
			.set_with_tags("item:3", &"C", None, &["tag3"])
			.await
			.unwrap();

		// Invalidate multiple tags
		tagged.invalidate_tags(&["tag1", "tag2"]).await.unwrap();

		// item:1 and item:2 should be invalidated
		let item1: Option<String> = tagged.get("item:1").await.unwrap();
		let item2: Option<String> = tagged.get("item:2").await.unwrap();
		assert_eq!(item1, None);
		assert_eq!(item2, None);

		// item:3 should still exist
		let item3: Option<String> = tagged.get("item:3").await.unwrap();
		assert_eq!(item3, Some("C".to_string()));
	}

	#[rstest]
	#[tokio::test]
	async fn test_tagged_cache_delete() {
		let cache = Arc::new(InMemoryCache::new());
		let tagged = TaggedCacheWrapper::new(cache);

		// Set value with tags
		tagged
			.set_with_tags("key1", &"value1", None, &["tag1", "tag2"])
			.await
			.unwrap();

		// Delete the key
		tagged.delete("key1").await.unwrap();

		// Key should be removed from tags
		let keys = tagged.get_keys_for_tag("tag1").await.unwrap();
		assert_eq!(keys.len(), 0);

		let keys = tagged.get_keys_for_tag("tag2").await.unwrap();
		assert_eq!(keys.len(), 0);

		// Value should be gone
		let value: Option<String> = tagged.get("key1").await.unwrap();
		assert_eq!(value, None);
	}

	#[rstest]
	#[tokio::test]
	async fn test_tagged_cache_overlapping_tags() {
		let cache = Arc::new(InMemoryCache::new());
		let tagged = TaggedCacheWrapper::new(cache);

		// Set values with overlapping tags
		tagged
			.set_with_tags("key1", &"value1", None, &["tag1", "common"])
			.await
			.unwrap();
		tagged
			.set_with_tags("key2", &"value2", None, &["tag2", "common"])
			.await
			.unwrap();

		// Invalidate common tag
		tagged.invalidate_tag("common").await.unwrap();

		// Both keys should be invalidated
		let value1: Option<String> = tagged.get("key1").await.unwrap();
		let value2: Option<String> = tagged.get("key2").await.unwrap();
		assert_eq!(value1, None);
		assert_eq!(value2, None);

		// Tags should be cleaned up
		let keys = tagged.get_keys_for_tag("tag1").await.unwrap();
		assert_eq!(keys.len(), 0);

		let keys = tagged.get_keys_for_tag("tag2").await.unwrap();
		assert_eq!(keys.len(), 0);
	}
}
