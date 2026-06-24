//! Client-side routing for the {{ app_name }} SPA.
//!
//! Route names are namespaced under `{{ app_name }}` (e.g.
//! `{{ app_name }}:placeholder`) when `src/config/urls.rs` mounts this app's
//! client router with `UnifiedRouter::with_namespace("{{ app_name }}")`.

use reinhardt::ClientRouter;

#[cfg(client)]
{% if is_workspace == "true" %}use crate::client::components;{% else %}use crate::apps::{{ app_name }}::client::components;{% endif %}

pub fn client_url_patterns() -> ClientRouter {
    #[cfg(client)]
    {
        ClientRouter::new().component(components::placeholder::placeholder)
    }
    #[cfg(not(client))]
    {
        ClientRouter::new()
    }
}

pub fn reverse(name: &str, params: &[(&str, &str)]) -> String {
    client_url_patterns()
        .reverse(name, params)
        .unwrap_or_else(|error| panic!("failed to reverse {{ app_name }} client route `{name}`: {error}"))
}
