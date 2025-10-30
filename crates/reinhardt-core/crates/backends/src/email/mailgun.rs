//! Mailgun email backend
//!
//! This backend sends emails through the Mailgun API.
//!
//! # Examples
//!
//! ```no_run
//! # #[cfg(feature = "email-mailgun")]
//! use reinhardt_backends::email::{Email, EmailBackend, MailgunBackend};
//!
//! # #[cfg(feature = "email-mailgun")]
//! #[tokio::main]
//! async fn main() {
//!     let backend = MailgunBackend::new(
//!         "your-api-key".to_string(),
//!         "your-domain.com".to_string(),
//!     );
//!
//!     let email = Email::builder()
//!         .from("sender@your-domain.com")
//!         .to("recipient@example.com")
//!         .subject("Test")
//!         .text_body("Hello!")
//!         .build();
//!
//!     backend.send_email(&email).await.unwrap();
//! }
//! # #[cfg(not(feature = "email-mailgun"))]
//! # fn main() {}
//! ```

use crate::email::types::{Email, EmailBackend, EmailBody, EmailError, EmailResult};
use async_trait::async_trait;
use reqwest::{Client, multipart};
use std::time::Duration;

/// Mailgun API region
#[derive(Debug, Clone)]
pub enum MailgunRegion {
    /// US region (api.mailgun.net)
    US,
    /// EU region (api.eu.mailgun.net)
    EU,
}

impl MailgunRegion {
    fn base_url(&self) -> &'static str {
        match self {
            MailgunRegion::US => "https://api.mailgun.net/v3",
            MailgunRegion::EU => "https://api.eu.mailgun.net/v3",
        }
    }
}

/// Mailgun email backend
///
/// Sends emails through the Mailgun API.
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "email-mailgun")]
/// use reinhardt_backends::email::{Email, EmailBackend, MailgunBackend};
///
/// # #[cfg(feature = "email-mailgun")]
/// #[tokio::main]
/// async fn main() {
///     let backend = MailgunBackend::new(
///         "your-api-key".to_string(),
///         "your-domain.com".to_string(),
///     );
///
///     let email = Email::builder()
///         .from("sender@your-domain.com")
///         .to("recipient@example.com")
///         .subject("Test")
///         .text_body("Hello!")
///         .build();
///
///     backend.send_email(&email).await.unwrap();
/// }
/// # #[cfg(not(feature = "email-mailgun"))]
/// # fn main() {}
/// ```
pub struct MailgunBackend {
    api_key: String,
    domain: String,
    region: MailgunRegion,
    client: Client,
}

impl MailgunBackend {
    /// Create a new Mailgun backend with US region
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "email-mailgun")]
    /// use reinhardt_backends::email::MailgunBackend;
    ///
    /// # #[cfg(feature = "email-mailgun")]
    /// let backend = MailgunBackend::new(
    ///     "your-api-key".to_string(),
    ///     "your-domain.com".to_string(),
    /// );
    /// ```
    pub fn new(api_key: String, domain: String) -> Self {
        Self::with_region(api_key, domain, MailgunRegion::US)
    }

    /// Create a new Mailgun backend with specified region
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "email-mailgun")]
    /// use reinhardt_backends::email::mailgun::{MailgunBackend, MailgunRegion};
    ///
    /// # #[cfg(feature = "email-mailgun")]
    /// let backend = MailgunBackend::with_region(
    ///     "your-api-key".to_string(),
    ///     "your-domain.com".to_string(),
    ///     MailgunRegion::EU,
    /// );
    /// ```
    pub fn with_region(api_key: String, domain: String, region: MailgunRegion) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            domain,
            region,
            client,
        }
    }

    /// Create a Mailgun backend with custom client
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "email-mailgun")]
    /// use reinhardt_backends::email::mailgun::{MailgunBackend, MailgunRegion};
    /// use reqwest::Client;
    ///
    /// # #[cfg(feature = "email-mailgun")]
    /// let client = Client::new();
    /// # #[cfg(feature = "email-mailgun")]
    /// let backend = MailgunBackend::with_client(
    ///     "your-api-key".to_string(),
    ///     "your-domain.com".to_string(),
    ///     MailgunRegion::US,
    ///     client,
    /// );
    /// ```
    pub fn with_client(
        api_key: String,
        domain: String,
        region: MailgunRegion,
        client: Client,
    ) -> Self {
        Self {
            api_key,
            domain,
            region,
            client,
        }
    }

    fn build_form(&self, email: &Email) -> EmailResult<multipart::Form> {
        let mut form = multipart::Form::new()
            .text("from", email.from.clone())
            .text("subject", email.subject.clone());

        // Add recipients
        for to in &email.to {
            form = form.text("to", to.clone());
        }

        // Add CC recipients
        if let Some(cc_list) = &email.cc {
            for cc in cc_list {
                form = form.text("cc", cc.clone());
            }
        }

        // Add BCC recipients
        if let Some(bcc_list) = &email.bcc {
            for bcc in bcc_list {
                form = form.text("bcc", bcc.clone());
            }
        }

        // Add body
        match &email.body {
            EmailBody::Text(text) => {
                form = form.text("text", text.clone());
            }
            EmailBody::Html(html) => {
                form = form.text("html", html.clone());
            }
            EmailBody::Both { text, html } => {
                form = form.text("text", text.clone()).text("html", html.clone());
            }
        }

        // Add attachments
        for attachment in &email.attachments {
            let part = multipart::Part::bytes(attachment.content.clone())
                .file_name(attachment.filename.clone())
                .mime_str(&attachment.content_type)
                .map_err(|e| {
                    EmailError::Internal(format!("Failed to set MIME type: {}", e))
                })?;
            form = form.part("attachment", part);
        }

        Ok(form)
    }
}

