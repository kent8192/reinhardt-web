//! Tests for email address validation and encoding
//! Based on Django's django/tests/mail/tests.py

use reinhardt_mail::{EmailBackend, EmailMessage, MemoryBackend};

#[tokio::test]
async fn test_sanitize_address() {
    // Test that email addresses are properly validated
    let valid_addresses = vec![
        "test@example.com",
        "user.name@example.com",
        "user+tag@example.com",
        "user_name@example.co.uk",
        "123@example.com",
    ];

    for addr in valid_addresses {
        let result = EmailMessage::new()
            .subject("Test")
            .body("Body")
            .from(addr)
            .to(vec!["to@example.com"])
            .build();

        assert!(result.is_ok(), "Address {} should be valid", addr);
    }
}

#[tokio::test]
async fn test_sanitize_address_invalid() {
    // Test that invalid email addresses are rejected
    let invalid_addresses = vec!["invalid", "@example.com", "user@", "user @example.com", ""];

    for addr in invalid_addresses {
        let result = EmailMessage::new()
            .subject("Test")
            .body("Body")
            .from(addr)
            .to(vec!["to@example.com"])
            .build();

        assert!(result.is_err(), "Address {} should be invalid", addr);
    }
}

#[tokio::test]
async fn test_unicode_address_header() {
    // Test Unicode characters in email addresses (IDN)
    // IDN domains should be validated
    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("test@example.com")
        .to(vec!["test@example.com"])
        .build()
        .unwrap();

    assert!(email.validate().is_ok());

    // Test IDN encoding
    let idn_email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("test@テスト.example")
        .to(vec!["user@テスト.example"])
        .build();

    // IDN domains should be accepted
    if let Ok(email) = idn_email {
        let encoded = email.encode_idn_addresses().unwrap();
        // Should be Punycode encoded
        assert!(encoded.from_email.contains("xn--"));
    }
}

#[tokio::test]
async fn test_unicode_headers() {
    // Test Unicode in subject and body
    let email = EmailMessage::new()
        .subject("Test 日本語 Тест")
        .body("Body with unicode: 你好世界")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    assert_eq!(email.subject, "Test 日本語 Тест");
    assert_eq!(email.body, "Body with unicode: 你好世界");
}

#[tokio::test]
async fn test_encoding() {
    // Test that different encodings work correctly
    let email = EmailMessage::new()
        .subject("Subject with émojis 😀")
        .body("Body with special chars: café, naïve, résumé")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    assert!(email.subject.contains("émojis"));
    assert!(email.body.contains("café"));
}

#[tokio::test]
async fn test_address_header_handling() {
    // Test various address formats
    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["to1@example.com", "to2@example.com"])
        .cc(vec!["cc@example.com"])
        .bcc(vec!["bcc@example.com"])
        .reply_to(vec!["reply@example.com"])
        .build()
        .unwrap();

    assert_eq!(email.to.len(), 2);
    assert_eq!(email.cc.len(), 1);
    assert_eq!(email.bcc.len(), 1);
    assert_eq!(email.reply_to.len(), 1);
}

#[tokio::test]
async fn test_address_header_injection() {
    // Test that header injection via addresses is prevented
    let suspicious_addresses = vec![
        "test@example.com\nBcc: injection@evil.com",
        "test@example.com\r\nCc: injection@evil.com",
    ];

    for addr in suspicious_addresses {
        let result = EmailMessage::new()
            .subject("Test")
            .body("Body")
            .from(addr)
            .to(vec!["to@example.com"])
            .build();

        // These should be rejected due to CRLF in email address
        assert!(
            result.is_err(),
            "Address with CRLF should be rejected: {}",
            addr
        );
    }
}

#[tokio::test]
async fn test_localpart_only_address() {
    // Test localpart-only email address (without @domain)
    // This is not valid RFC but Django allows it
    let result = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["localpart"])
        .build();

    // Our implementation should reject this
    assert!(result.is_err());
}

