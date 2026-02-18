//! WebSocket middleware integration
//!
//! This module provides middleware support for WebSocket connections,
//! allowing pre-processing and post-processing of connections and messages.

use crate::connection::{Message, WebSocketConnection};
use async_trait::async_trait;
use std::sync::Arc;

/// WebSocket middleware result
pub type MiddlewareResult<T> = Result<T, MiddlewareError>;

/// Middleware errors
#[derive(Debug, thiserror::Error)]
pub enum MiddlewareError {
	#[error("Connection rejected: {0}")]
	ConnectionRejected(String),
	#[error("Message rejected: {0}")]
	MessageRejected(String),
	#[error("Middleware error: {0}")]
	Error(String),
}

/// WebSocket connection context for middleware
pub struct ConnectionContext {
	/// Client IP address
	pub ip: String,
	/// Connection headers (if available)
	pub headers: std::collections::HashMap<String, String>,
	/// Custom metadata
	pub metadata: std::collections::HashMap<String, String>,
}

impl ConnectionContext {
	/// Create a new connection context
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::middleware::ConnectionContext;
	///
	/// let context = ConnectionContext::new("192.168.1.1".to_string());
	/// assert_eq!(context.ip, "192.168.1.1");
	/// ```
	pub fn new(ip: String) -> Self {
		Self {
			ip,
			headers: std::collections::HashMap::new(),
			metadata: std::collections::HashMap::new(),
		}
	}

	/// Add a header to the context
	pub fn with_header(mut self, key: String, value: String) -> Self {
		self.headers.insert(key, value);
		self
	}

	/// Add metadata to the context
	pub fn with_metadata(mut self, key: String, value: String) -> Self {
		self.metadata.insert(key, value);
		self
	}
}

/// WebSocket connection middleware trait
///
/// Implementors can intercept WebSocket connections before they are established.
#[async_trait]
pub trait ConnectionMiddleware: Send + Sync {
	/// Process a connection before it is established
	///
	/// # Arguments
	///
	/// * `context` - Connection context with client information
	///
	/// # Returns
	///
	/// Returns `Ok(())` to allow the connection, or an error to reject it.
	async fn on_connect(&self, context: &mut ConnectionContext) -> MiddlewareResult<()>;

	/// Process a connection after it is closed
	///
	/// # Arguments
	///
	/// * `connection` - The closed connection
	async fn on_disconnect(&self, connection: &Arc<WebSocketConnection>) -> MiddlewareResult<()>;
}

/// WebSocket message middleware trait
///
/// Implementors can intercept and modify messages before they are processed.
#[async_trait]
pub trait MessageMiddleware: Send + Sync {
	/// Process a message before it is handled
	///
	/// # Arguments
	///
	/// * `connection` - The connection that sent the message
	/// * `message` - The message to process
	///
	/// # Returns
	///
	/// Returns the processed message, or an error to reject it.
	async fn on_message(
		&self,
		connection: &Arc<WebSocketConnection>,
		message: Message,
	) -> MiddlewareResult<Message>;
}

/// Logging middleware for WebSocket connections
///
/// # Examples
///
/// ```
/// use reinhardt_websockets::middleware::{LoggingMiddleware, ConnectionMiddleware, ConnectionContext};
///
/// # tokio_test::block_on(async {
/// let middleware = LoggingMiddleware::new("WebSocket".to_string());
/// let mut context = ConnectionContext::new("192.168.1.1".to_string());
///
/// assert!(middleware.on_connect(&mut context).await.is_ok());
/// # });
/// ```
pub struct LoggingMiddleware {
	prefix: String,
}

impl LoggingMiddleware {
	/// Create a new logging middleware
	pub fn new(prefix: String) -> Self {
		Self { prefix }
	}
}

#[async_trait]
impl ConnectionMiddleware for LoggingMiddleware {
	async fn on_connect(&self, context: &mut ConnectionContext) -> MiddlewareResult<()> {
		println!(
			"[{}] Connection established from {}",
			self.prefix, context.ip
		);
		Ok(())
	}

