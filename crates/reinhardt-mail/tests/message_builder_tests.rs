//! EmailMessage Builder API integration tests
//!
//! Tests EmailMessage fluent API, covering builder pattern, validation,
//! headers, attachments, alternatives, encoding, and message construction.

use reinhardt_mail::{Alternative, Attachment, EmailMessage};
use rstest::rstest;

/// Test: Builder pattern basic construction
#[rstest]
fn test_builder_basic_construction() {
	let message = EmailMessage::builder()
		.from("sender@example.com")
		.to(vec!["recipient@example.com".to_string()])
		.subject("Test Subject")
		.body("Test Body")
		.build();

	assert_eq!(message.from_email, "sender@example.com");
	assert_eq!(message.to, vec!["recipient@example.com"]);
	assert_eq!(message.subject, "Test Subject");
	assert_eq!(message.body, "Test Body");
}

/// Test: Builder method chaining
#[rstest]
fn test_builder_method_chaining() {
	let message = EmailMessage::builder()
		.from("chain@example.com")
		.to(vec!["to@example.com".to_string()])
		.cc(vec!["cc@example.com".to_string()])
		.bcc(vec!["bcc@example.com".to_string()])
		.reply_to(vec!["reply@example.com".to_string()])
		.subject("Chained")
		.body("Body")
		.build();

	assert_eq!(message.from_email, "chain@example.com");
	assert_eq!(message.to, vec!["to@example.com"]);
	assert_eq!(message.cc, vec!["cc@example.com"]);
	assert_eq!(message.bcc, vec!["bcc@example.com"]);
	assert_eq!(message.reply_to, vec!["reply@example.com"]);
	assert_eq!(message.subject, "Chained");
	assert_eq!(message.body, "Body");
}

/// Test: Builder with HTML body
#[rstest]
fn test_builder_html_body() {
	let message = EmailMessage::builder()
		.from("html@example.com")
		.to(vec!["test@example.com".to_string()])
		.subject("HTML Email")
		.body("Plain text body")
		.html("<html><body><h1>HTML Body</h1></body></html>")
		.build();

	assert_eq!(message.body, "Plain text body");
	assert_eq!(
		message.html_body,
		Some("<html><body><h1>HTML Body</h1></body></html>".to_string())
	);
}

/// Test: Builder with multiple recipients
#[rstest]
fn test_builder_multiple_recipients() {
	let message = EmailMessage::builder()
		.from("sender@example.com")
		.to(vec![
			"to1@example.com".to_string(),
			"to2@example.com".to_string(),
			"to3@example.com".to_string(),
		])
		.cc(vec![
			"cc1@example.com".to_string(),
			"cc2@example.com".to_string(),
		])
		.subject("Multiple Recipients")
		.body("Test")
		.build();

	assert_eq!(message.to.len(), 3);
	assert_eq!(message.cc.len(), 2);
	assert!(message.to.contains(&"to1@example.com".to_string()));
	assert!(message.to.contains(&"to2@example.com".to_string()));
	assert!(message.to.contains(&"to3@example.com".to_string()));
}

/// Test: Builder with custom headers
#[rstest]
fn test_builder_custom_headers() {
	let message = EmailMessage::builder()
		.from("headers@example.com")
		.to(vec!["test@example.com".to_string()])
		.subject("Custom Headers")
		.body("Body")
		.header("X-Custom-Header", "CustomValue")
		.header("X-Priority", "1")
		.header("X-Mailer", "Reinhardt Mail")
		.build();

	assert_eq!(message.headers.len(), 3);
	assert!(
		message
			.headers
			.contains(&("X-Custom-Header".to_string(), "CustomValue".to_string()))
	);
	assert!(
		message
			.headers
			.contains(&("X-Priority".to_string(), "1".to_string()))
	);
	assert!(
		message
			.headers
			.contains(&("X-Mailer".to_string(), "Reinhardt Mail".to_string()))
	);
}

/// Test: Builder with single attachment
#[rstest]
fn test_builder_single_attachment() {
	let attachment = Attachment::new("document.txt", b"File content".to_vec());

	let message = EmailMessage::builder()
		.from("attach@example.com")
		.to(vec!["test@example.com".to_string()])
		.subject("Attachment Test")
		.body("Email with attachment")
		.attachment(attachment)
		.build();

	assert_eq!(message.attachments.len(), 1);
	assert_eq!(message.attachments[0].filename(), "document.txt");
}

/// Test: Builder with multiple attachments
#[rstest]
fn test_builder_multiple_attachments() {
	let attachment1 = Attachment::new("file1.txt", b"Content 1".to_vec());
	let attachment2 = Attachment::new("file2.pdf", b"Content 2".to_vec());
	let attachment3 = Attachment::new("image.png", b"Content 3".to_vec());

	let message = EmailMessage::builder()
		.from("multi@example.com")
		.to(vec!["test@example.com".to_string()])
		.subject("Multiple Attachments")
		.body("Email with 3 attachments")
		.attachment(attachment1)
		.attachment(attachment2)
		.attachment(attachment3)
		.build();

	assert_eq!(message.attachments.len(), 3);
	assert_eq!(message.attachments[0].filename(), "file1.txt");
	assert_eq!(message.attachments[1].filename(), "file2.pdf");
	assert_eq!(message.attachments[2].filename(), "image.png");
}

