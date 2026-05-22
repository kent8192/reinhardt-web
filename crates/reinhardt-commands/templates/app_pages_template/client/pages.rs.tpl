//! Page-level views for the {{ app_name }} application.
//!
//! Each function returns a `Page` mounted by `urls/client_router.rs`. Pages
//! typically wrap a per-app component (`super::components::...`) with the
//! project-wide site shell (`crate::client::components::nav::with_nav`)
//! so every routed page gets the same header.

use reinhardt::pages::component::Page;

use crate::client::components::nav::with_nav;

// -----------------------------------------------------------------------------
// PLACEHOLDER: delete or replace before shipping.
//
// Wraps the per-app placeholder component with the shared nav bar so you
// have a working `Page` to register in `urls/client_router.rs` while
// scaffolding the SPA. Replace once real pages exist.
// -----------------------------------------------------------------------------
pub fn placeholder_page() -> Page {
    with_nav(super::components::placeholder())
}
