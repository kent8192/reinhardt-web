//! Server Function Registry Types
//!
//! This module provides the type definitions used by the `#[server_fn]` macro
//! for server function registration.
//!
//! ## Usage
//!
//! These types are primarily used internally by the macro system.
//! For registering server functions with a router, use `ServerFnRouterExt`:
//!
//! ```ignore
//! use reinhardt::pages::server_fn::ServerFnRouterExt;
//! use crate::server_fn::{login, logout};
//!
//! let router = UnifiedRouter::new()
//!     .server_fn(login)
//!     .server_fn(logout);
//! ```

use bytes::Bytes;
use reinhardt_http::Request;
use std::future::Future;
use std::pin::Pin;

/// Handler function type for server functions.
///
/// This is the signature of the generated `__server_fn_static_wrapper_{name}` functions.
/// Returns `Result<Bytes, Bytes>` where:
/// - `Ok(bytes)` is the serialized successful response
/// - `Err(bytes)` is the serialized error response
pub type ServerFnHandler =
	fn(Request) -> Pin<Box<dyn Future<Output = Result<Bytes, Bytes>> + Send>>;

/// Server Function route registration entry.
///
/// This struct holds the metadata for a server function, used by
/// `ServerFnRegistration` to provide registration information.
pub struct ServerFnRoute {
	/// The HTTP path for this server function (e.g., "/api/server_fn/login")
	pub path: &'static str,
	/// The handler function that processes requests
	pub handler: ServerFnHandler,
	/// The name of the server function (for debugging)
	pub name: &'static str,
}
