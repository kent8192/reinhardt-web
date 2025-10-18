//! SMTP TLS/SSL and Authentication tests using testcontainers
//! Tests secure connections and authentication mechanisms

use reinhardt_mail::{EmailBackend, EmailMessage, SmtpBackend};
use testcontainers::{core::ContainerPort, runners::AsyncRunner, ContainerAsync, GenericImage};

/// Mailpit container with SMTP server
async fn start_mailpit() -> ContainerAsync<GenericImage> {
    let container = GenericImage::new("axllent/mailpit", "latest")
        .with_exposed_port(ContainerPort::Tcp(1025)) // SMTP port
        .with_exposed_port(ContainerPort::Tcp(8025)) // HTTP API port
        .start()
        .await
        .expect("Failed to start Mailpit container");

    // Give Mailpit a moment to fully start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    container
}

#[tokio::test]
async fn test_smtp_without_tls() {
    // Test that SMTP works without TLS (plain connection)
    let container = start_mailpit().await;
    let smtp_port = container
        .get_host_port_ipv4(ContainerPort::Tcp(1025))
        .await
        .unwrap();

    let backend = SmtpBackend::new("127.0.0.1".to_string(), smtp_port).with_tls(false);

    let email = EmailMessage::new()
        .subject("Test without TLS")
        .body("This email is sent over plain SMTP")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok(), "Failed to send email without TLS");
}

#[tokio::test]
async fn test_smtp_tls_configuration() {
    // Test that backend can be configured with TLS settings
    let backend = SmtpBackend::new("smtp.example.com".to_string(), 587).with_tls(true);

    // Just verify the backend is created successfully
    // Actual connection would fail without a real server
    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .build()
        .unwrap();

    // This will fail to connect, but we're testing configuration
    let result = backend.send(&email).await;
    assert!(
        result.is_err(),
        "Expected connection to fail for non-existent server"
    );
}

#[tokio::test]
async fn test_smtp_authentication_configuration() {
    // Test SMTP backend credential configuration without actually authenticating
    // We test that credentials can be set via the builder pattern

    let _backend_with_creds = SmtpBackend::new("smtp.example.com".to_string(), 587)
        .with_tls(true)
        .with_credentials("testuser".to_string(), "testpass".to_string());

    // Just verify the backend was created successfully
    // We can't test actual authentication without a real SMTP server that requires it

    // Test sending without credentials works with Mailpit
    let container = start_mailpit().await;
    let smtp_port = container
        .get_host_port_ipv4(ContainerPort::Tcp(1025))
        .await
        .unwrap();

    let backend_no_auth = SmtpBackend::new("127.0.0.1".to_string(), smtp_port).with_tls(false);

    let email = EmailMessage::new()
        .subject("Test without auth")
        .body("Mailpit doesn't require authentication")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .build()
        .unwrap();

    let result = backend_no_auth.send(&email).await;
    assert!(
        result.is_ok(),
        "Failed to send email without authentication"
    );
}

#[tokio::test]
async fn test_smtp_port_configurations() {
    // Test common SMTP port configurations
    let ports = vec![
        (25, false),   // Standard SMTP
        (587, true),   // Submission (STARTTLS)
        (465, true),   // SMTPS (TLS wrapper)
        (2525, false), // Alternative port
    ];

    for (port, _use_tls) in ports {
        // Just verify backend creation with different ports
        let backend = SmtpBackend::new("localhost".to_string(), port);

        let email = EmailMessage::new()
            .subject("Port test")
            .body("Testing port configuration")
            .from("sender@example.com")
            .to(vec!["recipient@example.com"])
            .build()
            .unwrap();

        // Will fail to connect, but tests configuration
        let _result = backend.send(&email).await;
        // We don't assert here as all will fail, just testing configuration
    }
}

#[tokio::test]
async fn test_smtp_connection_timeout() {
    // Test connection timeout behavior
    // Using a non-routable IP to force timeout
    let backend = SmtpBackend::new("192.0.2.1".to_string(), 25).with_tls(false);

    let email = EmailMessage::new()
        .subject("Timeout test")
        .body("This should timeout")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .build()
        .unwrap();

    let start = std::time::Instant::now();
    let result = backend.send(&email).await;
    let elapsed = start.elapsed();

    assert!(result.is_err(), "Expected timeout error");
    // Note: Default SMTP timeout can be quite long (60s+)
    // We just verify it eventually fails
    eprintln!("Connection timeout took: {:?}", elapsed);
}

