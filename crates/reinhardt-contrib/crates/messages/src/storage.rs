//! Message storage backends

pub mod cookie;
pub mod fallback;
pub mod memory;
pub mod session;

use crate::message::Message;

/// Trait for message storage backends
pub trait MessageStorage: Send + Sync {
	/// Add a message
	fn add(&mut self, message: Message);

	/// Get all messages and clear storage
	fn get_all(&mut self) -> Vec<Message>;

	/// Get messages without clearing
	fn peek(&self) -> Vec<Message>;

	/// Clear all messages
	fn clear(&mut self);
}

pub use cookie::CookieStorage;
pub use fallback::FallbackStorage;
pub use memory::MemoryStorage;
pub use session::SessionStorage;
