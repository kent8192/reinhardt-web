//! URL configuration for {{ project_name }} project (Pages)
//!
//! The `routes` function defines all URL patterns for this project.
//!
//! ## Registering server functions
//!
//! Server functions are NOT auto-registered. After running
//! `reinhardt-admin startapp <name> --with-pages`, append the new app's
//! markers manually:
//!
//! ```rust,ignore
//! use crate::apps::<name>::server_fn::{some_fn, other_fn};
//!
//! let router = UnifiedRouter::new().server(|s| s
//!     .server_fn(some_fn::marker)
//!     .server_fn(other_fn::marker)
//! );
//! ```
//!
//! ## Registering client routers
//!
//! Client routers for each app are registered via `#[url_patterns(..., mode = client)]`
//! in `src/apps/<app>/urls/client_router.rs` and discovered automatically by
//! `ClientLauncher::router_client(...)`. They do not appear in this
//! (server-side) router.

use reinhardt::prelude::*;
use reinhardt::routes;

#[routes]
pub fn routes() -> UnifiedRouter {
    let router = UnifiedRouter::new();

    // Add your API endpoint patterns here
    // Example:
    // router.include_router("/api/v1/", api_v1_router, Some("api_v1".to_string()));
    // router.function("/health", Method::GET, health_check);
    //
    // Or register ViewSets:
    // router.register_viewset("users", user_viewset);
    //
    // For server-function endpoints in `mode = unified` builder chains, use
    // `s.server_fn(marker)`. Since reinhardt-web v0.1.0-rc.28,
    // `ServerRouterStub` carries a no-op `server_fn` stub, so the same
    // builder chain compiles unchanged on `wasm32-unknown-unknown` — no
    // `#[cfg(native)]` workaround is required at call sites:
    //
    // router.server(|s| s.server_fn(my_server_fn_marker));

    router
}
