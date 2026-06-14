//! Direct symptom-shape test for the SPA navigation regression class
//! (#4075 / #4088 / #4122 / #4203 / #4213 / #4217 / #4221).
//!
//! Whereas Tier 4 (`spa_navigation_diag_named_test.rs`) reads
//! `history.state.route_name` via `js_sys::Reflect::get` (which silently
//! returns `None` for both "state is null" and "state is a JSON string"),
//! this test asserts the *exact* DevTools observation that #4221 reports:
//!
//!   typeof history.state === "object"   // expected
//!   typeof history.state === "string"   // failure shape of #4221
//!
//! Together with explicit `JsValue::is_object()` / `is_string()` checks,
//! this catches the regression even if the value happens to look like a
//! parseable JSON string.
//!
//! **Run with** (from the workspace root):
//!   `wasm-pack test --headless --chrome crates/reinhardt-pages \
//!        --features wasm-diag-test -- --test history_state_shape_test`
#![cfg(wasm)]
use reinhardt_pages::app::{ClientLauncher, with_spa_router};
use reinhardt_pages::component::Page;
use reinhardt_pages::page;
use reinhardt_urls::routers::ClientRouter;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);
fn home_page() -> Page {
	page!(|| {
		div {
			id: "route-home",
			"HOME"
		}
	})()
}
fn clusters_page() -> Page {
	page!(|| {
		div {
			id: "route-clusters",
			"CLUSTERS"
		}
	})()
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
/// Classify `history.state` into a JS-`typeof`-style label using
/// `wasm_bindgen` helpers (`is_null`, `is_string`, `is_object`, ...).
///
/// This is a coarse heuristic, not a call into JS `typeof`, but it
/// covers every shape relevant to the Issue #4221 reproduction
/// (object vs. string) and is expressed in the same vocabulary as
/// DevTools `typeof history.state`.
fn typeof_history_state() -> String {
	let window = web_sys::window().expect("window");
	let history = window.history().expect("history");
	let state = history.state().expect("state");
	if state.is_null() {
		"object".to_string()
	} else if state.is_undefined() {
		"undefined".to_string()
	} else if state.is_string() {
		"string".to_string()
	} else if state.as_f64().is_some() {
		"number".to_string()
	} else if state.as_bool().is_some() {
		"boolean".to_string()
	} else if state.is_object() {
		"object".to_string()
	} else {
		format!("unknown: {:?}", state)
	}
}
fn raw_history_state() -> JsValue {
	web_sys::window()
		.expect("window")
		.history()
		.expect("history")
		.state()
		.expect("state")
}
#[wasm_bindgen_test]
async fn history_state_is_object_after_router_push() {
	let _root = install_app_root();
	ClientLauncher::new("#app")
		.router_client(build_router)
		.launch()
		.expect("launch");
	// Drive a programmatic navigation, mirroring the link-interceptor's
	// `with_spa_router(|r| r.push(href))` call site exactly.
	with_spa_router(|r| r.push("/clusters")).expect("push /clusters");

	// Yield once so any async observers / scheduler tasks settle before
	// we read `history.state`. The state assertions below are independent
	// of observer ordering, but yielding keeps the failure shape stable
	// across runs.
	let promise = js_sys::Promise::resolve(&JsValue::UNDEFINED);
	let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
	let state = raw_history_state();
	let typeof_str = typeof_history_state();
	assert_eq!(
		typeof_str, "object",
		"#4221 symptom shape: typeof history.state expected \"object\" \
		 (the structured-JS-object format produced by `state_to_js_object`), \
		 got {:?}. \
		 Raw state JsValue: {:?}",
		typeof_str, state
	);
	assert!(
		!state.is_string(),
		"#4221 regression: history.state must NOT be a JS string. \
		 If this fires, `push_state` is taking a code path that calls \
		 `JsValue::from_str(&json)` instead of `serde_wasm_bindgen::to_value(...)`. \
		 Raw state: {:?}",
		state
	);
	assert!(
		state.is_object(),
		"#4221 regression: history.state expected to be a JS object \
		 (`is_object()`); got {:?}",
		state
	);
	let route_name_key = JsValue::from_str("route_name");
	let value = js_sys::Reflect::get(&state, &route_name_key)
		.expect("Reflect::get must succeed on an object");
	assert_eq!(
		value.as_string().as_deref(),
		Some("clusters:list"),
		"#4221: external-consumer property access must yield the matched \
		 route name. If state were a JSON string, Reflect::get would \
		 return undefined here."
	);
}
#[wasm_bindgen_test]
async fn history_state_is_object_after_router_replace() {
	let _root = install_app_root();
	ClientLauncher::new("#app")
		.router_client(build_router)
		.launch()
		.expect("launch");
	with_spa_router(|r| r.replace("/clusters")).expect("replace /clusters");
	let promise = js_sys::Promise::resolve(&JsValue::UNDEFINED);
	let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
	let state = raw_history_state();
	let typeof_str = typeof_history_state();
	assert_eq!(
		typeof_str, "object",
		"#4221 symptom shape (replace path): typeof history.state expected \
		 \"object\", got {:?}. Raw state: {:?}",
		typeof_str, state
	);
	assert!(
		!state.is_string(),
		"#4221 regression (replace path): history.state must NOT be a JS string. \
		 Raw state: {:?}",
		state
	);
	assert!(
		state.is_object(),
		"#4221 regression (replace path): history.state expected `is_object()`; \
		 got {:?}",
		state
	);
}
