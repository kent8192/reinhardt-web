use reinhardt_sessions::backends::database::DatabaseSessionBackend;
use reinhardt_sessions::backends::cache::SessionBackend;
use serde_json::json;

/// Creates a test database session backend
async fn create_test_backend() -> DatabaseSessionBackend {
    let backend = DatabaseSessionBackend::new("sqlite::memory:")
        .await
        .expect("Failed to create test backend");
    backend
        .create_table()
        .await
        .expect("Failed to create table");
    backend
}

#[tokio::test]
async fn test_save_and_load_session() {
    let backend = create_test_backend().await;
    let session_key = "test_session_1";
    let data = json!({
        "user_id": 123,
        "username": "testuser",
    });

    // Save session with 3600 second TTL
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
async fn test_session_exists() {
    let backend = create_test_backend().await;
    let session_key = "test_session_2";
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
async fn test_delete_session() {
    let backend = create_test_backend().await;
    let session_key = "test_session_3";
    let data = json!({"test": "data"});

    // Save session
    backend
        .save(session_key, &data, Some(3600))
        .await
        .expect("Failed to save session");

    // Verify session exists
    assert!(
        backend
            .exists(session_key)
            .await
            .expect("Failed to check existence")
    );

    // Delete session
    backend
        .delete(session_key)
        .await
        .expect("Failed to delete session");

    // Verify session no longer exists
    assert!(
        !backend
            .exists(session_key)
            .await
            .expect("Failed to check existence")
    );
}

#[tokio::test]
async fn test_expired_session() {
    let backend = create_test_backend().await;
    let session_key = "test_session_4";
    let data = json!({"test": "data"});

    // Save session with 0 second TTL (immediately expired)
    backend
        .save(session_key, &data, Some(0))
        .await
        .expect("Failed to save session");

    // Wait a moment to ensure expiration
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Try to load expired session
    let loaded: Option<serde_json::Value> = backend
        .load(session_key)
        .await
        .expect("Failed to load session");

    assert_eq!(loaded, None);
}

#[tokio::test]
async fn test_cleanup_expired() {
    let backend = create_test_backend().await;

    // Save some expired sessions
    for i in 0..5 {
        let key = format!("expired_{}", i);
        backend
            .save(&key, &json!({ "test": i }), Some(0))
            .await
            .expect("Failed to save session");
    }

    // Save some active sessions
    for i in 0..3 {
        let key = format!("active_{}", i);
        backend
            .save(&key, &json!({ "test": i }), Some(3600))
            .await
            .expect("Failed to save session");
    }

    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Clean up expired sessions
    let deleted = backend.cleanup_expired().await.expect("Failed to cleanup");

    assert_eq!(deleted, 5);

    // Verify active sessions still exist
    for i in 0..3 {
        let key = format!("active_{}", i);
        assert!(
            backend
                .exists(&key)
                .await
                .expect("Failed to check existence")
        );
    }
}
