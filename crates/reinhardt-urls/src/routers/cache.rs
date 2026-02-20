//! Route caching for performance optimization.
//!
//! This module provides caching mechanisms for route resolution results,
//! reducing the overhead of repeated path matching operations.
//!
//! # Cache Strategies
//!
//! - `RouteCache`: Thread-safe LRU cache for route matching results
//! - Configurable cache size
//! - Automatic eviction of least recently used entries
//!
//! # Examples
//!
//! ```
//! use reinhardt_urls::routers::cache::RouteCache;
//! use std::collections::HashMap;
//!
//! let cache = RouteCache::new(100); // Cache up to 100 entries
//!
//! let mut params = HashMap::new();
//! params.insert("id".to_string(), "123".to_string());
//!
//! // Cache a route match result
//! cache.put("/users/123/", ("user-detail".to_string(), params.clone()));
//!
//! // Retrieve from cache
//! let cached = cache.get("/users/123/");
//! assert!(cached.is_some());
//! ```

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::{Arc, Mutex};

/// Route match result: (handler_id, path_params)
pub type RouteCacheEntry = (String, HashMap<String, String>);

/// Thread-safe LRU cache for route matching results
///
/// This cache stores the results of path matching operations to avoid
/// redundant regex matching and parameter extraction.
///
/// # Thread Safety
///
/// `RouteCache` uses a `Mutex` internally, making it safe to share
/// across threads. However, this introduces some locking overhead.
///
/// # Examples
///
/// ```
/// use reinhardt_urls::routers::cache::RouteCache;
/// use std::collections::HashMap;
///
/// let cache = RouteCache::new(100);
///
/// let mut params = HashMap::new();
/// params.insert("id".to_string(), "123".to_string());
///
/// cache.put("/users/123/", ("user-detail".to_string(), params.clone()));
///
/// let result = cache.get("/users/123/");
/// assert!(result.is_some());
/// assert_eq!(result.unwrap().0, "user-detail");
/// ```
#[derive(Clone)]
pub struct RouteCache {
	inner: Arc<Mutex<LruCache>>,
}

impl RouteCache {
	/// Create a new route cache with the specified capacity
	///
	/// # Arguments
	///
	/// * `capacity` - Maximum number of entries to cache
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::cache::RouteCache;
	///
	/// let cache = RouteCache::new(100);
	/// assert_eq!(cache.capacity(), 100);
	/// ```
	pub fn new(capacity: usize) -> Self {
		Self {
			inner: Arc::new(Mutex::new(LruCache::new(capacity))),
		}
	}

	/// Get a cached route match result
	///
	/// Returns `None` if the path is not in the cache.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::cache::RouteCache;
	/// use std::collections::HashMap;
	///
	/// let cache = RouteCache::new(100);
	/// assert!(cache.get("/users/").is_none());
	///
	/// cache.put("/users/", ("users".to_string(), HashMap::new()));
	/// assert!(cache.get("/users/").is_some());
	/// ```
	pub fn get(&self, path: &str) -> Option<RouteCacheEntry> {
		let mut inner = self.inner.lock().unwrap();
		inner.get(path)
	}

	/// Cache a route match result
	///
	/// # Arguments
	///
	/// * `path` - The request path
	/// * `entry` - The route match result (handler_id, params)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::cache::RouteCache;
	/// use std::collections::HashMap;
	///
	/// let cache = RouteCache::new(100);
	/// cache.put("/users/", ("users".to_string(), HashMap::new()));
	/// ```
	pub fn put(&self, path: &str, entry: RouteCacheEntry) {
		let mut inner = self.inner.lock().unwrap();
		inner.put(path.to_string(), entry);
	}

	/// Clear all cached entries
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::cache::RouteCache;
	/// use std::collections::HashMap;
	///
	/// let cache = RouteCache::new(100);
	/// cache.put("/users/", ("users".to_string(), HashMap::new()));
	/// assert_eq!(cache.len(), 1);
	///
	/// cache.clear();
	/// assert_eq!(cache.len(), 0);
	/// ```
	pub fn clear(&self) {
		let mut inner = self.inner.lock().unwrap();
		inner.clear();
	}

