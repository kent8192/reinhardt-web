//! Server-side URL configuration for the {{ app_name }} application.
//!
//! `#[url_patterns(InstalledApp::{{ app_name }}, mode = server)]` wraps the
//! function below and emits a `url_resolvers` submodule namespaced under
//! `InstalledApp::{{ app_name }}`; it does not by itself mount the router
//! into the project. Because `config/urls.rs` uses `#[routes(standalone)]`,
//! per-app routers are NOT aggregated automatically — endpoints added here
//! become reachable only after `config/urls.rs` wires this function in
//! (e.g. via `router.mount("/{{ app_name }}/", server_url_patterns())` or
//! `router.server(|s| s.server_fn(...))` for individual server functions).
//!
//! # Placeholder note
//!
//! The freshly generated function returns an empty `ServerRouter`. Register
//! views/endpoints here:
//!
//! ```rust,ignore
{% if is_workspace == "true" %}//! use crate::views;{% else %}//! use crate::apps::{{ app_name }}::views;{% endif %}
//!
//! ServerRouter::new()
//!     .endpoint(views::index)
//! ```

use reinhardt::ServerRouter;
use reinhardt::url_patterns;

{% if is_workspace == "true" %}use {{ project_crate_name }}::config::apps::InstalledApp;{% else %}use crate::config::apps::InstalledApp;{% endif %}

#[url_patterns(InstalledApp::{{ app_name }}, mode = server)]
pub fn server_url_patterns() -> ServerRouter {
	ServerRouter::new()
}
