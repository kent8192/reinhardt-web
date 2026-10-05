//! URL configuration for {{ project_name }} project (RESTful)
//!
//! The `routes` function defines all URL patterns for this project.

use reinhardt::prelude::*;
use reinhardt::routes;

#[routes]
pub fn routes() -> UnifiedRouter {
    let router = UnifiedRouter::new();

    // Define endpoints and ViewSets in your app's views module before using them.
    // Builder methods consume the router: assign the result or chain calls.
    //
    // Register a typed endpoint:
    // ```rust
    // let router = router.endpoint(crate::apps::users::views::health_check);
    // ```
    //
    // Mount a generated REST app's ServerRouter with a child namespace:
    // ```rust
    // let router = router.mount(
    //     "/api/v1/",
    //     crate::apps::users::urls::server_url_patterns().with_namespace("api_v1"),
    // );
    // ```
    //
    // For an app returning UnifiedRouter, use mount_unified instead:
    // ```rust
    // let router = router.mount_unified(
    //     "/api/v2/",
    //     crate::apps::api_v2::urls::routes().with_namespace("api_v2"),
    // );
    // ```
    //
    // Register a native ViewSet (the standard preset includes viewset-routing;
    // enable that feature explicitly when using a custom feature set):
    // ```rust
    // let router = router.server(|server| {
    //     server.viewset("/users", crate::apps::users::views::UserViewSet::new())
    // });
    // ```

    router
}