	/// Get the number of cached entries
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::cache::RouteCache;
	/// use std::collections::HashMap;
	///
	/// let cache = RouteCache::new(100);
	/// assert_eq!(cache.len(), 0);
	///
	/// cache.put("/users/", ("users".to_string(), HashMap::new()));
	/// assert_eq!(cache.len(), 1);
	/// ```
	pub fn len(&self) -> usize {
		let inner = self.inner.lock().unwrap();
		inner.len()
	}

	/// Check if the cache is empty
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::cache::RouteCache;
	///
	/// let cache = RouteCache::new(100);
	/// assert!(cache.is_empty());
	/// ```
	pub fn is_empty(&self) -> bool {
		let inner = self.inner.lock().unwrap();
		inner.is_empty()
	}

	/// Get the cache capacity
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::cache::RouteCache;
	///
	/// let cache = RouteCache::new(100);
	/// assert_eq!(cache.capacity(), 100);
	/// ```
	pub fn capacity(&self) -> usize {
		let inner = self.inner.lock().unwrap();
		inner.capacity()
	}
}

impl std::fmt::Debug for RouteCache {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RouteCache")
			.field("capacity", &self.capacity())
			.field("len", &self.len())
			.finish()
	}
}

/// Internal LRU cache implementation
///
/// This uses a HashMap for O(1) lookups combined with a min-heap (BinaryHeap)
/// for efficient O(log n) LRU eviction tracking.
///
/// # Memory Considerations
///
/// The heap accumulates stale entries as keys are accessed repeatedly (lazy cleanup
/// strategy). To prevent unbounded heap growth, the heap is compacted whenever it
/// exceeds `HEAP_COMPACTION_FACTOR` times the number of live map entries. The live
/// entry count is bounded by `capacity`, so total heap memory is bounded by
/// `capacity * HEAP_COMPACTION_FACTOR` entries at all times.
struct LruCache {
	capacity: usize,
	map: HashMap<String, (RouteCacheEntry, usize)>, // (entry, access_order)
	heap: BinaryHeap<Reverse<(usize, String)>>,     // min-heap of (access_order, key)
	access_counter: usize,
}

/// Maximum ratio of heap size to map size before compacting stale heap entries.
///
/// When `heap.len() > map.len() * HEAP_COMPACTION_FACTOR`, a full heap
/// compaction is triggered, removing all stale entries in O(n) time. This
/// bounds total heap memory while amortizing compaction cost.
const HEAP_COMPACTION_FACTOR: usize = 4;

impl LruCache {
	fn new(capacity: usize) -> Self {
		Self {
			capacity,
			map: HashMap::new(),
			heap: BinaryHeap::new(),
			access_counter: 0,
		}
	}

	fn get(&mut self, path: &str) -> Option<RouteCacheEntry> {
		if let Some((entry, order)) = self.map.get_mut(path) {
			// Update access order
			self.access_counter += 1;
			*order = self.access_counter;
			// Add new access time to heap (old entries will be cleaned up lazily)
			self.heap
				.push(Reverse((self.access_counter, path.to_string())));
			Some(entry.clone())
		} else {
			None
		}
	}

	fn put(&mut self, path: String, entry: RouteCacheEntry) {
		// If we're at capacity, evict the least recently used entry
		if self.map.len() >= self.capacity && !self.map.contains_key(&path) {
			self.evict_lru();
		}

		self.access_counter += 1;
		self.heap.push(Reverse((self.access_counter, path.clone())));
		self.map.insert(path, (entry, self.access_counter));

		// Compact the heap when it grows too large relative to the live entry count
		// to prevent unbounded heap memory growth from accumulated stale entries.
		if self.heap.len() > self.map.len().saturating_mul(HEAP_COMPACTION_FACTOR) {
			self.compact_heap();
		}
	}

	/// Rebuild the heap containing only live (non-stale) entries.
	///
	/// This runs in O(n log n) time where n is the number of live entries, which
	/// is bounded by `capacity`. It is invoked lazily only when the heap has grown
	/// beyond `HEAP_COMPACTION_FACTOR` times the live entry count.
	fn compact_heap(&mut self) {
		let mut new_heap = BinaryHeap::with_capacity(self.map.len());
		for (key, (_, access_time)) in &self.map {
			new_heap.push(Reverse((*access_time, key.clone())));
		}
		self.heap = new_heap;
	}

