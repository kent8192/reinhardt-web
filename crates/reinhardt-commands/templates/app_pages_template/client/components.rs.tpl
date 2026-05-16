//! UI components for the {{ app_name }} application.
//!
//! Reached only on the WASM target through `#[cfg(client)] pub mod client;`
//! in the parent app aggregator, so contents below do not need additional
//! gates. Add reusable component functions here; each component typically
//! returns `reinhardt::pages::component::Page` (the concrete page type;
//! `Component` is the underlying trait), and may be wrapped by
//! `super::pages` for use as a routed entry.

use reinhardt::pages::component::Page;
use reinhardt::pages::page;

// -----------------------------------------------------------------------------
// PLACEHOLDER: delete or replace before shipping.
//
// Returns a minimal `Page` that renders a single placeholder string.
// Exists only so the module compiles and `super::pages::placeholder_page`
// has something to wrap.
// -----------------------------------------------------------------------------
pub fn placeholder() -> Page {
    page!(|| {
        div {
            class: "placeholder",
            "{{ app_name }} placeholder component"
        }
    })()
}
