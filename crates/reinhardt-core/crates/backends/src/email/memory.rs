//! In-memory email backend for testing
//!
//! This backend stores emails in memory without actually sending them.
//! Useful for testing and development purposes.
//!
//! # Examples
//!
//! ```
//! use reinhardt_backends::email::{Email, EmailBackend, MemoryEmailBackend};
//!
//! #[tokio::main]
//! async fn main() {
//!     let backend = MemoryEmailBackend::new();
//!
//!     let email = Email::builder()
//!         .from("sender@example.com")
//!         .to("recipient@example.com")
//!         .subject("Test")
//!         .text_body("Hello!")
//!         .build();
//!
//!     backend.send_email(&email).await.unwrap();
//!
//!     // Check sent emails
//!     let sent = backend.sent_emails();
//!     assert_eq!(sent.len(), 1);
//!     assert_eq!(sent[0].subject, "Test");
//! }
//! ```

use crate::email::types::{Email, EmailBackend, EmailError, EmailResult};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

/// In-memory email backend
///
/// Stores emails in memory for testing purposes. Does not actually send emails.
///
/// # Examples
///
/// ```
/// use reinhardt_backends::email::{Email, EmailBackend, MemoryEmailBackend};
///
/// #[tokio::main]
/// async fn main() {
///     let backend = MemoryEmailBackend::new();
///
///     let email = Email::builder()
///         .from("sender@example.com")
///         .to("recipient@example.com")
///         .subject("Test")
///         .text_body("Hello!")
///         .build();
///
///     backend.send_email(&email).await.unwrap();
///     assert_eq!(backend.sent_emails().len(), 1);
/// }
/// ```
#[derive(Clone, Default)]
pub struct MemoryEmailBackend {
	/// Stored emails
	emails: Arc<RwLock<Vec<Email>>>,
}

impl MemoryEmailBackend {
	/// Create a new memory email backend
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_backends::email::MemoryEmailBackend;
	///
	/// let backend = MemoryEmailBackend::new();
	/// ```
	pub fn new() -> Self {
		Self {
			emails: Arc::new(RwLock::new(Vec::new())),
		}
	}

	/// Get all sent emails
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_backends::email::{Email, EmailBackend, MemoryEmailBackend};
	///
	/// #[tokio::main]
	/// async fn main() {
	///     let backend = MemoryEmailBackend::new();
	///
	///     let email = Email::builder()
	///         .from("sender@example.com")
	///         .to("recipient@example.com")
	///         .subject("Test")
	///         .text_body("Body")
	///         .build();
	///
	///     backend.send_email(&email).await.unwrap();
	///
	///     let sent = backend.sent_emails();
	///     assert_eq!(sent.len(), 1);
	///     assert_eq!(sent[0].from, "sender@example.com");
	/// }
	/// ```
	pub fn sent_emails(&self) -> Vec<Email> {
		self.emails.read().clone()
	}

	/// Clear all sent emails
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_backends::email::{Email, EmailBackend, MemoryEmailBackend};
	///
	/// #[tokio::main]
	/// async fn main() {
	///     let backend = MemoryEmailBackend::new();
	///
	///     let email = Email::builder()
	///         .from("sender@example.com")
	///         .to("recipient@example.com")
	///         .subject("Test")
	///         .text_body("Body")
	///         .build();
	///
	///     backend.send_email(&email).await.unwrap();
	///     assert_eq!(backend.sent_emails().len(), 1);
	///
	///     backend.clear();
	///     assert_eq!(backend.sent_emails().len(), 0);
	/// }
	/// ```
	pub fn clear(&self) {
		self.emails.write().clear();
	}

	/// Count sent emails
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_backends::email::{Email, EmailBackend, MemoryEmailBackend};
	///
	/// #[tokio::main]
	/// async fn main() {
	///     let backend = MemoryEmailBackend::new();
	///     assert_eq!(backend.count(), 0);
	///
	///     let email = Email::builder()
	///         .from("sender@example.com")
	///         .to("recipient@example.com")
	///         .subject("Test")
	///         .text_body("Body")
	///         .build();
	///
	///     backend.send_email(&email).await.unwrap();
	///     assert_eq!(backend.count(), 1);
	/// }
	/// ```
	pub fn count(&self) -> usize {
		self.emails.read().len()
	}

	/// Find emails by subject
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_backends::email::{Email, EmailBackend, MemoryEmailBackend};
	///
	/// #[tokio::main]
	/// async fn main() {
	///     let backend = MemoryEmailBackend::new();
	///
	///     let email1 = Email::builder()
	///         .from("sender@example.com")
	///         .to("recipient@example.com")
	///         .subject("Important")
	///         .text_body("Body 1")
	///         .build();
	///
	///     let email2 = Email::builder()
	///         .from("sender@example.com")
	///         .to("recipient@example.com")
	///         .subject("Notification")
	///         .text_body("Body 2")
	///         .build();
	///
	///     backend.send_email(&email1).await.unwrap();
	///     backend.send_email(&email2).await.unwrap();
	///
	///     let found = backend.find_by_subject("Important");
	///     assert_eq!(found.len(), 1);
	///     assert_eq!(found[0].subject, "Important");
	/// }
	/// ```
	pub fn find_by_subject(&self, subject: &str) -> Vec<Email> {
		self.emails
			.read()
			.iter()
			.filter(|email| email.subject == subject)
			.cloned()
			.collect()
	}

