//! Integration test for the `pages/nav-diag-dom` opt-in diagnostic.
//!
//! When the `nav-diag-dom` feature is enabled, framework SPA navigation
//! sites must write their site name to `document.body.dataset.reinhardtNavSite`
//! (i.e. `body.getAttribute("data-reinhardt-nav-site")`). This bypasses
//! `console.debug` / wasm-bindgen import-shim subtleties so downstream
//! debuggers can verify which navigation code path executed via plain
//! DOM inspection.
//!
//! **Run with** (from the workspace root):
//!   `wasm-pack test --headless --chrome crates/reinhardt-pages \
//!        --features wasm-diag-test,nav-diag-dom \
//!        -- --test nav_diag_dom_test`

#![cfg(all(wasm, feature = "nav-diag-dom"))]

use reinhardt_pages::app::{ClientLauncher, with_spa_router};
use reinhardt_pages::component::{IntoPage, Page, PageElement};
use reinhardt_urls::routers::ClientRouter;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn home_page() -> Page {
	PageElement::new("div")
		.attr("id", "route-home")
		.child("HOME")
		.into_page()
}

fn clusters_page() -> Page {
	PageElement::new("div")
		.attr("id", "route-clusters")
		.child("CLUSTERS")
		.into_page()
}

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

fn build_router() -> ClientRouter {
	ClientRouter::new()
		.named_route("dashboard:home", "/", home_page)
		.named_route("clusters:list", "/clusters", clusters_page)
}

fn read_dataset_nav_site() -> Option<String> {
	let document = web_sys::window()?.document()?;
	let body = document.body()?;
	body.get_attribute("data-reinhardt-nav-site")
}

#[wasm_bindgen_test]
async fn nav_diag_dom_writes_some_site_at_launch() {
	let _root = install_app_root();
	// Clear any leftover attribute from a prior test in the same harness.
	if let Some(body) = web_sys::window()
		.and_then(|w| w.document())
		.and_then(|d| d.body())
	{
		let _ = body.remove_attribute("data-reinhardt-nav-site");
	}

	ClientLauncher::new("#app")
		.router_client(build_router)
		.launch()
		.expect("launch");

	// `store_router` is the first nav_diag_dom! site to fire after launch,
	// but subsequent launch-time sites (e.g. `notify_observers` from the
	// initial render) may overwrite it under last-write-wins semantics.
	// We therefore only assert the attribute is *present* and non-empty —
	// a separate test exercises a specific post-launch site.
	let site = read_dataset_nav_site();
	assert!(
		site.is_some(),
		"nav-diag-dom: data-reinhardt-nav-site must be set after launch when the feature is enabled"
	);
	assert!(
		site.as_deref() != Some(""),
		"nav-diag-dom: data-reinhardt-nav-site must be non-empty"
	);
}

#[wasm_bindgen_test]
async fn nav_diag_dom_writes_notify_observers_after_router_push() {
	let _root = install_app_root();

	ClientLauncher::new("#app")
		.router_client(build_router)
		.launch()
		.expect("launch");

	// The launch path itself ends with a `notify_observers` write, so
	// asserting `Some("notify_observers")` directly after launch would
	// pass even if `Router::push()` did nothing. Stamp a sentinel so the
	// post-push assertion can only succeed if `push → navigate →
	// notify_observers` actually wrote the attribute.
	let body = web_sys::window()
		.and_then(|w| w.document())
		.and_then(|d| d.body())
		.expect("body");
	body.set_attribute("data-reinhardt-nav-site", "test_sentinel")
		.expect("set sentinel");

	// Drive a programmatic navigation. After this returns, the most recent
	// nav_diag_dom! site to have written is `notify_observers` (push path:
	// push → navigate → notify_observers).
	with_spa_router(|r| r.push("/clusters")).expect("push /clusters");

	let promise = js_sys::Promise::resolve(&JsValue::UNDEFINED);
	let _ = wasm_bindgen_futures::JsFuture::from(promise).await;

	let site = read_dataset_nav_site();
	assert_ne!(
		site.as_deref(),
		Some("test_sentinel"),
		"nav-diag-dom: Router::push() did not overwrite the pre-push sentinel; \
		 nav_diag_dom! sites are not firing on the push path."
	);
	assert_eq!(
		site.as_deref(),
		Some("notify_observers"),
		"nav-diag-dom: after Router::push(), the last-write-wins value should be \
		 \"notify_observers\" (the final site in the navigate → notify_observers \
		 chain). Got: {:?}",
		site
	);
}

