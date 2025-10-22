//! Template caching for improved performance
//!
//! Note: With Tera template engine, templates are automatically cached by Tera itself.
//! This module is currently not used by the template rendering system, but is kept
//! for potential future use (e.g., caching rendered output).
//!
//! This module provides LRU (Least Recently Used) caching for template content
//! and rendered templates to improve performance in production environments.

#[cfg(feature = "templates")]
use lru::LruCache;
#[cfg(feature = "templates")]
use std::env;
#[cfg(feature = "templates")]
use std::num::NonZeroUsize;
#[cfg(feature = "templates")]
use std::sync::{Arc, Mutex};

/// Template cache using LRU eviction policy
///
/// This cache stores both raw template content and compiled template objects
/// to avoid repeated file I/O and template compilation overhead.
#[cfg(feature = "templates")]
pub struct TemplateCache {
    /// Cache for raw template content (file contents)
    content_cache: Arc<Mutex<LruCache<String, String>>>,

    /// Cache statistics for monitoring
    stats: Arc<Mutex<CacheStats>>,
}

/// Cache statistics for monitoring and debugging
#[cfg(feature = "templates")]
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: u64,

    /// Number of cache misses
    pub misses: u64,

    /// Number of items evicted from cache
    pub evictions: u64,
}

#[cfg(feature = "templates")]
impl CacheStats {
    /// Calculate cache hit rate as a percentage
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_shortcuts::template_cache::CacheStats;
    ///
    /// let stats = CacheStats {
    ///     hits: 80,
    ///     misses: 20,
    ///     evictions: 5,
    /// };
    ///
    /// assert_eq!(stats.hit_rate(), 80.0);
    /// ```
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }

    /// Reset all statistics to zero
    pub fn reset(&mut self) {
        self.hits = 0;
        self.misses = 0;
        self.evictions = 0;
    }
}

#[cfg(feature = "templates")]
impl TemplateCache {
    /// Create a new template cache with the specified capacity
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of templates to cache
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_shortcuts::template_cache::TemplateCache;
    ///
    /// let cache = TemplateCache::new(100);
    /// ```
    pub fn new(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(100).unwrap());

        Self {
            content_cache: Arc::new(Mutex::new(LruCache::new(capacity))),
            stats: Arc::new(Mutex::new(CacheStats::default())),
        }
    }

    /// Create a new template cache with capacity from environment variable
    ///
    /// Uses `REINHARDT_TEMPLATE_CACHE_SIZE` environment variable, defaults to 100
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_shortcuts::template_cache::TemplateCache;
    ///
    /// // With default capacity
    /// let cache = TemplateCache::from_env();
    ///
    /// // With environment variable set
    /// unsafe {
    ///     std::env::set_var("REINHARDT_TEMPLATE_CACHE_SIZE", "500");
    /// }
    /// let cache = TemplateCache::from_env();
    /// ```
    pub fn from_env() -> Self {
        let capacity = env::var("REINHARDT_TEMPLATE_CACHE_SIZE")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(100);

        Self::new(capacity)
    }

    /// Check if caching is enabled via environment variable
    ///
    /// Caching is disabled if `REINHARDT_DEBUG=true` or `REINHARDT_TEMPLATE_CACHE=false`
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_shortcuts::template_cache::TemplateCache;
    ///
    /// if TemplateCache::is_enabled() {
    ///     // Use caching
    /// } else {
    ///     // Skip caching in debug mode
    /// }
    /// ```
    pub fn is_enabled() -> bool {
        // Disable cache in debug mode
        if env::var("REINHARDT_DEBUG")
            .map(|v| v == "true")
            .unwrap_or(false)
        {
            return false;
        }

        // Check explicit cache disable
        env::var("REINHARDT_TEMPLATE_CACHE")
            .map(|v| v != "false")
            .unwrap_or(true)
    }

    /// Get template content from cache or None if not cached
    ///
    /// # Arguments
    ///
    /// * `key` - Template name/path to look up
    ///
    /// # Returns
    ///
    /// Some(content) if cached, None otherwise
    pub fn get(&self, key: &str) -> Option<String> {
        let mut cache = self.content_cache.lock().unwrap();
        let result = cache.get(key).cloned();

        let mut stats = self.stats.lock().unwrap();
        if result.is_some() {
            stats.hits += 1;
        } else {
            stats.misses += 1;
        }

        result
    }

    /// Put template content into cache
    ///
    /// # Arguments
    ///
    /// * `key` - Template name/path
    /// * `content` - Template content to cache
    ///
    /// # Returns
    ///
    /// The evicted item if cache was full, None otherwise
    pub fn put(&self, key: String, content: String) -> Option<(String, String)> {
        let mut cache = self.content_cache.lock().unwrap();
        let evicted = cache.push(key, content);

        if evicted.is_some() {
            let mut stats = self.stats.lock().unwrap();
            stats.evictions += 1;
        }

        evicted
    }

    /// Clear all cached templates
    pub fn clear(&self) {
        let mut cache = self.content_cache.lock().unwrap();
        cache.clear();
    }

    /// Get current cache statistics
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_shortcuts::template_cache::TemplateCache;
    ///
    /// let cache = TemplateCache::new(100);
    /// let stats = cache.stats();
    /// println!("Hit rate: {:.2}%", stats.hit_rate());
    /// ```
    pub fn stats(&self) -> CacheStats {
        self.stats.lock().unwrap().clone()
    }

    /// Reset cache statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock().unwrap();
        stats.reset();
    }

    /// Get current cache size (number of items)
    pub fn len(&self) -> usize {
        self.content_cache.lock().unwrap().len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.content_cache.lock().unwrap().is_empty()
    }
}

