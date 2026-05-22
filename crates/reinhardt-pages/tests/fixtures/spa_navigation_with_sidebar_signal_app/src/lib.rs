//! Tier 2 fixture — SPA navigation regression suite (#4122).
//!
//! Each route renders its own copy of a tiny nav with three `<a>` tags
//! whose `class` attribute is computed at render time from the current
//! `Router::current_path` Signal. There is no persistent layout shell —
//! the entire view is rebuilt on each navigation. This isolates the
//! "navigation pipeline reads current_path Signal correctly across
//! re-renders" axis from the "persistent layout shell" axis when
//! bisecting #4075 / #4088 / #4122.
//!
//! Routes:
//! - `/`         -> `<div id="route-home">`
//! - `/clusters` -> `<div id="route-clusters">`
//! - `/login`    -> `<div id="route-login">`
//!
//! The wasm-bindgen-test harness in
//! `tests/wasm/spa_navigation_diag_test.rs` boots this fixture through
//! `ClientLauncher::launch`, then drives navigation via synthesized
//! `<a>` clicks and asserts Inv-1 ~ Inv-4 against
//! `Router::__diag_*` and `ClientLauncher::__diag_render_count`.

use reinhardt_pages::app::{ClientLauncher, with_router};
use reinhardt_pages::component::{IntoPage, Page, PageElement};
use reinhardt_pages::router::Router;
use wasm_bindgen::prelude::*;

fn nav_link(href: &'static str, label: &'static str, current: &str) -> PageElement {
	let class = if current == href { "active" } else { "" };
	PageElement::new("a")
		.attr("href", href)
		.attr("class", class)
		.child(label)
}

fn page_with_nav(id: &'static str, label: &'static str) -> Page {
	// Read `current_path` AT RENDER TIME. `render_and_mount` runs only
	// after `Router::push` has updated the path Signal and notified
	// observers, so the value seen here is the freshly-navigated path.
	// This keeps the fixture API-compatible with `reinhardt-pages` even
	// though the public API has no per-attribute reactive helper.
	let current = with_router(|r| r.current_path().get());
	PageElement::new("div")
		.attr("id", id)
		.child(
			PageElement::new("nav")
				.child(nav_link("/", "Home", &current))
				.child(nav_link("/clusters", "Clusters", &current))
				.child(nav_link("/login", "Login", &current)),
		)
		.child(PageElement::new("p").child(label))
		.into_page()
}

pub fn home_page() -> Page {
	page_with_nav("route-home", "HOME VIEW")
}

pub fn clusters_page() -> Page {
	page_with_nav("route-clusters", "CLUSTERS VIEW")
}

pub fn login_page() -> Page {
	page_with_nav("route-login", "LOGIN VIEW")
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
	console_error_panic_hook::set_once();
	ClientLauncher::new("#app")
		.router(|| {
			Router::new()
				.route("/", home_page)
				.route("/clusters", clusters_page)
				.route("/login", login_page)
		})
		.launch()
}
