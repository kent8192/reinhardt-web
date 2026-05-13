// The `User` trait is deprecated in favour of the new `#[model]`-based user macro system.
// This crate re-exports it for downstream compatibility during the transition period.
#![allow(deprecated)]

//! # Reinhardt
//!
//! A full-stack API framework for Rust, inspired by Django and Django REST Framework.
//!
//! Reinhardt provides a complete, batteries-included solution for building production-ready
//! REST APIs with Rust. It follows Rust's composition patterns instead of Python's inheritance
//! model, making full use of traits, generics, and zero-cost abstractions.
//!
//! ## Core Principles
//!
//! - **Composition over Inheritance**: Uses Rust's trait system for composable behavior
//! - **Type Safety**: Leverages Rust's type system for compile-time guarantees
//! - **Zero-Cost Abstractions**: High-level ergonomics without runtime overhead
//! - **Async-First**: Built on tokio and async/await from the ground up
//!
//! ## Feature Flags
//!
//! Reinhardt provides flexible feature flags to control compilation and reduce binary size.
//!
//! ### Presets
//!
//! - `minimal` - Core functionality only (routing, DI, params)
//! - `full` (default) - All features enabled
//! - `standard` - Balanced for most projects
//! - `api-only` - REST API without templates/forms
//! - `graphql-server` - GraphQL-focused setup
//! - `websocket-server` - WebSocket-centric setup
//! - `cli-tools` - CLI and background jobs
//! - `test-utils` - Testing utilities
//!
//! ### Fine-grained Control
//!
//! Fine-grained feature flags for precise control over included functionality:
//!
//! #### Authentication
//! - `auth-jwt` - JWT authentication
//! - `auth-session` - Session-based authentication
//! - `auth-oauth` - OAuth2 support
//! - `auth-token` - Token authentication
//!
//! #### Database Backends
//! - `db-postgres` - PostgreSQL support
//! - `db-mysql` - MySQL support
//! - `db-sqlite` - SQLite support
//! - `db-cockroachdb` - CockroachDB support (distributed transactions)
//!
//! #### Middleware
//! - `middleware-cors` - CORS (Cross-Origin Resource Sharing) middleware
//! - `middleware-compression` - Response compression (Gzip, Brotli)
//! - `middleware-security` - Security headers (HSTS, XSS Protection, etc.)
//! - `middleware-rate-limit` - Rate limiting and throttling
//!
//! See [Cargo.toml feature definitions](https://github.com/kent8192/reinhardt/blob/main/Cargo.toml) for detailed documentation.
//!
//! ## Facade Layout (Issue #4362)
//!
//! The crate is organised into layered facade boundaries to confine `#[cfg]`
//! boilerplate:
//!
//! - `exports` (internal) — public re-export aggregation (cross-target /
//!   native / wasm / macros), surfaced via `pub use exports::*;` at the
//!   crate root
//! - [`prelude`] — convenience re-exports for `use reinhardt::prelude::*;`
//! - `compat` (internal, wasm-only) — shims that re-route macro-generated
//!   paths like `::reinhardt::reinhardt_apps`, `::reinhardt::urls`, and
//!   `::reinhardt::WebSocketRouter` on browser-WASM targets where the
//!   corresponding native-only crates (or their feature-gated preludes)
//!   are not available
//!
//! The crate root performs `pub use exports::*;` so the historical public API
//! paths remain unchanged. Both `exports` and `compat` are intentionally
//! hidden from rustdoc and should not be referenced as
//! `reinhardt::exports::*` / `reinhardt::compat::*` from downstream code.

#![cfg_attr(docsrs, feature(doc_cfg))]

// --- Layered facade -------------------------------------------------------
//
// `exports` and `compat` stay `pub` so that re-exports such as
// `pub use exports::*;` and `pub use compat::apps as reinhardt_apps;` resolve
// without visibility downgrades, but they are flagged `#[doc(hidden)]` so the
// only supported public surface is the crate root (and `prelude`).

