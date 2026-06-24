//! Client-side routing for the {{ app_name }} SPA.
//!
//! Route names are defined by this app's route-backed client components and
//! registered only in client builds.

{% if is_workspace == "true" %}use crate::client::components;{% else %}use crate::apps::{{ app_name }}::client::components;{% endif %}
use reinhardt::ClientRouter;

pub fn client_url_patterns() -> ClientRouter {
    ClientRouter::new().component(components::placeholder::placeholder)
}

pub fn reverse(name: &str, params: &[(&str, &str)]) -> String {
    client_url_patterns()
        .reverse(name, params)
        .unwrap_or_else(|error| panic!("failed to reverse {{ app_name }} client route `{name}`: {error}"))
}
