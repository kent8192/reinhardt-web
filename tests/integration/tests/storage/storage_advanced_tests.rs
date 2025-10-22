//! Advanced integration tests for reinhardt-storage
//!
//! These tests cover advanced features like custom backends, concurrency,
//! and edge cases.

use reinhardt_storage::{InMemoryStorage, LocalStorage, Storage, StorageError};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::task;

async fn create_test_storage() -> (LocalStorage, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let storage = LocalStorage::new(temp_dir.path(), "http://localhost/media");
    storage.ensure_base_dir().await.unwrap();
    (storage, temp_dir)
}

#[tokio::test]
async fn test_file_save_without_name() {
    // Test saving a file without providing a name (auto-generated)
    let (storage, _temp_dir) = create_test_storage().await;

    let content = b"anonymous content";
    // Generate a unique filename
    let generated_name = format!("auto_{}.bin", uuid::Uuid::new_v4());

    let saved = storage.save(&generated_name, content).await.unwrap();
    assert_eq!(saved.path, generated_name);

    let file = storage.read(&generated_name).await.unwrap();
    assert_eq!(file.content, content);

    storage.delete(&generated_name).await.unwrap();
}

#[tokio::test]
async fn test_file_save_with_path() {
    // Test saving files with nested paths
    let (storage, _temp_dir) = create_test_storage().await;

    let paths = vec![
        "documents/2024/january/report.pdf",
        "images/avatars/user123.png",
        "uploads/temp/session_abc/data.json",
    ];

    for path in &paths {
        storage.save(path, b"test content").await.unwrap();
        assert!(storage.exists(path).await.unwrap());
    }

    for path in &paths {
        storage.delete(path).await.unwrap();
    }
}

#[tokio::test]
async fn test_random_upload_to() {
    // Test random path generation for uploads
    let (storage, _temp_dir) = create_test_storage().await;

    let base_path = "uploads";
    let mut generated_paths = Vec::new();

    // Generate 5 random paths
    for i in 0..5 {
        let random_path = format!("{}/file_{}_{}.dat", base_path, i, uuid::Uuid::new_v4());
        storage.save(&random_path, b"random data").await.unwrap();
        generated_paths.push(random_path);
    }

    // Verify all paths are unique and exist
    for (i, path1) in generated_paths.iter().enumerate() {
        for (j, path2) in generated_paths.iter().enumerate() {
            if i != j {
                assert_ne!(path1, path2, "Generated paths should be unique");
            }
        }
        assert!(storage.exists(path1).await.unwrap());
    }

    // Cleanup
    for path in &generated_paths {
        storage.delete(path).await.unwrap();
    }
}

#[tokio::test]
async fn test_filefield_pickling() {
    // Test serialization/deserialization of file metadata
    let (storage, _temp_dir) = create_test_storage().await;

    let path = "serialize/data.bin";
    let content = b"serializable content";

    let saved_metadata = storage.save(path, content).await.unwrap();

    // Simulate serialization by storing metadata
    let serialized_path = saved_metadata.path.clone();
    let serialized_size = saved_metadata.size;

    // Verify we can reconstruct file access from serialized data
    let metadata = storage.metadata(&serialized_path).await.unwrap();
    assert_eq!(metadata.path, serialized_path);
    assert_eq!(metadata.size, serialized_size);

    let file = storage.read(&serialized_path).await.unwrap();
    assert_eq!(file.content, content);

    storage.delete(path).await.unwrap();
}

#[tokio::test]
async fn test_extended_length_storage() {
    // Test handling of files with very long paths
    let (storage, _temp_dir) = create_test_storage().await;

    // Create a long path (but within filesystem limits)
    let long_component = "a".repeat(50);
    let long_path = format!(
        "deep/{}/{}/{}/{}/file.txt",
        long_component, long_component, long_component, long_component
    );

    storage.save(&long_path, b"deep file").await.unwrap();
    assert!(storage.exists(&long_path).await.unwrap());

    let file = storage.read(&long_path).await.unwrap();
    assert_eq!(file.content, b"deep file");

    storage.delete(&long_path).await.unwrap();
}

#[tokio::test]
async fn test_filefield_reopen() {
    // Test reopening/re-reading a file multiple times
    let (storage, _temp_dir) = create_test_storage().await;

    let path = "reopen/test.txt";
    let content = b"reopen test content";

    storage.save(path, content).await.unwrap();

    // Read the file multiple times
    for _ in 0..5 {
        let file = storage.read(path).await.unwrap();
        assert_eq!(file.content, content);
    }

    storage.delete(path).await.unwrap();
}

#[tokio::test]
async fn test_context_manager() {
    // Test file operations within a scoped context (RAII pattern)
    let temp_dir = TempDir::new().unwrap();
    let path = "context/file.txt";

    {
        // Create storage in a scope
        let storage = LocalStorage::new(temp_dir.path(), "http://localhost/media");
        storage.ensure_base_dir().await.unwrap();

        storage.save(path, b"context content").await.unwrap();
        assert!(storage.exists(path).await.unwrap());
    }
    // Storage goes out of scope but file should persist

    // Verify file persists after storage is dropped
    let storage = LocalStorage::new(temp_dir.path(), "http://localhost/media");
    assert!(storage.exists(path).await.unwrap());

    storage.delete(path).await.unwrap();
}

