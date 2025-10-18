//! Integration tests for DatabaseSessionBackend
//!
//! These tests verify that the database session backend correctly integrates
//! with real database systems using TestContainers.

use reinhardt_sessions::backends::{DatabaseSessionBackend, SessionBackend};
use serde_json::json;
use testcontainers::{core::WaitFor, runners::AsyncRunner, GenericImage, ImageExt};

/// Helper function to create a PostgreSQL testcontainer and return the connection URL
async fn setup_postgres_container() -> (String, testcontainers::ContainerAsync<GenericImage>) {
    let postgres = GenericImage::new("postgres", "17-alpine")
        .with_exposed_port(5432.into())
        .with_env_var("POSTGRES_HOST_AUTH_METHOD", "trust")
        .with_env_var("POSTGRES_DB", "test_sessions")
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .start()
        .await
        .expect("Failed to start PostgreSQL container");

    let host_port = postgres
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get host port");

    let database_url = format!("postgres://postgres@127.0.0.1:{}/test_sessions", host_port);

    // Wait a bit for PostgreSQL to fully initialize
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    (database_url, postgres)
}

#[tokio::test]
async fn test_database_session_save_and_load() {
    let (database_url, _container) = setup_postgres_container().await;

    let backend = DatabaseSessionBackend::new(&database_url)
        .await
        .expect("Failed to create backend");

    backend
        .create_table()
        .await
        .expect("Failed to create table");

    let session_key = "test_session_save_load";
    let data = json!({
        "user_id": 42,
        "username": "testuser",
        "authenticated": true,
    });

    // Save session
    backend
        .save(session_key, &data, Some(3600))
        .await
        .expect("Failed to save session");

    // Load session
    let loaded: Option<serde_json::Value> = backend
        .load(session_key)
        .await
        .expect("Failed to load session");

    assert_eq!(loaded, Some(data));
}

#[tokio::test]
async fn test_database_session_exists() {
    let (database_url, _container) = setup_postgres_container().await;

    let backend = DatabaseSessionBackend::new(&database_url)
        .await
        .expect("Failed to create backend");

    backend
        .create_table()
        .await
        .expect("Failed to create table");

    let session_key = "test_session_exists";
    let data = json!({"test": "data"});

    // Session should not exist initially
    let exists = backend
        .exists(session_key)
        .await
        .expect("Failed to check existence");
    assert!(!exists);

    // Save session
    backend
        .save(session_key, &data, Some(3600))
        .await
        .expect("Failed to save session");

    // Session should now exist
    let exists = backend
        .exists(session_key)
        .await
        .expect("Failed to check existence");
    assert!(exists);
}

#[tokio::test]
async fn test_database_session_delete() {
    let (database_url, _container) = setup_postgres_container().await;

    let backend = DatabaseSessionBackend::new(&database_url)
        .await
        .expect("Failed to create backend");

    backend
        .create_table()
        .await
        .expect("Failed to create table");

    let session_key = "test_session_delete";
    let data = json!({"test": "data"});

    // Save session
    backend
        .save(session_key, &data, Some(3600))
        .await
        .expect("Failed to save session");

    // Verify session exists
    assert!(backend
        .exists(session_key)
        .await
        .expect("Failed to check existence"));

    // Delete session
    backend
        .delete(session_key)
        .await
        .expect("Failed to delete session");

    // Verify session no longer exists
    assert!(!backend
        .exists(session_key)
        .await
        .expect("Failed to check existence"));
}

#[tokio::test]
async fn test_database_session_expiration() {
    let (database_url, _container) = setup_postgres_container().await;

    let backend = DatabaseSessionBackend::new(&database_url)
        .await
        .expect("Failed to create backend");

    backend
        .create_table()
        .await
        .expect("Failed to create table");

    let session_key = "test_session_expiration";
    let data = json!({"test": "data"});

    // Save session with very short TTL
    backend
        .save(session_key, &data, Some(1))
        .await
        .expect("Failed to save session");

    // Session should exist immediately
    let exists = backend
        .exists(session_key)
        .await
        .expect("Failed to check existence");
    assert!(exists);

    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Session should no longer exist
    let exists = backend
        .exists(session_key)
        .await
        .expect("Failed to check existence");
    assert!(!exists);

    // Loading expired session should return None
    let loaded: Option<serde_json::Value> = backend
        .load(session_key)
        .await
        .expect("Failed to load session");
    assert_eq!(loaded, None);
}

