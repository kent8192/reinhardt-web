//! Site-wide navigation shell.
//!
//! `with_nav(body)` wraps a per-app `Component` with the shared header so
//! every routed page in the project gets the same nav bar.

use reinhardt::pages::component::{Component, Page};
use reinhardt::pages::page;

/// Wrap an app body with the shared nav bar.
pub fn with_nav(body: Component) -> Page {
	page!(|body: Component| {
		div {
			nav { class: "navbar", "{{ project_name }}" }
			{ body }
		}
	})(body)
}