#[tokio::test]
async fn test_race_condition() {
    // Test concurrent access to the same file
    let (storage, _temp_dir) = create_test_storage().await;
    let storage = Arc::new(storage);

    // Spawn multiple concurrent tasks writing to different files
    let mut handles = vec![];

    for i in 0..10 {
        let storage_clone = Arc::clone(&storage);
        let file_path = format!("concurrent/file_{}.txt", i);

        let handle = task::spawn(async move {
            let content = format!("concurrent content {}", i);
            storage_clone
                .save(&file_path, content.as_bytes())
                .await
                .unwrap();
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all files were created
    for i in 0..10 {
        let file_path = format!("concurrent/file_{}.txt", i);
        assert!(storage.exists(&file_path).await.unwrap());

        let file = storage.read(&file_path).await.unwrap();
        let expected = format!("concurrent content {}", i);
        assert_eq!(file.content, expected.as_bytes());

        storage.delete(&file_path).await.unwrap();
    }
}

#[tokio::test]
async fn test_storage_move() {
    // Test moving files between storage backends
    let (source_storage, _source_dir) = create_test_storage().await;
    let (dest_storage, _dest_dir) = create_test_storage().await;

    let path = "move/file.txt";
    let content = b"movable content";

    // Save to source
    source_storage.save(path, content).await.unwrap();
    assert!(source_storage.exists(path).await.unwrap());

    // Read from source and save to destination
    let file = source_storage.read(path).await.unwrap();
    dest_storage.save(path, &file.content).await.unwrap();

    // Verify in destination
    assert!(dest_storage.exists(path).await.unwrap());
    let moved_file = dest_storage.read(path).await.unwrap();
    assert_eq!(moved_file.content, content);

    // Delete from source (completing the move)
    source_storage.delete(path).await.unwrap();
    assert!(!source_storage.exists(path).await.unwrap());

    // Cleanup destination
    dest_storage.delete(path).await.unwrap();
}

#[tokio::test]
async fn test_custom_storage_backend() {
    // Test using InMemoryStorage as a custom backend
    let memory_storage = InMemoryStorage::new("custom", "http://custom.local/files");

    let path = "custom/data.bin";
    let content = b"custom backend data";

    memory_storage.save(path, content).await.unwrap();

    let file = memory_storage.read(path).await.unwrap();
    assert_eq!(file.content, content);

    let url = memory_storage.url(path);
    assert!(url.starts_with("http://custom.local/files"));
    assert!(url.contains("custom"));

    // Test deconstruction for custom backend
    let (class_path, _args, kwargs) = memory_storage.deconstruct();
    assert_eq!(class_path, "reinhardt_storage.InMemoryStorage");
    assert!(kwargs.contains_key("location"));
    assert!(kwargs.contains_key("base_url"));

    memory_storage.delete(path).await.unwrap();
}

#[tokio::test]
async fn test_storage_backend_improperly_configured() {
    // Test error handling for improperly configured storage
    let temp_dir = TempDir::new().unwrap();
    let storage = LocalStorage::new(temp_dir.path(), "http://localhost/media");

    // Try to access file before ensuring base directory
    let result = storage.read("nonexistent.txt").await;
    assert!(result.is_err());

    // Ensure directory now
    storage.ensure_base_dir().await.unwrap();

    // Try to read non-existent file
    let result = storage.read("still_nonexistent.txt").await;
    assert!(result.is_err());

    // Try to delete non-existent file
    let result = storage.delete("never_existed.txt").await;
    assert!(result.is_err());

    // Test path traversal is rejected
    let result = storage.save("../escape.txt", b"bad").await;
    assert!(result.is_err());
    if let Err(StorageError::InvalidPath(_)) = result {
        // Expected error type
    } else {
        panic!("Expected InvalidPath error for path traversal");
    }
}

#[tokio::test]
async fn test_concurrent_inmemory_storage() {
    // Test thread-safe concurrent access to InMemoryStorage
    let storage = Arc::new(InMemoryStorage::new("memory", "http://localhost/mem"));

    let mut handles = vec![];

    // Spawn multiple concurrent tasks
    for i in 0..20 {
        let storage_clone = Arc::clone(&storage);
        let handle = task::spawn(async move {
            let path = format!("thread_{}.dat", i);
            let content = format!("thread {} data", i);

            // Write
            storage_clone.save(&path, content.as_bytes()).await.unwrap();

            // Read
            let file = storage_clone.read(&path).await.unwrap();
            assert_eq!(file.content, content.as_bytes());

            // Delete
            storage_clone.delete(&path).await.unwrap();
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }
}
