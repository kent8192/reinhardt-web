//! Tests for HTML/multipart email
//! Based on Django's django/tests/mail/tests.py

use reinhardt_mail::{Attachment, EmailMessage, MemoryBackend};
use tempfile::TempDir;

#[tokio::test]
async fn test_html_email() {
    let email = EmailMessage::new()
        .subject("Test")
        .body("Plain text content")
        .html("<p>This is <strong>HTML</strong> content</p>")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    assert_eq!(email.body, "Plain text content");
    assert_eq!(
        email.html_body,
        Some("<p>This is <strong>HTML</strong> content</p>".to_string())
    );
}

#[tokio::test]
async fn test_alternatives() {
    // Test HTML alternative
    let html_content = "<p>This is <strong>html</strong></p>";

    let email = EmailMessage::new()
        .subject("Test")
        .body("Plain text")
        .html(html_content)
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    assert_eq!(email.html_body, Some(html_content.to_string()));
}

#[tokio::test]
async fn test_multipart_with_attachments() {
    // EmailMessage with HTML and attachments
    let html_content = "<p>This is <strong>html</strong></p>";

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("example.txt");
    std::fs::write(&file_path, b"Text file content").unwrap();

    let email = EmailMessage::new()
        .subject("Test")
        .body("")
        .html(html_content)
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .attach_file(file_path)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(email.body, "");
    assert_eq!(email.html_body, Some(html_content.to_string()));
    assert_eq!(email.attachments.len(), 1);

    // Cleanup happens automatically
}

#[tokio::test]
async fn test_attachments_with_alternative_parts() {
    // Message with attachment and alternative has correct structure
    let text_content = "This is an important message.";
    let html_content = "<p>This is an <strong>important</strong> message.</p>";

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("attachment.pdf");
    std::fs::write(&file_path, b"%PDF-1.4.%...").unwrap();

    let email = EmailMessage::new()
        .subject("Test")
        .body(text_content)
        .html(html_content)
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .attach_file(file_path)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(email.body, text_content);
    assert_eq!(email.html_body, Some(html_content.to_string()));
    assert_eq!(email.attachments.len(), 1);
    assert_eq!(email.attachments[0].content_type, "application/pdf");

    // Cleanup happens automatically
}

#[tokio::test]
async fn test_html_only() {
    // Email with only HTML content (no plain text)
    let html_content = "<h1>Welcome!</h1><p>This is HTML only.</p>";

    let email = EmailMessage::new()
        .subject("Test")
        .body("") // Empty body
        .html(html_content)
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    assert_eq!(email.body, "");
    assert_eq!(email.html_body, Some(html_content.to_string()));
}

#[tokio::test]
async fn test_plain_text_only() {
    // Email with only plain text (no HTML)
    let plain_content = "This is plain text only.";

    let email = EmailMessage::new()
        .subject("Test")
        .body(plain_content)
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    assert_eq!(email.body, plain_content);
    assert_eq!(email.html_body, None);
}

#[tokio::test]
async fn test_multipart_send() {
    // Test that multipart emails can be sent
    let backend = MemoryBackend::new();

    let email = EmailMessage::new()
        .subject("Test")
        .body("Plain text")
        .html("<p>HTML content</p>")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    email.send_with_backend(&backend).await.unwrap();

    assert_eq!(backend.count(), 1);
    let messages = backend.get_messages();
    assert_eq!(messages[0].body, "Plain text");
    assert_eq!(
        messages[0].html_body,
        Some("<p>HTML content</p>".to_string())
    );
}

#[tokio::test]
async fn test_unicode_in_html() {
    // Test Unicode characters in HTML content
    let html_content = "<p>Hello 世界! Привет мир! مرحبا العالم!</p>";

    let email = EmailMessage::new()
        .subject("Unicode Test")
        .body("Plain unicode: 世界")
        .html(html_content)
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    assert_eq!(email.html_body, Some(html_content.to_string()));
}

#[tokio::test]
async fn test_html_with_inline_images() {
    // Test HTML with inline images (using Content-ID)
    let html_content = r#"<html><body><img src="cid:image1"/></body></html>"#;

    let temp_dir = TempDir::new().unwrap();
    let img_path = temp_dir.path().join("logo.png");
    std::fs::write(&img_path, vec![0x89, 0x50, 0x4E, 0x47]).unwrap();

    let inline_attachment = Attachment::from_file(img_path)
        .unwrap()
        .inline("image1".to_string());

    let email = EmailMessage::new()
        .subject("Test")
        .body("See logo")
        .html(html_content)
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .attach(inline_attachment)
        .build()
        .unwrap();

    assert_eq!(email.attachments.len(), 1);
    assert!(email.attachments[0].inline);
    assert_eq!(email.attachments[0].content_id, Some("image1".to_string()));

    // Cleanup happens automatically
}

#[tokio::test]
async fn test_complex_multipart() {
    // Test complex email with plain text, HTML, regular attachment, and inline image
    let temp_dir = TempDir::new().unwrap();

    // Regular attachment
    let doc_path = temp_dir.path().join("document.pdf");
    std::fs::write(&doc_path, b"%PDF-1.4").unwrap();

    // Inline image
    let img_path = temp_dir.path().join("logo.png");
    std::fs::write(&img_path, vec![0x89, 0x50, 0x4E, 0x47]).unwrap();

    let inline_img = Attachment::from_file(img_path)
        .unwrap()
        .inline("logo".to_string());

    let email = EmailMessage::new()
        .subject("Complex Email")
        .body("Plain text version")
        .html(r#"<html><body><p>HTML version</p><img src="cid:logo"/></body></html>"#)
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .attach_file(doc_path)
        .unwrap()
        .attach(inline_img)
        .build()
        .unwrap();

    assert_eq!(email.attachments.len(), 2);
    assert!(!email.attachments[0].inline); // PDF
    assert!(email.attachments[1].inline); // PNG

    // Cleanup happens automatically
}

#[tokio::test]
async fn test_html_escaping_not_required() {
    // HTML content is not escaped - user is responsible for escaping
    let html = "<script>alert('test')</script>";

    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .html(html)
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    // Content is stored as-is
    assert_eq!(email.html_body, Some(html.to_string()));
}
