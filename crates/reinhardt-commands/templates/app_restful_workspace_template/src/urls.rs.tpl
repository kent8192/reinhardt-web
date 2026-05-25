//! URL configuration for {{ app_name }} app (RESTful)

use reinhardt::ServerRouter;

pub fn server_url_patterns() -> ServerRouter {
    ServerRouter::new()
}
