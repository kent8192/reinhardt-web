//! Target-neutral page entry points for the {{ app_name }} application.
//!
//! Keep route entry points here so native builds can aggregate the client
//! route table for metadata and reverse lookups. Browser-only UI details
//! belong in `client/*.rs` and are called behind `#[cfg(client)]`.

use reinhardt::pages::component::Page;

#[cfg(client)]
{% if is_workspace == "true" %}use {{ project_crate_name }}::client::components::nav::with_nav;
#[cfg(client)]
use crate::client::components::placeholder;{% else %}use crate::apps::{{ app_name }}::client::components::placeholder;
#[cfg(client)]
use crate::client::components::nav::with_nav;{% endif %}

#[reinhardt::pages::component("/{{ app_name }}/", "placeholder")]
pub fn placeholder_page() -> Page {
    #[cfg(client)]
    {
        with_nav(placeholder())
    }
    #[cfg(not(client))]
    {
        Page::Empty
    }
}
