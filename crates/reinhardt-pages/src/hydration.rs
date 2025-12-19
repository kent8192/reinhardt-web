//! Hydration Module for reinhardt-pages
//!
//! This module handles connecting client-side reactivity with SSR-rendered HTML.
//! During hydration, the client-side JavaScript/WASM code "takes over" the
//! pre-rendered HTML, attaching event listeners and reactive bindings.
//!
//! ## Features
//!
//! - **State Restoration**: Restore reactive state from embedded SSR data
//! - **Event Attachment**: Connect event handlers to existing DOM elements
//! - **DOM Reconciliation**: Verify SSR output matches expected component structure
//! - **Incremental Hydration**: Support partial page hydration (islands architecture)
//!
//! ## Usage
//!
//! ```ignore
//! use reinhardt_pages::hydration::hydrate_root;
//! use reinhardt_pages::component::Component;
//!
//! // Entry point for WASM hydration
//! #[wasm_bindgen(start)]
//! pub fn main() {
//!     hydrate_root::<MyApp>();
//! }
//! ```

mod events;
mod reconcile;
mod runtime;

pub use events::{EventBinding, EventRegistry, attach_event};
pub use reconcile::{ReconcileError, reconcile};
pub use runtime::{HydrationContext, HydrationError, hydrate, hydrate_root};
