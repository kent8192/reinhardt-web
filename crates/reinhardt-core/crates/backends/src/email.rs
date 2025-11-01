//! Email Backend Module
//!
//! This module provides a unified email backend system for the Reinhardt framework.
//! It supports multiple email service providers through a common trait interface.
//!
//! # Supported Backends
//!
//! - **SMTP**: Direct SMTP server connection using lettre
//! - **SendGrid**: SendGrid API integration
//! - **AWS SES**: Amazon Simple Email Service
//! - **Mailgun**: Mailgun API integration
//!
//! # Architecture
//!
//! All backends implement the `EmailBackend` trait, which provides:
//! - Single email sending
//! - Bulk email sending
//! - Support for HTML and plain text
//! - Attachments
//! - CC/BCC recipients
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```
//! use reinhardt_backends::email::{Email, EmailBody, EmailBackend};
//! # use reinhardt_backends::email::MemoryEmailBackend;
//!
//! #[tokio::main]
//! async fn main() {
//!     # let backend = MemoryEmailBackend::new();
//!     let email = Email {
//!         from: "sender@example.com".to_string(),
//!         to: vec!["recipient@example.com".to_string()],
//!         cc: None,
//!         bcc: None,
//!         subject: "Hello".to_string(),
//!         body: EmailBody::Text("Hello, World!".to_string()),
//!         attachments: vec![],
//!     };
//!
//!     backend.send_email(&email).await.unwrap();
//! }
//! ```

pub mod memory;
pub mod types;

#[cfg(feature = "email-smtp")]
pub mod smtp;

#[cfg(feature = "email-sendgrid")]
pub mod sendgrid;

#[cfg(feature = "email-ses")]
pub mod ses;

#[cfg(feature = "email-mailgun")]
pub mod mailgun;

// Re-exports
pub use memory::MemoryEmailBackend;
pub use types::{Attachment, Email, EmailBackend, EmailBody, EmailError, EmailResult};

#[cfg(feature = "email-smtp")]
pub use smtp::SmtpBackend;

#[cfg(feature = "email-sendgrid")]
pub use sendgrid::SendGridBackend;

#[cfg(feature = "email-ses")]
pub use ses::SesBackend;

#[cfg(feature = "email-mailgun")]
pub use mailgun::MailgunBackend;
