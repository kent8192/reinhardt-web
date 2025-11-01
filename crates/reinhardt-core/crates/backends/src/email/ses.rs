//! AWS SES email backend
//!
//! This backend sends emails through Amazon Simple Email Service (SES).
//!
//! # Examples
//!
//! ```no_run
//! # #[cfg(feature = "email-ses")]
//! use reinhardt_backends::email::{Email, EmailBackend, SesBackend};
//! # #[cfg(feature = "email-ses")]
//! use aws_config::BehaviorVersion;
//!
//! # #[cfg(feature = "email-ses")]
//! #[tokio::main]
//! async fn main() {
//!     let config = aws_config::defaults(BehaviorVersion::latest())
//!         .region("us-east-1")
//!         .load()
//!         .await;
//!
//!     let backend = SesBackend::new(&config);
//!
//!     let email = Email::builder()
//!         .from("sender@example.com")
//!         .to("recipient@example.com")
//!         .subject("Test")
//!         .text_body("Hello!")
//!         .build();
//!
//!     backend.send_email(&email).await.unwrap();
//! }
//! # #[cfg(not(feature = "email-ses"))]
//! # fn main() {}
//! ```

use crate::email::types::{Email, EmailBackend, EmailBody, EmailError, EmailResult};
use async_trait::async_trait;
use aws_sdk_ses::Client;
use aws_sdk_ses::types::{Body, Content, Destination, Message, RawMessage};
use aws_smithy_types::Blob;

/// AWS SES email backend
///
/// Sends emails through Amazon Simple Email Service.
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "email-ses")]
/// use reinhardt_backends::email::{Email, EmailBackend, SesBackend};
/// # #[cfg(feature = "email-ses")]
/// use aws_config::BehaviorVersion;
///
/// # #[cfg(feature = "email-ses")]
/// #[tokio::main]
/// async fn main() {
///     let config = aws_config::defaults(BehaviorVersion::latest()).load().await;
///     let backend = SesBackend::new(&config);
///
///     let email = Email::builder()
///         .from("sender@example.com")
///         .to("recipient@example.com")
///         .subject("Test")
///         .text_body("Hello!")
///         .build();
///
///     backend.send_email(&email).await.unwrap();
/// }
/// # #[cfg(not(feature = "email-ses"))]
/// # fn main() {}
/// ```
pub struct SesBackend {
	client: Client,
}

impl SesBackend {
	/// Create a new SES backend from AWS config
	///
	/// # Examples
	///
	/// ```no_run
	/// # #[cfg(feature = "email-ses")]
	/// use reinhardt_backends::email::SesBackend;
	/// # #[cfg(feature = "email-ses")]
	/// use aws_config::BehaviorVersion;
	///
	/// # #[cfg(feature = "email-ses")]
	/// #[tokio::main]
	/// async fn main() {
	///     let config = aws_config::defaults(BehaviorVersion::latest()).load().await;
	///     let backend = SesBackend::new(&config);
	/// }
	/// # #[cfg(not(feature = "email-ses"))]
	/// # fn main() {}
	/// ```
	pub fn new(config: &aws_config::SdkConfig) -> Self {
		let client = Client::new(config);
		Self { client }
	}

	/// Create a new SES backend with custom client
	///
	/// # Examples
	///
	/// ```no_run
	/// # #[cfg(feature = "email-ses")]
	/// use reinhardt_backends::email::SesBackend;
	/// # #[cfg(feature = "email-ses")]
	/// use aws_sdk_ses::Client;
	/// # #[cfg(feature = "email-ses")]
	/// use aws_config::BehaviorVersion;
	///
	/// # #[cfg(feature = "email-ses")]
	/// #[tokio::main]
	/// async fn main() {
	///     let config = aws_config::defaults(BehaviorVersion::latest()).load().await;
	///     let client = Client::new(&config);
	///     let backend = SesBackend::with_client(client);
	/// }
	/// # #[cfg(not(feature = "email-ses"))]
	/// # fn main() {}
	/// ```
	pub fn with_client(client: Client) -> Self {
		Self { client }
	}

