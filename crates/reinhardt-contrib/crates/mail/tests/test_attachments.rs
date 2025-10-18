//! Tests for email attachments
//! Based on Django's django/tests/mail/tests.py

use reinhardt_mail::{Attachment, EmailBackend, EmailMessage, MemoryBackend};
use tempfile::TempDir;

#[tokio::test]
async fn test_attachments() {
    // Create a temporary file for testing
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("example.txt");
    std::fs::write(&file_path, b"Text file content\n").unwrap();

    let attachment = Attachment::from_file(file_path.clone()).unwrap();

    assert_eq!(attachment.filename, "example.txt");
    assert_eq!(attachment.content, b"Text file content\n");
    assert_eq!(attachment.content_type, "text/plain");

    // Cleanup happens automatically when temp_dir is dropped
}

#[tokio::test]
async fn test_attachments_constructor() {
    let file_name = "example.txt";
    let file_content = b"Text file content\n";
    let mime_type = "text/plain";

    let attachment = Attachment {
        filename: file_name.to_string(),
        content: file_content.to_vec(),
        content_type: mime_type.to_string(),
        inline: false,
        content_id: None,
    };

    assert_eq!(attachment.filename, file_name);
    assert_eq!(attachment.content, file_content);
    assert_eq!(attachment.content_type, mime_type);
}

#[tokio::test]
async fn test_attach_in_email() {
    let attachment = Attachment {
        filename: "example.txt".to_string(),
        content: b"Text file content\n".to_vec(),
        content_type: "text/plain".to_string(),
        inline: false,
        content_id: None,
    };

    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .attach(attachment.clone())
        .build()
        .unwrap();

    assert_eq!(email.attachments.len(), 1);
    assert_eq!(email.attachments[0].filename, "example.txt");
    assert_eq!(email.attachments[0].content, b"Text file content\n");
}

#[tokio::test]
async fn test_attach_file() {
    // Create temporary files with different types
    let temp_dir = TempDir::new().unwrap();

    // Text file
    let txt_path = temp_dir.path().join("file.txt");
    std::fs::write(&txt_path, b"Text content").unwrap();

    // Create email and attach file
    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .attach_file(txt_path)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(email.attachments.len(), 1);
    assert_eq!(email.attachments[0].filename, "file.txt");
    assert_eq!(email.attachments[0].content, b"Text content");
    assert_eq!(email.attachments[0].content_type, "text/plain");

    // Cleanup happens automatically
}

#[tokio::test]
async fn test_non_ascii_attachment_filename() {
    // Regression test for Django #14964
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("une pièce jointe.pdf");
    std::fs::write(&file_path, b"%PDF-1.4.%...").unwrap();

    let attachment = Attachment::from_file(file_path).unwrap();

    assert_eq!(attachment.filename, "une pièce jointe.pdf");
    assert_eq!(attachment.content, b"%PDF-1.4.%...");

    // Cleanup happens automatically
}

#[tokio::test]
async fn test_inline_attachment() {
    let attachment = Attachment {
        filename: "image.png".to_string(),
        content: vec![0x89, 0x50, 0x4E, 0x47], // PNG header
        content_type: "image/png".to_string(),
        inline: false,
        content_id: None,
    }
    .inline("cid123".to_string());

    assert!(attachment.inline);
    assert_eq!(attachment.content_id, Some("cid123".to_string()));
}

#[tokio::test]
async fn test_multiple_attachments() {
    let temp_dir = TempDir::new().unwrap();

    let file1 = temp_dir.path().join("file1.txt");
    let file2 = temp_dir.path().join("file2.txt");

    std::fs::write(&file1, b"Content 1").unwrap();
    std::fs::write(&file2, b"Content 2").unwrap();

    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .attach_file(file1)
        .unwrap()
        .attach_file(file2)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(email.attachments.len(), 2);
    assert_eq!(email.attachments[0].filename, "file1.txt");
    assert_eq!(email.attachments[1].filename, "file2.txt");

    // Cleanup happens automatically
}

#[tokio::test]
async fn test_attachment_with_email() {
    // Test that attachments are sent correctly
    let backend = MemoryBackend::new();

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, b"Test content").unwrap();

    let email = EmailMessage::new()
        .subject("Test with attachment")
        .body("See attached file")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .attach_file(file_path)
        .unwrap()
        .build()
        .unwrap();

    email.send_with_backend(&backend).await.unwrap();

    assert_eq!(backend.count(), 1);
    let messages = backend.get_messages();
    assert_eq!(messages[0].attachments.len(), 1);
    assert_eq!(messages[0].attachments[0].filename, "test.txt");

    // Cleanup happens automatically
}

#[tokio::test]
async fn test_attach_binary_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("binary.bin");

    // Create a binary file
    let binary_data: Vec<u8> = vec![0x00, 0x01, 0x02, 0xFF, 0xFE];
    std::fs::write(&file_path, &binary_data).unwrap();

    let attachment = Attachment::from_file(file_path).unwrap();

    assert_eq!(attachment.filename, "binary.bin");
    assert_eq!(attachment.content, binary_data);
    // Binary files should default to application/octet-stream
    assert_eq!(attachment.content_type, "application/octet-stream");

    // Cleanup happens automatically
}

#[tokio::test]
async fn test_attach_image_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("image.png");

    // PNG file signature
    let png_data: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    std::fs::write(&file_path, &png_data).unwrap();

    let attachment = Attachment::from_file(file_path).unwrap();

    assert_eq!(attachment.filename, "image.png");
    assert_eq!(attachment.content, png_data);
    assert_eq!(attachment.content_type, "image/png");

    // Cleanup happens automatically
}

#[tokio::test]
async fn test_attachment_without_extension() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("noextension");

    std::fs::write(&file_path, b"Some content").unwrap();

    let attachment = Attachment::from_file(file_path).unwrap();

    assert_eq!(attachment.filename, "noextension");
    // Files without extension should default to application/octet-stream
    assert_eq!(attachment.content_type, "application/octet-stream");

    // Cleanup happens automatically
}
