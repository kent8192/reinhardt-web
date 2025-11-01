//! Caching for negotiation results

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Cache key based on Accept header
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
	accept_header: String,
}

impl CacheKey {
	/// Creates a new CacheKey from Accept header
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_negotiation::cache::CacheKey;
	///
	/// let key = CacheKey::new("application/json");
	/// ```
	pub fn new(accept_header: impl Into<String>) -> Self {
		Self {
			accept_header: accept_header.into(),
		}
	}

	/// Creates a CacheKey from multiple headers
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_negotiation::cache::CacheKey;
	///
	/// let key = CacheKey::from_headers(&[
	///     ("Accept", "application/json"),
	///     ("Accept-Language", "en-US"),
	/// ]);
	/// ```
	pub fn from_headers(headers: &[(&str, &str)]) -> Self {
		let combined = headers
			.iter()
			.map(|(k, v)| format!("{}:{}", k, v))
			.collect::<Vec<_>>()
			.join(";");

		Self {
			accept_header: combined,
		}
	}
}

/// Cached negotiation result
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
	value: T,
	expires_at: Instant,
}

impl<T> CacheEntry<T> {
	/// Creates a new cache entry with TTL
	fn new(value: T, ttl: Duration) -> Self {
		Self {
			value,
			expires_at: Instant::now() + ttl,
		}
	}

	/// Checks if the entry has expired
	fn is_expired(&self) -> bool {
		Instant::now() > self.expires_at
	}
}

/// Cache for negotiation results
#[derive(Debug)]
pub struct NegotiationCache<T>
where
	T: Clone,
{
	cache: HashMap<CacheKey, CacheEntry<T>>,
	ttl: Duration,
	max_entries: usize,
}

impl<T> NegotiationCache<T>
where
	T: Clone,
{
	/// Creates a new NegotiationCache with default settings
	///
	/// Default TTL: 5 minutes, Max entries: 1000
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_negotiation::cache::NegotiationCache;
	/// use reinhardt_negotiation::MediaType;
	///
	/// let cache: NegotiationCache<MediaType> = NegotiationCache::new();
	/// ```
	pub fn new() -> Self {
		Self {
			cache: HashMap::new(),
			ttl: Duration::from_secs(300), // 5 minutes
			max_entries: 1000,
		}
	}

	/// Creates a cache with custom TTL
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_negotiation::cache::NegotiationCache;
	/// use reinhardt_negotiation::MediaType;
	/// use std::time::Duration;
	///
	/// let cache: NegotiationCache<MediaType> = NegotiationCache::with_ttl(
	///     Duration::from_secs(600)
	/// );
	/// ```
	pub fn with_ttl(ttl: Duration) -> Self {
		Self {
			cache: HashMap::new(),
			ttl,
			max_entries: 1000,
		}
	}

	/// Creates a cache with custom TTL and max entries
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_negotiation::cache::NegotiationCache;
	/// use reinhardt_negotiation::MediaType;
	/// use std::time::Duration;
	///
	/// let cache: NegotiationCache<MediaType> = NegotiationCache::with_config(
	///     Duration::from_secs(600),
	///     500
	/// );
	/// ```
	pub fn with_config(ttl: Duration, max_entries: usize) -> Self {
		Self {
			cache: HashMap::new(),
			ttl,
			max_entries,
		}
	}

	/// Gets a cached value if it exists and hasn't expired
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_negotiation::cache::{NegotiationCache, CacheKey};
	/// use reinhardt_negotiation::MediaType;
	///
	/// let mut cache: NegotiationCache<MediaType> = NegotiationCache::new();
	/// let key = CacheKey::new("application/json");
	/// let media_type = MediaType::new("application", "json");
	///
	/// cache.set(key.clone(), media_type.clone());
	///
	/// let result = cache.get(&key);
	/// assert!(result.is_some());
	/// assert_eq!(result.unwrap().subtype, "json");
	/// ```
	pub fn get(&mut self, key: &CacheKey) -> Option<T> {
		if let Some(entry) = self.cache.get(key) {
			if entry.is_expired() {
				self.cache.remove(key);
				return None;
			}
			return Some(entry.value.clone());
		}
		None
	}

	/// Sets a cached value
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_negotiation::cache::{NegotiationCache, CacheKey};
	/// use reinhardt_negotiation::MediaType;
	///
	/// let mut cache: NegotiationCache<MediaType> = NegotiationCache::new();
	/// let key = CacheKey::new("application/json");
	/// let media_type = MediaType::new("application", "json");
	///
	/// cache.set(key, media_type);
	/// ```
	pub fn set(&mut self, key: CacheKey, value: T) {
		// Evict oldest entries if cache is full
		if self.cache.len() >= self.max_entries {
			self.evict_oldest();
		}

		let entry = CacheEntry::new(value, self.ttl);
		self.cache.insert(key, entry);
	}

	/// Gets or computes a cached value
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_negotiation::cache::{NegotiationCache, CacheKey};
	/// use reinhardt_negotiation::MediaType;
	///
	/// let mut cache: NegotiationCache<MediaType> = NegotiationCache::new();
	/// let key = CacheKey::new("application/json");
	///
	/// let result = cache.get_or_compute(&key, || {
	///     MediaType::new("application", "json")
	/// });
	///
	/// assert_eq!(result.subtype, "json");
	///
	/// // Second call returns cached value
	/// let result2 = cache.get_or_compute(&key, || {
	///     unreachable!("Should not be called")
	/// });
	/// assert_eq!(result2.subtype, "json");
	/// ```
	pub fn get_or_compute<F>(&mut self, key: &CacheKey, compute: F) -> T
	where
		F: FnOnce() -> T,
	{
		if let Some(cached) = self.get(key) {
			return cached;
		}

		let value = compute();
		self.set(key.clone(), value.clone());
		value
	}

	/// Clears all expired entries
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_negotiation::cache::NegotiationCache;
	/// use reinhardt_negotiation::MediaType;
	///
	/// let mut cache: NegotiationCache<MediaType> = NegotiationCache::new();
	/// cache.clear_expired();
	/// ```
	pub fn clear_expired(&mut self) {
		self.cache.retain(|_, entry| !entry.is_expired());
	}

	/// Clears all entries
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_negotiation::cache::NegotiationCache;
	/// use reinhardt_negotiation::MediaType;
	///
	/// let mut cache: NegotiationCache<MediaType> = NegotiationCache::new();
	/// cache.clear();
	/// assert_eq!(cache.len(), 0);
	/// ```
	pub fn clear(&mut self) {
		self.cache.clear();
	}

	/// Returns the number of cached entries
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_negotiation::cache::{NegotiationCache, CacheKey};
	/// use reinhardt_negotiation::MediaType;
	///
	/// let mut cache: NegotiationCache<MediaType> = NegotiationCache::new();
	/// assert_eq!(cache.len(), 0);
	///
	/// let key = CacheKey::new("application/json");
	/// cache.set(key, MediaType::new("application", "json"));
	/// assert_eq!(cache.len(), 1);
	/// ```
	pub fn len(&self) -> usize {
		self.cache.len()
	}

	/// Checks if the cache is empty
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_negotiation::cache::NegotiationCache;
	/// use reinhardt_negotiation::MediaType;
	///
	/// let cache: NegotiationCache<MediaType> = NegotiationCache::new();
	/// assert!(cache.is_empty());
	/// ```
	pub fn is_empty(&self) -> bool {
		self.cache.is_empty()
	}

	/// Evicts the oldest entry (simple FIFO strategy)
	fn evict_oldest(&mut self) {
		if let Some(key) = self.cache.keys().next().cloned() {
			self.cache.remove(&key);
		}
	}
}

