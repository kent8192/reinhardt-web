//! MSW-style network-level request mocking for Reinhardt tests.
//!
//! On WASM targets, [`MockServiceWorker`] overrides `window.fetch()` in the
//! browser environment. On native targets, [`MockServiceWorker`] starts a
//! loopback HTTP mock server and exposes its base URL via
//! [`MockServiceWorker::url`].
//!
//! Native MSW is explicit endpoint injection, not transparent interception:
//! pass `worker.url()` into the HTTP client, SDK endpoint, or service
//! configuration under test.
//!
//! # WASM example
//!
//! ```rust,ignore
//! use reinhardt_test::msw::*;
//!
//! #[wasm_bindgen_test]
//! async fn test_component() {
//!     let worker = MockServiceWorker::new();
//!     worker.handle(rest::get("/api/users").respond(MockResponse::json(vec![1, 2])));
//!     worker.start().await;
//!     // Render component that calls window.fetch().
//!     worker.calls_to("/api/users").assert_called();
//! }
//! ```
//!
//! # Native example
//!
//! ```rust,ignore
//! use reinhardt_test::msw::*;
//!
//! #[tokio::test]
//! async fn test_http_client() {
//!     let worker = MockServiceWorker::new();
//!     worker.handle(rest::get("/api/users").respond(MockResponse::json(vec![1, 2])));
//!     worker.start().await;
//!
//!     let endpoint = worker.url();
//!     // Pass `endpoint` into the code under test.
//!
//!     worker.calls_to("/api/users").assert_called();
//!     worker.stop().await;
//! }
//! ```

// On native builds, worker.rs and interceptor.rs are behind #[cfg(wasm)],
// making handler/recorder/context types appear unused in lib mode.
// They ARE exercised in WASM builds and native unit tests.
#![allow(dead_code, clippy::type_complexity)]

mod context;
mod error;
pub(crate) mod handler;
mod interceptor;
mod matcher;
#[cfg(native)]
mod native;
pub(crate) mod recorder;
mod response;
pub mod rest;
mod state;

pub use context::TestContext;
pub use error::MswError;
pub use handler::InterceptedRequest;
pub use matcher::{Segment, UrlMatcher};
pub use recorder::{CallQuery, RecordedRequest, ServerFnCallQuery};
pub use response::MockResponse;

#[cfg(wasm)]
mod worker;
#[cfg(wasm)]
pub use worker::{MockServiceWorker, UnhandledPolicy};

#[cfg(native)]
pub use native::{MockServiceWorker, UnhandledPolicy};