/// Test: Builder with alternative content
#[rstest]
fn test_builder_alternative_content() {
	let alt = Alternative::new("text/plain", "Alternative plain text".as_bytes().to_vec());

	let message = EmailMessage::builder()
		.from("alt@example.com")
		.to(vec!["test@example.com".to_string()])
		.subject("Alternative Content")
		.body("Main body")
		.alternative(alt)
		.build();

	assert_eq!(message.alternatives.len(), 1);
	assert_eq!(message.alternatives[0].content_type(), "text/plain");
}

/// Test: Builder default values
#[rstest]
fn test_builder_default_values() {
	let message = EmailMessage::builder().build();

	assert_eq!(message.subject, "");
	assert_eq!(message.body, "");
	assert_eq!(message.from_email, "");
	assert_eq!(message.to.len(), 0);
	assert_eq!(message.cc.len(), 0);
	assert_eq!(message.bcc.len(), 0);
	assert_eq!(message.reply_to.len(), 0);
	assert_eq!(message.html_body, None);
	assert_eq!(message.alternatives.len(), 0);
	assert_eq!(message.attachments.len(), 0);
	assert_eq!(message.headers.len(), 0);
}

/// Test: Builder UTF-8 content
#[rstest]
fn test_builder_utf8_content() {
	let message = EmailMessage::builder()
		.from("utf8@example.com")
		.to(vec!["test@example.com".to_string()])
		.subject("日本語の件名")
		.body("本文に日本語が含まれています。\nこれはテストメールです。")
		.build();

	assert_eq!(message.subject, "日本語の件名");
	assert!(message.body.contains("日本語"));
	assert!(message.body.contains("テストメール"));
}

/// Test: Builder from_email alias method
#[rstest]
fn test_builder_from_email_alias() {
	let message1 = EmailMessage::builder()
		.from("sender1@example.com")
		.to(vec!["test@example.com".to_string()])
		.subject("Test")
		.body("Body")
		.build();

	let message2 = EmailMessage::builder()
		.from_email("sender2@example.com")
		.to(vec!["test@example.com".to_string()])
		.subject("Test")
		.body("Body")
		.build();

	assert_eq!(message1.from_email, "sender1@example.com");
	assert_eq!(message2.from_email, "sender2@example.com");
}

/// Test: Builder Into<String> conversion
#[rstest]
fn test_builder_into_string_conversion() {
	let subject_owned = String::from("Owned String");
	let body_str = "String slice";

	let message = EmailMessage::builder()
		.from("convert@example.com")
		.to(vec!["test@example.com".to_string()])
		.subject(subject_owned)
		.body(body_str)
		.build();

	assert_eq!(message.subject, "Owned String");
	assert_eq!(message.body, "String slice");
}

/// Test: Builder large email construction
#[rstest]
fn test_builder_large_email() {
	let large_body = "Lorem ipsum dolor sit amet. ".repeat(1000);
	let large_html =
		"<html><body>".to_string() + &"<p>Paragraph</p>".repeat(1000) + "</body></html>";

	let message = EmailMessage::builder()
		.from("large@example.com")
		.to(vec!["test@example.com".to_string()])
		.subject("Large Email")
		.body(&large_body)
		.html(&large_html)
		.build();

	assert!(message.body.len() > 20000);
	assert!(message.html_body.unwrap().len() > 10000);
}

/// Test: Builder with empty collections
#[rstest]
fn test_builder_empty_collections() {
	let message = EmailMessage::builder()
		.from("empty@example.com")
		.to(vec![])
		.cc(vec![])
		.bcc(vec![])
		.reply_to(vec![])
		.subject("Empty Collections")
		.body("Test")
		.build();

	assert!(message.to.is_empty());
	assert!(message.cc.is_empty());
	assert!(message.bcc.is_empty());
	assert!(message.reply_to.is_empty());
}

/// Test: Builder header ordering
#[rstest]
fn test_builder_header_ordering() {
	let message = EmailMessage::builder()
		.from("order@example.com")
		.to(vec!["test@example.com".to_string()])
		.subject("Header Order")
		.body("Body")
		.header("X-First", "1")
		.header("X-Second", "2")
		.header("X-Third", "3")
		.build();

	assert_eq!(message.headers.len(), 3);
	assert_eq!(message.headers[0], ("X-First".to_string(), "1".to_string()));
	assert_eq!(
		message.headers[1],
		("X-Second".to_string(), "2".to_string())
	);
	assert_eq!(message.headers[2], ("X-Third".to_string(), "3".to_string()));
}

/// Test: Builder with long recipient list
#[rstest]
fn test_builder_long_recipient_list() {
	let recipients: Vec<String> = (1..=100)
		.map(|i| format!("user{}@example.com", i))
		.collect();

	let message = EmailMessage::builder()
		.from("sender@example.com")
		.to(recipients.clone())
		.subject("Long Recipient List")
		.body("Mass email")
		.build();

	assert_eq!(message.to.len(), 100);
	assert_eq!(message.to[0], "user1@example.com");
	assert_eq!(message.to[99], "user100@example.com");
}