/// End-to-end click reproducer requested by the cloud-side reporter on
/// #4221. Mirrors the dashboard's exact runtime path:
/// `ClientLauncher::launch()` (installs link interceptor) → real
/// `<a href="/clusters">` click → `link_interceptor → Router::push →
/// Router::navigate → notify_observers` chain. If this passes upstream
/// while the dashboard still observes `setAttr_total: 0` and a
/// JSON-string `history.state`, the bug is **not** in the framework's
/// link-click → push → push_state path — it is in a code path that
/// bypasses `Router::navigate` and calls `History::push_state_with_url`
/// directly with `JsValue::from_str(json)`.
#[wasm_bindgen_test]
async fn nav_diag_dom_advances_through_full_link_click_chain() {
	let _root = install_app_root();

	// `ClientRouter::new()` reads `current_path()` from `window.location` at
	// construction time, so a previous test in the same wasm test binary
	// that navigated away from `/` would make this reproducer start on a
	// non-`/` path. Reset history to `/` before launching to keep the
	// initial state deterministic regardless of test execution order.
	let history = web_sys::window()
		.expect("window")
		.history()
		.expect("history");
	let _ = history.replace_state_with_url(&JsValue::NULL, "", Some("/"));

	ClientLauncher::new("#app")
		.router_client(build_router)
		.launch()
		.expect("launch");

	let document = web_sys::window()
		.and_then(|w| w.document())
		.expect("document");
	let body = document.body().expect("body");

	// Inject a real `<a href="/clusters">` anchor that the link interceptor
	// must pick up on click. Append to body (outside `#app`) so re-renders
	// of the route view do not detach the anchor before the click fires.
	// Use a stable id so we can remove any leftover anchor from a previous
	// run of this test in the same wasm test binary.
	const ANCHOR_ID: &str = "test-link-click-reproducer-anchor";
	if let Some(prev) = document.get_element_by_id(ANCHOR_ID) {
		prev.remove();
	}
	let anchor: web_sys::HtmlElement = document
		.create_element("a")
		.expect("create a")
		.dyn_into()
		.expect("dyn HtmlElement");
	anchor.set_id(ANCHOR_ID);
	anchor.set_attribute("href", "/clusters").expect("set href");
	anchor.set_text_content(Some("go to clusters"));
	body.append_child(&anchor).expect("append a");

	// Stamp a sentinel so the post-click assertion can only succeed if
	// some `nav_diag_dom!` site fired during the click handling chain.
	body.set_attribute("data-reinhardt-nav-site", "test_sentinel")
		.expect("set sentinel");

	// Snapshot `history.state` shape before the click so we can prove the
	// click transitioned it to a JS object (the canonical post-#4218 shape).
	let state_before = history.state().expect("state_before");

	// `HtmlElement::click()` dispatches a real, bubbling MouseEvent so the
	// document-level listener installed by `ClientLauncher::launch()` sees
	// it — equivalent to a user clicking the anchor.
	anchor.click();

	// Yield to the microtask queue once so any reactive scheduler updates
	// flush before we read the body attribute.
	let promise = js_sys::Promise::resolve(&JsValue::UNDEFINED);
	let _ = wasm_bindgen_futures::JsFuture::from(promise).await;

	let site = read_dataset_nav_site();
	assert_ne!(
		site.as_deref(),
		Some("test_sentinel"),
		"nav-diag-dom: click on `<a href=\"/clusters\">` did not overwrite the \
		 pre-click sentinel; the link interceptor or the navigate chain is not \
		 firing nav_diag_dom! sites in this build."
	);
	assert_eq!(
		site.as_deref(),
		Some("notify_observers"),
		"nav-diag-dom: after a real link click, last-write-wins value should be \
		 \"notify_observers\" (the final site in link_interceptor → navigate → \
		 notify_observers). Got: {:?}",
		site
	);

	// The original #4221 symptom: after a navigation, `history.state` must
	// be a JS object (structured-clone of the serde-wasm-bindgen value),
	// not a JSON string. If this assertion fires, the bug returned via the
	// canonical `push_state` and `wasm_bindgen_abi_pin_test` is the next
	// line of defence.
	let state_after = history.state().expect("state_after");
	assert!(
		!state_after.is_string(),
		"#4221 click reproducer: history.state is a JS string after link click. \
		 Pre-click state: {:?}. Post-click state: {:?}",
		state_before,
		state_after
	);
	assert!(
		state_after.is_object(),
		"#4221 click reproducer: history.state is not is_object() after link \
		 click. Post-click state: {:?}",
		state_after
	);

	// Cleanup so subsequent tests in the same wasm test binary start with a
	// clean DOM and history state, mirroring `wasm_bindgen_abi_pin_test`.
	if let Some(prev) = document.get_element_by_id(ANCHOR_ID) {
		prev.remove();
	}
	let _ = history.replace_state_with_url(&JsValue::NULL, "", Some("/"));
}
