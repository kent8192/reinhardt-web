//! Reinhardt Pages - WASM-based Frontend Framework
//!
//! A Django-inspired frontend framework for Reinhardt that preserves the benefits of
//! Django templates while leveraging WebAssembly for modern interactivity.
//!
//! ## Features
//!
//! - **Fine-grained Reactivity**: Leptos/Solid.js-style Signal system with automatic dependency tracking
//! - **Hybrid Rendering**: SSR + Client-side Hydration for optimal performance and SEO
//! - **Django-like API**: Familiar patterns for Reinhardt developers
//! - **Low-level Only**: Built on wasm-bindgen, web-sys, and js-sys (no high-level framework dependencies)
//! - **Security First**: Built-in CSRF protection, XSS prevention, and session management
//!
//! ## Architecture
//!
//! This framework consists of several key modules:
//!
//! - [`reactive`]: Fine-grained reactivity system (Signal, Effect, Memo)
//! - [`dom`]: DOM abstraction layer
//! - [`builder`]: HTML element builder API
//! - [`component`]: Component system with IntoView trait
//! - [`form`]: Django Form integration
//! - [`csrf`]: CSRF protection
//! - [`auth`]: Authentication integration
//! - [`api`]: API client with Django QuerySet-like interface
//! - [`server_fn`]: Server Functions (RPC)
//! - [`ssr`]: Server-side rendering
//! - [`hydration`]: Client-side hydration
//! - [`router`]: Client-side routing (reinhardt-urls compatible)
//!
//! ## Example
//!
//! ```ignore
//! use reinhardt_pages::prelude::*;
//!
//!
//! #[component]
//! fn Counter() -> impl IntoView {
//!     let count = Signal::new(0);
//!
//!     view! {
//!         <div>
//!             <p>"Count: " {count}</p>
//!             <button on:click=move |_| count.update(|n| *n += 1)>
//!                 "Increment"
//!             </button>
//!         </div>
//!     }
//! }
//! ```

#![warn(missing_docs)]
#![cfg_attr(target_arch = "wasm32", no_std)]

// Core modules
pub mod builder;
pub mod dom;
pub mod reactive;

// Component system
pub mod component;

// Form and security
pub mod auth;
pub mod csrf;
pub mod form;

// API and communication
pub mod api;
pub mod server_fn;

// Server-side rendering
pub mod ssr;

// Client-side hydration
pub mod hydration;

// Client-side routing
pub mod router;

// Re-export commonly used types
pub use api::{ApiModel, ApiQuerySet, Filter, FilterOp};
pub use auth::{AuthData, AuthError, AuthState, auth_state};
pub use builder::{
	attributes::{AriaAttributes, BooleanAttributes},
	html::{
		a, button, div, form, h1, h2, h3, img, input, li, ol, option, p, select, span, textarea, ul,
	},
};
pub use component::{Component, ElementView, IntoView, Props, View};
pub use csrf::{CsrfManager, get_csrf_token};
pub use dom::{Document, Element, EventHandle, EventType, document};
pub use form::{FormBinding, FormComponent};
pub use hydration::{HydrationContext, HydrationError, hydrate};
pub use reactive::{Effect, Memo, Resource, ResourceState, Signal};
#[cfg(target_arch = "wasm32")]
pub use reactive::{create_resource, create_resource_with_deps};
pub use router::{Link, PathPattern, Route, Router, RouterOutlet};
pub use server_fn::{ServerFn, ServerFnError};
pub use ssr::{SsrOptions, SsrRenderer, SsrState};
