//! Core types for email backends

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Email backend errors
#[derive(Debug, Error)]
pub enum EmailError {
	/// Email validation error
	#[error("Email validation error: {0}")]
	Validation(String),

	/// Connection error
	#[error("Connection error: {0}")]
	Connection(String),

	/// Authentication error
	#[error("Authentication error: {0}")]
	Authentication(String),

	/// Sending error
	#[error("Failed to send email: {0}")]
	Send(String),

	/// Serialization error
	#[error("Serialization error: {0}")]
	Serialization(String),

	/// API error
	#[error("API error: {0}")]
	Api(String),

	/// Configuration error
	#[error("Configuration error: {0}")]
	Configuration(String),

	/// Internal error
	#[error("Internal error: {0}")]
	Internal(String),
}

/// Result type for email operations
pub type EmailResult<T> = Result<T, EmailError>;

/// Email body content
///
/// Supports plain text, HTML, or both formats.
///
/// # Examples
///
/// ```
/// use reinhardt_backends::email::EmailBody;
///
/// // Plain text email
/// let text = EmailBody::Text("Hello, World!".to_string());
///
/// // HTML email
/// let html = EmailBody::Html("<h1>Hello, World!</h1>".to_string());
///
/// // Both formats (recommended for best compatibility)
/// let both = EmailBody::Both {
///     text: "Hello, World!".to_string(),
///     html: "<h1>Hello, World!</h1>".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmailBody {
	/// Plain text content
	Text(String),
	/// HTML content
	Html(String),
	/// Both plain text and HTML (recommended)
	Both {
		/// Plain text version
		text: String,
		/// HTML version
		html: String,
	},
}

impl EmailBody {
	/// Get the plain text content
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_backends::email::EmailBody;
	///
	/// let body = EmailBody::Text("Hello".to_string());
	/// assert_eq!(body.text(), Some("Hello"));
	///
	/// let body = EmailBody::Both {
	///     text: "Hello".to_string(),
	///     html: "<h1>Hello</h1>".to_string(),
	/// };
	/// assert_eq!(body.text(), Some("Hello"));
	/// ```
	pub fn text(&self) -> Option<&str> {
		match self {
			EmailBody::Text(t) => Some(t),
			EmailBody::Both { text, .. } => Some(text),
			EmailBody::Html(_) => None,
		}
	}

	/// Get the HTML content
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_backends::email::EmailBody;
	///
	/// let body = EmailBody::Html("<h1>Hello</h1>".to_string());
	/// assert_eq!(body.html(), Some("<h1>Hello</h1>"));
	///
	/// let body = EmailBody::Both {
	///     text: "Hello".to_string(),
	///     html: "<h1>Hello</h1>".to_string(),
	/// };
	/// assert_eq!(body.html(), Some("<h1>Hello</h1>"));
	/// ```
	pub fn html(&self) -> Option<&str> {
		match self {
			EmailBody::Html(h) => Some(h),
			EmailBody::Both { html, .. } => Some(html),
			EmailBody::Text(_) => None,
		}
	}
}

/// Email attachment
///
/// Represents a file attachment to be sent with an email.
///
/// # Examples
///
/// ```
/// use reinhardt_backends::email::Attachment;
///
/// let attachment = Attachment {
///     filename: "document.pdf".to_string(),
///     content_type: "application/pdf".to_string(),
///     content: vec![0x25, 0x50, 0x44, 0x46], // PDF header
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
	/// Filename to display
	pub filename: String,
	/// MIME content type (e.g., "application/pdf", "image/png")
	pub content_type: String,
	/// Binary content of the file
	pub content: Vec<u8>,
}

impl Attachment {
	/// Create a new attachment
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_backends::email::Attachment;
	///
	/// let attachment = Attachment::new(
	///     "document.pdf",
	///     "application/pdf",
	///     vec![0x25, 0x50, 0x44, 0x46],
	/// );
	/// assert_eq!(attachment.filename, "document.pdf");
	/// ```
	pub fn new(
		filename: impl Into<String>,
		content_type: impl Into<String>,
		content: Vec<u8>,
	) -> Self {
		Self {
			filename: filename.into(),
			content_type: content_type.into(),
			content,
		}
	}
}

