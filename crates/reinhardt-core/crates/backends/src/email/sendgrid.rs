//! SendGrid email backend
//!
//! This backend sends emails through the SendGrid API.
//!
//! # Examples
//!
//! ```no_run
//! # #[cfg(feature = "email-sendgrid")]
//! use reinhardt_backends::email::{Email, EmailBackend, SendGridBackend};
//!
//! # #[cfg(feature = "email-sendgrid")]
//! #[tokio::main]
//! async fn main() {
//!     let backend = SendGridBackend::new("your-api-key".to_string());
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
//! # #[cfg(not(feature = "email-sendgrid"))]
//! # fn main() {}
//! ```

use crate::email::types::{Email, EmailBackend, EmailBody, EmailError, EmailResult};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const SENDGRID_API_URL: &str = "https://api.sendgrid.com/v3/mail/send";

/// SendGrid email content
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SendGridContent {
	#[serde(rename = "type")]
	content_type: String,
	value: String,
}

/// SendGrid email address
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SendGridEmail {
	email: String,
}

/// SendGrid personalization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SendGridPersonalization {
	to: Vec<SendGridEmail>,
	#[serde(skip_serializing_if = "Option::is_none")]
	cc: Option<Vec<SendGridEmail>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	bcc: Option<Vec<SendGridEmail>>,
	subject: String,
}

/// SendGrid attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SendGridAttachment {
	content: String,
	#[serde(rename = "type")]
	content_type: String,
	filename: String,
}

/// SendGrid API request
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SendGridRequest {
	personalizations: Vec<SendGridPersonalization>,
	from: SendGridEmail,
	subject: String,
	content: Vec<SendGridContent>,
	#[serde(skip_serializing_if = "Option::is_none")]
	attachments: Option<Vec<SendGridAttachment>>,
}

/// SendGrid email backend
///
/// Sends emails through the SendGrid API.
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "email-sendgrid")]
/// use reinhardt_backends::email::{Email, EmailBackend, SendGridBackend};
///
/// # #[cfg(feature = "email-sendgrid")]
/// #[tokio::main]
/// async fn main() {
///     let backend = SendGridBackend::new("your-api-key".to_string());
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
/// # #[cfg(not(feature = "email-sendgrid"))]
/// # fn main() {}
/// ```
pub struct SendGridBackend {
	api_key: String,
	client: Client,
}

impl SendGridBackend {
	/// Create a new SendGrid backend
	///
	/// # Examples
	///
	/// ```no_run
	/// # #[cfg(feature = "email-sendgrid")]
	/// use reinhardt_backends::email::SendGridBackend;
	///
	/// # #[cfg(feature = "email-sendgrid")]
	/// let backend = SendGridBackend::new("your-api-key".to_string());
	/// ```
	pub fn new(api_key: String) -> Self {
		let client = Client::builder()
			.timeout(Duration::from_secs(30))
			.build()
			.expect("Failed to create HTTP client");

		Self { api_key, client }
	}

	/// Create a SendGrid backend with custom client
	///
	/// # Examples
	///
	/// ```no_run
	/// # #[cfg(feature = "email-sendgrid")]
	/// use reinhardt_backends::email::SendGridBackend;
	/// use reqwest::Client;
	///
	/// # #[cfg(feature = "email-sendgrid")]
	/// let client = Client::new();
	/// # #[cfg(feature = "email-sendgrid")]
	/// let backend = SendGridBackend::with_client("your-api-key".to_string(), client);
	/// ```
	pub fn with_client(api_key: String, client: Client) -> Self {
		Self { api_key, client }
	}

