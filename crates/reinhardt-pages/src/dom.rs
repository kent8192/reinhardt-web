//! DOM Abstraction Layer
//!
//! This module provides a thin, safe wrapper around web-sys DOM APIs.
//! The design emphasizes RAII patterns for automatic resource cleanup
//! and integration with the reactive system.
//!
//! ## Architecture
//!
//! - **Element**: Wrapper around `web_sys::Element` with type-safe operations
//! - **Document**: Wrapper around `web_sys::Document` for DOM creation
//! - **EventHandle**: RAII wrapper for event listeners (Drop-based cleanup)
//!
//! ## RAII Pattern
//!
//! All resources (event listeners, etc.) are automatically cleaned up when dropped:
//!
//! ```ignore
//! {
//!     let element = document().create_element("div")?;
//!     let handle = element.add_event_listener("click", || {
//!         console::log_1(&"Clicked!".into());
//!     });
//!     // Event listener automatically removed when `handle` is dropped
//! }
//! ```
//!
//! ## Integration with Reactive System
//!
//! Elements can bind to Signals for automatic updates:
//!
//! ```ignore
//! use reinhardt_pages::{reactive::Signal, dom::Element};
//!
//! let count = Signal::new(0);
//! let element = document().create_element("div")?;
//!
//! // Reactive attribute: updates automatically when count changes
//! element.set_reactive_attribute("data-count", count.clone());
//! ```

pub mod document;
pub mod element;
pub mod event;

// Re-exports for convenience
pub use document::Document;
pub use element::{Element, EventHandle};
pub use event::EventType;

/// Get the global Document instance
///
/// This is a convenience function that returns a cached Document wrapper.
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::dom::document;
///
/// let doc = document();
/// let div = doc.create_element("div")?;
/// ```
pub fn document() -> Document {
	Document::global()
}
