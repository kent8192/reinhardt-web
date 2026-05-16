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
//! Client routers for each app are declared via `#[url_patterns(..., mode = client)]`
//! in `src/apps/<app>/urls/client_router.rs`. They do not appear in this
//! (server-side) router. `#[url_patterns]` only wraps the per-app function
//! and emits client URL resolver modules; it does not register the router
//! itself with the launcher. Pass the desired `client_url_patterns()` (or
//! a combined router) explicitly to `ClientLauncher::router_client(...)` in
//! `src/client/lib.rs`.

use reinhardt::prelude::*;
use reinhardt::routes;

// `standalone` mode is required because per-app `urls.rs` aggregators only
// declare `pub mod server_urls;` and `pub mod client_router;` — the
// `url_resolvers` / `ws_urls` items emitted by `#[url_patterns]` therefore
// live at `crate::apps::<app>::urls::server_urls::url_resolvers`, one level
// deeper than the non-standalone `#[routes]` lookup expects. `standalone`
// disables that auto-aggregation, so per-app `server_url_patterns()`
// functions are NOT mounted into this router automatically: only `#[routes]`
// performs inventory registration; `#[url_patterns]` just wraps each
// per-app function and emits resolver modules. Any server endpoint added
// under `src/apps/<app>/urls/server_urls.rs` must be wired in below — for
// example via `router.mount(...)` or `router.server(|s| s.server_fn(...))`.
// This matches the pattern used by `examples-tutorial-basis/src/config/urls.rs`.
#[routes(standalone)]
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
