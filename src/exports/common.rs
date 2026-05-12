//! Cross-target re-exports for `reinhardt`.
//!
//! Items in this module compile on both native and `wasm32-unknown-unknown`.
//! Per-item feature gates from the pre-refactor `src/lib.rs` are preserved
//! verbatim to keep the public API surface unchanged.

// Re-export `reinhardt_pages` as a wrapper module. Macro-generated code uses
// paths like `::reinhardt::reinhardt_pages::...`, so the wrapper provides the
// expected namespace.
#[cfg(feature = "pages")]
#[doc(hidden)]
pub mod reinhardt_pages {
	pub use reinhardt_pages::*;
}

// Re-export `reinhardt_types` for macro path resolution.
#[doc(hidden)]
pub mod reinhardt_types {
	// Public API surface glob re-export requires allowing unused imports and unreachable pub.
	#[allow(unused_imports, unreachable_pub)]
	pub use reinhardt_core::types::*;
}

// Re-export ApplyUpdate trait (cross-target).
pub use reinhardt_core::apply_update::ApplyUpdate;

// Re-export URL resolver trait (cross-target).
pub use reinhardt_urls::routers::ClientUrlResolver;

// Re-export client-router types (requires client-router feature).
// These types enable UnifiedRouter<V> with both .server() and .client() methods.
// The feature gate is target-agnostic: client-router compiles on native too.
#[cfg(feature = "client-router")]
pub use reinhardt_urls::routers::{
	ClientPathPattern, ClientRoute, ClientRouteMatch, ClientRouter, ClientUrlReverser, FromPath,
	HistoryState, NavigationType, ParamContext, SingleFromPath, UnifiedRouter,
	clear_client_reverser, get_client_reverser, register_client_reverser,
};

// Path extractor for client-side routing (separate from server-side Path from reinhardt-di).
#[cfg(feature = "client-router")]
pub use reinhardt_urls::routers::Path as ClientPath;
