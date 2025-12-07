//! Views module for dm app (RESTful)
//!
//! Re-exports all views from the views/ directory

pub mod messages;
pub mod rooms;

pub use messages::{delete_message, get_message, list_messages, mark_as_read, send_message};
pub use rooms::{create_room, delete_room, get_room, list_rooms};
