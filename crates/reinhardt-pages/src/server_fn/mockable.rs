//! `MockableServerFn` trait for MSW-style testing.
//!
//! This trait exposes the call-site argument and response types of a
//! `#[server_fn]` for type-safe mocking in WASM tests. It is automatically
//! implemented by the `#[server_fn]` macro when the `msw` feature is
//! enabled.
//!
//! Constants common to every target (`PATH`, `NAME`, `CODEC`,
//! `INJECTED_PARAMS`) are inherited from the cross-target supertrait
//! [`ServerFnMetadata`](super::metadata::ServerFnMetadata) — they are not
//! duplicated here.

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::metadata::ServerFnMetadata;

/// Exposes server function metadata for MSW-style mocking.
///
/// Automatically implemented by `#[server_fn]` macro when the `msw` feature
/// is enabled. Adds the `Args` / `Response` associated types on top of the
/// cross-target [`ServerFnMetadata`] supertrait, enabling type-safe mock
/// handler registration.
pub trait MockableServerFn: ServerFnMetadata {
	/// Client-side argument struct (excludes `#[inject]` parameters).
	type Args: Serialize + DeserializeOwned + 'static;

	/// Success response type (the `Ok` variant of the function's return type).
	type Response: Serialize + DeserializeOwned + 'static;
}
