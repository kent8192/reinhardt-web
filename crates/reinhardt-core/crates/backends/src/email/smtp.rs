//! SMTP email backend using lettre
//!
//! This backend sends emails through an SMTP server using the lettre library.
//! Supports TLS/STARTTLS and various authentication methods.
//!
//! # Examples
//!
//! ```no_run
//! use reinhardt_backends::email::{Email, EmailBackend};
//! # #[cfg(feature = "email-smtp")]
//! use reinhardt_backends::email::smtp::{SmtpBackend, SmtpConfig, SmtpAuth, SmtpEncryption};
//!
//! # #[cfg(feature = "email-smtp")]
//! #[tokio::main]
//! async fn main() {
//!     let config = SmtpConfig {
//!         host: "smtp.gmail.com".to_string(),
//!         port: 587,
//!         encryption: SmtpEncryption::StartTls,
//!         auth: Some(SmtpAuth {
//!             username: "user@gmail.com".to_string(),
//!             password: "password".to_string(),
//!         }),
//!         timeout: std::time::Duration::from_secs(30),
//!         pool_size: 5,
//!     };
//!
//!     let backend = SmtpBackend::new(config).await.unwrap();
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
//! # #[cfg(not(feature = "email-smtp"))]
//! # fn main() {}
//! ```

use crate::email::types::{Email, EmailBackend, EmailBody, EmailError, EmailResult};
use async_trait::async_trait;
use lettre::message::{Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::time::Duration;

/// SMTP encryption method
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "email-smtp")]
/// use reinhardt_backends::email::smtp::SmtpEncryption;
///
/// # #[cfg(feature = "email-smtp")]
/// let tls = SmtpEncryption::Tls;
/// # #[cfg(feature = "email-smtp")]
/// let starttls = SmtpEncryption::StartTls;
/// # #[cfg(feature = "email-smtp")]
/// let none = SmtpEncryption::None;
/// ```
#[derive(Debug, Clone)]
pub enum SmtpEncryption {
	/// No encryption (not recommended)
	None,
	/// STARTTLS (upgrade to TLS after connection)
	StartTls,
	/// Direct TLS connection
	Tls,
}

/// SMTP authentication credentials
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "email-smtp")]
/// use reinhardt_backends::email::smtp::SmtpAuth;
///
/// # #[cfg(feature = "email-smtp")]
/// let auth = SmtpAuth {
///     username: "user@example.com".to_string(),
///     password: "password".to_string(),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct SmtpAuth {
	/// SMTP username
	pub username: String,
	/// SMTP password
	pub password: String,
}

/// SMTP backend configuration
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "email-smtp")]
/// use reinhardt_backends::email::smtp::{SmtpConfig, SmtpAuth, SmtpEncryption};
/// use std::time::Duration;
///
/// # #[cfg(feature = "email-smtp")]
/// let config = SmtpConfig {
///     host: "smtp.gmail.com".to_string(),
///     port: 587,
///     encryption: SmtpEncryption::StartTls,
///     auth: Some(SmtpAuth {
///         username: "user@gmail.com".to_string(),
///         password: "password".to_string(),
///     }),
///     timeout: Duration::from_secs(30),
///     pool_size: 5,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct SmtpConfig {
	/// SMTP server hostname
	pub host: String,
	/// SMTP server port
	pub port: u16,
	/// Encryption method
	pub encryption: SmtpEncryption,
	/// Optional authentication credentials
	pub auth: Option<SmtpAuth>,
	/// Connection timeout
	pub timeout: Duration,
	/// Connection pool size
	pub pool_size: usize,
}

impl Default for SmtpConfig {
	fn default() -> Self {
		Self {
			host: "localhost".to_string(),
			port: 25,
			encryption: SmtpEncryption::None,
			auth: None,
			timeout: Duration::from_secs(30),
			pool_size: 5,
		}
	}
}

/// SMTP email backend
///
/// Sends emails through an SMTP server using lettre.
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "email-smtp")]
/// use reinhardt_backends::email::{Email, EmailBackend};
/// use reinhardt_backends::email::smtp::{SmtpBackend, SmtpConfig, SmtpEncryption};
///
/// # #[cfg(feature = "email-smtp")]
/// #[tokio::main]
/// async fn main() {
///     let config = SmtpConfig {
///         host: "smtp.example.com".to_string(),
///         port: 587,
///         encryption: SmtpEncryption::StartTls,
///         ..Default::default()
///     };
///
///     let backend = SmtpBackend::new(config).await.unwrap();
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
/// # #[cfg(not(feature = "email-smtp"))]
/// # fn main() {}
/// ```
pub struct SmtpBackend {
	transport: AsyncSmtpTransport<Tokio1Executor>,
}

impl SmtpBackend {
	/// Create a new SMTP backend
	///
	/// # Examples
	///
	/// ```no_run
	/// # #[cfg(feature = "email-smtp")]
	/// use reinhardt_backends::email::smtp::{SmtpBackend, SmtpConfig};
	///
	/// # #[cfg(feature = "email-smtp")]
	/// #[tokio::main]
	/// async fn main() {
	///     let config = SmtpConfig::default();
	///     let backend = SmtpBackend::new(config).await.unwrap();
	/// }
	/// # #[cfg(not(feature = "email-smtp"))]
	/// # fn main() {}
	/// ```
	pub async fn new(config: SmtpConfig) -> EmailResult<Self> {
		let transport = Self::create_transport(&config)?;

		Ok(Self { transport })
	}

