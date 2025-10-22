// Integration tests for static file handling
// These tests verify the interaction between multiple components

use reinhardt_static::handler::{StaticError, StaticFileHandler};
use reinhardt_static::storage::{
    FileSystemStorage, MemoryStorage, StaticFilesConfig, StaticFilesFinder, Storage,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// Test helpers inlined from common module

/// Test file setup helper
struct TestFileSetup {
    temp_dir: TempDir,
    #[allow(dead_code)]
    file_path: PathBuf,
    content: Vec<u8>,
}

impl TestFileSetup {
    fn new(filename: &str, content: &[u8]) -> Self {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(filename);
        fs::write(&file_path, content).unwrap();

        Self {
            temp_dir,
            file_path,
            content: content.to_vec(),
        }
    }
}

mod assertions {
    use reinhardt_static::handler::{StaticError, StaticFile};

    pub fn assert_file_served_successfully(
        result: Result<StaticFile, StaticError>,
        expected_content: &[u8],
    ) {
        assert!(result.is_ok(), "File should be served successfully");
        let static_file = result.unwrap();
        assert_eq!(static_file.content, expected_content);
    }
}

mod integration_helpers {
    use super::*;

    pub struct IntegrationTestSetup {
        pub temp_dirs: Vec<TempDir>,
        #[allow(dead_code)]
        config: StaticFilesConfig,
        pub finder: StaticFilesFinder,
        pub handler: StaticFileHandler,
    }

    impl IntegrationTestSetup {
        pub fn with_multiple_dirs() -> Self {
            let temp_dir1 = TempDir::new().unwrap();
            let temp_dir2 = TempDir::new().unwrap();

            let config = StaticFilesConfig {
                static_root: temp_dir1.path().to_path_buf(),
                static_url: "/static/".to_string(),
                staticfiles_dirs: vec![
                    temp_dir1.path().to_path_buf(),
                    temp_dir2.path().to_path_buf(),
                ],
                media_url: None,
            };

            let finder = StaticFilesFinder::new(config.staticfiles_dirs.clone());
            let handler = StaticFileHandler::new(temp_dir1.path().to_path_buf());

            Self {
                temp_dirs: vec![temp_dir1, temp_dir2],
                config,
                finder,
                handler,
            }
        }

        pub fn create_test_file(&self, filename: &str, content: &[u8]) -> PathBuf {
            let file_path = self.temp_dirs[0].path().join(filename);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&file_path, content).unwrap();
            file_path
        }
    }
}

#[tokio::test]
async fn test_serve_static_file_integration() {
    let setup = TestFileSetup::new("app.css", b"body { color: red; }");
    let handler = StaticFileHandler::new(setup.temp_dir.path().to_path_buf());
    let result = handler.serve("app.css").await;

    assertions::assert_file_served_successfully(result, &setup.content);
}

#[tokio::test]
async fn test_finder_and_handler_integration() {
    let setup = integration_helpers::IntegrationTestSetup::with_multiple_dirs();

    // Create files in different directories
    setup.create_test_file("file1.txt", b"Content 1");
    setup.temp_dirs[1].path().join("file2.txt");
    std::fs::write(setup.temp_dirs[1].path().join("file2.txt"), b"Content 2").unwrap();

    // Find file1 (should be in temp_dir1)
    let found1 = setup.finder.find("file1.txt");
    assert!(found1.is_ok());

    // Find file2 (should be in temp_dir2)
    let found2 = setup.finder.find("file2.txt");
    assert!(found2.is_ok());

    // Now serve the files using handler
    let result = setup.handler.serve("file1.txt").await;
    assert!(result.is_ok());
}

#[test]
fn test_storage_and_finder_integration() {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileSystemStorage::new(temp_dir.path(), "/static/");

    // Save files using storage
    storage.save("file1.txt", b"Content 1").unwrap();
    storage.save("file2.txt", b"Content 2").unwrap();
    storage.save("nested/file3.txt", b"Content 3").unwrap();

    // Create finder with the same directory
    let config = StaticFilesConfig {
        static_root: temp_dir.path().to_path_buf(),
        static_url: "/static/".to_string(),
        staticfiles_dirs: vec![temp_dir.path().to_path_buf()],
        media_url: None,
    };

    let finder = StaticFilesFinder::new(config.staticfiles_dirs);

    // Finder should be able to locate files saved by storage
    let found1 = finder.find("file1.txt");
    assert!(found1.is_ok());

    let found2 = finder.find("file2.txt");
    assert!(found2.is_ok());

    // Verify storage can read the files
    assert_eq!(storage.open("file1.txt").unwrap(), b"Content 1");
    assert_eq!(storage.open("file2.txt").unwrap(), b"Content 2");
}

#[test]
fn test_multiple_storages_same_config() {
    let temp_dir = TempDir::new().unwrap();

    let storage1 = FileSystemStorage::new(temp_dir.path(), "/static/");
    let storage2 = FileSystemStorage::new(temp_dir.path(), "/static/");

    // Save with storage1
    storage1.save("test.txt", b"Test content").unwrap();

    // Read with storage2 (different instance, same location)
    assert!(storage2.exists("test.txt"));
    assert_eq!(storage2.open("test.txt").unwrap(), b"Test content");
}

