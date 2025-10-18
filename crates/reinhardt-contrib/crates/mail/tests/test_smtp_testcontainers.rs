//! SMTP backend tests using testcontainers
//! Uses real SMTP server (Mailpit) for integration testing

use reinhardt_mail::{EmailBackend, EmailMessage, SmtpBackend};
use testcontainers::{core::ContainerPort, runners::AsyncRunner, ContainerAsync, GenericImage};

/// Mailpit container for testing
/// Mailpit provides SMTP server on port 1025 and HTTP API on port 8025
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
async fn test_smtp_real_server_basic() {
    let container = start_mailpit().await;
    let smtp_port = container
        .get_host_port_ipv4(ContainerPort::Tcp(1025))
        .await
        .unwrap();

    let backend = SmtpBackend::new("127.0.0.1".to_string(), smtp_port).with_tls(false);

    let email = EmailMessage::new()
        .subject("Test Email")
        .body("This is a test email body")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok(), "Failed to send email: {:?}", result.err());
}

#[tokio::test]
async fn test_smtp_real_server_with_cc_bcc() {
    let container = start_mailpit().await;
    let smtp_port = container
        .get_host_port_ipv4(ContainerPort::Tcp(1025))
        .await
        .unwrap();

    let backend = SmtpBackend::new("127.0.0.1".to_string(), smtp_port).with_tls(false);

    let email = EmailMessage::new()
        .subject("Test with CC and BCC")
        .body("Body text")
        .from("sender@example.com")
        .to(vec!["to@example.com"])
        .cc(vec!["cc@example.com"])
        .bcc(vec!["bcc@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_smtp_real_server_html_email() {
    let container = start_mailpit().await;
    let smtp_port = container
        .get_host_port_ipv4(ContainerPort::Tcp(1025))
        .await
        .unwrap();

    let backend = SmtpBackend::new("127.0.0.1".to_string(), smtp_port).with_tls(false);

    let email = EmailMessage::new()
        .subject("HTML Email Test")
        .body("Plain text version")
        .html("<html><body><h1>HTML Version</h1></body></html>")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_smtp_real_server_with_attachment() {
    use reinhardt_mail::Attachment;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    let container = start_mailpit().await;
    let smtp_port = container
        .get_host_port_ipv4(ContainerPort::Tcp(1025))
        .await
        .unwrap();

    let backend = SmtpBackend::new("127.0.0.1".to_string(), smtp_port).with_tls(false);

    // Create a temporary file to attach
    let temp_file = NamedTempFile::new().unwrap();
    std::fs::write(temp_file.path(), b"Attachment content").unwrap();

    let attachment = Attachment::from_file(PathBuf::from(temp_file.path())).unwrap();

    let email = EmailMessage::new()
        .subject("Email with Attachment")
        .body("See attached file")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .attach(attachment)
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_smtp_real_server_unicode_subject() {
    let container = start_mailpit().await;
    let smtp_port = container
        .get_host_port_ipv4(ContainerPort::Tcp(1025))
        .await
        .unwrap();

    let backend = SmtpBackend::new("127.0.0.1".to_string(), smtp_port).with_tls(false);

    let email = EmailMessage::new()
        .subject("テスト: Unicode Subject 日本語")
        .body("Unicode content: こんにちは")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_smtp_real_server_custom_headers() {
    let container = start_mailpit().await;
    let smtp_port = container
        .get_host_port_ipv4(ContainerPort::Tcp(1025))
        .await
        .unwrap();

    let backend = SmtpBackend::new("127.0.0.1".to_string(), smtp_port).with_tls(false);

    let email = EmailMessage::new()
        .subject("Custom Headers Test")
        .body("Body")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .header("X-Custom-Header", "CustomValue")
        .header("X-Priority", "1")
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_smtp_real_server_multiple_emails() {
    let container = start_mailpit().await;
    let smtp_port = container
        .get_host_port_ipv4(ContainerPort::Tcp(1025))
        .await
        .unwrap();

    let backend = SmtpBackend::new("127.0.0.1".to_string(), smtp_port).with_tls(false);

    // Send multiple emails in sequence
    for i in 1..=5 {
        let email = EmailMessage::new()
            .subject(format!("Test Email {}", i))
            .body(format!("Body of email {}", i))
            .from("sender@example.com")
            .to(vec!["recipient@example.com"])
            .build()
            .unwrap();

        let result = backend.send(&email).await;
        assert!(result.is_ok(), "Failed to send email {}", i);
    }
}

#[tokio::test]
async fn test_smtp_real_server_with_reply_to() {
    let container = start_mailpit().await;
    let smtp_port = container
        .get_host_port_ipv4(ContainerPort::Tcp(1025))
        .await
        .unwrap();

    let backend = SmtpBackend::new("127.0.0.1".to_string(), smtp_port).with_tls(false);

    let email = EmailMessage::new()
        .subject("Reply-To Test")
        .body("Please reply to the reply-to address")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .reply_to(vec!["replyto@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_smtp_real_server_multipart_alternative() {
    let container = start_mailpit().await;
    let smtp_port = container
        .get_host_port_ipv4(ContainerPort::Tcp(1025))
        .await
        .unwrap();

    let backend = SmtpBackend::new("127.0.0.1".to_string(), smtp_port).with_tls(false);

    let email = EmailMessage::new()
        .subject("Multipart Alternative Test")
        .body("Plain text version")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .attach_alternative(
            "<html><body><h1>HTML Version</h1></body></html>",
            "text/html",
        )
        .attach_alternative(
            "<amp-html><body>AMP Version</body></amp-html>",
            "text/x-amp-html",
        )
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_smtp_real_server_connection_failure() {
    // Use an invalid port to test connection failure
    let backend = SmtpBackend::new("127.0.0.1".to_string(), 19999).with_tls(false); // Port unlikely to be in use

    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_err(), "Expected connection failure");
}

#[tokio::test]
async fn test_smtp_real_server_idn_encoding() {
    let container = start_mailpit().await;
    let smtp_port = container
        .get_host_port_ipv4(ContainerPort::Tcp(1025))
        .await
        .unwrap();

    let backend = SmtpBackend::new("127.0.0.1".to_string(), smtp_port).with_tls(false);

    // Create email with IDN domain (will be encoded to Punycode)
    let email = EmailMessage::new()
        .subject("IDN Test")
        .body("Testing IDN encoding")
        .from("sender@example.com")
        .to(vec!["user@テスト.example"])
        .build()
        .unwrap();

    // Encode IDN addresses
    let encoded_email = email.encode_idn_addresses().unwrap();

    // Verify encoding happened
    assert!(encoded_email.to[0].contains("xn--"));

    let result = backend.send(&encoded_email).await;
    assert!(result.is_ok());
}
