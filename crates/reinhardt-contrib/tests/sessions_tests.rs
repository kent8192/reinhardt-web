//! Sessions integration tests
//!
//! Based on Django's sessions tests from:
//! - django/tests/sessions_tests/tests.py

use chrono::Duration;
use reinhardt_cache::InMemoryCache;
use reinhardt_sessions::{
    CacheSessionBackend, InMemorySessionBackend, SessionBackend, SessionData,
};
use std::sync::Arc;

// ========== SessionData Tests ==========

#[test]
fn test_session_data_creation() {
    let session = SessionData::new();
    assert!(!session.is_expired());
}

#[test]
fn test_session_data_with_expiry() {
    let duration = Duration::hours(1);
    let session = SessionData::new().with_expiry(duration);

    // Should not be expired immediately
    assert!(!session.is_expired());
}

#[test]
fn test_session_data_set_and_get() {
    let mut session = SessionData::new();

    session.set("user_id", &42).expect("Failed to set");
    let user_id: Option<i32> = session.get("user_id").expect("Failed to get");

    assert_eq!(user_id, Some(42));
}

#[test]
fn test_session_data_set_multiple_types() {
    let mut session = SessionData::new();

    session.set("user_id", &42).unwrap();
    session.set("username", &"john_doe").unwrap();
    session.set("is_admin", &true).unwrap();

    let user_id: Option<i32> = session.get("user_id").unwrap();
    let username: Option<String> = session.get("username").unwrap();
    let is_admin: Option<bool> = session.get("is_admin").unwrap();

    assert_eq!(user_id, Some(42));
    assert_eq!(username, Some("john_doe".to_string()));
    assert_eq!(is_admin, Some(true));
}

#[test]
fn test_session_data_get_nonexistent() {
    let session = SessionData::new();

    let result: Option<i32> = session.get("nonexistent").unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_session_data_remove() {
    let mut session = SessionData::new();

    session.set("key", &"value").unwrap();
    assert!(session.get::<String>("key").unwrap().is_some());

    session.remove("key");
    assert!(session.get::<String>("key").unwrap().is_none());
}

#[test]
fn test_session_data_clear() {
    let mut session = SessionData::new();

    session.set("key1", &1).unwrap();
    session.set("key2", &2).unwrap();
    session.set("key3", &3).unwrap();

    session.clear();

    assert!(session.get::<i32>("key1").unwrap().is_none());
    assert!(session.get::<i32>("key2").unwrap().is_none());
    assert!(session.get::<i32>("key3").unwrap().is_none());
}

#[test]
fn test_session_data_expiry_check() {
    let expired = SessionData::new().with_expiry(Duration::seconds(-1));
    assert!(expired.is_expired());

    let not_expired = SessionData::new().with_expiry(Duration::hours(1));
    assert!(!not_expired.is_expired());
}

// ========== CacheSessionBackend Tests ==========

#[tokio::test]
async fn test_cache_backend_save_and_load() {
    let cache = Arc::new(InMemoryCache::new());
    let backend = CacheSessionBackend::new(cache);

    let mut session_data = SessionData::new();
    session_data.set("user_id", &123).unwrap();

    backend.save("test_session", &session_data).await.unwrap();

    let loaded = backend.load("test_session").await.unwrap();
    assert!(loaded.is_some());

    let user_id: Option<i32> = loaded.unwrap().get("user_id").unwrap();
    assert_eq!(user_id, Some(123));
}

#[tokio::test]
async fn test_cache_backend_load_nonexistent() {
    let cache = Arc::new(InMemoryCache::new());
    let backend = CacheSessionBackend::new(cache);

    let result = backend.load("nonexistent").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_cache_backend_delete() {
    let cache = Arc::new(InMemoryCache::new());
    let backend = CacheSessionBackend::new(cache);

    let session_data = SessionData::new();
    backend.save("test_session", &session_data).await.unwrap();

    assert!(backend.exists("test_session").await.unwrap());

    backend.delete("test_session").await.unwrap();

    assert!(!backend.exists("test_session").await.unwrap());
}

#[tokio::test]
async fn test_cache_backend_exists() {
    let cache = Arc::new(InMemoryCache::new());
    let backend = CacheSessionBackend::new(cache);

    assert!(!backend.exists("new_session").await.unwrap());

    let session_data = SessionData::new();
    backend.save("new_session", &session_data).await.unwrap();

    assert!(backend.exists("new_session").await.unwrap());
}

#[tokio::test]
async fn test_cache_backend_custom_prefix() {
    let cache = Arc::new(InMemoryCache::new());
    let backend = CacheSessionBackend::new(cache).with_prefix("custom:");

    let session_data = SessionData::new();
    backend.save("test", &session_data).await.unwrap();

    assert!(backend.exists("test").await.unwrap());
}

#[tokio::test]
async fn test_cache_backend_overwrite_session() {
    let cache = Arc::new(InMemoryCache::new());
    let backend = CacheSessionBackend::new(cache);

    let mut session1 = SessionData::new();
    session1.set("value", &1).unwrap();
    backend.save("session", &session1).await.unwrap();

    let mut session2 = SessionData::new();
    session2.set("value", &2).unwrap();
    backend.save("session", &session2).await.unwrap();

    let loaded = backend.load("session").await.unwrap().unwrap();
    let value: Option<i32> = loaded.get("value").unwrap();
    assert_eq!(value, Some(2));
}

#[tokio::test]
async fn test_inmemory_session_backend() {
    let cache = Arc::new(InMemoryCache::new());
    let backend: InMemorySessionBackend = CacheSessionBackend::new(cache);

    let mut session_data = SessionData::new();
    session_data.set("test_key", &"test_value").unwrap();

    backend.save("inmemory_test", &session_data).await.unwrap();

    let loaded = backend.load("inmemory_test").await.unwrap();
    assert!(loaded.is_some());
}

#[tokio::test]
async fn test_session_with_complex_data() {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct UserData {
        id: i32,
        username: String,
        roles: Vec<String>,
    }

    let cache = Arc::new(InMemoryCache::new());
    let backend = CacheSessionBackend::new(cache);

    let user = UserData {
        id: 1,
        username: "alice".to_string(),
        roles: vec!["admin".to_string(), "user".to_string()],
    };

    let mut session_data = SessionData::new();
    session_data.set("user", &user).unwrap();

    backend
        .save("complex_session", &session_data)
        .await
        .unwrap();

    let loaded = backend.load("complex_session").await.unwrap().unwrap();
    let loaded_user: Option<UserData> = loaded.get("user").unwrap();

    assert_eq!(loaded_user, Some(user));
}

#[tokio::test]
async fn test_multiple_sessions() {
    let cache = Arc::new(InMemoryCache::new());
    let backend = CacheSessionBackend::new(cache);

    for i in 0..10 {
        let mut session_data = SessionData::new();
        session_data.set("index", &i).unwrap();
        backend
            .save(&format!("session_{}", i), &session_data)
            .await
            .unwrap();
    }

    for i in 0..10 {
        let loaded = backend.load(&format!("session_{}", i)).await.unwrap();
        assert!(loaded.is_some());
        let index: Option<i32> = loaded.unwrap().get("index").unwrap();
        assert_eq!(index, Some(i));
    }
}
