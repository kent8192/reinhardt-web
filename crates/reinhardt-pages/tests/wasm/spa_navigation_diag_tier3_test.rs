//! WASM-bindgen tests asserting navigation observer invariants
//! (Inv-1 ~ Inv-4) plus DOM swap against the Tier 3 (full-layout)
//! fixture. Refs #4122.
//!
//! Run with (from the workspace root):
//!   wasm-pack test --chrome --headless --features wasm-diag-test \
//!     crates/reinhardt-pages -- --test spa_navigation_diag_tier3_test
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
//! Additionally, this Tier 3 suite asserts the DOM-swap property:
//! after each navigation, the previous route's content section is
//! removed from the DOM and the new route's content section is
//! mounted.
//!
//! Tier 2 and Tier 3 ship as SEPARATE `[[test]]` entries (separate
//! cdylib binaries) because each `ClientLauncher::launch` populates a
//! thread-local Router that cannot be reset within a single wasm
//! module load — co-locating both tiers in one binary would cause the
//! second `launch()` to panic on the lingering thread-local.
//!
//! The Inv-1 ~ Inv-4 + DOM-swap assertions are co-located in a single
//! `#[wasm_bindgen_test]` for the same reason described in
//! `spa_navigation_diag_test.rs`: the launcher's thread-local router
//! state lives for the full sequence so the dispatch / render counters
//! share a coherent baseline.

#![cfg(all(target_arch = "wasm32", feature = "wasm-diag-test"))]

