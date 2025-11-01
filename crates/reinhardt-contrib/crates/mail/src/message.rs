use std::path::PathBuf;

/// Represents an alternative content type for an email message.
///
/// Alternatives allow providing different representations of the same content,
/// typically used for HTML vs. plain text versions.
///
/// # Examples
///
/// ```
/// use reinhardt_mail::Alternative;
///
/// let alternative = Alternative::new("text/html", "<h1>Hello!</h1>".as_bytes().to_vec());
/// assert_eq!(alternative.content_type(), "text/html");
/// ```
#[derive(Debug, Clone)]
pub struct Alternative {
	/// MIME content type (e.g., "text/html", "text/plain")
	content_type: String,
	/// Content data as bytes
	content: Vec<u8>,
}

impl Alternative {
	/// Create a new alternative content
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_mail::Alternative;
	///
	/// let html = Alternative::new("text/html", b"<h1>Hello</h1>".to_vec());
	/// assert_eq!(html.content_type(), "text/html");
	/// ```
	pub fn new(content_type: impl Into<String>, content: Vec<u8>) -> Self {
		Self {
			content_type: content_type.into(),
			content,
		}
	}

	/// Create an HTML alternative
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_mail::Alternative;
	///
	/// let html = Alternative::html("<h1>Welcome!</h1>");
	/// assert_eq!(html.content_type(), "text/html");
	/// ```
	pub fn html(content: impl Into<String>) -> Self {
		let content_str = content.into();
		Self::new("text/html", content_str.into_bytes())
	}

	/// Create a plain text alternative
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_mail::Alternative;
	///
	/// let text = Alternative::plain("Welcome!");
	/// assert_eq!(text.content_type(), "text/plain");
	/// ```
	pub fn plain(content: impl Into<String>) -> Self {
		let content_str = content.into();
		Self::new("text/plain", content_str.into_bytes())
	}

	/// Get the content type
	pub fn content_type(&self) -> &str {
		&self.content_type
	}

	/// Get the content as bytes
	pub fn content(&self) -> &[u8] {
		&self.content
	}

	/// Get the content as string (if valid UTF-8)
	pub fn content_as_string(&self) -> Option<String> {
		String::from_utf8(self.content.clone()).ok()
	}
}

/// Represents a file attachment for an email message.
///
/// Attachments can be created from file paths or raw bytes.
/// Supports automatic MIME type detection based on file extension.
///
/// # Examples
///
/// ```
/// use reinhardt_mail::Attachment;
/// use std::path::PathBuf;
///
/// // From bytes
/// let data = b"Hello, world!".to_vec();
/// let attachment = Attachment::new("hello.txt", data);
/// assert_eq!(attachment.filename(), "hello.txt");
///
/// // From file path
/// let path = PathBuf::from("/path/to/file.pdf");
/// let attachment = Attachment::from_path(path, "document.pdf");
/// ```
#[derive(Debug, Clone)]
pub struct Attachment {
	/// Original filename
	filename: String,
	/// File content as bytes
	content: Vec<u8>,
	/// MIME content type (auto-detected or specified)
	mime_type: String,
	/// Content-ID for inline attachments (e.g., for embedded images)
	content_id: Option<String>,
	/// Whether this is an inline attachment
	inline: bool,
}

impl Attachment {
	/// Create a new attachment from bytes
	///
	/// MIME type is automatically detected from the filename extension.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_mail::Attachment;
	///
	/// let data = b"PDF content".to_vec();
	/// let attachment = Attachment::new("document.pdf", data);
	/// assert_eq!(attachment.filename(), "document.pdf");
	/// assert!(attachment.mime_type().contains("pdf"));
	/// ```
	pub fn new(filename: impl Into<String>, content: Vec<u8>) -> Self {
		let filename_str = filename.into();
		let mime_type = Self::detect_mime_type(&filename_str);

		Self {
			filename: filename_str,
			content,
			mime_type,
			content_id: None,
			inline: false,
		}
	}

