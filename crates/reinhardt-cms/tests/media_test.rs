//! Tests for media management

use reinhardt_cms::media::{CropMode, MediaManager, RenditionSpec};
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn test_media_upload_and_retrieve() {
	let mut manager = MediaManager::new();

	// Upload a file
	let filename = "test.jpg".to_string();
	let data = vec![0u8; 1024]; // 1KB dummy data
	let media = manager.upload(filename.clone(), data).await.unwrap();

	// Verify upload
	assert_eq!(media.filename, filename);
	assert_eq!(media.size, 1024);
	assert_eq!(media.mime_type, "image/jpeg");

	// Retrieve the file
	let retrieved = manager.get(media.id).await.unwrap();
	assert_eq!(retrieved.id, media.id);
	assert_eq!(retrieved.filename, filename);
}

#[rstest]
#[tokio::test]
async fn test_media_delete() {
	let mut manager = MediaManager::new();

	// Upload a file
	let filename = "test.png".to_string();
	let data = vec![0u8; 512];
	let media = manager.upload(filename, data).await.unwrap();

	// Delete the file
	manager.delete(media.id).await.unwrap();

	// Verify deletion
	let result = manager.get(media.id).await;
	assert!(result.is_err());
}

#[rstest]
#[tokio::test]
async fn test_rendition_generation() {
	let mut manager = MediaManager::new();

	// Upload an image
	let filename = "test.jpg".to_string();
	let data = vec![0u8; 2048];
	let media = manager.upload(filename, data).await.unwrap();

	// Create rendition spec
	let spec = RenditionSpec {
		width: Some(800),
		height: Some(600),
		mode: CropMode::Fill,
		format: None,
		quality: Some(85),
	};

	// Get rendition
	let rendition = manager.get_rendition(media.id, spec.clone()).await.unwrap();
	assert_eq!(rendition.media_id, media.id);
	assert_eq!(rendition.spec.width, Some(800));
	assert_eq!(rendition.spec.height, Some(600));

	// Get the same rendition again (should return existing one)
	let rendition2 = manager.get_rendition(media.id, spec).await.unwrap();
	assert_eq!(rendition.id, rendition2.id);
}

#[rstest]
#[tokio::test]
async fn test_mime_type_detection() {
	let mut manager = MediaManager::new();

	// Test various file types
	let test_cases = vec![
		("test.jpg", "image/jpeg"),
		("test.png", "image/png"),
		("test.pdf", "application/pdf"),
		("test.txt", "text/plain"),
		("test.unknown", "application/octet-stream"),
	];

	for (filename, expected_mime) in test_cases {
		let data = vec![0u8; 100];
		let media = manager.upload(filename.to_string(), data).await.unwrap();
		assert_eq!(media.mime_type, expected_mime);
	}
}
