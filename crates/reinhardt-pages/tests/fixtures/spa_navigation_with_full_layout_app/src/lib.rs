//! Tier 3 fixture — SPA navigation regression suite (#4122).
//!
//! Mirrors the cloud-dashboard structural traits we believed could
//! trigger a navigation listener loss: a persistent layout shell with
//! `<aside>` sidebar, content swapped per-route inside `<main>`, three
//! routes, nested `<a>` tags inside the shell so the link interceptor
//! must walk up parent elements.
//!
//! Routes:
//! - `/`         → "Home" content under the persistent shell
//! - `/clusters` → "Clusters" content
//! - `/login`    → "Login" content
//!
//! Exposes `__diag_observer_count_js`, `__diag_dispatch_count_js`, and
//! `__diag_render_count_js` via `#[wasm_bindgen]` so the e2e_cdp test
//! (driven by Chrome via CDP) can read the counters through
//! `execute_js`.
//!
//! Tests pass on current main HEAD — they exist as a future-regression
//! net for the listener-loss class (#4075, #4088, #4122) so it cannot
//! silently re-emerge through a real source change or a stale-wasm-
//! bundle deployment hazard (the actual root cause of #4122; see
//! tracking issues #4127 and #4128).

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

fn layout_shell(content_id: &'static str, content_label: &'static str) -> Page {
	// Read `current_path` AT RENDER TIME so the sidebar's `active` class
	// reflects the freshly-navigated path. `render_and_mount` runs only
	// after `Router::push` has updated the path Signal and notified
	// observers.
	let current = with_router(|r| r.current_path().get());
	PageElement::new("div")
		.attr("id", "shell")
		.child(
			PageElement::new("aside").attr("id", "sidebar").child(
				PageElement::new("ul")
					.child(PageElement::new("li").child(nav_link("/", "Home", &current)))
					.child(
						PageElement::new("li")
							.child(nav_link("/clusters", "Clusters", &current)),
					)
					.child(PageElement::new("li").child(nav_link("/login", "Login", &current))),
			),
		)
		.child(
			PageElement::new("main").attr("id", "content").child(
				PageElement::new("section")
					.attr("id", content_id)
					.child(PageElement::new("h1").child(content_label)),
			),
		)
		.into_page()
}

pub fn home_page() -> Page {
	layout_shell("route-home", "HOME VIEW")
}

pub fn clusters_page() -> Page {
	layout_shell("route-clusters", "CLUSTERS VIEW")
}

pub fn login_page() -> Page {
	layout_shell("route-login", "LOGIN VIEW")
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

#[wasm_bindgen]
pub fn __diag_observer_count_js() -> usize {
	with_router(|r| r.__diag_observer_count())
}

#[wasm_bindgen]
pub fn __diag_dispatch_count_js() -> u64 {
	with_router(|r| r.__diag_dispatch_count())
}

#[wasm_bindgen]
pub fn __diag_render_count_js() -> u64 {
	ClientLauncher::__diag_render_count()
}
