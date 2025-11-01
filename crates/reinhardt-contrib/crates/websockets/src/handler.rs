//! WebSocket handler

use crate::connection::Message;
use crate::connection::WebSocketResult;

/// WebSocket handler trait
pub trait WebSocketHandler: Send + Sync {
	/// Handle incoming message
	fn on_message(
		&self,
		message: Message,
	) -> impl std::future::Future<Output = WebSocketResult<()>> + Send;

	/// Handle connection open
	fn on_connect(&self) -> impl std::future::Future<Output = WebSocketResult<()>> + Send;

	/// Handle connection close
	fn on_disconnect(&self) -> impl std::future::Future<Output = WebSocketResult<()>> + Send;
}
