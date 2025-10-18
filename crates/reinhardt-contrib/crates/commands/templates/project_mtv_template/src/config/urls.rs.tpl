//! URL configuration for {{ project_name }}

use reinhardt_routers::Router;

/// Define URL patterns for the application
pub fn url_patterns() -> Router {
    let mut router = Router::new();

    // Add your URL patterns here
    // Example:
    // router.add_route("/", home_view);
    // router.add_route("/about", about_view);

    router
}
