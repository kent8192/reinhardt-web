//! Server-Side Rendering (SSR) Infrastructure
//!
//! This module provides SSR capabilities for reinhardt-pages, using a component-based
//! approach instead of traditional template engines. It renders Component trees to
//! HTML strings on the server side, with support for hydration markers and state serialization.
//!
//! ## Features
//!
//! - **Component-based SSR**: Render any Component to HTML
//! - **Hydration Markers**: Automatically embed markers for client-side hydration
//! - **State Serialization**: Serialize reactive state for client restoration
//! - **Layout Support**: Wrap rendered content in HTML layouts
//!
//! ## Usage
//!
//! ```ignore
//! use reinhardt_pages::ssr::{SsrRenderer, SsrOptions};
//! use reinhardt_pages::component::Component;
//!
//! // Simple rendering
//! let html = SsrRenderer::render(&my_component);
//!
//! // With options
//! let html = SsrRenderer::with_options(SsrOptions::default())
//!     .render(&my_component);
//!
//! // Full page rendering
//! let page = SsrRenderer::render_page(
//!     &my_component,
//!     "My Page Title",
//!     Some("<!DOCTYPE html>...")
//! );
//! ```

mod markers;
mod renderer;
mod state;

pub use markers::{HYDRATION_ATTR_ID, HYDRATION_ATTR_PROPS, HydrationMarker};
pub use renderer::{SsrOptions, SsrRenderer};
pub use state::{SsrState, StateEntry};