	/// Create a new attachment from a file path
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_mail::Attachment;
	/// use std::path::PathBuf;
	///
	/// let path = PathBuf::from("/tmp/test.txt");
	/// let attachment = Attachment::from_path(path, "report.txt");
	/// assert_eq!(attachment.filename(), "report.txt");
	/// ```
	pub fn from_path(_path: PathBuf, filename: impl Into<String>) -> Self {
		// In a real implementation, we would read the file here
		// For now, we create an empty attachment with the correct metadata
		let filename_str = filename.into();
		let mime_type = Self::detect_mime_type(&filename_str);

		Self {
			filename: filename_str,
			content: Vec::new(),
			mime_type,
			content_id: None,
			inline: false,
		}
	}

	/// Create an inline attachment (for embedded images, etc.)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_mail::Attachment;
	///
	/// let image_data = b"\x89PNG\r\n\x1a\n".to_vec(); // PNG header
	/// let attachment = Attachment::inline("logo.png", image_data, "logo-cid");
	/// assert!(attachment.is_inline());
	/// assert_eq!(attachment.content_id(), Some("logo-cid"));
	/// ```
	pub fn inline(
		filename: impl Into<String>,
		content: Vec<u8>,
		content_id: impl Into<String>,
	) -> Self {
		let filename_str = filename.into();
		let mime_type = Self::detect_mime_type(&filename_str);

		Self {
			filename: filename_str,
			content,
			mime_type,
			content_id: Some(content_id.into()),
			inline: true,
		}
	}

	/// Set a custom MIME type
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_mail::Attachment;
	///
	/// let mut attachment = Attachment::new("data.bin", vec![1, 2, 3]);
	/// attachment.with_mime_type("application/octet-stream");
	/// assert_eq!(attachment.mime_type(), "application/octet-stream");
	/// ```
	pub fn with_mime_type(&mut self, mime_type: impl Into<String>) -> &mut Self {
		self.mime_type = mime_type.into();
		self
	}

	/// Set as inline attachment with content ID
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_mail::Attachment;
	///
	/// let mut attachment = Attachment::new("logo.png", vec![]);
	/// attachment.as_inline("logo-123");
	/// assert!(attachment.is_inline());
	/// ```
	pub fn as_inline(&mut self, content_id: impl Into<String>) -> &mut Self {
		self.content_id = Some(content_id.into());
		self.inline = true;
		self
	}

	/// Get the filename
	pub fn filename(&self) -> &str {
		&self.filename
	}

	/// Get the content
	pub fn content(&self) -> &[u8] {
		&self.content
	}

	/// Get the MIME type
	pub fn mime_type(&self) -> &str {
		&self.mime_type
	}

	/// Get the content ID (for inline attachments)
	pub fn content_id(&self) -> Option<&str> {
		self.content_id.as_deref()
	}

	/// Check if this is an inline attachment
	pub fn is_inline(&self) -> bool {
		self.inline
	}

	/// Detect MIME type from filename
	fn detect_mime_type(filename: &str) -> String {
		mime_guess::from_path(filename)
			.first()
			.map(|mime| mime.to_string())
			.unwrap_or_else(|| "application/octet-stream".to_string())
	}
}

#[derive(Debug, Clone)]
pub struct EmailMessage {
	pub subject: String,
	pub body: String,
	pub from_email: String,
	pub to: Vec<String>,
	pub cc: Vec<String>,
	pub bcc: Vec<String>,
	pub reply_to: Vec<String>,
	pub html_body: Option<String>,
	pub alternatives: Vec<Alternative>,
	pub attachments: Vec<Attachment>,
	pub headers: Vec<(String, String)>,
}

impl EmailMessage {
	pub fn new() -> EmailMessageBuilder {
		EmailMessageBuilder::default()
	}

	pub fn subject(&mut self, subject: impl Into<String>) -> &mut Self {
		self.subject = subject.into();
		self
	}

