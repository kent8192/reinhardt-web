//! SMTP backend tests with mock SMTP server
//! Based on Django's django/tests/mail/tests.py SMTPBackendTests

use reinhardt_mail::{EmailBackend, EmailMessage, SmtpBackend};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

/// Simple mock SMTP server for testing
struct MockSmtpServer {
    port: u16,
    messages: Arc<Mutex<Vec<String>>>,
    auth_required: bool,
    use_tls: bool,
    should_fail: bool,
}

impl MockSmtpServer {
    async fn new(auth_required: bool, use_tls: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let messages = Arc::new(Mutex::new(Vec::new()));

        let messages_clone = messages.clone();
        let auth_req = auth_required;

        tokio::spawn(async move {
            Self::run_server(listener, messages_clone, auth_req).await;
        });

        // Give server time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Self {
            port,
            messages,
            auth_required,
            use_tls,
            should_fail: false,
        }
    }

    async fn run_server(
        listener: TcpListener,
        messages: Arc<Mutex<Vec<String>>>,
        auth_required: bool,
    ) {
        while let Ok((mut socket, _)) = listener.accept().await {
            let messages = messages.clone();

            tokio::spawn(async move {
                let (reader, mut writer) = socket.split();
                let mut reader = BufReader::new(reader);
                let mut line = String::new();

                // Send greeting
                writer.write_all(b"220 localhost SMTP Mock\r\n").await.ok();

                let mut authenticated = !auth_required;
                let mut mail_from = String::new();
                let mut rcpt_to = Vec::new();
                let mut data_mode = false;
                let mut email_data = String::new();

                loop {
                    line.clear();
                    if reader.read_line(&mut line).await.is_err() {
                        break;
                    }

                    if line.is_empty() {
                        break;
                    }

                    let cmd = line.trim();

                    if data_mode {
                        if cmd == "." {
                            data_mode = false;
                            messages.lock().await.push(email_data.clone());
                            email_data.clear();
                            writer.write_all(b"250 OK\r\n").await.ok();
                        } else {
                            email_data.push_str(cmd);
                            email_data.push('\n');
                        }
                        continue;
                    }

                    if cmd.starts_with("EHLO") || cmd.starts_with("HELO") {
                        writer.write_all(b"250-localhost\r\n").await.ok();
                        if auth_required {
                            writer.write_all(b"250 AUTH PLAIN LOGIN\r\n").await.ok();
                        } else {
                            writer.write_all(b"250 OK\r\n").await.ok();
                        }
                    } else if cmd.starts_with("AUTH") {
                        authenticated = true;
                        writer
                            .write_all(b"235 Authentication successful\r\n")
                            .await
                            .ok();
                    } else if cmd.starts_with("MAIL FROM:") {
                        if auth_required && !authenticated {
                            writer
                                .write_all(b"530 Authentication required\r\n")
                                .await
                                .ok();
                        } else {
                            mail_from = cmd[10..].trim().to_string();
                            writer.write_all(b"250 OK\r\n").await.ok();
                        }
                    } else if cmd.starts_with("RCPT TO:") {
                        rcpt_to.push(cmd[8..].trim().to_string());
                        writer.write_all(b"250 OK\r\n").await.ok();
                    } else if cmd == "DATA" {
                        data_mode = true;
                        writer.write_all(b"354 Start mail input\r\n").await.ok();
                    } else if cmd == "QUIT" {
                        writer.write_all(b"221 Bye\r\n").await.ok();
                        break;
                    } else {
                        writer.write_all(b"250 OK\r\n").await.ok();
                    }
                }
            });
        }
    }

    fn get_port(&self) -> u16 {
        self.port
    }

    async fn get_messages(&self) -> Vec<String> {
        self.messages.lock().await.clone()
    }
}

#[tokio::test]
async fn test_smtp_send_basic() {
    let server = MockSmtpServer::new(false, false).await;

    let backend = SmtpBackend::new("127.0.0.1".to_string(), server.get_port()).with_tls(false);

    let email = EmailMessage::new()
        .subject("Test Subject")
        .body("Test body")
        .from("sender@example.com")
        .to(vec!["recipient@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok(), "Failed to send email: {:?}", result.err());

    // Give server time to process
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let messages = server.get_messages().await;
    assert!(messages.len() > 0, "No messages received by server");
}

#[tokio::test]
async fn test_smtp_authentication_disabled() {
    // Test that SMTP works without authentication
    let server = MockSmtpServer::new(false, false).await;

    let backend = SmtpBackend::new("127.0.0.1".to_string(), server.get_port()).with_tls(false);

    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_smtp_with_credentials() {
    // Test SMTP with authentication
    let server = MockSmtpServer::new(true, false).await;

    let backend = SmtpBackend::new("127.0.0.1".to_string(), server.get_port())
        .with_credentials("user".to_string(), "pass".to_string())
        .with_tls(false);

    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_smtp_multiple_recipients() {
    let server = MockSmtpServer::new(false, false).await;

    let backend = SmtpBackend::new("127.0.0.1".to_string(), server.get_port()).with_tls(false);

    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["to1@example.com", "to2@example.com"])
        .cc(vec!["cc@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok());

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let messages = server.get_messages().await;
    assert!(messages.len() > 0);
}

#[tokio::test]
async fn test_smtp_with_html() {
    let server = MockSmtpServer::new(false, false).await;

    let backend = SmtpBackend::new("127.0.0.1".to_string(), server.get_port()).with_tls(false);

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
async fn test_smtp_unicode_subject() {
    let server = MockSmtpServer::new(false, false).await;

    let backend = SmtpBackend::new("127.0.0.1".to_string(), server.get_port()).with_tls(false);

    let email = EmailMessage::new()
        .subject("Test 日本語 Subject")
        .body("Body with unicode: 你好")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    assert!(result.is_ok());
}

// Note: TLS/SSL tests require more complex setup and are not included in this basic implementation
// These would typically use testcontainers or a more sophisticated mock

#[tokio::test]
async fn test_smtp_connection_error_handling() {
    // Test connection to invalid port
    let backend = SmtpBackend::new("127.0.0.1".to_string(), 19999).with_tls(false);

    let email = EmailMessage::new()
        .subject("Test")
        .body("Body")
        .from("from@example.com")
        .to(vec!["to@example.com"])
        .build()
        .unwrap();

    let result = backend.send(&email).await;
    // Should fail to connect
    assert!(result.is_err());
}

#[tokio::test]
async fn test_smtp_send_multiple_messages() {
    let server = MockSmtpServer::new(false, false).await;

    let backend = SmtpBackend::new("127.0.0.1".to_string(), server.get_port()).with_tls(false);

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

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let messages = server.get_messages().await;
    assert!(messages.len() >= 1);
}

#[tokio::test]
async fn test_smtp_idn_encoding() {
    // Test that IDN domains are properly encoded for SMTP
    let server = MockSmtpServer::new(false, false).await;

    let backend = SmtpBackend::new("127.0.0.1".to_string(), server.get_port()).with_tls(false);

    let email = EmailMessage::new()
        .subject("Test IDN")
        .body("Body")
        .from("test@münchen.de")
        .to(vec!["user@münchen.de"])
        .build()
        .unwrap();

    // Encode IDN addresses before sending
    let encoded_email = email.encode_idn_addresses().unwrap();

    let result = backend.send(&encoded_email).await;
    assert!(result.is_ok());
}