	/// Evict the least recently used entry
	///
	/// Uses a min-heap for O(log n) performance instead of O(n) linear scan.
	/// Lazy cleanup is used to handle stale heap entries.
	fn evict_lru(&mut self) {
		// Pop from heap until we find a valid LRU entry
		while let Some(Reverse((access_time, key))) = self.heap.pop() {
			// Check if this entry is still valid (not updated since)
			if let Some((_, current_access_time)) = self.map.get(&key)
				&& *current_access_time == access_time
			{
				// This is the true LRU entry
				self.map.remove(&key);
				return;
			}
			// Otherwise, this is a stale heap entry, continue to next
		}
	}

	fn clear(&mut self) {
		self.map.clear();
		self.heap.clear();
		self.access_counter = 0;
	}

	fn len(&self) -> usize {
		self.map.len()
	}

	fn is_empty(&self) -> bool {
		self.map.is_empty()
	}

	fn capacity(&self) -> usize {
		self.capacity
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_route_cache_new() {
		let cache = RouteCache::new(100);
		assert_eq!(cache.capacity(), 100);
		assert_eq!(cache.len(), 0);
		assert!(cache.is_empty());
	}

	#[test]
	fn test_route_cache_put_and_get() {
		let cache = RouteCache::new(100);

		let mut params = HashMap::new();
		params.insert("id".to_string(), "123".to_string());

		cache.put("/users/123/", ("user-detail".to_string(), params.clone()));

		let result = cache.get("/users/123/");
		assert!(result.is_some());

		let (handler_id, cached_params) = result.unwrap();
		assert_eq!(handler_id, "user-detail");
		assert_eq!(cached_params.get("id"), Some(&"123".to_string()));
	}

	#[test]
	fn test_route_cache_miss() {
		let cache = RouteCache::new(100);
		assert!(cache.get("/nonexistent/").is_none());
	}

	#[test]
	fn test_route_cache_clear() {
		let cache = RouteCache::new(100);

		cache.put("/users/", ("users".to_string(), HashMap::new()));
		assert_eq!(cache.len(), 1);

		cache.clear();
		assert_eq!(cache.len(), 0);
		assert!(cache.is_empty());
	}

	#[test]
	fn test_route_cache_lru_eviction() {
		let cache = RouteCache::new(2);

		cache.put("/route1/", ("handler1".to_string(), HashMap::new()));
		cache.put("/route2/", ("handler2".to_string(), HashMap::new()));
		assert_eq!(cache.len(), 2);

		// This should evict /route1/ (least recently used)
		cache.put("/route3/", ("handler3".to_string(), HashMap::new()));
		assert_eq!(cache.len(), 2);

		assert!(cache.get("/route1/").is_none());
		assert!(cache.get("/route2/").is_some());
		assert!(cache.get("/route3/").is_some());
	}

	#[test]
	fn test_route_cache_lru_access_order() {
		let cache = RouteCache::new(2);

		cache.put("/route1/", ("handler1".to_string(), HashMap::new()));
		cache.put("/route2/", ("handler2".to_string(), HashMap::new()));

		// Access route1 to update its order
		let _ = cache.get("/route1/");

		// This should evict /route2/ (now least recently used)
		cache.put("/route3/", ("handler3".to_string(), HashMap::new()));

		assert!(cache.get("/route1/").is_some());
		assert!(cache.get("/route2/").is_none());
		assert!(cache.get("/route3/").is_some());
	}

	#[test]
	fn test_route_cache_update_existing() {
		let cache = RouteCache::new(2);

		cache.put("/users/", ("handler1".to_string(), HashMap::new()));
		cache.put("/posts/", ("handler2".to_string(), HashMap::new()));

		// Update existing entry should not evict anything
		let mut new_params = HashMap::new();
		new_params.insert("new".to_string(), "param".to_string());
		cache.put("/users/", ("handler1_updated".to_string(), new_params));

		assert_eq!(cache.len(), 2);
		assert!(cache.get("/users/").is_some());
		assert!(cache.get("/posts/").is_some());
	}

	#[test]
	fn test_route_cache_thread_safety() {
		use std::thread;

		let cache = RouteCache::new(100);
		let cache_clone = cache.clone();

		let handle = thread::spawn(move || {
			cache_clone.put("/thread1/", ("handler1".to_string(), HashMap::new()));
		});

		cache.put("/main/", ("handler_main".to_string(), HashMap::new()));

		handle.join().unwrap();

		// Both entries should be present
		assert!(cache.get("/main/").is_some());
		assert!(cache.get("/thread1/").is_some());
	}
}