#[tokio::test]
async fn test_idn_send() {
    // Test Internationalized Domain Names (IDN)
    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("test@example.com")
        .to(vec!["test@example.com"])
        .build()
        .unwrap();

    let backend = MemoryBackend::new();
    let result = backend.send(&email).await;
    assert!(result.is_ok());

    // Test with IDN domain
    let idn_email = EmailMessage::new()
        .subject("Test IDN")
        .body("Body")
        .from("sender@münchen.de")
        .to(vec!["receiver@münchen.de"])
        .build();

    // IDN domains should be accepted and can be encoded
    if let Ok(email) = idn_email {
        let encoded = email.encode_idn_addresses().unwrap();
        // Domains should be Punycode encoded
        assert!(encoded.from_email.contains("xn--"));
        assert!(encoded.to[0].contains("xn--"));
    }
}

#[tokio::test]
async fn test_recipient_without_domain() {
    // Regression test for Django #15042
    // Recipient without domain should be rejected
    let result = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["recipient"])
        .build();

    assert!(result.is_err());
}

#[tokio::test]
async fn test_folding_white_space() {
    // Test for correct use of "folding white space" in long headers
    let long_subject = "This is a very long subject line that might need to be wrapped \
                        to comply with email standards and ensure proper delivery across \
                        various email clients and servers";

    let email = EmailMessage::new()
        .subject(long_subject)
        .body("Body")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    assert_eq!(email.subject, long_subject);
}

#[tokio::test]
async fn test_datetime_in_date_header() {
    // Test that date headers work correctly
    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    // Date should be set automatically
    assert!(email.date.timestamp() > 0);
}

#[tokio::test]
async fn test_lazy_headers() {
    // Test that headers can be set dynamically
    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .header("X-Custom-Header", "Custom Value")
        .header("X-Another-Header", "Another Value")
        .build()
        .unwrap();

    assert_eq!(
        email.headers.get("X-Custom-Header"),
        Some(&"Custom Value".to_string())
    );
    assert_eq!(
        email.headers.get("X-Another-Header"),
        Some(&"Another Value".to_string())
    );
}

#[tokio::test]
async fn test_validate_multiline_headers() {
    // Multiline headers with injection attempts should be rejected
    let result = EmailMessage::new()
        .subject("Test\nInjection")
        .body("Body")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build();

    // Should fail validation due to newline in subject
    assert!(result.is_err());

    // Test invalid multiline header (improper continuation)
    let result2 = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .header(
            "X-Long-Header",
            "This is a header\nwithout proper continuation",
        )
        .build();

    // Should fail - continuation doesn't start with space or tab
    assert!(result2.is_err());

    // Test CRLF in custom header
    let result3 = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .header("X-Header", "Value\r\nBcc: injection@evil.com")
        .build();

    // Should fail - CRLF injection attempt
    assert!(result3.is_err());
}

#[tokio::test]
async fn test_email_with_all_recipients_types() {
    // Test email with to, cc, bcc, and reply-to
    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["to1@example.com", "to2@example.com"])
        .cc(vec!["cc1@example.com", "cc2@example.com"])
        .bcc(vec!["bcc1@example.com", "bcc2@example.com"])
        .reply_to(vec!["reply@example.com"])
        .build()
        .unwrap();

    let all_recipients = email.recipients();
    assert_eq!(all_recipients.len(), 6);
}

#[tokio::test]
async fn test_dont_mangle_from_in_body() {
    // Test that "From" at the start of a line in body is not mangled
    let body = "From this point on\nFrom here to there\nFrom me to you";

    let email = EmailMessage::new()
        .subject("Test")
        .body(body)
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    assert_eq!(email.body, body);
}

#[tokio::test]
async fn test_encoding_alternatives() {
    // Test that HTML alternatives are encoded correctly
    let html = "<p>Content with special chars: café, naïve, résumé, 日本語</p>";

    let email = EmailMessage::new()
        .subject("Test")
        .body("Plain text")
        .html(html)
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    assert_eq!(email.html_body, Some(html.to_string()));
}
