//! URL configuration for {{ project_name }} project (RESTful)
//!
//! The `url_patterns` routes URLs to handlers.

use reinhardt_routers::Router;

pub fn url_patterns() -> Router {
    let mut router = Router::new();

    // Add your API endpoint patterns here
    // Example:
    // router.include("/api/v1/", api_v1_urls());
    // router.get("/health", health_check);

    router
}
