//! WASM shim for the `urls` module (Issue #4161).
//!
//! Provides the namespace structure that `#[url_patterns]` and downstream
//! wasm SPAs reference (`reinhardt::urls::prelude::UnifiedRouter`,
//! `reinhardt::urls::proxy`). The real `reinhardt-urls` crate is wasm-safe,
//! but its `prelude` is gated `#[cfg(all(feature = "routers", native))]`.
//!
//! When the `client-router` feature is enabled (the realistic configuration
//! for wasm consumers that use `mode = unified`), this re-exports the real
//! wasm-side `UnifiedRouter` from `reinhardt_urls::routers`. That type
//! provides the correct closure signatures
//! (`server: FnOnce(ServerRouterStub) -> ServerRouterStub`,
//! `client: FnOnce(ClientRouter) -> ClientRouter`) so user-supplied bodies
//! such as `.client(|c| c.named_route(...))` type-check on wasm.
//!
//! Without `client-router`, an inert stub is exposed so that the path
//! resolves; user bodies that invoke `.server`/`.client` on the stub are
//! expected to be no-ops in that minimal configuration.
//!
//! Re-exported at `crate::urls` from `src/lib.rs` (wasm-only) to preserve the
//! canonical user-facing path.

/// Wasm-side stub mirroring `reinhardt_urls::prelude`.
pub mod prelude {
	// Real wasm `UnifiedRouter` (with `ServerRouterStub` / `ClientRouter`
	// builder closures). Available when `client-router` is enabled.
	#[cfg(feature = "client-router")]
	pub use reinhardt_urls::routers::unified_router::ServerRouterStub;
	#[cfg(feature = "client-router")]
	pub use reinhardt_urls::routers::{ClientRouter, UnifiedRouter};

	// Inert fallback for wasm builds without `client-router`. Closures
	// receive a stub parameter typed to match the real wasm API shape so
	// that no-argument forms (`.server(|_| _)`) still type-check.
	#[cfg(not(feature = "client-router"))]
	pub use stub::*;

	#[cfg(not(feature = "client-router"))]
	mod stub {
		/// Empty stand-in for `reinhardt_urls::routers::ServerRouterStub`.
		pub struct ServerRouterStub;
		/// Empty stand-in for `reinhardt_urls::routers::client_router::ClientRouter`.
		pub struct ClientRouter;

		pub struct UnifiedRouter {
			_private: (),
		}

		impl UnifiedRouter {
			pub fn new() -> Self {
				Self { _private: () }
			}

			pub fn with_namespace(self, _namespace: impl Into<String>) -> Self {
				self
			}

			pub fn server<F>(self, _f: F) -> Self
			where
				F: FnOnce(ServerRouterStub) -> ServerRouterStub,
			{
				self
			}

			pub fn client<F>(self, _f: F) -> Self
			where
				F: FnOnce(ClientRouter) -> ClientRouter,
			{
				self
			}
		}

		impl Default for UnifiedRouter {
			fn default() -> Self {
				Self::new()
			}
		}
	}
}

/// Wasm-side stub for the `proxy` submodule referenced by
/// `crate_paths::get_reinhardt_proxy_crate()`. Empty on wasm.
pub mod proxy {}
