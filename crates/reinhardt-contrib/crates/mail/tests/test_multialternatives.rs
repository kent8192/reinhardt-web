//! Tests for EmailMultiAlternatives functionality
//! Based on Django's multipart/alternative email tests

use reinhardt_mail::{Alternative, EmailBackend, EmailMessage, MemoryBackend};

#[tokio::test]
async fn test_attach_alternative_basic() {
    // Test basic attach_alternative functionality
    let backend = MemoryBackend::new();

    let email = EmailMessage::new()
        .subject("Test Subject")
        .body("Plain text body")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .attach_alternative("<p>HTML body</p>", "text/html")
        .build()
        .unwrap();

    assert_eq!(email.alternatives.len(), 1);
    assert_eq!(email.alternatives[0].content, "<p>HTML body</p>");
    assert_eq!(email.alternatives[0].content_type, "text/html");

    let result = backend.send(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_attach_multiple_alternatives() {
    // Test attaching multiple alternative content types
    let backend = MemoryBackend::new();

    let email = EmailMessage::new()
        .subject("Test")
        .body("Plain text")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .attach_alternative("<p>HTML version</p>", "text/html")
        .attach_alternative("<amp-html>AMP version</amp-html>", "text/x-amp-html")
        .build()
        .unwrap();

    assert_eq!(email.alternatives.len(), 2);
    assert_eq!(email.alternatives[0].content_type, "text/html");
    assert_eq!(email.alternatives[1].content_type, "text/x-amp-html");

    let result = backend.send(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_alternatives_with_html_body() {
    // Test that html() method and attach_alternative can coexist
    let email = EmailMessage::new()
        .subject("Test")
        .body("Plain text")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .html("<p>HTML from html()</p>")
        .attach_alternative("<amp>AMP content</amp>", "text/x-amp-html")
        .build()
        .unwrap();

    assert!(email.html_body.is_some());
    assert_eq!(email.html_body.as_ref().unwrap(), "<p>HTML from html()</p>");
    assert_eq!(email.alternatives.len(), 1);
    assert_eq!(email.alternatives[0].content_type, "text/x-amp-html");
}

#[tokio::test]
async fn test_alternatives_with_attachments() {
    // Test that alternatives and attachments can coexist
    use reinhardt_mail::Attachment;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    let temp_file = NamedTempFile::new().unwrap();
    std::fs::write(temp_file.path(), b"test content").unwrap();

    let attachment = Attachment::from_file(PathBuf::from(temp_file.path())).unwrap();

    let email = EmailMessage::new()
        .subject("Test")
        .body("Plain text")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .attach_alternative("<p>HTML</p>", "text/html")
        .attach(attachment)
        .build()
        .unwrap();

    assert_eq!(email.alternatives.len(), 1);
    assert_eq!(email.attachments.len(), 1);
}

#[tokio::test]
async fn test_alternative_content_types() {
    // Test various MIME types for alternatives
    let mime_types = vec![
        "text/html",
        "text/x-amp-html",
        "text/calendar",
        "application/json",
    ];

    for mime_type in mime_types {
        let email = EmailMessage::new()
            .subject("Test")
            .body("Plain text")
            .from("from@example.com")
            .to(vec!["to@example.com"])
            .attach_alternative("Alternative content", mime_type)
            .build()
            .unwrap();

        assert_eq!(email.alternatives.len(), 1);
        assert_eq!(email.alternatives[0].content_type, mime_type);
    }
}

#[tokio::test]
async fn test_alternative_struct_creation() {
    // Test Alternative struct direct creation
    let alt = Alternative::new("Content", "text/html");
    assert_eq!(alt.content, "Content");
    assert_eq!(alt.content_type, "text/html");
}