	async fn on_disconnect(&self, connection: &Arc<WebSocketConnection>) -> MiddlewareResult<()> {
		println!("[{}] Connection closed: {}", self.prefix, connection.id());
		Ok(())
	}
}

#[async_trait]
impl MessageMiddleware for LoggingMiddleware {
	async fn on_message(
		&self,
		connection: &Arc<WebSocketConnection>,
		message: Message,
	) -> MiddlewareResult<Message> {
		match &message {
			Message::Text { data } => {
				println!(
					"[{}] Text message from {}: {}",
					self.prefix,
					connection.id(),
					data
				);
			}
			Message::Binary { data } => {
				println!(
					"[{}] Binary message from {}: {} bytes",
					self.prefix,
					connection.id(),
					data.len()
				);
			}
			Message::Ping => {
				println!("[{}] Ping from {}", self.prefix, connection.id());
			}
			Message::Pong => {
				println!("[{}] Pong from {}", self.prefix, connection.id());
			}
			Message::Close { .. } => {
				println!("[{}] Close from {}", self.prefix, connection.id());
			}
		}
		Ok(message)
	}
}

/// IP filtering middleware
///
/// # Examples
///
/// ```
/// use reinhardt_websockets::middleware::{IpFilterMiddleware, ConnectionMiddleware, ConnectionContext};
///
/// # tokio_test::block_on(async {
/// let middleware = IpFilterMiddleware::whitelist(vec!["192.168.1.1".to_string()]);
/// let mut context = ConnectionContext::new("192.168.1.1".to_string());
///
/// assert!(middleware.on_connect(&mut context).await.is_ok());
///
/// let mut blocked_context = ConnectionContext::new("10.0.0.1".to_string());
/// assert!(middleware.on_connect(&mut blocked_context).await.is_err());
/// # });
/// ```
pub struct IpFilterMiddleware {
	allowed_ips: Vec<String>,
	blocked_ips: Vec<String>,
	mode: IpFilterMode,
}

#[derive(Debug, Clone, Copy)]
enum IpFilterMode {
	Whitelist,
	Blacklist,
}

impl IpFilterMiddleware {
	/// Create a whitelist-based filter
	pub fn whitelist(allowed_ips: Vec<String>) -> Self {
		Self {
			allowed_ips,
			blocked_ips: Vec::new(),
			mode: IpFilterMode::Whitelist,
		}
	}

	/// Create a blacklist-based filter
	pub fn blacklist(blocked_ips: Vec<String>) -> Self {
		Self {
			allowed_ips: Vec::new(),
			blocked_ips,
			mode: IpFilterMode::Blacklist,
		}
	}
}

#[async_trait]
impl ConnectionMiddleware for IpFilterMiddleware {
	async fn on_connect(&self, context: &mut ConnectionContext) -> MiddlewareResult<()> {
		match self.mode {
			IpFilterMode::Whitelist => {
				if self.allowed_ips.contains(&context.ip) {
					Ok(())
				} else {
					Err(MiddlewareError::ConnectionRejected(format!(
						"IP not in whitelist: {}",
						context.ip
					)))
				}
			}
			IpFilterMode::Blacklist => {
				if self.blocked_ips.contains(&context.ip) {
					Err(MiddlewareError::ConnectionRejected(format!(
						"IP is blacklisted: {}",
						context.ip
					)))
				} else {
					Ok(())
				}
			}
		}
	}

	async fn on_disconnect(&self, _connection: &Arc<WebSocketConnection>) -> MiddlewareResult<()> {
		Ok(())
	}
}

