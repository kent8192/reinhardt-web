//! Integration tests for DatabaseBackend
//!
//! These tests use TestContainers to run PostgreSQL and test the DatabaseBackend.

use reinhardt_settings::backends::DatabaseBackend;
use serde_json::json;
use testcontainers::clients::Cli;
use testcontainers::images::postgres::Postgres;
use testcontainers::Container;

/// Helper function to create a test backend with PostgreSQL
async fn create_test_backend(container: &Container<'_, Postgres>) -> DatabaseBackend {
    let port = container.get_host_port_ipv4(5432);
    let connection_url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);

    let backend = DatabaseBackend::new(&connection_url)
        .await
        .expect("Failed to create database backend");

    backend
        .create_table()
        .await
        .expect("Failed to create table");

    backend
}

#[tokio::test]
async fn test_database_backend_set_and_get() {
    let docker = Cli::default();
    let container = docker.run(Postgres::default());
    let backend = create_test_backend(&container).await;

    let key = "test_key_1";
    let value = json!({
        "debug": true,
        "port": 8080,
        "features": ["feature1", "feature2"],
    });

    // Set value
    backend
        .set(key, &value, Some(3600))
        .await
        .expect("Failed to set value");

    // Get value
    let retrieved = backend.get(key).await.expect("Failed to get value");

    assert_eq!(retrieved, Some(value));
}

#[tokio::test]
async fn test_database_backend_exists() {
    let docker = Cli::default();
    let container = docker.run(Postgres::default());
    let backend = create_test_backend(&container).await;

    let key = "test_key_2";
    let value = json!({"test": "data"});

    // Initially should not exist
    assert!(!backend
        .exists(key)
        .await
        .expect("Failed to check existence"));

    // Set value
    backend
        .set(key, &value, None)
        .await
        .expect("Failed to set value");

    // Now should exist
    assert!(backend
        .exists(key)
        .await
        .expect("Failed to check existence"));
}

#[tokio::test]
async fn test_database_backend_delete() {
    let docker = Cli::default();
    let container = docker.run(Postgres::default());
    let backend = create_test_backend(&container).await;

    let key = "test_key_3";
    let value = json!({"test": "data"});

    // Set value
    backend
        .set(key, &value, None)
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
async fn test_database_backend_expired() {
    let docker = Cli::default();
    let container = docker.run(Postgres::default());
    let backend = create_test_backend(&container).await;

    let key = "test_key_4";
    let value = json!({"test": "data"});

    // Set with 0 second TTL (immediately expired)
    backend
        .set(key, &value, Some(0))
        .await
        .expect("Failed to set value");

    // Wait a moment
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Get should return None
    let retrieved = backend.get(key).await.expect("Failed to get value");
    assert_eq!(retrieved, None);

    // Exists should return false
    assert!(!backend
        .exists(key)
        .await
        .expect("Failed to check existence"));
}

#[tokio::test]
async fn test_database_backend_cleanup_expired() {
    let docker = Cli::default();
    let container = docker.run(Postgres::default());
    let backend = create_test_backend(&container).await;

    // Create expired settings
    for i in 0..5 {
        let key = format!("expired_{}", i);
        backend
            .set(&key, &json!({ "index": i }), Some(0))
            .await
            .expect("Failed to set expired value");
    }

    // Create active settings
    for i in 0..3 {
        let key = format!("active_{}", i);
        backend
            .set(&key, &json!({ "index": i }), Some(3600))
            .await
            .expect("Failed to set active value");
    }

    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Cleanup
    let deleted = backend.cleanup_expired().await.expect("Failed to cleanup");

    assert_eq!(deleted, 5);

    // Verify active settings still exist
    for i in 0..3 {
        let key = format!("active_{}", i);
        assert!(backend
            .exists(&key)
            .await
            .expect("Failed to check existence"));
    }
}

#[tokio::test]
async fn test_database_backend_without_ttl() {
    let docker = Cli::default();
    let container = docker.run(Postgres::default());
    let backend = create_test_backend(&container).await;

    let key = "permanent_key";
    let value = json!({"permanent": true});

    // Set without TTL
    backend
        .set(key, &value, None)
        .await
        .expect("Failed to set value");

    // Get value
    let retrieved = backend.get(key).await.expect("Failed to get value");
    assert_eq!(retrieved, Some(value));

    // Should exist
    assert!(backend
        .exists(key)
        .await
        .expect("Failed to check existence"));
}

#[tokio::test]
async fn test_database_backend_overwrite() {
    let docker = Cli::default();
    let container = docker.run(Postgres::default());
    let backend = create_test_backend(&container).await;

    let key = "overwrite_key";

    // Set initial value
    backend
        .set(key, &json!({"version": 1}), None)
        .await
        .expect("Failed to set initial value");

    // Overwrite
    backend
        .set(key, &json!({"version": 2}), None)
        .await
        .expect("Failed to overwrite value");

    // Get updated value
    let retrieved = backend.get(key).await.expect("Failed to get value");
    assert_eq!(retrieved, Some(json!({"version": 2})));
}

#[tokio::test]
async fn test_database_backend_complex_values() {
    let docker = Cli::default();
    let container = docker.run(Postgres::default());
    let backend = create_test_backend(&container).await;

    let key = "complex_value";
    let value = json!({
        "nested": {
            "array": [1, 2, 3],
            "object": {"key": "value"},
            "boolean": true,
            "null": null,
        },
        "string": "test",
        "number": 42,
    });

    // Set complex value
    backend
        .set(key, &value, None)
        .await
        .expect("Failed to set complex value");

    // Get complex value
    let retrieved = backend.get(key).await.expect("Failed to get value");
    assert_eq!(retrieved, Some(value));
}

#[tokio::test]
async fn test_database_backend_multiple_keys() {
    let docker = Cli::default();
    let container = docker.run(Postgres::default());
    let backend = create_test_backend(&container).await;

    // Set multiple keys
    for i in 0..10 {
        let key = format!("key_{}", i);
        backend
            .set(&key, &json!({ "index": i }), None)
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
        assert_eq!(value, Some(json!({ "index": i })));
    }
}