	/// Send email using SES SendEmail API (simple email)
	async fn send_simple_email(&self, email: &Email) -> EmailResult<()> {
		// Build destination
		let mut destination = Destination::builder().set_to_addresses(Some(email.to.clone()));

		if let Some(cc_list) = &email.cc {
			destination = destination.set_cc_addresses(Some(cc_list.clone()));
		}

		if let Some(bcc_list) = &email.bcc {
			destination = destination.set_bcc_addresses(Some(bcc_list.clone()));
		}

		let destination = destination.build();

		// Build message body
		let body = match &email.body {
			EmailBody::Text(text) => Body::builder()
				.text(Content::builder().data(text).build().map_err(|e| {
					EmailError::Internal(format!("Failed to build text content: {}", e))
				})?)
				.build(),
			EmailBody::Html(html) => Body::builder()
				.html(Content::builder().data(html).build().map_err(|e| {
					EmailError::Internal(format!("Failed to build HTML content: {}", e))
				})?)
				.build(),
			EmailBody::Both { text, html } => Body::builder()
				.text(Content::builder().data(text).build().map_err(|e| {
					EmailError::Internal(format!("Failed to build text content: {}", e))
				})?)
				.html(Content::builder().data(html).build().map_err(|e| {
					EmailError::Internal(format!("Failed to build HTML content: {}", e))
				})?)
				.build(),
		};

		// Build message
		let message = Message::builder()
			.subject(
				Content::builder()
					.data(&email.subject)
					.build()
					.map_err(|e| EmailError::Internal(format!("Failed to build subject: {}", e)))?,
			)
			.body(body)
			.build();

		// Send email
		self.client
			.send_email()
			.source(&email.from)
			.destination(destination)
			.message(message)
			.send()
			.await
			.map_err(|e| EmailError::Send(format!("SES send_email failed: {}", e)))?;

		Ok(())
	}

	/// Build raw email message for attachments
	fn build_raw_message(&self, email: &Email) -> EmailResult<String> {
		use std::fmt::Write;

		let mut raw = String::new();

		// Headers
		writeln!(raw, "From: {}", email.from).unwrap();
		writeln!(raw, "To: {}", email.to.join(", ")).unwrap();

		if let Some(cc) = &email.cc {
			writeln!(raw, "Cc: {}", cc.join(", ")).unwrap();
		}

		if let Some(bcc) = &email.bcc {
			writeln!(raw, "Bcc: {}", bcc.join(", ")).unwrap();
		}

		writeln!(raw, "Subject: {}", email.subject).unwrap();
		writeln!(raw, "MIME-Version: 1.0").unwrap();

		let boundary = "----=_NextPart_000_0000_01234567.89ABCDEF";

		// Content-Type for multipart
		if !email.attachments.is_empty() {
			writeln!(
				raw,
				"Content-Type: multipart/mixed; boundary=\"{}\"",
				boundary
			)
			.unwrap();
			writeln!(raw).unwrap();
			writeln!(raw, "--{}", boundary).unwrap();
		}

		// Body
		match &email.body {
			EmailBody::Text(text) => {
				writeln!(raw, "Content-Type: text/plain; charset=utf-8").unwrap();
				writeln!(raw).unwrap();
				writeln!(raw, "{}", text).unwrap();
			}
			EmailBody::Html(html) => {
				writeln!(raw, "Content-Type: text/html; charset=utf-8").unwrap();
				writeln!(raw).unwrap();
				writeln!(raw, "{}", html).unwrap();
			}
			EmailBody::Both { text, html } => {
				let alt_boundary = "----=_NextPart_001_0001_01234567.89ABCDEF";
				writeln!(
					raw,
					"Content-Type: multipart/alternative; boundary=\"{}\"",
					alt_boundary
				)
				.unwrap();
				writeln!(raw).unwrap();

				writeln!(raw, "--{}", alt_boundary).unwrap();
				writeln!(raw, "Content-Type: text/plain; charset=utf-8").unwrap();
				writeln!(raw).unwrap();
				writeln!(raw, "{}", text).unwrap();

				writeln!(raw, "--{}", alt_boundary).unwrap();
				writeln!(raw, "Content-Type: text/html; charset=utf-8").unwrap();
				writeln!(raw).unwrap();
				writeln!(raw, "{}", html).unwrap();

				writeln!(raw, "--{}--", alt_boundary).unwrap();
			}
		}

		// Attachments
		if !email.attachments.is_empty() {
			for attachment in &email.attachments {
				writeln!(raw, "--{}", boundary).unwrap();
				writeln!(
					raw,
					"Content-Type: {}; name=\"{}\"",
					attachment.content_type, attachment.filename
				)
				.unwrap();
				writeln!(raw, "Content-Transfer-Encoding: base64").unwrap();
				writeln!(
					raw,
					"Content-Disposition: attachment; filename=\"{}\"",
					attachment.filename
				)
				.unwrap();
				writeln!(raw).unwrap();

				let encoded = base64::Engine::encode(
					&base64::engine::general_purpose::STANDARD,
					&attachment.content,
				);
				writeln!(raw, "{}", encoded).unwrap();
			}
			writeln!(raw, "--{}--", boundary).unwrap();
		}

		Ok(raw)
	}

