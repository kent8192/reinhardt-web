//! Message framework for Reinhardt
//!
//! This crate provides Django-style messaging for displaying one-time
//! notifications to users.
//!
//! ## Features
//!
//! - **Message Storage**: Multiple storage backends (Memory, Cookie, Session, Fallback)
//! - **Template Context**: Easy integration with template engines
//! - **Message Filtering**: Filter messages by level or tags
//! - **Type-Safe Levels**: Predefined message levels (Debug, Info, Success, Warning, Error)
//!
//! ## Note
//!
//! HTTP middleware integration (`MessagesMiddleware`) has been moved to
//! `reinhardt-http` crate to prevent circular dependencies.
//!
//! ## Example
//!
//! ```rust,no_run
//! use reinhardt_messages::{Message, middleware::MessagesContainer, storage::MemoryStorage};
//!
//! // Create a message container
//! let container = MessagesContainer::new(vec![]);
//!
//! // Add messages
//! container.add(Message::success("Operation completed successfully!"));
//! container.add(Message::warning("Please review your settings"));
//!
//! // Get messages
//! let messages = container.get_messages();
//! ```

pub mod context;
pub mod levels;
pub mod message;
pub mod middleware;
pub mod safedata;
pub mod storage;
pub mod utils;

pub use context::MessagesContext;
pub use levels::Level;
pub use message::{Message, MessageConfig};
pub use middleware::MessagesContainer;
pub use safedata::SafeData;
pub use storage::{CookieStorage, FallbackStorage, MemoryStorage, MessageStorage, SessionStorage};

/// Re-export commonly used types
pub mod prelude {
	pub use crate::context::*;
	pub use crate::levels::*;
	pub use crate::message::*;
	pub use crate::middleware::*;
	pub use crate::safedata::*;
	pub use crate::storage::*;
	pub use crate::utils::*;
}
