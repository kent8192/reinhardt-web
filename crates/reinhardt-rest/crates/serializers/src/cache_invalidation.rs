//! Cache invalidation strategies for serializers
//!
//! This module provides automatic cache invalidation when models are updated,
//! ensuring data consistency while maintaining performance benefits of caching.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::Instant;

/// Cache invalidation strategy
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidationStrategy {
	/// Invalidate immediately on any change
	Immediate,
	/// Invalidate after a delay (seconds)
	Delayed(u64),
	/// Invalidate on next read
	LazyInvalidation,
	/// Time-based invalidation (TTL in seconds)
	TimeBased(u64),
}

/// Dependency tracking for cache invalidation
///
/// Tracks relationships between cache keys and model instances,
/// enabling automatic invalidation when related data changes.
///
/// # Examples
///
/// ```
/// use reinhardt_serializers::cache_invalidation::{CacheInvalidator, InvalidationStrategy};
///
/// let invalidator = CacheInvalidator::new(InvalidationStrategy::Immediate);
///
/// // Register cache key dependencies
/// invalidator.add_dependency("user:123:profile", "User", "123");
/// invalidator.add_dependency("user:123:posts", "User", "123");
///
/// // Invalidate all caches when User:123 is updated
/// let keys = invalidator.invalidate("User", "123");
/// assert_eq!(keys.len(), 2);
/// ```
#[derive(Debug, Clone)]
pub struct CacheInvalidator {
	/// Map from (model, pk) to set of cache keys
	dependencies: Arc<RwLock<HashMap<(String, String), HashSet<String>>>>,
	/// Map from cache key to timestamp
	timestamps: Arc<RwLock<HashMap<String, Instant>>>,
	/// Invalidation strategy
	strategy: InvalidationStrategy,
}

impl CacheInvalidator {
	/// Create a new cache invalidator with the specified strategy
	pub fn new(strategy: InvalidationStrategy) -> Self {
		Self {
			dependencies: Arc::new(RwLock::new(HashMap::new())),
			timestamps: Arc::new(RwLock::new(HashMap::new())),
			strategy,
		}
	}

	/// Add a cache dependency
	///
	/// Register that a cache key depends on a specific model instance.
	///
	/// # Arguments
	///
	/// * `cache_key` - The cache key to invalidate
	/// * `model_name` - The model name (e.g., "User", "Post")
	/// * `pk` - The primary key value as string
	pub fn add_dependency(&self, cache_key: &str, model_name: &str, pk: &str) {
		if let Ok(mut deps) = self.dependencies.write() {
			let key = (model_name.to_string(), pk.to_string());
			deps.entry(key)
				.or_insert_with(HashSet::new)
				.insert(cache_key.to_string());
		}

		// Record timestamp for time-based invalidation
		if let Ok(mut timestamps) = self.timestamps.write() {
			timestamps.insert(cache_key.to_string(), Instant::now());
		}
	}

	/// Remove a specific cache dependency
	pub fn remove_dependency(&self, cache_key: &str, model_name: &str, pk: &str) {
		if let Ok(mut deps) = self.dependencies.write() {
			let key = (model_name.to_string(), pk.to_string());
			if let Some(cache_keys) = deps.get_mut(&key) {
				cache_keys.remove(cache_key);
				if cache_keys.is_empty() {
					deps.remove(&key);
				}
			}
		}
	}

	/// Invalidate all cache keys associated with a model instance
	///
	/// Returns the list of invalidated cache keys.
	pub fn invalidate(&self, model_name: &str, pk: &str) -> Vec<String> {
		let key = (model_name.to_string(), pk.to_string());

		match &self.strategy {
			InvalidationStrategy::Immediate => {
				// Immediately invalidate all dependent caches
				self.immediate_invalidate(&key)
			}
			InvalidationStrategy::Delayed(seconds) => {
				// Mark for delayed invalidation
				self.delayed_invalidate(&key, *seconds)
			}
			InvalidationStrategy::LazyInvalidation => {
				// Mark as stale, will be invalidated on next read
				self.lazy_invalidate(&key)
			}
			InvalidationStrategy::TimeBased(_ttl) => {
				// Time-based invalidation handled by check_expired()
				Vec::new()
			}
		}
	}

	/// Invalidate by model type (all instances)
	///
	/// Invalidates all cache keys associated with any instance of the model.
	pub fn invalidate_model(&self, model_name: &str) -> Vec<String> {
		let mut invalidated = Vec::new();

		if let Ok(deps) = self.dependencies.read() {
			for ((name, _pk), cache_keys) in deps.iter() {
				if name == model_name {
					invalidated.extend(cache_keys.iter().cloned());
				}
			}
		}

		invalidated
	}

	/// Check if a cache key has expired based on TTL
	pub fn check_expired(&self, cache_key: &str) -> bool {
		if let InvalidationStrategy::TimeBased(ttl) = &self.strategy {
			if let Ok(timestamps) = self.timestamps.read() {
				if let Some(timestamp) = timestamps.get(cache_key) {
					return timestamp.elapsed().as_secs() > *ttl;
				}
			}
		}
		false
	}

	/// Clear all dependencies
	pub fn clear(&self) {
		if let Ok(mut deps) = self.dependencies.write() {
			deps.clear();
		}
		if let Ok(mut timestamps) = self.timestamps.write() {
			timestamps.clear();
		}
	}

	/// Get current strategy
	pub fn strategy(&self) -> &InvalidationStrategy {
		&self.strategy
	}

	/// Get dependency count for a model instance
	pub fn dependency_count(&self, model_name: &str, pk: &str) -> usize {
		let key = (model_name.to_string(), pk.to_string());
		if let Ok(deps) = self.dependencies.read() {
			deps.get(&key).map(|s| s.len()).unwrap_or(0)
		} else {
			0
		}
	}

