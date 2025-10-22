//! Integration tests for RedisBackend
//!
//! These tests use TestContainers to run Redis and test the RedisBackend.

use reinhardt_settings::backends::RedisBackend;
use testcontainers::clients::Cli;
use testcontainers::images::redis::Redis;
use testcontainers::Container;

/// Helper function to create a test backend with Redis
async fn create_test_backend(container: &Container<'_, Redis>) -> RedisBackend {
    let port = container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://localhost:{}", port);

    RedisBackend::new(&redis_url)
        .await
        .expect("Failed to create Redis backend")
}

#[tokio::test]
async fn test_redis_backend_set_and_get() {
    let docker = Cli::default();
    let container = docker.run(Redis::default());
    let backend = create_test_backend(&container).await;

    let key = "test_key_1";
    let value = r#"{"debug": true, "port": 8080}"#;

    // Set value
    backend
        .set(key, value, Some(3600))
        .await
        .expect("Failed to set value");

    // Get value
    let retrieved = backend.get(key).await.expect("Failed to get value");

    assert_eq!(retrieved, Some(value.to_string()));
}

#[tokio::test]
async fn test_redis_backend_exists() {
    let docker = Cli::default();
    let container = docker.run(Redis::default());
    let backend = create_test_backend(&container).await;

    let key = "test_key_2";
    let value = "test_value";

    // Initially should not exist
    assert!(!backend
        .exists(key)
        .await
        .expect("Failed to check existence"));

    // Set value
    backend
        .set(key, value, None)
        .await
        .expect("Failed to set value");

    // Now should exist
    assert!(backend
        .exists(key)
        .await
        .expect("Failed to check existence"));
}

#[tokio::test]
async fn test_redis_backend_delete() {
    let docker = Cli::default();
    let container = docker.run(Redis::default());
    let backend = create_test_backend(&container).await;

    let key = "test_key_3";
    let value = "test_value";

    // Set value
    backend
        .set(key, value, None)
        .await
        .expect("Failed to set value");

    // Verify exists
    assert!(backend
        .exists(key)
        .await
        .expect("Failed to check existence"));

    // Delete
    backend.delete(key).await.expect("Failed to delete value");

    // Verify no longer exists
    assert!(!backend
        .exists(key)
        .await
        .expect("Failed to check existence"));
}

#[tokio::test]
async fn test_redis_backend_ttl_expiration() {
    let docker = Cli::default();
    let container = docker.run(Redis::default());
    let backend = create_test_backend(&container).await;

    let key = "test_key_4";
    let value = "test_value";

    // Set with 1 second TTL
    backend
        .set(key, value, Some(1))
        .await
        .expect("Failed to set value");

    // Verify exists
    assert!(backend
        .exists(key)
        .await
        .expect("Failed to check existence"));

    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Should not exist anymore
    assert!(!backend
        .exists(key)
        .await
        .expect("Failed to check existence"));

    // Get should return None
    let retrieved = backend.get(key).await.expect("Failed to get value");
    assert_eq!(retrieved, None);
}

#[tokio::test]
async fn test_redis_backend_without_ttl() {
    let docker = Cli::default();
    let container = docker.run(Redis::default());
    let backend = create_test_backend(&container).await;

    let key = "permanent_key";
    let value = "permanent_value";

    // Set without TTL
    backend
        .set(key, value, None)
        .await
        .expect("Failed to set value");

    // Get value
    let retrieved = backend.get(key).await.expect("Failed to get value");
    assert_eq!(retrieved, Some(value.to_string()));

    // Should exist
    assert!(backend
        .exists(key)
        .await
        .expect("Failed to check existence"));
}

#[tokio::test]
async fn test_redis_backend_overwrite() {
    let docker = Cli::default();
    let container = docker.run(Redis::default());
    let backend = create_test_backend(&container).await;

    let key = "overwrite_key";

    // Set initial value
    backend
        .set(key, "value1", None)
        .await
        .expect("Failed to set initial value");

    // Overwrite
    backend
        .set(key, "value2", None)
        .await
        .expect("Failed to overwrite value");

    // Get updated value
    let retrieved = backend.get(key).await.expect("Failed to get value");
    assert_eq!(retrieved, Some("value2".to_string()));
}

#[tokio::test]
async fn test_redis_backend_keys() {
    let docker = Cli::default();
    let container = docker.run(Redis::default());
    let backend = create_test_backend(&container).await;

    // Set multiple keys
    backend
        .set("key1", "value1", None)
        .await
        .expect("Failed to set key1");
    backend
        .set("key2", "value2", None)
        .await
        .expect("Failed to set key2");
    backend
        .set("key3", "value3", None)
        .await
        .expect("Failed to set key3");

    // Get all keys
    let keys = backend.keys().await.expect("Failed to get keys");

    assert!(keys.contains(&"key1".to_string()));
    assert!(keys.contains(&"key2".to_string()));
    assert!(keys.contains(&"key3".to_string()));
}

#[tokio::test]
async fn test_redis_backend_json_values() {
    let docker = Cli::default();
    let container = docker.run(Redis::default());
    let backend = create_test_backend(&container).await;

    let key = "json_value";
    let value = r#"{"nested": {"array": [1, 2, 3], "object": {"key": "value"}}}"#;

    // Set JSON value
    backend
        .set(key, value, None)
        .await
        .expect("Failed to set JSON value");

    // Get JSON value
    let retrieved = backend.get(key).await.expect("Failed to get value");
    assert_eq!(retrieved, Some(value.to_string()));
}

#[tokio::test]
async fn test_redis_backend_multiple_keys() {
    let docker = Cli::default();
    let container = docker.run(Redis::default());
    let backend = create_test_backend(&container).await;

    // Set multiple keys
    for i in 0..10 {
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        backend
            .set(&key, &value, None)
            .await
            .expect("Failed to set value");
    }

    // Verify all keys exist
    for i in 0..10 {
        let key = format!("key_{}", i);
        assert!(backend
            .exists(&key)
            .await
            .expect("Failed to check existence"));

        let value = backend.get(&key).await.expect("Failed to get value");
        assert_eq!(value, Some(format!("value_{}", i)));
    }
}

#[tokio::test]
async fn test_redis_backend_empty_value() {
    let docker = Cli::default();
    let container = docker.run(Redis::default());
    let backend = create_test_backend(&container).await;

    let key = "empty_key";
    let value = "";

    // Set empty value
    backend
        .set(key, value, None)
        .await
        .expect("Failed to set empty value");

    // Get empty value
    let retrieved = backend.get(key).await.expect("Failed to get value");
    assert_eq!(retrieved, Some(String::new()));
}

#[tokio::test]
async fn test_redis_backend_special_characters() {
    let docker = Cli::default();
    let container = docker.run(Redis::default());
    let backend = create_test_backend(&container).await;

    let key = "special_key";
    let value = r#"{"special": "value with \n newlines and \"quotes\""}"#;

    // Set value with special characters
    backend
        .set(key, value, None)
        .await
        .expect("Failed to set value");

    // Get value with special characters
    let retrieved = backend.get(key).await.expect("Failed to get value");
    assert_eq!(retrieved, Some(value.to_string()));
}
