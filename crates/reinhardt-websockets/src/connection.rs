use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::mpsc;

#[derive(Debug, thiserror::Error)]
pub enum WebSocketError {
	#[error("Connection error: {0}")]
	Connection(String),
	#[error("Send error: {0}")]
	Send(String),
	#[error("Receive error: {0}")]
	Receive(String),
	#[error("Protocol error: {0}")]
	Protocol(String),
	#[error("Internal error: {0}")]
	Internal(String),
}

pub type WebSocketResult<T> = Result<T, WebSocketError>;

/// WebSocket message types
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Message {
	Text { data: String },
	Binary { data: Vec<u8> },
	Ping,
	Pong,
	Close { code: u16, reason: String },
}

impl Message {
	/// Creates a new text message.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::Message;
	///
	/// let msg = Message::text("Hello, World!".to_string());
	/// match msg {
	///     Message::Text { data } => assert_eq!(data, "Hello, World!"),
	///     _ => panic!("Expected text message"),
	/// }
	/// ```
	pub fn text(data: String) -> Self {
		Self::Text { data }
	}
	/// Creates a new binary message.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::Message;
	///
	/// let data = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello" in bytes
	/// let msg = Message::binary(data.clone());
	/// match msg {
	///     Message::Binary { data: d } => assert_eq!(d, data),
	///     _ => panic!("Expected binary message"),
	/// }
	/// ```
	pub fn binary(data: Vec<u8>) -> Self {
		Self::Binary { data }
	}
	/// Creates a text message containing JSON-serialized data.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::Message;
	/// use serde::Serialize;
	///
	/// #[derive(Serialize)]
	/// struct User {
	///     name: String,
	///     age: u32,
	/// }
	///
	/// let user = User {
	///     name: "Alice".to_string(),
	///     age: 30,
	/// };
	///
	/// let msg = Message::json(&user).unwrap();
	/// match msg {
	///     Message::Text { data } => {
	///         assert!(data.contains("Alice"));
	///         assert!(data.contains("30"));
	///     },
	///     _ => panic!("Expected text message"),
	/// }
	/// ```
	pub fn json<T: serde::Serialize>(data: &T) -> WebSocketResult<Self> {
		let json =
			serde_json::to_string(data).map_err(|e| WebSocketError::Protocol(e.to_string()))?;
		Ok(Self::text(json))
	}
	/// Parses the message content as JSON into the target type.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::Message;
	/// use serde::Deserialize;
	///
	/// #[derive(Deserialize, Debug, PartialEq)]
	/// struct User {
	///     name: String,
	///     age: u32,
	/// }
	///
	/// let msg = Message::text(r#"{"name":"Bob","age":25}"#.to_string());
	/// let user: User = msg.parse_json().unwrap();
	/// assert_eq!(user.name, "Bob");
	/// assert_eq!(user.age, 25);
	/// ```
	pub fn parse_json<T: serde::de::DeserializeOwned>(&self) -> WebSocketResult<T> {
		match self {
			Message::Text { data } => {
				serde_json::from_str(data).map_err(|e| WebSocketError::Protocol(e.to_string()))
			}
			_ => Err(WebSocketError::Protocol("Not a text message".to_string())),
		}
	}
}

/// WebSocket connection
pub struct WebSocketConnection {
	id: String,
	tx: mpsc::UnboundedSender<Message>,
	closed: Arc<RwLock<bool>>,
	/// Subprotocol (negotiated protocol during WebSocket handshake)
	subprotocol: Option<String>,
}

impl WebSocketConnection {
	/// Creates a new WebSocket connection with the given ID and sender.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::{WebSocketConnection, Message};
	/// use tokio::sync::mpsc;
	///
	/// let (tx, _rx) = mpsc::unbounded_channel();
	/// let conn = WebSocketConnection::new("connection_1".to_string(), tx);
	/// assert_eq!(conn.id(), "connection_1");
	/// ```
	pub fn new(id: String, tx: mpsc::UnboundedSender<Message>) -> Self {
		Self {
			id,
			tx,
			closed: Arc::new(RwLock::new(false)),
			subprotocol: None,
		}
	}

