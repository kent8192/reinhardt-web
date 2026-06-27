//! Site-wide navigation shell.
//!
//! `with_nav(body)` wraps a per-app `Page` with the shared header so every
//! routed page in the project gets the same nav bar.

use reinhardt::pages::component::Page;
use reinhardt::pages::page;

/// Render the shared navigation bar.
pub fn nav_bar() -> Page {
    page!(|| {
        nav {
            class: "navbar",
            "{{ project_name }}"
        }
    })()
}

/// Wrap an app body with the shared nav bar.
pub fn with_nav(body: Page) -> Page {
    Page::Fragment(vec![nav_bar(), body])
}
