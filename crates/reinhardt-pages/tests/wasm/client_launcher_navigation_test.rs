//! WASM regression test for issue #4075 — verifies that
//! `ClientLauncher::launch()` installs a render Effect that re-fires
//! when `Router::push` updates the path Signal.
//!
//! Without the fix, the SPA renders only the route mounted at boot;
//! every subsequent `Router::push` updates the path Signal but the
//! root view is never re-mounted.
//!
//! **Run with** (from the workspace root):
//!   `wasm-pack test --headless --chrome crates/reinhardt-pages -- --test client_launcher_navigation_test`
//!
//! Cargo args (such as `--test ...`) must follow `--`; `wasm-pack` does not
//! accept Cargo flags before the path argument.

#![cfg(wasm)]

use reinhardt_pages::app::{ClientLauncher, with_router};
use reinhardt_pages::component::{IntoPage, Page, PageElement};
use reinhardt_pages::router::Router;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// Each route renders a div with a stable id and a unique text marker so the
// assertions can be tight regardless of how `Page::Text` serialises into
// `inner_html`.
fn page_root() -> Page {
	PageElement::new("div")
		.attr("id", "route-root")
		.child("ROUTE-ROOT-CONTENT")
		.into_page()
}

fn page_a() -> Page {
	PageElement::new("div")
		.attr("id", "route-a")
		.child("ROUTE-A-CONTENT")
		.into_page()
}

fn page_b() -> Page {
	PageElement::new("div")
		.attr("id", "route-b")
		.child("ROUTE-B-CONTENT")
		.into_page()
}

fn install_app_root() -> web_sys::Element {
	let document = web_sys::window().unwrap().document().unwrap();
	// Remove any pre-existing root left behind by a prior test run.
	if let Some(prev) = document.get_element_by_id("app") {
		prev.remove();
	}
	let root = document.create_element("div").unwrap();
	root.set_id("app");
	document.body().unwrap().append_child(&root).unwrap();
	root
}

/// Yields control so the reactive scheduler (which uses
/// `wasm_bindgen_futures::spawn_local`) can drain queued work.
async fn yield_to_microtasks() {
	gloo_timers::future::TimeoutFuture::new(0).await;
}

#[wasm_bindgen_test]
async fn client_launcher_re_renders_on_router_push() {
	let root = install_app_root();

	// Register `/` so the boot mount has a deterministic view regardless of
	// the test harness's starting URL.
	ClientLauncher::new("#app")
		.router(|| {
			Router::new()
				.route("/", page_root)
				.route("/a", page_a)
				.route("/b", page_b)
		})
		.launch()
		.expect("launch");

	// First yield: let any deferred reactive work after launch settle.
	yield_to_microtasks().await;

	// Navigate to /a and confirm the body switches.
	with_router(|r| r.push("/a")).expect("push /a");
	yield_to_microtasks().await;
	yield_to_microtasks().await;

	let html_after_a = root.inner_html();
	assert!(
		html_after_a.contains("ROUTE-A-CONTENT"),
		"expected /a view after push('/a'), got: {html_after_a}"
	);
	assert!(
		!html_after_a.contains("ROUTE-B-CONTENT"),
		"expected /b view absent after push('/a'), got: {html_after_a}"
	);

	// Navigate to /b — this is the regression-critical step.
	// Pre-fix: the render Effect never re-fires, so `inner_html` still
	// shows route-a content.
	with_router(|r| r.push("/b")).expect("push /b");
	yield_to_microtasks().await;
	yield_to_microtasks().await;

	let html_after_b = root.inner_html();
	assert!(
		html_after_b.contains("ROUTE-B-CONTENT"),
		"expected /b view after push('/b'), got: {html_after_b}"
	);
	assert!(
		!html_after_b.contains("ROUTE-A-CONTENT"),
		"expected /a view absent after push('/b'), got: {html_after_b}"
	);
}