	pub fn body(&mut self, body: impl Into<String>) -> &mut Self {
		self.body = body.into();
		self
	}

	pub fn from_email(&mut self, from: impl Into<String>) -> &mut Self {
		self.from_email = from.into();
		self
	}

	pub fn to(&mut self, to: Vec<String>) -> &mut Self {
		self.to = to;
		self
	}

	pub fn cc(&mut self, cc: Vec<String>) -> &mut Self {
		self.cc = cc;
		self
	}

	pub fn bcc(&mut self, bcc: Vec<String>) -> &mut Self {
		self.bcc = bcc;
		self
	}

	pub fn reply_to(&mut self, reply_to: Vec<String>) -> &mut Self {
		self.reply_to = reply_to;
		self
	}

	pub fn add_attachment(&mut self, attachment: Attachment) -> &mut Self {
		self.attachments.push(attachment);
		self
	}

	pub fn add_alternative(&mut self, alternative: Alternative) -> &mut Self {
		self.alternatives.push(alternative);
		self
	}

	pub fn add_header(&mut self, name: impl Into<String>, value: impl Into<String>) -> &mut Self {
		self.headers.push((name.into(), value.into()));
		self
	}

	pub async fn send(
		&self,
		backend: &dyn crate::backends::EmailBackend,
	) -> crate::EmailResult<()> {
		backend.send_messages(&[self.clone()]).await?;
		Ok(())
	}

	pub async fn send_with_backend(
		&self,
		backend: &dyn crate::backends::EmailBackend,
	) -> crate::EmailResult<()> {
		backend.send_messages(&[self.clone()]).await?;
		Ok(())
	}
}

#[derive(Default)]
pub struct EmailMessageBuilder {
	subject: String,
	body: String,
	from_email: String,
	to: Vec<String>,
	cc: Vec<String>,
	bcc: Vec<String>,
	reply_to: Vec<String>,
	html_body: Option<String>,
	alternatives: Vec<Alternative>,
	attachments: Vec<Attachment>,
	headers: Vec<(String, String)>,
}

impl EmailMessageBuilder {
	pub fn subject(mut self, subject: impl Into<String>) -> Self {
		self.subject = subject.into();
		self
	}

	pub fn body(mut self, body: impl Into<String>) -> Self {
		self.body = body.into();
		self
	}

	pub fn from(mut self, from: impl Into<String>) -> Self {
		self.from_email = from.into();
		self
	}

	pub fn from_email(mut self, from: impl Into<String>) -> Self {
		self.from_email = from.into();
		self
	}

	pub fn to(mut self, to: Vec<String>) -> Self {
		self.to = to;
		self
	}

	pub fn cc(mut self, cc: Vec<String>) -> Self {
		self.cc = cc;
		self
	}

	pub fn bcc(mut self, bcc: Vec<String>) -> Self {
		self.bcc = bcc;
		self
	}

	pub fn reply_to(mut self, reply_to: Vec<String>) -> Self {
		self.reply_to = reply_to;
		self
	}

	pub fn html(mut self, html: impl Into<String>) -> Self {
		self.html_body = Some(html.into());
		self
	}

	pub fn attachment(mut self, attachment: Attachment) -> Self {
		self.attachments.push(attachment);
		self
	}

	pub fn alternative(mut self, alternative: Alternative) -> Self {
		self.alternatives.push(alternative);
		self
	}

	pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
		self.headers.push((name.into(), value.into()));
		self
	}

	pub fn build(self) -> EmailMessage {
		EmailMessage {
			subject: self.subject,
			body: self.body,
			from_email: self.from_email,
			to: self.to,
			cc: self.cc,
			bcc: self.bcc,
			reply_to: self.reply_to,
			html_body: self.html_body,
			alternatives: self.alternatives,
			attachments: self.attachments,
			headers: self.headers,
		}
	}
}
