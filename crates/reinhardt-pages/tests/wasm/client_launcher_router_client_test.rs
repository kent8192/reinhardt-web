//! WASM acceptance test for `ClientLauncher::router_client`.
//!
//! Build a `urls::ClientRouter`, mount it via
//! `ClientLauncher::router_client`, and exercise the SPA pipeline
//! (mount -> match -> observer dispatch). Mirrors the shape of
//! `tests/wasm/client_launcher_navigation_test.rs`.
//!
//! Refs #4234, cloud#578 Phase E.
//!
//! **Run with** (from the workspace root):
//!   `wasm-pack test --headless --chrome crates/reinhardt-pages -- --test client_launcher_router_client_test`
#![cfg(wasm)]
#![allow(deprecated)]
use reinhardt_core::page::Page;
use reinhardt_pages::app::ClientLauncher;
use reinhardt_urls::routers::ClientRouter;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);
fn home() -> Page {
	Page::empty()
}
fn about() -> Page {
	Page::empty()
}
/// Install a fresh `#app` root element on `document.body` so each
/// test starts from a known DOM state. Mirrors the helper in
/// `client_launcher_navigation_test.rs`.
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
fn router_client_launcher_accepts_configured_router() {
	let _root = install_app_root();
	let dispatched = Rc::new(Cell::new(0u64));
	let dispatched_clone = dispatched.clone();
	let launcher_result = ClientLauncher::new("#app")
		.router_client(move || {
			let r = ClientRouter::new()
				.named_route("home", "/", home)
				.named_route("about", "/about", about);
			let _sub = r.on_navigate(move |_, _| {
				dispatched_clone.set(dispatched_clone.get() + 1);
			});
			r
		})
		.launch();
	assert!(
		launcher_result.is_ok(),
		"router_client launch must succeed, got: {:?}",
		launcher_result.err()
	);
}
#[wasm_bindgen_test]
fn router_client_and_router_are_mutually_exclusive() {
	let _root = install_app_root();
	let result = ClientLauncher::new("#app")
		.router(reinhardt_pages::router::Router::new)
		.router_client(ClientRouter::new)
		.launch();
	assert!(
		result.is_err(),
		"setting both `router(...)` and `router_client(...)` must error"
	);
}
