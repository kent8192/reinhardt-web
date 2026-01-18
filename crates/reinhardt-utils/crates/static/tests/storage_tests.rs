use reinhardt_utils::r#static::storage::{FileSystemStorage, MemoryStorage, Storage};
use tempfile::TempDir;

#[tokio::test]
async fn test_filesystem_storage_save_and_open() {
	let temp_dir = TempDir::new().unwrap();
	let storage = FileSystemStorage::new(temp_dir.path(), "/static/");

	let content = b"Test content";
	let url = storage.save("test.txt", content).await.unwrap();

	assert_eq!(url, "/static/test.txt");
	assert!(storage.exists("test.txt"));

	let read_content = storage.open("test.txt").await.unwrap();
	assert_eq!(read_content, content);
}

#[tokio::test]
async fn test_filesystem_storage_url() {
	let temp_dir = TempDir::new().unwrap();
	let storage = FileSystemStorage::new(temp_dir.path(), "/static/");

	assert_eq!(storage.url("test.txt"), "/static/test.txt");
	assert_eq!(storage.url("/test.txt"), "/static/test.txt");
}

#[tokio::test]
async fn test_filesystem_storage_url_with_trailing_slash() {
	let temp_dir = TempDir::new().unwrap();
	let storage = FileSystemStorage::new(temp_dir.path(), "/static");

	assert_eq!(storage.url("test.txt"), "/static/test.txt");
}

#[tokio::test]
async fn test_filesystem_storage_delete() {
	let temp_dir = TempDir::new().unwrap();
	let storage = FileSystemStorage::new(temp_dir.path(), "/static/");

	let content = b"Test content";
	storage.save("test.txt", content).await.unwrap();
	assert!(storage.exists("test.txt"));

	storage.delete("test.txt").await.unwrap();
	assert!(!storage.exists("test.txt"));
}

#[tokio::test]
async fn test_filesystem_storage_delete_nonexistent() {
	let temp_dir = TempDir::new().unwrap();
	let storage = FileSystemStorage::new(temp_dir.path(), "/static/");

	// Should not error when deleting non-existent file
	let result = storage.delete("nonexistent.txt").await;
	assert!(result.is_ok());
}

#[tokio::test]
async fn test_filesystem_storage_nested_path() {
	let temp_dir = TempDir::new().unwrap();
	let storage = FileSystemStorage::new(temp_dir.path(), "/static/");

	let content = b"Test content";
	let url = storage.save("nested/path/test.txt", content).await.unwrap();

	assert_eq!(url, "/static/nested/path/test.txt");
	assert!(storage.exists("nested/path/test.txt"));

	let read_content = storage.open("nested/path/test.txt").await.unwrap();
	assert_eq!(read_content, content);
}

#[tokio::test]
async fn test_filesystem_storage_open_nonexistent() {
	let temp_dir = TempDir::new().unwrap();
	let storage = FileSystemStorage::new(temp_dir.path(), "/static/");

	let result = storage.open("nonexistent.txt").await;
	assert!(result.is_err());
}

#[tokio::test]
async fn test_memory_storage_save_and_open() {
	let storage = MemoryStorage::new("/static/");

	let content = b"Test content";
	let url = storage.save("test.txt", content).await.unwrap();

	assert_eq!(url, "/static/test.txt");
	assert!(storage.exists("test.txt"));

	let read_content = storage.open("test.txt").await.unwrap();
	assert_eq!(read_content, content);
}

#[tokio::test]
async fn test_memory_storage_url() {
	let storage = MemoryStorage::new("/static/");

	assert_eq!(storage.url("test.txt"), "/static/test.txt");
	assert_eq!(storage.url("/test.txt"), "/static/test.txt");
}

#[tokio::test]
async fn test_memory_storage_delete() {
	let storage = MemoryStorage::new("/static/");

	let content = b"Test content";
	storage.save("test.txt", content).await.unwrap();
	assert!(storage.exists("test.txt"));

	storage.delete("test.txt").await.unwrap();
	assert!(!storage.exists("test.txt"));
}

#[tokio::test]
async fn test_memory_storage_open_nonexistent() {
	let storage = MemoryStorage::new("/static/");

	let result = storage.open("nonexistent.txt").await;
	assert!(result.is_err());
}

#[tokio::test]
async fn test_memory_storage_multiple_files() {
	let storage = MemoryStorage::new("/static/");

	storage.save("file1.txt", b"Content 1").await.unwrap();
	storage.save("file2.txt", b"Content 2").await.unwrap();
	storage.save("file3.txt", b"Content 3").await.unwrap();

	assert!(storage.exists("file1.txt"));
	assert!(storage.exists("file2.txt"));
	assert!(storage.exists("file3.txt"));

	assert_eq!(storage.open("file1.txt").await.unwrap(), b"Content 1");
	assert_eq!(storage.open("file2.txt").await.unwrap(), b"Content 2");
	assert_eq!(storage.open("file3.txt").await.unwrap(), b"Content 3");
}

#[tokio::test]
async fn test_memory_storage_overwrite() {
	let storage = MemoryStorage::new("/static/");

	storage.save("test.txt", b"Original content").await.unwrap();
	assert_eq!(storage.open("test.txt").await.unwrap(), b"Original content");

	storage.save("test.txt", b"New content").await.unwrap();
	assert_eq!(storage.open("test.txt").await.unwrap(), b"New content");
}

#[tokio::test]
async fn test_filesystem_storage_path_normalization() {
	let temp_dir = TempDir::new().unwrap();
	let storage = FileSystemStorage::new(temp_dir.path(), "/static/");

	let content = b"Test content";
	storage.save("/test.txt", content).await.unwrap();

	// Both with and without leading slash should work
	assert!(storage.exists("test.txt"));
	assert!(storage.exists("/test.txt"));

	let read_content1 = storage.open("test.txt").await.unwrap();
	let read_content2 = storage.open("/test.txt").await.unwrap();
	assert_eq!(read_content1, content);
	assert_eq!(read_content2, content);
}