/// Middleware that validates WebSocket connection Origin headers.
///
/// Checks the `Origin` header against a list of allowed origins to prevent
/// cross-site WebSocket hijacking attacks (CSWSH).
///
/// # Examples
///
/// ```
/// use reinhardt_websockets::OriginValidationMiddleware;
///
/// let middleware = OriginValidationMiddleware::new(vec![
///     "https://example.com".to_string(),
///     "https://app.example.com".to_string(),
/// ]);
/// ```
pub struct OriginValidationMiddleware {
	allowed_origins: Vec<String>,
	allow_missing_origin: bool,
}

impl OriginValidationMiddleware {
	/// Create a new Origin validation middleware with specified allowed origins
	pub fn new(allowed_origins: Vec<String>) -> Self {
		Self {
			allowed_origins,
			allow_missing_origin: false,
		}
	}

	/// Allow connections that don't include an Origin header
	pub fn allow_missing_origin(mut self) -> Self {
		self.allow_missing_origin = true;
		self
	}

	/// Allow all origins (disables validation)
	pub fn allow_all() -> Self {
		Self {
			allowed_origins: Vec::new(),
			allow_missing_origin: true,
		}
	}

	/// Check if this middleware is in allow-all mode
	fn is_allow_all(&self) -> bool {
		self.allowed_origins.is_empty() && self.allow_missing_origin
	}
}

#[async_trait]
impl ConnectionMiddleware for OriginValidationMiddleware {
	async fn on_connect(&self, context: &mut ConnectionContext) -> MiddlewareResult<()> {
		if self.is_allow_all() {
			return Ok(());
		}

		let origin = context
			.headers
			.get("Origin")
			.or_else(|| context.headers.get("origin"))
			.cloned();

		match origin {
			None => {
				if self.allow_missing_origin {
					Ok(())
				} else {
					Err(MiddlewareError::ConnectionRejected(
						"Missing Origin header".to_string(),
					))
				}
			}
			Some(origin_value) => {
				if self.allowed_origins.contains(&origin_value) {
					Ok(())
				} else {
					Err(MiddlewareError::ConnectionRejected(format!(
						"Origin not allowed: {}",
						origin_value
					)))
				}
			}
		}
	}

	async fn on_disconnect(&self, _connection: &Arc<WebSocketConnection>) -> MiddlewareResult<()> {
		Ok(())
	}
}

/// Message size limit middleware
///
/// # Examples
///
/// ```
/// use reinhardt_websockets::middleware::{MessageSizeLimitMiddleware, MessageMiddleware};
/// use reinhardt_websockets::{Message, WebSocketConnection};
/// use tokio::sync::mpsc;
/// use std::sync::Arc;
///
/// # tokio_test::block_on(async {
/// let middleware = MessageSizeLimitMiddleware::new(100);
///
/// let (tx, _rx) = mpsc::unbounded_channel();
/// let conn = Arc::new(WebSocketConnection::new("test".to_string(), tx));
///
/// let small_msg = Message::text("Small".to_string());
/// assert!(middleware.on_message(&conn, small_msg).await.is_ok());
///
/// let large_msg = Message::text("x".repeat(200));
/// assert!(middleware.on_message(&conn, large_msg).await.is_err());
/// # });
/// ```
pub struct MessageSizeLimitMiddleware {
	max_size: usize,
}

impl MessageSizeLimitMiddleware {
	/// Create a new message size limit middleware
	pub fn new(max_size: usize) -> Self {
		Self { max_size }
	}
}

#[async_trait]
impl MessageMiddleware for MessageSizeLimitMiddleware {
	async fn on_message(
		&self,
		_connection: &Arc<WebSocketConnection>,
		message: Message,
	) -> MiddlewareResult<Message> {
		let size = match &message {
			Message::Text { data } => data.len(),
			Message::Binary { data } => data.len(),
			_ => 0,
		};

		if size > self.max_size {
			Err(MiddlewareError::MessageRejected(format!(
				"Message size {} exceeds limit {}",
				size, self.max_size
			)))
		} else {
			Ok(message)
		}
	}
}