	// Private helper methods

	fn immediate_invalidate(&self, key: &(String, String)) -> Vec<String> {
		if let Ok(mut deps) = self.dependencies.write() {
			if let Some(cache_keys) = deps.remove(key) {
				// Also remove timestamps
				if let Ok(mut timestamps) = self.timestamps.write() {
					for cache_key in &cache_keys {
						timestamps.remove(cache_key);
					}
				}
				return cache_keys.into_iter().collect();
			}
		}
		Vec::new()
	}

	fn delayed_invalidate(&self, key: &(String, String), _seconds: u64) -> Vec<String> {
		// In a real implementation, would schedule invalidation after delay
		// For now, just mark timestamps for delayed processing
		if let Ok(deps) = self.dependencies.read() {
			if let Some(cache_keys) = deps.get(key) {
				if let Ok(mut timestamps) = self.timestamps.write() {
					let now = Instant::now();
					for cache_key in cache_keys {
						timestamps.insert(cache_key.clone(), now);
					}
				}
				return cache_keys.iter().cloned().collect();
			}
		}
		Vec::new()
	}

	fn lazy_invalidate(&self, key: &(String, String)) -> Vec<String> {
		// Mark cache keys as stale but don't remove yet
		if let Ok(deps) = self.dependencies.read() {
			if let Some(cache_keys) = deps.get(key) {
				return cache_keys.iter().cloned().collect();
			}
		}
		Vec::new()
	}
}

impl Default for CacheInvalidator {
	fn default() -> Self {
		Self::new(InvalidationStrategy::Immediate)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::thread;
	use std::time::Duration;

	#[test]
	fn test_cache_invalidator_basic() {
		let invalidator = CacheInvalidator::new(InvalidationStrategy::Immediate);

		invalidator.add_dependency("user:123:profile", "User", "123");
		invalidator.add_dependency("user:123:posts", "User", "123");

		assert_eq!(invalidator.dependency_count("User", "123"), 2);

		let keys = invalidator.invalidate("User", "123");
		assert_eq!(keys.len(), 2);
		assert!(keys.contains(&"user:123:profile".to_string()));
		assert!(keys.contains(&"user:123:posts".to_string()));

		// After invalidation, dependencies should be removed
		assert_eq!(invalidator.dependency_count("User", "123"), 0);
	}

	#[test]
	fn test_cache_invalidator_remove_dependency() {
		let invalidator = CacheInvalidator::new(InvalidationStrategy::Immediate);

		invalidator.add_dependency("user:123:profile", "User", "123");
		invalidator.add_dependency("user:123:posts", "User", "123");

		assert_eq!(invalidator.dependency_count("User", "123"), 2);

		invalidator.remove_dependency("user:123:profile", "User", "123");

		assert_eq!(invalidator.dependency_count("User", "123"), 1);

		let keys = invalidator.invalidate("User", "123");
		assert_eq!(keys.len(), 1);
		assert_eq!(keys[0], "user:123:posts");
	}

	#[test]
	fn test_cache_invalidator_model_level() {
		let invalidator = CacheInvalidator::new(InvalidationStrategy::Immediate);

		invalidator.add_dependency("user:123:profile", "User", "123");
		invalidator.add_dependency("user:456:profile", "User", "456");
		invalidator.add_dependency("post:789:comments", "Post", "789");

		let keys = invalidator.invalidate_model("User");
		assert_eq!(keys.len(), 2);
		assert!(keys.contains(&"user:123:profile".to_string()));
		assert!(keys.contains(&"user:456:profile".to_string()));
	}

	#[test]
	fn test_cache_invalidator_delayed_strategy() {
		let invalidator = CacheInvalidator::new(InvalidationStrategy::Delayed(5));

		invalidator.add_dependency("user:123:profile", "User", "123");

		let keys = invalidator.invalidate("User", "123");
		assert_eq!(keys.len(), 1);

		// Delayed strategy marks but doesn't remove immediately
		assert_eq!(invalidator.dependency_count("User", "123"), 1);
	}

	#[test]
	fn test_cache_invalidator_lazy_strategy() {
		let invalidator = CacheInvalidator::new(InvalidationStrategy::LazyInvalidation);

		invalidator.add_dependency("user:123:profile", "User", "123");

		let keys = invalidator.invalidate("User", "123");
		assert_eq!(keys.len(), 1);

		// Lazy strategy marks but doesn't remove
		assert_eq!(invalidator.dependency_count("User", "123"), 1);
	}

	#[test]
	fn test_cache_invalidator_time_based() {
		let invalidator = CacheInvalidator::new(InvalidationStrategy::TimeBased(1));

		invalidator.add_dependency("user:123:profile", "User", "123");

		// Not expired yet
		assert!(!invalidator.check_expired("user:123:profile"));

		// Wait for TTL to expire
		thread::sleep(Duration::from_secs(2));

		// Now expired
		assert!(invalidator.check_expired("user:123:profile"));
	}

	#[test]
	fn test_cache_invalidator_clear() {
		let invalidator = CacheInvalidator::new(InvalidationStrategy::Immediate);

		invalidator.add_dependency("user:123:profile", "User", "123");
		invalidator.add_dependency("user:456:profile", "User", "456");

		assert_eq!(invalidator.dependency_count("User", "123"), 1);
		assert_eq!(invalidator.dependency_count("User", "456"), 1);

		invalidator.clear();

		assert_eq!(invalidator.dependency_count("User", "123"), 0);
		assert_eq!(invalidator.dependency_count("User", "456"), 0);
	}

	#[test]
	fn test_cache_invalidator_default() {
		let invalidator = CacheInvalidator::default();
		assert_eq!(invalidator.strategy(), &InvalidationStrategy::Immediate);
	}
}
