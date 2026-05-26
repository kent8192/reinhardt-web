//! URL configuration for {{ app_name }} app (RESTful)

use reinhardt::url_patterns;
use reinhardt::ServerRouter;

{% if is_workspace == "true" %}use {{ project_crate_name }}::config::apps::InstalledApp;{% else %}use crate::config::apps::InstalledApp;{% endif %}

#[url_patterns(InstalledApp::{{ app_name }}, mode = server)]
pub fn server_url_patterns() -> ServerRouter {
    ServerRouter::new()
}