/// Middleware chain for composing multiple middlewares
///
/// # Examples
///
/// ```
/// use reinhardt_websockets::middleware::{
///     MiddlewareChain, LoggingMiddleware, ConnectionContext, ConnectionMiddleware
/// };
///
/// # tokio_test::block_on(async {
/// let mut chain = MiddlewareChain::new();
/// chain.add_connection_middleware(Box::new(LoggingMiddleware::new("WS".to_string())));
///
/// let mut context = ConnectionContext::new("192.168.1.1".to_string());
/// assert!(chain.process_connect(&mut context).await.is_ok());
/// # });
/// ```
pub struct MiddlewareChain {
	connection_middlewares: Vec<Box<dyn ConnectionMiddleware>>,
	message_middlewares: Vec<Box<dyn MessageMiddleware>>,
}

impl MiddlewareChain {
	/// Create a new middleware chain
	pub fn new() -> Self {
		Self {
			connection_middlewares: Vec::new(),
			message_middlewares: Vec::new(),
		}
	}

	/// Add a connection middleware to the chain
	pub fn add_connection_middleware(&mut self, middleware: Box<dyn ConnectionMiddleware>) {
		self.connection_middlewares.push(middleware);
	}

	/// Add a message middleware to the chain
	pub fn add_message_middleware(&mut self, middleware: Box<dyn MessageMiddleware>) {
		self.message_middlewares.push(middleware);
	}

	/// Process connection through all middlewares
	pub async fn process_connect(&self, context: &mut ConnectionContext) -> MiddlewareResult<()> {
		for middleware in &self.connection_middlewares {
			middleware.on_connect(context).await?;
		}
		Ok(())
	}

	/// Process disconnection through all middlewares
	pub async fn process_disconnect(
		&self,
		connection: &Arc<WebSocketConnection>,
	) -> MiddlewareResult<()> {
		for middleware in &self.connection_middlewares {
			middleware.on_disconnect(connection).await?;
		}
		Ok(())
	}

	/// Process message through all middlewares
	pub async fn process_message(
		&self,
		connection: &Arc<WebSocketConnection>,
		mut message: Message,
	) -> MiddlewareResult<Message> {
		for middleware in &self.message_middlewares {
			message = middleware.on_message(connection, message).await?;
		}
		Ok(message)
	}
}

impl Default for MiddlewareChain {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use tokio::sync::mpsc;

	#[rstest]
	#[tokio::test]
	async fn test_connection_context() {
		// Arrange / Act
		let context = ConnectionContext::new("192.168.1.1".to_string())
			.with_header("User-Agent".to_string(), "Test".to_string())
			.with_metadata("session_id".to_string(), "abc123".to_string());

		// Assert
		assert_eq!(context.ip, "192.168.1.1");
		assert_eq!(context.headers.get("User-Agent").unwrap(), "Test");
		assert_eq!(context.metadata.get("session_id").unwrap(), "abc123");
	}

