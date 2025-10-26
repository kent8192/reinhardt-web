//! Message framework for Reinhardt
//!
//! This crate provides Django-style messaging for displaying one-time
//! notifications to users.
//!
//! ## Features
//!
//! - **Message Storage**: Multiple storage backends (Memory, Cookie, Session, Fallback)
//! - **Middleware Integration**: Automatic message loading and saving during request lifecycle
//! - **Template Context**: Easy integration with template engines
//! - **Message Filtering**: Filter messages by level or tags
//! - **Type-Safe Levels**: Predefined message levels (Debug, Info, Success, Warning, Error)
//!
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_messages::{Message, middleware::MessagesMiddleware, storage::MemoryStorage};
//!
//! // Create middleware with memory storage
//! let storage = MemoryStorage::new();
//! let middleware = MessagesMiddleware::new(storage);
//!
//! // Add messages during request processing
//! container.add(Message::success("Operation completed successfully!"));
//! container.add(Message::warning("Please review your settings"));
//! ```

pub mod context;
pub mod levels;
pub mod message;
pub mod middleware;
pub mod safedata;
pub mod storage;
pub mod utils;

pub use context::{add_message, get_messages_context, MessagesContext};
pub use levels::Level;
pub use message::{Message, MessageConfig};
pub use middleware::{MessagesContainer, MessagesMiddleware};
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
