use crate::message::EmailMessage;
use crate::{EmailError, EmailResult};
use lettre::message::{Mailbox, MultiPart, SinglePart, header};
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::time::Duration;

/// Trait for email backends
#[async_trait::async_trait]
pub trait EmailBackend: Send + Sync {
	async fn send_messages(&self, messages: &[EmailMessage]) -> EmailResult<usize>;
}

pub fn backend_from_settings() {}

/// Console backend for development
///
/// Prints email messages to the console instead of sending them.
pub struct ConsoleBackend;

#[async_trait::async_trait]
impl EmailBackend for ConsoleBackend {
	async fn send_messages(&self, messages: &[EmailMessage]) -> EmailResult<usize> {
		for (i, msg) in messages.iter().enumerate() {
			println!("========== Email {} ==========", i + 1);
			println!("From: {}", msg.from_email);
			println!("To: {}", msg.to.join(", "));
			if !msg.cc.is_empty() {
				println!("Cc: {}", msg.cc.join(", "));
			}
			if !msg.bcc.is_empty() {
				println!("Bcc: {}", msg.bcc.join(", "));
			}
			println!("Subject: {}", msg.subject);
			println!("\n{}", msg.body);
			if let Some(html) = &msg.html_body {
				println!("\n--- HTML ---\n{}", html);
			}
			println!("==============================\n");
		}
		Ok(messages.len())
	}
}

/// File backend for saving emails to files
pub struct FileBackend {
	directory: std::path::PathBuf,
}

impl FileBackend {
	pub fn new(directory: impl Into<std::path::PathBuf>) -> Self {
		Self {
			directory: directory.into(),
		}
	}
}

#[async_trait::async_trait]
impl EmailBackend for FileBackend {
	async fn send_messages(&self, messages: &[EmailMessage]) -> EmailResult<usize> {
		std::fs::create_dir_all(&self.directory)?;

		for msg in messages.iter() {
			let filename = format!(
				"email_{}.eml",
				chrono::Utc::now().format("%Y%m%d_%H%M%S_%f")
			);
			let path = self.directory.join(filename);

			let content = format!(
				"From: {}\nTo: {}\nSubject: {}\n\n{}",
				msg.from_email,
				msg.to.join(", "),
				msg.subject,
				msg.body
			);

			tokio::fs::write(path, content).await?;
		}

		Ok(messages.len())
	}
}

/// Memory backend for testing
pub struct MemoryBackend {
	messages: std::sync::Arc<tokio::sync::Mutex<Vec<EmailMessage>>>,
}

impl MemoryBackend {
	pub fn new() -> Self {
		Self {
			messages: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
		}
	}

	pub async fn count(&self) -> usize {
		self.messages.lock().await.len()
	}

	pub async fn get_messages(&self) -> Vec<EmailMessage> {
		self.messages.lock().await.clone()
	}

	pub async fn clear(&self) {
		self.messages.lock().await.clear();
	}
}

impl Default for MemoryBackend {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait::async_trait]
impl EmailBackend for MemoryBackend {
	async fn send_messages(&self, messages: &[EmailMessage]) -> EmailResult<usize> {
		let mut stored = self.messages.lock().await;
		stored.extend_from_slice(messages);
		Ok(messages.len())
	}
}

/// SMTP connection security mode
#[derive(Debug, Clone)]
pub enum SmtpSecurity {
	/// No encryption
	None,
	/// STARTTLS (upgrade to TLS)
	StartTls,
	/// Direct TLS/SSL connection
	Tls,
}

/// SMTP authentication mechanism
#[derive(Debug, Clone)]
pub enum SmtpAuthMechanism {
	/// PLAIN authentication
	Plain,
	/// LOGIN authentication
	Login,
	/// Any supported mechanism
	Auto,
}

/// Configuration for SMTP backend
#[derive(Debug, Clone)]
pub struct SmtpConfig {
	pub host: String,
	pub port: u16,
	pub username: Option<String>,
	pub password: Option<String>,
	pub security: SmtpSecurity,
	pub auth_mechanism: SmtpAuthMechanism,
	pub timeout: Duration,
}

impl Default for SmtpConfig {
	fn default() -> Self {
		Self {
			host: "localhost".to_string(),
			port: 25,
			username: None,
			password: None,
			security: SmtpSecurity::None,
			auth_mechanism: SmtpAuthMechanism::Auto,
			timeout: Duration::from_secs(30),
		}
	}
}

impl SmtpConfig {
	pub fn new(host: impl Into<String>, port: u16) -> Self {
		Self {
			host: host.into(),
			port,
			..Default::default()
		}
	}

	pub fn with_credentials(mut self, username: String, password: String) -> Self {
		self.username = Some(username);
		self.password = Some(password);
		self
	}

	pub fn with_security(mut self, security: SmtpSecurity) -> Self {
		self.security = security;
		self
	}

	pub fn with_auth_mechanism(mut self, mechanism: SmtpAuthMechanism) -> Self {
		self.auth_mechanism = mechanism;
		self
	}

	pub fn with_timeout(mut self, timeout: Duration) -> Self {
		self.timeout = timeout;
		self
	}
}

/// SMTP backend for sending emails
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_mail::{SmtpBackend, SmtpConfig, SmtpSecurity};
/// use std::time::Duration;
///
/// let config = SmtpConfig::new("smtp.gmail.com", 587)
///     .with_credentials("user@gmail.com".to_string(), "password".to_string())
///     .with_security(SmtpSecurity::StartTls)
///     .with_timeout(Duration::from_secs(30));
///
/// let backend = SmtpBackend::new(config)?;
/// ```
pub struct SmtpBackend {
	config: SmtpConfig,
}

