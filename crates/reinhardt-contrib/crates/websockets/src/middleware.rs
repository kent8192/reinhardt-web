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
	use tokio::sync::mpsc;

	#[tokio::test]
	async fn test_connection_context() {
		let context = ConnectionContext::new("192.168.1.1".to_string())
			.with_header("User-Agent".to_string(), "Test".to_string())
			.with_metadata("session_id".to_string(), "abc123".to_string());

		assert_eq!(context.ip, "192.168.1.1");
		assert_eq!(context.headers.get("User-Agent").unwrap(), "Test");
		assert_eq!(context.metadata.get("session_id").unwrap(), "abc123");
	}

	#[tokio::test]
	async fn test_logging_middleware_connect() {
		let middleware = LoggingMiddleware::new("Test".to_string());
		let mut context = ConnectionContext::new("192.168.1.1".to_string());

		assert!(middleware.on_connect(&mut context).await.is_ok());
	}

	#[tokio::test]
	async fn test_logging_middleware_message() {
		let middleware = LoggingMiddleware::new("Test".to_string());
		let (tx, _rx) = mpsc::unbounded_channel();
		let conn = Arc::new(WebSocketConnection::new("test".to_string(), tx));
		let msg = Message::text("Hello".to_string());

		let result = middleware.on_message(&conn, msg).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_ip_filter_whitelist_allowed() {
		let middleware = IpFilterMiddleware::whitelist(vec!["192.168.1.1".to_string()]);
		let mut context = ConnectionContext::new("192.168.1.1".to_string());

		assert!(middleware.on_connect(&mut context).await.is_ok());
	}

	#[tokio::test]
	async fn test_ip_filter_whitelist_blocked() {
		let middleware = IpFilterMiddleware::whitelist(vec!["192.168.1.1".to_string()]);
		let mut context = ConnectionContext::new("10.0.0.1".to_string());

		let result = middleware.on_connect(&mut context).await;
		assert!(result.is_err());
		assert!(matches!(
			result.unwrap_err(),
			MiddlewareError::ConnectionRejected(_)
		));
	}

	#[tokio::test]
	async fn test_ip_filter_blacklist_allowed() {
		let middleware = IpFilterMiddleware::blacklist(vec!["10.0.0.1".to_string()]);
		let mut context = ConnectionContext::new("192.168.1.1".to_string());

		assert!(middleware.on_connect(&mut context).await.is_ok());
	}

	#[tokio::test]
	async fn test_ip_filter_blacklist_blocked() {
		let middleware = IpFilterMiddleware::blacklist(vec!["10.0.0.1".to_string()]);
		let mut context = ConnectionContext::new("10.0.0.1".to_string());

		let result = middleware.on_connect(&mut context).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_message_size_limit_within_limit() {
		let middleware = MessageSizeLimitMiddleware::new(100);
		let (tx, _rx) = mpsc::unbounded_channel();
		let conn = Arc::new(WebSocketConnection::new("test".to_string(), tx));
		let msg = Message::text("Small message".to_string());

		assert!(middleware.on_message(&conn, msg).await.is_ok());
	}

	#[tokio::test]
	async fn test_message_size_limit_exceeds_limit() {
		let middleware = MessageSizeLimitMiddleware::new(10);
		let (tx, _rx) = mpsc::unbounded_channel();
		let conn = Arc::new(WebSocketConnection::new("test".to_string(), tx));
		let msg = Message::text("This is a very long message".to_string());

		let result = middleware.on_message(&conn, msg).await;
		assert!(result.is_err());
		assert!(matches!(
			result.unwrap_err(),
			MiddlewareError::MessageRejected(_)
		));
	}

	#[tokio::test]
	async fn test_middleware_chain_connect() {
		let mut chain = MiddlewareChain::new();
		chain.add_connection_middleware(Box::new(LoggingMiddleware::new("WS".to_string())));

		let mut context = ConnectionContext::new("192.168.1.1".to_string());
		assert!(chain.process_connect(&mut context).await.is_ok());
	}

	#[tokio::test]
	async fn test_middleware_chain_message() {
		let mut chain = MiddlewareChain::new();
		chain.add_message_middleware(Box::new(MessageSizeLimitMiddleware::new(100)));

		let (tx, _rx) = mpsc::unbounded_channel();
		let conn = Arc::new(WebSocketConnection::new("test".to_string(), tx));
		let msg = Message::text("Test".to_string());

		assert!(chain.process_message(&conn, msg).await.is_ok());
	}

	#[tokio::test]
	async fn test_middleware_chain_rejection() {
		let mut chain = MiddlewareChain::new();
		chain.add_connection_middleware(Box::new(IpFilterMiddleware::whitelist(vec![
			"192.168.1.1".to_string(),
		])));

		let mut context = ConnectionContext::new("10.0.0.1".to_string());
		assert!(chain.process_connect(&mut context).await.is_err());
	}
}
