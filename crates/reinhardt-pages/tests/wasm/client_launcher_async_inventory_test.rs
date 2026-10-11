//! WASM acceptance test for `ClientLauncher::launch_async` with an async
//! `#[routes]` registration.
//!
//! The registration below is submitted with the constructor that the
//! `#[routes]` macro emits for an `async fn` on `wasm32-unknown-unknown`, so
//! this binary links exactly one async client registration. `launch()` must
//! reject it without running the factory; `launch_async()` must run the
//! `before_launch` callbacks, await the factory, install its routes, and only
//! then run `after_launch` callbacks.
//!
//! Refs #6733.
//!
//! **Run with** (from `crates/reinhardt-pages`):
//!   `cargo test --target wasm32-unknown-unknown --test client_launcher_async_inventory_test`
//!   with `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner`.

#![cfg(wasm)]

use reinhardt_core::page::Page;
use reinhardt_pages::app::{ClientLauncher, with_spa_router};
use reinhardt_urls::routers::{ClientRouter, ClientRouterRegistration};
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

thread_local! {
	/// Launch lifecycle events observed by the factory and the hooks.
	static EVENTS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

fn record(event: &'static str) {
	EVENTS.with(|events| events.borrow_mut().push(event));
}

fn take_events() -> Vec<&'static str> {
	EVENTS.with(|events| std::mem::take(&mut *events.borrow_mut()))
}

fn home() -> Page {
	Page::empty()
}

fn about() -> Page {
	Page::empty()
}

// The registration contract stores `Arc<ClientRouter>` although the router is
// thread-affine; the browser test runs on a single thread.
#[allow(clippy::arc_with_non_send_sync)]
fn async_routes() -> Pin<Box<dyn Future<Output = Arc<ClientRouter>>>> {
	Box::pin(async {
		// Yield to the microtask queue so the launcher has to await the
		// registration instead of finding it already resolved.
		wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(&JsValue::NULL))
			.await
			.expect("resolved promise");
		record("routes");
		Arc::new(
			ClientRouter::new()
				.route("home", "/", home)
				.route("about", "/about/", about),
		)
	})
}

reinhardt_urls::inventory::submit! {
	ClientRouterRegistration::__macro_new_async(async_routes)
}

/// Install a fresh `#app` root element on `document.body` so each test starts
/// from a known DOM state.
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

#[wasm_bindgen_test]
fn launch_rejects_async_inventory_registration_without_running_it() {
	// Arrange
	let _root = install_app_root();
	take_events();

	// Act
	let result = ClientLauncher::new("#app")
		.register_routes_from_inventory()
		.before_launch(|| record("before_launch"))
		.after_launch(|_| record("after_launch"))
		.launch();

	// Assert
	let error = result.expect_err("launch() must reject an async registration");
	assert_eq!(
		error.as_string().as_deref(),
		Some(
			"ClientLauncher::launch: a `#[routes]` registration comes from an `async fn` \
			 and cannot be resolved synchronously. Call `launch_async().await` instead of \
			 `launch()`."
		)
	);
	assert_eq!(take_events(), vec!["before_launch"]);
}

#[wasm_bindgen_test]
async fn launch_async_installs_routes_from_async_registration() {
	// Arrange
	let _root = install_app_root();
	take_events();

	// Act
	let result = ClientLauncher::new("#app")
		.register_routes_from_inventory()
		.before_launch(|| record("before_launch"))
		.after_launch(|_| record("after_launch"))
		.launch_async()
		.await;

	// Assert
	assert_eq!(result.err().and_then(|error| error.as_string()), None);
	assert_eq!(with_spa_router(|router| router.route_count()), 2);
	assert_eq!(
		with_spa_router(|router| router.reverse("about", &[])).ok(),
		Some(String::from("/about/"))
	);
	assert_eq!(
		take_events(),
		vec!["before_launch", "routes", "after_launch"]
	);
}
