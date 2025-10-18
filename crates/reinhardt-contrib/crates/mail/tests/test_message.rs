//! Tests for EmailMessage functionality
//! Based on Django's django/tests/mail/tests.py

use reinhardt_mail::{EmailMessage, MemoryBackend};

#[tokio::test]
async fn test_ascii() {
    // Test basic ASCII email message creation
    let email = EmailMessage::new()
        .subject("Subject")
        .body("Content\n")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    assert_eq!(email.subject, "Subject");
    assert_eq!(email.body, "Content\n");
    assert_eq!(email.from_email, "from@example.com");
    assert_eq!(email.to, vec!["to@example.com"]);
}

#[tokio::test]
async fn test_mail_message_multiple_recipients() {
    let email = EmailMessage::new()
        .subject("Subject")
        .body("Content\n")
        .from("from@example.com")
        .to(vec!["to@example.com", "other@example.com"])
        .build()
        .unwrap();

    assert_eq!(email.subject, "Subject");
    assert_eq!(email.body, "Content\n");
    assert_eq!(email.from_email, "from@example.com");
    assert_eq!(email.to.len(), 2);
    assert!(email.to.contains(&"to@example.com".to_string()));
    assert!(email.to.contains(&"other@example.com".to_string()));
}

#[tokio::test]
async fn test_header_omitted_for_no_to_recipients() {
    // Email with only CC recipients should not have a To header in actual impl
    let email = EmailMessage::new()
        .subject("Subject")
        .body("Content")
        .from("from@example.com")
        .add_cc("cc@example.com")
        .build()
        .unwrap();

    assert!(email.to.is_empty());
    assert_eq!(email.cc.len(), 1);
    assert_eq!(email.cc[0], "cc@example.com");
}

#[tokio::test]
async fn test_recipients_with_empty_strings() {
    // Empty strings should be filtered out before building
    let to_addrs = vec!["to@example.com", ""];
    let cc_addrs = vec!["cc@example.com", ""];
    let bcc_addrs = vec!["", "bcc@example.com"];

    // Filter empty strings before building
    let to_filtered: Vec<&str> = to_addrs.iter().filter(|s| !s.is_empty()).copied().collect();
    let cc_filtered: Vec<&str> = cc_addrs.iter().filter(|s| !s.is_empty()).copied().collect();
    let bcc_filtered: Vec<&str> = bcc_addrs
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect();

    let email = EmailMessage::new()
        .subject("Subject")
        .body("Content")
        .from("from@example.com")
        .to(to_filtered)
        .cc(cc_filtered)
        .bcc(bcc_filtered)
        .build()
        .unwrap();

    let recipients = email.recipients();
    assert_eq!(recipients.len(), 3);
    assert!(recipients.contains(&"to@example.com".to_string()));
    assert!(recipients.contains(&"cc@example.com".to_string()));
    assert!(recipients.contains(&"bcc@example.com".to_string()));
}

#[tokio::test]
async fn test_cc() {
    // Regression test for #7722 (Django issue)
    let email = EmailMessage::new()
        .subject("Subject")
        .body("Content")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .cc(vec!["cc@example.com"])
        .build()
        .unwrap();

    assert_eq!(email.cc, vec!["cc@example.com"]);
    let recipients = email.recipients();
    assert_eq!(recipients.len(), 2);
    assert!(recipients.contains(&"to@example.com".to_string()));
    assert!(recipients.contains(&"cc@example.com".to_string()));
}

#[tokio::test]
async fn test_cc_headers() {
    // Test multiple CC and To recipients
    let email = EmailMessage::new()
        .subject("Subject")
        .body("Content")
        .from("from@example.com")
        .to(vec!["to@example.com", "other@example.com"])
        .cc(vec!["cc@example.com", "cc.other@example.com"])
        .build()
        .unwrap();

    let recipients = email.recipients();
    assert_eq!(recipients.len(), 4);
    assert!(recipients.contains(&"to@example.com".to_string()));
    assert!(recipients.contains(&"other@example.com".to_string()));
    assert!(recipients.contains(&"cc@example.com".to_string()));
    assert!(recipients.contains(&"cc.other@example.com".to_string()));
}

#[tokio::test]
async fn test_cc_in_headers_only() {
    // CC can be provided only in headers
    let email = EmailMessage::new()
        .subject("Subject")
        .body("Content")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .header("Cc", "cc@example.com")
        .build()
        .unwrap();

    // Headers are separate from cc list in our implementation
    assert!(email.headers.contains_key("Cc"));
}

#[tokio::test]
async fn test_bcc_not_in_headers() {
    // BCC should be in recipients but not in message headers
    let email = EmailMessage::new()
        .subject("Subject")
        .body("Content")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .bcc(vec!["bcc@example.com"])
        .build()
        .unwrap();

    let recipients = email.recipients();
    assert_eq!(recipients.len(), 2);
    assert!(recipients.contains(&"bcc@example.com".to_string()));

    // BCC should not appear in headers
    assert!(!email.headers.contains_key("Bcc"));
}

#[tokio::test]
async fn test_reply_to() {
    let email = EmailMessage::new()
        .subject("Subject")
        .body("Content")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .reply_to(vec!["reply@example.com"])
        .build()
        .unwrap();

    assert_eq!(email.reply_to, vec!["reply@example.com"]);
}

