//! Email Handler Integration Tests
//!
//! Integration tests for reinhardt-logging's email handler functionality
//! working with reinhardt-mail's email backend. These tests verify that
//! log records can be sent via email using various email backends.
//!
//! Tests for AdminEmailHandler that sends log records via email.
//! Based on Django's AdminEmailHandler tests.

use reinhardt_logging::{Handler, LogLevel, LogRecord, Logger};
use reinhardt_mail::{EmailBackend, EmailMessage, MemoryBackend};
use std::sync::Arc;

/// Admin email handler - sends error/critical log records via email
pub struct AdminEmailHandler {
    level: LogLevel,
    from_email: String,
    recipient_list: Vec<String>,
    backend: Arc<MemoryBackend>,
    fail_silently: bool,
    include_html: bool,
}

impl AdminEmailHandler {
    pub fn new(
        from_email: String,
        recipient_list: Vec<String>,
        backend: Arc<MemoryBackend>,
    ) -> Self {
        Self {
            level: LogLevel::Error,
            from_email,
            recipient_list,
            backend,
            fail_silently: false,
            include_html: false,
        }
    }

    pub fn with_fail_silently(mut self, fail_silently: bool) -> Self {
        self.fail_silently = fail_silently;
        self
    }

    pub fn with_include_html(mut self, include_html: bool) -> Self {
        self.include_html = include_html;
        self
    }

    fn format_subject(&self, record: &LogRecord) -> String {
        // Replace newlines with spaces in subject
        let base_subject = format!("[{}] {}", record.level.as_str(), record.logger_name);
        base_subject.replace('\n', " ").replace('\r', " ")
    }
}

#[async_trait::async_trait]
impl Handler for AdminEmailHandler {
    async fn handle(&self, record: &LogRecord) {
        let subject = self.format_subject(record);
        let body = format!(
            "Logger: {}\nLevel: {}\nTimestamp: {}\n\n{}",
            record.logger_name,
            record.level.as_str(),
            record.timestamp,
            record.message
        );

        let mut message_builder = EmailMessage::new()
            .subject(subject)
            .body(body.clone())
            .from(self.from_email.clone())
            .to(self.recipient_list.clone());

        if self.include_html {
            let html_body = format!(
                "<html><body><h2>Log Record</h2><p><strong>Logger:</strong> {}</p><p><strong>Level:</strong> {}</p><pre>{}</pre></body></html>",
                record.logger_name,
                record.level.as_str(),
                record.message
            );
            message_builder = message_builder.html(html_body);
        }

        match message_builder.build() {
            Ok(message) => {
                if let Err(e) = self.backend.send(&message).await {
                    if !self.fail_silently {
                        eprintln!("Failed to send email: {}", e);
                    }
                }
            }
            Err(e) => {
                if !self.fail_silently {
                    eprintln!("Failed to build email: {}", e);
                }
            }
        }
    }

    fn level(&self) -> LogLevel {
        self.level
    }

    fn set_level(&mut self, level: LogLevel) {
        self.level = level;
    }
}

#[tokio::test]
async fn test_logging_email_multiple_recipients() {
    // Handler should support multiple recipients
    let memory = Arc::new(MemoryBackend::new());

    let handler = AdminEmailHandler::new(
        "from@example.com".to_string(),
        vec![
            "admin1@example.com".to_string(),
            "admin2@example.com".to_string(),
            "admin3@example.com".to_string(),
        ],
        memory.clone(),
    );

    let logger = Logger::new("test.multi".to_string());
    logger.add_handler(Box::new(handler)).await;
    logger.set_level(LogLevel::Error).await;

    logger.error("Multiple recipients test".to_string()).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let messages = memory.get_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].to.len(), 3);
    assert!(messages[0].to.contains(&"admin1@example.com".to_string()));
    assert!(messages[0].to.contains(&"admin2@example.com".to_string()));
    assert!(messages[0].to.contains(&"admin3@example.com".to_string()));
}
