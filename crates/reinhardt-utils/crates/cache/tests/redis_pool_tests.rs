//! Redis connection pool integration tests
//!
//! These tests verify the connection pooling functionality of RedisCache.

#[cfg(feature = "redis-backend")]
mod redis_pool_integration {
    use deadpool_redis::Config as PoolConfig;
    use reinhardt_cache::{redis_backend::RedisCache, Cache};
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::time::Duration;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        id: i32,
        name: String,
    }

    // Helper function to get Redis URL from environment or use default
    fn get_redis_url() -> String {
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
    }

    // Helper function to create a test cache instance
    async fn create_test_cache() -> RedisCache {
        let url = get_redis_url();
        RedisCache::new(url)
            .await
            .expect("Failed to create Redis cache")
    }

    #[tokio::test]
    async fn test_pool_creation_from_url() {
        let cache = create_test_cache().await;
        assert!(cache.pool().status().size > 0);
    }

    #[tokio::test]
    async fn test_pool_creation_from_config() {
        let url = get_redis_url();
        let mut config = PoolConfig::from_url(url);
        config.pool = Some(deadpool_redis::PoolConfig::new(10));

        let cache =
            RedisCache::with_pool_config(config).expect("Failed to create cache from config");

        assert!(cache.pool().status().size > 0);
    }

    #[tokio::test]
    async fn test_pool_get_connection() {
        let cache = create_test_cache().await;
        let conn = cache.pool().get().await;
        assert!(conn.is_ok(), "Should be able to get connection from pool");
    }

    #[tokio::test]
    async fn test_concurrent_pool_access() {
        let cache = create_test_cache().await;
        let cache_clone1 = cache.clone();
        let cache_clone2 = cache.clone();
        let cache_clone3 = cache.clone();

        let task1 =
            tokio::spawn(
                async move { cache_clone1.set("concurrent_key_1", &"value1", None).await },
            );

        let task2 =
            tokio::spawn(
                async move { cache_clone2.set("concurrent_key_2", &"value2", None).await },
            );

        let task3 =
            tokio::spawn(
                async move { cache_clone3.set("concurrent_key_3", &"value3", None).await },
            );

        let results = tokio::join!(task1, task2, task3);
        assert!(results.0.is_ok());
        assert!(results.1.is_ok());
        assert!(results.2.is_ok());

        // Cleanup
        let _ = cache.delete("concurrent_key_1").await;
        let _ = cache.delete("concurrent_key_2").await;
        let _ = cache.delete("concurrent_key_3").await;
    }

    #[tokio::test]
    async fn test_pool_with_key_prefix() {
        let url = get_redis_url();
        let cache = RedisCache::new(url)
            .await
            .expect("Failed to create cache")
            .with_key_prefix("test_prefix");

        cache
            .set("key1", &"value1", None)
            .await
            .expect("Failed to set value");

        let value: Option<String> = cache.get("key1").await.expect("Failed to get value");
        assert_eq!(value, Some("value1".to_string()));

        // Cleanup
        cache.delete("key1").await.expect("Failed to delete key");
    }

    #[tokio::test]
    async fn test_pool_with_default_ttl() {
        let url = get_redis_url();
        let cache = RedisCache::new(url)
            .await
            .expect("Failed to create cache")
            .with_default_ttl(Duration::from_secs(60));

        cache
            .set("ttl_key", &"ttl_value", None)
            .await
            .expect("Failed to set value");

        let value: Option<String> = cache.get("ttl_key").await.expect("Failed to get value");
        assert_eq!(value, Some("ttl_value".to_string()));

        // Cleanup
        cache.delete("ttl_key").await.expect("Failed to delete key");
    }

    #[tokio::test]
    async fn test_pool_set_and_get() {
        let cache = create_test_cache().await;
        let test_data = TestData {
            id: 1,
            name: "Test".to_string(),
        };

        cache
            .set("pool_test_data", &test_data, None)
            .await
            .expect("Failed to set value");

        let retrieved: Option<TestData> = cache
            .get("pool_test_data")
            .await
            .expect("Failed to get value");

        assert_eq!(retrieved, Some(test_data));

        // Cleanup
        cache
            .delete("pool_test_data")
            .await
            .expect("Failed to delete key");
    }

    #[tokio::test]
    async fn test_pool_set_with_ttl() {
        let cache = create_test_cache().await;

        cache
            .set("ttl_test", &"expires_soon", Some(Duration::from_secs(2)))
            .await
            .expect("Failed to set value with TTL");

        let value: Option<String> = cache.get("ttl_test").await.expect("Failed to get value");
        assert_eq!(value, Some("expires_soon".to_string()));

        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(3)).await;

        let expired: Option<String> = cache
            .get("ttl_test")
            .await
            .expect("Failed to get expired value");
        assert_eq!(expired, None);
    }

    #[tokio::test]
    async fn test_pool_delete() {
        let cache = create_test_cache().await;

        cache
            .set("delete_test", &"to_be_deleted", None)
            .await
            .expect("Failed to set value");

        cache
            .delete("delete_test")
            .await
            .expect("Failed to delete key");

        let value: Option<String> = cache
            .get("delete_test")
            .await
            .expect("Failed to get deleted value");
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_pool_has_key() {
        let cache = create_test_cache().await;

        cache
            .set("exists_test", &"exists", None)
            .await
            .expect("Failed to set value");

        let exists = cache
            .has_key("exists_test")
            .await
            .expect("Failed to check key existence");
        assert!(exists);

        let not_exists = cache
            .has_key("nonexistent_key")
            .await
            .expect("Failed to check key existence");
        assert!(!not_exists);

        // Cleanup
        cache
            .delete("exists_test")
            .await
            .expect("Failed to delete key");
    }

    #[tokio::test]
    async fn test_pool_get_many() {
        let cache = create_test_cache().await;

        cache
            .set("many_1", &"value1", None)
            .await
            .expect("Failed to set value");
        cache
            .set("many_2", &"value2", None)
            .await
            .expect("Failed to set value");
        cache
            .set("many_3", &"value3", None)
            .await
            .expect("Failed to set value");

        let keys = vec!["many_1", "many_2", "many_3", "nonexistent"];
        let values: HashMap<String, Option<String>> = cache
            .get_many(&keys)
            .await
            .expect("Failed to get many values");

        assert_eq!(values.len(), 4);
        assert_eq!(values.get("many_1"), Some(&Some("value1".to_string())));
        assert_eq!(values.get("many_2"), Some(&Some("value2".to_string())));
        assert_eq!(values.get("many_3"), Some(&Some("value3".to_string())));
        assert_eq!(values.get("nonexistent"), Some(&None));

        // Cleanup
        cache.delete("many_1").await.expect("Failed to delete key");
        cache.delete("many_2").await.expect("Failed to delete key");
        cache.delete("many_3").await.expect("Failed to delete key");
    }

    #[tokio::test]
    async fn test_pool_set_many() {
        let cache = create_test_cache().await;

        let mut items = HashMap::new();
        items.insert("set_many_1".to_string(), "value1");
        items.insert("set_many_2".to_string(), "value2");
        items.insert("set_many_3".to_string(), "value3");

        cache
            .set_many(items, None)
            .await
            .expect("Failed to set many values");

        let value1: Option<String> = cache.get("set_many_1").await.expect("Failed to get value");
        let value2: Option<String> = cache.get("set_many_2").await.expect("Failed to get value");
        let value3: Option<String> = cache.get("set_many_3").await.expect("Failed to get value");

        assert_eq!(value1, Some("value1".to_string()));
        assert_eq!(value2, Some("value2".to_string()));
        assert_eq!(value3, Some("value3".to_string()));

        // Cleanup
        cache
            .delete("set_many_1")
            .await
            .expect("Failed to delete key");
        cache
            .delete("set_many_2")
            .await
            .expect("Failed to delete key");
        cache
            .delete("set_many_3")
            .await
            .expect("Failed to delete key");
    }

    #[tokio::test]
    async fn test_pool_delete_many() {
        let cache = create_test_cache().await;

        cache
            .set("delete_many_1", &"value1", None)
            .await
            .expect("Failed to set value");
        cache
            .set("delete_many_2", &"value2", None)
            .await
            .expect("Failed to set value");

        let keys = vec!["delete_many_1", "delete_many_2"];
        cache
            .delete_many(&keys)
            .await
            .expect("Failed to delete many keys");

        let value1: Option<String> = cache
            .get("delete_many_1")
            .await
            .expect("Failed to get value");
        let value2: Option<String> = cache
            .get("delete_many_2")
            .await
            .expect("Failed to get value");

        assert_eq!(value1, None);
        assert_eq!(value2, None);
    }

    #[tokio::test]
    async fn test_pool_incr() {
        let cache = create_test_cache().await;

        let new_value = cache
            .incr("counter_incr", 1)
            .await
            .expect("Failed to increment");
        assert_eq!(new_value, 1);

        let new_value = cache
            .incr("counter_incr", 5)
            .await
            .expect("Failed to increment");
        assert_eq!(new_value, 6);

        // Cleanup
        cache
            .delete("counter_incr")
            .await
            .expect("Failed to delete key");
    }

    #[tokio::test]
    async fn test_pool_decr() {
        let cache = create_test_cache().await;

        // Set initial value
        cache
            .set("counter_decr", &10i64, None)
            .await
            .expect("Failed to set initial value");

        let new_value = cache
            .decr("counter_decr", 1)
            .await
            .expect("Failed to decrement");
        assert_eq!(new_value, 9);

        let new_value = cache
            .decr("counter_decr", 5)
            .await
            .expect("Failed to decrement");
        assert_eq!(new_value, 4);

        // Cleanup
        cache
            .delete("counter_decr")
            .await
            .expect("Failed to delete key");
    }

    #[tokio::test]
    async fn test_pool_clear() {
        let cache = create_test_cache().await.with_key_prefix("clear_test");

        cache
            .set("key1", &"value1", None)
            .await
            .expect("Failed to set value");
        cache
            .set("key2", &"value2", None)
            .await
            .expect("Failed to set value");

        cache.clear().await.expect("Failed to clear cache");

        let value1: Option<String> = cache.get("key1").await.expect("Failed to get value");
        let value2: Option<String> = cache.get("key2").await.expect("Failed to get value");

        assert_eq!(value1, None);
        assert_eq!(value2, None);
    }

    #[tokio::test]
    async fn test_pool_connection_reuse() {
        let cache = create_test_cache().await;

        // Perform multiple operations to ensure connections are reused
        for i in 0..20 {
            let key = format!("reuse_test_{}", i);
            cache
                .set(&key, &i, None)
                .await
                .expect("Failed to set value");

            let value: Option<i32> = cache.get(&key).await.expect("Failed to get value");
            assert_eq!(value, Some(i));

            cache.delete(&key).await.expect("Failed to delete key");
        }
    }

    #[tokio::test]
    async fn test_pool_status() {
        let cache = create_test_cache().await;
        let status = cache.pool().status();

        assert!(status.size > 0, "Pool should have connections");
        assert!(
            status.available > 0,
            "Pool should have available connections"
        );
    }

    #[tokio::test]
    async fn test_pool_multiple_concurrent_operations() {
        let cache = create_test_cache().await;
        let mut handles = vec![];

        for i in 0..50 {
            let cache_clone = cache.clone();
            let handle = tokio::spawn(async move {
                let key = format!("concurrent_{}", i);
                cache_clone
                    .set(&key, &i, None)
                    .await
                    .expect("Failed to set value");

                let value: Option<i32> = cache_clone.get(&key).await.expect("Failed to get value");
                assert_eq!(value, Some(i));

                cache_clone
                    .delete(&key)
                    .await
                    .expect("Failed to delete key");
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.expect("Task panicked");
        }
    }
}