	#[rstest]
	#[tokio::test]
	async fn test_logging_middleware_connect() {
		// Arrange
		let middleware = LoggingMiddleware::new("Test".to_string());
		let mut context = ConnectionContext::new("192.168.1.1".to_string());

		// Act
		let result = middleware.on_connect(&mut context).await;

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_logging_middleware_message() {
		// Arrange
		let middleware = LoggingMiddleware::new("Test".to_string());
		let (tx, _rx) = mpsc::unbounded_channel();
		let conn = Arc::new(WebSocketConnection::new("test".to_string(), tx));
		let msg = Message::text("Hello".to_string());

		// Act
		let result = middleware.on_message(&conn, msg).await;

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_ip_filter_whitelist_allowed() {
		// Arrange
		let middleware = IpFilterMiddleware::whitelist(vec!["192.168.1.1".to_string()]);
		let mut context = ConnectionContext::new("192.168.1.1".to_string());

		// Act
		let result = middleware.on_connect(&mut context).await;

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_ip_filter_whitelist_blocked() {
		// Arrange
		let middleware = IpFilterMiddleware::whitelist(vec!["192.168.1.1".to_string()]);
		let mut context = ConnectionContext::new("10.0.0.1".to_string());

		// Act
		let result = middleware.on_connect(&mut context).await;

		// Assert
		assert!(result.is_err());
		assert!(matches!(
			result.unwrap_err(),
			MiddlewareError::ConnectionRejected(_)
		));
	}

	#[rstest]
	#[tokio::test]
	async fn test_ip_filter_blacklist_allowed() {
		// Arrange
		let middleware = IpFilterMiddleware::blacklist(vec!["10.0.0.1".to_string()]);
		let mut context = ConnectionContext::new("192.168.1.1".to_string());

		// Act
		let result = middleware.on_connect(&mut context).await;

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_ip_filter_blacklist_blocked() {
		// Arrange
		let middleware = IpFilterMiddleware::blacklist(vec!["10.0.0.1".to_string()]);
		let mut context = ConnectionContext::new("10.0.0.1".to_string());

		// Act
		let result = middleware.on_connect(&mut context).await;

		// Assert
		assert!(result.is_err());
	}

	#[rstest]
	#[tokio::test]
	async fn test_message_size_limit_within_limit() {
		// Arrange
		let middleware = MessageSizeLimitMiddleware::new(100);
		let (tx, _rx) = mpsc::unbounded_channel();
		let conn = Arc::new(WebSocketConnection::new("test".to_string(), tx));
		let msg = Message::text("Small message".to_string());

		// Act
		let result = middleware.on_message(&conn, msg).await;

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_message_size_limit_exceeds_limit() {
		// Arrange
		let middleware = MessageSizeLimitMiddleware::new(10);
		let (tx, _rx) = mpsc::unbounded_channel();
		let conn = Arc::new(WebSocketConnection::new("test".to_string(), tx));
		let msg = Message::text("This is a very long message".to_string());

		// Act
		let result = middleware.on_message(&conn, msg).await;

		// Assert
		assert!(result.is_err());
		assert!(matches!(
			result.unwrap_err(),
			MiddlewareError::MessageRejected(_)
		));
	}

	#[rstest]
	#[tokio::test]
	async fn test_middleware_chain_connect() {
		// Arrange
		let mut chain = MiddlewareChain::new();
		chain.add_connection_middleware(Box::new(LoggingMiddleware::new("WS".to_string())));
		let mut context = ConnectionContext::new("192.168.1.1".to_string());

		// Act
		let result = chain.process_connect(&mut context).await;

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_middleware_chain_message() {
		// Arrange
		let mut chain = MiddlewareChain::new();
		chain.add_message_middleware(Box::new(MessageSizeLimitMiddleware::new(100)));
		let (tx, _rx) = mpsc::unbounded_channel();
		let conn = Arc::new(WebSocketConnection::new("test".to_string(), tx));
		let msg = Message::text("Test".to_string());

		// Act
		let result = chain.process_message(&conn, msg).await;

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_middleware_chain_rejection() {
		// Arrange
		let mut chain = MiddlewareChain::new();
		chain.add_connection_middleware(Box::new(IpFilterMiddleware::whitelist(vec![
			"192.168.1.1".to_string(),
		])));
		let mut context = ConnectionContext::new("10.0.0.1".to_string());

		// Act
		let result = chain.process_connect(&mut context).await;

		// Assert
		assert!(result.is_err());
	}

	#[rstest]
	#[tokio::test]
	async fn test_origin_validation_allowed_origin_accepted() {
		// Arrange
		let middleware = OriginValidationMiddleware::new(vec!["https://example.com".to_string()]);
		let mut context = ConnectionContext::new("192.168.1.1".to_string())
			.with_header("Origin".to_string(), "https://example.com".to_string());

		// Act
		let result = middleware.on_connect(&mut context).await;

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_origin_validation_rejected_origin() {
		// Arrange
		let middleware = OriginValidationMiddleware::new(vec!["https://example.com".to_string()]);
		let mut context = ConnectionContext::new("192.168.1.1".to_string())
			.with_header("Origin".to_string(), "https://evil.com".to_string());

		// Act
		let result = middleware.on_connect(&mut context).await;

		// Assert
		assert!(result.is_err());
		assert!(matches!(
			result.unwrap_err(),
			MiddlewareError::ConnectionRejected(_)
		));
	}

	#[rstest]
	#[tokio::test]
	async fn test_origin_validation_missing_origin_rejected() {
		// Arrange
		let middleware = OriginValidationMiddleware::new(vec!["https://example.com".to_string()]);
		let mut context = ConnectionContext::new("192.168.1.1".to_string());

		// Act
		let result = middleware.on_connect(&mut context).await;

		// Assert
		assert!(result.is_err());
		assert!(matches!(
			result.unwrap_err(),
			MiddlewareError::ConnectionRejected(_)
		));
	}

	#[rstest]
	#[tokio::test]
	async fn test_origin_validation_missing_origin_allowed() {
		// Arrange
		let middleware = OriginValidationMiddleware::new(vec!["https://example.com".to_string()])
			.allow_missing_origin();
		let mut context = ConnectionContext::new("192.168.1.1".to_string());

		// Act
		let result = middleware.on_connect(&mut context).await;

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_origin_validation_allow_all() {
		// Arrange
		let middleware = OriginValidationMiddleware::allow_all();
		let mut context = ConnectionContext::new("192.168.1.1".to_string())
			.with_header("Origin".to_string(), "https://anything.com".to_string());

		// Act
		let result = middleware.on_connect(&mut context).await;

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_origin_validation_multiple_origins() {
		// Arrange
		let middleware = OriginValidationMiddleware::new(vec![
			"https://example.com".to_string(),
			"https://app.example.com".to_string(),
			"https://staging.example.com".to_string(),
		]);

		// Act / Assert - each allowed origin should be accepted
		let mut ctx1 = ConnectionContext::new("10.0.0.1".to_string())
			.with_header("Origin".to_string(), "https://example.com".to_string());
		assert!(middleware.on_connect(&mut ctx1).await.is_ok());

		let mut ctx2 = ConnectionContext::new("10.0.0.2".to_string())
			.with_header("Origin".to_string(), "https://app.example.com".to_string());
		assert!(middleware.on_connect(&mut ctx2).await.is_ok());

		let mut ctx3 = ConnectionContext::new("10.0.0.3".to_string()).with_header(
			"Origin".to_string(),
			"https://staging.example.com".to_string(),
		);
		assert!(middleware.on_connect(&mut ctx3).await.is_ok());

		// Unlisted origin should be rejected
		let mut ctx4 = ConnectionContext::new("10.0.0.4".to_string())
			.with_header("Origin".to_string(), "https://other.com".to_string());
		assert!(middleware.on_connect(&mut ctx4).await.is_err());
	}

	#[rstest]
	#[tokio::test]
	async fn test_origin_validation_case_sensitive() {
		// Arrange - Origin matching is case-sensitive per RFC 6454
		let middleware = OriginValidationMiddleware::new(vec!["https://example.com".to_string()]);

		// Act / Assert - exact case matches
		let mut ctx_match = ConnectionContext::new("10.0.0.1".to_string())
			.with_header("Origin".to_string(), "https://example.com".to_string());
		assert!(middleware.on_connect(&mut ctx_match).await.is_ok());

		// Different case should be rejected
		let mut ctx_upper = ConnectionContext::new("10.0.0.2".to_string())
			.with_header("Origin".to_string(), "https://Example.com".to_string());
		assert!(middleware.on_connect(&mut ctx_upper).await.is_err());

		let mut ctx_all_upper = ConnectionContext::new("10.0.0.3".to_string())
			.with_header("Origin".to_string(), "HTTPS://EXAMPLE.COM".to_string());
		assert!(middleware.on_connect(&mut ctx_all_upper).await.is_err());
	}
}