/// Email message structure
///
/// Represents a complete email message with all components.
///
/// # Examples
///
/// ```
/// use reinhardt_backends::email::{Email, EmailBody, Attachment};
///
/// let email = Email {
///     from: "sender@example.com".to_string(),
///     to: vec!["recipient@example.com".to_string()],
///     cc: Some(vec!["cc@example.com".to_string()]),
///     bcc: Some(vec!["bcc@example.com".to_string()]),
///     subject: "Important Message".to_string(),
///     body: EmailBody::Both {
///         text: "Hello, World!".to_string(),
///         html: "<h1>Hello, World!</h1>".to_string(),
///     },
///     attachments: vec![],
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
	/// Sender email address
	pub from: String,
	/// List of recipient email addresses
	pub to: Vec<String>,
	/// Optional CC recipients
	pub cc: Option<Vec<String>>,
	/// Optional BCC recipients
	pub bcc: Option<Vec<String>>,
	/// Email subject line
	pub subject: String,
	/// Email body content
	pub body: EmailBody,
	/// File attachments
	pub attachments: Vec<Attachment>,
}

impl Email {
	/// Create a new email builder
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_backends::email::{Email, EmailBody};
	///
	/// let email = Email::builder()
	///     .from("sender@example.com")
	///     .to("recipient@example.com")
	///     .subject("Hello")
	///     .text_body("Hello, World!")
	///     .build();
	///
	/// assert_eq!(email.from, "sender@example.com");
	/// assert_eq!(email.to, vec!["recipient@example.com"]);
	/// ```
	pub fn builder() -> EmailBuilder {
		EmailBuilder::default()
	}

	/// Validate email addresses
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_backends::email::{Email, EmailBody};
	///
	/// let email = Email {
	///     from: "sender@example.com".to_string(),
	///     to: vec!["recipient@example.com".to_string()],
	///     cc: None,
	///     bcc: None,
	///     subject: "Test".to_string(),
	///     body: EmailBody::Text("Test".to_string()),
	///     attachments: vec![],
	/// };
	///
	/// assert!(email.validate().is_ok());
	/// ```
	pub fn validate(&self) -> EmailResult<()> {
		if self.from.is_empty() {
			return Err(EmailError::Validation(
				"From address is required".to_string(),
			));
		}
		if self.to.is_empty() {
			return Err(EmailError::Validation(
				"At least one recipient is required".to_string(),
			));
		}
		if self.subject.is_empty() {
			return Err(EmailError::Validation("Subject is required".to_string()));
		}
		Ok(())
	}
}

impl fmt::Display for Email {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"Email from {} to {} - Subject: {}",
			self.from,
			self.to.join(", "),
			self.subject
		)
	}
}

/// Email builder for fluent construction
///
/// # Examples
///
/// ```
/// use reinhardt_backends::email::Email;
///
/// let email = Email::builder()
///     .from("sender@example.com")
///     .to("recipient@example.com")
///     .subject("Hello")
///     .text_body("Hello, World!")
///     .build();
/// ```
#[derive(Default)]
pub struct EmailBuilder {
	from: Option<String>,
	to: Vec<String>,
	cc: Vec<String>,
	bcc: Vec<String>,
	subject: Option<String>,
	body: Option<EmailBody>,
	attachments: Vec<Attachment>,
}

impl EmailBuilder {
	/// Set the sender address
	pub fn from(mut self, from: impl Into<String>) -> Self {
		self.from = Some(from.into());
		self
	}

	/// Add a recipient
	pub fn to(mut self, to: impl Into<String>) -> Self {
		self.to.push(to.into());
		self
	}

	/// Add multiple recipients
	pub fn to_list(mut self, to: Vec<String>) -> Self {
		self.to.extend(to);
		self
	}

	/// Add a CC recipient
	pub fn cc(mut self, cc: impl Into<String>) -> Self {
		self.cc.push(cc.into());
		self
	}

	/// Add a BCC recipient
	pub fn bcc(mut self, bcc: impl Into<String>) -> Self {
		self.bcc.push(bcc.into());
		self
	}

	/// Set the subject
	pub fn subject(mut self, subject: impl Into<String>) -> Self {
		self.subject = Some(subject.into());
		self
	}

	/// Set plain text body
	pub fn text_body(mut self, text: impl Into<String>) -> Self {
		self.body = Some(EmailBody::Text(text.into()));
		self
	}

	/// Set HTML body
	pub fn html_body(mut self, html: impl Into<String>) -> Self {
		self.body = Some(EmailBody::Html(html.into()));
		self
	}

	/// Set both text and HTML body
	pub fn both_body(mut self, text: impl Into<String>, html: impl Into<String>) -> Self {
		self.body = Some(EmailBody::Both {
			text: text.into(),
			html: html.into(),
		});
		self
	}

	/// Add an attachment
	pub fn attachment(mut self, attachment: Attachment) -> Self {
		self.attachments.push(attachment);
		self
	}

	/// Build the email
	pub fn build(self) -> Email {
		Email {
			from: self.from.unwrap_or_default(),
			to: self.to,
			cc: if self.cc.is_empty() {
				None
			} else {
				Some(self.cc)
			},
			bcc: if self.bcc.is_empty() {
				None
			} else {
				Some(self.bcc)
			},
			subject: self.subject.unwrap_or_default(),
			body: self.body.unwrap_or(EmailBody::Text(String::new())),
			attachments: self.attachments,
		}
	}
}

