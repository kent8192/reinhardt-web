//! URL configuration for {{ app_name }} app (RESTful)

use reinhardt_routers::Router;

pub fn url_patterns() -> Router {
    let mut router = Router::new();

    // Add your API endpoint patterns here
    // Example:
    // router.register("/mymodel/", MyModelViewSet::new());

    router
}
