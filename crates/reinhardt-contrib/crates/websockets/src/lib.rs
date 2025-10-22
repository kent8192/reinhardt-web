//! WebSocket support for Reinhardt framework

pub mod connection;
pub mod handler;
pub mod room;

pub use connection::{Message, WebSocketConnection, WebSocketError, WebSocketResult};
pub use handler::{RoomManager, WebSocketHandler};
pub use room::{Room, RoomError, RoomResult};

#[cfg(test)]
mod tests;
