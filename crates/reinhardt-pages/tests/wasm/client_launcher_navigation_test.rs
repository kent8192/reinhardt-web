//! WASM regression test for issues #4075 and #4088 — verifies that
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
use reinhardt_pages::reactive::with_runtime;
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

	ClientLauncher::new("#app")
		.router(|| {
			Router::new()
				.route("/", page_root)
				.route("/a", page_a)
				.route("/b", page_b)
		})
		.launch()
		.expect("launch");

	yield_to_microtasks().await;

	// === Diagnostic: launcher Effect must be subscribed to path Signal ===
	let path_signal_id = with_router(|r| r.current_path().id());
	let subscribers_after_launch = with_runtime(|rt| rt.debug_subscribers(path_signal_id));
	assert!(
		!subscribers_after_launch.is_empty(),
		"[DIAG #4088] path_signal has no subscribers after launch — launcher Effect was not tracked. \
		 observer_stack: {:?}, dependencies: {:?}",
		with_runtime(|rt| rt.debug_observer_stack()),
		with_runtime(|rt| rt.debug_dependencies(path_signal_id)),
	);

	// Navigate to /a and confirm the body switches.
	with_router(|r| r.push("/a")).expect("push /a");
	let pending_after_push_a = with_runtime(|rt| rt.debug_pending_updates());
	yield_to_microtasks().await;
	yield_to_microtasks().await;

	let html_after_a = root.inner_html();
	assert!(
		html_after_a.contains("ROUTE-A-CONTENT"),
		"[DIAG #4088] expected /a view after push('/a'). \
		 pending_updates immediately after push: {:?}, \
		 subscribers of path_signal now: {:?}, \
		 actual html: {}",
		pending_after_push_a,
		with_runtime(|rt| rt.debug_subscribers(path_signal_id)),
		html_after_a,
	);
	assert!(
		!html_after_a.contains("ROUTE-B-CONTENT"),
		"expected /b view absent after push('/a'), got: {html_after_a}"
	);

	// Navigate to /b — this is the regression-critical step.
	with_router(|r| r.push("/b")).expect("push /b");
	let pending_after_push_b = with_runtime(|rt| rt.debug_pending_updates());
	yield_to_microtasks().await;
	yield_to_microtasks().await;

	let html_after_b = root.inner_html();
	assert!(
		html_after_b.contains("ROUTE-B-CONTENT"),
		"[DIAG #4088] expected /b view after push('/b'). \
		 pending_updates immediately after push: {:?}, \
		 subscribers of path_signal now: {:?}, \
		 actual html: {}",
		pending_after_push_b,
		with_runtime(|rt| rt.debug_subscribers(path_signal_id)),
		html_after_b,
	);
	assert!(
		!html_after_b.contains("ROUTE-A-CONTENT"),
		"expected /a view absent after push('/b'), got: {html_after_b}"
	);
}

/// Direct reproduction of Issue #4088: simulates the reinhardt-cloud dashboard
/// flow with /, /login, /register, /clusters where /clusters has no route
/// (falls through to not_found).
#[wasm_bindgen_test]
async fn client_launcher_reproduces_issue_4088_navigation_flow() {
	let root = install_app_root();

	fn login_page() -> Page {
		PageElement::new("div")
			.attr("id", "login")
			.child("LOGIN-CONTENT")
			.into_page()
	}
	fn register_page() -> Page {
		PageElement::new("div")
			.attr("id", "register")
			.child("REGISTER-CONTENT")
			.into_page()
	}
	fn dashboard() -> Page {
		PageElement::new("div")
			.attr("id", "dashboard")
			.child("DASHBOARD-CONTENT")
			.into_page()
	}
	fn not_found() -> Page {
		PageElement::new("div")
			.attr("id", "not-found")
			.child("NOT-FOUND-CONTENT")
			.into_page()
	}

	ClientLauncher::new("#app")
		.router(|| {
			Router::new()
				.route("/", dashboard)
				.route("/login", login_page)
				.route("/register", register_page)
				.not_found(not_found)
		})
		.launch()
		.expect("launch");

	yield_to_microtasks().await;
	assert!(
		root.inner_html().contains("DASHBOARD-CONTENT"),
		"boot mount: expected DASHBOARD-CONTENT, got {}",
		root.inner_html()
	);

	// Issue #4088 row 2: /login should render login_page after Router::push
	with_router(|r| r.push("/login")).expect("push /login");
	yield_to_microtasks().await;
	yield_to_microtasks().await;
	assert!(
		root.inner_html().contains("LOGIN-CONTENT"),
		"#4088 reproduction: /login must render login_page, got {}",
		root.inner_html()
	);

	// Issue #4088 row 3: /register
	with_router(|r| r.push("/register")).expect("push /register");
	yield_to_microtasks().await;
	yield_to_microtasks().await;
	assert!(
		root.inner_html().contains("REGISTER-CONTENT"),
		"#4088 reproduction: /register must render register_page, got {}",
		root.inner_html()
	);

	// Issue #4088 row 4: /clusters has no route — must fall through to not_found
	with_router(|r| r.push("/clusters")).expect("push /clusters");
	yield_to_microtasks().await;
	yield_to_microtasks().await;
	assert!(
		root.inner_html().contains("NOT-FOUND-CONTENT"),
		"#4088 reproduction: unmatched path /clusters must render not_found, got {}",
		root.inner_html()
	);
}