#[tokio::test]
async fn test_404_handling_integration() {
    let temp_dir = TempDir::new().unwrap();

    let config = StaticFilesConfig {
        static_root: temp_dir.path().to_path_buf(),
        static_url: "/static/".to_string(),
        staticfiles_dirs: vec![temp_dir.path().to_path_buf()],
        media_url: None,
    };

    let finder = StaticFilesFinder::new(config.staticfiles_dirs);
    let handler = StaticFileHandler::new(temp_dir.path().to_path_buf());

    // File doesn't exist
    let found = finder.find("nonexistent.txt");
    assert!(found.is_err());

    // Handler should return NotFound error
    let result = handler.serve("nonexistent.txt").await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), StaticError::NotFound(_)));
}

#[tokio::test]
async fn test_security_integration() {
    let temp_dir = TempDir::new().unwrap();

    // Create a file outside the static root
    let parent_dir = temp_dir.path().parent().unwrap();
    let outside_file = parent_dir.join("sensitive.txt");
    fs::write(&outside_file, b"Sensitive data").unwrap();

    let handler = StaticFileHandler::new(temp_dir.path().to_path_buf());

    // Attempt directory traversal
    let result = handler.serve("../sensitive.txt").await;
    assert!(result.is_err());

    // Cleanup
    let _ = fs::remove_file(&outside_file);
}

#[test]
fn test_memory_storage_isolation() {
    let storage1 = MemoryStorage::new("/static/");
    let storage2 = MemoryStorage::new("/static/");

    // Save with storage1
    storage1.save("file1.txt", b"Content 1").unwrap();

    // storage2 should not see storage1's files (different instances)
    assert!(!storage2.exists("file1.txt"));

    // Save with storage2
    storage2.save("file2.txt", b"Content 2").unwrap();

    // storage1 should not see storage2's files
    assert!(!storage1.exists("file2.txt"));
}

#[test]
fn test_url_consistency_across_operations() {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileSystemStorage::new(temp_dir.path(), "/static/");

    // Save file and get URL
    let url = storage.save("test.txt", b"Content").unwrap();
    assert_eq!(url, "/static/test.txt");

    // URL method should return the same URL
    assert_eq!(storage.url("test.txt"), "/static/test.txt");
}

#[tokio::test]
async fn test_serve_with_custom_index_files_integration() {
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("docs");
    fs::create_dir(&subdir).unwrap();

    // Create custom index file
    fs::write(subdir.join("home.html"), b"<html>Home</html>").unwrap();

    // Handler with custom index files
    let handler = StaticFileHandler::new(temp_dir.path().to_path_buf())
        .with_index_files(vec!["home.html".to_string(), "index.html".to_string()]);

    let result = handler.serve("docs").await;
    assert!(result.is_ok());
    let static_file = result.unwrap();
    assert_eq!(static_file.content, b"<html>Home</html>");
}

#[test]
fn test_finder_priority_order() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();

    // Create same file in both directories with different content
    fs::write(temp_dir1.path().join("duplicate.txt"), b"From dir 1").unwrap();
    fs::write(temp_dir2.path().join("duplicate.txt"), b"From dir 2").unwrap();

    // temp_dir1 should have priority (listed first)
    let config = StaticFilesConfig {
        static_root: temp_dir1.path().to_path_buf(),
        static_url: "/static/".to_string(),
        staticfiles_dirs: vec![
            temp_dir1.path().to_path_buf(),
            temp_dir2.path().to_path_buf(),
        ],
        media_url: None,
    };

    let finder = StaticFilesFinder::new(config.staticfiles_dirs);
    let found = finder.find("duplicate.txt").unwrap();

    // Read the found file to verify it's from temp_dir1
    let content = fs::read(&found).unwrap();
    assert_eq!(content, b"From dir 1");
}

#[tokio::test]
async fn test_etag_generation_integration() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, b"Test content").unwrap();

    let handler = StaticFileHandler::new(temp_dir.path().to_path_buf());
    let result = handler.serve("test.txt").await;

    assert!(result.is_ok());
    let static_file = result.unwrap();

    // ETag should be generated
    let etag = static_file.etag();
    assert!(!etag.is_empty());
    assert!(etag.starts_with('"'));
    assert!(etag.ends_with('"'));

    // ETag should be consistent for same content
    let result2 = handler.serve("test.txt").await;
    let static_file2 = result2.unwrap();
    assert_eq!(static_file.etag(), static_file2.etag());
}

#[test]
fn test_concurrent_storage_operations() {
    use std::sync::Arc;
    use std::thread;

    let temp_dir = TempDir::new().unwrap();
    let storage = Arc::new(FileSystemStorage::new(temp_dir.path(), "/static/"));

    let mut handles = vec![];

    // Spawn multiple threads to save files concurrently
    for i in 0..5 {
        let storage_clone = Arc::clone(&storage);
        let handle = thread::spawn(move || {
            let filename = format!("file{}.txt", i);
            let content = format!("Content {}", i);
            storage_clone.save(&filename, content.as_bytes()).unwrap();
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all files were saved
    for i in 0..5 {
        let filename = format!("file{}.txt", i);
        assert!(storage.exists(&filename));
        let content = storage.open(&filename).unwrap();
        assert_eq!(content, format!("Content {}", i).as_bytes());
    }
}