#[tokio::test]
async fn test_recipients_as_tuple() {
    // In Rust, we use Vec instead of tuples
    let email = EmailMessage::new()
        .subject("Subject")
        .body("Content")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .cc(vec!["cc@example.com"])
        .bcc(vec!["bcc@example.com"])
        .build()
        .unwrap();

    assert_eq!(email.to, vec!["to@example.com"]);
    assert_eq!(email.cc, vec!["cc@example.com"]);
    assert_eq!(email.bcc, vec!["bcc@example.com"]);
}

#[tokio::test]
async fn test_recipients_as_string() {
    // Single string recipient
    let email = EmailMessage::new()
        .subject("Subject")
        .body("Content")
        .from("from@example.com")
        .add_to("to@example.com")
        .build()
        .unwrap();

    assert_eq!(email.to, vec!["to@example.com"]);
}

#[tokio::test]
async fn test_header_injection() {
    // Headers with newlines should be rejected
    let result = EmailMessage::new()
        .subject("Subject\nInjection")
        .body("Content")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build();

    // Should fail validation due to CRLF in subject
    assert!(result.is_err());

    // Test CRLF injection in custom headers
    let result2 = EmailMessage::new()
        .subject("Subject")
        .body("Content")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .header("X-Custom", "Value\r\nBcc: hacker@evil.com")
        .build();

    assert!(result2.is_err());
}

#[tokio::test]
async fn test_message_header_overrides() {
    // Custom headers should override defaults
    let email = EmailMessage::new()
        .subject("Subject")
        .body("Content")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .header("Message-ID", "<custom@example.com>")
        .header("Date", "Mon, 1 Jan 2024 00:00:00 +0000")
        .build()
        .unwrap();

    assert_eq!(
        email.headers.get("Message-ID"),
        Some(&"<custom@example.com>".to_string())
    );
    assert_eq!(
        email.headers.get("Date"),
        Some(&"Mon, 1 Jan 2024 00:00:00 +0000".to_string())
    );
}

#[tokio::test]
async fn test_from_header() {
    // Make sure we can manually set the From header
    let email = EmailMessage::new()
        .subject("Subject")
        .body("Content")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    assert_eq!(email.from_email, "from@example.com");
}

#[tokio::test]
async fn test_to_header() {
    // Make sure we can manually set the To header
    let email = EmailMessage::new()
        .subject("Subject")
        .body("Content")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    assert_eq!(email.to, vec!["to@example.com"]);
}

#[tokio::test]
async fn test_to_in_headers_only() {
    // To can be provided only via headers
    let email = EmailMessage::new()
        .subject("Subject")
        .body("Content")
        .from("from@example.com")
        .header("To", "to@example.com")
        .add_cc("cc@example.com")
        .build()
        .unwrap();

    assert!(email.headers.contains_key("To"));
}

#[tokio::test]
async fn test_reply_to_header() {
    // Reply-To in headers should work
    let email = EmailMessage::new()
        .subject("Subject")
        .body("Content")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .reply_to(vec!["reply@example.com"])
        .header("Reply-To", "override@example.com")
        .build()
        .unwrap();

    // Header should override reply_to
    assert_eq!(
        email.headers.get("Reply-To"),
        Some(&"override@example.com".to_string())
    );
}

#[tokio::test]
async fn test_reply_to_in_headers_only() {
    let email = EmailMessage::new()
        .subject("Subject")
        .body("Content")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .header("Reply-To", "reply@example.com")
        .build()
        .unwrap();

    assert!(email.headers.contains_key("Reply-To"));
}

#[tokio::test]
async fn test_multiple_message_call() {
    // Calling message() multiple times should not change headers
    let backend = MemoryBackend::new();

    let email = EmailMessage::new()
        .subject("Subject")
        .body("Content")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    // Send multiple times
    email.send_with_backend(&backend).await.unwrap();
    email.send_with_backend(&backend).await.unwrap();

    assert_eq!(backend.count(), 2);
}

#[tokio::test]
async fn test_none_body() {
    // Email with empty body should work
    let email = EmailMessage::new()
        .subject("Subject")
        .body("")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    assert_eq!(email.body, "");
}

#[tokio::test]
async fn test_all_params_optional() {
    // All parameters are optional except those required for validation
    // This should fail validation
    let result = EmailMessage::new().build();
    assert!(result.is_err());
}

#[tokio::test]
async fn test_positional_arguments_order() {
    // Test that builder pattern works correctly
    let email = EmailMessage::new()
        .subject("Subject")
        .body("Content")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .cc(vec!["cc@example.com"])
        .bcc(vec!["bcc@example.com"])
        .build()
        .unwrap();

    assert_eq!(email.subject, "Subject");
    assert_eq!(email.body, "Content");
    assert_eq!(email.from_email, "from@example.com");
    assert_eq!(email.to, vec!["to@example.com"]);
    assert_eq!(email.cc, vec!["cc@example.com"]);
    assert_eq!(email.bcc, vec!["bcc@example.com"]);
}

#[tokio::test]
async fn test_all_params_can_be_set_before_send() {
    // All parameters can be set at any time before send
    let mut email = EmailMessage::default();
    email.subject = "Subject".to_string();
    email.body = "Content".to_string();
    email.from_email = "from@example.com".to_string();
    email.to = vec!["to@example.com".to_string()];

    let result = email.validate();
    assert!(result.is_ok());
}