impl SmtpBackend {
	pub fn new(config: SmtpConfig) -> EmailResult<Self> {
		Ok(Self { config })
	}

	fn build_transport(&self) -> EmailResult<AsyncSmtpTransport<Tokio1Executor>> {
		let mut builder =
			AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&self.config.host)
				.port(self.config.port)
				.timeout(Some(self.config.timeout));

		// Configure TLS/SSL
		match &self.config.security {
			SmtpSecurity::None => {
				// No encryption
			}
			SmtpSecurity::StartTls => {
				let tls_params = TlsParameters::builder(self.config.host.clone())
					.build()
					.map_err(|e| EmailError::SmtpError(format!("TLS error: {}", e)))?;
				builder = builder.tls(Tls::Required(tls_params));
			}
			SmtpSecurity::Tls => {
				let tls_params = TlsParameters::builder(self.config.host.clone())
					.build()
					.map_err(|e| EmailError::SmtpError(format!("TLS error: {}", e)))?;
				builder = builder.tls(Tls::Wrapper(tls_params));
			}
		}

		// Configure authentication
		if let (Some(username), Some(password)) = (&self.config.username, &self.config.password) {
			let credentials = Credentials::new(username.clone(), password.clone());

			builder = match &self.config.auth_mechanism {
				SmtpAuthMechanism::Plain => builder
					.credentials(credentials)
					.authentication(vec![Mechanism::Plain]),
				SmtpAuthMechanism::Login => builder
					.credentials(credentials)
					.authentication(vec![Mechanism::Login]),
				SmtpAuthMechanism::Auto => builder.credentials(credentials),
			};
		}

		Ok(builder.build())
	}

	fn build_message(&self, email: &EmailMessage) -> EmailResult<Message> {
		// Parse from address
		let from = email
			.from_email
			.parse::<Mailbox>()
			.map_err(|e| EmailError::InvalidAddress(format!("Invalid from address: {}", e)))?;

		// Start building the message
		let mut builder = Message::builder().from(from).subject(&email.subject);

		// Add recipients
		for to in &email.to {
			let mailbox = to
				.parse::<Mailbox>()
				.map_err(|e| EmailError::InvalidAddress(format!("Invalid to address: {}", e)))?;
			builder = builder.to(mailbox);
		}

		// Add CC recipients
		for cc in &email.cc {
			let mailbox = cc
				.parse::<Mailbox>()
				.map_err(|e| EmailError::InvalidAddress(format!("Invalid cc address: {}", e)))?;
			builder = builder.cc(mailbox);
		}

		// Add BCC recipients
		for bcc in &email.bcc {
			let mailbox = bcc
				.parse::<Mailbox>()
				.map_err(|e| EmailError::InvalidAddress(format!("Invalid bcc address: {}", e)))?;
			builder = builder.bcc(mailbox);
		}

		// Add Reply-To
		for reply_to in &email.reply_to {
			let mailbox = reply_to.parse::<Mailbox>().map_err(|e| {
				EmailError::InvalidAddress(format!("Invalid reply-to address: {}", e))
			})?;
			builder = builder.reply_to(mailbox);
		}

		// TODO: Custom headers would need to be added using proper Header types
		// This is a limitation of the lettre API - headers must implement the Header trait
		// For now, we skip custom headers in the SMTP backend

		// Build the body
		let message = if let Some(html) = &email.html_body {
			// HTML with plain text alternative
			let multipart = MultiPart::alternative()
				.singlepart(SinglePart::plain(email.body.clone()))
				.singlepart(SinglePart::html(html.clone()));

			builder
				.multipart(multipart)
				.map_err(|e| EmailError::BackendError(format!("Failed to build message: {}", e)))?
		} else if !email.attachments.is_empty() {
			// Plain text with attachments
			let mut multipart =
				MultiPart::mixed().singlepart(SinglePart::plain(email.body.clone()));

			for attachment in &email.attachments {
				let part = if let Some(cid) = attachment.content_id() {
					// Inline attachment with content ID
					SinglePart::builder()
						.header(header::ContentDisposition::inline())
						.header(header::ContentId::from(cid.to_string()))
						.body(attachment.content().to_vec())
				} else {
					// Regular attachment
					SinglePart::builder()
						.header(header::ContentDisposition::attachment(
							attachment.filename(),
						))
						.body(attachment.content().to_vec())
				};

				multipart = multipart.singlepart(part);
			}

			builder
				.multipart(multipart)
				.map_err(|e| EmailError::BackendError(format!("Failed to build message: {}", e)))?
		} else {
			// Plain text only
			builder
				.body(email.body.clone())
				.map_err(|e| EmailError::BackendError(format!("Failed to build message: {}", e)))?
		};

		Ok(message)
	}
}

#[async_trait::async_trait]
impl EmailBackend for SmtpBackend {
	async fn send_messages(&self, messages: &[EmailMessage]) -> EmailResult<usize> {
		let transport = self.build_transport()?;

		let mut sent_count = 0;
		for email in messages {
			let message = self.build_message(email)?;

			transport
				.send(message)
				.await
				.map_err(|e| EmailError::SmtpError(format!("Failed to send email: {}", e)))?;

			sent_count += 1;
		}

		Ok(sent_count)
	}
}
