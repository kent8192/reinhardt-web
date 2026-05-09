//! Standing CI guard for the wasm-bindgen ABI hypothesis from #4221.
//!
//! Independently of the framework's `state_to_js_object` call site,
//! this test asserts the **library-level invariant** that
//! `serde_wasm_bindgen::to_value(&HistoryStateJson)` returns a JS
//! object (not a string), and that the value round-trips through
//! `History::push_state_with_url` preserving `typeof === "object"`.
//!
//! Regression-prevention rationale: #4221 tracked a runtime symptom
//! (`history.state` is a JSON string) that was once thought to be a
//! `wasm-bindgen 0.2.118` ABI bug, fixable by bumping to 0.2.121.
//! That hypothesis was falsified — the round-trip is correct on
//! 0.2.118 too — but the assertion remains the cheapest, most direct
//! verification that the SPA navigation regression class isn't
//! returning via this layer. If a future `wasm-bindgen` /
//! `serde-wasm-bindgen` upgrade silently changes how `to_value`
//! produces structured-clone-compatible JsValues from a struct
//! containing `HashMap<String, String>` fields, this test catches it
//! before it reaches downstream consumers.
//!
//! **Run with**:
//!   `wasm-pack test --headless --chrome crates/reinhardt-pages \
//!        --features wasm-diag-test \
//!        -- --test wasm_bindgen_abi_pin_test`

#![cfg(wasm)]
#![allow(deprecated)] // (Refs #4234) Test exercises deprecated `pages::Router` surface.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// Mirror of `reinhardt-urls`'s private `HistoryStateJson` wire shape.
///
/// Kept here as a literal copy rather than re-exporting the framework
/// type so the test exercises **only** the serde-wasm-bindgen → wasm
/// layer, divorced from any framework-internal helper. If the framework
/// type evolves, update this struct to mirror it.
#[derive(Serialize, Deserialize)]
struct HistoryStateJson {
	path: String,
	params: HashMap<String, String>,
	route_name: Option<String>,
	data: HashMap<String, String>,
	scroll_x: Option<i32>,
	scroll_y: Option<i32>,
}

fn make_state() -> HistoryStateJson {
	HistoryStateJson {
		path: "/clusters".to_string(),
		params: HashMap::new(),
		route_name: Some("dashboard:clusters".to_string()),
		data: HashMap::new(),
		scroll_x: None,
		scroll_y: None,
	}
}

#[wasm_bindgen_test]
fn serde_wasm_bindgen_to_value_returns_object_not_string() {
	let state = make_state();
	let js_value = serde_wasm_bindgen::to_value(&state).expect("to_value");

	assert!(
		!js_value.is_string(),
		"#4221 ABI: serde_wasm_bindgen::to_value produced a JS string \
		 instead of an object. wasm-bindgen / serde-wasm-bindgen ABI \
		 skew is the prime suspect. Raw JsValue: {:?}",
		js_value
	);
	assert!(
		js_value.is_object(),
		"#4221 ABI: serde_wasm_bindgen::to_value produced a non-object \
		 JsValue. Raw JsValue: {:?}",
		js_value
	);
}

#[wasm_bindgen_test]
fn pushstate_round_trip_preserves_object_shape() {
	let state = make_state();
	let js_value = serde_wasm_bindgen::to_value(&state).expect("to_value");

	let window = web_sys::window().expect("window");
	let history = window.history().expect("history");
	history
		.push_state_with_url(&js_value, "", Some("/clusters"))
		.expect("push_state");

	let read_back = history.state().expect("state");

	assert!(
		!read_back.is_string(),
		"#4221: history.state is a JS string after push_state. \
		 If this fires, the wasm-bindgen / serde-wasm-bindgen / web-sys \
		 stack has regressed `to_value` → structured-clone behaviour. \
		 Raw: {:?}",
		read_back
	);
	assert!(
		read_back.is_object(),
		"#4221: history.state is not is_object() after push_state. Raw: {:?}",
		read_back
	);

	// Cleanup so subsequent tests start with a clean slate.
	let _ = history.replace_state_with_url(&JsValue::NULL, "", Some("/"));
}
