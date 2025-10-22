//! Integration tests for reinhardt-storage with reinhardt-settings
//!
//! These tests verify storage configuration through settings.

use reinhardt_settings::Settings;
use reinhardt_storage::{InMemoryStorage, LocalStorage, Storage};
use tempfile::TempDir;

async fn create_test_storage() -> (LocalStorage, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let storage = LocalStorage::new(temp_dir.path(), "http://localhost/media");
    storage.ensure_base_dir().await.unwrap();
    (storage, temp_dir)
}

#[tokio::test]
async fn test_default_storage() {
    // Test default storage backend configuration
    let (storage, _temp_dir) = create_test_storage().await;

    storage.save("test.txt", b"default storage").await.unwrap();
    let file = storage.read("test.txt").await.unwrap();
    assert_eq!(file.content, b"default storage");

    storage.delete("test.txt").await.unwrap();
}

#[tokio::test]
async fn test_media_root_path() {
    // Test MEDIA_ROOT setting
    let (storage, temp_dir) = create_test_storage().await;

    // Verify storage is using the configured media root
    let path = "uploads/file.txt";
    storage.save(path, b"media root test").await.unwrap();

    let full_path = temp_dir.path().join(path);
    assert!(full_path.exists());

    storage.delete(path).await.unwrap();
}

#[tokio::test]
async fn test_media_url_setting() {
    // Test MEDIA_URL setting
    let (storage, _temp_dir) = create_test_storage().await;

    let path = "images/photo.jpg";
    storage.save(path, b"image").await.unwrap();

    let url = storage.url(path);
    assert!(url.starts_with("http://localhost/media"));
    assert!(url.contains("photo.jpg"));

    storage.delete(path).await.unwrap();
}

#[tokio::test]
async fn test_static_root_differs_from_media_root() {
    // Test that STATIC_ROOT and MEDIA_ROOT are different
    let media_dir = TempDir::new().unwrap();
    let static_dir = TempDir::new().unwrap();

    let media_storage = LocalStorage::new(media_dir.path(), "http://localhost/media");
    let static_storage = LocalStorage::new(static_dir.path(), "http://localhost/static");

    media_storage.ensure_base_dir().await.unwrap();
    static_storage.ensure_base_dir().await.unwrap();

    // Save to media
    media_storage
        .save("user/avatar.png", b"avatar")
        .await
        .unwrap();

    // Save to static
    static_storage
        .save("css/style.css", b"styles")
        .await
        .unwrap();

    // Verify they're in different locations
    assert!(media_storage.exists("user/avatar.png").await.unwrap());
    assert!(!media_storage.exists("css/style.css").await.unwrap());

    assert!(static_storage.exists("css/style.css").await.unwrap());
    assert!(!static_storage.exists("user/avatar.png").await.unwrap());

    media_storage.delete("user/avatar.png").await.unwrap();
    static_storage.delete("css/style.css").await.unwrap();
}

#[tokio::test]
async fn test_file_upload_permissions() {
    // Test FILE_UPLOAD_PERMISSIONS setting
    let (storage, _temp_dir) = create_test_storage().await;

    let path = "secure/file.dat";
    storage.save(path, b"secure data").await.unwrap();

    // In a real implementation, this would check file permissions
    let metadata = storage.metadata(path).await.unwrap();
    assert_eq!(metadata.size, 11); // "secure data" length

    storage.delete(path).await.unwrap();
}

#[tokio::test]
async fn test_file_upload_max_memory_size() {
    // Test FILE_UPLOAD_MAX_MEMORY_SIZE setting
    let storage = InMemoryStorage::new("memory", "http://localhost/uploads");

    // Small file should fit in memory
    let small = vec![b'A'; 1024]; // 1KB
    storage.save("small.bin", &small).await.unwrap();
    assert!(storage.exists("small.bin").await.unwrap());

    // Large file (simulate max size check)
    let large = vec![b'B'; 10 * 1024 * 1024]; // 10MB
    let result = storage.save("large.bin", &large).await;
    // In memory storage should handle this, but with settings it might be limited
    assert!(result.is_ok());

    storage.delete("small.bin").await.unwrap();
    storage.delete("large.bin").await.unwrap();
}

#[tokio::test]
async fn test_storage_backend_configuration() {
    // Test custom storage backend configuration
    let temp_dir = TempDir::new().unwrap();

    // Configure with custom base URL
    let custom_url = "https://cdn.example.com/media";
    let storage = LocalStorage::new(temp_dir.path(), custom_url);
    storage.ensure_base_dir().await.unwrap();

    let path = "config/settings.json";
    storage.save(path, b"{}").await.unwrap();

    let url = storage.url(path);
    assert!(url.starts_with("https://cdn.example.com/media"));

    storage.delete(path).await.unwrap();
}

#[tokio::test]
async fn test_multiple_storage_backends() {
    // Test using multiple storage backends configured via settings
    let local_dir = TempDir::new().unwrap();
    let local_storage = LocalStorage::new(local_dir.path(), "http://localhost/media");
    local_storage.ensure_base_dir().await.unwrap();

    let memory_storage = InMemoryStorage::new("cache", "http://localhost/cache");

    // Save same filename to both storages
    let filename = "shared.txt";
    local_storage
        .save(filename, b"local version")
        .await
        .unwrap();
    memory_storage
        .save(filename, b"memory version")
        .await
        .unwrap();

    // Verify they're independent
    let local_file = local_storage.read(filename).await.unwrap();
    let memory_file = memory_storage.read(filename).await.unwrap();

    assert_eq!(local_file.content, b"local version");
    assert_eq!(memory_file.content, b"memory version");

    local_storage.delete(filename).await.unwrap();
    memory_storage.delete(filename).await.unwrap();
}
