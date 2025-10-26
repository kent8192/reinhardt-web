//! # Reinhardt Email
//!
//! Django-style email sending for Reinhardt.
//!
//! ## Features
//!
//! - **EmailMessage**: Flexible email message builder
//! - **Multiple Backends**: SMTP, Console, File, Memory
//! - **HTML Email**: Support for HTML content with plain text fallback
//! - **Attachments**: File attachments and inline images
//! - **Templates**: Integration with template system
//! - **Bulk Sending**: Connection pooling for bulk emails
//!
//! ## Planned Features
//! TODO: Complete template system integration for dynamic email content
//! TODO: Fully implement file attachment support (Alternative and Attachment structs)
//! TODO: Add inline image support for HTML emails
//! TODO: Implement advanced SMTP features (TLS/SSL, authentication methods)
//! TODO: Add email validation and sanitization
//! TODO: Complete async email sending capabilities
//! TODO: Implement connection pooling for bulk email operations
//!
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_mail::{send_mail, EmailMessage, SmtpBackend};
//!
//! // Simple send_mail helper
//! send_mail(
//!     "Subject",
//!     "Message body",
//!     "from@example.com",
//!     vec!["to@example.com"],
//!     None,
//! ).await?;
//!
//! // Advanced EmailMessage
//! let email = EmailMessage::new()
//!     .subject("Welcome!")
//!     .body("Welcome to our service")
//!     .from("noreply@example.com")
//!     .to(vec!["user@example.com"])
//!     .html("<h1>Welcome!</h1>")
//!     .build()?;
//!
//! email.send().await?;
//! ```

pub mod backends;
pub mod message;
pub mod utils;

use thiserror::Error;

pub use backends::{
    backend_from_settings, ConsoleBackend, EmailBackend, FileBackend, MemoryBackend, SmtpBackend,
};
pub use message::{Alternative, Attachment, EmailMessage, EmailMessageBuilder};
pub use utils::{mail_admins, mail_managers, send_mail, send_mail_with_backend, send_mass_mail};

#[derive(Debug, Error)]
pub enum EmailError {
    #[error("Invalid email address: {0}")]
    InvalidAddress(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Backend error: {0}")]
    BackendError(String),

    #[error("SMTP error: {0}")]
    SmtpError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Template error: {0}")]
    TemplateError(String),

    #[error("Attachment error: {0}")]
    AttachmentError(String),

    #[error("Invalid header: {0}")]
    InvalidHeader(String),

    #[error("Header injection attempt detected: {0}")]
    HeaderInjection(String),
}

pub type EmailResult<T> = std::result::Result<T, EmailError>;