#[tokio::test]
async fn test_database_session_cleanup_expired() {
    let (database_url, _container) = setup_postgres_container().await;

    let backend = DatabaseSessionBackend::new(&database_url)
        .await
        .expect("Failed to create backend");

    backend
        .create_table()
        .await
        .expect("Failed to create table");

    // Create some expired sessions
    for i in 0..5 {
        let key = format!("expired_{}", i);
        backend
            .save(&key, &json!({ "test": i }), Some(0))
            .await
            .expect("Failed to save session");
    }

    // Create some active sessions
    for i in 0..3 {
        let key = format!("active_{}", i);
        backend
            .save(&key, &json!({ "test": i }), Some(3600))
            .await
            .expect("Failed to save session");
    }

    // Wait for expired sessions to expire
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Clean up expired sessions
    let deleted = backend.cleanup_expired().await.expect("Failed to cleanup");

    assert_eq!(deleted, 5);

    // Verify active sessions still exist
    for i in 0..3 {
        let key = format!("active_{}", i);
        assert!(backend
            .exists(&key)
            .await
            .expect("Failed to check existence"));
    }

    // Verify expired sessions are gone
    for i in 0..5 {
        let key = format!("expired_{}", i);
        assert!(!backend
            .exists(&key)
            .await
            .expect("Failed to check existence"));
    }
}

#[tokio::test]
async fn test_database_session_update_existing() {
    let (database_url, _container) = setup_postgres_container().await;

    let backend = DatabaseSessionBackend::new(&database_url)
        .await
        .expect("Failed to create backend");

    backend
        .create_table()
        .await
        .expect("Failed to create table");

    let session_key = "test_session_update";
    let initial_data = json!({
        "user_id": 42,
        "username": "initial",
    });

    // Save initial session
    backend
        .save(session_key, &initial_data, Some(3600))
        .await
        .expect("Failed to save session");

    // Update session with new data
    let updated_data = json!({
        "user_id": 42,
        "username": "updated",
        "additional_field": "new value",
    });

    backend
        .save(session_key, &updated_data, Some(3600))
        .await
        .expect("Failed to update session");

    // Load session and verify it has updated data
    let loaded: Option<serde_json::Value> = backend
        .load(session_key)
        .await
        .expect("Failed to load session");

    assert_eq!(loaded, Some(updated_data));
}

#[tokio::test]
async fn test_database_session_multiple_sessions() {
    let (database_url, _container) = setup_postgres_container().await;

    let backend = DatabaseSessionBackend::new(&database_url)
        .await
        .expect("Failed to create backend");

    backend
        .create_table()
        .await
        .expect("Failed to create table");

    // Create multiple sessions
    let sessions = vec![
        ("session_1", json!({"user": "alice"})),
        ("session_2", json!({"user": "bob"})),
        ("session_3", json!({"user": "charlie"})),
    ];

    for (key, data) in &sessions {
        backend
            .save(key, data, Some(3600))
            .await
            .expect("Failed to save session");
    }

    // Verify all sessions exist and have correct data
    for (key, expected_data) in &sessions {
        let loaded: Option<serde_json::Value> =
            backend.load(key).await.expect("Failed to load session");

        assert_eq!(loaded, Some(expected_data.clone()));
    }
}

#[tokio::test]
async fn test_database_session_complex_data() {
    let (database_url, _container) = setup_postgres_container().await;

    let backend = DatabaseSessionBackend::new(&database_url)
        .await
        .expect("Failed to create backend");

    backend
        .create_table()
        .await
        .expect("Failed to create table");

    let session_key = "test_complex_data";
    let complex_data = json!({
        "user": {
            "id": 42,
            "name": "Test User",
            "roles": ["admin", "user"],
            "metadata": {
                "last_login": "2025-10-18T10:00:00Z",
                "preferences": {
                    "theme": "dark",
                    "language": "en",
                }
            }
        },
        "cart": {
            "items": [
                {"id": 1, "quantity": 2},
                {"id": 2, "quantity": 1}
            ],
            "total": 99.99
        }
    });

    // Save complex data
    backend
        .save(session_key, &complex_data, Some(3600))
        .await
        .expect("Failed to save session");

    // Load and verify complex data
    let loaded: Option<serde_json::Value> = backend
        .load(session_key)
        .await
        .expect("Failed to load session");

    assert_eq!(loaded, Some(complex_data));
}
