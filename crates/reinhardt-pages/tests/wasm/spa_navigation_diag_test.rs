//! WASM-bindgen tests asserting navigation observer invariants
//! (Inv-1 ~ Inv-4) against the Tier 2 (sidebar-signal) fixture.
//! Refs #4122.
//!
//! Run with (from the workspace root):
//!   wasm-pack test --chrome --headless --features wasm-diag-test \
//!     crates/reinhardt-pages -- --test spa_navigation_diag_test
//!
//! The Inv-N invariants are:
//!   - Inv-1: launch() registers at least one navigation observer.
//!   - Inv-2: observer count is monotonic non-decreasing across
//!     navigations.
//!   - Inv-3: every `<a>` click that resolves to a known route
//!     produces exactly one `notify_observers` dispatch (i.e.
//!     `__diag_dispatch_count` increments by 1 per click).
//!   - Inv-4: every such click produces exactly one
//!     `render_and_mount` call (i.e. `__diag_render_count`
//!     increments by 1 per click).
//!
//! The four assertions are co-located in a single
//! `#[wasm_bindgen_test]` so that the launcher's thread-local router
//! state lives for the full sequence and so that the dispatch /
//! render counters share a coherent baseline. Splitting into four
//! tests would either require resetting the launcher's thread-local
//! between cases (no public API for that today) or accept a fresh
//! router whose pre-click counter baselines are not directly
//! comparable to the post-click values from a previous case.

#![cfg(all(target_arch = "wasm32", feature = "wasm-diag-test"))]

use reinhardt_pages::app::{ClientLauncher, with_router};
use reinhardt_pages::component::{IntoPage, Page, PageElement};
use reinhardt_pages::router::Router;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ---- Page builders (mirror the standalone fixture in
// `tests/fixtures/spa_navigation_with_sidebar_signal_app/src/lib.rs`) ----

fn nav_link(href: &'static str, label: &'static str, current: &str) -> PageElement {
	let class = if current == href { "active" } else { "" };
	PageElement::new("a")
		.attr("href", href)
		.attr("class", class)
		.child(label)
}

fn page_with_nav(id: &'static str, label: &'static str) -> Page {
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

fn home_page() -> Page {
	page_with_nav("route-home", "HOME VIEW")
}

fn clusters_page() -> Page {
	page_with_nav("route-clusters", "CLUSTERS VIEW")
}

fn login_page() -> Page {
	page_with_nav("route-login", "LOGIN VIEW")
}

fn build_router() -> Router {
	Router::new()
		.route("/", home_page)
		.route("/clusters", clusters_page)
		.route("/login", login_page)
}

// ---- DOM helpers ----

fn install_app_root() -> web_sys::Element {
	let document = web_sys::window().unwrap().document().unwrap();
	if let Some(prev) = document.get_element_by_id("app") {
		prev.remove();
	}
	let root = document.create_element("div").unwrap();
	root.set_id("app");
	document.body().unwrap().append_child(&root).unwrap();
	root
}

fn click_link(href: &str) {
	let document = web_sys::window().unwrap().document().unwrap();
	let selector = format!("a[href='{}']", href);
	let anchor = document
		.query_selector(&selector)
		.expect("query_selector")
		.unwrap_or_else(|| panic!("a[href={}] should exist after render", href));
	let html = anchor
		.dyn_ref::<web_sys::HtmlElement>()
		.expect("anchor must be an HtmlElement");
	html.click();
}

// ---- Test ----

#[wasm_bindgen_test]
fn tier2_invariants_inv1_through_inv4() {
	let _root = install_app_root();

	ClientLauncher::new("#app")
		.router(build_router)
		.launch()
		.expect("launch");

	// Inv-1: launch() must register at least one navigation observer.
	let observer_count_initial = with_router(|r| r.__diag_observer_count());
	assert!(
		observer_count_initial >= 1,
		"Inv-1 violated: launch() must register the render listener; got {}",
		observer_count_initial
	);

	// Capture baselines after launch but before any navigation. The
	// initial render performed during launch may or may not bump
	// `__diag_render_count` and `__diag_dispatch_count`; for Inv-3 /
	// Inv-4 we only assert per-click increments, so the absolute
	// baseline is irrelevant.
	let dispatch_before = with_router(|r| r.__diag_dispatch_count());
	let render_before = ClientLauncher::__diag_render_count();

	// First navigation: / -> /clusters via synthesized click.
	click_link("/clusters");

	let observer_after_one = with_router(|r| r.__diag_observer_count());
	let dispatch_after_one = with_router(|r| r.__diag_dispatch_count());
	let render_after_one = ClientLauncher::__diag_render_count();

	// Inv-2 (step 1): observer count must not have decreased.
	assert!(
		observer_after_one >= observer_count_initial,
		"Inv-2 violated after click 1: observer count dropped {} -> {}",
		observer_count_initial,
		observer_after_one
	);

	// Inv-3 (step 1): exactly one dispatch per click.
	assert_eq!(
		dispatch_after_one,
		dispatch_before + 1,
		"Inv-3 violated after click 1: dispatch_count expected {} got {}",
		dispatch_before + 1,
		dispatch_after_one
	);

	// Inv-4 (step 1): exactly one render per click.
	assert_eq!(
		render_after_one,
		render_before + 1,
		"Inv-4 violated after click 1: render_count expected {} got {}",
		render_before + 1,
		render_after_one
	);

	// Second navigation: /clusters -> /login.
	click_link("/login");

	let observer_after_two = with_router(|r| r.__diag_observer_count());
	let dispatch_after_two = with_router(|r| r.__diag_dispatch_count());
	let render_after_two = ClientLauncher::__diag_render_count();

	// Inv-2 (step 2): still monotonic across the second navigation.
	assert!(
		observer_after_two >= observer_after_one,
		"Inv-2 violated after click 2: observer count dropped {} -> {}",
		observer_after_one,
		observer_after_two
	);

	// Inv-3 (step 2): cumulative dispatches increased by exactly two.
	assert_eq!(
		dispatch_after_two,
		dispatch_before + 2,
		"Inv-3 violated after click 2: dispatch_count expected {} got {}",
		dispatch_before + 2,
		dispatch_after_two
	);

	// Inv-4 (step 2): cumulative renders increased by exactly two.
	assert_eq!(
		render_after_two,
		render_before + 2,
		"Inv-4 violated after click 2: render_count expected {} got {}",
		render_before + 2,
		render_after_two
	);
}
