//! URL configuration for {{ app_name }} app (RESTful)

use reinhardt::UnifiedRouter;

pub fn url_patterns() -> UnifiedRouter {
    let router = UnifiedRouter::builder()
        .build();

    // Add your API endpoint patterns here
    // Example:
    // router.register_viewset("mymodel", MyModelViewSet::new());

    router
}
