use reinhardt_negotiation::cache::{CacheKey, NegotiationCache};
use reinhardt_negotiation::MediaType;
use std::thread;
use std::time::Duration;

#[test]
fn test_cache_key_new() {
    let key = CacheKey::new("application/json");
    let key2 = CacheKey::new("application/json");

    assert_eq!(key, key2);
}

#[test]
fn test_cache_key_from_headers() {
    let key =
        CacheKey::from_headers(&[("Accept", "application/json"), ("Accept-Language", "en-US")]);

    // Different order should produce different keys
    let key2 =
        CacheKey::from_headers(&[("Accept-Language", "en-US"), ("Accept", "application/json")]);

    // Keys should be consistent
    assert_ne!(key, key2);
}

#[test]
fn test_cache_basic_operations() {
    let mut cache: NegotiationCache<MediaType> = NegotiationCache::new();
    let key = CacheKey::new("application/json");
    let media_type = MediaType::new("application", "json");

    // Initially empty
    assert!(cache.get(&key).is_none());
    assert!(cache.is_empty());

    // Set value
    cache.set(key.clone(), media_type.clone());
    assert_eq!(cache.len(), 1);
    assert!(!cache.is_empty());

    // Get value
    let result = cache.get(&key);
    assert!(result.is_some());
    assert_eq!(result.unwrap().subtype, "json");
}

#[test]
fn test_cache_get_or_compute() {
    let mut cache: NegotiationCache<MediaType> = NegotiationCache::new();
    let key = CacheKey::new("application/json");

    // First call should compute
    let result = cache.get_or_compute(&key, || MediaType::new("application", "json"));
    assert_eq!(result.subtype, "json");
    assert_eq!(cache.len(), 1);

    // Second call should use cached value
    let mut compute_called = false;
    let result2 = cache.get_or_compute(&key, || {
        compute_called = true;
        MediaType::new("text", "html")
    });

    assert!(!compute_called);
    assert_eq!(result2.subtype, "json"); // Should still be json
}

#[test]
fn test_cache_expiration() {
    let mut cache: NegotiationCache<MediaType> =
        NegotiationCache::with_ttl(Duration::from_millis(50));
    let key = CacheKey::new("application/json");
    let media_type = MediaType::new("application", "json");

    cache.set(key.clone(), media_type);

    // Should exist immediately
    assert!(cache.get(&key).is_some());
    assert_eq!(cache.len(), 1);

    // Wait for expiration
    thread::sleep(Duration::from_millis(100));

    // Should be expired and removed
    assert!(cache.get(&key).is_none());
    assert_eq!(cache.len(), 0);
}

#[test]
fn test_cache_clear() {
    let mut cache: NegotiationCache<MediaType> = NegotiationCache::new();
    let key1 = CacheKey::new("application/json");
    let key2 = CacheKey::new("text/html");

    cache.set(key1, MediaType::new("application", "json"));
    cache.set(key2, MediaType::new("text", "html"));

    assert_eq!(cache.len(), 2);

    cache.clear();
    assert_eq!(cache.len(), 0);
    assert!(cache.is_empty());
}

#[test]
fn test_cache_clear_expired() {
    let mut cache: NegotiationCache<MediaType> =
        NegotiationCache::with_ttl(Duration::from_millis(50));

    let key1 = CacheKey::new("key1");
    let key2 = CacheKey::new("key2");

    cache.set(key1.clone(), MediaType::new("application", "json"));
    thread::sleep(Duration::from_millis(30));
    cache.set(key2.clone(), MediaType::new("text", "html"));

    assert_eq!(cache.len(), 2);

    // Wait for first entry to expire
    thread::sleep(Duration::from_millis(30));
    cache.clear_expired();

    // First entry should be expired
    assert!(cache.get(&key1).is_none());
    // Second entry should still exist
    assert!(cache.get(&key2).is_some());
}

#[test]
fn test_cache_max_entries() {
    let mut cache: NegotiationCache<MediaType> =
        NegotiationCache::with_config(Duration::from_secs(300), 3);

    cache.set(CacheKey::new("key1"), MediaType::new("application", "json"));
    cache.set(CacheKey::new("key2"), MediaType::new("text", "html"));
    cache.set(CacheKey::new("key3"), MediaType::new("application", "xml"));

    assert_eq!(cache.len(), 3);

    // Adding 4th entry should evict oldest
    cache.set(CacheKey::new("key4"), MediaType::new("text", "plain"));

    // Should still be at max capacity
    assert_eq!(cache.len(), 3);
}

#[test]
fn test_cache_with_custom_config() {
    let cache: NegotiationCache<MediaType> =
        NegotiationCache::with_config(Duration::from_secs(600), 500);

    assert_eq!(cache.len(), 0);
    assert!(cache.is_empty());
}

#[test]
fn test_cache_multiple_types() {
    let mut cache: NegotiationCache<MediaType> = NegotiationCache::new();

    let json_key = CacheKey::new("application/json");
    let html_key = CacheKey::new("text/html");
    let xml_key = CacheKey::new("application/xml");

    cache.set(json_key.clone(), MediaType::new("application", "json"));
    cache.set(html_key.clone(), MediaType::new("text", "html"));
    cache.set(xml_key.clone(), MediaType::new("application", "xml"));

    assert_eq!(cache.len(), 3);

    assert_eq!(cache.get(&json_key).unwrap().subtype, "json");
    assert_eq!(cache.get(&html_key).unwrap().subtype, "html");
    assert_eq!(cache.get(&xml_key).unwrap().subtype, "xml");
}

#[test]
fn test_cache_update_existing() {
    let mut cache: NegotiationCache<MediaType> = NegotiationCache::new();
    let key = CacheKey::new("application/json");

    cache.set(key.clone(), MediaType::new("application", "json"));
    assert_eq!(cache.len(), 1);

    // Update with different value
    cache.set(key.clone(), MediaType::new("text", "html"));
    assert_eq!(cache.len(), 1); // Same entry updated

    let result = cache.get(&key);
    assert_eq!(result.unwrap().subtype, "html"); // Value updated
}
