//! URL patterns for {{ app_name }}

use reinhardt_routers::UnifiedRouter;

pub fn url_patterns() -> UnifiedRouter {
    let router = UnifiedRouter::builder()
        .build();

    // Add URL patterns here
    // Example:
    // router.add_function_route("/", Method::GET, views::index);

    router
}