	/// Find emails by recipient
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_backends::email::{Email, EmailBackend, MemoryEmailBackend};
	///
	/// #[tokio::main]
	/// async fn main() {
	///     let backend = MemoryEmailBackend::new();
	///
	///     let email = Email::builder()
	///         .from("sender@example.com")
	///         .to("user1@example.com")
	///         .to("user2@example.com")
	///         .subject("Test")
	///         .text_body("Body")
	///         .build();
	///
	///     backend.send_email(&email).await.unwrap();
	///
	///     let found = backend.find_by_recipient("user1@example.com");
	///     assert_eq!(found.len(), 1);
	/// }
	/// ```
	pub fn find_by_recipient(&self, recipient: &str) -> Vec<Email> {
		self.emails
			.read()
			.iter()
			.filter(|email| email.to.contains(&recipient.to_string()))
			.cloned()
			.collect()
	}
}

#[async_trait]
impl EmailBackend for MemoryEmailBackend {
	async fn send_email(&self, email: &Email) -> EmailResult<()> {
		// Validate email
		email
			.validate()
			.map_err(|e| EmailError::Validation(format!("Email validation failed: {}", e)))?;

		// Store email
		self.emails.write().push(email.clone());

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

	#[tokio::test]
	async fn test_memory_backend_send_single() {
		let backend = MemoryEmailBackend::new();

		let email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.subject("Test Email")
			.text_body("This is a test")
			.build();

		let result = backend.send_email(&email).await;
		assert!(result.is_ok());

		let sent = backend.sent_emails();
		assert_eq!(sent.len(), 1);
		assert_eq!(sent[0].subject, "Test Email");
	}

	#[tokio::test]
	async fn test_memory_backend_send_bulk() {
		let backend = MemoryEmailBackend::new();

		let emails = vec![
			Email::builder()
				.from("sender@example.com")
				.to("user1@example.com")
				.subject("Email 1")
				.text_body("Body 1")
				.build(),
			Email::builder()
				.from("sender@example.com")
				.to("user2@example.com")
				.subject("Email 2")
				.text_body("Body 2")
				.build(),
		];

		let results = backend.send_bulk(&emails).await.unwrap();
		assert_eq!(results.len(), 2);
		assert!(results[0].is_ok());
		assert!(results[1].is_ok());

		let sent = backend.sent_emails();
		assert_eq!(sent.len(), 2);
	}

	#[tokio::test]
	async fn test_memory_backend_validation() {
		let backend = MemoryEmailBackend::new();

		// Invalid email (no recipient)
		let invalid_email = Email::builder()
			.from("sender@example.com")
			.subject("Test")
			.text_body("Body")
			.build();

		let result = backend.send_email(&invalid_email).await;
		assert!(result.is_err());

		let sent = backend.sent_emails();
		assert_eq!(sent.len(), 0);
	}

	#[tokio::test]
	async fn test_memory_backend_clear() {
		let backend = MemoryEmailBackend::new();

		let email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.subject("Test")
			.text_body("Body")
			.build();

		backend.send_email(&email).await.unwrap();
		assert_eq!(backend.count(), 1);

		backend.clear();
		assert_eq!(backend.count(), 0);
	}

	#[tokio::test]
	async fn test_memory_backend_find_by_subject() {
		let backend = MemoryEmailBackend::new();

		let email1 = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.subject("Important")
			.text_body("Body 1")
			.build();

		let email2 = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.subject("Notification")
			.text_body("Body 2")
			.build();

		backend.send_email(&email1).await.unwrap();
		backend.send_email(&email2).await.unwrap();

		let found = backend.find_by_subject("Important");
		assert_eq!(found.len(), 1);
		assert_eq!(found[0].subject, "Important");
	}

	#[tokio::test]
	async fn test_memory_backend_find_by_recipient() {
		let backend = MemoryEmailBackend::new();

		let email = Email::builder()
			.from("sender@example.com")
			.to("user1@example.com")
			.to("user2@example.com")
			.subject("Test")
			.text_body("Body")
			.build();

		backend.send_email(&email).await.unwrap();

		let found = backend.find_by_recipient("user1@example.com");
		assert_eq!(found.len(), 1);

		let found = backend.find_by_recipient("user3@example.com");
		assert_eq!(found.len(), 0);
	}

	#[tokio::test]
	async fn test_memory_backend_with_html_body() {
		let backend = MemoryEmailBackend::new();

		let email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.subject("HTML Email")
			.html_body("<h1>Hello, World!</h1>")
			.build();

		backend.send_email(&email).await.unwrap();

		let sent = backend.sent_emails();
		assert_eq!(sent.len(), 1);
		assert_eq!(sent[0].body.html(), Some("<h1>Hello, World!</h1>"));
	}

	#[tokio::test]
	async fn test_memory_backend_with_both_body() {
		let backend = MemoryEmailBackend::new();

		let email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.subject("Mixed Email")
			.both_body("Hello, World!", "<h1>Hello, World!</h1>")
			.build();

		backend.send_email(&email).await.unwrap();

		let sent = backend.sent_emails();
		assert_eq!(sent.len(), 1);
		assert_eq!(sent[0].body.text(), Some("Hello, World!"));
		assert_eq!(sent[0].body.html(), Some("<h1>Hello, World!</h1>"));
	}
}
