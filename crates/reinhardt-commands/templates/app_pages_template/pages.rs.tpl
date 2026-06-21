//! Target-neutral page entry points for the {{ app_name }} application.
//!
//! Keep route entry points here so native builds can aggregate the client
//! route table for metadata and reverse lookups. Browser-only UI details
//! belong in `client/*.rs` and are called behind `#[cfg(client)]`.

use reinhardt::pages::component::Page;

#[cfg(client)]
{% if is_workspace == "true" %}use {{ project_crate_name }}::client::components::nav::with_nav;{% else %}use crate::client::components::nav::with_nav;{% endif %}

pub fn placeholder_page() -> Page {
    #[cfg(client)]
    {
        with_nav(super::client::components::placeholder())
    }
    #[cfg(not(client))]
    {
        Page::Empty
    }
}