impl<T> Default for NegotiationCache<T>
where
	T: Clone,
{
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::MediaType;

	#[test]
	fn test_cache_key_new() {
		let key = CacheKey::new("application/json");
		assert_eq!(key.accept_header, "application/json");
	}

	#[test]
	fn test_cache_key_from_headers() {
		let key =
			CacheKey::from_headers(&[("Accept", "application/json"), ("Accept-Language", "en-US")]);
		assert!(key.accept_header.contains("Accept:application/json"));
		assert!(key.accept_header.contains("Accept-Language:en-US"));
	}

	#[test]
	fn test_cache_get_set() {
		let mut cache: NegotiationCache<MediaType> = NegotiationCache::new();
		let key = CacheKey::new("application/json");
		let media_type = MediaType::new("application", "json");

		cache.set(key.clone(), media_type);

		let result = cache.get(&key);
		assert!(result.is_some());
		assert_eq!(result.unwrap().subtype, "json");
	}

	#[test]
	fn test_cache_get_or_compute() {
		let mut cache: NegotiationCache<MediaType> = NegotiationCache::new();
		let key = CacheKey::new("application/json");

		let result = cache.get_or_compute(&key, || MediaType::new("application", "json"));
		assert_eq!(result.subtype, "json");

		// Second call should use cached value
		let mut called = false;
		let result2 = cache.get_or_compute(&key, || {
			called = true;
			MediaType::new("application", "xml")
		});
		assert!(!called);
		assert_eq!(result2.subtype, "json");
	}

	#[test]
	fn test_cache_expiration() {
		let mut cache: NegotiationCache<MediaType> =
			NegotiationCache::with_ttl(Duration::from_millis(10));
		let key = CacheKey::new("application/json");
		let media_type = MediaType::new("application", "json");

		cache.set(key.clone(), media_type);

		// Should exist immediately
		assert!(cache.get(&key).is_some());

		// Wait for expiration
		std::thread::sleep(Duration::from_millis(20));

		// Should be expired
		assert!(cache.get(&key).is_none());
	}

	#[test]
	fn test_cache_clear() {
		let mut cache: NegotiationCache<MediaType> = NegotiationCache::new();
		let key = CacheKey::new("application/json");
		cache.set(key, MediaType::new("application", "json"));

		assert_eq!(cache.len(), 1);
		cache.clear();
		assert_eq!(cache.len(), 0);
	}

	#[test]
	fn test_cache_max_entries() {
		let mut cache: NegotiationCache<MediaType> = NegotiationCache::with_config(
			Duration::from_secs(300),
			2, // Max 2 entries
		);

		cache.set(CacheKey::new("key1"), MediaType::new("application", "json"));
		cache.set(CacheKey::new("key2"), MediaType::new("text", "html"));

		assert_eq!(cache.len(), 2);

		// Adding third entry should evict oldest
		cache.set(CacheKey::new("key3"), MediaType::new("application", "xml"));

		assert_eq!(cache.len(), 2);
	}
}
