//! HTML Builder API
//!
//! This module provides a fluent API for constructing HTML elements in a type-safe manner.
//!
//! ## Features
//!
//! - **Fluent API**: Chain method calls for readable element construction
//! - **Type-safe attributes**: Compile-time validation of attribute names
//! - **Event shortcuts**: Convenient methods like `.on_click()` for common events
//! - **Integration with Phase 1**: Works seamlessly with Signal/Effect system
//!
//! ## Example
//!
//! ```ignore
//! use reinhardt_pages::builder::html::div;
//! use reinhardt_pages::Signal;
//!
//! let count = Signal::new(0);
//!
//! let button = div()
//!     .class("counter")
//!     .child(
//!         button()
//!             .text("Increment")
//!             .on_click(move || count.update(|n| *n += 1))
//!             .build()
//!     )
//!     .build();
//! ```

pub mod attributes;
pub mod html;

// Re-exports for convenience
pub use attributes::*;
pub use html::*;
