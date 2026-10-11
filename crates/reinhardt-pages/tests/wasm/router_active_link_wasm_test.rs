//! Browser coverage for the reactive current route exposed by `use_router()`
//! and for active-link rendering in `router::Link` (Issue #6727).

#![cfg(wasm)]

use reinhardt_pages::app::{__clear_spa_router_for_test, __install_client_router_for_test};
use reinhardt_pages::component::{Component, IntoPage, Page, PageElement, PageExt};
use reinhardt_pages::dom::Element;
use reinhardt_pages::prelude::defer_yield;
use reinhardt_pages::reactive::hooks::use_router;
use reinhardt_pages::reactive::{ReactiveScope, with_runtime};
use reinhardt_pages::router::Link;
use reinhardt_urls::routers::ClientRouter;
use rstest::rstest;
use serial_test::serial;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// Restores the browser location and clears the SPA router slot on drop.
struct BrowserStateGuard {
	href: String,
	state: JsValue,
}

impl BrowserStateGuard {
	fn capture() -> Self {
		let window = web_sys::window().expect("window");
		let href = window.location().href().expect("current href");
		let state = window
			.history()
			.expect("history")
			.state()
			.expect("history state");
		Self { href, state }
	}
}

impl Drop for BrowserStateGuard {
	fn drop(&mut self) {
		__clear_spa_router_for_test();
		web_sys::window()
			.expect("window")
			.history()
			.expect("history")
			.replace_state_with_url(&self.state, "", Some(&self.href))
			.expect("restore browser location");
	}
}

/// Appends a mount root to the document body and removes it on drop.
struct BodyRoot {
	element: web_sys::Element,
}

impl BodyRoot {
	fn new(id: &str) -> Self {
		let document = web_sys::window()
			.expect("window")
			.document()
			.expect("document");
		let element = document.create_element("div").expect("create root");
		element.set_id(id);
		document
			.body()
			.expect("body")
			.append_child(&element)
			.expect("append root");
		Self { element }
	}

	fn anchor(&self, href: &str) -> web_sys::Element {
		self.element
			.query_selector(&format!("a[href=\"{href}\"]"))
			.expect("query anchor")
			.expect("anchor exists")
	}
}

impl Drop for BodyRoot {
	fn drop(&mut self) {
		reinhardt_pages::cleanup_reactive_nodes();
		self.element.remove();
	}
}

fn replace_path(path: &str) {
	web_sys::window()
		.expect("window")
		.history()
		.expect("history")
		.replace_state_with_url(&JsValue::NULL, "", Some(path))
		.expect("replace test path");
}

fn shell_router() -> ClientRouter {
	ClientRouter::new()
		.route("overview", "/", || Page::text("Overview"))
		.route("project", "/projects/{project_id}/", || {
			Page::text("Project")
		})
}

#[rstest]
#[wasm_bindgen_test]
#[serial(router)]
fn current_path_signal_changes_after_push() {
	ReactiveScope::run(|| {
		// Arrange
		let _state = BrowserStateGuard::capture();
		replace_path("/");
		__install_client_router_for_test(shell_router());
		let router = use_router();
		let path = router
			.current_path()
			.expect("an installed router exposes its path signal");
		let initial = path.get();

		// Act
		router
			.push("/projects/41/")
			.expect("push to a registered route");

		// Assert
		assert_eq!(initial, "/");
		assert_eq!(path.get(), "/projects/41/");
	});
}

#[rstest]
#[wasm_bindgen_test]
#[serial(router)]
fn current_route_name_resolves_the_named_route() {
	ReactiveScope::run(|| {
		// Arrange
		let _state = BrowserStateGuard::capture();
		replace_path("/projects/7/");
		__install_client_router_for_test(shell_router());
		let router = use_router();
		let route_name = router
			.current_route_name()
			.expect("an installed router exposes its route-name signal");
		let initial = route_name.get();

		// Act
		router.push("/").expect("push to a registered route");

		// Assert
		assert_eq!(initial.as_deref(), Some("project"));
		assert_eq!(route_name.get().as_deref(), Some("overview"));
	});
}

// `wasm_bindgen_test`'s attribute path does not end in a `test` segment, so
// rstest needs `test_attr` to recognize this as an async test.
#[rstest]
#[serial(router)]
#[test_attr(wasm_bindgen_test)]
async fn link_toggles_aria_current_on_navigation() {
	// Arrange
	let _state = BrowserStateGuard::capture();
	replace_path("/");
	let root = BodyRoot::new("router-active-link");
	// `ReactiveScope` disposes itself on drop. Declared last, it is dropped
	// first, so the mounted effects are disposed before the root is removed
	// and the router is cleared, even when an assertion fails.
	let scope = ReactiveScope::new();
	scope.enter(|| {
		__install_client_router_for_test(shell_router());
		PageElement::new("nav")
			.child(
				Link::new("/", "Overview")
					.class("nav")
					.active_class("active")
					.render(),
			)
			.child(
				Link::new("/projects/41/", "Project")
					.class("nav")
					.active_class("active")
					.render(),
			)
			.into_page()
			.mount(&Element::new(root.element.clone()))
			.expect("navigation mounts");
	});
	let overview = root.anchor("/");
	let project = root.anchor("/projects/41/");
	let overview_before = (
		overview.get_attribute("aria-current"),
		overview.get_attribute("class"),
	);
	let project_before = (
		project.get_attribute("aria-current"),
		project.get_attribute("class"),
	);

	// Act
	scope.enter(|| {
		use_router()
			.push("/projects/41/")
			.expect("push to a registered route");
	});
	// Mounted reactive attributes are passive effects. Without
	// `ClientLauncher::launch` no scheduler is installed, so flush them
	// explicitly before reading the DOM.
	with_runtime(|runtime| runtime.flush_updates());
	defer_yield().await;

	// Assert
	assert_eq!(
		overview_before,
		(Some("page".to_owned()), Some("nav active".to_owned()))
	);
	assert_eq!(project_before, (None, Some("nav".to_owned())));
	assert_eq!(overview.get_attribute("aria-current"), None);
	assert_eq!(overview.get_attribute("class").as_deref(), Some("nav"));
	assert_eq!(
		project.get_attribute("aria-current").as_deref(),
		Some("page")
	);
	assert_eq!(
		project.get_attribute("class").as_deref(),
		Some("nav active")
	);
}
