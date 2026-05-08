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

use reinhardt_pages::app::{ClientLauncher, with_router};
use reinhardt_pages::component::{IntoPage, Page, PageElement};
use reinhardt_pages::router::Router;
use wasm_bindgen::JsValue;
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

fn build_router() -> Router {
	Router::new()
		.named_route("dashboard:home", "/", home_page)
		.named_route("clusters:list", "/clusters", clusters_page)
}

fn read_dataset_nav_site() -> Option<String> {
	let document = web_sys::window()?.document()?;
	let body = document.body()?;
	body.get_attribute("data-reinhardt-nav-site")
}

#[wasm_bindgen_test]
async fn nav_diag_dom_writes_store_router_at_launch() {
	let _root = install_app_root();
	// Clear any leftover attribute from a prior test in the same harness.
	if let Some(body) = web_sys::window()
		.and_then(|w| w.document())
		.and_then(|d| d.body())
	{
		let _ = body.remove_attribute("data-reinhardt-nav-site");
	}

	ClientLauncher::new("#app")
		.router(build_router)
		.launch()
		.expect("launch");

	// `store_router` is the first nav_diag_dom! site to fire after launch.
	// Subsequent launch-time sites (notify_observers from the initial render)
	// may overwrite, so we only assert the attribute is *present* and non-empty.
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
async fn nav_diag_dom_advances_to_navigate_after_router_push() {
	let _root = install_app_root();

	ClientLauncher::new("#app")
		.router(build_router)
		.launch()
		.expect("launch");

	// Drive a programmatic navigation. After this returns, the most recent
	// nav_diag_dom! site to have written is `notify_observers` (link click
	// path: link_interceptor → navigate → notify_observers, but here we
	// skip the link click so the path is push → navigate → notify_observers).
	with_router(|r| r.push("/clusters")).expect("push /clusters");

	let promise = js_sys::Promise::resolve(&JsValue::UNDEFINED);
	let _ = wasm_bindgen_futures::JsFuture::from(promise).await;

	let site = read_dataset_nav_site();
	assert_eq!(
		site.as_deref(),
		Some("notify_observers"),
		"nav-diag-dom: after Router::push(), the last-write-wins value should be \
		 \"notify_observers\" (the final site in the navigate → notify_observers \
		 chain). Got: {:?}",
		site
	);
}