	/// Send email using SES SendRawEmail API (with attachments)
	async fn send_raw_email(&self, email: &Email) -> EmailResult<()> {
		let raw_message = self.build_raw_message(email)?;

		let raw_message = RawMessage::builder()
			.data(Blob::new(raw_message.as_bytes()))
			.build()
			.map_err(|e| EmailError::Internal(format!("Failed to build raw message: {}", e)))?;

		self.client
			.send_raw_email()
			.raw_message(raw_message)
			.send()
			.await
			.map_err(|e| EmailError::Send(format!("SES send_raw_email failed: {}", e)))?;

		Ok(())
	}
}

#[async_trait]
impl EmailBackend for SesBackend {
	async fn send_email(&self, email: &Email) -> EmailResult<()> {
		// Validate email
		email.validate()?;

		// Use raw email if there are attachments
		if !email.attachments.is_empty() {
			self.send_raw_email(email).await
		} else {
			self.send_simple_email(email).await
		}
	}

	async fn send_bulk(&self, emails: &[Email]) -> EmailResult<Vec<EmailResult<()>>> {
		let mut results = Vec::with_capacity(emails.len());

		for email in emails {
			results.push(self.send_email(email).await);
		}

		Ok(results)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::email::types::Attachment;
	use aws_config::BehaviorVersion;

	fn create_test_config() -> aws_config::SdkConfig {
		aws_config::SdkConfig::builder()
			.behavior_version(BehaviorVersion::latest())
			.build()
	}

	#[test]
	fn test_ses_build_raw_message_text() {
		let config = create_test_config();
		let backend = SesBackend::new(&config);

		let email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.subject("Test")
			.text_body("Hello!")
			.build();

		let raw = backend.build_raw_message(&email).unwrap();
		assert!(raw.contains("From: sender@example.com"));
		assert!(raw.contains("To: recipient@example.com"));
		assert!(raw.contains("Subject: Test"));
		assert!(raw.contains("Hello!"));
	}

	#[test]
	fn test_ses_build_raw_message_html() {
		let config = create_test_config();
		let backend = SesBackend::new(&config);

		let email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.subject("Test")
			.html_body("<h1>Hello!</h1>")
			.build();

		let raw = backend.build_raw_message(&email).unwrap();
		assert!(raw.contains("Content-Type: text/html"));
		assert!(raw.contains("<h1>Hello!</h1>"));
	}

	#[test]
	fn test_ses_build_raw_message_both() {
		let config = create_test_config();
		let backend = SesBackend::new(&config);

		let email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.subject("Test")
			.both_body("Hello!", "<h1>Hello!</h1>")
			.build();

		let raw = backend.build_raw_message(&email).unwrap();
		assert!(raw.contains("multipart/alternative"));
		assert!(raw.contains("Hello!"));
		assert!(raw.contains("<h1>Hello!</h1>"));
	}

	#[test]
	fn test_ses_build_raw_message_with_attachment() {
		let config = create_test_config();
		let backend = SesBackend::new(&config);

		let attachment = Attachment::new("file.txt", "text/plain", b"Content".to_vec());

		let email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.subject("Test")
			.text_body("Body")
			.attachment(attachment)
			.build();

		let raw = backend.build_raw_message(&email).unwrap();
		assert!(raw.contains("multipart/mixed"));
		assert!(raw.contains("Content-Disposition: attachment"));
		assert!(raw.contains("filename=\"file.txt\""));
	}

	#[test]
	fn test_ses_build_raw_message_with_cc_bcc() {
		let config = create_test_config();
		let backend = SesBackend::new(&config);

		let email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.cc("cc@example.com")
			.bcc("bcc@example.com")
			.subject("Test")
			.text_body("Body")
			.build();

		let raw = backend.build_raw_message(&email).unwrap();
		assert!(raw.contains("Cc: cc@example.com"));
		assert!(raw.contains("Bcc: bcc@example.com"));
	}
}