	fn build_request(&self, email: &Email) -> EmailResult<SendGridRequest> {
		// Build content
		let content = match &email.body {
			EmailBody::Text(text) => vec![SendGridContent {
				content_type: "text/plain".to_string(),
				value: text.clone(),
			}],
			EmailBody::Html(html) => vec![SendGridContent {
				content_type: "text/html".to_string(),
				value: html.clone(),
			}],
			EmailBody::Both { text, html } => vec![
				SendGridContent {
					content_type: "text/plain".to_string(),
					value: text.clone(),
				},
				SendGridContent {
					content_type: "text/html".to_string(),
					value: html.clone(),
				},
			],
		};

		// Build personalizations
		let to = email
			.to
			.iter()
			.map(|e| SendGridEmail { email: e.clone() })
			.collect();

		let cc = email.cc.as_ref().map(|cc_list| {
			cc_list
				.iter()
				.map(|e| SendGridEmail { email: e.clone() })
				.collect()
		});

		let bcc = email.bcc.as_ref().map(|bcc_list| {
			bcc_list
				.iter()
				.map(|e| SendGridEmail { email: e.clone() })
				.collect()
		});

		let personalization = SendGridPersonalization {
			to,
			cc,
			bcc,
			subject: email.subject.clone(),
		};

		// Build attachments
		let attachments = if email.attachments.is_empty() {
			None
		} else {
			Some(
				email
					.attachments
					.iter()
					.map(|att| SendGridAttachment {
						content: base64::Engine::encode(
							&base64::engine::general_purpose::STANDARD,
							&att.content,
						),
						content_type: att.content_type.clone(),
						filename: att.filename.clone(),
					})
					.collect(),
			)
		};

		Ok(SendGridRequest {
			personalizations: vec![personalization],
			from: SendGridEmail {
				email: email.from.clone(),
			},
			subject: email.subject.clone(),
			content,
			attachments,
		})
	}
}

#[async_trait]
impl EmailBackend for SendGridBackend {
	async fn send_email(&self, email: &Email) -> EmailResult<()> {
		// Validate email
		email.validate()?;

		// Build request
		let request = self.build_request(email)?;

		// Send request
		let response = self
			.client
			.post(SENDGRID_API_URL)
			.header("Authorization", format!("Bearer {}", self.api_key))
			.header("Content-Type", "application/json")
			.json(&request)
			.send()
			.await
			.map_err(|e| EmailError::Send(format!("SendGrid API request failed: {}", e)))?;

		// Check response status
		if !response.status().is_success() {
			let status = response.status();
			let body = response
				.text()
				.await
				.unwrap_or_else(|_| "Unknown error".to_string());
			return Err(EmailError::Api(format!(
				"SendGrid API error ({}): {}",
				status, body
			)));
		}

		Ok(())
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

	#[test]
	fn test_sendgrid_backend_new() {
		let backend = SendGridBackend::new("test-api-key".to_string());
		assert_eq!(backend.api_key, "test-api-key");
	}

	#[test]
	fn test_sendgrid_build_request_text() {
		let backend = SendGridBackend::new("test-key".to_string());

		let email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.subject("Test")
			.text_body("Hello!")
			.build();

		let request = backend.build_request(&email).unwrap();
		assert_eq!(request.from.email, "sender@example.com");
		assert_eq!(
			request.personalizations[0].to[0].email,
			"recipient@example.com"
		);
		assert_eq!(request.content[0].content_type, "text/plain");
		assert_eq!(request.content[0].value, "Hello!");
	}

	#[test]
	fn test_sendgrid_build_request_html() {
		let backend = SendGridBackend::new("test-key".to_string());

		let email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.subject("Test")
			.html_body("<h1>Hello!</h1>")
			.build();

		let request = backend.build_request(&email).unwrap();
		assert_eq!(request.content[0].content_type, "text/html");
		assert_eq!(request.content[0].value, "<h1>Hello!</h1>");
	}

	#[test]
	fn test_sendgrid_build_request_both() {
		let backend = SendGridBackend::new("test-key".to_string());

		let email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.subject("Test")
			.both_body("Hello!", "<h1>Hello!</h1>")
			.build();

		let request = backend.build_request(&email).unwrap();
		assert_eq!(request.content.len(), 2);
		assert_eq!(request.content[0].content_type, "text/plain");
		assert_eq!(request.content[1].content_type, "text/html");
	}

	#[test]
	fn test_sendgrid_build_request_with_cc_bcc() {
		let backend = SendGridBackend::new("test-key".to_string());

		let email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.cc("cc@example.com")
			.bcc("bcc@example.com")
			.subject("Test")
			.text_body("Body")
			.build();

		let request = backend.build_request(&email).unwrap();
		assert!(request.personalizations[0].cc.is_some());
		assert!(request.personalizations[0].bcc.is_some());
	}

	#[test]
	fn test_sendgrid_build_request_with_attachments() {
		let backend = SendGridBackend::new("test-key".to_string());

		let attachment = Attachment::new("file.txt", "text/plain", b"Hello".to_vec());

		let email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.subject("Test")
			.text_body("Body")
			.attachment(attachment)
			.build();

		let request = backend.build_request(&email).unwrap();
		assert!(request.attachments.is_some());
		let attachments = request.attachments.unwrap();
		assert_eq!(attachments.len(), 1);
		assert_eq!(attachments[0].filename, "file.txt");
	}
}
