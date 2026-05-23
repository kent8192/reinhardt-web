//! WASM-bindgen tests asserting navigation observer invariants
//! against the Tier 4 (named-routes) fixture. Refs #4203.
//!
//! Run with (from the workspace root):
//!   wasm-pack test --chrome --headless --features wasm-diag-test \
//!     crates/reinhardt-pages -- --test spa_navigation_diag_named_test
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
//!   - Inv-5: after `Router::push` matching a **named** route `r`,
//!     `history.state.route_name == r.name()`. Tier 1〜3 only used
//!     anonymous routes, so they could not exercise this code path
//!     and missed the regression class behind issue #4203.
//!   - Inv-6: `__diag_router_id` is identical at registration and
//!     after every click. Falsifies the orphan-listener hypothesis
//!     (#4203 H4) — a divergence proves two router instances exist.
//!
//! Tier 4 ships as its own `[[test]]` entry alongside Tier 2 and
//! Tier 3 because each `ClientLauncher::launch` populates a
//! thread-local Router that cannot be reset within a single wasm
//! module load.

#![cfg(all(target_arch = "wasm32", feature = "wasm-diag-test"))]

use reinhardt_pages::app::{ClientLauncher, with_spa_router};
use reinhardt_pages::component::{IntoPage, Page, PageElement};
use reinhardt_urls::routers::ClientRouter;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ---- Page builders (mirror the standalone fixture in
// `tests/fixtures/spa_navigation_with_named_routes_app/src/client/pages.rs`).
//
// These builders duplicate the standalone fixture by design: the
// fixture exists so the e2e_cdp test can drive a real WASM bundle
// through Chrome, while this wasm-bindgen-test runs in-process
// against the same logical structure. Cargo's `optional = true` is
// not fully supported on `[dev-dependencies]`, so the fixture crate
// cannot be pulled in as a dev-dep. Drift between the two would
// surface immediately as a test-vs-fixture divergence in CI.

fn nav_link(href: &'static str, label: &'static str, current: &str) -> PageElement {
	let class = if current == href { "active" } else { "" };
	PageElement::new("a")
		.attr("href", href)
		.attr("class", class)
		.child(label)
}

fn layout_shell(content_id: &'static str, content_label: &'static str) -> Page {
	let current = with_spa_router(|r| r.current_path().get());
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
					.child(PageElement::new("li").child(nav_link(
						"/deployments",
						"Deployments",
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

fn deployments_page() -> Page {
	layout_shell("route-deployments", "DEPLOYMENTS VIEW")
}

fn login_page() -> Page {
	layout_shell("route-login", "LOGIN VIEW")
}

fn build_router() -> ClientRouter {
	ClientRouter::new()
		.named_route("dashboard:home", "/", home_page)
		.named_route("clusters:list", "/clusters", clusters_page)
		.named_route("deployments:list", "/deployments", deployments_page)
		.named_route("auth:login", "/login", login_page)
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

/// Read `history.state.route_name` for Inv-5. Returns `None` if
/// `history.state` is null or has no `route_name` field, treating
/// "missing" the same as "empty" for assertion purposes.
fn read_history_route_name() -> Option<String> {
	let window = web_sys::window()?;
	let history = window.history().ok()?;
	let state = history.state().ok()?;
	if state.is_null() || state.is_undefined() {
		return None;
	}
	let key = JsValue::from_str("route_name");
	let value = js_sys::Reflect::get(&state, &key).ok()?;
	value.as_string()
}

async fn yield_microtask() {
	let promise = js_sys::Promise::resolve(&wasm_bindgen::JsValue::UNDEFINED);
	let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

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

// ---- Test ----

#[wasm_bindgen_test]
async fn tier4_invariants_inv1_through_inv6_named_routes() {
	let _root = install_app_root();

	ClientLauncher::new("#app")
		.router_client(build_router)
		.launch()
		.expect("launch");

	// Inv-1: launch() must register at least one navigation observer.
	let observer_count_initial = with_spa_router(|r| r.__diag_observer_count());
	assert!(
		observer_count_initial >= 1,
		"Inv-1 (Tier 4) violated: launch() must register the render listener; got {}",
		observer_count_initial
	);

	// Inv-6 baseline: capture the router id at launch time.
	let router_id_initial = with_spa_router(|r| r.__diag_router_id());

	// DOM check: home content is mounted at boot.
	await_element("#route-home", 100).await;
	let document = web_sys::window().unwrap().document().unwrap();

	let dispatch_before = with_spa_router(|r| r.__diag_dispatch_count());
	let render_before = ClientLauncher::__diag_render_count();

	// ---- Click 1: / -> /clusters ----
	click_link("/clusters");
	await_element("#route-clusters", 100).await;

	let observer_after_one = with_spa_router(|r| r.__diag_observer_count());
	let dispatch_after_one = with_spa_router(|r| r.__diag_dispatch_count());
	let render_after_one = ClientLauncher::__diag_render_count();
	let router_id_after_one = with_spa_router(|r| r.__diag_router_id());
	let route_name_after_one = read_history_route_name();

	assert!(
		observer_after_one >= observer_count_initial,
		"Inv-2 (Tier 4) violated after click 1: observer count dropped {} -> {}",
		observer_count_initial,
		observer_after_one
	);
	assert_eq!(
		dispatch_after_one,
		dispatch_before + 1,
		"Inv-3 (Tier 4) violated after click 1: dispatch_count expected {} got {}",
		dispatch_before + 1,
		dispatch_after_one
	);
	assert_eq!(
		render_after_one,
		render_before + 1,
		"Inv-4 (Tier 4) violated after click 1: render_count expected {} got {}",
		render_before + 1,
		render_after_one
	);
	assert_eq!(
		route_name_after_one.as_deref(),
		Some("clusters:list"),
		"Inv-5 (Tier 4) violated after click 1: history.state.route_name expected \
		 Some(\"clusters:list\") got {:?}. The named route matched but its name() was \
		 not written into history.state — this is the failure shape of #4203.",
		route_name_after_one
	);
	assert_eq!(
		router_id_after_one, router_id_initial,
		"Inv-6 (Tier 4) violated after click 1: router_id changed {} -> {} \
		 (orphan listener hypothesis would expose itself here)",
		router_id_initial, router_id_after_one
	);

	// DOM swap (click 1)
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

	// ---- Click 2: /clusters -> /deployments ----
	click_link("/deployments");
	await_element("#route-deployments", 100).await;

	let observer_after_two = with_spa_router(|r| r.__diag_observer_count());
	let dispatch_after_two = with_spa_router(|r| r.__diag_dispatch_count());
	let render_after_two = ClientLauncher::__diag_render_count();
	let router_id_after_two = with_spa_router(|r| r.__diag_router_id());
	let route_name_after_two = read_history_route_name();

	assert!(
		observer_after_two >= observer_after_one,
		"Inv-2 (Tier 4) violated after click 2: observer count dropped {} -> {}",
		observer_after_one,
		observer_after_two
	);
	assert_eq!(
		dispatch_after_two,
		dispatch_before + 2,
		"Inv-3 (Tier 4) violated after click 2: dispatch_count expected {} got {}",
		dispatch_before + 2,
		dispatch_after_two
	);
	assert_eq!(
		render_after_two,
		render_before + 2,
		"Inv-4 (Tier 4) violated after click 2: render_count expected {} got {}",
		render_before + 2,
		render_after_two
	);
	assert_eq!(
		route_name_after_two.as_deref(),
		Some("deployments:list"),
		"Inv-5 (Tier 4) violated after click 2: history.state.route_name expected \
		 Some(\"deployments:list\") got {:?}",
		route_name_after_two
	);
	assert_eq!(
		router_id_after_two, router_id_initial,
		"Inv-6 (Tier 4) violated after click 2: router_id changed {} -> {}",
		router_id_initial, router_id_after_two
	);

	// ---- Click 3: /deployments -> /login (the exact path from #4203) ----
	click_link("/login");
	await_element("#route-login", 100).await;

	let dispatch_after_three = with_spa_router(|r| r.__diag_dispatch_count());
	let render_after_three = ClientLauncher::__diag_render_count();
	let router_id_after_three = with_spa_router(|r| r.__diag_router_id());
	let route_name_after_three = read_history_route_name();

	assert_eq!(
		dispatch_after_three,
		dispatch_before + 3,
		"Inv-3 (Tier 4) violated after click 3"
	);
	assert_eq!(
		render_after_three,
		render_before + 3,
		"Inv-4 (Tier 4) violated after click 3"
	);
	assert_eq!(
		route_name_after_three.as_deref(),
		Some("auth:login"),
		"Inv-5 (Tier 4) violated after click 3: history.state.route_name expected \
		 Some(\"auth:login\") got {:?}. This is the exact regression shape of #4203 — \
		 [active] state updates and history advances, but the named route's name is \
		 lost in transit so downstream consumers (and the render listener under \
		 some setups) never observe a resolved route.",
		route_name_after_three
	);
	assert_eq!(
		router_id_after_three, router_id_initial,
		"Inv-6 (Tier 4) violated after click 3: router_id changed {} -> {}",
		router_id_initial, router_id_after_three
	);

	// DOM swap (click 3)
	assert!(
		document
			.query_selector("#route-login")
			.expect("query_selector")
			.is_some(),
		"login page must be in DOM after click 3"
	);
	assert!(
		document
			.query_selector("#route-deployments")
			.expect("query_selector")
			.is_none(),
		"deployments page must be removed from DOM after navigation to /login"
	);
}