	fn create_transport(config: &SmtpConfig) -> EmailResult<AsyncSmtpTransport<Tokio1Executor>> {
		let mut builder = match config.encryption {
			SmtpEncryption::None => {
				AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
			}
			SmtpEncryption::StartTls => {
				AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
					.map_err(|e| EmailError::Configuration(format!("STARTTLS error: {}", e)))?
			}
			SmtpEncryption::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
				.map_err(|e| EmailError::Configuration(format!("TLS error: {}", e)))?,
		};

		builder = builder.port(config.port);
		builder = builder.timeout(Some(config.timeout));
		builder = builder.pool_config(
			lettre::transport::smtp::PoolConfig::new()
				.max_size(config.pool_size.try_into().unwrap_or(5)),
		);

		if let Some(auth) = &config.auth {
			builder = builder.credentials(Credentials::new(
				auth.username.clone(),
				auth.password.clone(),
			));
		}

		Ok(builder.build())
	}

	fn build_message(&self, email: &Email) -> EmailResult<Message> {
		// Parse from address
		let from: Mailbox = email
			.from
			.parse()
			.map_err(|e| EmailError::Validation(format!("Invalid from address: {}", e)))?;

		// Start building message
		let mut builder = Message::builder().from(from);

		// Add recipients
		for to in &email.to {
			let mailbox: Mailbox = to
				.parse()
				.map_err(|e| EmailError::Validation(format!("Invalid to address: {}", e)))?;
			builder = builder.to(mailbox);
		}

		// Add CC recipients
		if let Some(cc_list) = &email.cc {
			for cc in cc_list {
				let mailbox: Mailbox = cc
					.parse()
					.map_err(|e| EmailError::Validation(format!("Invalid cc address: {}", e)))?;
				builder = builder.cc(mailbox);
			}
		}

		// Add BCC recipients
		if let Some(bcc_list) = &email.bcc {
			for bcc in bcc_list {
				let mailbox: Mailbox = bcc
					.parse()
					.map_err(|e| EmailError::Validation(format!("Invalid bcc address: {}", e)))?;
				builder = builder.bcc(mailbox);
			}
		}

		// Add subject
		builder = builder.subject(&email.subject);

		// Build body based on content type
		let message = match &email.body {
			EmailBody::Text(text) => builder
				.singlepart(SinglePart::plain(text.clone()))
				.map_err(|e| EmailError::Internal(format!("Failed to build message: {}", e)))?,
			EmailBody::Html(html) => builder
				.singlepart(SinglePart::html(html.clone()))
				.map_err(|e| EmailError::Internal(format!("Failed to build message: {}", e)))?,
			EmailBody::Both { text, html } => {
				let multipart = MultiPart::alternative()
					.singlepart(SinglePart::plain(text.clone()))
					.singlepart(SinglePart::html(html.clone()));

				builder
					.multipart(multipart)
					.map_err(|e| EmailError::Internal(format!("Failed to build message: {}", e)))?
			}
		};

		Ok(message)
	}
}

#[async_trait]
impl EmailBackend for SmtpBackend {
	async fn send_email(&self, email: &Email) -> EmailResult<()> {
		// Validate email
		email.validate()?;

		// Build message
		let message = self.build_message(email)?;

		// Send email
		self.transport
			.send(message)
			.await
			.map_err(|e| EmailError::Send(format!("SMTP send failed: {}", e)))?;

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

	#[test]
	fn test_smtp_config_default() {
		let config = SmtpConfig::default();
		assert_eq!(config.host, "localhost");
		assert_eq!(config.port, 25);
		assert_eq!(config.pool_size, 5);
	}

	#[test]
	fn test_smtp_auth() {
		let auth = SmtpAuth {
			username: "user@example.com".to_string(),
			password: "password".to_string(),
		};
		assert_eq!(auth.username, "user@example.com");
	}

	#[tokio::test]
	async fn test_smtp_backend_build_message() {
		let config = SmtpConfig::default();
		let backend = SmtpBackend::new(config).await.unwrap();

		let email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.subject("Test")
			.text_body("Body")
			.build();

		let message = backend.build_message(&email);
		assert!(message.is_ok());
	}

	#[tokio::test]
	async fn test_smtp_backend_build_message_with_html() {
		let config = SmtpConfig::default();
		let backend = SmtpBackend::new(config).await.unwrap();

		let email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.subject("Test")
			.html_body("<h1>Body</h1>")
			.build();

		let message = backend.build_message(&email);
		assert!(message.is_ok());
	}

	#[tokio::test]
	async fn test_smtp_backend_build_message_with_both() {
		let config = SmtpConfig::default();
		let backend = SmtpBackend::new(config).await.unwrap();

		let email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.subject("Test")
			.both_body("Text", "<h1>HTML</h1>")
			.build();

		let message = backend.build_message(&email);
		assert!(message.is_ok());
	}

	#[tokio::test]
	async fn test_smtp_backend_build_message_with_cc_bcc() {
		let config = SmtpConfig::default();
		let backend = SmtpBackend::new(config).await.unwrap();

		let email = Email::builder()
			.from("sender@example.com")
			.to("recipient@example.com")
			.cc("cc@example.com")
			.bcc("bcc@example.com")
			.subject("Test")
			.text_body("Body")
			.build();

		let message = backend.build_message(&email);
		assert!(message.is_ok());
	}
}
