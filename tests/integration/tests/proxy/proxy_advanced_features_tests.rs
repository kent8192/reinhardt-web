//! Advanced proxy features integration tests
//!
//! These tests verify advanced proxy functionality including lazy loading,
//! eager loading, caching, and complex relationship traversal.

use reinhardt_proxy::{EagerLoadConfig, LazyLoaded, LoadStrategy, RelationshipCache, ScalarValue};

#[tokio::test]
async fn test_lazy_loaded_basic() {
    // Create a lazy-loaded value that simulates loading from database
    let lazy = LazyLoaded::new(|| Box::pin(async { Ok(vec![1, 2, 3, 4, 5]) }));

    // Initially not loaded
    assert!(!lazy.is_loaded());

    // Access should trigger loading
    lazy.load().await.unwrap();

    // Now it should be loaded
    assert!(lazy.is_loaded());

    // Subsequent access should use cached value
    let data = lazy.get_if_loaded().unwrap();
    assert_eq!(data, &vec![1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn test_lazy_loaded_preloaded() {
    // Create a lazy-loaded value that's already loaded
    let lazy = LazyLoaded::preloaded(vec!["a", "b", "c"], || {
        Box::pin(async { Ok(vec!["x", "y", "z"]) })
    });

    // Should already be loaded
    assert!(lazy.is_loaded());

    // Should return the preloaded data
    let data = lazy.get_if_loaded().unwrap();
    assert_eq!(data, &vec!["a", "b", "c"]);
}

#[tokio::test]
async fn test_lazy_loaded_reset() {
    let lazy = LazyLoaded::new(|| Box::pin(async { Ok("initial value".to_string()) }));

    // Load the value
    lazy.load().await.unwrap();
    assert!(lazy.is_loaded());

    // Reset should clear the loaded state
    lazy.reset();
    assert!(!lazy.is_loaded());
}

#[tokio::test]
async fn test_eager_load_config() {
    // Test creating an eager load configuration
    let config = EagerLoadConfig::new()
        .with_relationship("posts")
        .with_relationship("comments")
        .with_relationship("tags")
        .max_depth(3);

    assert_eq!(config.max_depth, 3);
    assert_eq!(config.relationships.len(), 3);
    assert!(config.relationships.contains(&"posts".to_string()));
    assert!(config.relationships.contains(&"comments".to_string()));
    assert!(config.relationships.contains(&"tags".to_string()));
}

#[tokio::test]
async fn test_eager_load_config_default() {
    let config = EagerLoadConfig::default();

    assert_eq!(config.max_depth, 2);
    assert_eq!(config.relationships.len(), 0);
}

#[tokio::test]
async fn test_relationship_cache_basic() {
    let cache = RelationshipCache::new();

    // Initially empty
    assert!(!cache.contains("posts"));

    // Set a value
    cache.set("posts".to_string(), vec![1, 2, 3]);

    // Now it should contain the key
    assert!(cache.contains("posts"));

    // Get the value
    let posts: Vec<i32> = cache.get("posts").unwrap();
    assert_eq!(posts, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_relationship_cache_remove() {
    let cache = RelationshipCache::new();

    cache.set("key1".to_string(), "value1".to_string());
    cache.set("key2".to_string(), "value2".to_string());

    assert!(cache.contains("key1"));
    assert!(cache.contains("key2"));

    // Remove one key
    assert!(cache.remove("key1"));

    assert!(!cache.contains("key1"));
    assert!(cache.contains("key2"));
}

#[tokio::test]
async fn test_relationship_cache_clear() {
    let cache = RelationshipCache::new();

    cache.set("key1".to_string(), 100);
    cache.set("key2".to_string(), 200);
    cache.set("key3".to_string(), 300);

    assert!(cache.contains("key1"));
    assert!(cache.contains("key2"));
    assert!(cache.contains("key3"));

    // Clear all
    cache.clear();

    assert!(!cache.contains("key1"));
    assert!(!cache.contains("key2"));
    assert!(!cache.contains("key3"));
}

#[tokio::test]
async fn test_relationship_cache_different_types() {
    let cache = RelationshipCache::new();

    // Store different types
    cache.set("int".to_string(), 42_i32);
    cache.set("string".to_string(), "hello".to_string());
    cache.set("vec".to_string(), vec![1, 2, 3]);

    // Retrieve with correct types
    let int_value: i32 = cache.get("int").unwrap();
    assert_eq!(int_value, 42);

    let string_value: String = cache.get("string").unwrap();
    assert_eq!(string_value, "hello");

    let vec_value: Vec<i32> = cache.get("vec").unwrap();
    assert_eq!(vec_value, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_load_strategy_variants() {
    // Test that all load strategy variants exist
    let eager = LoadStrategy::Eager;
    let lazy = LoadStrategy::Lazy;
    let select = LoadStrategy::Select;
    let joined = LoadStrategy::Joined;

    assert_eq!(eager, LoadStrategy::Eager);
    assert_eq!(lazy, LoadStrategy::Lazy);
    assert_eq!(select, LoadStrategy::Select);
    assert_eq!(joined, LoadStrategy::Joined);
}

#[tokio::test]
async fn test_lazy_loaded_concurrent_access() {
    use std::sync::Arc;

    // Create a lazy-loaded value wrapped in Arc for concurrent access
    let lazy = Arc::new(LazyLoaded::new(|| {
        Box::pin(async {
            // Simulate expensive computation
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            Ok(vec![1, 2, 3, 4, 5])
        })
    }));

    // Clone for concurrent access
    let lazy1 = Arc::clone(&lazy);
    let lazy2 = Arc::clone(&lazy);

    // Spawn concurrent loads
    let handle1 = tokio::spawn(async move { lazy1.load().await });

    let handle2 = tokio::spawn(async move { lazy2.load().await });

    // Both should succeed
    handle1.await.unwrap().unwrap();
    handle2.await.unwrap().unwrap();

    // Should be loaded
    assert!(lazy.is_loaded());
}

#[tokio::test]
async fn test_relationship_cache_concurrent_access() {
    use std::sync::Arc;

    let cache = Arc::new(RelationshipCache::new());

    // Clone for concurrent access
    let cache1 = Arc::clone(&cache);
    let cache2 = Arc::clone(&cache);
    let cache3 = Arc::clone(&cache);

    // Spawn concurrent writes
    let handle1 = tokio::spawn(async move {
        cache1.set("key1".to_string(), 100);
    });

    let handle2 = tokio::spawn(async move {
        cache2.set("key2".to_string(), 200);
    });

    let handle3 = tokio::spawn(async move {
        cache3.set("key3".to_string(), 300);
    });

    // Wait for all writes
    handle1.await.unwrap();
    handle2.await.unwrap();
    handle3.await.unwrap();

    // All keys should be present
    assert!(cache.contains("key1"));
    assert!(cache.contains("key2"));
    assert!(cache.contains("key3"));

    // Values should be correct
    assert_eq!(cache.get::<i32>("key1").unwrap(), 100);
    assert_eq!(cache.get::<i32>("key2").unwrap(), 200);
    assert_eq!(cache.get::<i32>("key3").unwrap(), 300);
}

#[tokio::test]
async fn test_eager_load_with_nested_relationships() {
    // Test building a complex eager load configuration
    let config = EagerLoadConfig::new()
        .with_relationship("posts")
        .with_relationship("posts.comments")
        .with_relationship("posts.comments.author")
        .max_depth(3);

    assert_eq!(config.relationships.len(), 3);
    assert!(config.relationships.contains(&"posts".to_string()));
    assert!(config.relationships.contains(&"posts.comments".to_string()));
    assert!(config
        .relationships
        .contains(&"posts.comments.author".to_string()));
}

#[tokio::test]
async fn test_lazy_loaded_error_handling() {
    // Create a lazy-loaded value that fails to load
    let lazy = LazyLoaded::new(|| {
        Box::pin(async {
            Err(reinhardt_proxy::ProxyError::DatabaseError(
                "Connection failed".to_string(),
            ))
        })
    });

    // Load should fail
    let result = lazy.load().await;
    assert!(result.is_err());

    // Should still not be loaded
    assert!(!lazy.is_loaded());

    // get_if_loaded should return None
    assert!(lazy.get_if_loaded().is_none());
}

#[tokio::test]
async fn test_relationship_cache_get_nonexistent() {
    let cache = RelationshipCache::new();

    // Get non-existent key should return None
    let result: Option<i32> = cache.get("nonexistent");
    assert!(result.is_none());
}

#[tokio::test]
async fn test_relationship_cache_overwrite() {
    let cache = RelationshipCache::new();

    // Set initial value
    cache.set("key".to_string(), 100);
    assert_eq!(cache.get::<i32>("key").unwrap(), 100);

    // Overwrite with new value
    cache.set("key".to_string(), 200);
    assert_eq!(cache.get::<i32>("key").unwrap(), 200);
}
