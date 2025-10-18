//! Tests for email backends
//! Based on Django's django/tests/mail/tests.py

use reinhardt_mail::{ConsoleBackend, EmailBackend, EmailMessage, FileBackend, MemoryBackend};
use tempfile::TempDir;

#[tokio::test]
async fn test_mail_backends_console() {
    let backend = ConsoleBackend::new();

    let email = EmailMessage::new()
        .subject("Test Subject")
        .body("Test body content")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    // Should print to console without error
    let result = backend.send(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_console_backend_with_cc_bcc() {
    let backend = ConsoleBackend::new();

    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .cc(vec!["cc@example.com"])
        .bcc(vec!["bcc@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_console_backend_with_html() {
    let backend = ConsoleBackend::new();

    let email = EmailMessage::new()
        .subject("Test")
        .body("Plain text")
        .html("<p>HTML content</p>")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_mail_backends_memory() {
    let backend = MemoryBackend::new();

    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    backend.send(&email).await.unwrap();

    assert_eq!(backend.count(), 1);
    let messages = backend.get_messages();
    assert_eq!(messages[0].subject, "Test");
    assert_eq!(messages[0].body, "Body");
}

#[tokio::test]
async fn test_memory_backend_multiple_messages() {
    let backend = MemoryBackend::new();

    let email1 = EmailMessage::new()
        .subject("Test 1")
        .body("Body 1")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    let email2 = EmailMessage::new()
        .subject("Test 2")
        .body("Body 2")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    backend.send(&email1).await.unwrap();
    backend.send(&email2).await.unwrap();

    assert_eq!(backend.count(), 2);
    let messages = backend.get_messages();
    assert_eq!(messages[0].subject, "Test 1");
    assert_eq!(messages[1].subject, "Test 2");
}

#[tokio::test]
async fn test_mail_backends_memory_clear() {
    let backend = MemoryBackend::new();

    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    backend.send(&email).await.unwrap();
    assert_eq!(backend.count(), 1);

    backend.clear();
    assert_eq!(backend.count(), 0);
}

#[tokio::test]
async fn test_locmem_shared_messages() {
    // Test that memory backend properly stores messages
    let backend = MemoryBackend::new();

    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    backend.send(&email).await.unwrap();

    // Messages should be retrievable
    let messages = backend.get_messages();
    assert_eq!(messages.len(), 1);
}

#[tokio::test]
async fn test_outbox_not_mutated_after_send() {
    // Test that getting messages doesn't affect the backend
    let backend = MemoryBackend::new();

    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    backend.send(&email).await.unwrap();

    let messages1 = backend.get_messages();
    let messages2 = backend.get_messages();

    assert_eq!(messages1.len(), messages2.len());
    assert_eq!(backend.count(), 1);
}

#[tokio::test]
async fn test_file_backend() {
    let temp_dir = TempDir::new().unwrap();
    let backend = FileBackend::new(temp_dir.path().to_path_buf());

    let email = EmailMessage::new()
        .subject("Test Subject")
        .body("Test body")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    backend.send(&email).await.unwrap();

    // Check that file was created
    let entries: Vec<_> = std::fs::read_dir(temp_dir.path()).unwrap().collect();
    assert_eq!(entries.len(), 1);

    // Read the file and check content
    let file_path = entries[0].as_ref().unwrap().path();
    let content = std::fs::read_to_string(&file_path).unwrap();

    assert!(content.contains("From: from@example.com"));
    assert!(content.contains("To: to@example.com"));
    assert!(content.contains("Subject: Test Subject"));
    assert!(content.contains("Test body"));

    // Cleanup happens automatically
}

#[tokio::test]
async fn test_file_backend_multiple_messages() {
    let temp_dir = TempDir::new().unwrap();
    let backend = FileBackend::new(temp_dir.path().to_path_buf());

    let email1 = EmailMessage::new()
        .subject("Test 1")
        .body("Body 1")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    let email2 = EmailMessage::new()
        .subject("Test 2")
        .body("Body 2")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    backend.send(&email1).await.unwrap();
    backend.send(&email2).await.unwrap();

    // Check that two files were created
    let entries: Vec<_> = std::fs::read_dir(temp_dir.path()).unwrap().collect();
    assert_eq!(entries.len(), 2);

    // Cleanup happens automatically
}

#[tokio::test]
async fn test_file_backend_with_cc_bcc() {
    let temp_dir = TempDir::new().unwrap();
    let backend = FileBackend::new(temp_dir.path().to_path_buf());

    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .cc(vec!["cc@example.com"])
        .bcc(vec!["bcc@example.com"])
        .build()
        .unwrap();

    backend.send(&email).await.unwrap();

    let entries: Vec<_> = std::fs::read_dir(temp_dir.path()).unwrap().collect();
    let file_path = entries[0].as_ref().unwrap().path();
    let content = std::fs::read_to_string(&file_path).unwrap();

    assert!(content.contains("Cc: cc@example.com"));
    // BCC should not appear in the file (it's sent but not in headers)

    // Cleanup happens automatically
}

#[tokio::test]
async fn test_file_backend_with_html() {
    let temp_dir = TempDir::new().unwrap();
    let backend = FileBackend::new(temp_dir.path().to_path_buf());

    let email = EmailMessage::new()
        .subject("Test")
        .body("Plain text")
        .html("<p>HTML content</p>")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    backend.send(&email).await.unwrap();

    let entries: Vec<_> = std::fs::read_dir(temp_dir.path()).unwrap().collect();
    let file_path = entries[0].as_ref().unwrap().path();
    let content = std::fs::read_to_string(&file_path).unwrap();

    assert!(content.contains("Plain text"));
    assert!(content.contains("<p>HTML content</p>"));

    // Cleanup happens automatically
}

#[tokio::test]
async fn test_send_messages_bulk() {
    let backend = MemoryBackend::new();

    let emails: Vec<EmailMessage> = (0..3)
        .map(|i| {
            EmailMessage::new()
                .subject(format!("Test {}", i))
                .body(format!("Body {}", i))
                .from("from@example.com")
                .to(vec!["to@example.com"])
                .build()
                .unwrap()
        })
        .collect();

    let email_refs: Vec<&EmailMessage> = emails.iter().collect();
    let results = backend.send_messages(email_refs).await.unwrap();

    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.is_ok()));
    assert_eq!(backend.count(), 3);
}

#[tokio::test]
async fn test_backend_validation() {
    // Backends should validate emails before sending
    let backend = MemoryBackend::new();

    let mut email = EmailMessage::default();
    email.subject = "Test".to_string();
    // Missing from and to - should fail validation

    let result = backend.send(&email).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_close_connection() {
    // Test that backends can be created and dropped without issue
    // (simulating connection closing)
    {
        let backend = MemoryBackend::new();
        let email = EmailMessage::new()
            .subject("Test")
            .body("Body")
            .from("from@example.com")
            .to(vec!["to@example.com"])
            .build()
            .unwrap();

        backend.send(&email).await.unwrap();
    } // backend is dropped here

    // No errors should occur
}

#[tokio::test]
async fn test_use_as_contextmanager() {
    // Test that backends work correctly within a scope
    let backend = MemoryBackend::new();

    {
        let email = EmailMessage::new()
            .subject("Test")
            .body("Body")
            .from("from@example.com")
            .to(vec!["to@example.com"])
            .build()
            .unwrap();

        backend.send(&email).await.unwrap();
    }

    assert_eq!(backend.count(), 1);
}
