//! Event System
//!
//! Provides type-safe event handling with automatic cleanup.
//!
//! ## Event Types
//!
//! This module re-exports the `EventType` enum from `reinhardt-types` to provide
//! type safety when working with DOM events.
//!
//! ## EventHandle
//!
//! The `EventHandle` struct (defined in `element.rs`) uses RAII to ensure
//! event listeners are automatically removed when the handle is dropped.
//!
//! ## Example
//!
//! ```ignore
//! use reinhardt_pages::dom::{Document, event::EventType};
//!
//! let doc = Document::global();
//! let button = doc.create_element("button")?;
//!
//! // Type-safe event handling
//! let handle = button.add_event_listener("click", || {
//!     console::log_1(&"Button clicked!".into());
//! });
//!
//! // handle is automatically cleaned up when dropped
//! ```

// Re-export EventType from reinhardt-types
pub use reinhardt_core::types::page::EventType;