	/// Creates a new WebSocket connection with the given ID, sender, and subprotocol.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::{WebSocketConnection, Message};
	/// use tokio::sync::mpsc;
	///
	/// let (tx, _rx) = mpsc::unbounded_channel();
	/// let conn = WebSocketConnection::with_subprotocol(
	///     "connection_1".to_string(),
	///     tx,
	///     Some("chat".to_string())
	/// );
	/// assert_eq!(conn.id(), "connection_1");
	/// assert_eq!(conn.subprotocol(), Some("chat"));
	/// ```
	pub fn with_subprotocol(
		id: String,
		tx: mpsc::UnboundedSender<Message>,
		subprotocol: Option<String>,
	) -> Self {
		Self {
			id,
			tx,
			closed: Arc::new(RwLock::new(false)),
			subprotocol,
		}
	}

	/// Gets the negotiated subprotocol, if any.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::{WebSocketConnection, Message};
	/// use tokio::sync::mpsc;
	///
	/// let (tx, _rx) = mpsc::unbounded_channel();
	/// let conn = WebSocketConnection::with_subprotocol(
	///     "test".to_string(),
	///     tx,
	///     Some("chat".to_string())
	/// );
	/// assert_eq!(conn.subprotocol(), Some("chat"));
	/// ```
	pub fn subprotocol(&self) -> Option<&str> {
		self.subprotocol.as_deref()
	}
	/// Gets the connection ID.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::{WebSocketConnection, Message};
	/// use tokio::sync::mpsc;
	///
	/// let (tx, _rx) = mpsc::unbounded_channel();
	/// let conn = WebSocketConnection::new("test_id".to_string(), tx);
	/// assert_eq!(conn.id(), "test_id");
	/// ```
	pub fn id(&self) -> &str {
		&self.id
	}
	/// Sends a message through the WebSocket connection.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::{WebSocketConnection, Message};
	/// use tokio::sync::mpsc;
	///
	/// # tokio_test::block_on(async {
	/// let (tx, mut rx) = mpsc::unbounded_channel();
	/// let conn = WebSocketConnection::new("test".to_string(), tx);
	///
	/// let message = Message::text("Hello".to_string());
	/// conn.send(message).await.unwrap();
	///
	/// let received = rx.recv().await.unwrap();
	/// assert!(matches!(received, Message::Text { .. }));
	/// # });
	/// ```
	pub async fn send(&self, message: Message) -> WebSocketResult<()> {
		if *self.closed.read().await {
			return Err(WebSocketError::Send("Connection closed".to_string()));
		}

		self.tx
			.send(message)
			.map_err(|e| WebSocketError::Send(e.to_string()))
	}
	/// Sends a text message through the WebSocket connection.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::{WebSocketConnection, Message};
	/// use tokio::sync::mpsc;
	///
	/// # tokio_test::block_on(async {
	/// let (tx, mut rx) = mpsc::unbounded_channel();
	/// let conn = WebSocketConnection::new("test".to_string(), tx);
	///
	/// conn.send_text("Hello World".to_string()).await.unwrap();
	///
	/// let received = rx.recv().await.unwrap();
	/// match received {
	///     Message::Text { data } => assert_eq!(data, "Hello World"),
	///     _ => panic!("Expected text message"),
	/// }
	/// # });
	/// ```
	pub async fn send_text(&self, text: String) -> WebSocketResult<()> {
		self.send(Message::text(text)).await
	}
	/// Sends a binary message through the WebSocket connection.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::{WebSocketConnection, Message};
	/// use tokio::sync::mpsc;
	///
	/// # tokio_test::block_on(async {
	/// let (tx, mut rx) = mpsc::unbounded_channel();
	/// let conn = WebSocketConnection::new("test".to_string(), tx);
	///
	/// let binary_data = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello"
	/// conn.send_binary(binary_data.clone()).await.unwrap();
	///
	/// let received = rx.recv().await.unwrap();
	/// match received {
	///     Message::Binary { data } => assert_eq!(data, binary_data),
	///     _ => panic!("Expected binary message"),
	/// }
	/// # });
	/// ```
	pub async fn send_binary(&self, data: Vec<u8>) -> WebSocketResult<()> {
		self.send(Message::binary(data)).await
	}
	/// Sends a JSON message through the WebSocket connection.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::{WebSocketConnection, Message};
	/// use tokio::sync::mpsc;
	/// use serde::Serialize;
	///
	/// #[derive(Serialize)]
	/// struct User {
	///     name: String,
	///     age: u32,
	/// }
	///
	/// # tokio_test::block_on(async {
	/// let (tx, mut rx) = mpsc::unbounded_channel();
	/// let conn = WebSocketConnection::new("test".to_string(), tx);
	///
	/// let user = User { name: "Alice".to_string(), age: 30 };
	/// conn.send_json(&user).await.unwrap();
	///
	/// let received = rx.recv().await.unwrap();
	/// match received {
	///     Message::Text { data } => assert!(data.contains("Alice")),
	///     _ => panic!("Expected text message"),
	/// }
	/// # });
	/// ```
	pub async fn send_json<T: serde::Serialize>(&self, data: &T) -> WebSocketResult<()> {
		let message = Message::json(data)?;
		self.send(message).await
	}
	/// Closes the WebSocket connection.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::{WebSocketConnection, Message};
	/// use tokio::sync::mpsc;
	///
	/// # tokio_test::block_on(async {
	/// let (tx, mut rx) = mpsc::unbounded_channel();
	/// let conn = WebSocketConnection::new("test".to_string(), tx);
	///
	/// conn.close().await.unwrap();
	/// assert!(conn.is_closed().await);
	/// # });
	/// ```
	pub async fn close(&self) -> WebSocketResult<()> {
		// First send the close message
		let result = self
			.tx
			.send(Message::Close {
				code: 1000,
				reason: "Normal closure".to_string(),
			})
			.map_err(|e| WebSocketError::Send(e.to_string()));

		// Then mark as closed
		*self.closed.write().await = true;

		result
	}
	/// Checks if the WebSocket connection is closed.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::{WebSocketConnection, Message};
	/// use tokio::sync::mpsc;
	///
	/// # tokio_test::block_on(async {
	/// let (tx, _rx) = mpsc::unbounded_channel();
	/// let conn = WebSocketConnection::new("test".to_string(), tx);
	///
	/// assert!(!conn.is_closed().await);
	/// # });
	/// ```
	pub async fn is_closed(&self) -> bool {
		*self.closed.read().await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_message_text() {
		let msg = Message::text("Hello".to_string());
		match msg {
			Message::Text { data } => assert_eq!(data, "Hello"),
			_ => panic!("Expected text message"),
		}
	}

	#[rstest]
	fn test_message_json() {
		#[derive(serde::Serialize)]
		struct TestData {
			value: i32,
		}

		let data = TestData { value: 42 };
		let msg = Message::json(&data).unwrap();

		match msg {
			Message::Text { data } => assert!(data.contains("42")),
			_ => panic!("Expected text message"),
		}
	}

	#[rstest]
	#[tokio::test]
	async fn test_connection_send() {
		let (tx, mut rx) = mpsc::unbounded_channel();
		let conn = WebSocketConnection::new("test".to_string(), tx);

		conn.send_text("Hello".to_string()).await.unwrap();

		let received = rx.recv().await.unwrap();
		match received {
			Message::Text { data } => assert_eq!(data, "Hello"),
			_ => panic!("Expected text message"),
		}
	}
}
