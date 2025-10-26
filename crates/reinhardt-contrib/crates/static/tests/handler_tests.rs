mod common;

use common::{TestFileSetup, assertions};
use reinhardt_static::handler::{StaticError, StaticFileHandler};
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_serve_existing_file() {
    let setup = TestFileSetup::new("test.txt", b"Test content");
    let handler = StaticFileHandler::new(setup.temp_dir.path().to_path_buf());
    let result = handler.serve("test.txt").await;

    assertions::assert_file_served_successfully(result, &setup.content);
}

#[tokio::test]
async fn test_serve_nonexistent_file() {
    let setup = TestFileSetup::new("test.txt", b"Test content");
    let handler = StaticFileHandler::new(setup.temp_dir.path().to_path_buf());

    let result = handler.serve("nonexistent.txt").await;
    assertions::assert_file_not_found_error(result);
}

#[tokio::test]
async fn test_serve_with_leading_slash() {
    let setup = TestFileSetup::new("test.txt", b"Test content");
    let handler = StaticFileHandler::new(setup.temp_dir.path().to_path_buf());
    let result = handler.serve("/test.txt").await;

    assertions::assert_file_served_successfully(result, &setup.content);
}

#[tokio::test]
async fn test_serve_nested_path() {
    let setup = TestFileSetup::with_nested_path("nested/path", "test.txt", b"Nested content");
    let handler = StaticFileHandler::new(setup.temp_dir.path().to_path_buf());
    let result = handler.serve("nested/path/test.txt").await;

    assertions::assert_file_served_successfully(result, &setup.content);
}

#[tokio::test]
async fn test_directory_traversal_protection() {
    let setup = TestFileSetup::new("test.txt", b"Test content");
    let handler = StaticFileHandler::new(setup.temp_dir.path().to_path_buf());

    // Try to access file outside root using directory traversal
    let result = handler.serve("../outside.txt").await;
    assertions::assert_directory_traversal_blocked(result);
}

#[tokio::test]
async fn test_serve_directory_with_index() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("subdir");
    fs::create_dir(&dir_path).unwrap();
    let index_path = dir_path.join("index.html");
    fs::write(&index_path, b"<html>Index</html>").unwrap();

    let handler = StaticFileHandler::new(temp_dir.path().to_path_buf());
    let result = handler.serve("subdir").await;

    assert!(result.is_ok());
    let static_file = result.unwrap();
    assert_eq!(static_file.content, b"<html>Index</html>");
}

#[tokio::test]
async fn test_serve_directory_without_index() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("subdir");
    fs::create_dir(&dir_path).unwrap();

    let handler = StaticFileHandler::new(temp_dir.path().to_path_buf());
    let result = handler.serve("subdir").await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), StaticError::NotFound(_)));
}

#[tokio::test]
async fn test_custom_index_files() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("subdir");
    fs::create_dir(&dir_path).unwrap();
    let custom_index = dir_path.join("default.html");
    fs::write(&custom_index, b"<html>Custom Index</html>").unwrap();

    let handler = StaticFileHandler::new(temp_dir.path().to_path_buf())
        .with_index_files(vec!["default.html".to_string()]);
    let result = handler.serve("subdir").await;

    assert!(result.is_ok());
    let static_file = result.unwrap();
    assert_eq!(static_file.content, b"<html>Custom Index</html>");
}

#[tokio::test]
async fn test_mime_type_detection_html() {
    let setup = TestFileSetup::new("test.html", b"<html></html>");
    let handler = StaticFileHandler::new(setup.temp_dir.path().to_path_buf());
    let result = handler.serve("test.html").await;

    assert!(result.is_ok());
    let static_file = result.unwrap();
    assert_eq!(static_file.mime_type, "text/html");
}

#[tokio::test]
async fn test_mime_type_detection_css() {
    let setup = TestFileSetup::new("style.css", b"body { color: red; }");
    let handler = StaticFileHandler::new(setup.temp_dir.path().to_path_buf());
    let result = handler.serve("style.css").await;

    assert!(result.is_ok());
    let static_file = result.unwrap();
    assert_eq!(static_file.mime_type, "text/css");
}

#[tokio::test]
async fn test_mime_type_detection_javascript() {
    let setup = TestFileSetup::new("script.js", b"console.log('test');");
    let handler = StaticFileHandler::new(setup.temp_dir.path().to_path_buf());
    let result = handler.serve("script.js").await;

    assert!(result.is_ok());
    let static_file = result.unwrap();
    assert!(
        static_file.mime_type.contains("javascript") || static_file.mime_type == "text/javascript"
    );
}

#[tokio::test]
async fn test_mime_type_detection_json() {
    let setup = TestFileSetup::new("data.json", b"{\"key\": \"value\"}");
    let handler = StaticFileHandler::new(setup.temp_dir.path().to_path_buf());
    let result = handler.serve("data.json").await;

    assert!(result.is_ok());
    let static_file = result.unwrap();
    assert_eq!(static_file.mime_type, "application/json");
}

#[tokio::test]
async fn test_mime_type_detection_png() {
    let setup = TestFileSetup::new("image.png", b"fake png data");
    let handler = StaticFileHandler::new(setup.temp_dir.path().to_path_buf());
    let result = handler.serve("image.png").await;

    assert!(result.is_ok());
    let static_file = result.unwrap();
    assert_eq!(static_file.mime_type, "image/png");
}

#[tokio::test]
async fn test_resolve_path_success() {
    let setup = TestFileSetup::new("test.txt", b"Test");
    let handler = StaticFileHandler::new(setup.temp_dir.path().to_path_buf());
    let result = handler.resolve_path("test.txt").await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_resolve_path_not_found() {
    let setup = TestFileSetup::new("test.txt", b"Test");
    let handler = StaticFileHandler::new(setup.temp_dir.path().to_path_buf());

    let result = handler.resolve_path("nonexistent.txt").await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), StaticError::NotFound(_)));
}
