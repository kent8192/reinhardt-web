//! Placeholder route-backed component for the {{ app_name }} application.
//!
//! Replace this module with real components when the app gets its first page.

use reinhardt::pages::component::Page;
use reinhardt::pages::page;

#[cfg(client)]
{% if is_workspace == "true" %}use {{ project_crate_name }}::client::components::nav::with_nav;{% else %}use crate::client::components::nav::with_nav;{% endif %}

#[reinhardt::pages::component("/{{ app_name }}/", "placeholder")]
pub fn placeholder() -> Page {
    with_nav(page!(|| {
        div {
            class: "placeholder",
            "{{ app_name }} placeholder component"
        }
    })())
}