/// Email backend trait
///
/// This trait defines the interface for all email backends.
/// Implementations must handle sending individual emails and bulk emails.
///
/// # Examples
///
/// ```
/// use reinhardt_backends::email::{Email, EmailBackend, EmailBody};
/// # use reinhardt_backends::email::MemoryEmailBackend;
/// # use async_trait::async_trait;
///
/// #[tokio::main]
/// async fn main() {
///     # let backend = MemoryEmailBackend::new();
///     let email = Email::builder()
///         .from("sender@example.com")
///         .to("recipient@example.com")
///         .subject("Test")
///         .text_body("Hello!")
///         .build();
///
///     backend.send_email(&email).await.unwrap();
/// }
/// ```
#[async_trait]
pub trait EmailBackend: Send + Sync {
	/// Send a single email
	///
	/// # Arguments
	///
	/// * `email` - The email to send
	///
	/// # Errors
	///
	/// Returns an error if the email cannot be sent.
	async fn send_email(&self, email: &Email) -> EmailResult<()>;

	/// Send multiple emails in bulk
	///
	/// # Arguments
	///
	/// * `emails` - List of emails to send
	///
	/// # Returns
	///
	/// A vector of results, one for each email. Failed emails will have an error result,
	/// but the operation continues for remaining emails.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_backends::email::{Email, EmailBackend, EmailBody};
	/// # use reinhardt_backends::email::MemoryEmailBackend;
	///
	/// #[tokio::main]
	/// async fn main() {
	///     # let backend = MemoryEmailBackend::new();
	///     let emails = vec![
	///         Email::builder()
	///             .from("sender@example.com")
	///             .to("user1@example.com")
	///             .subject("Notification")
	///             .text_body("Message 1")
	///             .build(),
	///         Email::builder()
	///             .from("sender@example.com")
	///             .to("user2@example.com")
	///             .subject("Notification")
	///             .text_body("Message 2")
	///             .build(),
	///     ];
	///
	///     let results = backend.send_bulk(&emails).await.unwrap();
	///     assert_eq!(results.len(), 2);
	/// }
	/// ```
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

	#[test]
	fn test_email_body_text() {
		let body = EmailBody::Text("Hello".to_string());
		assert_eq!(body.text(), Some("Hello"));
		assert_eq!(body.html(), None);
	}

	#[test]
	fn test_email_body_html() {
		let body = EmailBody::Html("<h1>Hello</h1>".to_string());
		assert_eq!(body.text(), None);
		assert_eq!(body.html(), Some("<h1>Hello</h1>"));
	}

	#[test]
	fn test_email_body_both() {
		let body = EmailBody::Both {
			text: "Hello".to_string(),
			html: "<h1>Hello</h1>".to_string(),
		};
		assert_eq!(body.text(), Some("Hello"));
		assert_eq!(body.html(), Some("<h1>Hello</h1>"));
	}

	#[test]
	fn test_attachment_new() {
		let attachment = Attachment::new("file.txt", "text/plain", vec![1, 2, 3]);
		assert_eq!(attachment.filename, "file.txt");
		assert_eq!(attachment.content_type, "text/plain");
		assert_eq!(attachment.content, vec![1, 2, 3]);
	}

	#[test]
	fn test_email_builder() {
		let email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.cc("cc@example.com")
			.bcc("bcc@example.com")
			.subject("Test Subject")
			.text_body("Test Body")
			.build();

		assert_eq!(email.from, "sender@example.com");
		assert_eq!(email.to, vec!["recipient@example.com"]);
		assert_eq!(email.cc, Some(vec!["cc@example.com".to_string()]));
		assert_eq!(email.bcc, Some(vec!["bcc@example.com".to_string()]));
		assert_eq!(email.subject, "Test Subject");
		assert_eq!(email.body.text(), Some("Test Body"));
	}

	#[test]
	fn test_email_validation() {
		let valid_email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.subject("Test")
			.text_body("Body")
			.build();
		assert!(valid_email.validate().is_ok());

		let invalid_email = Email::builder()
			.from("sender@example.com")
			.subject("Test")
			.text_body("Body")
			.build();
		assert!(invalid_email.validate().is_err());
	}

	#[test]
	fn test_email_display() {
		let email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.subject("Test Subject")
			.text_body("Body")
			.build();

		let display = format!("{}", email);
		assert!(display.contains("sender@example.com"));
		assert!(display.contains("recipient@example.com"));
		assert!(display.contains("Test Subject"));
	}
}