#[cfg(all(test, feature = "templates"))]
mod tests {
    use super::*;

    #[test]
    fn test_cache_new() {
        let cache = TemplateCache::new(50);
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_put_and_get() {
        let cache = TemplateCache::new(10);

        cache.put("template1".to_string(), "<h1>Hello</h1>".to_string());

        let result = cache.get("template1");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "<h1>Hello</h1>");
    }

    #[test]
    fn test_cache_miss() {
        let cache = TemplateCache::new(10);

        let result = cache.get("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_stats() {
        let cache = TemplateCache::new(10);

        cache.put("template1".to_string(), "content1".to_string());

        // Hit
        let _ = cache.get("template1");

        // Miss
        let _ = cache.get("template2");

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate(), 50.0);
    }

    #[test]
    fn test_cache_eviction() {
        let cache = TemplateCache::new(2);

        cache.put("template1".to_string(), "content1".to_string());
        cache.put("template2".to_string(), "content2".to_string());

        // This should evict template1 (LRU)
        let evicted = cache.put("template3".to_string(), "content3".to_string());

        assert!(evicted.is_some());
        assert_eq!(cache.len(), 2);

        // template1 should be evicted
        assert!(cache.get("template1").is_none());
        assert!(cache.get("template2").is_some());
        assert!(cache.get("template3").is_some());
    }

    #[test]
    fn test_cache_clear() {
        let cache = TemplateCache::new(10);

        cache.put("template1".to_string(), "content1".to_string());
        cache.put("template2".to_string(), "content2".to_string());

        assert_eq!(cache.len(), 2);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_stats_reset() {
        let cache = TemplateCache::new(10);

        cache.put("template1".to_string(), "content1".to_string());
        let _ = cache.get("template1");
        let _ = cache.get("template2");

        let stats = cache.stats();
        assert!(stats.hits > 0 || stats.misses > 0);

        cache.reset_stats();

        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn test_hit_rate_calculation() {
        let stats = CacheStats {
            hits: 75,
            misses: 25,
            evictions: 0,
        };

        assert_eq!(stats.hit_rate(), 75.0);
    }

    #[test]
    fn test_hit_rate_no_requests() {
        let stats = CacheStats::default();
        assert_eq!(stats.hit_rate(), 0.0);
    }
}
