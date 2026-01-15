//! Template and rendering module.
//!
//! This module provides access to server-side rendering (SSR).
//!
//! ## Server-Side Rendering
//!
//! For component-based server-side rendering, use `reinhardt-pages`:
//!
//! ```rust,ignore
//! use reinhardt::pages::{SsrRenderer, SsrOptions, Component};
//! use reinhardt::pages::component::PageElement;
//!
//! let mut renderer = SsrRenderer::with_options(
//!     SsrOptions::new()
//!         .title("My Page")
//!         .css("/styles.css")
//! );
//! let html = renderer.render_page(&my_component);
//! ```
//!
//! ## Migration from Tera Templates
//!
//! The Tera-based templating system has been removed in favor of
//! SSR (Server-Side Rendering) via `reinhardt-pages` for component-based rendering.
//!
//! See the respective module documentation for migration guidance.

// Re-export SSR functionality from reinhardt-pages when templates feature is enabled
#[cfg(feature = "templates")]
pub use reinhardt_pages::{
	component::{Component, IntoPage, Page, PageElement},
	hydration::HydrationContext,
	ssr::{SsrOptions, SsrRenderer, SsrState},
};