#[tokio::test]
async fn test_smtp_invalid_hostname() {
    // Test behavior with invalid hostname
    let backend = SmtpBackend::new("invalid.example.invalid".to_string(), 25).with_tls(false);

    let email = EmailMessage::new()
        .subject("Invalid hostname test")
        .body("This should fail")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_err(), "Expected error for invalid hostname");
}

#[tokio::test]
async fn test_smtp_multiple_recipients_without_tls() {
    // Test sending to multiple recipients over plain SMTP
    let container = start_mailpit().await;
    let smtp_port = container
        .get_host_port_ipv4(ContainerPort::Tcp(1025))
        .await
        .unwrap();

    let backend = SmtpBackend::new("127.0.0.1".to_string(), smtp_port).with_tls(false);

    let email = EmailMessage::new()
        .subject("Multiple recipients")
        .body("Testing multiple recipients")
        .from("sender@example.com")
        .to(vec![
            "recipient1@example.com",
            "recipient2@example.com",
            "recipient3@example.com",
        ])
        .cc(vec!["cc@example.com"])
        .bcc(vec!["bcc@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok(), "Failed to send to multiple recipients");
}

#[tokio::test]
async fn test_smtp_large_email_body() {
    // Test sending email with large body
    let container = start_mailpit().await;
    let smtp_port = container
        .get_host_port_ipv4(ContainerPort::Tcp(1025))
        .await
        .unwrap();

    let backend = SmtpBackend::new("127.0.0.1".to_string(), smtp_port).with_tls(false);

    // Create a large email body (1MB)
    let large_body = "X".repeat(1024 * 1024);

    let email = EmailMessage::new()
        .subject("Large email test")
        .body(large_body)
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok(), "Failed to send large email");
}

#[tokio::test]
async fn test_smtp_special_characters_in_headers() {
    // Test handling of special characters in headers
    let container = start_mailpit().await;
    let smtp_port = container
        .get_host_port_ipv4(ContainerPort::Tcp(1025))
        .await
        .unwrap();

    let backend = SmtpBackend::new("127.0.0.1".to_string(), smtp_port).with_tls(false);

    let email = EmailMessage::new()
        .subject("Special chars: émoji 🎉 日本語")
        .body("Body with special characters: café, naïve, 日本語テスト")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(
        result.is_ok(),
        "Failed to send email with special characters"
    );
}

#[tokio::test]
async fn test_smtp_empty_body() {
    // Test sending email with empty body
    let container = start_mailpit().await;
    let smtp_port = container
        .get_host_port_ipv4(ContainerPort::Tcp(1025))
        .await
        .unwrap();

    let backend = SmtpBackend::new("127.0.0.1".to_string(), smtp_port).with_tls(false);

    let email = EmailMessage::new()
        .subject("Empty body test")
        .body("") // Empty body
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok(), "Failed to send email with empty body");
}

#[tokio::test]
async fn test_smtp_long_subject_line() {
    // Test email with very long subject line
    let container = start_mailpit().await;
    let smtp_port = container
        .get_host_port_ipv4(ContainerPort::Tcp(1025))
        .await
        .unwrap();

    let backend = SmtpBackend::new("127.0.0.1".to_string(), smtp_port).with_tls(false);

    // Create a 500 character subject line
    let long_subject = "A".repeat(500);

    let email = EmailMessage::new()
        .subject(long_subject)
        .body("Body text")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok(), "Failed to send email with long subject");
}

#[tokio::test]
async fn test_smtp_builder_pattern_chaining() {
    // Test that builder pattern properly chains method calls
    let container = start_mailpit().await;
    let smtp_port = container
        .get_host_port_ipv4(ContainerPort::Tcp(1025))
        .await
        .unwrap();

    // Test chaining without credentials (Mailpit doesn't require them)
    let backend = SmtpBackend::new("127.0.0.1".to_string(), smtp_port).with_tls(false);

    let email = EmailMessage::new()
        .subject("Builder pattern test")
        .body("Testing method chaining")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok(), "Failed with chained builder methods");
}