#[async_trait]
impl EmailBackend for MailgunBackend {
    async fn send_email(&self, email: &Email) -> EmailResult<()> {
        // Validate email
        email.validate()?;

        // Build form
        let form = self.build_form(email)?;

        // Build URL
        let url = format!("{}/{}/messages", self.region.base_url(), self.domain);

        // Send request
        let response = self
            .client
            .post(&url)
            .basic_auth("api", Some(&self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| EmailError::Send(format!("Mailgun API request failed: {}", e)))?;

        // Check response status
        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(EmailError::Api(format!(
                "Mailgun API error ({}): {}",
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
    fn test_mailgun_region_us() {
        let region = MailgunRegion::US;
        assert_eq!(region.base_url(), "https://api.mailgun.net/v3");
    }

    #[test]
    fn test_mailgun_region_eu() {
        let region = MailgunRegion::EU;
        assert_eq!(region.base_url(), "https://api.eu.mailgun.net/v3");
    }

    #[test]
    fn test_mailgun_backend_new() {
        let backend = MailgunBackend::new(
            "test-api-key".to_string(),
            "test-domain.com".to_string(),
        );
        assert_eq!(backend.api_key, "test-api-key");
        assert_eq!(backend.domain, "test-domain.com");
    }

    #[test]
    fn test_mailgun_backend_with_region() {
        let backend = MailgunBackend::with_region(
            "test-api-key".to_string(),
            "test-domain.com".to_string(),
            MailgunRegion::EU,
        );
        assert_eq!(backend.api_key, "test-api-key");
        assert_eq!(backend.domain, "test-domain.com");
    }

    #[tokio::test]
    async fn test_mailgun_build_form_text() {
        let backend = MailgunBackend::new(
            "test-key".to_string(),
            "test-domain.com".to_string(),
        );

        let email = Email::builder()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test")
            .text_body("Hello!")
            .build();

        let form = backend.build_form(&email);
        assert!(form.is_ok());
    }

    #[tokio::test]
    async fn test_mailgun_build_form_html() {
        let backend = MailgunBackend::new(
            "test-key".to_string(),
            "test-domain.com".to_string(),
        );

        let email = Email::builder()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test")
            .html_body("<h1>Hello!</h1>")
            .build();

        let form = backend.build_form(&email);
        assert!(form.is_ok());
    }

    #[tokio::test]
    async fn test_mailgun_build_form_both() {
        let backend = MailgunBackend::new(
            "test-key".to_string(),
            "test-domain.com".to_string(),
        );

        let email = Email::builder()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test")
            .both_body("Hello!", "<h1>Hello!</h1>")
            .build();

        let form = backend.build_form(&email);
        assert!(form.is_ok());
    }

    #[tokio::test]
    async fn test_mailgun_build_form_with_cc_bcc() {
        let backend = MailgunBackend::new(
            "test-key".to_string(),
            "test-domain.com".to_string(),
        );

        let email = Email::builder()
            .from("sender@example.com")
            .to("recipient@example.com")
            .cc("cc@example.com")
            .bcc("bcc@example.com")
            .subject("Test")
            .text_body("Body")
            .build();

        let form = backend.build_form(&email);
        assert!(form.is_ok());
    }

    #[tokio::test]
    async fn test_mailgun_build_form_with_attachments() {
        let backend = MailgunBackend::new(
            "test-key".to_string(),
            "test-domain.com".to_string(),
        );

        let attachment = Attachment::new("file.txt", "text/plain", b"Hello".to_vec());

        let email = Email::builder()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test")
            .text_body("Body")
            .attachment(attachment)
            .build();

        let form = backend.build_form(&email);
        assert!(form.is_ok());
    }
}
