//! URL configuration for {{ project_name }} project (RESTful)
//!
//! The `url_patterns` routes URLs to handlers.

use reinhardt::prelude::*;
use std::sync::Arc;

pub fn url_patterns() -> Arc<UnifiedRouter> {
    let router = UnifiedRouter::builder()
        .build();

    // Add your API endpoint patterns here
    // Example:
    // router.include_router("/api/v1/", api_v1_router, Some("api_v1".to_string()));
    // router.add_function_route("/health", Method::GET, health_check);
    //
    // Or register ViewSets:
    // router.register_viewset("users", user_viewset);

    Arc::new(router)
}
