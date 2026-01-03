//! Component System for reinhardt-pages
//!
//! This module provides a component abstraction for building reusable UI elements.
//! Components can be rendered both on the server (SSR) and client (WASM).
//!
//! ## Features
//!
//! - **IntoView trait**: Convert any type into a renderable View
//! - **Component trait**: Define reusable UI components
//! - **View enum**: Unified representation of DOM elements, text, and fragments
//! - **Props system**: Type-safe component properties
//!
//! ## Usage
//!
//! ```ignore
//! use reinhardt_pages::component::{Component, IntoView, View};
//!
//! // Simple component as a function
//! fn greeting(name: &str) -> impl IntoView {
//!     div()
//!         .class("greeting")
//!         .child(p().text(&format!("Hello, {}!", name)).build())
//!         .build()
//! }
//!
//! // Using the component
//! let view = greeting("World").into_view();
//! let html = view.render_to_string();
//! ```

mod head;
mod into_view;
mod props;
mod r#trait;

pub use head::{Head, LinkTag, MetaTag, ScriptTag, StyleTag};
#[cfg(not(target_arch = "wasm32"))]
pub use into_view::DummyEvent;
pub use into_view::{ElementView, IntoView, MountError, View, ViewEventHandler};
pub use props::Props;
pub use r#trait::Component;