use reinhardt_pages::app::{ClientLauncher, with_router};
use reinhardt_pages::component::{IntoPage, Page, PageElement};
use reinhardt_pages::router::Router;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ---- Page builders (mirror the standalone fixture in
// `tests/fixtures/spa_navigation_with_full_layout_app/src/lib.rs`) ----
//
// These page builders duplicate the standalone fixture by design: the
// fixture exists so the e2e_cdp test can build a real WASM bundle and
// drive it through Chrome, while this wasm-bindgen-test runs in-process
// against the same logical structure. Cargo's `optional = true` is not
// fully supported on `[dev-dependencies]`, so we cannot pull the
// fixture crate in as a dev-dep. Drift between the two would surface
// immediately as a test-vs-fixture divergence in CI.

fn nav_link(href: &'static str, label: &'static str, current: &str) -> PageElement {
	let class = if current == href { "active" } else { "" };
	PageElement::new("a")
		.attr("href", href)
		.attr("class", class)
		.child(label)
}

fn layout_shell(content_id: &'static str, content_label: &'static str) -> Page {
	let current = with_router(|r| r.current_path().get());
	PageElement::new("div")
		.attr("id", "shell")
		.child(
			PageElement::new("aside").attr("id", "sidebar").child(
				PageElement::new("ul")
					.child(PageElement::new("li").child(nav_link("/", "Home", &current)))
					.child(PageElement::new("li").child(nav_link(
						"/clusters",
						"Clusters",
						&current,
					)))
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

fn home_page() -> Page {
	layout_shell("route-home", "HOME VIEW")
}

fn clusters_page() -> Page {
	layout_shell("route-clusters", "CLUSTERS VIEW")
}

fn login_page() -> Page {
	layout_shell("route-login", "LOGIN VIEW")
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

/// Yields execution to the event loop's microtask queue. Used after
/// each synthesized click so any pending async work scheduled by the
/// reactive runtime (Effect scheduling lives behind
/// `wasm_bindgen_futures::spawn_local` per `app.rs::launch::Phase A`)
/// has a chance to run before the test samples counters or DOM state.
///
/// The current navigate -> notify_observers -> render_and_mount path
/// is fully synchronous, so this yield is defensive insurance against
/// future async refactors of that path. Without it, a hypothetical
/// async render would make this test flake by sampling before the
/// dispatch / render completes (Copilot review feedback on PR #4129).
async fn yield_microtask() {
	let promise = js_sys::Promise::resolve(&wasm_bindgen::JsValue::UNDEFINED);
	let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// Polls `query_selector(selector)` until it returns `Some`, yielding
/// to the microtask queue between attempts. Errors out with a clear
/// message after `max_iterations` (each iteration costs ~one
/// microtask, so 100 iterations corresponds to "give the runtime a
/// few frames to settle"). Used by the Tier 3 test as a defensive
/// post-click wait.
async fn await_element(selector: &str, max_iterations: u32) {
	let document = web_sys::window().unwrap().document().unwrap();
	for _ in 0..max_iterations {
		if document
			.query_selector(selector)
			.expect("query_selector")
			.is_some()
		{
			return;
		}
		yield_microtask().await;
	}
	panic!(
		"timed out waiting for `{}` to appear after {} microtask yields",
		selector, max_iterations
	);
}

#[wasm_bindgen_test]
async fn tier3_invariants_inv1_through_inv4_with_dom_swap() {
	let _root = install_app_root();

	ClientLauncher::new("#app")
		.router(build_router)
		.launch()
		.expect("launch");

	// Inv-1: launch() must register at least one navigation observer.
	let observer_count_initial = with_router(|r| r.__diag_observer_count());
	assert!(
		observer_count_initial >= 1,
		"Inv-1 (Tier 3) violated: launch() must register the render listener; got {}",
		observer_count_initial
	);

	// DOM check: home content is mounted at boot. Wait for it explicitly
	// so the test does not assume a fully synchronous initial mount.
	await_element("#route-home", 100).await;
	let document = web_sys::window().unwrap().document().unwrap();

	// Capture baselines after launch but before any navigation.
	let dispatch_before = with_router(|r| r.__diag_dispatch_count());
	let render_before = ClientLauncher::__diag_render_count();

	// First navigation: / -> /clusters via synthesized click. Wait for
	// the post-click DOM to settle before sampling counters.
	click_link("/clusters");
	await_element("#route-clusters", 100).await;

	let observer_after_one = with_router(|r| r.__diag_observer_count());
	let dispatch_after_one = with_router(|r| r.__diag_dispatch_count());
	let render_after_one = ClientLauncher::__diag_render_count();

	// Inv-2 (step 1): observer count must not have decreased.
	assert!(
		observer_after_one >= observer_count_initial,
		"Inv-2 (Tier 3) violated after click 1: observer count dropped {} -> {}",
		observer_count_initial,
		observer_after_one
	);

	// Inv-3 (step 1): exactly one dispatch per click.
	assert_eq!(
		dispatch_after_one,
		dispatch_before + 1,
		"Inv-3 (Tier 3) violated after click 1: dispatch_count expected {} got {}",
		dispatch_before + 1,
		dispatch_after_one
	);

	// Inv-4 (step 1): exactly one render per click.
	assert_eq!(
		render_after_one,
		render_before + 1,
		"Inv-4 (Tier 3) violated after click 1: render_count expected {} got {}",
		render_before + 1,
		render_after_one
	);

	// DOM swap (step 1): clusters mounted, home gone.
	assert!(
		document
			.query_selector("#route-clusters")
			.expect("query_selector")
			.is_some(),
		"clusters page must be in DOM after click 1"
	);
	assert!(
		document
			.query_selector("#route-home")
			.expect("query_selector")
			.is_none(),
		"home page must be removed from DOM after navigation to /clusters"
	);

	// Second navigation: /clusters -> /login. Wait for the post-click
	// DOM to settle before sampling counters.
	click_link("/login");
	await_element("#route-login", 100).await;

	let observer_after_two = with_router(|r| r.__diag_observer_count());
	let dispatch_after_two = with_router(|r| r.__diag_dispatch_count());
	let render_after_two = ClientLauncher::__diag_render_count();

	// Inv-2 (step 2): still monotonic across the second navigation.
	assert!(
		observer_after_two >= observer_after_one,
		"Inv-2 (Tier 3) violated after click 2: observer count dropped {} -> {}",
		observer_after_one,
		observer_after_two
	);

	// Inv-3 (step 2): cumulative dispatches increased by exactly two.
	assert_eq!(
		dispatch_after_two,
		dispatch_before + 2,
		"Inv-3 (Tier 3) violated after click 2: dispatch_count expected {} got {}",
		dispatch_before + 2,
		dispatch_after_two
	);

	// Inv-4 (step 2): cumulative renders increased by exactly two.
	assert_eq!(
		render_after_two,
		render_before + 2,
		"Inv-4 (Tier 3) violated after click 2: render_count expected {} got {}",
		render_before + 2,
		render_after_two
	);

	// DOM swap (step 2): login mounted, clusters gone.
	assert!(
		document
			.query_selector("#route-login")
			.expect("query_selector")
			.is_some(),
		"login page must be in DOM after click 2"
	);
	assert!(
		document
			.query_selector("#route-clusters")
			.expect("query_selector")
			.is_none(),
		"clusters page must be removed from DOM after navigation to /login"
	);
}
