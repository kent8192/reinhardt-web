//! Server-Side Rendering (SSR) Infrastructure
//!
//! This module provides SSR capabilities for reinhardt-pages, using a component-based
//! approach instead of traditional template engines. It renders Component trees to
//! HTML streams or buffered strings on the server side, with support for
//! hydration markers, state serialization, async resource resolution, and
//! Suspense replacement chunks.
//!
//! ## Features
//!
//! - **Component-based SSR**: Render any Component to HTML
//! - **Hydration Markers**: Automatically embed markers for client-side hydration
//! - **State Serialization**: Serialize reactive state for client restoration
//! - **Layout Support**: Wrap rendered content in HTML layouts
//! - **Route Preparation**: Resolve matched route loaders before rendering
//!
//! ## Usage
//!
//! ```ignore
//! use reinhardt_pages::ssr::{SsrRenderer, SsrOptions};
//! use reinhardt_pages::component::Component;
//!
//! // Simple buffered rendering
//! let mut renderer = SsrRenderer::new();
//! let html = renderer.render(&my_component).await;
//!
//! // With options
//! let mut renderer = SsrRenderer::with_options(SsrOptions::default());
//! let html = renderer.render(&my_component).await;
//!
//! // Full page streaming
//! let mut renderer = SsrRenderer::new();
//! let stream = renderer.render_page(&my_component).await;
//!
//! // Full page buffered string
//! let html = renderer.render_page_to_string(&my_component).await;
//! ```
//!
//! For a matched `ClientRouter`, [`SsrRenderer::render_route_to_string`] adds
//! entry-blocking route-loader preparation and emits the successful values in
//! the normal [`SsrState`] hydration payload.

mod markers;
#[cfg(native)]
mod renderer;
#[cfg(native)]
pub(crate) mod resource_context;
#[cfg(native)]
mod route;
mod state;
#[cfg(native)]
mod stream;

pub use markers::{
	HYDRATION_ATTR_ID, HYDRATION_ATTR_PROPS, HydrationMarker, HydrationMarkerBuilder,
	HydrationStrategy,
};
#[cfg(native)]
pub use renderer::{IntoSsrRendererConfig, SsrOptions, SsrRenderConfig, SsrRenderer};
#[cfg(native)]
pub use route::SsrRouteOutput;
pub use state::{SsrState, StateEntry};
#[cfg(native)]
pub use stream::{SsrChunk, SsrStream};
