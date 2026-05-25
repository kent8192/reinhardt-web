//! Regression tests for `install_link_interceptor` robustness (Refs #4330, #4331).
//!
//! - #4330: A click whose `event.target` is a non-Element `Node` (e.g. a
//!   `Text` node inside `<a><span>label</span></a>` or directly inside
//!   `<a>label</a>`) must still resolve the enclosing `<a>` ancestor and
//!   trigger SPA navigation. Previously the interceptor cast straight to
//!   `Element` and silently no-op'd for text-node targets.
//!
//! - #4331: The `let _ = r.push(href);` swallow was replaced with a
//!   `match` that emits `nav_diag!` + `console.warn` on failure. The
//!   success path test below guards against accidentally short-
//!   circuiting the happy path in the refactor.
//!
//! **Run with** (from the workspace root):
//!   `wasm-pack test --headless --chrome crates/reinhardt-pages \
//!        --features wasm-diag-test,nav-diag-dom \
//!        -- --test link_interceptor_robustness_test`

#![cfg(all(wasm, feature = "nav-diag-dom"))]

use reinhardt_pages::app::ClientLauncher;
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
		.route("dashboard:home", "/", home_page)
		.route("clusters:list", "/clusters", clusters_page)
}

fn read_dataset_nav_site() -> Option<String> {
	let document = web_sys::window()?.document()?;
	let body = document.body()?;
	body.get_attribute("data-reinhardt-nav-site")
}

/// Click inside `<a><span>label</span></a>` whose `event.target` is the
/// inner element must still trigger SPA navigation. Before the #4330
/// fix, the interceptor cast straight to `Element` and silently no-op'd
/// when the target was anything other than the anchor itself.
#[wasm_bindgen_test]
async fn link_interceptor_resolves_anchor_from_nested_element_target() {
	let _root = install_app_root();

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

	const ANCHOR_ID: &str = "test-link-interceptor-nested-anchor";
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

	let span: web_sys::HtmlElement = document
		.create_element("span")
		.expect("create span")
		.dyn_into()
		.expect("dyn HtmlElement");
	span.set_text_content(Some("go to clusters"));
	anchor.append_child(&span).expect("append span");
	body.append_child(&anchor).expect("append a");

	body.set_attribute("data-reinhardt-nav-site", "test_sentinel")
		.expect("set sentinel");

	// Click on the inner `<span>`. Its `event.target` is the span, not
	// the enclosing `<a>`. The fix walks up via `parent_element()` to
	// find the anchor.
	span.click();

	let promise = js_sys::Promise::resolve(&JsValue::UNDEFINED);
	let _ = wasm_bindgen_futures::JsFuture::from(promise).await;

	let site = read_dataset_nav_site();
	assert_ne!(
		site.as_deref(),
		Some("test_sentinel"),
		"#4330: click on <span> inside <a> did not trigger SPA navigation; \
		 the link interceptor failed to resolve the enclosing <a> ancestor."
	);
	assert_eq!(
		site.as_deref(),
		Some("notify_observers"),
		"#4330: after a click on a nested element of <a>, last-write-wins \
		 value should be \"notify_observers\". Got: {:?}",
		site
	);

	if let Some(prev) = document.get_element_by_id(ANCHOR_ID) {
		prev.remove();
	}
	let _ = history.replace_state_with_url(&JsValue::NULL, "", Some("/"));
}

/// Dispatch a bubbling MouseEvent whose `target` is a `Text` node
/// directly inside `<a>label</a>`. This exercises the Node-walking
/// fallback added for #4330: the previous `dyn_ref::<Element>()` cast
/// returned `None` for `Text` targets so the handler silently returned.
#[wasm_bindgen_test]
async fn link_interceptor_resolves_anchor_from_text_node_target() {
	let _root = install_app_root();

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

	const ANCHOR_ID: &str = "test-link-interceptor-textnode-anchor";
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
	let text = document.create_text_node("go");
	anchor.append_child(&text).expect("append text");
	body.append_child(&anchor).expect("append a");

	body.set_attribute("data-reinhardt-nav-site", "test_sentinel")
		.expect("set sentinel");

	// Construct a bubbling MouseEvent and dispatch it directly from the
	// `Text` node — this guarantees `event.target` is the Text node
	// (a non-Element `Node`), exercising the parent_node() walk.
	let init = web_sys::MouseEventInit::new();
	init.set_bubbles(true);
	init.set_cancelable(true);
	let event = web_sys::MouseEvent::new_with_mouse_event_init_dict("click", &init)
		.expect("new MouseEvent");
	// dispatch_event returns false if any listener called prevent_default().
	// The interceptor under test is expected to call prevent_default() once it
	// walks up from the Text node to the enclosing <a>, so the return value is
	// not a reliable setup check. Behavior is verified by the dataset assertions
	// below.
	let _ = text.dispatch_event(&event).expect("dispatch_event text");

	let promise = js_sys::Promise::resolve(&JsValue::UNDEFINED);
	let _ = wasm_bindgen_futures::JsFuture::from(promise).await;

	let site = read_dataset_nav_site();
	assert_ne!(
		site.as_deref(),
		Some("test_sentinel"),
		"#4330: click whose event.target is a Text node inside <a> did not \
		 trigger SPA navigation; the link interceptor failed to walk up \
		 from the non-Element target to the enclosing <a>."
	);
	assert_eq!(
		site.as_deref(),
		Some("notify_observers"),
		"#4330: after a Text-node-target click, last-write-wins value \
		 should be \"notify_observers\". Got: {:?}",
		site
	);

	if let Some(prev) = document.get_element_by_id(ANCHOR_ID) {
		prev.remove();
	}
	let _ = history.replace_state_with_url(&JsValue::NULL, "", Some("/"));
}

/// Push-success path must still complete after the #4331 error-handling
/// refactor (replaced `let _ = r.push(href);` with a `match`). This
/// guards against a refactor regression where the new `match` arm
/// accidentally short-circuits on success.
#[wasm_bindgen_test]
async fn link_interceptor_push_success_still_drives_navigation_chain() {
	let _root = install_app_root();

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

	const ANCHOR_ID: &str = "test-link-interceptor-push-success-anchor";
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
	anchor.set_text_content(Some("go"));
	body.append_child(&anchor).expect("append a");

	body.set_attribute("data-reinhardt-nav-site", "test_sentinel")
		.expect("set sentinel");

	anchor.click();

	let promise = js_sys::Promise::resolve(&JsValue::UNDEFINED);
	let _ = wasm_bindgen_futures::JsFuture::from(promise).await;

	let site = read_dataset_nav_site();
	assert_eq!(
		site.as_deref(),
		Some("notify_observers"),
		"#4331 regression guard: after the success-path refactor, a normal \
		 link click must still reach `notify_observers`. Got: {:?}",
		site,
	);

	if let Some(prev) = document.get_element_by_id(ANCHOR_ID) {
		prev.remove();
	}
	let _ = history.replace_state_with_url(&JsValue::NULL, "", Some("/"));
}
