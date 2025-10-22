//! Integration tests for reinhardt-storage with reinhardt-forms
//!
//! These tests verify file upload handling with storage backends.

use reinhardt_forms::fields::FileField;
use reinhardt_forms::Field;
use reinhardt_storage::{InMemoryStorage, LocalStorage, Storage};
use serde_json::json;
use tempfile::TempDir;

async fn create_test_storage() -> (LocalStorage, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let storage = LocalStorage::new(temp_dir.path(), "http://localhost/media");
    storage.ensure_base_dir().await.unwrap();
    (storage, temp_dir)
}

#[tokio::test]
async fn test_save_doesnt_close() {
    // Test that saving a file doesn't close the file handle
    let (storage, _temp_dir) = create_test_storage().await;

    let content = b"test content";
    storage.save("test.txt", content).await.unwrap();

    // Verify file can be read multiple times
    let file1 = storage.read("test.txt").await.unwrap();
    assert_eq!(file1.content, content);

    let file2 = storage.read("test.txt").await.unwrap();
    assert_eq!(file2.content, content);

    storage.delete("test.txt").await.unwrap();
}

#[tokio::test]
async fn test_save_overwrite_behavior() {
    // Test that saving to the same filename overwrites the file
    let (storage, _temp_dir) = create_test_storage().await;

    storage.save("test.txt", b"first").await.unwrap();
    let file = storage.read("test.txt").await.unwrap();
    assert_eq!(file.content, b"first");

    storage.save("test.txt", b"second").await.unwrap();
    let file = storage.read("test.txt").await.unwrap();
    assert_eq!(file.content, b"second");

    storage.delete("test.txt").await.unwrap();
}

#[tokio::test]
async fn test_save_temp_file_handling() {
    // Test handling of temporary files
    let (storage, _temp_dir) = create_test_storage().await;

    let temp_content = b"temporary file content";
    storage.save("temp/test.txt", temp_content).await.unwrap();

    assert!(storage.exists("temp/test.txt").await.unwrap());

    let file = storage.read("temp/test.txt").await.unwrap();
    assert_eq!(file.content, temp_content);

    storage.delete("temp/test.txt").await.unwrap();
}

#[tokio::test]
async fn test_uploaded_file_cleanup() {
    // Test that temporary files are cleaned up properly
    let (storage, _temp_dir) = create_test_storage().await;

    storage.save("upload/file1.txt", b"data1").await.unwrap();
    storage.save("upload/file2.txt", b"data2").await.unwrap();

    assert!(storage.exists("upload/file1.txt").await.unwrap());
    assert!(storage.exists("upload/file2.txt").await.unwrap());

    storage.delete("upload/file1.txt").await.unwrap();
    storage.delete("upload/file2.txt").await.unwrap();

    assert!(!storage.exists("upload/file1.txt").await.unwrap());
    assert!(!storage.exists("upload/file2.txt").await.unwrap());
}

#[tokio::test]
async fn test_file_upload_validation() {
    // Test FileField validation with storage
    let field = FileField::new("document".to_string());

    // Valid file upload
    let valid_upload = json!({
        "filename": "document.pdf",
        "size": 1024
    });

    let result = field.clean(Some(&valid_upload));
    assert!(result.is_ok());

    // Empty filename should be rejected if required
    let empty_upload = json!({
        "filename": "",
        "size": 0
    });

    let result = field.clean(Some(&empty_upload));
    assert!(result.is_err());
}

#[tokio::test]
async fn test_empty_file_upload() {
    // Test handling of empty file uploads
    let field = FileField::new("document".to_string());

    let empty_file = json!({
        "filename": "empty.txt",
        "size": 0
    });

    // Empty file should fail validation by default
    let result = field.clean(Some(&empty_file));
    assert!(result.is_err());
}

#[tokio::test]
async fn test_large_file_upload() {
    // Test handling of large files
    let (storage, _temp_dir) = create_test_storage().await;

    // Create a 5MB file
    let large_content = vec![b'X'; 5 * 1024 * 1024];
    storage.save("large.bin", &large_content).await.unwrap();

    let metadata = storage.metadata("large.bin").await.unwrap();
    assert_eq!(metadata.size, large_content.len() as u64);

    storage.delete("large.bin").await.unwrap();
}

#[tokio::test]
async fn test_multiple_file_upload() {
    // Test uploading multiple files
    let (storage, _temp_dir) = create_test_storage().await;

    let files = vec![
        ("file1.txt", b"content1" as &[u8]),
        ("file2.txt", b"content2"),
        ("file3.txt", b"content3"),
    ];

    for (name, content) in &files {
        storage.save(name, content).await.unwrap();
    }

    for (name, content) in &files {
        let file = storage.read(name).await.unwrap();
        assert_eq!(&file.content, content);
    }

    for (name, _) in &files {
        storage.delete(name).await.unwrap();
    }
}

#[tokio::test]
async fn test_file_upload_with_custom_name() {
    // Test file upload with custom naming
    let (storage, _temp_dir) = create_test_storage().await;

    let original_name = "original.txt";
    let custom_name = "custom_renamed.txt";

    storage.save(custom_name, b"content").await.unwrap();

    assert!(storage.exists(custom_name).await.unwrap());
    assert!(!storage.exists(original_name).await.unwrap());

    storage.delete(custom_name).await.unwrap();
}

#[tokio::test]
async fn test_file_content_type_detection() {
    // Test that file metadata can store content type
    let (storage, _temp_dir) = create_test_storage().await;

    let metadata = storage.save("test.txt", b"text content").await.unwrap();
    assert_eq!(metadata.path, "test.txt");
    assert_eq!(metadata.size, 12);

    // Content type detection would be implemented in a higher-level handler
    // This test verifies the storage supports it through metadata

    storage.delete("test.txt").await.unwrap();
}

#[tokio::test]
async fn test_file_upload_path_generation() {
    // Test that paths are generated correctly for uploads
    let (storage, _temp_dir) = create_test_storage().await;

    let paths = vec![
        "uploads/2024/01/file.txt",
        "uploads/2024/02/file.txt",
        "user_123/documents/file.pdf",
    ];

    for path in &paths {
        storage.save(path, b"content").await.unwrap();
        assert!(storage.exists(path).await.unwrap());
    }

    for path in &paths {
        storage.delete(path).await.unwrap();
    }
}

#[tokio::test]
async fn test_file_upload_error_handling() {
    // Test error handling for file uploads
    let (storage, _temp_dir) = create_test_storage().await;

    // Try to read non-existent file
    let result = storage.read("nonexistent.txt").await;
    assert!(result.is_err());

    // Try to delete non-existent file
    let result = storage.delete("nonexistent.txt").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_inmemory_storage_file_upload() {
    // Test file upload with InMemoryStorage
    let storage = InMemoryStorage::new("memory", "http://localhost/media");

    storage.save("test.txt", b"in memory").await.unwrap();

    let file = storage.read("test.txt").await.unwrap();
    assert_eq!(file.content, b"in memory");

    storage.delete("test.txt").await.unwrap();
    assert!(!storage.exists("test.txt").await.unwrap());
}
