//! WASM entry point for the Tier 4 fixture.
//!
//! Delegates startup to [`ClientLauncher`] and exposes the
//! `__diag_*_js` accessors the e2e_cdp test reads back through
//! `execute_js`. Mounts on `#app` to match the Tier 2/3 e2e harness
//! convention.

use reinhardt_pages::app::{ClientLauncher, with_spa_router};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

use super::router;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
	#[cfg(debug_assertions)]
	console_error_panic_hook::set_once();
	ClientLauncher::new("#app")
		.router_client(router::init_router)
		.launch()
}

#[wasm_bindgen]
pub fn __diag_router_id_js() -> usize {
	with_spa_router(|r| r.__diag_router_id())
}

#[wasm_bindgen]
pub fn __diag_observer_count_js() -> usize {
	with_spa_router(|r| r.__diag_observer_count())
}

#[wasm_bindgen]
pub fn __diag_dispatch_count_js() -> u64 {
	with_spa_router(|r| r.__diag_dispatch_count())
}

#[wasm_bindgen]
pub fn __diag_render_count_js() -> u64 {
	ClientLauncher::__diag_render_count()
}

#[derive(Deserialize)]
struct PartialHistoryState {
	#[serde(default)]
	route_name: String,
}

/// Read `history.state.route_name` so the e2e_cdp test can assert
/// Inv-5 without parsing JSON in JS. Returns the empty string when
/// `history.state` is null or has no `route_name` field.
#[wasm_bindgen]
pub fn __diag_history_route_name_js() -> String {
	let Some(window) = web_sys::window() else {
		return String::new();
	};
	let Ok(history) = window.history() else {
		return String::new();
	};
	let Ok(state) = history.state() else {
		return String::new();
	};
	if state.is_null() || state.is_undefined() {
		return String::new();
	}
	match serde_wasm_bindgen::from_value::<PartialHistoryState>(state) {
		Ok(s) => s.route_name,
		Err(_) => String::new(),
	}
}
