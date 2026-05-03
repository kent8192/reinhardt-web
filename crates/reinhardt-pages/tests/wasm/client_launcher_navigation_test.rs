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

/// Regression test for the popstate gap fixed in #4108: browser
/// back/forward must trigger the launcher's render path, which means
/// the underlying `on_navigate` observers must fire from popstate.
/// Before the fix, only programmatic `Router::push` / `Router::replace`
/// woke observers.
#[wasm_bindgen_test]
async fn client_launcher_re_renders_on_popstate() {
	let root = install_app_root();

	let observed_paths: std::rc::Rc<std::cell::RefCell<Vec<String>>> =
		std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
	let observed_paths_for_listener = observed_paths.clone();

	ClientLauncher::new("#app")
		.router(|| {
			Router::new()
				.route("/", page_root)
				.route("/a", page_a)
				.route("/b", page_b)
		})
		.after_launch(move |_ctx| {
			// after_launch is FnOnce, so observed_paths_for_listener can
			// be moved straight into the listener body without an extra
			// clone.
			let sub = with_router(move |r| {
				r.on_navigate(move |path, _params| {
					observed_paths_for_listener
						.borrow_mut()
						.push(path.to_string());
				})
			});
			// Leak the subscription for the lifetime of the test; it is
			// dropped naturally when the WASM module exits.
			std::mem::forget(sub);
		})
		.launch()
		.expect("launch");

	yield_to_microtasks().await;

	// Arrange: navigate forward to /a then /b so popstate has somewhere
	// to pop back to.
	with_router(|r| r.push("/a")).expect("push /a");
	yield_to_microtasks().await;
	yield_to_microtasks().await;
	with_router(|r| r.push("/b")).expect("push /b");
	yield_to_microtasks().await;
	yield_to_microtasks().await;

	let html_at_b = root.inner_html();
	assert!(
		html_at_b.contains("ROUTE-B-CONTENT"),
		"setup precondition: should be on /b before history.back, got: {html_at_b}"
	);

	// Act: simulate the browser back button.
	let history = web_sys::window().unwrap().history().unwrap();
	history.back().expect("history.back");
	// popstate is dispatched as a macrotask. The yield_to_microtasks
	// helper uses TimeoutFuture::new(0), which is itself a macrotask
	// boundary (setTimeout(0)), so two yields suffice to let popstate
	// fire and then for the launcher's render Effect (or, post-#4101,
	// on_navigate listener) to commit the DOM update.
	yield_to_microtasks().await;
	yield_to_microtasks().await;

	// Assert: DOM reflects /a, and the on_navigate observer received the
	// popstate path (this is the bit that used to silently break).
	let html_after_back = root.inner_html();
	assert!(
		html_after_back.contains("ROUTE-A-CONTENT"),
		"expected /a view after history.back from /b, got: {html_after_back}"
	);
	assert!(
		!html_after_back.contains("ROUTE-B-CONTENT"),
		"/b view should be gone after history.back, got: {html_after_back}"
	);

	let paths = observed_paths.borrow();
	assert_eq!(
		paths.as_slice(),
		&["/a".to_string(), "/b".to_string(), "/a".to_string()],
		"on_navigate listener must fire for each push and the popstate, in order"
	);
}

/// Regression coverage for the structural fragility class
/// (#3348, #4075, #4088). Multiple back-to-back navigations exercise
/// the full render -> cleanup_reactive_nodes -> remount cycle
/// repeatedly. Once PR #4101 removes the launcher's render Effect,
/// the Effect/Signal auto-tracking corruption pattern is structurally
/// impossible regardless of the reactive primitives embedded in route
/// views; this test guards the navigation cycle itself.
///
/// Refs #4101, #4088, #4075, #3348.
#[wasm_bindgen_test]
async fn client_launcher_handles_back_to_back_navigations() {
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

	// Act: bounce between /a and /b multiple times. The regression
	// class manifested as the second or third navigation no longer
	// re-mounting.
	for iteration in 0..3 {
		with_router(|r| r.push("/a")).expect("push /a");
		yield_to_microtasks().await;
		yield_to_microtasks().await;
		assert!(
			root.inner_html().contains("ROUTE-A-CONTENT"),
			"iteration {iteration}: expected /a view after push('/a'), got: {}",
			root.inner_html()
		);
		assert!(
			!root.inner_html().contains("ROUTE-B-CONTENT"),
			"iteration {iteration}: /b view should be gone after push('/a'), got: {}",
			root.inner_html()
		);

		with_router(|r| r.push("/b")).expect("push /b");
		yield_to_microtasks().await;
		yield_to_microtasks().await;
		assert!(
			root.inner_html().contains("ROUTE-B-CONTENT"),
			"iteration {iteration}: expected /b view after push('/b'), got: {}",
			root.inner_html()
		);
		assert!(
			!root.inner_html().contains("ROUTE-A-CONTENT"),
			"iteration {iteration}: /a view should be gone after push('/b'), got: {}",
			root.inner_html()
		);
	}
}
