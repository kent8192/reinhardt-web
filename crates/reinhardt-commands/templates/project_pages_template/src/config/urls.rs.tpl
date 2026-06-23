//! URL configuration for {{ project_name }} project (Pages)
//!
//! The `routes` function defines all URL patterns for this project.
//!
//! ## Aggregating app routers
//!
//! Each app owns its server-function marker registrations in
//! `src/apps/<app>/urls/server_router.rs` and exposes them through the
//! target-neutral aggregate in `src/apps/<app>/urls.rs`. After running
//! `reinhardt-admin startapp <name> --with-pages`, aggregate the app-level
//! router functions here:
//!
//! ```rust,ignore
//! let router = UnifiedRouter::new().mount_unified(
//!     "/",
//!     UnifiedRouter::new()
//!         .server(|s| s.mount("/", crate::apps::<name>::urls::server_url_patterns()))
//!         .client(|_| crate::apps::<name>::urls::client_url_patterns()),
//! );
//! ```
//!
//! ## Registering client routers
//!
//! Client route tables for each app are declared in
//! `src/apps/<app>/urls/client_router.rs`. Aggregate them here through each
//! app's `urls.rs`; the WASM launcher collects the route table from the
//! `#[routes]` registration.

use reinhardt::prelude::*;
use reinhardt::routes;

#[routes]
pub fn routes() -> UnifiedRouter {
    let router = UnifiedRouter::new();

    // Add your API endpoint patterns here
    // Example:
    // router.include_router("/api/v1/", api_v1_router, Some("api_v1".to_string()));
    // router.endpoint(health_check);
    //
    // Or register ViewSets:
    // router.register_viewset("users", user_viewset);
    //
    // Add Pages app routers here. Do not import each app's server functions
    // in this project-level file; each app's `urls` module owns that list.
    //
    // let router = router.mount_unified(
    //     "/",
    //     UnifiedRouter::new()
    //         .server(|s| s.mount("/", crate::apps::<your_app>::urls::server_url_patterns()))
    //         .client(|_| crate::apps::<your_app>::urls::client_url_patterns()),
    // );

    router
}