#[doc(hidden)]
pub mod exports;
pub mod prelude;

#[cfg(not(native))]
#[doc(hidden)]
pub mod compat;

pub use exports::*;

// On wasm, re-expose the compat shims at the canonical user-facing paths so
// macro-generated code referencing `::reinhardt::reinhardt_apps`,
// `::reinhardt::urls`, and `::reinhardt::WebSocketRouter` continues to resolve.
#[cfg(not(native))]
#[doc(hidden)]
pub use compat::apps as reinhardt_apps;

#[cfg(not(native))]
pub use compat::urls;

#[cfg(not(native))]
pub use compat::websockets::WebSocketRouter;

/// Glob re-export of every proc macro in `reinhardt-macros` under a stable
/// `::reinhardt::macros::*` namespace.
///
/// Used by macro-generated code (e.g. `#[derive(::reinhardt::macros::Model)]`).
/// Ungated on wasm per Issue #4161: macros are host-side and wasm-safe.
#[doc(hidden)]
pub mod macros {
	pub use reinhardt_macros::*;
}

// --- Source-file modules --------------------------------------------------
// Declarations for `src/<name>.rs` (and matching directories under
// `src/<name>/`). `#[cfg]` gates are preserved verbatim from the pre-refactor
// layout. `pub mod` declarations live here because module files must be
// declared relative to their parent module's source location.

// WASM-compatible modules (always available with the feature)
#[cfg(feature = "pages")]
pub mod pages;

#[cfg(feature = "streaming")]
pub mod streaming;

// Server-side only modules (NOT for WASM)
#[cfg(all(feature = "admin", native))]
pub mod admin;
#[cfg(all(feature = "core", native))]
pub mod apps;
#[cfg(all(feature = "commands", native))]
pub mod commands;
#[cfg(all(feature = "conf", native))]
pub mod conf;
#[cfg(all(feature = "core", native))]
pub mod core;
#[cfg(all(feature = "deeplink", native))]
pub mod deeplink;
#[cfg(all(feature = "dentdelion", native))]
pub mod dentdelion;
#[cfg(all(feature = "di", native))]
pub mod di;
#[cfg(all(feature = "dispatch", native))]
pub mod dispatch;
#[cfg(all(feature = "forms", native))]
pub mod forms;
#[cfg(all(feature = "graphql", native))]
pub mod graphql;
#[cfg(all(feature = "grpc", native))]
pub mod grpc;
#[cfg(native)]
pub mod http;
#[cfg(all(feature = "i18n", native))]
pub mod i18n;
#[cfg(all(feature = "mail", native))]
pub mod mail;
#[cfg(all(any(feature = "standard", feature = "middleware"), native))]
pub mod middleware;
#[cfg(all(feature = "rest", native))]
pub mod rest;
#[cfg(all(feature = "server", native))]
pub mod server;
#[cfg(all(feature = "shortcuts", native))]
pub mod shortcuts;
#[cfg(all(feature = "tasks", native))]
pub mod tasks;
#[cfg(all(feature = "templates", native))]
pub mod template;
#[cfg(all(feature = "test", native))]
pub mod test;
#[cfg(native)]
pub mod urls;
#[cfg(native)]
pub mod utils;
#[cfg(native)]
pub mod views;

/// SQL query builder module.
///
/// Re-exports [`reinhardt_query`] for building type-safe SQL queries.
/// Requires `database` feature.
#[cfg(all(feature = "database", native))]
pub mod query;

// --- Deprecated macro shim ------------------------------------------------

/// Re-export `flatten_imports` and provide a deprecated `define_views!` shim
/// for compatibility. `#[macro_export]` forces this macro to the crate root
/// regardless of where it is defined.
#[cfg(native)]
#[deprecated(
	since = "0.1.0-rc.16",
	note = "use `flatten_imports!` instead. `define_views!` will be removed in a future version."
)]
#[macro_export]
macro_rules! define_views {
    ($($tt:tt)*) => {
        $crate::flatten_imports!($($tt)*)
    };
}
